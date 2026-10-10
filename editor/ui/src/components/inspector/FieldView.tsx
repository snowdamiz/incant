import { useId } from 'react';
import type { CSSProperties, InputHTMLAttributes, MouseEvent, ReactNode } from 'react';
import type { ComponentSchema, Diagnostic, FieldSchema } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import type { IconName } from '../../icons/Icon';
import type { AxisName, FieldHint, FieldStatus } from './presentation';
import { MASK_BITS, fieldHint, isMask, maskDescription, maskGroups, maskSummary } from './presentation';
import { exactTitle, formatNumber, humanize, isRecord, listAll, listChoices, looksLikeUlid, pointer, safeJson, unescapePointer } from './format';
import { ObjectListView, isObjectList } from './ObjectList';
import { NumberListView, StringListView, isLongNumberList, isStringList } from './ValueLists';
import { breakAfterSlashes } from '../breakAfterSlashes';

export { formatNumber, humanize, pointer, safeJson } from './format';

export function fieldDomId(entity: string, component: string, path: readonly string[]): string {
  // Encode each UTF-16 code unit outside the alphanumeric alphabet (including
  // the escape marker and separator). Empty tokens and token boundaries remain
  // distinct, even for arbitrary schema keys or malformed Unicode in input data.
  const tokens = [entity, component, ...path].map((token) =>
    token.replace(/[^A-Za-z0-9]/g, (unit) => `_${unit.charCodeAt(0).toString(16).padStart(4, '0')}`),
  );
  // A terminal marker keeps derived IDs such as `${id}-label` outside the field
  // ID namespace: a child named "label" cannot claim its parent's label ID.
  return `field-${tokens.join('-')}--value`;
}

/** What an exact ID in the open project names: an asset or an entity. */
export interface Reference {
  readonly name: string;
  /** The asset or entity kind, for its icon. */
  readonly kind: string;
  /** Words for assistive tech: "Model asset crate_stack", "Entity Player". */
  readonly spoken: string;
}
export type ReferenceResolver = (id: string) => Reference | null;

export interface FieldContext {
  readonly entity: string;
  readonly component: string;
  /** Diagnostics for this component, matched to fields by path prefix. */
  readonly diagnostics: readonly Diagnostic[];
  /** The whole component value, for presentation tags that depend on sibling fields. */
  readonly value?: Readonly<Record<string, unknown>> | undefined;
  /** The component's schema, so such tags only name modes (variants) it declares. */
  readonly schema?: ComponentSchema | undefined;
  /** Nesting depth of the rows: each group indents and narrows its label column by one step. */
  readonly depth?: number | undefined;
  /** Names for IDs that exactly match a project asset or entity. Presentation only. */
  readonly refs?: ReferenceResolver | undefined;
}

/**
 * Read-only, schema-driven presentation of one property. The UI does not
 * validate: diagnostics come from the engine. It only refuses to *present* a
 * value the schema does not describe, and says so explicitly.
 */
export function FieldView({
  name,
  schema,
  value,
  path,
  ctx,
}: {
  name: string;
  schema: FieldSchema;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
}) {
  const hint = fieldHint(ctx.component, pointer(path));
  const label = schema.title ?? hint.title ?? humanize(name);
  const unset = unsetKind(schema, value);
  const fallback = unset === 'omitted' ? presentableDefault(schema) : null;
  if (fallback) {
    // An omitted optional field shown as its schema default, always tagged "Default"
    // so it never reads as authored data. The document is not changed.
    const note = hint.omittedNote ?? DEFAULT_NOTE;
    if (schema.type === 'tagged-union' && 'variants' in schema) {
      return <UnionView label={label} schema={schema} hint={hint} value={fallback.value} path={path} ctx={ctx} implied={note} />;
    }
    return <FieldRow label={label} schema={schema} hint={hint} value={fallback.value} path={path} ctx={ctx} implied={note} />;
  }
  if (unset) {
    return <FieldRow label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} unset={unset} />;
  }
  if (schema.type === 'tagged-union' && 'variants' in schema) {
    return <UnionView label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
  }
  if (schema.type === 'object' && 'properties' in schema) {
    return <ObjectGroup label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
  }
  if (schema.type === 'object' && isRecord(value)) {
    // An open object (additionalProperties, no listed properties), such as a script's props.
    return <FreeformGroup label={label} value={value} path={path} ctx={ctx} />;
  }
  if (schema.type === 'array' && 'items' in schema && Array.isArray(value)) {
    if (isObjectList(schema)) {
      // Selection is per entity: a different entity starts at its own first problem (or part 1).
      return <ObjectListView key={ctx.entity} label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
    }
    if (isLongNumberList(schema, value, hint)) {
      return <NumberListView key={ctx.entity} label={label} schema={schema} hint={hint} value={value as number[]} path={path} ctx={ctx} />;
    }
    if (isStringList(schema, value)) {
      return <StringListView label={label} schema={schema} hint={hint} value={value as string[]} path={path} ctx={ctx} />;
    }
  }
  return <FieldRow label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
}

