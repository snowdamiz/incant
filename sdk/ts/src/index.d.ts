/** Sandbox API. Scripts cannot import host modules or access filesystem/network. */
export type Id = string;
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export interface Entity {
  readonly id: Id;
  readonly scene_id: Id;
  readonly name: string;
  readonly parent: Id | null;
  readonly components: Readonly<Record<string, Json>>;
}
export type { Command, Transform } from './generated';
/** Host-managed save envelope; scripts do not receive filesystem access. */
export type { GameSave, PlayAssertions } from './generated';
import type { Command } from './generated';
export type { AudioSource, AudioBus, AudioListener } from './generated';
export type { InputEvent, InputFrame, InputRecording } from './generated';
import type { InputFrame } from './generated';
export type { ScriptClock } from './generated';
import type { ScriptClock, TimerRequest as GeneratedTimerRequest, TimerEvent as GeneratedTimerEvent } from './generated';
export type TimerRequest = Omit<GeneratedTimerRequest, 'payload'> & { payload?: Json };
export type TimerEvent = Omit<GeneratedTimerEvent, 'payload'> & { payload: Json };
export type { PhysicsRayQuery as RayQuery, PhysicsRayHit as RayHit, PhysicsTriggerEvent as TriggerEvent } from './generated';
import type { PhysicsRayQuery as RayQuery, PhysicsRayHit as RayHit, PhysicsTriggerEvent as TriggerEvent } from './generated';
export type { PhysicsCharacterQuery as CharacterQuery, PhysicsCharacterMovement as CharacterMovement } from './generated';
import type { PhysicsCharacterQuery as CharacterQuery, PhysicsCharacterMovement as CharacterMovement } from './generated';
export interface ScriptApi {
  /** Clock of the tick being processed (first update is tick 1). Saved/restored. */
  clock(): ScriptClock;
  /** Create/replace a named timer, delivered to onTimer before update. Delays
   * and intervals are positive fixed ticks; at most 128 timers and 256 actions
   * per tick. A same-tick cancellation/replacement suppresses later callbacks. */
  setTimer(timer: TimerRequest): void;
  cancelTimer(id: string): void;
  /** Validated physical input for this tick. Querying returns an isolated copy.
   * Positions/motion are logical pixels. Sticks are raw [-1,1], +Y down; analog
   * buttons are [0,1], with a pressed threshold of 0.5. Edges last one tick. */
  input(): InputFrame;
  /** Read-only sweep for a nonsensor kinematic body. World +Y is up. Costs 16 of
   * the shared 256 physics-query units per tick. Apply translation/dt as Velocity
   * through api.command for the next tick. Gravity/jumping are behavior-owned. */
  computeCharacterMotion(query: CharacterQuery): CharacterMovement;
  /** Play-session query; at most 256 per tick. Direction is normalized. */
  raycast(query: RayQuery): RayHit | null;
  /** Sorted entry/exit transitions for the just-completed simulation tick. */
  triggerEvents(): readonly TriggerEvent[];
  query(component?: string): readonly Entity[];
  command(command: Command): void;
  /** Structured output, published only after this tick commits. Maximum 64 messages
   * per tick and 4096 UTF-8 bytes per message; no host console or file access. */
  log(message: string, level?: 'debug' | 'info' | 'warn' | 'error'): void;
}
export interface Behavior<State extends Record<string, Json>> {
  /** Keep durable gameplay data here. Save/load preserves this JSON state;
   * module globals, closures, pending logs and physics contacts are rebuilt. */
  initialState: State;
  /** Synchronous only. Async/Promise and generator returns fail before commit. */
  update(api: ScriptApi, dt: number, state: State): undefined;
  /** Same command/state transaction and execution budget as update. Timer order
   * is (due tick, ID). Timer payload and pending schedule survive game saves. */
  onTimer?(api: ScriptApi, event: TimerEvent, state: State): undefined;
}
declare global {
  function defineBehavior<State extends Record<string, Json>>(behavior: Behavior<State>): Behavior<State>;
}
