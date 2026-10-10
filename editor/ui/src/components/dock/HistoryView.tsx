import { Fragment, useLayoutEffect, useRef } from 'react';
import type { Origin } from '../../bridge/contract';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';
import { projectState } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { comboText, usePlatform } from '../ShortcutsDialog';
import { StateView } from '../StateView';
import { Pager, usePage } from './paging';
import { formatTime } from './time';

/**
 * Edit history, oldest first: each transaction with who made it and when, and a "Now"
 * marker between what is applied and what Redo would bring back. At most one page of edits is mounted; the view follows the
 * page that holds Now until another page is chosen, and Now returns to it.
 */
export function HistoryList() {
  const { snapshot, run } = useShell();
  const platform = usePlatform();
  const entries = snapshot?.history.entries ?? [];
  const applied = snapshot?.history.applied ?? 0;
  const page = usePage(entries.length, applied);
  const listRef = useRef<HTMLOListElement>(null);
  useLayoutEffect(() => {
    if (listRef.current) listRef.current.scrollTop = 0;
  }, [page.index]);
  if (!snapshot) return <StateView icon="plug" title="No engine connected" compact />;
  const project = projectState(snapshot);
  // Without an open project there is no history to undo; no toolbar or "0 of 0" claim.
  if (project.kind === 'loading') return <StateView icon="history" title="Opening project…" quiet />;
  if (project.kind === 'failed') {
    return (
      <StateView icon="history" title="No project loaded" compact />
    );
  }
  return (
    <div className="dock-view history">
      <div className="dock-toolbar history__toolbar">
        <UnavailableAwareButton
          icon="undo"
          label="Undo"
          shortcut={comboText(['Mod', 'Z'], platform)}
          keyShortcuts="Meta+Z Control+Z"
          disabled={applied === 0}
          onActivate={() => void run({ type: 'history.undo' })}
          capability="history.undo"
        />
        <UnavailableAwareButton
          icon="redo"
          label="Redo"
          shortcut={comboText(platform === 'mac' ? ['Shift', 'Mod', 'Z'] : ['Ctrl', 'Y'], platform)}
          keyShortcuts="Meta+Shift+Z Control+Y"
          disabled={applied >= entries.length}
          onActivate={() => void run({ type: 'history.redo' })}
          capability="history.redo"
        />
        <span className="panel__meta">
          {applied.toLocaleString()} of {entries.length.toLocaleString()} applied
        </span>
        <Pager page={page} noun="edits" anchorLabel="Now" />
      </div>
      {entries.length === 0 ? (
        <StateView icon="history" title="No history yet" compact />
      ) : (
        // Focusable: the rows are text, not controls, so the list itself is the keyboard stop that scrolls.
        <ol ref={listRef} className="list timeline" aria-label="Edit history, oldest first" tabIndex={0}>
          {entries.slice(page.start, page.end).map((entry, offset) => {
            const index = page.start + offset;
            const { label, detail } = originLabel(entry.origin);
            const undone = index >= applied;
            return (
              <Fragment key={entry.transaction}>
                {index === applied ? <NowMarker /> : null}
                <li className={`list__row history__row timeline__row origin-row--${entry.origin.kind}${undone ? ' is-undone' : ''}`}>
                  <span className="list__main">
                    {entry.description}
                    {undone ? <span className="history__undone"> (undone)</span> : null}
                  </span>
                  <span className={`origin origin--${entry.origin.kind}`} title={detail || undefined}>
                    {label}
                  </span>
                  {detail ? <span className="list__meta mono timeline__detail">{detail}</span> : null}
                  <time className="list__meta mono timeline__time" dateTime={entry.at}>
                    {formatTime(entry.at)}
                  </time>
                </li>
              </Fragment>
            );
          })}
          {applied === entries.length && page.end === entries.length ? <NowMarker /> : null}
        </ol>
      )}
    </div>
  );
}

/** The current position in history: everything above is applied, everything below can be redone. */
function NowMarker() {
  return (
    <li className="timeline__now" aria-label="Current position">
      <span className="timeline__now-label" aria-hidden="true">
        Now
      </span>
    </li>
  );
}

function originLabel(origin: Origin): { label: string; detail: string } {
  switch (origin.kind) {
    case 'user':
      return { label: 'User', detail: '' };
    case 'agent':
      return { label: 'Agent', detail: origin.model };
    case 'script':
      return { label: 'Script', detail: origin.script };
    case 'import':
      return { label: 'Import', detail: origin.source };
  }
}

function UnavailableAwareButton({
  icon,
  label,
  shortcut,
  keyShortcuts,
  disabled,
  onActivate,
  capability,
}: {
  icon: IconName;
  label: string;
  /** The keys as written on this platform, for the tooltip. */
  shortcut: string;
  keyShortcuts: string;
  disabled: boolean;
  onActivate: () => void;
  capability: 'history.undo' | 'history.redo';
}) {
  const { capabilities } = useShell();
  const unavailable = !capabilities.has(capability);
  return (
    <button
      type="button"
      className="button button--ghost"
      // aria-disabled keeps the button focusable so activating it can explain why it is unavailable.
      aria-disabled={disabled || unavailable || undefined}
      aria-keyshortcuts={keyShortcuts}
      title={`${label} (${shortcut})`}
      onClick={() => {
        if (disabled && !unavailable) return;
        onActivate();
      }}
    >
      <Icon name={icon} size={14} />
      {label}
    </button>
  );
}