type UnionSchema = Extract<FieldSchema, { type: 'tagged-union' }>;
type ObjectSchema = Extract<FieldSchema, { type: 'object' }>;

const DEFAULT_NOTE = 'Not authored. The schema default applies.';

/** Style for a group's rows: one indent step deeper than its parent. */
export function depthStyle(depth: number): CSSProperties {
  return { '--depth': depth } as CSSProperties;
}

/**
 * The schema's own `default`, when it is something this field can present: a
 * finite number for a number field, or a record naming a known variant for a
 * tagged union. Anything else (no default, a mistyped or unknown one) returns
 * null and the field stays "Not set"; a default is never guessed or repaired.
 */
export function presentableDefault(schema: FieldSchema): { value: unknown } | null {
  if (!Object.hasOwn(schema, 'default')) return null;
  const value: unknown = schema.default;
  if ((schema.type === 'number' || schema.type === 'integer') && typeof value === 'number' && Number.isFinite(value)) {
    return { value };
  }
  if (schema.type === 'tagged-union' && 'variants' in schema && isRecord(value)) {
    const tag = value[schema.discriminator];
    if (typeof tag === 'string' && Object.hasOwn(schema.variants, tag)) return { value };
  }
  return null;
}

/** Profile order first (the authored meaning), then any other keys in schema order. */
export function orderedProperties(properties: Readonly<Record<string, FieldSchema>>, order: readonly string[] | undefined): string[] {
  const all = Object.keys(properties);
  const first = (order ?? []).filter((key) => all.includes(key));
  return [...first, ...all.filter((key) => !first.includes(key))];
}

/** A nested object with listed properties: a captioned, indented group of rows. */
function ObjectGroup({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: ObjectSchema;
  hint: FieldHint;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
}) {
  const record = isRecord(value) ? value : null;
  const properties = schema.properties as Readonly<Record<string, FieldSchema>>;
  const depth = (ctx.depth ?? 0) + 1;
  const inner = { ...ctx, depth };
  return (
    <fieldset className="field-group" style={depthStyle(depth)}>
      <legend className="field-group__legend">{label}</legend>
      {record ? (
        orderedProperties(properties, hint.order).map((key) => (
          <FieldView key={key} name={key} schema={properties[key]!} value={record[key]} path={[...path, key]} ctx={inner} />
        ))
      ) : (
        <Mismatch expected="object" value={value} />
      )}
      {record
        ? Object.keys(record)
            .filter((key) => !Object.hasOwn(properties, key))
            .map((key) => (
              <ExtraField key={key} name={key} owner={label.toLowerCase()} value={record[key]} />
            ))
        : null}
    </fieldset>
  );
}

/** A value's own shape, for an open object whose schema lists no properties. */
function inferredSchema(key: string, value: unknown): FieldSchema {
  if (typeof value === 'number') return { type: 'number', title: key };
  if (typeof value === 'string') return { type: 'string', title: key };
  if (typeof value === 'boolean') return { type: 'boolean', title: key };
  if (isRecord(value)) return { type: 'object', title: key } as FieldSchema;
  return { type: 'x-json', title: key } as FieldSchema;
}

/**
 * An open object (a script's props): each authored key as it is written, with its
 * value presented by its own type. Lists and null show as raw JSON. Nothing is
 * inferred beyond the value itself, and nothing is hidden.
 */
