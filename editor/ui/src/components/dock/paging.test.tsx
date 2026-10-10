import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { Pager, usePage } from './paging';
import type { Page } from './paging';
afterEach(cleanup);
function Probe({ total }: { total: number }) {
  const page = usePage(total, total - 1);
  return <><Pager page={page} noun="messages" anchorLabel="Newest" /><output aria-label="Page">{page.index}</output></>;
}
describe('pager focus and list replacement', () => {
  it('cancels a pending repeated announcement on unmount or a newer navigation', () => {
    vi.useFakeTimers();
    try {
      const page: Page = { index: 0, count: 3, start: 0, end: 100, total: 300, anchorPage: null, go: () => undefined };
      const { unmount } = render(<Pager page={page} noun="messages" />);
      const next = screen.getByRole('button', { name: 'Next page' });
      fireEvent.click(next);
      fireEvent.click(next);
      expect(vi.getTimerCount()).toBe(1);
      fireEvent.click(screen.getByRole('button', { name: 'Last page' }));
      expect(vi.getTimerCount()).toBe(0);
      expect(screen.getByRole('status').textContent).toBe('Showing messages 201–300 of 300');
      fireEvent.click(screen.getByRole('button', { name: 'Last page' }));
      expect(vi.getTimerCount()).toBe(1);
      unmount();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('keeps keyboard focus within the pager after following its disappearing anchor', () => {
    render(<Probe total={1000} />);
    fireEvent.click(screen.getByRole('button', { name: 'First page' }));
    const anchor = screen.getByRole('button', { name: 'Newest' });
    anchor.focus();
    fireEvent.click(anchor);
    expect(screen.getByRole('button', { name: 'Last page' }).closest('[role="group"]')?.contains(document.activeElement)).toBe(true);
    expect(document.activeElement).not.toBe(document.body);
    expect(screen.getByLabelText('Page').textContent).toBe('9');
  });
  it('retains the clamped page if a reduced list later grows again', () => {
    const { rerender } = render(<Probe total={1000} />);
    fireEvent.click(screen.getByRole('button', { name: 'Previous page' }));
    expect(screen.getByLabelText('Page').textContent).toBe('8');
    rerender(<Probe total={150} />);
    expect(screen.getByLabelText('Page').textContent).toBe('1');
    rerender(<Probe total={1000} />);
    expect(screen.getByLabelText('Page').textContent).toBe('1');
  });
  it('announces a chosen page from the pager, whose range is already visible', () => {
    render(<Probe total={1000} />);
    const status = within(screen.getByRole('group', { name: 'Pages of messages' })).getByRole('status');
    expect(status.textContent).toBe('');
    fireEvent.click(screen.getByRole('button', { name: 'Previous page' }));
    expect(status.textContent).toBe('Showing messages 801–900 of 1,000');
  });
});
