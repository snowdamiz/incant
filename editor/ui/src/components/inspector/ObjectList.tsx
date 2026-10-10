import { useEffect, useId, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { Diagnostic, FieldSchema } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';
import type { FieldContext } from './FieldView';
import { FieldView, Notice, ProblemList, fieldDomId, formatNumber, humanize, pointer, safeJson, severityOf } from './FieldView';
import type { FieldHint } from './presentation';
import { fieldHint } from './presentation';

type ObjectSchema = Extract<FieldSchema, { type: 'object' }>;
type UnionSchema = Extract<FieldSchema, { type: 'tagged-union' }>;

/** An array whose items are records (Collider compound parts), not a numeric vector. */
export function isObjectList(schema: FieldSchema): boolean {
  return schema.type === 'array' && 'items' in schema && objectSchema(schema.items) !== undefined;
}

/**
 * Read-only master/detail presentation of a list of records. The list is one
 * Tab stop (a listbox: arrow keys, Home/End, Page Up/Down; selection follows
 * focus) in a bounded, scrollable well, so 64 parts neither flood the panel nor
 * the tab sequence. Only the selected item's fields are rendered, below the
 * list, with exact diagnostic paths (`/shape/parts/3/shape/radius`). Items
 * with problems are marked in the list; the first one starts selected.
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
  const [selected, setSelected] = useState(() => {
    const error = issues.findIndex((list) => severityOf(list) === 'error');
    const warning = issues.findIndex((list) => severityOf(list) === 'warning');
    return error >= 0 ? error : warning >= 0 ? warning : 0;
  });
  const current = Math.min(selected, Math.max(0, value.length - 1));
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
  const summaries = value.map((entry, index) => summarize(item, entry, [...path, String(index)], ctx.component));

  // A first problem past the visible rows starts scrolled into view, inside the list only.
  useEffect(() => {
    const list = listRef.current;
    const row = list?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (!list || !row) return;
    if (row.offsetTop < list.scrollTop || row.offsetTop + row.offsetHeight > list.scrollTop + list.clientHeight) {
      list.scrollTop = row.offsetTop - (list.clientHeight - row.offsetHeight) / 2;
    }
    // Mount only: later selection moves focus, and focus scrolls the row itself.
  }, []);

  const focusItem = (index: number) => {
    const next = Math.max(0, Math.min(value.length - 1, index));
    setSelected(next);
    listRef.current?.querySelector<HTMLElement>(`[data-index="${next}"]`)?.focus();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step: Record<string, number> = { ArrowDown: 1, ArrowUp: -1, PageDown: 5, PageUp: -5 };
    if (event.key in step) focusItem(current + step[event.key]!);
    else if (event.key === 'Home') focusItem(0);
    else if (event.key === 'End') focusItem(value.length - 1);
    else return;
    event.preventDefault();
  };

  return (
    <div className="object-list" data-severity={severity ?? undefined}>
      <div className={`field field--list${severity ? ` field--${severity}` : ''}`} title={description ?? label}>
        <span className="field__label" id={`${id}-label`}>
          {label}
        </span>
        <span className="object-list__summary" id={summaryId} title={listSummary(noun, summaries)}>
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
            ref={listRef}
            className="object-list__items"
            role="listbox"
            id={id}
            tabIndex={-1}
            aria-labelledby={`${id}-label`}
            aria-describedby={[summaryId, keysId, descriptionId, own.length > 0 ? messageId : undefined].filter(Boolean).join(' ')}
            aria-invalid={severity === 'error' || undefined}
            onKeyDown={onKeyDown}
            // Problems links focus the list by its path; hand focus to the selected row.
            onFocus={(event) => {
              if (event.target === event.currentTarget) focusItem(current);
            }}
          >
            {summaries.map((summary, index) => {
              const problems = issues[index]!;
              const tone = severityOf(problems);
              const errors = problems.filter((d) => d.severity === 'error').length;
              const warnings = problems.filter((d) => d.severity === 'warning').length;
              const counts = [
                errors ? `${errors} ${errors === 1 ? 'error' : 'errors'}` : '',
                warnings ? `${warnings} ${warnings === 1 ? 'warning' : 'warnings'}` : '',
              ].filter(Boolean);
              return (
                <div
                  key={index}
                  data-index={index}
                  id={fieldDomId(ctx.entity, ctx.component, [...path, String(index)])}
                  className={`object-list__row${tone ? ` object-list__row--${tone}` : ''}`}
                  role="option"
                  aria-selected={index === current}
                  tabIndex={index === current ? 0 : -1}
                  title={summary.spoken}
                  onFocus={() => setSelected(index)}
                  onClick={() => focusItem(index)}
                >
                  {/*
                   * Name from content: one spoken sentence in words ("half extents", not "½"),
                   * so it holds everything the compact cells show without depending on how a
                   * browser joins grid cells. No aria-label, so the name stays the content.
                   */}
                  <span className="visually-hidden">{[`${noun} ${index + 1}: ${summary.spoken}`, ...counts].join(', ')}</span>
                  <span className="object-list__index mono" aria-hidden="true">
                    {index + 1}
                  </span>
                  <span className={`object-list__tag${summary.unreadable ? ' is-unreadable' : ''}`} aria-hidden="true">
                    {summary.unreadable ? <Icon name="warning" size={12} /> : null}
                    {summary.tag}
                  </span>
                  <span className="object-list__dims mono" aria-hidden="true">
                    {summary.raw ?? <ItemValues parts={summary.parts} />}
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
          <ItemDetail
            key={current}
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
  const keys = itemKeys(schema, fieldHint(ctx.component, here).order);
  // Item-level problems: the item itself, or a key no row below presents.
  const own = ctx.diagnostics.filter((d) => {
    if (d.path === null) return false;
    if (d.path === here) return true;
    if (!d.path.startsWith(`${here}/`)) return false;
    const key = d.path.slice(here.length + 1).split('/')[0] ?? '';
    return !record || !keys.includes(key.replace(/~1/g, '/').replace(/~0/g, '~'));
  });
  const extra = record ? Object.keys(record).filter((key) => !Object.hasOwn(schema.properties, key)) : [];
  return (
    <div className="object-list__detail" role="group" aria-labelledby={captionId}>
      <p className="object-list__caption" id={captionId}>
        <span>
          {noun} {index + 1} <span className="object-list__of">of {count}</span>
        </span>
        <code className="object-list__path" title={`Document path ${here}`}>
          <span className="visually-hidden">, path </span>
          {here}
        </code>
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
  readonly unreadable: boolean;
  /** The variant's values, shown compactly ("½ 0.6 × 0.08 × 0.5 m"). */
  readonly parts: readonly ValuePart[];
  /** Raw JSON shown instead of values when the item cannot be read. */
  readonly raw?: string;
  /** Full words for the accessible name and tooltip. */
  readonly spoken: string;
}

