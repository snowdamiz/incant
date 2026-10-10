import type { ReactNode } from 'react';
import type { IconName } from '../icons/Icon';
import { Icon } from '../icons/Icon';

type Tone = 'neutral' | 'error' | 'warning' | 'success';

/** Shared empty / error presentation for panels. Loading uses `Skeleton` instead. */
export function StateView({
  icon,
  title,
  children,
  tone = 'neutral',
  action,
  compact = false,
  spinner = false,
  alert = tone === 'error',
}: {
  icon: IconName;
  title: string;
  children?: ReactNode;
  tone?: Tone;
  action?: ReactNode;
  compact?: boolean;
  /** Shows a spinner instead of the icon, for work in progress. */
  spinner?: boolean;
  /**
   * Error views are announced as alerts by default. Pass false when the same failure
   * is already announced elsewhere (the connection banner), so it is read once.
   */
  alert?: boolean;
}) {
  return (
    <div className={`state-view state-view--${tone}${compact ? ' state-view--compact' : ''}`} role={alert ? 'alert' : undefined}>
      <span className="state-view__badge" aria-hidden="true">
        {spinner ? <span className="spinner" /> : <Icon name={icon} size={compact ? 16 : 20} className="state-view__icon" />}
      </span>
      <p className="state-view__title">{title}</p>
      {children ? <div className="state-view__body">{children}</div> : null}
      {action ? <div className="state-view__action">{action}</div> : null}
    </div>
  );
}

/** Loading placeholder rows shaped like tree rows: an icon and a name, at varied depths. */
export function Skeleton({ rows = 8, label }: { rows?: number; label: string }) {
  return (
    <div className="skeleton" role="status" aria-label={label}>
      <span className="visually-hidden">{label}</span>
      {Array.from({ length: rows }, (_, index) => (
        <span key={index} className="skeleton__line" aria-hidden="true" style={{ paddingInlineStart: `${(index % 3) * 14}px` }}>
          <span className="skeleton__icon" />
          <span className="skeleton__row" style={{ width: `${68 - (index % 4) * 11}%` }} />
        </span>
      ))}
    </div>
  );
}
