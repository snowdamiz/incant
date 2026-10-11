import { act, fireEvent, render, screen, within } from '@testing-library/react';
import axe, { type AxeResults } from 'axe-core';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../App';
import type { BridgeSnapshot, ConsoleEntry, Diagnostic, EditorBridge, HistoryEntry, Ulid } from '../bridge/contract';
import { fixtureSnapshot } from '../bridge/fixture';
import { PAGE_SIZE } from './dock/paging';

// These behavioral cases retain thousands of diagnostics and a full visible-page
// accessibility scan. Hosted runners exceeded Vitest's default 5s; this is not a
// wall-clock performance assertion. Keep the larger budget local to those cases.
const LARGE_PAGE_TEST_TIMEOUT = 20_000;
let pendingAxe: Promise<AxeResults> | undefined;
async function checkAccessibility(container: HTMLElement) {
  pendingAxe = axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  const results = await pendingAxe;
  expect(results.violations.map((v) => v.id)).toEqual([]);
}
afterEach(async () => {
  // A timed-out test does not cancel its async work. Drain the scanner before the
  // next test can call axe, including when an assertion/timeout interrupted it.
  try {
    await pendingAxe;
  } finally {
    pendingAxe = undefined;
  }
}, LARGE_PAGE_TEST_TIMEOUT);

/** TEST DOUBLE: serves a snapshot that tests replace to simulate arriving entries; refuses every command. */
function liveBridge(initial: BridgeSnapshot) {
  const listeners = new Set<() => void>();
  let snapshot = initial;
  const refused = Promise.resolve({ ok: false as const, error: { code: 'test-double', message: 'Test double.' } });
  const bridge: EditorBridge = {
    protocolVersion: 1,
    capabilities: [],
    label: 'Test double',
    isFixture: false,
    getSnapshot: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispatch: () => refused,
    request: () => refused,
  };
  const publish = (next: BridgeSnapshot) =>
    act(() => {
      snapshot = next;
      for (const listener of listeners) listener();
    });
  return { bridge, publish };
}

const sample = fixtureSnapshot('sample');
const sampleIds = Object.keys(sample.entities) as Ulid[];
const diag = (n: number, entity: Ulid | null, severity: Diagnostic['severity'] = 'error'): Diagnostic => ({
  id: `d-${n}`,
  severity,
  message: `Problem ${n}: a long engine message that wraps to its full length in the row`,
  entity,
  component: 'Collider',
  path: `/shape/parts/${n}`,
});
const message = (n: number): ConsoleEntry => ({ id: `m-${n}`, level: 'info', source: 'native', message: `Message ${n}`, at: new Date(Date.UTC(2026, 9, 10, 12, 0, n % 60)).toISOString() });
const edit = (n: number): HistoryEntry => ({
  transaction: `01JA33TX${String(n).padStart(18, '0')}` as Ulid,
  description: `Edit ${n}`,
  origin: { kind: 'user' },
  at: new Date(Date.UTC(2026, 9, 10, 12, 0, n % 60)).toISOString(),
});
const with_ = (patch: Partial<BridgeSnapshot>): BridgeSnapshot => ({ ...sample, ...patch });

/** The visible range ("101–200 of 3,000"); the spoken text adds the noun. */
const range = (name: string) => {
  const text = screen.getByRole('group', { name: `Pages of ${name}` }).querySelector('.pager__range')!.textContent ?? '';
  expect(text.endsWith(` ${name}`)).toBe(true);
  return text.slice(0, -(name.length + 1));
};
const pager = (name: string) => screen.getByRole('group', { name: `Pages of ${name}` });
const press = (name: string, button: string) => fireEvent.click(within(pager(name)).getByRole('button', { name: button }));
const mountedProblems = () => document.querySelectorAll('.problems__item').length;
const headings = () => [...document.querySelectorAll('.problems__heading-name')].map((node) => node.textContent);

