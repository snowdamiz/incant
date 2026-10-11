import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { NativeBridge } from '../../../bridge/native';
import { createNativePlayDouble, doubleRead } from '../test/nativePlayDouble';
import type { AuthoringSnapshot } from './ShellContext';
import { ShellProvider, capabilityOf, useShell } from './ShellContext';

describe('shell snapshot', () => {
  it('does not re-render authoring panels for Play status reads, and keeps Play status out of them', async () => {
    const double = createNativePlayDouble({ read: doubleRead('single', {}) });
    const bridge = new NativeBridge(double.invoke);
    await bridge.startEngineUpdates(double.listen);
    await bridge.refreshPlay();
    const seen: (AuthoringSnapshot | null)[] = [];
    function Panel() {
      seen.push(useShell().snapshot);
      return null;
    }
    render(
      <ShellProvider bridge={bridge}>
        <Panel />
      </ShellProvider>,
    );
    const before = seen.length;
    const shown = seen.at(-1)!;
    expect(shown).not.toHaveProperty('play');
    // Status reads while a session runs: a new status object four times a second.
    for (let tick = 1; tick <= 12; tick += 1) {
      act(() => bridge.updatePlay({ ...double.status(), phase: 'running', sequence: 100 + tick, tick }));
    }
    expect(seen).toHaveLength(before);
    expect(bridge.getSnapshot().play?.tick).toBe(12);
    // An authoring change still reaches them.
    await act(async () => {
      double.bumpRevision();
      await bridge.start();
    });
    expect(seen.length).toBeGreaterThan(before);
    expect(seen.at(-1)).not.toBe(shown);
    expect(seen.at(-1)).not.toHaveProperty('play');
  });

  it('checks Play operations against the play.control capability', () => {
    expect(capabilityOf({ type: 'play.step' })).toBe('play.control');
    expect(capabilityOf({ type: 'play.inspect', entity: 'X' as never })).toBe('play.inspect');
    expect(capabilityOf({ type: 'play.input', generation: 1, events: [] })).toBe('play.input');
    expect(capabilityOf({ type: 'window.close' })).toBe('window.close');
  });
});
