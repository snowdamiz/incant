import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../App';
import { createProviderFixtureBridge } from '../bridge/providerFixture';

/**
 * jsdom has no layout, so the transcript's box and content heights are stubbed. The
 * browser capture (handoffs/0022-physics-inspector/tools/capture-agent.mjs) checks the
 * real 180 px panel; this pins the keyboard contract that depends on it.
 */
function stubTranscript(content: number, box: number) {
  const scroll = vi.spyOn(HTMLElement.prototype, 'scrollHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('agent__transcript') ? content : 0;
  });
  const client = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.classList.contains('agent__transcript') ? box : 0;
  });
  return () => {
    scroll.mockRestore();
    client.mockRestore();
  };
}

let restore = () => {};
afterEach(() => restore());

describe('Agent empty state in a short panel', () => {
  it('makes the transcript a keyboard stop only while its text overflows', () => {
    restore = stubTranscript(78, 62);
    render(<App resolution={{ kind: 'bridge', bridge: createProviderFixtureBridge('sample', 'signed-in') }} />);
    const log = screen.getByRole('log', { name: 'Agent conversation' });
    expect(log.tabIndex).toBe(0);
    expect(log.textContent).toContain('Agent not ready');
  });

  it('adds no extra tab stop when everything fits', () => {
    restore = stubTranscript(62, 62);
    render(<App resolution={{ kind: 'bridge', bridge: createProviderFixtureBridge('sample', 'signed-in') }} />);
    const log = screen.getByRole('log', { name: 'Agent conversation' });
    expect(log.hasAttribute('tabindex')).toBe(false);
  });
});
