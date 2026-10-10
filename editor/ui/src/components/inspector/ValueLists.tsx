import { useEffect, useId, useMemo, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import type { FieldSchema } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import { mountedIndices, rowRange, useViewport } from '../../shell/windowing';
import type { FieldContext } from './FieldView';
import { ProblemList, fieldDomId, severityOf } from './FieldView';
import { count, formatNumber, pointer } from './format';
import type { FieldHint } from './presentation';

type ArraySchema = Extract<FieldSchema, { type: 'array' }>;

/** A number list longer than a vector: shown as a bounded table, never one control per value. */
export function isLongNumberList(schema: FieldSchema, value: readonly unknown[], hint: FieldHint): boolean {
  if (schema.type !== 'array' || !('items' in schema)) return false;
  if (schema.items.type !== 'number' && schema.items.type !== 'integer') return false;
  if (hint.axes) return false;
  return (value.length > 4 || hint.grid !== undefined) && value.every((item) => typeof item === 'number');
}

/** A list of text values (IDs, names): one row each in a bounded list. */
export function isStringList(schema: FieldSchema, value: readonly unknown[]): boolean {
  return schema.type === 'array' && 'items' in schema && schema.items.type === 'string' && value.every((item) => typeof item === 'string');
}

/** The diagnostics a list row presents: its own path and any index beneath it. */
function listProblems(ctx: FieldContext, here: string) {
  return ctx.diagnostics.filter((d) => d.path !== null && (d.path === here || d.path.startsWith(`${here}/`)));
}

/** "Materials" -> "Material"; anything else -> "Item". */
function singular(label: string): string {
  return /[^s]s$/i.test(label) ? label.slice(0, -1) : 'Item';
}

/** The row a list hangs from: its label, a one-line summary, and any problems. */
function ListHeader({
  id,
  label,
  summary,
  spoken,
  summaryId,
  description,
  severity,
  problems,
}: {
  id: string;
  label: string;
  summary: string;
  /** Words for the summary when they say more than the visible text. */
  spoken?: string | undefined;
  summaryId: string;
  description: string | undefined;
  severity: 'error' | 'warning' | null;
  problems: ReturnType<typeof listProblems>;
}) {
  const messageId = useId();
  return (
    <div className={`field field--list${severity ? ` field--${severity}` : ''}`} title={description ?? label}>
      <span className="field__label" id={`${id}-label`}>
        {label}
      </span>
      <span className="object-list__summary" id={summaryId} title={spoken ?? summary}>
        {spoken && spoken !== summary ? (
          <>
            <span aria-hidden="true">{summary}</span>
            <span className="visually-hidden">{spoken}</span>
          </>
        ) : (
          summary
        )}
      </span>
      {description ? (
        <span id={`${id}-description`} className="visually-hidden">
          {description}
        </span>
      ) : null}
      {problems.length > 0 ? <ProblemList id={messageId} problems={problems} here={pointer([])} /> : null}
    </div>
  );
}

const CELL_ROW = 22;

/**
 * A long list of numbers (a navigation grid's costs). The values are text in a
 * bounded, scrollable box that is one Tab stop; only the rows in view are mounted,
 * so 65,536 values cost the same as 200. When the profile names a sibling width
 * and height that fit the list, a map draws every value as one pixel first, and
 * the exact values open beneath it.
 */
export function NumberListView({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: ArraySchema;
  hint: FieldHint;
  value: readonly number[];
  path: readonly string[];
  ctx: FieldContext;
}) {
  const here = pointer(path);
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const summaryId = `${id}-summary`;
  const problems = listProblems(ctx, here);
  const severity = severityOf(problems);
  const errorIndexes = useMemo(() => {
    const set = new Set<number>();
    for (const d of problems) {
      const index = d.severity === 'error' && d.path ? Number(d.path.slice(here.length + 1).split('/')[0]) : NaN;
      if (Number.isInteger(index)) set.add(index);
    }
    return set;
  }, [problems, here]);
  const stats = useMemo(() => {
    let min = Infinity;
    let max = -Infinity;
    for (const n of value) {
      if (n < min) min = n;
      if (n > max) max = n;
    }
    return { min, max };
  }, [value]);
  const size = gridSize(hint, ctx, value.length);
  const [showValues, setShowValues] = useState(size === null);
  const description = schema.description ?? hint.description;
  const range = value.length === 0 ? '' : stats.min === stats.max ? ` · all ${formatNumber(stats.min)}` : ` · ${formatNumber(stats.min)} to ${formatNumber(stats.max)}`;
  const blocked = size && hint.grid ? value.reduce((n, v) => (v === hint.grid!.blocked ? n + 1 : n), 0) : 0;
  // A grid's visible summary stays short (its legend shows the cost range); the full
  // sentence is the summary's tooltip and what assistive tech reads.
  const summary = size
    ? `${size.width} × ${size.height}${blocked ? ` · ${blocked.toLocaleString()} blocked` : ''}`
    : `${count(value.length, 'value')}${range}`;
  const spoken = size
    ? `${size.width} by ${size.height} grid, ${count(blocked, 'blocked cell')}${walkRange(value, hint.grid?.blocked).replace(' · cost', ', walkable cost')}`
    : summary;
  const tableId = `${id}-values`;
  return (
    <div className="value-list" data-severity={severity ?? undefined}>
      <ListHeader
        id={id}
        label={label}
        summary={summary}
        spoken={spoken}
        summaryId={summaryId}
        description={description}
        severity={severity}
        problems={problems}
      />
      {value.length === 0 ? (
        <p className="object-list__empty" id={id}>
          No values.
        </p>
      ) : (
        <>
          {size && hint.grid ? (
            <GridMap id={id} values={value} width={size.width} height={size.height} blocked={hint.grid.blocked} />
          ) : null}
          {size ? (
            <button
              type="button"
              className="value-list__toggle"
              aria-expanded={showValues}
              aria-controls={tableId}
              onClick={() => setShowValues(!showValues)}
            >
              <Icon name="chevron" size={12} className={`value-list__chevron${showValues ? ' is-open' : ''}`} />
              {showValues ? 'Hide values' : 'Show values'}
            </button>
          ) : null}
          {showValues ? (
            <NumberTable
              id={size ? tableId : id}
              values={value}
              columns={size ? size.width : null}
              label={label}
              labelledBy={`${id}-label`}
              describedBy={[summaryId, description ? `${id}-description` : null].filter(Boolean).join(' ')}
              errorIndexes={errorIndexes}
              invalid={severity === 'error'}
            />
          ) : null}
        </>
      )}
    </div>
  );
}

/** Walkable cost range of a grid, excluding the blocked value. */
function walkRange(values: readonly number[], blocked: number | undefined): string {
  let min = Infinity;
  let max = -Infinity;
  for (const v of values) {
    if (v === blocked) continue;
    if (v < min) min = v;
    if (v > max) max = v;
  }
  if (min === Infinity) return '';
  return min === max ? ` · cost ${formatNumber(min)}` : ` · cost ${formatNumber(min)} to ${formatNumber(max)}`;
}

/** The sibling width × height named by the profile, only when it exactly fits the list. */
function gridSize(hint: FieldHint, ctx: FieldContext, length: number): { width: number; height: number } | null {
  if (!hint.grid || !ctx.value) return null;
  const key = hint.grid.size.replace(/^\//, '');
  const size = ctx.value[key];
  if (!Array.isArray(size) || size.length !== 2) return null;
  const [width, height] = size as unknown[];
  if (typeof width !== 'number' || typeof height !== 'number' || !Number.isInteger(width) || !Number.isInteger(height)) return null;
  if (width < 1 || height < 1 || width * height !== length) return null;
  return { width, height };
}

/**
 * Grid costs as a picture, one pixel per cell, scaled without smoothing: walkable
 * cells from a cool low cost to a warm high cost, blocked cells dark red. Drawn
 * once per value; decorative for assistive tech, which reads the summary and the
 * exact values instead.
 */
function GridMap({ id, values, width, height, blocked }: { id: string; values: readonly number[]; width: number; height: number; blocked: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const { min, max } = useMemo(() => {
    let lo = Infinity;
    let hi = -Infinity;
    for (const v of values) {
      if (v === blocked) continue;
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
    return { min: lo, max: hi };
  }, [values, blocked]);
  useEffect(() => {
    const context = canvas.current?.getContext?.('2d');
    if (!context) return;
    const image = context.createImageData(width, height);
    const span = max > min ? max - min : 1;
    for (let i = 0; i < values.length; i += 1) {
      const v = values[i]!;
      const [r, g, b] = v === blocked ? BLOCKED : mix(LOW, HIGH, max > min ? (v - min) / span : 0);
      image.data[i * 4] = r;
      image.data[i * 4 + 1] = g;
      image.data[i * 4 + 2] = b;
      image.data[i * 4 + 3] = 255;
    }
    context.putImageData(image, 0, 0);
  }, [values, width, height, blocked, min, max]);
  const style = { aspectRatio: `${width} / ${height}` } as CSSProperties;
  return (
    <div className="grid-map" id={`${id}-map`} aria-hidden="true">
      <div className="grid-map__frame">
        <canvas ref={canvas} className="grid-map__canvas" width={width} height={height} style={style} />
      </div>
      <div className="grid-map__legend">
        <span className="grid-map__key">
          <span className="grid-map__swatch" style={{ background: `rgb(${BLOCKED.join(',')})` }} />
          Blocked
        </span>
        {min !== Infinity ? (
          <span className="grid-map__key">
            {formatNumber(min)}
            <span className="grid-map__ramp" style={{ background: `linear-gradient(90deg, rgb(${LOW.join(',')}), rgb(${HIGH.join(',')}))` }} />
            {formatNumber(max)}
          </span>
        ) : null}
        <span className="grid-map__note">Row 0 at the top</span>
      </div>
    </div>
  );
}

const LOW = [44, 70, 86] as const;
const HIGH = [226, 178, 92] as const;
const BLOCKED = [92, 38, 40] as const;

function mix(a: readonly number[], b: readonly number[], t: number): [number, number, number] {
  return [0, 1, 2].map((i) => Math.round(a[i]! + (b[i]! - a[i]!) * t)) as [number, number, number];
}

/**
 * Exact values in rows: one grid row per table row when `columns` is given, else as
 * many as fit. Mounted rows are limited to the scroll window; the box is focusable,
 * so arrow and page keys scroll it.
 */
function NumberTable({
  id,
  values,
  columns,
  label,
  labelledBy,
  describedBy,
  errorIndexes,
  invalid,
}: {
  id: string;
  values: readonly number[];
  columns: number | null;
  label: string;
  labelledBy: string;
  describedBy: string;
  errorIndexes: ReadonlySet<number>;
  invalid: boolean;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  const viewport = useViewport(scroller, CELL_ROW * 4);
  const longest = useMemo(() => values.reduce((n, v) => Math.max(n, formatNumber(v).length), 1), [values]);
  const gutter = String(Math.max(0, values.length - 1)).length;
  // Monospace at 12 px is 7.2 px per character; each cell adds 10 px of padding.
  const cellWidth = longest * 7.2 + 10;
  const gutterWidth = (gutter + 2) * 7.2;
  const fit = Math.max(1, Math.floor((viewport.width - gutterWidth - 16) / cellWidth));
  const perRow = columns ?? fit;
  const rows = Math.ceil(values.length / perRow);
  const [first, last] = rowRange(viewport, rows, CELL_ROW, 6, 4);
  const indices = mountedIndices(first, last, rows, []);
  // A grid row wider than the box mounts only the columns in view too.
  const firstColumn = columns === null ? 0 : Math.max(0, Math.floor((viewport.left - gutterWidth) / cellWidth) - 4);
  const lastColumn = columns === null ? perRow : Math.min(perRow, Math.ceil((viewport.left + viewport.width) / cellWidth) + 4);
  const style = { '--cell-chars': longest, '--gutter-chars': gutter } as CSSProperties;
  return (
    <div
      ref={scroller}
      className={`number-table${columns !== null ? ' is-grid' : ''}`}
      id={id}
      role="group"
      tabIndex={0}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      aria-invalid={invalid || undefined}
      aria-roledescription="value table"
      style={style}
    >
      <span className="visually-hidden">
        {label}: {count(values.length, 'value')}
        {columns !== null ? `, ${columns} per row` : ''}. Scroll for more.
      </span>
      <div
        className="number-table__canvas"
        role="none"
        style={{ height: rows * CELL_ROW + 8, width: columns !== null ? gutterWidth + perRow * cellWidth + 8 : undefined }}
      >
        {indices.map((row) => {
          const start = row * perRow;
          const end = Math.min(values.length, start + perRow);
          const from = start + Math.min(firstColumn, end - start);
          const to = Math.min(end, start + lastColumn);
          return (
            <div key={row} className="number-table__row" style={{ top: row * CELL_ROW + 4 }}>
              <span className="number-table__index" title={columns !== null ? `Row ${row}` : `Values from index ${start}`}>
                {columns !== null ? row : start}
              </span>
              {from > start ? <span className="number-table__skip" style={{ width: (from - start) * cellWidth }} aria-hidden="true" /> : null}
              {values.slice(from, to).map((v, offset) => {
                const index = from + offset;
                return (
                  <span
                    key={index}
                    className={`number-table__cell${errorIndexes.has(index) ? ' is-error' : ''}`}
                    title={columns !== null ? `x ${index - start}, y ${row}: ${v}` : `[${index}] ${v}`}
                  >
                    {formatNumber(v)}
                  </span>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}

const STRING_ROW = 26;

/**
 * A list of text values, such as a mesh's material IDs. Each value is a read-only
 * box in a bounded list; an ID that names a project asset or entity also shows
 * its name. Long lists mount only the rows in view.
 */
export function StringListView({
  label,
  schema,
  hint,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: ArraySchema;
  hint: FieldHint;
  value: readonly string[];
  path: readonly string[];
  ctx: FieldContext;
}) {
  const here = pointer(path);
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const summaryId = `${id}-summary`;
  const problems = listProblems(ctx, here);
  const severity = severityOf(problems);
  const noun = singular(label);
  const description = schema.description ?? hint.description;
  const named = value.filter((item) => ctx.refs?.(item)).length;
  const summary = `${count(value.length, noun.toLowerCase())}${named > 0 && named < value.length ? ` · ${named} named` : ''}`;
  const scroller = useRef<HTMLDivElement>(null);
  const viewport = useViewport(scroller, STRING_ROW * 4);
  const [first, last] = rowRange(viewport, value.length, STRING_ROW, 6, 4);
  const windowed = value.length > 48;
  const indices = windowed ? mountedIndices(first, last, value.length, []) : value.map((_, index) => index);
  return (
    <div className="value-list" data-severity={severity ?? undefined}>
      <ListHeader id={id} label={label} summary={summary} summaryId={summaryId} description={description} severity={severity} problems={problems} />
      {value.length === 0 ? (
        <p className="object-list__empty" id={id}>
          No {label.toLowerCase()}.
        </p>
      ) : (
        <div
          ref={scroller}
          className={`string-list${windowed ? ' is-windowed' : ''}`}
          id={id}
          role="list"
          aria-labelledby={`${id}-label`}
          aria-describedby={summaryId}
          tabIndex={windowed ? 0 : undefined}
        >
          <div className="string-list__canvas" role="none" style={windowed ? { height: value.length * STRING_ROW + 8 } : undefined}>
            {indices.map((index) => {
              const item = value[index]!;
              const reference = ctx.refs?.(item) ?? null;
              const itemId = fieldDomId(ctx.entity, ctx.component, [...path, String(index)]);
              return (
                <div key={index} role="listitem" className="string-list__row" style={windowed ? { top: index * STRING_ROW + 4 } : undefined}>
                  <span className="string-list__index mono" aria-hidden="true">
                    {index + 1}
                  </span>
                  <span className={`control control--ref${reference ? ' is-resolved' : ''}`}>
                    {reference ? <Icon name={iconForKind(reference.kind)} size={12} className={`kind--${reference.kind}`} /> : null}
                    {reference ? (
                      <span className="ref__name" aria-hidden="true" title={reference.name}>
                        {reference.name}
                      </span>
                    ) : null}
                    <input
                      id={itemId}
                      className="control__input mono"
                      aria-label={`${noun} ${index + 1}`}
                      aria-describedby={reference ? `${itemId}-ref` : undefined}
                      aria-invalid={problems.some((d) => d.severity === 'error' && d.path === `${here}/${index}`) || undefined}
                      readOnly
                      aria-readonly
                      value={item}
                      title={item}
                    />
                    {reference ? (
                      <span id={`${itemId}-ref`} className="visually-hidden">
                        {reference.spoken}
                      </span>
                    ) : null}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