describe('Problems pages', () => {
  it('mounts one page across many groups, says which slice is shown, and keeps every count', async () => {
    // 3,000 problems on 1,500 entities (only the first 25 exist in the hierarchy).
    const diagnostics = Array.from({ length: 3000 }, (_, n) => diag(n, (sampleIds[Math.floor(n / 2) % sampleIds.length]! + (n >= 50 ? `-${Math.floor(n / 2)}` : '')) as Ulid));
    const { container } = render(<App resolution={{ kind: 'bridge', bridge: liveBridge(with_({ diagnostics })).bridge }} />);
    expect(mountedProblems()).toBe(PAGE_SIZE);
    expect(range('problems')).toBe('1–100 of 3,000');
    expect(within(screen.getByRole('group', { name: 'Show problem severities' })).getByRole('button', { name: /^Errors/ }).textContent).toContain('3,000');
    press('problems', 'Next page');
    expect(range('problems')).toBe('101–200 of 3,000');
    expect(mountedProblems()).toBe(PAGE_SIZE);
    press('problems', 'Last page');
    expect(range('problems')).toBe('2,901–3,000 of 3,000');
    const next = within(pager('problems')).getByRole('button', { name: 'Next page' });
    expect(next.getAttribute('aria-disabled')).toBe('true');
    // At the end, Next stays focusable and does nothing.
    act(() => next.focus());
    fireEvent.click(next);
    expect(document.activeElement).toBe(next);
    expect(range('problems')).toBe('2,901–3,000 of 3,000');
    await checkAccessibility(container);
  }, LARGE_PAGE_TEST_TIMEOUT);

  it('splits one huge group across pages, marks the continuation, and says how many follow', () => {
    const crate = sampleIds.find((id) => sample.entities[id]?.name === 'Crate 03')!;
    const diagnostics = Array.from({ length: 5000 }, (_, n) => diag(n, crate));
    render(<App resolution={{ kind: 'bridge', bridge: liveBridge(with_({ diagnostics })).bridge }} />);
    expect(headings()).toEqual(['Crate 03']);
    expect(document.querySelector('.problems__continued')).toBeNull();
    expect(document.querySelector('.problems__later')?.textContent).toBe('4,900 more problems on Crate 03 on the next pages');
    // The heading counts the whole group, not the page.
    expect(document.querySelector('.problems__heading-counts')?.textContent).toContain('5000');
    press('problems', 'Next page');
    expect(document.querySelector('.problems__continued')?.textContent).toBe('continued');
    expect(screen.getByText(/^Problem 100:/)).toBeTruthy();
    // Full text, and the row still reveals its entity.
    fireEvent.click(screen.getByText(/^Problem 150:/).closest('button')!);
    expect(screen.getByRole('treeitem', { name: /^Crate 03/ }).getAttribute('aria-selected')).toBe('true');
  }, LARGE_PAGE_TEST_TIMEOUT);

  it('returns to the first page when a filter changes and keeps a valid page as problems change', () => {
    const errors = Array.from({ length: 250 }, (_, n) => diag(n, sampleIds[0]!));
    const warnings = Array.from({ length: 30 }, (_, n) => diag(1000 + n, sampleIds[1]!, 'warning'));
    const { bridge, publish } = liveBridge(with_({ diagnostics: [...errors, ...warnings] }));
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    press('problems', 'Last page');
    expect(range('problems')).toBe('201–280 of 280');
    fireEvent.click(within(screen.getByRole('group', { name: 'Show problem severities' })).getByRole('button', { name: /^Errors/ }));
    // 30 warnings fit on one page: no pager, all listed.
    expect(screen.queryByRole('group', { name: 'Pages of problems' })).toBeNull();
    expect(mountedProblems()).toBe(30);
    fireEvent.click(within(screen.getByRole('group', { name: 'Show problem severities' })).getByRole('button', { name: /^Errors/ }));
    expect(range('problems')).toBe('1–100 of 280');
    press('problems', 'Last page');
    // New diagnostics arrive with fewer problems: the last page that still exists is shown.
    publish(with_({ diagnostics: errors.slice(0, 120) }));
    expect(range('problems')).toBe('101–120 of 120');
    expect(mountedProblems()).toBe(20);
  });
});

