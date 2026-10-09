import { useId } from 'react';
import type { InputHTMLAttributes, ReactNode } from 'react';
import type { Diagnostic, FieldSchema } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import type { FieldHint } from './presentation';
import { MASK_BITS, fieldHint, isMask, maskDescription, maskGroups, maskSummary } from './presentation';

/** JSON pointer for a property path, matching Diagnostic.path. */
export function pointer(path: readonly string[]): string {
  return path.map((part) => `/${part.replace(/~/g, '~0').replace(/\//g, '~1')}`).join('');
}

export function fieldDomId(entity: string, component: string, path: readonly string[]): string {
  return `field-${entity}-${component}${pointer(path)}`.replace(/[^A-Za-z0-9_-]/g, '_');
}

export interface FieldContext {
  readonly entity: string;
  readonly component: string;
  /** Diagnostics for this component, matched to fields by path prefix. */
  readonly diagnostics: readonly Diagnostic[];
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
  if (unset) {
    return <FieldRow label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} unset={unset} />;
  }
  if (schema.type === 'tagged-union' && 'variants' in schema) {
    return <UnionView label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
  }
  if (schema.type === 'object' && 'properties' in schema) {
    const record = isRecord(value) ? value : null;
    return (
      <fieldset className="field-group">
        <legend className="field-group__legend">{label}</legend>
        {record ? (
          Object.entries(schema.properties).map(([key, child]) => (
            <FieldView key={key} name={key} schema={child} value={record[key]} path={[...path, key]} ctx={ctx} />
          ))
        ) : (
          <Mismatch expected="object" value={value} />
        )}
      </fieldset>
    );
  }
  return <FieldRow label={label} schema={schema} hint={hint} value={value} path={path} ctx={ctx} />;
}

type UnionSchema = Extract<FieldSchema, { type: 'tagged-union' }>;

/** "box", "sphere" and "capsule" -> "box, sphere or capsule". */
function listChoices(choices: readonly string[]): string {
  return choices.length < 2 ? (choices[0] ?? 'nothing') : `${choices.slice(0, -1).join(', ')} or ${choices.at(-1)}`;
}

/**
 * A tagged union (such as Collider.shape) is one row naming the variant, with
 * that variant's fields grouped beneath it. A variant is selected only when the
 * value is a record whose tag names a known variant; a missing, non-string or
 * unknown tag is a visible mismatch and no variant (and no default) is shown.
 * The engine still validates: this only refuses to guess.
 */
