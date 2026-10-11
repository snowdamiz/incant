/** Native QuickJS migration API. Separate from the document-backed ScriptApi. */
import type { LocaleSettings, LocalizeRequest, LocalizedText, CalendarDate, DateLength, LocalizationReportRequest, LocalizationReport, InputFrame, ScriptClock, TimerEvent, PhysicsRayQuery, PhysicsRayHit, PhysicsCharacterQuery, PhysicsCharacterMovement, PhysicsTriggerEvent, PhysicsRayQuery2D, PhysicsRayHit2D, PhysicsTriggerEvent2D, NavigationQuery, NavigationPath, GridNavigationQuery, GridPath } from './generated';
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
export type NativeProp = null | boolean | number | string | readonly NativeProp[] | { readonly [key: string]: NativeProp };
export interface NativeInstance {
  readonly entity: NativeId;
  readonly props: { readonly [key: string]: NativeProp };
}
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
  /** Buffered runtime locale change; visible on the next accepted update. At
   * most 64 per tick within the shared structural quota. The entire numeric,
   * locale and structural batch rolls back on refusal. Catalog is immutable. */
  setLocale(settings: LocaleSettings): void;
  /** Desired playing state for an existing source. False pauses its retained
   * cursor; true starts an unstarted source or resumes a paused one. Unchanged
   * true does not replay an ended voice. False then true needs separate sync
   * points to create a new edge; this API does not expose stop/restart. */
  setAudioPlaying(id: NativeId, playing: boolean): void;
  /** Source gain in finite [-80,24] dB, preserving clip, cursor and routing. */
  setAudioGain(id: NativeId, gainDb: number): void;
  /** Gain for an existing bus in finite [-80,24] dB; preserves bus topology.
   * These controls share the structural quota/lifetime and atomic rollback. */
  setAudioBusGain(id: NativeId, gainDb: number): void;
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
export type NativeRayQuery2D = PhysicsRayQuery2D;
export type NativeRayHit2D = PhysicsRayHit2D;
export type NativeTriggerEvent2D = PhysicsTriggerEvent2D;
export interface NativePhysics {
  /** XY ray against committed 2D colliders and solid tilemaps. Direction is
   * normalized; zero direction refuses. Starting inside a shape hits at zero.
   * include_sensors, exclude_entity and symmetric membership/filter masks apply.
   * Shares the 1,024-call budget, active-tick lifetime and rollback below. */
  raycast2D(query: NativeRayQuery2D): NativeRayHit2D | null;
  /** Owned sorted planar sensor enter/exit events from the last solver step.
   * Active scene is implicit; reading does not drain or include pending writes.
   * Shares the query budget and active-tick lifetime with raycast2D/raycast. */
  triggerEvents2D(): NativeTriggerEvent2D[];
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
export type NativePathRequest = NavigationQuery['path'];
export type NativeNavigationPath = NavigationPath;
export type NativeGridPathRequest = GridNavigationQuery['path'];
export type NativeGridPath = GridPath;
/** Read-only committed navigation in the active native scene. All results are
 * owned mutable values, safe to retain. Functions expire after this update and
 * never reactivate. Active-call errors, even caught, reject the whole update.
 * Up to 32 calls within the shared 1,024 physics/navigation query quota. The
 * sum of requested max_visited/max_expansions is at most 65,536 per update,
 * including misses. CPU deadline checked before/after bounded native searches;
 * native search itself is synchronous. Queries do not clone caches or geometry. */
export interface NativeNavigation {
  /** World-space mesh path or null when disconnected/outside snap distance.
   * Points follow the quantized surface; use physics for exact grounding.
   * Each traversal identifies a consecutive point pair across an off-mesh link;
   * gameplay must explicitly handle it, never treat it as a walkable segment.
   * Search exhaustion throws. Generation changes invalidate retained paths.
   * Generations are scoped to this native world and owner; never reuse paths
   * across sessions/worlds. Save restore advances the retained generation.
   * Pending numeric writes/commands are invisible until commit. */
  findPath(entity: NativeId, request: NativePathRequest): NativeNavigationPath | null;
  /** Adjacent grid cell indices or null for blocked/disconnected endpoints.
   * Diagonal movement (default false) never cuts blocked corners. World mapping,
   * clearance and movement belong to gameplay. Default max_expansions=65,536
   * consumes the entire tick budget; set it explicitly to make multiple queries.
   * An absent mesh/grid, invalid endpoint, or exhausted search throws. */
  findGridPath(entity: NativeId, request: NativeGridPathRequest): NativeGridPath | null;
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
/** Read-only committed locale/catalog. Each call costs four of the shared
 * 1,024 native query units, at most 64 localization calls per whole tick.
 * Arguments together are bounded to 64 KiB and returned text/metadata to 256 KiB.
 * Inputs/results are owned typed values. Active errors poison the whole tick,
 * even when caught; retained methods never reactivate. ICU formatters use
 * compiled data; this API neither reads source files nor renders fonts. */
export interface NativeLocalization {
  localize(request: LocalizeRequest): LocalizedText;
  locale(): LocaleSettings;
  formatNumber(value: number): string;
  formatDate(value: CalendarDate, length?: DateLength): string;
  /** Prepared coverage of declared keys, in canonical table/key order. Offset
   * 0..4096; limit 1..128 (default 128). Unknown runtime keys are returned by
   * localize().missing. No report accumulates mutable per-tick diagnostics. */
  report(request?: LocalizationReportRequest): LocalizationReport;
}
export interface NativeContext {
  /** Authored attachments for this source, sorted by stable entity ID. The array
   * and initial props are deeply frozen. One callback per nonempty source runs
   * in source-ID order with the same world-wide bulk views/transaction. A queued
   * despawn changes membership only after commit; plain Spawn never reattaches,
   * even when reusing an ID. Standalone behaviors receive an empty array.
   * Dynamic attachment, props replacement and legacy state hooks are unsupported. */
  readonly instances: readonly NativeInstance[];
  readonly flipbooks: NativeFlipbooks;
  readonly chunks: readonly NativeChunk[];
  readonly dt: number;
  readonly commands: NativeCommands;
  readonly input: NativeInput;
  readonly physics: NativePhysics;
  readonly navigation: NativeNavigation;
  readonly localization: NativeLocalization;
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