function FreeformGroup({ label, value, path, ctx }: { label: string; value: Record<string, unknown>; path: readonly string[]; ctx: FieldContext }) {
  const depth = (ctx.depth ?? 0) + 1;
  const inner = { ...ctx, depth };
  const keys = Object.keys(value);
  return (
    <fieldset className="field-group field-group--open" style={depthStyle(depth)}>
      <legend className="field-group__legend" title="Keys and values as authored; the schema does not list them.">
        {label}
      </legend>
      {keys.length === 0 ? <p className="field-group__note">No entries.</p> : null}
      {keys.map((key) => (
        <FieldView key={key} name={key} schema={inferredSchema(key, value[key])} value={value[key]} path={[...path, key]} ctx={inner} />
      ))}
    </fieldset>
  );
}

function ExtraField({ name, owner, value }: { name: string; owner: string; value: unknown }) {
  return (
    <p className="component__notice">
      <span>
        <Icon name="warning" size={12} /> Field <code>{name}</code> is not part of {owner}: <code>{safeJson(value)}</code>
      </span>
    </p>
  );
}

/**
 * A tagged union (such as Collider.shape) is one row naming the variant, with
 * that variant's fields grouped beneath it. A variant is selected only when the
 * value is a record whose tag names a known variant; a missing, non-string or
 * unknown tag is a visible mismatch and no variant (and no default) is shown.
 * The engine still validates: this only refuses to guess.
 *
 * The variant's fields continue at the selector's own depth, with no indent or
 * guide: the selector names the variant, and the variant's note sits under its
 * value. Nesting stays for real nested objects, so a compound part is not boxed
 * three levels deep.
 */
function UnionView({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
  implied,
}: {
  label: string;
  schema: UnionSchema;
  hint: FieldHint;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
  /** Set when the value is the documented default for an omitted field, not authored data. */
  implied?: string | undefined;
}) {
  const tagKey = schema.discriminator;
  const choices = Object.keys(schema.variants);
  const record = isRecord(value) ? value : null;
  const tag = record?.[tagKey];
  const variant = typeof tag === 'string' && Object.hasOwn(schema.variants, tag) ? schema.variants[tag] : undefined;
  const fields = objectProperties(variant);
  const here = pointer(path);
  const children = fields ? Object.keys(fields).filter((key) => key !== tagKey) : [];
  // The row owns the union path, its tag and anything beneath it that no child row presents.
  const claim = (target: string) => {
    if (target === here) return true;
    if (!target.startsWith(`${here}/`)) return false;
    const [first] = target.slice(here.length + 1).split('/');
    return !children.includes(unescapePointer(first ?? ''));
  };
  let notice: ReactNode = null;
  if (!record) {
    notice = `Cannot show as a ${label.toLowerCase()} (${listChoices(choices)}); raw value:`;
  } else if (tag === undefined) {
    notice = `No “${tagKey}” is set, so the ${label.toLowerCase()} cannot be shown. Expected ${listChoices(choices)}; raw value:`;
  } else if (!variant) {
    notice = `Unknown ${label.toLowerCase()} ${tagKey} ${safeJson(tag)}. Expected ${listChoices(choices)}; raw value:`;
  } else if (!fields) {
    notice = `The “${String(tag)}” variant has no presentable fields; raw value:`;
  }
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const extra = record && fields ? Object.keys(record).filter((key) => key !== tagKey && !Object.hasOwn(fields, key)) : [];
  const variantNote = variant?.description;
  // A variant with nothing beneath its tag (perspective) needs no empty group.
  const grouped = fields && record && (children.length > 0 || extra.length > 0 || variantNote);
  return (
    <>
      <FieldRow
        label={label}
        schema={schema}
        hint={hint}
        value={tag}
        path={path}
        ctx={ctx}
        claim={claim}
        notice={notice === null ? undefined : { text: notice, value }}
        noteId={variantNote ? `${id}-variant` : undefined}
        implied={implied}
      />
      {grouped ? (
        <div className="field-group field-group--variant" role="group" aria-labelledby={`${id}-label`} style={depthStyle(ctx.depth ?? 0)}>
          {variantNote ? (
            <p className="field-group__note field__help" id={`${id}-variant`}>
              {variantNote}
            </p>
          ) : null}
          {children.map((key) => (
            <FieldView key={key} name={key} schema={fields[key]!} value={record[key]} path={[...path, key]} ctx={ctx} />
          ))}
          {extra.map((key) => (
            <ExtraField key={key} name={key} owner={`the “${String(tag)}” ${label.toLowerCase()}`} value={record[key]} />
          ))}
        </div>
      ) : null}
    </>
  );
}

