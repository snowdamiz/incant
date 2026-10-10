import { useLayoutEffect, useRef, useState } from 'react';
import type { Ref } from 'react';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';

/**
 * Rows a dock list mounts at most. Problems, console messages and history entries are
 * not capped by the bridge, and their rows wrap to their full text, so the lists page
 * instead of mounting every entry: each page is a bounded slice, and the pager always
 * says which slice of how many is shown. Nothing is dropped or hidden.
 */
export const PAGE_SIZE = 100;

export interface Page {
  /** Zero-based page index. */
  readonly index: number;
  readonly count: number;
  /** First and one-past-last item index of this page. */
  readonly start: number;
  readonly end: number;
  readonly total: number;
  /** Page that holds the anchor (the newest message, the Now marker), if any. */
  readonly anchorPage: number | null;
  /** Shows a page; `null` follows the anchor again (or returns to the first page). */
  readonly go: (index: number | null) => void;
}

/**
 * One page of a list of `total` items. With an `anchor` item index, the page follows the
 * anchor's page (new messages, a moved Now marker) until another page is chosen. A list
 * that shrinks keeps the chosen page when it still exists, else shows the last one.
 */
export function usePage(total: number, anchor?: number): Page {
  const count = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const [chosen, setChosen] = useState<number | null>(null);
  // Commit a shrink's clamp, so later growth cannot resurrect an old page.
  useLayoutEffect(() => {
    if (chosen !== null && chosen >= count) setChosen(count - 1);
  }, [chosen, count]);
  const anchorPage = anchor === undefined ? null : Math.min(count - 1, Math.max(0, Math.floor(anchor / PAGE_SIZE)));
  const index = chosen === null ? (anchorPage ?? 0) : Math.min(chosen, count - 1);
  const start = index * PAGE_SIZE;
  return { index, count, start, end: Math.min(total, start + PAGE_SIZE), total, anchorPage, go: setChosen };
}

/**
 * "101–200 of 5,000" with first, previous, next and last page buttons, plus a button
 * back to the anchor (Newest, Now) when it is on another page. Renders nothing for a
 * list that fits on one page. Buttons at an end stay focusable and are marked disabled,
 * so focus never drops to the page. A chosen page is announced from the pager itself:
 * the range is already on screen, so the status bar does not repeat it.
 */
export function Pager({
  page,
  noun,
  anchorLabel,
}: {
  page: Page;
  /** Plural noun for the range, as spoken: "problems", "messages", "edits". */
  noun: string;
  /** Visible name of the way back to the anchor ("Newest", "Now"). */
  anchorLabel?: string;
}) {
  const lastButton = useRef<HTMLButtonElement>(null);
  const [spoken, setSpoken] = useState('');
  if (page.count <= 1) return null;
  const range = (index: number) => {
    const start = index * PAGE_SIZE;
    return `${(start + 1).toLocaleString()}–${Math.min(page.total, start + PAGE_SIZE).toLocaleString()}`;
  };
  // Choosing the anchor's page (Last page for the console) follows the anchor again.
  const go = (target: number) => {
    if (target < 0 || target >= page.count || target === page.index) return;
    page.go(target === page.anchorPage ? null : target);
    setSpoken(`Showing ${noun} ${range(target)} of ${page.total.toLocaleString()}`);
  };
  const atStart = page.index === 0;
  const atEnd = page.index >= page.count - 1;
  return (
    <div className="pager" role="group" aria-label={`Pages of ${noun}`}>
      {anchorLabel && page.anchorPage !== null && page.anchorPage !== page.index ? (
        <button type="button" className="pager__anchor" onClick={() => {
          go(page.anchorPage!);
          // This anchor disappears when followed. Keep focus on a stable final
          // pager control, so Tab can continue into the list instead of the page.
          lastButton.current?.focus({ preventScroll: true });
        }}>
          {anchorLabel}
        </button>
      ) : null}
      <span className="pager__range">
        {range(page.index)} <span className="pager__of">of {page.total.toLocaleString()}</span>
        <span className="visually-hidden"> {noun}</span>
      </span>
      <PagerButton label="First page" icon="pageFirst" disabled={atStart} onClick={() => go(0)} />
      <PagerButton label="Previous page" icon="pagePrev" disabled={atStart} onClick={() => go(page.index - 1)} />
      <PagerButton label="Next page" icon="pageNext" disabled={atEnd} onClick={() => go(page.index + 1)} />
      <PagerButton buttonRef={lastButton} label="Last page" icon="pageLast" disabled={atEnd} onClick={() => go(page.count - 1)} />
      <span className="visually-hidden" role="status">
        {spoken}
      </span>
    </div>
  );
}

function PagerButton({ label, icon, disabled, onClick, buttonRef }: { label: string; icon: IconName; disabled: boolean; onClick: () => void; buttonRef?: Ref<HTMLButtonElement> }) {
  return (
    <button
      type="button"
      ref={buttonRef}
      className="tool-button tool-button--small pager__button"
      aria-label={label}
      title={label}
      aria-disabled={disabled || undefined}
      onClick={disabled ? undefined : onClick}
    >
      <Icon name={icon} size={14} />
    </button>
  );
}
