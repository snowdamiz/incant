import type { ReactNode } from 'react';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';

/**
 * What the Inspector shows, in the panel's 36 px header line: a kind glyph, the
 * name (a heading, and the focus target when asset details open) and actions.
 * Long names end in an ellipsis; the full name is the tooltip.
 */
export function IdentityHeader({
  icon,
  kind,
  name,
  titleId,
  focusable = false,
  actions,
}: {
  icon: IconName;
  kind?: string | undefined;
  name: string;
  titleId?: string | undefined;
  focusable?: boolean;
  actions?: ReactNode;
}) {
  return (
    <>
      <span className={`inspector-header__glyph${kind ? ` kind--${kind}` : ''}`} aria-hidden="true">
        <Icon name={icon} size={16} />
      </span>
      <h3 id={titleId} className="inspector__name" tabIndex={focusable ? -1 : undefined} title={name}>
        {name}
      </h3>
      {actions ? <div className="inspector-header__actions">{actions}</div> : null}
    </>
  );
}