function objectProperties(schema: FieldSchema | undefined): Readonly<Record<string, FieldSchema>> | undefined {
  return schema?.type === 'object' && 'properties' in schema
    ? (schema.properties as Readonly<Record<string, FieldSchema>>)
    : undefined;
}

/**
 * How an absent value is authored, when the schema allows it. A missing value is
 * only valid for an optional field and an explicit null only for a nullable one;
 * anything else is left to the normal type check, which reports a mismatch.
 */
export type UnsetKind = 'omitted' | 'null';

export function unsetKind(schema: FieldSchema, value: unknown): UnsetKind | null {
  if (value === undefined) return schema.optional === true ? 'omitted' : null;
  if (value === null) return schema.nullable === true ? 'null' : null;
  return null;
}

/** True for a value shown as a row of axis cells (2 to 4 numbers). */
function isVectorValue(schema: FieldSchema, value: unknown): boolean {
  return schema.type === 'array' && Array.isArray(value) && value.length >= 3 && value.every((item) => typeof item === 'number');
}

export function FieldRow({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
  unset,
  claim,
  notice,
  noteId,
  implied,
}: {
  label: string;
  schema: FieldSchema;
  hint: FieldHint;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
  unset?: UnsetKind | undefined;
  /** Which diagnostic paths this row presents. Default: its own path and anything beneath it. */
  claim?: ((target: string) => boolean) | undefined;
  /** Replaces the value with an explicit mismatch notice. */
  notice?: { text: ReactNode; value: unknown } | undefined;
  /** Extra visible text that describes the value (e.g. the selected variant). */
  noteId?: string | undefined;
  /** The value is a documented default for an omitted field; this sentence says so. */
  implied?: string | undefined;
}) {
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const messageId = useId();
  const here = pointer(path);
  const owns = claim ?? ((target: string) => target === here || target.startsWith(`${here}/`));
  const problems = ctx.diagnostics.filter((d) => d.path !== null && owns(d.path));
  const severity = severityOf(problems);
  const errorPaths = new Set(problems.filter((d) => d.severity === 'error').map((d) => d.path ?? ''));
  const description = schema.description ?? hint.description;
  const descriptionId = description ? `${id}-description` : undefined;
  const tag: FieldStatus | null =
    notice || unset ? null : implied ? { short: 'Default', long: implied } : (ctx.value && hint.status?.(ctx.value, ctx.schema)) || null;
  const tagId = tag ? `${id}-tag` : undefined;
  const describedBy =
    [descriptionId, tagId, noteId, problems.length > 0 ? messageId : undefined].filter(Boolean).join(' ') || undefined;
  const unit = unitOf(schema);
  const vectorUnit = schema.type === 'array' && !unset && !notice ? unit : undefined;
  const vector = !unset && !notice && isVectorValue(schema, value);
  return (
    // The hover tooltip lives on the row, never on the <label>: WebKit names a field
    // from its label's title attribute, which would replace the visible label with
    // the description (seen in the native AX tree). The description is announced
    // through aria-describedby instead.
    <div
      className={`field${severity ? ` field--${severity}` : ''}${vector ? ' field--vector' : ''}`}
      title={description ?? label}
    >
      {/* A real <label> (plus aria-labelledby for non-input controls) keeps the name robust even when clipped. */}
      <label className="field__label" id={`${id}-label`} htmlFor={id}>
        {label}
        {vectorUnit ? (
          <span className="field__unit">
            <span className="visually-hidden">, in</span> {vectorUnit}
          </span>
        ) : null}
      </label>
      {description ? (
        <span id={descriptionId} className="visually-hidden">
          {description}
        </span>
      ) : null}
      {tag ? (
        <span id={tagId} className="visually-hidden">
          {tag.long}
        </span>
      ) : null}
      <div className="field__value">
        {notice ? (
          <Notice id={id} value={notice.value} describedBy={describedBy}>
            {notice.text}
          </Notice>
        ) : unset ? (
          <UnsetControl id={id} schema={schema} unset={unset} describedBy={describedBy} />
        ) : (
          <ValueControl
            id={id}
            path={path}
            ctx={ctx}
            schema={schema}
            unit={unit}
            tag={tag}
            variants={hint.variants}
            variantIcons={hint.variantIcons}
            axes={hint.axes}
            value={value}
            invalid={severity === 'error'}
            errorPaths={errorPaths}
            describedBy={describedBy}
            identifier={hint.identifier === true}
          />
        )}
      </div>
      {problems.length > 0 ? <ProblemList id={messageId} problems={problems} here={here} /> : null}
    </div>
  );
}

