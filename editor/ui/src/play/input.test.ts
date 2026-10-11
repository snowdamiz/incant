import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BridgeResult, PlayInput } from '../bridge/contract';
import type { GameInputState } from './input';
import { GameInput, MAX_PACKET, MAX_PENDING, PACKET_INTERVAL_MS, RETRY_DELAY_MS, buttonOf, wheelOf } from './input';

interface Sent {
  generation: number;
  events: PlayInput[];
  settle: (result: BridgeResult) => void;
}

/** A recording sink whose replies the test releases, so in-flight packets can be held. */
function sink() {
  const sent: Sent[] = [];
  const errors: (string | null)[] = [];
  const input = new GameInput({
    send: (generation, events) => new Promise<BridgeResult>((settle) => sent.push({ generation, events: [...events], settle })),
    onError: (message) => errors.push(message),
  });
  /** Sends what is due and lets the reply land. */
  const flush = async (ms = PACKET_INTERVAL_MS) => {
    await vi.advanceTimersByTimeAsync(ms);
  };
  const ok = async (index = sent.length - 1) => {
    sent[index]!.settle({ ok: true });
    await vi.advanceTimersByTimeAsync(0);
  };
  const refuse = async (message: string, index = sent.length - 1) => {
    sent[index]!.settle({ ok: false, error: { code: 'engine.request', message } });
    await vi.advanceTimersByTimeAsync(0);
  };
  return { input, sent, errors, flush, ok, refuse };
}

