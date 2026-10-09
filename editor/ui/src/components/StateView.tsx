import type { ReactNode } from 'react';
import type { IconName } from '../icons/Icon';
import { Icon } from '../icons/Icon';

type Tone = 'neutral' | 'error' | 'warning';

/** Shared empty / error presentation for panels. Loading uses `Skeleton` instead. */
export function StateView({
  icon,
  title,
  children,
  tone = 'neutral',
  action,
  compact = false,
}: {
  icon: IconName;
  title: string;
  children?: ReactNode;
  tone?: Tone;
  action?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div className={`state-view state-view--${tone}${compact ? ' state-view--compact' : ''}`} role={tone === 'error' ? 'alert' : undefined}>
      <span className="state-view__tile" aria-hidden="true">
        <Icon name={icon} size={compact ? 16 : 20} className="state-view__icon" />
      </span>
      <p className="state-view__title">{title}</p>
      {children ? <div className="state-view__body">{children}</div> : null}
      {action ? <div className="state-view__action">{action}</div> : null}
    </div>
  );
}

/** Loading placeholder rows. Announces once via aria-busy on the owning region. */
export function Skeleton({ rows = 8, label }: { rows?: number; label: string }) {
  return (
    <div className="skeleton" role="status" aria-label={label}>
      <span className="visually-hidden">{label}</span>
      {Array.from({ length: rows }, (_, index) => (
        <span
          key={index}
          className="skeleton__row"
          aria-hidden="true"
          style={{ marginInlineStart: `${(index % 3) * 16}px`, width: `${70 - (index % 4) * 12}%` }}
        />
      ))}
    </div>
  );
}
