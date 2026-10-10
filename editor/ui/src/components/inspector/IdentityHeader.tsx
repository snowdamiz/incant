import { useId } from 'react';
import type { ReactNode } from 'react';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';

/**
 * What the Inspector shows, in the panel's 36 px header line: the kind's glyph, the
 * name (a heading, and the focus target when asset details open) and actions.
 * Long names end in an ellipsis; the full name is the tooltip. `description` (the
 * kind and stable ID) is spoken with the name instead of taking a row of its own.
 */
export function IdentityHeader({
  icon,
  kind,
  name,
  titleId,
  focusable = false,
  description,
  actions,
}: {
  icon: IconName;
  kind?: string | undefined;
  name: string;
  titleId?: string | undefined;
  focusable?: boolean;
  description?: string | undefined;
  actions?: ReactNode;
}) {
  const descriptionId = useId();
  return (
    <>
      <Icon name={icon} size={16} className={`inspector-header__glyph${kind ? ` kind--${kind}` : ''}`} />
      <h3
        id={titleId}
        className="inspector__name"
        tabIndex={focusable ? -1 : undefined}
        title={name}
        aria-describedby={description ? descriptionId : undefined}
      >
        {name}
      </h3>
      {description ? (
        <span id={descriptionId} className="visually-hidden">
          {description}
        </span>
      ) : null}
      {actions ? <div className="inspector-header__actions">{actions}</div> : null}
    </>
  );
}
