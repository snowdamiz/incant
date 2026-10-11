import type { BridgeResult, PlayInput } from '../bridge/contract';
import type { KeyCode } from './keys';
import { keyCodeOf } from './keys';

/** Worker limits (editor/app/src/play): 128 events per packet, 1024 pending per tick. */
export const MAX_PACKET = 128;
export const MAX_PENDING = 1024;
/** At most one packet per display frame: the game consumes input per tick, not per DOM event. */
export const PACKET_INTERVAL_MS = 16;
/** After a refused packet, wait for the next status read (4 Hz) before sending again. */
export const RETRY_DELAY_MS = 250;
/** The worker refuses pointer and wheel values beyond this magnitude. */
const LIMIT = 1_000_000;

export type GameButton = 'left' | 'right' | 'middle';
type Wheel = Extract<PlayInput, { type: 'wheel' }>;

/** DOM `MouseEvent.button` as an engine desktop button; back and forward are not supported. */
export function buttonOf(button: number): GameButton | null {
  return button === 0 ? 'left' : button === 1 ? 'middle' : button === 2 ? 'right' : null;
}

/**
 * A DOM wheel event in the engine's convention, which the native window player passes
 * through from tao: +x rightward and +y away from the user (the wheel rolled up). DOM
 * `deltaY` is positive when scrolling down, so it is negated. Page units are dropped.
 */
export function wheelOf(deltaX: number, deltaY: number, deltaMode: number): Wheel | null {
  const unit = deltaMode === 0 ? 'pixels' : deltaMode === 1 ? 'lines' : null;
  const delta: [number, number] = [deltaX, deltaY === 0 ? 0 : -deltaY];
  return unit && inRange(delta) ? { type: 'wheel', delta, unit } : null;
}

function inRange(pair: readonly number[]): boolean {
  return pair.every((value) => Number.isFinite(value) && Math.abs(value) <= LIMIT);
}

const clampSum = (a: number, b: number) => Math.max(-LIMIT, Math.min(LIMIT, a + b));

export interface GameInputState {
  /** The current session's generation, or null without a session. */
  readonly generation: number | null;
  /** The last status says the worker takes input: running, not suspended, readable. */
  readonly accepting: boolean;
  /**
   * The game view has deliberate keyboard focus in a focused window, and no Play control
   * request is in flight: input may be collected for the game.
   */
  readonly focused: boolean;
}

export interface GameInputOptions {
  /** Sends one packet, labelled with the generation its events were collected in. */
  readonly send: (generation: number, events: readonly PlayInput[]) => Promise<BridgeResult>;
  /** A lasting input failure, once per distinct message; null when packets succeed again. */
  readonly onError?: (message: string | null) => void;
  readonly now?: () => number;
  readonly setTimer?: (callback: () => void, delay: number) => unknown;
  readonly clearTimer?: (handle: unknown) => void;
}

/**
 * Desktop game input for native Play, collected only while the viewport is focused and
 * the session runs. Framework-free so its bounds are testable:
 * - every packet carries the generation its events were collected in; a new generation
 *   drops queued events and ignores a late reply, never relabelling them;
 * - one request in flight, at most one packet per frame, at most 128 events each;
 * - a run of pointer positions keeps its first and latest (as the native window player
 *   does), and consecutive wheel deltas of one unit are summed;
 * - held keys and buttons are tracked: releases are sent only for sent presses. Every
 *   stream opens with a focus cycle, which releases whatever the game still holds; a
 *   stream the editor ends while the worker still takes input, a refused packet or a
 *   backlog of 1024 events sends a release instead of leaving a key held in the game.
 */
export class GameInput {
  private state: GameInputState = { generation: null, accepting: false, focused: false };
  private queue: PlayInput[] = [];
  /** Index after the last non-pointer event: where the trailing pointer run starts. */
  private motionStart = 0;
  private sending = false;
  private timer: unknown = undefined;
  private lastSent = Number.NEGATIVE_INFINITY;
  private retryAt = Number.NEGATIVE_INFINITY;
  /** The pointer position the game was last given; a focus event clears it in the game. */
  private lastPosition: readonly [number, number] | null = null;
  /** A refused packet's message, reported only if the next packet is refused too. */
  private suspect: string | null = null;
  private error: string | null = null;
  private readonly keys = new Set<KeyCode>();
  private readonly buttons = new Set<GameButton>();
  private readonly now: () => number;
  private readonly setTimer: (callback: () => void, delay: number) => unknown;
  private readonly clearTimer: (handle: unknown) => void;

