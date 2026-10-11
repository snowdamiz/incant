import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ContextMenu } from './Menu';

afterEach(cleanup);

function setup(unavailable?: string) {
  const rename = vi.fn();
  const copy = vi.fn();
  const remove = vi.fn();
  const onClose = vi.fn();
  const onUnavailable = vi.fn();
  const props = {
    label: 'Entity actions',
    at: { x: 10, y: 10 },
    onClose,
    onUnavailable,
    items: [
      { id: 'rename', label: 'Rename', onSelect: rename },
      { id: 'copy', label: 'Copy ID', onSelect: copy },
      { id: 'delete', label: 'Delete', unavailable, onSelect: remove },
    ],
  };
  const view = render(<ContextMenu {...props} />);
  const rows = screen.getAllByRole('menuitem');
  return { ...view, props, rows, rename, copy, remove, onClose, onUnavailable };
}

describe('context menu focus and activation', () => {
  it.each(['Enter', ' '])('focuses the hovered item before %j activates it', (key) => {
    const { rows, rename, copy, remove, onClose } = setup();
    expect(document.activeElement).toBe(rows[0]);
    fireEvent.mouseEnter(rows[2]!);
    expect(document.activeElement).toBe(rows[2]);
    expect(rows.map((row) => row.tabIndex)).toEqual([-1, -1, 0]);
    fireEvent.keyDown(document.activeElement!, { key });
    expect(remove).toHaveBeenCalledOnce();
    expect(rename).not.toHaveBeenCalled();
    expect(copy).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledWith(true);
  });

  it('updates the tab stop and arrow navigation after programmatic focus', () => {
    const { rows, rename, copy, remove } = setup();
    act(() => rows[2]!.focus());
    expect(rows.map((row) => row.tabIndex)).toEqual([-1, -1, 0]);
    fireEvent.keyDown(rows[2]!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(rows[1]);
    fireEvent.keyDown(document.activeElement!, { key: 'Enter' });
    expect(copy).toHaveBeenCalledOnce();
    expect(rename).not.toHaveBeenCalled();
    expect(remove).not.toHaveBeenCalled();
  });

  it('activates DOM focus even before a focus state update has committed', () => {
    const { rows, rename, copy } = setup();
    act(() => {
      rows[1]!.focus();
      rows[1]!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    });
    expect(copy).toHaveBeenCalledOnce();
    expect(rename).not.toHaveBeenCalled();
  });

  it.each(['Enter', ' '])('keeps unavailable items focusable and explains them on %j', (key) => {
    const reason = 'Deleting entities is unavailable on this bridge.';
    const { rows, remove, rename, onClose, onUnavailable } = setup(reason);
    fireEvent.mouseEnter(rows[2]!);
    expect(document.activeElement).toBe(rows[2]);
    expect(rows[2]!.getAttribute('aria-disabled')).toBe('true');
    fireEvent.keyDown(document.activeElement!, { key });
    expect(onUnavailable).toHaveBeenCalledExactlyOnceWith(reason);
    expect(onClose).toHaveBeenCalledExactlyOnceWith(true);
    expect(remove).not.toHaveBeenCalled();
    expect(rename).not.toHaveBeenCalled();
  });

  it('synchronizes focus and the tab stop when reopened at a new location', () => {
    const { rows, props, rerender, rename, remove } = setup();
    fireEvent.keyDown(rows[0]!, { key: 'End' });
    expect(document.activeElement).toBe(rows[2]);
    rerender(<ContextMenu {...props} at={{ x: 30, y: 40 }} />);
    expect(document.activeElement).toBe(rows[0]);
    expect(rows.map((row) => row.tabIndex)).toEqual([0, -1, -1]);
    fireEvent.keyDown(rows[0]!, { key: 'Enter' });
    expect(rename).toHaveBeenCalledOnce();
    expect(remove).not.toHaveBeenCalled();
  });

  it('wraps arrow navigation from the hovered item, including unavailable rows', () => {
    const { rows } = setup('Unavailable');
    fireEvent.mouseEnter(rows[1]!);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(rows[2]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(rows[0]);
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(rows[2]);
  });
});