export function severityOf(problems: readonly Diagnostic[]): 'error' | 'warning' | null {
  return problems.some((d) => d.severity === 'error')
    ? 'error'
    : problems.some((d) => d.severity === 'warning')
      ? 'warning'
      : null;
}

/** Engine diagnostics under a row; a deeper path than the row's own is shown exactly. */
export function ProblemList({ id, problems, here }: { id: string; problems: readonly Diagnostic[]; here: string }) {
  return (
    <ul className="field__problems" id={id}>
      {problems.map((d) => (
        <li key={d.id} className={`field__problem field__problem--${d.severity}`}>
          <Icon name={d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info'} size={12} />
          <span>
            {d.message}
            {d.path !== here ? <code className="field__problem-path">{breakAfterSlashes(d.path ?? '')}</code> : null}
          </span>
        </li>
      ))}
    </ul>
  );
}

export function unitOf(schema: FieldSchema): string | undefined {
  return 'x-incant-unit' in schema ? schema['x-incant-unit'] : undefined;
}

/** A click anywhere in a value box focuses its input, as it would in a text field. */
function focusOwnInput(event: MouseEvent<HTMLElement>) {
  const input = event.currentTarget.querySelector('input');
  if (!input || event.target === input) return;
  event.preventDefault();
  input.focus();
}

/** Width of a monospace value in `ch`, so a unit can sit right after the number. */
function charsStyle(text: string): CSSProperties {
  return { width: `calc(${Math.max(1, text.length)}ch + 1px)` };
}

function ValueControl({
  id,
  path,
  ctx,
  schema,
  unit,
  tag,
  variants,
  variantIcons,
  axes,
  value,
  invalid,
  errorPaths,
  describedBy,
  identifier,
}: {
  id: string;
  path: readonly string[];
  ctx: FieldContext;
  schema: FieldSchema;
  unit: string | undefined;
  tag: FieldStatus | null;
  variants: Readonly<Record<string, string>> | undefined;
  variantIcons: Readonly<Record<string, IconName>> | undefined;
  axes: readonly AxisName[] | undefined;
  value: unknown;
  invalid: boolean;
  errorPaths: ReadonlySet<string>;
  describedBy: string | undefined;
  identifier: boolean;
}) {
  const fieldKey = path[path.length - 1] ?? '';
  const here = pointer(path);
  const labelledBy = `${id}-label`;
  const common = {
    id,
    'aria-labelledby': labelledBy,
    readOnly: true,
    'aria-readonly': true,
    'aria-invalid': invalid || undefined,
    'aria-describedby': describedBy,
  } as const;
  switch (schema.type) {
    case 'string': {
      if (typeof value !== 'string') return <Mismatch id={id} expected="text" value={value} />;
      const format = 'format' in schema ? schema.format : undefined;
      if (format === 'color') {
        return (
          <span className="control control--color" onMouseDown={focusOwnInput}>
            <span className="swatch" style={{ background: value }} aria-hidden="true" />
            <input {...common} className="control__input mono" value={value} />
          </span>
        );
      }
      if (identifier) {
        // The item's own stable ID: monospace like a reference, but no link icon (it points nowhere).
        return (
          <span className="control control--ref control--identifier" onMouseDown={focusOwnInput}>
            <input {...common} className="control__input mono" value={value} title={value} />
          </span>
        );
      }
      const reference = ctx.refs?.(value) ?? null;
      if (format === 'asset-ref' || format === 'ulid' || reference || looksLikeUlid(value)) {
        return <ReferenceControl common={common} value={value} reference={reference} linked={format === 'asset-ref' || format === 'ulid'} />;
      }
      return <input {...common} className="control control__input" value={value} />;
    }
    case 'number':
    case 'integer': {
      const widget = 'x-incant-widget' in schema ? schema['x-incant-widget'] : undefined;
      if (widget === 'collision-mask') {
        if (!isMask(value)) return <Mismatch id={id} expected="a 32-bit group mask" value={value} />;
        return <MaskControl common={common} value={value} />;
      }
      if (typeof value !== 'number') return <Mismatch id={id} expected="a number" value={value} />;
      const text = formatNumber(value);
      return (
        <span className={`control control--number${unit || tag ? ' has-suffix' : ''}`} onMouseDown={focusOwnInput}>
          <input {...common} className="control__input mono" value={text} title={exactTitle(value)} style={unit || tag ? charsStyle(text) : undefined} />
          {unit ? <span className="control__unit">{unit}</span> : null}
          <Tag tag={tag} />
        </span>
      );
    }
    case 'tagged-union': {
      // The union row shows the selected variant's tag; UnionView only passes a recognized one.
      if (typeof value !== 'string') return <Mismatch id={id} expected="a variant name" value={value} />;
      const shown = variants && Object.hasOwn(variants, value) ? variants[value]! : value;
      const glyph = variantIcons && Object.hasOwn(variantIcons, value) ? variantIcons[value]! : null;
      if (!tag && !glyph) return <input {...common} className="control control__input" value={shown} />;
      return (
        <span className="control control--choice" onMouseDown={focusOwnInput}>
          {glyph ? <Icon name={glyph} size={14} className="control__glyph" /> : null}
          <input {...common} className="control__input" value={shown} />
          <Tag tag={tag} />
        </span>
      );
    }
    case 'boolean': {
      if (typeof value !== 'boolean') return <Mismatch id={id} expected="on or off" value={value} />;
      return (
        <span
          id={id}
          className={`control control--bool${value ? ' is-on' : ''}`}
          role="checkbox"
          aria-labelledby={labelledBy}
          aria-checked={value}
          aria-readonly="true"
          aria-describedby={describedBy}
          tabIndex={0}
        >
          <span className="control--bool__box" aria-hidden="true" />
          {value ? 'On' : 'Off'}
        </span>
      );
    }
    case 'object':
      return <Mismatch id={id} expected="an object" value={value} />;
    case 'x-json':
      // An open object's list or null: the authored JSON, in full on hover.
      return <input {...common} className="control control__input control--json mono" value={safeJson(value)} title={JSON.stringify(value) ?? 'undefined'} />;
    case 'array': {
      if (!('items' in schema)) return <Unsupported id={id} type="array without items" value={value} />;
      // Lists of records are presented by ObjectListView; only a non-array value reaches here.
      if (isObjectList(schema)) return <Mismatch id={id} expected="a list" value={value} />;
      if (schema.items.type === 'string') return <Mismatch id={id} expected="a list of text" value={value} />;
      if (schema.items.type !== 'number' && schema.items.type !== 'integer') {
        return <Unsupported id={id} type={`array of ${schema.items.type}`} value={value} />;
      }
      if (!Array.isArray(value) || !value.every((item) => typeof item === 'number')) {
        return <Mismatch id={id} expected="a list of numbers" value={value} />;
      }
      // Named entries (width, height) are not spatial axes: a list of another
      // length cannot be labelled honestly, so it is shown raw.
      if (axes && value.length !== axes.length) {
        const names = listAll(axes.map((axis) => axis.name.toLowerCase()));
        return <Mismatch id={id} expected={`${axes.length} numbers (${names})`} value={value} />;
      }
      const widget = schema['x-incant-widget'];
      const channels = axes ?? arrayChannels(widget, fieldKey, value.length);
      const preview = (widget === 'rgb' || widget === 'rgba') && channels ? linearPreview(value) : null;
      return (
        <span
          className={`control control--vector${preview ? ' has-swatch' : ''}`}
          role="group"
          aria-labelledby={labelledBy}
          id={id}
          data-count={value.length}
          // An error on the whole vector (a non-unit quaternion) outlines the box once.
          data-invalid={errorPaths.has(here) ? 'all' : undefined}
        >
          {preview ? <span className="vector__swatch" style={{ background: preview }} aria-hidden="true" title={`Preview of linear RGB ${value.map(formatNumber).join(', ')}, shown in sRGB`} /> : null}
          {value.map((item, index) => (
            <label key={index} className="vector__axis">
              {/* Tint classes stay positional: R/G/B share the X/Y/Z red/green/blue tints.
                  Named entries such as W/H carry no spatial meaning, so they stay neutral. */}
              <span
                className={`vector__axis-name ${axes ? 'axis--plain' : `axis--${AXES[index]?.toLowerCase() ?? 'n'}`}`}
                aria-hidden={channels ? true : undefined}
              >
                {channels?.[index]?.short ?? AXES[index] ?? index}
              </span>
              <input
                id={fieldDomId(ctx.entity, ctx.component, [...path, String(index)])}
                className="control__input mono"
                aria-label={channels?.[index]?.name}
                readOnly
                aria-readonly
                aria-invalid={errorPaths.has(here) || errorPaths.has(`${here}/${index}`) || undefined}
                aria-describedby={describedBy}
                value={formatNumber(item)}
                title={exactTitle(item)}
              />
            </label>
          ))}
        </span>
      );
    }
    default:
      return <Unsupported id={id} type={schema.type} value={value} />;
  }
}

/**
 * A stable ID that names something in the project. When the ID exactly matches an
 * asset or entity, its icon and name lead the box and the authored ID follows in
 * monospace; the name is also spoken. Other IDs show as authored.
 */
function ReferenceControl({
  common,
  value,
  reference,
  linked,
}: {
  common: InputHTMLAttributes<HTMLInputElement> & { readonly id: string };
  value: string;
  reference: Reference | null;
  linked: boolean;
}) {
  const spokenId = `${common.id}-ref`;
  return (
    <span className={`control control--ref${reference ? ' is-resolved' : ''}`} onMouseDown={focusOwnInput}>
      {reference ? (
        <Icon name={iconForKind(reference.kind)} size={12} className={`kind--${reference.kind}`} />
      ) : linked ? (
        <Icon name="link" size={12} />
      ) : null}
      {reference ? (
        <span className="ref__name" title={reference.name} aria-hidden="true">
          {reference.name}
        </span>
      ) : null}
      <input
        {...common}
        aria-describedby={[reference ? spokenId : undefined, common['aria-describedby']].filter(Boolean).join(' ') || undefined}
        className="control__input mono"
        value={value}
        title={value}
      />
      {reference ? (
        <span id={spokenId} className="visually-hidden">
          {reference.spoken}
        </span>
      ) : null}
    </span>
  );
}

/** Linear RGB (0..1, clamped) as an sRGB CSS colour, for a swatch beside the channels. */
function linearPreview(value: readonly number[]): string | null {
  if (value.length < 3 || !value.every((n) => Number.isFinite(n))) return null;
  const encode = (c: number) => {
    const v = Math.min(1, Math.max(0, c));
    return Math.round(255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055));
  };
  return `rgb(${encode(value[0]!)}, ${encode(value[1]!)}, ${encode(value[2]!)})`;
}