const state = (generation: number | null, accepting: boolean, focused: boolean): GameInputState => ({ generation, accepting, focused });
const live = (generation: number | null = 4) => state(generation, true, true);
/** Every stream opens by releasing whatever the game still holds. */
const OPEN: PlayInput[] = [
  { type: 'focus', focused: false },
  { type: 'focus', focused: true },
];

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('game input queue', () => {
  it('collects nothing until a running session has a deliberately focused view', async () => {
    const { input, sent, flush } = sink();
    expect(input.key('KeyW', true)).toBe(false);
    input.update(state(4, true, false));
    expect(input.key('KeyW', true)).toBe(false);
    input.update(state(4, false, true));
    input.pointer(10, 10);
    await flush(100);
    expect(sent).toEqual([]);
  });

  it('opens with a focus cycle, then sends keys and buttons in order under the session generation', async () => {
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    expect(input.key('KeyW', true)).toBe(true);
    input.button(0, true);
    await flush();
    expect(sent).toHaveLength(1);
    expect(sent[0]!.generation).toBe(4);
    expect(sent[0]!.events).toEqual([...OPEN, { type: 'key', code: 'KeyW', down: true }, { type: 'mouse_button', button: 'left', down: true }]);
    await ok();
  });

  it('keeps one packet in flight, at most 128 events each, in order', async () => {
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    await flush();
    // Hold the first packet; 300 presses and releases queue behind it.
    const codes = ['KeyA', 'KeyB', 'KeyC', 'KeyD', 'KeyE'];
    for (let index = 0; index < 150; index += 1) {
      const code = codes[index % codes.length]!;
      input.key(code, true);
      input.key(code, false);
    }
    await flush(200);
    expect(sent).toHaveLength(1);
    await ok();
    await flush();
    expect(sent).toHaveLength(2);
    expect(sent[1]!.events).toHaveLength(MAX_PACKET);
    await ok();
    await flush();
    await ok();
    await flush();
    const events = sent.slice(1).flatMap((packet) => packet.events);
    expect(sent.slice(1).map((packet) => packet.events.length)).toEqual([128, 128, 44]);
    expect(events).toHaveLength(300);
    expect(events.slice(0, 4)).toEqual([
      { type: 'key', code: 'KeyA', down: true },
      { type: 'key', code: 'KeyA', down: false },
      { type: 'key', code: 'KeyB', down: true },
      { type: 'key', code: 'KeyB', down: false },
    ]);
    expect(sent.every((packet) => packet.events.length <= MAX_PACKET)).toBe(true);
  });

  it('sends at most one packet per frame however many pointer events arrive', async () => {
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    await flush();
    await ok();
    for (let index = 0; index < 500; index += 1) input.pointer(index, index / 2);
    await flush();
    // A run keeps its first and latest position, as the native window player does.
    expect(sent[1]!.events).toEqual([
      { type: 'pointer_position', position: [0, 0] },
      { type: 'pointer_position', position: [499, 249.5] },
    ]);
    await ok();
    await flush(PACKET_INTERVAL_MS * 3);
    expect(sent).toHaveLength(2);
  });

  it('skips a position the game already has, and drops invalid coordinates', async () => {
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    input.pointer(80, 60);
    input.pointer(80, 60);
    input.button(0, true);
    input.pointer(Number.NaN, 2);
    input.pointer(2_000_000, 2);
    await flush();
    await ok();
    // Already sent: a release at the same place adds no position.
    input.pointer(80, 60);
    input.button(0, false);
    await flush();
    expect(sent.map((packet) => packet.events)).toEqual([
      [...OPEN, { type: 'pointer_position', position: [80, 60] }, { type: 'mouse_button', button: 'left', down: true }],
      [{ type: 'mouse_button', button: 'left', down: false }],
    ]);
  });

  it('sums wheel deltas per unit in the engine convention and drops page units', async () => {
    const { input, sent, flush } = sink();
    input.update(live(4));
    input.wheel(0, 40, 0);
    input.wheel(5, 20, 0);
    expect(input.wheel(0, 3, 2)).toBe(true);
    input.wheel(0, -1, 1);
    await flush();
    expect(sent[0]!.events.slice(OPEN.length)).toEqual([
      { type: 'wheel', delta: [5, -60], unit: 'pixels' },
      { type: 'wheel', delta: [0, 1], unit: 'lines' },
    ]);
    expect(wheelOf(0, 0, 0)).toEqual({ type: 'wheel', delta: [0, 0], unit: 'pixels' });
    expect(wheelOf(0, Number.POSITIVE_INFINITY, 0)).toBeNull();
  });

  it('drops auto-repeat, unsupported keys and buttons, and releases it never pressed', async () => {
    const { input, sent, flush } = sink();
    input.update(live(4));
    expect(input.key('KeyW', true)).toBe(true);
    expect(input.key('KeyW', true, true)).toBe(true);
    expect(input.key('KeyW', true)).toBe(true);
    expect(input.key('IntlBackslash', true)).toBe(true);
    expect(input.key('KeyQ', false)).toBe(true);
    expect(input.button(3, true)).toBe(true);
    expect(input.button(2, false)).toBe(true);
    expect(buttonOf(1)).toBe('middle');
    expect(buttonOf(4)).toBeNull();
    await flush();
    expect(sent[0]!.events).toEqual([...OPEN, { type: 'key', code: 'KeyW', down: true }]);
  });

  it('tells the game when the view loses focus and forgets held keys', async () => {
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    await ok();
    input.update(state(4, true, false));
    await flush();
    expect(sent[1]!.events).toEqual([{ type: 'focus', focused: false }]);
    await ok();
    // Focus back: the old press is not released again; the stream opens afresh.
    input.update(live(4));
    expect(input.key('KeyW', false)).toBe(true);
    await flush();
    expect(sent[2]!.events).toEqual(OPEN);
  });

  it('tells the game it has no focus whenever the worker starts taking input while another panel has it', async () => {
    // The worker gives the game focus itself when Play starts or resumes and when the
    // window comes back (found in the native review: Resume with the Inspector focused).
    const { input, sent, flush, ok } = sink();
    input.update(state(4, true, false));
    expect(input.key('KeyW', true)).toBe(false);
    await flush();
    expect(sent[0]!.events).toEqual([{ type: 'focus', focused: false }]);
    await ok();
    // Paused, then resumed, still from another panel: said again, once.
    input.update(state(4, false, false));
    input.update(state(4, true, false));
    input.update(state(4, true, false));
    await flush();
    expect(sent[1]!.events).toEqual([{ type: 'focus', focused: false }]);
    await ok();
    // A new session that starts without the view's focus is told as well.
    input.update(state(5, true, false));
    await flush();
    expect(sent.map((packet) => packet.generation)).toEqual([4, 4, 5]);
    expect(sent[2]!.events).toEqual([{ type: 'focus', focused: false }]);
    await ok();
    // The view takes focus: the stream opens as always.
    input.update(live(5));
    await flush();
    expect(sent[3]!.events).toEqual(OPEN);
  });

  it('drops a focus notice that the worker stops taking before it is sent', async () => {
    const { input, sent, flush } = sink();
    input.update(state(4, true, false));
    input.update(state(4, false, false));
    await flush(100);
    expect(sent).toEqual([]);
  });

  it('releases held input when the editor holds it back but the worker still takes it', async () => {
    // A Pause request in flight: the view stays focused but collects nothing.
    const { input, sent, flush, ok } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    await ok();
    input.update(state(4, true, false));
    await flush();
    expect(sent[1]!.events).toEqual([{ type: 'focus', focused: false }]);
  });

  it('drops queued input when the worker pauses, suspends or the status is unreadable; it released input itself', async () => {
    const { input, sent, flush } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    input.update(state(4, false, true));
    await flush(100);
    expect(sent).toEqual([]);
    expect(input.key('KeyW', false)).toBe(false);
  });

  it('never relabels collected input for a new session and ignores a late reply', async () => {
    const { input, sent, errors, flush, refuse } = sink();
    input.update(live(4));
    await flush();
    input.key('KeyW', true);
    input.key('KeyD', true);
    // The session is replaced while the first packet is in flight and two presses wait.
    input.update(live(5));
    input.key('KeyA', true);
    await refuse('Game input session changed or is unavailable.', 0);
    await flush();
    expect(sent.map((packet) => packet.generation)).toEqual([4, 5]);
    expect(sent[1]!.events).toEqual([...OPEN, { type: 'key', code: 'KeyA', down: true }]);
    expect(errors).toEqual([]);
  });

  it('still releases held keys when the packet after a blur is refused', async () => {
    const { input, sent, flush, ok, refuse } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    await ok();
    input.key('KeyD', true);
    await flush();
    // The view loses focus while KeyD is in flight; that packet is then refused.
    input.update(state(4, true, false));
    await refuse('Play request queue is full; retry shortly');
    await flush(RETRY_DELAY_MS);
    expect(sent.at(-1)!.events).toEqual([{ type: 'focus', focused: false }]);
    // And again if the release itself is refused.
    await refuse('Play request queue is full; retry shortly');
    await flush(RETRY_DELAY_MS);
    expect(sent.at(-1)!.events).toEqual([{ type: 'focus', focused: false }]);
    await ok();
    await flush(RETRY_DELAY_MS * 2);
    expect(sent).toHaveLength(4);
  });

  it('releases everything and says so once when 1024 events back up behind a stuck packet', async () => {
    const { input, sent, errors, flush } = sink();
    input.update(live(4));
    await flush();
    for (let index = 0; index < MAX_PENDING + 10; index += 1) input.wheel(0, 1, index % 2);
    expect(errors).toEqual(['Game input fell behind, so held keys and buttons were released.']);
    for (let index = 0; index < MAX_PENDING * 2; index += 1) input.wheel(0, 1, index % 2);
    expect(errors).toHaveLength(1);
    expect(sent).toHaveLength(1);
  });

  it('reports a refusal only when it repeats, releases held input and waits before retrying', async () => {
    const { input, sent, errors, flush, ok, refuse } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    await refuse('Game input queue is full');
    // One refusal is usually a pause racing the packet: nothing is reported yet.
    expect(errors).toEqual([]);
    input.key('KeyD', true);
    await flush(RETRY_DELAY_MS - 20);
    expect(sent).toHaveLength(1);
    await flush(40);
    expect(sent).toHaveLength(2);
    // The lost press is released in the game by a focus cycle before new input.
    expect(sent[1]!.events).toEqual([...OPEN, { type: 'key', code: 'KeyD', down: true }]);
    await refuse('Game input queue is full');
    expect(errors).toEqual(['Game input queue is full']);
    await flush(RETRY_DELAY_MS);
    await refuse('Game input queue is full');
    expect(errors).toEqual(['Game input queue is full']);
    await flush(RETRY_DELAY_MS);
    await ok();
    expect(errors).toEqual(['Game input queue is full', null]);
  });

  it('stays silent about refusals once the worker no longer takes input', async () => {
    const { input, sent, errors, flush, refuse } = sink();
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    input.key('KeyD', true);
    // Paused meanwhile: the in-flight packet is refused, and nothing is retried or reported.
    input.update(state(4, false, true));
    await refuse('Game input is inactive');
    await flush(RETRY_DELAY_MS * 2);
    expect(sent).toHaveLength(1);
    // Running again: a second refusal is still only a suspicion, reported only if it repeats.
    input.update(live(4));
    await flush();
    expect(sent).toHaveLength(2);
    expect(sent[1]!.events).toEqual(OPEN);
    await refuse('Game input is inactive');
    expect(errors).toEqual([]);
  });

  it('works again after dispose, as React StrictMode re-runs effects', async () => {
    const { input, sent, flush } = sink();
    input.update(live(4));
    input.dispose();
    await flush(100);
    expect(sent).toEqual([]);
    input.update(live(4));
    input.key('KeyW', true);
    await flush();
    expect(sent[0]!.events).toEqual([...OPEN, { type: 'key', code: 'KeyW', down: true }]);
  });
});