describe('Console pages', () => {
  it('opens on the newest page, follows new messages, and stays put on an older page until Newest', () => {
    const { bridge, publish } = liveBridge(with_({ console: Array.from({ length: 1000 }, (_, n) => message(n)) }));
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    fireEvent.click(screen.getByRole('tab', { name: /^Console/ }));
    const rows = () => within(screen.getByRole('log', { name: 'Console messages' })).getAllByRole('listitem');
    expect(range('messages')).toBe('901–1,000 of 1,000');
    expect(rows()).toHaveLength(PAGE_SIZE);
    expect(rows().at(-1)?.textContent).toContain('Message 999');
    publish(with_({ console: Array.from({ length: 1050 }, (_, n) => message(n)) }));
    expect(range('messages')).toBe('1,001–1,050 of 1,050');
    expect(rows().at(-1)?.textContent).toContain('Message 1049');
    press('messages', 'First page');
    publish(with_({ console: Array.from({ length: 1100 }, (_, n) => message(n)) }));
    expect(range('messages')).toBe('1–100 of 1,100');
    expect(rows()[0]?.textContent).toContain('Message 0');
    press('messages', 'Newest');
    expect(range('messages')).toBe('1,001–1,100 of 1,100');
    expect(within(pager('messages')).queryByRole('button', { name: 'Newest' })).toBeNull();
  });
});

describe('History pages', () => {
  it('opens on the page that holds Now, keeps applied and undone states, and returns to Now', async () => {
    const entries = Array.from({ length: 1000 }, (_, n) => edit(n));
    const { bridge, publish } = liveBridge(with_({ history: { entries, applied: 450 } }));
    const { container } = render(<App resolution={{ kind: 'bridge', bridge }} />);
    fireEvent.click(screen.getByRole('tab', { name: /^History/ }));
    const list = () => screen.getByRole('list', { name: 'Edit history, oldest first' });
    expect(range('edits')).toBe('401–500 of 1,000');
    expect(list().querySelectorAll('.history__row')).toHaveLength(PAGE_SIZE);
    const items = [...list().children];
    const now = items.findIndex((item) => item.getAttribute('aria-label') === 'Current position');
    expect(items[now - 1]?.textContent).toContain('Edit 449');
    expect(items[now - 1]?.textContent).not.toContain('(undone)');
    expect(items[now + 1]?.textContent).toContain('Edit 450 (undone)');
    expect(screen.getByText('450 of 1,000 applied')).toBeTruthy();
    press('edits', 'First page');
    expect(list().querySelector('.timeline__now')).toBeNull();
    // The Now marker moves (an undo elsewhere): the chosen page stays until Now is asked for.
    publish(with_({ history: { entries, applied: 700 } }));
    expect(range('edits')).toBe('1–100 of 1,000');
    press('edits', 'Now');
    expect(range('edits')).toBe('701–800 of 1,000');
    expect(list().firstElementChild?.getAttribute('aria-label')).toBe('Current position');
    await checkAccessibility(container);
  });

  it('puts Now after the last edit when everything is applied, on the last page', () => {
    const entries = Array.from({ length: 200 }, (_, n) => edit(n));
    render(<App resolution={{ kind: 'bridge', bridge: liveBridge(with_({ history: { entries, applied: 200 } })).bridge }} />);
    fireEvent.click(screen.getByRole('tab', { name: /^History/ }));
    expect(range('edits')).toBe('101–200 of 200');
    const list = screen.getByRole('list', { name: 'Edit history, oldest first' });
    expect(list.lastElementChild?.getAttribute('aria-label')).toBe('Current position');
  });
});
