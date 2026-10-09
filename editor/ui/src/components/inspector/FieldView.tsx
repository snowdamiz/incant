import { useId } from 'react';
import type { ReactNode } from 'react';
import type { Diagnostic, FieldSchema } from '../../bridge/contract';
import { Icon } from '../../icons/Icon';

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
  const label = schema.title ?? humanize(name);
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
  return <FieldRow label={label} schema={schema} value={value} path={path} ctx={ctx} />;
}

function FieldRow({
  label,
  schema,
  value,
  path,
  ctx,
}: {
  label: string;
  schema: FieldSchema;
  value: unknown;
  path: readonly string[];
  ctx: FieldContext;
}) {
  const id = fieldDomId(ctx.entity, ctx.component, path);
  const messageId = useId();
  const here = pointer(path);
  const problems = ctx.diagnostics.filter((d) => d.path === here);
  const severity = problems.some((d) => d.severity === 'error')
    ? 'error'
    : problems.some((d) => d.severity === 'warning')
      ? 'warning'
      : null;
  const describedBy = problems.length > 0 ? messageId : undefined;
  return (
    <div className={`field${severity ? ` field--${severity}` : ''}`}>
      {/* A real <label> (plus aria-labelledby for non-input controls) keeps the name robust even when clipped. */}
      <label className="field__label" id={`${id}-label`} htmlFor={id} title={schema.description ?? label}>
        {label}
      </label>
      <div className="field__value">
        <ValueControl
          id={id}
          fieldKey={path[path.length - 1] ?? ''}
          schema={schema}
          value={value}
          invalid={severity === 'error'}
          describedBy={describedBy}
        />
      </div>
      {problems.length > 0 ? (
        <ul className="field__problems" id={messageId}>
          {problems.map((d) => (
            <li key={d.id} className={`field__problem field__problem--${d.severity}`}>
              <Icon name={d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info'} size={12} />
              <span>{d.message}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function ValueControl({
  id,
  fieldKey,
  schema,
  value,
  invalid,
  describedBy,
}: {
  id: string;
  fieldKey: string;
  schema: FieldSchema;
  value: unknown;
  invalid: boolean;
  describedBy: string | undefined;
}) {
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
      if (typeof value !== 'number') return <Mismatch id={id} expected="a number" value={value} />;
      const unit = 'x-incant-unit' in schema ? schema['x-incant-unit'] : undefined;
      return (
        <span className="control control--number">
          <input {...common} className="control__input mono" value={formatNumber(value)} />
          {unit ? <span className="control__unit">{unit}</span> : null}
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
                className="control__input mono"
                aria-label={channels?.[index]?.name}
                readOnly
                aria-readonly
                aria-invalid={invalid || undefined}
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

function Notice({ id, children, value }: { id?: string | undefined; children: ReactNode; value: unknown }) {
  return (
    <span className="control control--notice" id={id} role="group" aria-labelledby={id ? `${id}-label` : undefined}>
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
