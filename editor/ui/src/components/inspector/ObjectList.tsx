import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { Diagnostic, FieldSchema } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import type { IconName } from '../../icons/Icon';
import { mountedIndices, revealSpan, rowRange, useViewport } from '../../shell/windowing';
import type { FieldContext } from './FieldView';
import { FieldView, Notice, ProblemList, depthStyle, fieldDomId, orderedProperties, severityOf } from './FieldView';
import { formatNumber, humanize, isRecord, pointer, safeJson, unescapePointer } from './format';
import type { FieldHint, NameOf } from './presentation';
import { fieldHint } from './presentation';
import { revealIndex, useFieldReveal } from './reveal';

type ObjectSchema = Extract<FieldSchema, { type: 'object' }>;
type UnionSchema = Extract<FieldSchema, { type: 'tagged-union' }>;

/** Height of one list row; the CSS row height matches. */
const ROW = 26;
/** Rows shown before the list box scrolls. */
const VISIBLE_ROWS = 6.5;

/** An array whose items are records (Collider compound parts), not a numeric vector. */
export function isObjectList(schema: FieldSchema): boolean {
  return schema.type === 'array' && 'items' in schema && objectSchema(schema.items) !== undefined;
}

/**
 * Read-only master/detail presentation of a list of records. The list is one
 * Tab stop (a listbox: arrow keys, Home/End, Page Up/Down; selection follows
 * focus) in a bounded, scrollable box, so 64 parts or 1,024 sources neither flood
 * the panel nor the tab sequence; only the rows near the scroll position are
 * mounted, plus the selected one. Only the selected item's fields are rendered,
 * below the list, with exact diagnostic paths (`/shape/parts/3/shape/radius`).
 * Items with problems are marked in the list; the first one starts selected.
 *
 * The selected item's fields are not boxed: a captioned hairline opens them and
 * they continue at the list's own depth, so their labels and values line up with
 * the rest of the component.
 *
 * Presentation only: nothing here validates, reorders or edits. A malformed
 * item stays listed in place and its raw value is shown.
 */