/**
 * A 32-bit collision group mask. The raw integer stays the focusable value; a
 * short summary sits where a unit would, and a strip of 32 cells (group 1 on the
 * left, in four bytes) shows which groups are set. The full list is spoken.
 */
function MaskControl({
  common,
  value,
}: {
  common: InputHTMLAttributes<HTMLInputElement> & { readonly id: string };
  value: number;
}) {
  const spokenId = `${common.id}-groups`;
  const set = new Set(maskGroups(value));
  const description = maskDescription(value);
  const text = String(value);
  return (
    <span className="control control--mask">
      <span className="control--number has-suffix" onMouseDown={focusOwnInput}>
        <input
          {...common}
          aria-describedby={[spokenId, common['aria-describedby']].filter(Boolean).join(' ')}
          className="control__input mono"
          value={text}
          style={charsStyle(text)}
        />
        <span className="control__unit control--mask__summary" aria-hidden="true">
          {maskSummary(value)}
        </span>
      </span>
      <span id={spokenId} className="visually-hidden">
        {description}
      </span>
      <span className="mask-strip" aria-hidden="true" title={description}>
        {Array.from({ length: MASK_BITS / 8 }, (_, byte) => (
          <span key={byte} className="mask-strip__byte">
            {Array.from({ length: 8 }, (_, bit) => {
              const group = byte * 8 + bit + 1;
              return <span key={group} className={`mask-strip__cell${set.has(group) ? ' is-set' : ''}`} />;
            })}
          </span>
        ))}
      </span>
    </span>
  );
}

