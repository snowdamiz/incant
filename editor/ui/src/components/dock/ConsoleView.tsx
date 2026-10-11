import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ConsoleLevel } from '../../bridge/contract';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';
import { useShell } from '../../shell/ShellContext';
import { StateView } from '../StateView';
import { Pager, usePage } from './paging';
import { formatTime } from './time';

const LEVELS: ConsoleLevel[] = ['error', 'warn', 'info', 'debug'];
const LEVEL_LABEL: Record<ConsoleLevel, string> = { error: 'Errors', warn: 'Warnings', info: 'Info', debug: 'Debug' };
const LEVEL_ICON: Record<ConsoleLevel, IconName> = { error: 'error', warn: 'warning', info: 'info', debug: 'console' };

/**
 * Console messages as a table: time, level, source and message; levels filter the rows.
 * At most one page of messages is mounted. The view follows the newest page as
 * messages arrive, until another page is chosen; Newest returns to it.
 */
export function ConsoleList() {
  const { snapshot } = useShell();
  const [hidden, setHidden] = useState<Set<ConsoleLevel>>(() => new Set(['debug']));
  const entries = useMemo(() => snapshot?.console ?? [], [snapshot?.console]);
  const visible = useMemo(() => entries.filter((entry) => !hidden.has(entry.level)), [entries, hidden]);
  const page = usePage(visible.length, visible.length - 1);
  const listRef = useRef<HTMLOListElement>(null);
  useLayoutEffect(() => {
    if (listRef.current) listRef.current.scrollTop = 0;
  }, [page.index]);
  return (
    <div className="dock-view console">
      <div className="dock-toolbar">
        <div className="dock-toolbar__group" role="group" aria-label="Show console levels">
          {LEVELS.map((level) => {
            const count = entries.filter((entry) => entry.level === level).length;
            return (
              <button
                key={level}
                type="button"
                className={`filter-chip filter-chip--${level}`}
                aria-pressed={!hidden.has(level)}
                onClick={() => {
                  setHidden((current) => {
                    const next = new Set(current);
                    if (next.has(level)) next.delete(level);
                    else next.add(level);
                    return next;
                  });
                  page.go(null);
                }}
              >
                <Icon name={LEVEL_ICON[level]} size={12} />
                {LEVEL_LABEL[level]} <span className="filter-chip__count">{count.toLocaleString()}</span>
              </button>
            );
          })}
        </div>
        <Pager page={page} noun="messages" anchorLabel="Newest" />
      </div>
      {!snapshot ? (
        <StateView icon="plug" title="No engine connected" compact />
      ) : visible.length === 0 ? (
        <StateView icon="console" title={entries.length ? 'All messages are filtered out' : 'Console is empty'} compact />
      ) : (
        <div className="console__log" role="log" aria-label="Console messages">
          {/* Focusable: the rows are text, so the list itself is the keyboard stop that scrolls. */}
          <ol ref={listRef} className="list list--console" tabIndex={0} aria-label={`Console messages ${page.start + 1} to ${page.end} of ${visible.length}`}>
            {visible.slice(page.start, page.end).map((entry) => (
              <li key={entry.id} className={`list__row console__row console__row--${entry.level}`}>
                <time className="console__time mono" dateTime={entry.at}>
                  {formatTime(entry.at)}
                </time>
                <span className={`console__level console__level--${entry.level}`}>{entry.level}</span>
                <span className="console__source mono">{entry.source}</span>
                <span className="list__main">{entry.message}</span>
              </li>
            ))}
          </ol>
        </div>
      )}
    </div>
  );
}
