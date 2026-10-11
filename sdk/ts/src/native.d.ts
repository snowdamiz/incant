/** Native QuickJS migration API. Separate from the document-backed ScriptApi. */
import type { InputFrame, ScriptClock, TimerEvent, PhysicsRayQuery, PhysicsRayHit, PhysicsCharacterQuery, PhysicsCharacterMovement, PhysicsTriggerEvent } from './generated';
/** VM-owned bounded copy; no native pointers/handles or JSON conversion.
 * Retention cannot hold an ECS loan or alter the next input frame. */
type InputReadonly<T> = T extends object ? { readonly [K in keyof T]: InputReadonly<T[K]> } : T;
export type NativeInput = InputReadonly<InputFrame>;
/** Accepted update count (first update is 1) and accumulated simulation seconds.
 * No wall-clock time; reset on host creation, not a save/VM snapshot. */
export type NativeClock = Readonly<ScriptClock>;
export type NativeTimerEvent = Readonly<Omit<TimerEvent, 'payload'>>;
export interface NativeTimers {
  /** Due events sorted by ID. Handle synchronously in update. Cancelling an
   * interval here stops future events, not the event already published. */
  readonly events: readonly NativeTimerEvent[];
  /** Replace the ID's timer; due after delayTicks additional accepted updates.
   * IDs: 1–64 ASCII letters/digits/_.:-; delays: positive u32 integers.
   * At most 128 pending timers, 256 set/cancel calls per update. No callbacks,
   * promises or serialized payloads. Store local gameplay data in the behavior. */
  set(id: string, delayTicks: number, intervalTicks?: number): void;
  /** Cancelling an absent ID is valid and still consumes one command. */
  cancel(id: string): void;
}
export type NativeId = string; // canonical uppercase, 26-character ULID
export type NativeVector3 = readonly [number, number, number] | Float64Array;
/** translation xyz, rotation xyzw (unit quaternion), scale xyz; length 10. */
export type NativeTransform = readonly [number, number, number, number, number, number, number, number, number, number] | Float64Array;
export interface NativeChunk {
  readonly ids: readonly NativeId[];
  readonly transformStride: 10;
  readonly velocityStride: 3;
  /** Direct ECS storage; detached at the end of update, including failure. */
  readonly transforms: Float64Array;
  readonly velocities: Float64Array | undefined;
}
/** Bounded to 65,536 operations per tick. Validated/applied after views detach.
 * Active-call errors reject numeric writes and the complete batch even when
 * caught, and fail the host. An expired function always throws; catching that
 * error alone does not poison the current update.
 * Functions belong to one tick only; retained functions never reactivate.
 * Vector/transform inputs are copied as fixed numeric fields, never JSON. */
export interface NativeCommands {
  /** Ordered with this same structural buffer. Seek resets phase/completion;
   * play on a completed once sequence restarts it. Readback remains the committed
   * pre-command snapshot until the next update. Invalid controls reject the
   * whole callback, including numeric writes and earlier queued commands. */
  controlFlipbook(id: NativeId, control: 'play' | 'pause' | 'restart'): void;
  controlFlipbook(id: NativeId, control: 'seek', frame: number): void;
  spawn(id: NativeId, transform: NativeTransform, parent: NativeId | null, velocity: NativeVector3 | null): void;
  /** No cascading deletion. Reparent/remove children in the same batch. */
  despawn(id: NativeId): void;
  /** Keeps local Transform. Parent may be spawned later in this batch. */
  setParent(id: NativeId, parent: NativeId | null): void;
  /** Add, replace or remove the actual Velocity component. */
  setVelocity(id: NativeId, velocity: NativeVector3 | null): void;
}
/** Schema-derived query; the active native scene is implicit. */
export type NativeRayQuery = Omit<PhysicsRayQuery, 'scene_id'>;
export type NativeRayHit = PhysicsRayHit;
export type NativeCharacterQuery = Omit<PhysicsCharacterQuery, 'scene_id'>;
export type NativeCharacterMovement = PhysicsCharacterMovement;
export type NativeTriggerEvent = PhysicsTriggerEvent;
export interface NativePhysics {
  /** Computes a world-space displacement against committed bodies without moving
   * them. Apply it through the existing columns/commands. Up to 128 per update,
   * within the shared 1,024-call query budget; the update dt is implicit. */
  computeCharacterMotion(query: NativeCharacterQuery): NativeCharacterMovement;
  /** Owned sorted events from the last committed solver step. Does not drain.
   * Pending movement/commands affect events only after a later solver step. */
  triggerEvents(): NativeTriggerEvent[];
  /** Within the shared 1,024-call budget per update; invalid/caught queries reject the whole update.
   * Reads the last committed solver, not pending column writes or commands.
   * A retained function expires after update and never reactivates.
   * Returns owned values; mutating a result cannot alter native state. */
  raycast(query: NativeRayQuery): NativeRayHit | null;
}
export type NativeLogLevel = 'debug' | 'info' | 'warn' | 'error';
/** Bulk committed state captured after simulation, before script commands.
 * Entries with the same index belong together; order is unspecified. Flags:
 * bit 0 = playing, bit 1 = completed, all other bits zero. These are VM-owned
 * copies, not borrowed ECS columns: retaining/mutating them cannot change the
 * world or a later snapshot. At most 32,768 rows, within the VM memory budget. */
export interface NativeFlipbooks {
  readonly ids: readonly NativeId[];
  readonly frames: Uint32Array;
  readonly flags: Uint32Array;
}
export interface NativeContext {
  readonly flipbooks: NativeFlipbooks;
  readonly chunks: readonly NativeChunk[];
  readonly dt: number;
  readonly commands: NativeCommands;
  readonly input: NativeInput;
  readonly physics: NativePhysics;
  readonly clock: NativeClock;
  /** Functions expire after this update. Invalid/caught commands reject numeric,
   * structural and scheduling changes together and permanently fail the host. */
  readonly timers: NativeTimers;
  /** Primitive strings only; defaults to info. At most 1,024 UTF-8 bytes per
   * message, 64 messages / 16 KiB per update. Invalid/caught calls reject the
   * whole script transaction. The function expires when this update ends.
   * A session retains the first 1,024 entries / 256 KiB and counts later drops.
   * Logs publish after successful commit; failed sessions hide all outputs. */
  readonly log: (message: string, level?: NativeLogLevel) => void;
}
export interface NativeBehavior {
  /** Synchronous only; no promises, deferred work or non-undefined return. */
  update(context: NativeContext): undefined;
}
