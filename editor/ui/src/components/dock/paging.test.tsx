import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { Pager, usePage } from './paging';
afterEach(cleanup);
function Probe({ total }: { total: number }) {
  const page = usePage(total, total - 1);
  return <><Pager page={page} noun="messages" anchorLabel="Newest" announce={() => {}} /><output aria-label="Page">{page.index}</output></>;
}
describe('pager focus and list replacement', () => {
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
});