/**
 * One-line summary of an item: the tag of its first tagged-union field (a
 * part's shape) and that variant's values, or else its first scalar values.
 * Missing or unknown tags are said plainly; the detail shows the raw value.
 */
function summarize(schema: ObjectSchema, value: unknown, path: readonly string[], component: string): ItemSummary {
  if (!isRecord(value)) {
    return { tag: 'not an object', unreadable: true, parts: [], raw: safeJson(value), spoken: `not an object: ${safeJson(value)}` };
  }
  const unionKey = Object.keys(schema.properties).find((key) => schema.properties[key]!.type === 'tagged-union');
  if (!unionKey) {
    const parts = valueParts(schema, value, path, component).slice(0, 3);
    return { tag: '', unreadable: false, parts, spoken: spokenParts(parts) || 'no values' };
  }
  const union = schema.properties[unionKey] as UnionSchema;
  const inner = value[unionKey];
  const noun = (union.title ?? fieldHint(component, pointer([...path, unionKey])).title ?? humanize(unionKey)).toLowerCase();
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
  const parts = variant ? valueParts(variant, inner, [...path, unionKey], component, disc) : [];
  return { tag, unreadable: false, parts, spoken: [tag, spokenParts(parts)].filter(Boolean).join(', ') };
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
  skip?: string,
): ValuePart[] {
  const out: ValuePart[] = [];
  for (const [key, child] of Object.entries(schema.properties)) {
    if (key === skip) continue;
    const raw = value[key];
    const unit = 'x-incant-unit' in child ? child['x-incant-unit'] : undefined;
    const hint = fieldHint(component, pointer([...path, key]));
    const long = (child.title ?? hint.title ?? humanize(key)).toLowerCase();
    let text: string | null = null;
    if (typeof raw === 'number') text = formatNumber(raw);
    else if (Array.isArray(raw) && raw.length > 0 && raw.every((v) => typeof v === 'number')) text = raw.map(formatNumber).join(' × ');
    else if (typeof raw === 'string' && child.type === 'string' && !hint.identifier) text = raw;
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

/** "4 parts · 3 box, 1 capsule". */
function listSummary(noun: string, summaries: readonly ItemSummary[]): string {
  const count = `${summaries.length} ${summaries.length === 1 ? noun.toLowerCase() : `${noun.toLowerCase()}s`}`;
  const tags = new Map<string, number>();
  let unreadable = 0;
  for (const summary of summaries) {
    if (summary.unreadable) unreadable += 1;
    else if (summary.tag) tags.set(summary.tag, (tags.get(summary.tag) ?? 0) + 1);
  }
  const breakdown = [...[...tags].map(([tag, n]) => `${n} ${tag}`), ...(unreadable ? [`${unreadable} unreadable`] : [])];
  return breakdown.length > 0 ? `${count} · ${breakdown.join(', ')}` : count;
}

function itemProblems(diagnostics: readonly Diagnostic[], itemPointer: string): Diagnostic[] {
  return diagnostics.filter((d) => d.path !== null && (d.path === itemPointer || d.path.startsWith(`${itemPointer}/`)));
}

/** Profile order first (the authored meaning), then any other schema keys in schema order. */
function itemKeys(schema: ObjectSchema, order: readonly string[] | undefined): string[] {
  const all = Object.keys(schema.properties);
  const first = (order ?? []).filter((key) => all.includes(key));
  return [...first, ...all.filter((key) => !first.includes(key))];
}

function objectSchema(schema: FieldSchema | undefined): ObjectSchema | undefined {
  return schema?.type === 'object' && 'properties' in schema ? (schema as ObjectSchema) : undefined;
}

/** "Parts" -> "Part"; anything else -> "Item". */
function singular(label: string): string {
  return /[^s]s$/i.test(label) ? label.slice(0, -1) : 'Item';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
