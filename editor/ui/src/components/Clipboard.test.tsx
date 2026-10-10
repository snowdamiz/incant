import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../App';
import { createFixtureBridge } from '../bridge/fixture';
import { COPY_ID_UNAVAILABLE } from '../shell/clipboard';

const status = () => document.querySelector('.statusbar__message')?.textContent ?? '';
const row = (name: string) => screen.getByRole('treeitem', { name: new RegExp(`^${name}`) });

/** Installs (or removes, with null) a clipboard on this jsdom navigator, which has none. */
function setClipboard(clipboard: { writeText: (text: string) => Promise<void> } | null) {
  if (clipboard === null) Reflect.deleteProperty(navigator, 'clipboard');
  else Object.defineProperty(navigator, 'clipboard', { configurable: true, value: clipboard });
}

afterEach(() => setClipboard(null));

const renderSample = () => render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
const copyFromMenu = () => {
  fireEvent.contextMenu(row('Crate 01'), { clientX: 120, clientY: 300 });
  return within(screen.getByRole('menu')).getByRole('menuitem', { name: /^Copy ID/ });
};
const copyButton = () => within(document.querySelector<HTMLElement>('[data-region="inspector"]')!).getByRole('button', { name: 'Copy entity ID' });

describe('Copy ID without a clipboard', () => {
  it('marks the menu item unavailable and says why instead of doing nothing', async () => {
    setClipboard(null);
    renderSample();
    const item = copyFromMenu();
    expect(item.getAttribute('aria-disabled')).toBe('true');
    expect(item.textContent).toContain('unavailable');
    fireEvent.click(item);
    await waitFor(() => expect(status()).toBe(COPY_ID_UNAVAILABLE));
  });

  it('keeps the Inspector button focusable, marked unavailable, and explains on activation', async () => {
    setClipboard(null);
    renderSample();
    fireEvent.click(row('Crate 01'));
    const button = copyButton();
    expect(button.getAttribute('aria-disabled')).toBe('true');
    expect(document.getElementById(button.getAttribute('aria-describedby')!)?.textContent).toBe(COPY_ID_UNAVAILABLE);
    act(() => button.focus());
    fireEvent.click(button);
    await waitFor(() => expect(status()).toBe(COPY_ID_UNAVAILABLE));
    expect(document.activeElement).toBe(button);
  });
});

describe('Copy ID with a clipboard', () => {
  it('announces a refused write from the menu and from the Inspector', async () => {
    const writeText = vi.fn(() => Promise.reject(new DOMException('Write permission denied.', 'NotAllowedError')));
    setClipboard({ writeText });
    renderSample();
    fireEvent.click(copyFromMenu());
    await waitFor(() => expect(status()).toBe('Could not copy: clipboard access was denied.'));
    fireEvent.click(copyButton());
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(2));
    // The same message is announced again (the status line clears it first so it is re-read).
    await waitFor(() => expect(status()).toBe('Could not copy: clipboard access was denied.'));
  });

  it('copies the exact entity ID and says so', async () => {
    const writeText = vi.fn(() => Promise.resolve());
    setClipboard({ writeText });
    renderSample();
    const id = row('Crate 01').id.replace('tree-row-', '');
    fireEvent.click(copyFromMenu());
    await waitFor(() => expect(status()).toBe('Entity ID copied'));
    expect(writeText).toHaveBeenCalledWith(id);
    expect(copyButton().hasAttribute('aria-disabled')).toBe(false);
  });
});