/**
 * Quiet, read-only presentation of an optional value that is not authored. An
 * optional settings group (an object, such as a light's shadows) is a feature
 * that is off while absent; an optional scalar is simply not set. The accessible
 * description says which authored form produced it, so null and omission stay
 * distinguishable without adding visual noise.
 */
function UnsetControl({
  id,
  schema,
  unset,
  describedBy,
}: {
  id: string;
  schema: FieldSchema;
  unset: UnsetKind;
  describedBy: string | undefined;
}) {
  const hintId = `${id}-unset`;
  const text = schema.type === 'object' ? 'Off' : 'Not set';
  const hint =
    unset === 'null'
      ? 'Optional value, set to null in the document.'
      : 'Optional value, not present in the document.';
  return (
    <span className="control control--unset" data-unset={unset} title={hint}>
      <input
        id={id}
        className="control__input"
        aria-labelledby={`${id}-label`}
        aria-describedby={[hintId, describedBy].filter(Boolean).join(' ')}
        readOnly
        aria-readonly
        value={text}
      />
      <span id={hintId} className="visually-hidden">
        {hint}
      </span>
    </span>
  );
}

const AXES = ['X', 'Y', 'Z', 'W'] as const;
const COLOR_CHANNELS = [
  { short: 'R', name: 'Red' },
  { short: 'G', name: 'Green' },
  { short: 'B', name: 'Blue' },
  { short: 'A', name: 'Alpha' },
] as const;

