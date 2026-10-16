/** Native QuickJS migration API. Separate from the document-backed ScriptApi. */
import type { CoroutineResult, LocaleSettings, LocalizeRequest, LocalizedText, CalendarDate, DateLength, LocalizationReportRequest, LocalizationReport, InputFrame, ScriptClock, TimerEvent, PhysicsRayQuery, PhysicsRayHit, PhysicsCharacterQuery, PhysicsCharacterMovement, PhysicsTriggerEvent, PhysicsRayQuery2D, PhysicsRayHit2D, PhysicsTriggerEvent2D, NavigationQuery, NavigationPath, GridNavigationQuery, GridPath, SteeringAgent, SteeringObstacle, SteeringQuery, SteeringVelocity } from './generated';
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
/** Host-issued ephemeral actuator identity. Generations expire on reconnect or
 * session replacement and must never be restored from saved gameplay state. */
export type HapticTarget =
  | { readonly kind: "gamepad"; readonly id: number; readonly generation: number }
  | { readonly kind: "device"; readonly generation: number };
export type HapticEffect =
  | { readonly kind: "rumble"; readonly strong: number; readonly weak: number; readonly durationMs: number }
  | { readonly kind: "impact"; readonly intensity: number };
/** Bounded to 65,536 operations per tick. Validated/applied after views detach.
 * Active-call errors reject numeric writes and the complete batch even when
 * caught, and fail the host. An expired function always throws; catching that
 * error alone does not poison the current update.
 * Functions belong to one tick only; retained functions never reactivate.
 * Vector/transform inputs are copied as fixed numeric fields, never JSON. */
export interface NativeCommands {
  /** Existing punctual light only; linear RGB finite [0,1]. Retains kind,
   * range/cones/shadows, topology and binding generation. Ordered with this
   * tick's complete atomic batch; caught active-call errors reject all writes.
   * Readback remains committed pre-command state. Environment is unsupported. */
  setLightColor(id: NativeId, color: NativeVector3): void;
  /** Finite [0,1_000_000], lux (directional) or candela (point/spot).
   * Uses the final SetSpatial declaration; missing/despawned/non-light refuses. */
  setLightIntensity(id: NativeId, intensity: number): void;
  /** Current host capabilities. Empty on ordinary/headless/mobile hosts; Wasm
   * parity is not yet implemented. Gamepad targets support rumble, device targets
   * impact. Query again after topology/session changes; never assume support. */
  hapticCapabilities(): ReadonlyArray<HapticTarget>;
  /** At most32 requests/tick within the shared command quota. Finite amplitudes
   * 0..1, integer rumble duration1..5000ms. Target generation and effect kind must
   * match current authority. Errors (even caught) reject the whole transaction.
   * Delivery follows complete game acceptance; OS acceptance is asynchronous,
   * not physical confirmation. Desktop timing is quantized upward to50ms. */
  playHaptic(target: HapticTarget, effect: HapticEffect): void;
  /** Stop the matching current target. This is also a committed bounded request. */
  stopHaptic(target: HapticTarget): void;

  /** Request whole-session replacement at the next accepted host boundary.
   * Requires an explicitly restart-capable native host; unsupported hosts refuse
   * and poison the active tick. No arguments, coalesced within the shared command
   * quota. Publication waits for all source callbacks, world, audio and consumer
   * acceptance. Does not reset this callback's state. Build-linked and development
   * Wasm expose the same request through the Rust SDK. */
  requestRestart(): void;