  constructor(private readonly options: GameInputOptions) {
    this.now = options.now ?? (() => performance.now());
    this.setTimer = options.setTimer ?? ((callback, delay) => setTimeout(callback, delay));
    this.clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));
  }

  /** True while events are collected and sent. */
  get active(): boolean {
    const { generation, accepting, focused } = this.state;
    return generation !== null && accepting && focused;
  }

  update(next: GameInputState): void {
    const before = this.state;
    // Another session starts its own stream, opened like any other.
    const wasActive = this.active && next.generation === before.generation;
    this.state = next;
    if (next.generation !== before.generation) {
      // Collected input belongs to its own session only.
      this.drop();
      this.suspect = null;
      this.retryAt = Number.NEGATIVE_INFINITY;
      this.setError(null);
    }
    const active = this.active;
    if (!wasActive && active) {
      // The game may still hold keys from before an interruption this editor could not
      // report (an unreadable status, a request that failed): release them first.
      this.push({ type: 'focus', focused: false });
      this.push({ type: 'focus', focused: true });
    } else if (wasActive && !active) {
      this.keys.clear();
      this.buttons.clear();
      // Focus left the view or a control request is in flight while the worker still takes
      // input: the game must release what it holds. Paused, suspended, stopped or
      // unreadable: the worker released input itself, or nothing can be sent.
      if (next.accepting) this.push({ type: 'focus', focused: false });
      else this.drop();
    }
    this.schedule();
  }

  /** True when the key belongs to the game; the caller then prevents its default action. */
  key(code: string, down: boolean, repeat = false): boolean {
    if (!this.active) return false;
    const key = keyCodeOf(code);
    // Keys the engine does not define and auto-repeat are dropped, never sent.
    if (!key || (down && repeat)) return true;
    if (down ? this.keys.has(key) : !this.keys.has(key)) return true;
    this.makeRoom();
    if (down) this.keys.add(key);
    else this.keys.delete(key);
    this.push({ type: 'key', code: key, down });
    this.schedule();
    return true;
  }

  /** True when the button belongs to the game. Back and forward buttons are dropped. */
  button(index: number, down: boolean): boolean {
    if (!this.active) return false;
    const button = buttonOf(index);
    if (!button || (down ? this.buttons.has(button) : !this.buttons.has(button))) return true;
    this.makeRoom();
    if (down) this.buttons.add(button);
    else this.buttons.delete(button);
    this.push({ type: 'mouse_button', button, down });
    this.schedule();
    return true;
  }

  /** Viewport-local CSS pixels. */
  pointer(x: number, y: number): void {
    if (!this.active) return;
    const position: [number, number] = [x, y];
    if (!inRange(position)) return;
    // A press or release where the pointer already is adds nothing.
    if (this.lastPosition?.[0] === x && this.lastPosition[1] === y) return;
    this.makeRoom();
    const event: PlayInput = { type: 'pointer_position', position };
    if (this.queue.length - this.motionStart >= 2) this.queue[this.queue.length - 1] = event;
    else this.push(event);
    this.lastPosition = position;
    this.schedule();
  }

  /** True when the wheel event belongs to the game. */
  wheel(deltaX: number, deltaY: number, deltaMode: number): boolean {
    if (!this.active) return false;
    const event = wheelOf(deltaX, deltaY, deltaMode);
    if (!event) return true;
    this.makeRoom();
    const last = this.queue[this.queue.length - 1];
    if (last?.type === 'wheel' && last.unit === event.unit) {
      this.queue[this.queue.length - 1] = {
        type: 'wheel',
        unit: event.unit,
        delta: [clampSum(last.delta[0], event.delta[0]), clampSum(last.delta[1], event.delta[1])],
      };
    } else this.push(event);
    this.schedule();
    return true;
  }

  /** Drops everything. A later update() starts again, as React re-runs effects in StrictMode. */
  dispose(): void {
    this.state = { generation: null, accepting: false, focused: false };
    this.drop();
    this.suspect = null;
    this.retryAt = Number.NEGATIVE_INFINITY;
  }

  private push(event: PlayInput): void {
    this.queue.push(event);
    if (event.type !== 'pointer_position') this.motionStart = this.queue.length;
    if (event.type === 'focus') this.lastPosition = null;
  }

  /** A backlog this long means packets are not being accepted: release instead of growing. */
  private makeRoom(): void {
    if (this.queue.length < MAX_PENDING) return;
    this.release();
    this.setError('Game input fell behind, so held keys and buttons were released.');
  }

  /**
   * Replaces the backlog with a release of every key and button in the game, followed by
   * focus again when the view still has it. Nothing is queued once the worker no longer
   * takes input; it released input itself.
   */
  private release(): void {
    this.queue = [];
    this.motionStart = 0;
    this.lastPosition = null;
    this.keys.clear();
    this.buttons.clear();
    if (this.state.generation === null || !this.state.accepting) return;
    this.push({ type: 'focus', focused: false });
    if (this.active) this.push({ type: 'focus', focused: true });
  }

  private drop(): void {
    this.queue = [];
    this.motionStart = 0;
    this.lastPosition = null;
    this.keys.clear();
    this.buttons.clear();
    if (this.timer !== undefined) {
      this.clearTimer(this.timer);
      this.timer = undefined;
    }
  }

  private schedule(): void {
    if (this.sending || this.timer !== undefined) return;
    if (this.queue.length === 0 || this.state.generation === null) return;
    const now = this.now();
    const wait = Math.max(0, this.lastSent + PACKET_INTERVAL_MS - now, this.retryAt - now);
    this.timer = this.setTimer(() => {
      this.timer = undefined;
      void this.flush();
    }, wait);
  }

  private async flush(): Promise<void> {
    const generation = this.state.generation;
    if (this.sending || generation === null || this.queue.length === 0) return;
    const packet = this.queue.splice(0, MAX_PACKET);
    this.motionStart = Math.max(0, this.motionStart - packet.length);
    this.sending = true;
    this.lastSent = this.now();
    let result: BridgeResult;
    try {
      result = await this.options.send(generation, packet);
    } catch (error) {
      result = { ok: false, error: { code: 'play.input', message: error instanceof Error ? error.message : String(error) } };
    }
    this.sending = false;
    // A later session never receives this packet's outcome or a resend, but its own
    // events, collected meanwhile, still go out.
    if (generation === this.state.generation) {
      if (result.ok) {
        this.suspect = null;
        this.setError(null);
      } else if (this.state.accepting) {
        // Lost presses or releases: release everything, and wait for the next status read
        // before sending again. A single refusal is usually a pause or suspension racing
        // the packet, so it is reported only when the next packet is refused as well.
        this.retryAt = this.now() + RETRY_DELAY_MS;
        this.release();
        if (this.suspect !== null) this.setError(result.error.message);
        this.suspect = result.error.message;
      }
    }
    this.schedule();
  }

  private setError(message: string | null): void {
    if (message === this.error) return;
    this.error = message;
    this.options.onError?.(message);
  }
}