/**
 * Colour arrays read as channels, not spatial axes. Native schemas carry no
 * widget hint for colour yet, so a 3- or 4-number array whose key names a
 * colour ("color", "baseColor", "emissive_color") is presented as R/G/B(/A).
 * Presentation only: values and commands are unchanged.
 */
export function arrayChannels(
  widget: string | undefined,
  key: string,
  length: number,
): readonly { short: string; name: string }[] | null {
  const hinted = widget === 'color' || widget === 'rgb' || widget === 'rgba';
  const named = widget === undefined && /colou?r$/i.test(key);
  return (hinted || named) && (length === 3 || length === 4) ? COLOR_CHANNELS.slice(0, length) : null;
}

/** Visible short form of a status; the full sentence is in the field's accessible description. */
function Tag({ tag }: { tag: FieldStatus | null }) {
  if (!tag) return null;
  return (
    <span className="control__tag" aria-hidden="true" title={tag.long}>
      {tag.short}
    </span>
  );
}

export function Notice({
  id,
  children,
  value,
  describedBy,
}: {
  id?: string | undefined;
  children: ReactNode;
  value: unknown;
  describedBy?: string | undefined;
}) {
  // Focusable when it stands in for a field, so keyboard users (and Problems links) reach it.
  return (
    <span
      className="control control--notice"
      id={id}
      role="group"
      aria-labelledby={id ? `${id}-label` : undefined}
      aria-describedby={describedBy}
      tabIndex={id ? 0 : undefined}
    >
      <span className="control--notice__text">
        <Icon name="warning" size={12} />
        <span>{children}</span>
      </span>
      <code className="control--notice__raw">{safeJson(value)}</code>
    </span>
  );
}

export function Mismatch({ id, expected, value }: { id?: string | undefined; expected: string; value: unknown }) {
  return (
    <Notice id={id} value={value}>
      Cannot show as {expected}; raw value:
    </Notice>
  );
}

function Unsupported({ id, type, value }: { id?: string | undefined; type: string; value: unknown }) {
  return (
    <Notice id={id} value={value}>
      {type === 'unsupported' ? 'This field’s type is not supported yet; raw value:' : `Unsupported field type “${type}”; raw value:`}
    </Notice>
  );
}