  /** Ordered existing-widget controls. Preserve role, fonts, layout and hierarchy.
   * Literal content is UTF-8 <=16KiB; use localization.localize(...).text for
   * localized values. At most 256 HUD controls and 1MiB text per whole tick,
   * within the shared structural quota. Invalid caught calls reject the tick.
   * A changed text-input declaration replaces its buffer; unchanged text keeps
   * user edits. Reconciliation retires old focus receipts/composition. */
  setUiText(id: NativeId, text: string): void;
  /** Disabled descendants keep layout but lose focus/input eligibility. */
  setUiDisabled(id: NativeId, disabled: boolean): void;
  /** Existing world-space Text content, preserving fonts and text layout. */
  setText(id: NativeId, text: string): void;
  /** Buffered runtime locale change; visible on the next accepted update. At
   * most 64 per tick within the shared structural quota. The entire numeric,
   * locale and structural batch rolls back on refusal. Catalog is immutable. */
  setLocale(settings: LocaleSettings): void;
  /** Desired playing state for an existing source. False pauses its retained
   * cursor; true starts an unstarted source or resumes a paused one. Unchanged
   * true does not replay an ended voice. False then true needs separate sync
   * points to create a new edge; use restartAudio for an explicit replay. */
  setAudioPlaying(id: NativeId, playing: boolean): void;
  /** Explicit ordered transport requests. Stop resets to the beginning without
   * playback; restart resets and plays, including completed one-shot voices.
   * Commands publish only after native transaction acceptance. Retired voices
   * drain before replacements begin; no overlapping playback is introduced. */
  stopAudio(id: NativeId): void;
  restartAudio(id: NativeId): void;
  /** Finite [0,3600] seconds, within the clip duration checked by the audio host.
   * Preserves desired playing state. Seeking to the same position still resets
   * the cursor; repeated setAudioPlaying(true) remains a no-op. */
  seekAudio(id: NativeId, seconds: number): void;
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
  /** Select a concrete clip (integer 0..255) or null for the rest pose. Seconds
   * must be finite/nonnegative and within the selected asset's duration. These
   * controls share the structural quota, tick lifetime and atomic rollback;
   * the committed snapshot does not reflect queued changes. */
  controlAnimation(id: NativeId, control: 'select', clip: number | null, seconds: number): void;
  /** Duration is finite 0..60 seconds; zero selects immediately. A nonzero
   * fade cannot interrupt an active fade. Select/Seek cancel it. */
  controlAnimation(id: NativeId, control: 'crossfade', clip: number | null, seconds: number, duration: number): void;
  controlAnimation(id: NativeId, control: 'seek', seconds: number): void;
  controlAnimation(id: NativeId, control: 'playing' | 'looping', enabled: boolean): void;
  /** Finite [-16,16]; negative speed plays backward, zero holds the sample. */
  controlAnimation(id: NativeId, control: 'speed', speed: number): void;
  /** Applying controls body movement, not root stripping. Reset clears only the
   * four observed deltas; neither command seeks or applies stored motion.
   * Shares the ordered atomic tick queue, including caught-error rollback. */
  controlRootMotion(id: NativeId, control: 'applying', enabled: boolean): void;
  controlRootMotion(id: NativeId, control: 'reset'): void;
  /** Set a declared graph parameter within its authored range. */
  setAnimationParameter(id: NativeId, parameter: NativeId, value: number): void;
  /** Ordered two-bone constraints solve after the full animation pose. Coordinates
   * are cooked MODEL space, before entity/world transforms; not world-space
   * targets. Finite coordinates within +/-1e9 and weight 0..1. Chain topology
   * and constraint IDs are immutable cooked data. Calls share the atomic tick
   * queue; invalid caught calls still fail the tick. Affine/zero-length sampled
   * chains refuse consumption explicitly; no stretching or silent skip. */
  setAnimationIk(id: NativeId, constraint: NativeId, tx: number, ty: number, tz: number,
    px: number, py: number, pz: number, weight: number): void;
  /** Immediately select a declared graph state; preserves play/pause and speed. */
  selectAnimationState(id: NativeId, state: NativeId): void;
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
export type NativeVector2 = readonly [number, number] | Float64Array;
export type NativeSteeringAgent = Omit<InputReadonly<SteeringAgent>, 'position' | 'velocity' | 'preferred_velocity'> & {
  readonly position: NativeVector3;
  readonly velocity: NativeVector2;
  readonly preferred_velocity: NativeVector2;
};
export type NativeSteeringObstacle = Omit<InputReadonly<SteeringObstacle>, 'vertices'> & {
  readonly vertices: readonly NativeVector2[];
};
export type NativeSteeringQuery = Omit<InputReadonly<SteeringQuery>, 'agents' | 'obstacles'> & {
  readonly agents: readonly NativeSteeringAgent[];
  readonly obstacles?: readonly NativeSteeringObstacle[];
};
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
  /** Bulk reciprocal avoidance proposals, sorted by stable ID. Input positions
   * are world-space feet (+Y up); velocities use XZ. IDs correlate values and
   * need not name scene entities. Apply proposals through ordinary gameplay
   * motion/physics; separation is not guaranteed for infeasible constraints.
   * Uses this update's dt. Maximum 128 agents and 128 obstacle edges total per
   * update, shared across calls/authored scripts, within the 32 navigation-call
   * quota. Up to 32 obstacles per call; margin defaults to 0.05 m. Fixed-field
   * copies, no JSON or document access. Owned results survive the tick. */
  steerAgents(request: NativeSteeringQuery): SteeringVelocity[];
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
/** Aligned committed animation playback, captured before script commands.
 * clips: -1 for the rest pose, otherwise an asset-local index 0..255. Flags:
 * bit 0 = playing, bit 1 = looping, bit 2 = completed; other bits zero.
 * VM-owned copies within its memory budget. Retention/mutation cannot change
 * native playback or a later snapshot; order is unspecified. */
/** Committed graph rows, a subset of animations.ids. Parameters for row i occupy
 * [parameterOffsets[i], parameterOffsets[i+1]) in both parameter columns.
 * Source sample/weight represents the same two-input pose as the renderer.
 * All arrays are owned snapshots; controls become visible after acceptance. */
export interface NativeAnimationGraphs {
  readonly ids: readonly NativeId[];
  readonly states: readonly NativeId[];
  /** Explicit master pause; completed poses may still evaluate transitions. */
  readonly paused: Uint8Array;
  readonly seconds: Float64Array;
  readonly elapsed: Float64Array;
  readonly parameterIds: readonly NativeId[];
  readonly parameterOffsets: Uint32Array;
  readonly parameterValues: Float64Array;
  readonly blending: Uint8Array;
  /** 1 for a weighted tree; full samples are in animations.pose* for the same ID.
   * sourceClips/sourceSeconds/weights describe only the legacy two-pose path. */
  readonly weighted: Uint8Array;
  /** Compound-state transition: active bit0, outgoing playing bit1. Source
   * state is null and numeric fields zero when absent. Both states read the
   * same committed parameters; outgoing speed is captured on transition. */
  readonly fadeStates: readonly (NativeId | null)[];
  readonly fadeFlags: Uint8Array;
  readonly fadeSeconds: Float64Array;
  readonly fadeSpeeds: Float64Array;
  readonly fadeElapsed: Float64Array;
  readonly fadeDuration: Float64Array;
  readonly sourceClips: Int32Array;
  readonly sourceSeconds: Float64Array;
  readonly weights: Float64Array;
}
/** Owned committed rows (<=4096 total, <=32/owner), ordered by entity then
 * authored constraint order. Three lanes per row in chains/targets/poles;
 * weights and both ID arrays have one lane per row. Guest mutations never
 * affect native state. Model-node indices are not entity or skin-palette IDs. */
export interface NativeAnimationIk {
  readonly ids: readonly NativeId[];
  readonly constraintIds: readonly NativeId[];
  readonly chains: Uint16Array;
  readonly targets: Float64Array;
  readonly poles: Float64Array;
  readonly weights: Float64Array;
}
/** Committed root rows, sorted by stable owner ID, at most 1024. Every array is
 * VM-owned; mutating or retaining it cannot alter native state. Row identities
 * bind the captured model fingerprint (32 bytes/row) and model-node index.
 * Policies: 0 extract-only, 1 bodyless transform, 2 velocity kinematic 3D,
 * 3 collision-limited character 3D. Flags: bit0 applying, bit1 outgoing present.
 * Clock owner is state ULID for graphs, else clip index (-1 rest). Absent
 * outgoing clock has null state, -1 clip, zero cycle/seconds. Graph phases use
 * their authored cycle, not a representative clip's duration. Exact signed
 * cycles and accepted tick are BigInts; never convert them to Number.
 * Each delta has stride7 [tx,ty,tz,qx,qy,qz,qw]. Requested is model-space local
 * locomotion before application; applied is actual displacement expressed in
 * the pre-step entity frame (including physics collision limitation). Bodyless
 * deltas remain in unscaled model units; the owner transform applies its
 * admitted uniform scale. Physics policies require unit scale.
 * Accumulations compose these accepted interval deltas in order. Disabling
 * application preserves extraction/clocks and strips the root from the pose.
 * Parameter/select/seek changes alone do not apply a displacement. */
export interface NativeRootMotion {
  readonly ids: readonly NativeId[];
  readonly fingerprints: Uint8Array;
  readonly roots: Uint16Array;
  readonly policies: Uint8Array;
  readonly flags: Uint8Array;
  readonly ticks: BigUint64Array;
  readonly states: readonly (NativeId | null)[];
  readonly clips: Int32Array;
  readonly cycles: BigInt64Array;
  readonly seconds: Float64Array;
  readonly outgoingStates: readonly (NativeId | null)[];
  readonly outgoingClips: Int32Array;
  readonly outgoingCycles: BigInt64Array;
  readonly outgoingSeconds: Float64Array;
  readonly requested: Float64Array;
  readonly applied: Float64Array;
  readonly accumulatedRequested: Float64Array;
  readonly accumulatedApplied: Float64Array;
}
export interface NativeAnimations {
  readonly rootMotion: NativeRootMotion;
  readonly ik: NativeAnimationIk;
  readonly graphs: NativeAnimationGraphs;
  /** Packed complete poses. Empty span means legacy clip/two-pose playback.
   * poseKinds[2*i..2*i+2] uses 0=clip, 1=ordered spherical pair, 2=weighted
   * quaternion mean, 255=absent. poseSplits[i] is zero for one group, otherwise
   * the number of samples in the outgoing group. Compound fades have at most
   * eight samples per group; a single weighted pose has at most sixteen.
   * Weights normalize within each group, never across an outer transition.
   * poseOuterWeights[i] applies one final TRS crossfade between the two groups.
   * clips/seconds remain the incoming state's representative scalar sample. */
  readonly poseKinds: Uint8Array;
  readonly poseSplits: Uint8Array;
  readonly poseOuterWeights: Float64Array;
  readonly poseOffsets: Uint32Array;
  readonly poseClips: Int32Array;
  readonly poseSeconds: Float64Array;
  readonly poseWeights: Float64Array;
  readonly ids: readonly NativeId[];
  readonly clips: Int32Array;
  readonly seconds: Float64Array;
  readonly speeds: Float64Array;
  readonly flags: Uint32Array;
  /** Active = bit 0, captured outgoing playing = bit 1, looping = bit 2.
   * All source/time fields are zero when inactive; active sourceClips uses
   * -1 for rest. Destination contribution is elapsed / duration. Master
   * playing pauses both clocks; speed/loop controls affect only destination. */
  readonly transitionFlags: Uint32Array;
  readonly sourceClips: Int32Array;
  readonly sourceSeconds: Float64Array;
  readonly sourceSpeeds: Float64Array;
  readonly transitionElapsed: Float64Array;
  readonly transitionDuration: Float64Array;
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
/** Explicit game-checkpoint data. No functions, accessors, symbols, class
 * instances, sparse arrays, cycles or non-finite numbers. Returned data is
 * deeply frozen; state.set copies bounded own data properties. */
export type NativePersistentValue = null | boolean | number | string |
  readonly NativePersistentValue[] | { readonly [key: string]: NativePersistentValue };
export interface NativePersistentState {
  /** Source state when omitted, otherwise one of this source's live instances.
   * Missing data is null. Reads include this update's staged writes. */
  get(entity?: NativeId): NativePersistentValue;
  /** Replace data for the source/instance. Commits atomically with world/timers.
   * At most 256 get/set calls and 1 MiB/65,536 transferred nodes per whole update;
   * retained state across all sources is 1 MiB/65,536 nodes, depth at most 16.
   * Active API invalid data/quota calls fail the tick even when caught. Functions
   * expire after this source update; expired/wrong-source calls throw without
   * poisoning the current source update. */
  set(value: NativePersistentValue, entity?: NativeId): void;
}
/** Owned pre-command declarations sorted by stable entity ID, <=1024 widgets
 * and <=1024 world-text rows, combined text <=1MiB. This is not editable input
 * buffer state: use input.ui events for accepted edits/submission. Queued writes
 * appear next tick. Immutable properties/array length; no borrowed native data.
 * Native Rust uses the same typed ControlUi/ControlText queue; Wasm parity for
 * these controls/snapshots remains unsupported. No runtime authoring mutation. */
export interface NativeHud {
  readonly widgets: readonly { readonly id: NativeId; readonly role: 'root' | 'container' | 'label' | 'button' | 'text_input'; readonly text: string | undefined; readonly disabled: boolean }[];
  readonly texts: readonly { readonly id: NativeId; readonly text: string }[];
}
export interface NativeContext {
  /** Available when the program declares a coroutine catalog. Functions expire
   * with this source tick; waits count accepted updates, not wall-clock time. */
  readonly coroutines?: NativeCoroutines;
  readonly hud: NativeHud;
  /** Authored attachments for this source, sorted by stable entity ID. The array
   * and initial props are deeply frozen. One callback per nonempty source runs
   * in source-ID order with the same world-wide bulk views/transaction. A queued
   * despawn changes membership only after commit; plain Spawn never reattaches,
   * even when reusing an ID. Standalone behaviors receive an empty array.
   * Dynamic attachment, props replacement and legacy state hooks are unsupported. */
  readonly instances: readonly NativeInstance[];
  /** Available only to behaviors explicitly declaring persistence version 1. */
  readonly state: NativePersistentState;
  readonly flipbooks: NativeFlipbooks;
  readonly animations: NativeAnimations;
  readonly chunks: readonly NativeChunk[];
  readonly dt: number;
  readonly commands: NativeCommands;
  /** Immutable post-routing input. input.ui contains accepted button/value/submit
   * outcomes (<=256, <=64KiB of value bytes), current focus and a bounded stale
   * receipt count. Keys/buttons captured by widgets are absent from gameplay
   * edges/actions; controls held before capture are released and suppressed
   * until their physical release. UI outcomes and script/world changes share
   * one accepted tick. This does not expose OS IME, widget mutation or save APIs. */
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
/** Owned, deeply frozen explicit continuation. Never a VM/closure snapshot. */
export interface NativeCoroutineTask {
  readonly id: string;
  readonly state: NativePersistentValue;
  readonly entity?: NativeId;
}
export type NativeCoroutineResult = Readonly<Extract<CoroutineResult, { kind: "done" }>> |
  Readonly<Omit<Extract<CoroutineResult, { kind: "wait" }>, "state"> & { state: NativePersistentValue }>;
/** Resume callbacks always run with a captured coroutine catalog. */
export interface NativeCoroutineContext extends NativeContext {
  readonly coroutines: NativeCoroutines;
}
export interface NativeCoroutine {
  /** Current accepted tick capabilities; return a finite data continuation.
   * No generator/async syntax. All state needed after load belongs in task.state,
   * context.state or the native world, never module globals/closures. */
  resume(context: NativeCoroutineContext, task: NativeCoroutineTask): NativeCoroutineResult;
}
export interface NativeCoroutines {
  /** Starts on the next accepted update. Names/IDs: 1–64 ASCII letters/digits/_.:-.
   * Existing task IDs refuse; cancel then start replaces. Optional entity must
   * be this source's live authored instance; retirement cancels owned tasks.
   * <=128 tasks, <=256 start/cancel calls per whole update. State <=16KiB/1024
   * nodes per task, <=256KiB/16384 retained nodes, depth <=16. Whole-tick state
   * transfer <=1MiB/65536 nodes. Invalid/caught calls fail the transaction. */
  start(id: string, routine: string, state: NativePersistentValue, entity?: NativeId): void;
  /** Absent IDs are allowed and charged. Cancellation/replacement by a running
   * task wins over its return. No cancelled continuation is run for cleanup. */
  cancel(id: string): void;
}
export interface NativeBehavior {
  /** Captured once. <=32 named routines per source and <=128 per program.
   * Due tasks resume after this source update in task-ID order, at most once
   * per accepted update. Their typed continuations participate in checkpoints. */
  readonly coroutines?: Readonly<Record<string, NativeCoroutine>>;
  /** Explicit ordinary game-load contract, NOT a VM/closure continuation.
   * Module initialization runs again on load. Module globals and closures are
   * ephemeral: all gameplay state needed after load must be in context.state or
   * native world components. Native source-scoped timer data is restored; timer
   * events are still handled by update. Non-opted-in active behaviors prevent
   * checkpoint creation. Static modules/physics/audio remain unsupported by the
   * initial native-session checkpoint API. */
  readonly persistence?: { readonly version: 1 };
  /** Synchronous only; no promises, deferred work or non-undefined return. */
  update(context: NativeContext): undefined;
}