export function ObjectListView({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: Extract<FieldSchema, { type: 'array' }>;
  hint: FieldHint;
  value: readonly unknown[];
  path: readonly string[];
  ctx: FieldContext;
}) {
  const item = objectSchema(schema.items)!;
  const here = pointer(path);
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const summaryId = `${id}-summary`;
  const keysId = useId();
  const noun = singular(label);
  const listRef = useRef<HTMLDivElement>(null);
  const issues = value.map((_, index) => itemProblems(ctx.diagnostics, `${here}/${index}`));
  const identity = Object.keys(item.properties).find((key) => fieldHint(ctx.component, pointer([...path, '0', key])).identifier);
  const ids = value.map((entry) => (identity && isRecord(entry) && typeof entry[identity] === 'string' ? (entry[identity] as string) : null));
  const counts = new Map<string, number>();
  for (const key of ids) if (key !== null) counts.set(key, (counts.get(key) ?? 0) + 1);
  // Malformed/duplicate identifiers still get independent rows. Valid stable
  // identities preserve the selected part and its DOM focus across reordering.
  const keys = ids.map((key, index) => (key !== null && counts.get(key) === 1 ? `id:${key}` : `index:${index}`));
  const [selected, setSelected] = useState(() => {
    const error = issues.findIndex((list) => severityOf(list) === 'error');
    const warning = issues.findIndex((list) => severityOf(list) === 'warning');
    const index = error >= 0 ? error : warning >= 0 ? warning : 0;
    return { key: keys[index], index };
  });
  const found = selected.key === undefined ? -1 : keys.indexOf(selected.key);
  const current = found >= 0 ? found : Math.min(selected.index, Math.max(0, value.length - 1));
  const currentKey = keys[current];
  useEffect(() => {
    if (selected.key !== currentKey || selected.index !== current) setSelected({ key: currentKey, index: current });
  }, [current, currentKey, selected.key, selected.index]);
  // The list row owns its own path and anything that does not address a listed item.
  const own = ctx.diagnostics.filter((d) => {
    if (d.path === null) return false;
    if (d.path === here) return true;
    if (!d.path.startsWith(`${here}/`)) return false;
    const index = d.path.slice(here.length + 1).split('/')[0] ?? '';
    return !/^\d+$/.test(index) || Number(index) >= value.length;
  });
  const severity = severityOf(own);
  const description = schema.description ?? hint.description;
  const descriptionId = description ? `${id}-description` : undefined;
  const messageId = useId();
  const nameOf: NameOf = (target) => ctx.refs?.(target)?.name ?? null;
  const summaries = value.map((entry, index) => summarize(item, entry, [...path, String(index)], ctx.component, nameOf));

  const [viewport, attachViewport, remeasure] = useViewport(listRef, ROW * 4);
  const [first, last] = rowRange(viewport, value.length, ROW, 6, 4);
  const mounted = mountedIndices(first, last, value.length, [current]);
  const pendingFocus = useRef<number | null>(null);

  // A first problem past the visible rows starts scrolled into view, inside the list only.
  useEffect(() => {
    const list = listRef.current;
    if (list && list.clientHeight > 0) {
      list.scrollTop = Math.max(0, current * ROW + 4 - (list.clientHeight - ROW) / 2);
      remeasure();
    }
    // Mount only: later selection moves focus and reveals the row itself.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // A keyboard move focuses its row once the row is committed (it is always mounted as
  // the selection). This runs after React's commit, not on an animation frame, so it
  // also holds where WebKit throttles frames.
  useLayoutEffect(() => {
    if (pendingFocus.current !== current) return;
    pendingFocus.current = null;
    listRef.current?.querySelector<HTMLElement>(`[data-index="${current}"]`)?.focus();
  }, [current]);

  const focusItem = (index: number) => {
    const next = Math.max(0, Math.min(value.length - 1, index));
    // Scroll first and measure at once, so the rows around the target mount in the
    // same render as the new selection instead of after a scroll notification.
    revealSpan(listRef.current, next * ROW + 4, ROW);
    remeasure();
    if (next === current) {
      listRef.current?.querySelector<HTMLElement>(`[data-index="${next}"]`)?.focus();
      return;
    }
    pendingFocus.current = next;
    setSelected({ key: keys[next], index: next });
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step: Record<string, number> = { ArrowDown: 1, ArrowUp: -1, PageDown: 5, PageUp: -5 };
    if (event.key in step) focusItem(current + step[event.key]!);
    else if (event.key === 'Home') focusItem(0);
    else if (event.key === 'End') focusItem(value.length - 1);
    else return;
    event.preventDefault();
  };
  useFieldReveal(id, (remaining) => {
    const index = revealIndex(remaining, value.length);
    if (index !== null) focusItem(index);
  });

  return (
    <div className="object-list" data-severity={severity ?? undefined}>
      <div className={`field field--list${severity ? ` field--${severity}` : ''}`} title={description ?? label}>
        {/* "Parts 64" and the breakdown; the sentence form is for assistive tech only. */}
        <span className="field__label" id={`${id}-label`}>
          {label}
          {value.length > 0 ? (
            <span className="object-list__count" aria-hidden="true">
              {value.length}
            </span>
          ) : null}
        </span>
        <span className="object-list__breakdown" aria-hidden="true">
          {listBreakdown(summaries)}
        </span>
        <span className="object-list__summary visually-hidden" id={summaryId}>
          {listSummary(noun, summaries)}
        </span>
        {description ? (
          <span id={descriptionId} className="visually-hidden">
            {description}
          </span>
        ) : null}
        {own.length > 0 ? <ProblemList id={messageId} problems={own} here={here} /> : null}
      </div>
      {value.length === 0 ? (
        <p className="object-list__empty" id={id}>
          No {label.toLowerCase()}.
        </p>
      ) : (
        <>
          <span id={keysId} className="visually-hidden">
            Read-only. Arrow keys choose a {noun.toLowerCase()}; its fields follow the list.
          </span>
          <div
            ref={attachViewport}
            className="object-list__items"
            role="listbox"
            id={id}
            tabIndex={-1}
            aria-labelledby={`${id}-label`}
            aria-describedby={[summaryId, keysId, descriptionId, own.length > 0 ? messageId : undefined].filter(Boolean).join(' ')}
            aria-invalid={severity === 'error' || undefined}
            onKeyDown={onKeyDown}
            style={{ maxHeight: `${Math.min(value.length, VISIBLE_ROWS) * ROW + 8}px` }}
            // Problems links focus the list by its path; hand focus to the selected row.
            onFocus={(event) => {
              if (event.target === event.currentTarget) focusItem(current);
            }}
          >
            <div className="object-list__canvas" role="none" style={{ height: value.length * ROW + 8 }}>
              {mounted.map((index) => {
                const summary = summaries[index]!;
                const problems = issues[index]!;
                const tone = severityOf(problems);
                const errors = problems.filter((d) => d.severity === 'error').length;
                const warnings = problems.filter((d) => d.severity === 'warning').length;
                const counted = [
                  errors ? `${errors} ${errors === 1 ? 'error' : 'errors'}` : '',
                  warnings ? `${warnings} ${warnings === 1 ? 'warning' : 'warnings'}` : '',
                ].filter(Boolean);
                return (
                  <div
                    key={keys[index]}
                    data-index={index}
                    id={fieldDomId(ctx.entity, ctx.component, [...path, String(index)])}
                    className={`object-list__row${tone ? ` object-list__row--${tone}` : ''}`}
                    role="option"
                    aria-selected={index === current}
                    aria-posinset={index + 1}
                    aria-setsize={value.length}
                    tabIndex={index === current ? 0 : -1}
                    title={summary.spoken}
                    style={{ top: index * ROW + 4 }}
                    onFocus={() => setSelected({ key: keys[index], index })}
                    onClick={() => focusItem(index)}
                  >
                    {/*
                     * Name from content: one spoken sentence in words ("half extents", not "½"),
                     * so it holds everything the compact cells show without depending on how a
                     * browser joins grid cells. No aria-label, so the name stays the content.
                     */}
                    <span className="visually-hidden">{[`${noun} ${index + 1}: ${summary.spoken}`, ...counted].join(', ')}</span>
                    <span className="object-list__index mono" aria-hidden="true">
                      {index + 1}
                    </span>
                    <span className={`object-list__tag${summary.unreadable ? ' is-unreadable' : ''}`} aria-hidden="true">
                      {summary.unreadable ? (
                        <Icon name="warning" size={12} />
                      ) : summary.icon ? (
                        <Icon name={summary.icon} size={12} className="object-list__glyph" />
                      ) : null}
                      {summary.tag}
                    </span>
                    <span className="object-list__dims mono" aria-hidden="true">
                      {summary.raw ?? (summary.text !== undefined ? summary.text : <ItemValues parts={summary.parts} />)}
                    </span>
                    {tone ? (
                      <span className={`object-list__status object-list__status--${tone}`} aria-hidden="true">
                        <Icon name={tone === 'error' ? 'error' : tone === 'warning' ? 'warning' : 'info'} size={12} />
                        {errors + warnings > 1 ? errors + warnings : null}
                      </span>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
          <ItemDetail
            key={currentKey}
            noun={noun}
            schema={item}
            value={value[current]}
            index={current}
            count={value.length}
            path={[...path, String(current)]}
            ctx={ctx}
          />
        </>
      )}
    </div>
  );
}

/** The selected item's fields, in the profile's (or schema's) order, with exact paths. */
function ItemDetail({
  noun,
  schema,
  value,
  index,
  count,
  path,
  ctx,
}: {
  noun: string;
  schema: ObjectSchema;
  value: unknown;
  index: number;
  count: number;
  path: readonly string[];
  ctx: FieldContext;
}) {
  const here = pointer(path);
  const captionId = `${fieldDomId(ctx.entity, ctx.component, path)}-detail`;
  const messageId = useId();
  const record = isRecord(value) ? value : null;
  const keys = orderedProperties(schema.properties as Readonly<Record<string, FieldSchema>>, fieldHint(ctx.component, here).order);
  // Item-level problems: the item itself, or a key no row below presents.
  const own = ctx.diagnostics.filter((d) => {
    if (d.path === null) return false;
    if (d.path === here) return true;
    if (!d.path.startsWith(`${here}/`)) return false;
    const key = d.path.slice(here.length + 1).split('/')[0] ?? '';
    return !record || !keys.includes(unescapePointer(key));
  });
  const extra = record ? Object.keys(record).filter((key) => !Object.hasOwn(schema.properties, key)) : [];
  return (
    <div className="object-list__detail" role="group" aria-labelledby={captionId} style={depthStyle(ctx.depth ?? 0)}>
      {/* The document path (/shape/parts/41) is the tooltip and spoken, not a visible column. */}
      <p className="object-list__caption" id={captionId} title={`Document path ${here}`}>
        <span className="object-list__title">
          {noun} {index + 1} <span className="object-list__of">of {count}</span>
        </span>
        <span className="object-list__path visually-hidden">, path {here}</span>
      </p>
      {own.length > 0 ? <ProblemList id={messageId} problems={own} here={here} /> : null}
      {record ? (
        keys.map((key) => (
          <FieldView key={key} name={key} schema={schema.properties[key]!} value={record[key]} path={[...path, key]} ctx={ctx} />
        ))
      ) : (
        <Notice value={value}>
          Cannot show {noun.toLowerCase()} {index + 1} as an object; raw value:
        </Notice>
      )}
      {extra.map((key) => (
        <p key={key} className="component__notice">
          <span>
            <Icon name="warning" size={12} /> Field <code>{key}</code> is not in the {noun.toLowerCase()} schema:{' '}
            <code>{safeJson(record?.[key])}</code>
          </span>
        </p>
      ))}
    </div>
  );
}

interface ItemSummary {
  /** Variant tag, or a short mismatch phrase. */
  readonly tag: string;
  /** Glyph for the variant tag (a collider primitive), when the profile names one. */
  readonly icon?: IconName | undefined;
  readonly unreadable: boolean;
  /** The variant's values, shown compactly ("½ 0.6 × 0.08 × 0.5 m"). */
  readonly parts: readonly ValuePart[];
  /** A profile's compact text, shown instead of the generic values. */
  readonly text?: string;
  /** Raw JSON shown instead of values when the item cannot be read. */
  readonly raw?: string;
  /** Full words for the accessible name and tooltip. */
  readonly spoken: string;
}

/**
 * One-line summary of an item: the profile's own summary when it can read the
 * item; else the tag of its first tagged-union field (a part's shape) and that
 * variant's values, or its first values. Missing or unknown tags are said
 * plainly; the detail shows the raw value.
 */
function summarize(schema: ObjectSchema, value: unknown, path: readonly string[], component: string, nameOf: NameOf): ItemSummary {
  if (!isRecord(value)) {
    return { tag: 'not an object', unreadable: true, parts: [], raw: safeJson(value), spoken: `not an object: ${safeJson(value)}` };
  }
  const custom = fieldHint(component, pointer(path)).summary?.(value, nameOf);
  if (custom) return { tag: custom.tag, unreadable: false, parts: [], text: custom.text, spoken: custom.spoken };
  const unionKey = Object.keys(schema.properties).find((key) => schema.properties[key]!.type === 'tagged-union');
  if (!unionKey) {
    const order = fieldHint(component, pointer(path)).order;
    const parts = valueParts(schema, value, path, component, nameOf, undefined, order).slice(0, 3);
    return { tag: '', unreadable: false, parts, spoken: spokenParts(parts) || 'no values' };
  }
  const union = schema.properties[unionKey] as UnionSchema;
  const inner = value[unionKey];
  const unionHint = fieldHint(component, pointer([...path, unionKey]));
  const noun = (union.title ?? unionHint.title ?? humanize(unionKey)).toLowerCase();
  if (!isRecord(inner)) {
    return { tag: `no ${noun}`, unreadable: true, parts: [], raw: safeJson(inner), spoken: `${noun} cannot be shown: ${safeJson(inner)}` };
  }
  const tag = inner[union.discriminator];
  const disc = union.discriminator;
  if (tag === undefined) return { tag: `no ${disc}`, unreadable: true, parts: [], spoken: `${noun} has no ${disc}` };
  if (typeof tag !== 'string' || !Object.hasOwn(union.variants, tag)) {
    const text = safeJson(tag);
    return { tag: text, unreadable: true, parts: [], spoken: `unknown ${noun} ${disc} ${text}` };
  }
  const variant = objectSchema(union.variants[tag]);
  const parts = variant ? valueParts(variant, inner, [...path, unionKey], component, nameOf, disc) : [];
  const icon = unionHint.variantIcons && Object.hasOwn(unionHint.variantIcons, tag) ? unionHint.variantIcons[tag] : undefined;
  return { tag, icon, unreadable: false, parts, spoken: [tag, spokenParts(parts)].filter(Boolean).join(', ') };
}

interface ValuePart {
  readonly short: string;
  readonly long: string;
  readonly text: string;
  readonly unit: string | undefined;
}

function valueParts(
  schema: ObjectSchema,
  value: Record<string, unknown>,
  path: readonly string[],
  component: string,
  nameOf: NameOf,
  skip?: string,
  order?: readonly string[],
): ValuePart[] {
  const out: ValuePart[] = [];
  const properties = schema.properties as Readonly<Record<string, FieldSchema>>;
  for (const key of orderedProperties(properties, order)) {
    const child = properties[key]!;
    if (key === skip) continue;
    const raw = value[key];
    const unit = 'x-incant-unit' in child ? child['x-incant-unit'] : undefined;
    const hint = fieldHint(component, pointer([...path, key]));
    const long = (child.title ?? hint.title ?? humanize(key)).toLowerCase();
    let text: string | null = null;
    if (typeof raw === 'number') text = formatNumber(raw);
    else if (Array.isArray(raw) && raw.length > 0 && raw.every((v) => typeof v === 'number')) text = raw.map(formatNumber).join(' × ');
    else if (typeof raw === 'string' && child.type === 'string' && !hint.identifier) text = nameOf(raw) ?? raw;
    if (text !== null) out.push({ short: hint.short ?? long, long, text, unit });
  }
  return out;
}

/** One unit for every value, when they all share it ("m"). */
function sharedUnit(parts: readonly ValuePart[]): string | undefined {
  const units = new Set(parts.map((part) => part.unit));
  return units.size === 1 ? [...units][0] : undefined;
}

/** "half extents 0.6 × 0.1 × 0.5 m" or "half height 0.32, radius 0.04 m". */
function spokenParts(parts: readonly ValuePart[]): string {
  const shared = sharedUnit(parts);
  const body = parts.map((part) => `${part.long} ${part.text}${shared === undefined && part.unit ? ` ${part.unit}` : ''}`).join(', ');
  return shared && body ? `${body} ${shared}` : body;
}

/**
 * Visible "½h 0.32 · r 0.04 m" (the row's spoken name uses the full words). A
 * lone value without an abbreviation shows no name at all.
 */
function ItemValues({ parts }: { parts: readonly ValuePart[] }) {
  const shared = sharedUnit(parts);
  const body = parts
    .map((part) => {
      const name = part.short !== part.long ? `${part.short} ` : parts.length > 1 ? `${part.long} ` : '';
      return `${name}${part.text}${shared === undefined && part.unit ? ` ${part.unit}` : ''}`;
    })
    .join(' · ');
  return <>{shared && body ? `${body} ${shared}` : body}</>;
}

/** "3 box, 1 capsule": how many items of each kind, or empty when items have no kind. */
function listBreakdown(summaries: readonly ItemSummary[]): string {
  const tags = new Map<string, number>();
  let unreadable = 0;
  for (const summary of summaries) {
    if (summary.unreadable) unreadable += 1;
    else if (summary.tag) tags.set(summary.tag, (tags.get(summary.tag) ?? 0) + 1);
  }
  return [...[...tags].map(([tag, n]) => `${n} ${tag}`), ...(unreadable ? [`${unreadable} unreadable`] : [])].join(', ');
}

/** "4 parts · 3 box, 1 capsule", spoken with the list. */
function listSummary(noun: string, summaries: readonly ItemSummary[]): string {
  const count = `${summaries.length} ${summaries.length === 1 ? noun.toLowerCase() : `${noun.toLowerCase()}s`}`;
  const breakdown = listBreakdown(summaries);
  return breakdown ? `${count} · ${breakdown}` : count;
}

function itemProblems(diagnostics: readonly Diagnostic[], itemPointer: string): Diagnostic[] {
  return diagnostics.filter((d) => d.path !== null && (d.path === itemPointer || d.path.startsWith(`${itemPointer}/`)));
}

function objectSchema(schema: FieldSchema | undefined): ObjectSchema | undefined {
  return schema?.type === 'object' && 'properties' in schema ? (schema as ObjectSchema) : undefined;
}

/** "Parts" -> "Part"; "Off-mesh links" -> "Off-mesh link"; anything else -> "Item". */
function singular(label: string): string {
  return /[^s]s$/i.test(label) ? label.slice(0, -1) : 'Item';
}