function UnionView({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: UnionSchema;
  hint: FieldHint;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
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
      />
      {fields && record ? (
        <div className="field-group field-group--variant" role="group" aria-labelledby={`${id}-label`}>
          {variantNote ? (
            <p className="field-group__note" id={`${id}-variant`}>
              {variantNote}
            </p>
          ) : null}
          {children.map((key) => (
            <FieldView key={key} name={key} schema={fields[key]!} value={record[key]} path={[...path, key]} ctx={ctx} />
          ))}
          {extra.map((key) => (
            <p key={key} className="component__notice">
              <span>
                <Icon name="warning" size={12} /> Field <code>{key}</code> is not part of the “{String(tag)}” {label.toLowerCase()}:{' '}
                <code>{safeJson(record[key])}</code>
              </span>
            </p>
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

function unescapePointer(part: string): string {
  return part.replace(/~1/g, '/').replace(/~0/g, '~');
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

function FieldRow({
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
}) {
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const messageId = useId();
  const here = pointer(path);
  const owns = claim ?? ((target: string) => target === here || target.startsWith(`${here}/`));
  const problems = ctx.diagnostics.filter((d) => d.path !== null && owns(d.path));
  const severity = problems.some((d) => d.severity === 'error')
    ? 'error'
    : problems.some((d) => d.severity === 'warning')
      ? 'warning'
      : null;
  const errorPaths = new Set(problems.filter((d) => d.severity === 'error').map((d) => d.path ?? ''));
  const describedBy = [noteId, problems.length > 0 ? messageId : undefined].filter(Boolean).join(' ') || undefined;
  const unit = unitOf(schema);
  const vectorUnit = schema.type === 'array' && !unset && !notice ? unit : undefined;
  return (
    <div className={`field${severity ? ` field--${severity}` : ''}`}>
      {/* A real <label> (plus aria-labelledby for non-input controls) keeps the name robust even when clipped. */}
      <label
        className="field__label"
        id={`${id}-label`}
        htmlFor={id}
        title={schema.description ?? hint.description ?? label}
      >
        {label}
        {vectorUnit ? (
          <span className="field__unit">
            <span className="visually-hidden">, in </span>
            {vectorUnit}
          </span>
        ) : null}
      </label>
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
            value={value}
            invalid={severity === 'error'}
            errorPaths={errorPaths}
            describedBy={describedBy}
          />
        )}
      </div>
      {problems.length > 0 ? (
        <ul className="field__problems" id={messageId}>
          {problems.map((d) => (
            <li key={d.id} className={`field__problem field__problem--${d.severity}`}>
              <Icon name={d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info'} size={12} />
              <span>
                {d.message}
                {d.path !== here ? <code className="field__problem-path">{d.path}</code> : null}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function unitOf(schema: FieldSchema): string | undefined {
  return 'x-incant-unit' in schema ? schema['x-incant-unit'] : undefined;
}

function ValueControl({
  id,
  path,
  ctx,
  schema,
  unit,
  value,
  invalid,
  errorPaths,
  describedBy,
}: {
  id: string;
  path: readonly string[];
  ctx: FieldContext;
  schema: FieldSchema;
  unit: string | undefined;
  value: unknown;
  invalid: boolean;
  errorPaths: ReadonlySet<string>;
  describedBy: string | undefined;
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
          <span className="control control--color">
            <span className="swatch" style={{ background: value }} aria-hidden="true" />
            <input {...common} className="control__input mono" value={value} />
          </span>
        );
      }
      if (format === 'asset-ref' || format === 'ulid') {
        return (
          <span className="control control--ref">
            <Icon name="link" size={12} />
            <input {...common} className="control__input mono" value={value} title={value} />
          </span>
        );
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
      return (
        <span className="control control--number">
          <input {...common} className="control__input mono" value={formatNumber(value)} />
          {unit ? <span className="control__unit">{unit}</span> : null}
        </span>
      );
    }
    case 'tagged-union': {
      // The union row shows the selected variant's tag; UnionView only passes a recognized one.
      if (typeof value !== 'string') return <Mismatch id={id} expected="a variant name" value={value} />;
      return <input {...common} className="control control__input" value={value} />;
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
    case 'array': {
      if (!('items' in schema)) return <Unsupported id={id} type="array without items" value={value} />;
      if (!Array.isArray(value) || !value.every((item) => typeof item === 'number')) {
        return <Mismatch id={id} expected="a list of numbers" value={value} />;
      }
      const channels = arrayChannels(schema['x-incant-widget'], fieldKey, value.length);
      return (
        <span className="control control--vector" role="group" aria-labelledby={labelledBy} id={id}>
          {value.map((item, index) => (
            <label key={index} className="vector__axis">
              {/* Tint classes stay positional: R/G/B share the X/Y/Z red/green/blue tints. */}
              <span className={`vector__axis-name axis--${AXES[index]?.toLowerCase() ?? 'n'}`} aria-hidden={channels ? true : undefined}>
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
  return (
    <span className="control control--mask">
      <span className="control--number">
        <input
          {...common}
          aria-describedby={[spokenId, common['aria-describedby']].filter(Boolean).join(' ')}
          className="control__input mono"
          value={String(value)}
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

function formatNumber(value: number): string {
  if (Number.isInteger(value)) return String(value);
  return String(Number(value.toFixed(4)));
}

function Notice({
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
        {children}
      </span>
      <code className="control--notice__raw">{safeJson(value)}</code>
    </span>
  );
}

function Mismatch({ id, expected, value }: { id?: string | undefined; expected: string; value: unknown }) {
  return (
    <Notice id={id} value={value}>
      Cannot show as {expected}; raw value:
    </Notice>
  );
}

function Unsupported({ id, type, value }: { id?: string | undefined; type: string; value: unknown }) {
  return (
    <Notice id={id} value={value}>
      Unsupported field type “{type}”; raw value:
    </Notice>
  );
}

/** "translation" -> "Translation", "castShadows" / "cast_shadows" -> "Cast shadows". */
export function humanize(key: string): string {
  const words = key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .trim()
    .toLowerCase();
  return words ? words[0]!.toUpperCase() + words.slice(1) : key;
}

export function safeJson(value: unknown): string {
  try {
    const text = JSON.stringify(value);
    return text === undefined ? 'undefined' : text.length > 160 ? `${text.slice(0, 157)}…` : text;
  } catch {
    return '[unserializable]';
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
