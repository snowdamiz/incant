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
export type { InputActions, InputEvent, InputFrame, InputRecording } from './generated';
import type { InputFrame } from './generated';
export type { ScriptClock } from './generated';
import type { ScriptClock, TimerRequest as GeneratedTimerRequest, TimerEvent as GeneratedTimerEvent } from './generated';
export type TimerRequest = Omit<GeneratedTimerRequest, 'payload'> & { payload?: Json };
export type TimerEvent = Omit<GeneratedTimerEvent, 'payload'> & { payload: Json };
export type { PhysicsRayQuery as RayQuery, PhysicsRayHit as RayHit, PhysicsTriggerEvent as TriggerEvent } from './generated';
import type { PhysicsRayQuery as RayQuery, PhysicsRayHit as RayHit, PhysicsTriggerEvent as TriggerEvent } from './generated';
export type { PhysicsCharacterQuery as CharacterQuery, PhysicsCharacterMovement as CharacterMovement } from './generated';
import type { PhysicsCharacterQuery as CharacterQuery, PhysicsCharacterMovement as CharacterMovement } from './generated';
export type { StringTable, LocaleSettings, LocalizeRequest, LocalizedText, MissingString, CalendarDate, DateLength } from './generated';
import type { LocaleSettings, LocalizeRequest, LocalizedText, MissingString, CalendarDate, DateLength } from './generated';
export type { NavigationMesh, NavigationQuery, NavigationPath, OffMeshLink, OffMeshTraversal } from './generated';
import type { NavigationQuery, NavigationPath } from './generated';
export type { SteeringQuery, SteeringAgent, SteeringObstacle, SteeringVelocity } from './generated';
import type { SteeringQuery, SteeringVelocity } from './generated';
export interface ScriptApi {
  /** Read-only reciprocal local avoidance in world XZ coordinates. One snapshot
   * of at most 128 agents and 32 convex obstacles (128 total edges); 64 KiB input.
   * Costs 128 of the shared 256 query units. Returns ID-sorted proposed velocities,
   * never moves entities. Apply with ordinary commands and account for physics.
   * Heights filter separate floors; routes, navmesh containment, arrival and
   * recovery from infeasible crowding remain behavior-owned. */
  steerAgents(query: SteeringQuery): readonly SteeringVelocity[];
  /** Bounded A* with funnel smoothing on the selected scene mesh. Costs 64 of
   * 256 shared native-query units per tick. Null means no nearby/reachable path;
   * invalid inputs or exhausted budgets throw. The path uses world coordinates
   * and is a snapshot; recalculate after source changes. Generation is local to
   * the play session, not durable across saves. Commands become visible next tick.
   * Heights lie on the quantized navigation surface; steps are approximated.
   * Use physics for grounding/placement. The chosen corridor can include tile bends.
   * `traversals` identifies explicit off-mesh edges between consecutive point indices.
   * Behavior must execute jumps, ladders or teleports itself; those edges are never
   * smoothed into walking segments or treated as collision-safe movement. */
  findPath(query: NavigationQuery): NavigationPath | null;
  /** Read-only localization snapshot at the start of this tick. Each localization
   * query costs 4 of the shared 256 native-query units. Change locale with the
   * set_locale command; successful changes are visible next tick and saved.
   * Missing keys return an explicit marker and diagnostic; bad arguments throw. */
  localize(request: LocalizeRequest): LocalizedText;
  locale(): LocaleSettings;
  formatNumber(value: number): string;
  /** ISO calendar input; formatted using the current locale's calendar. */
  formatDate(value: CalendarDate, length?: DateLength): string;
  /** Deterministic coverage for all declared keys in the current locale.
   * Unknown keys requested at runtime are reported by localize().missing. */
  localizationReport(): readonly MissingString[];
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
   * the shared 256 native-query units per tick. Apply translation/dt as Velocity
   * through api.command for the next tick. Gravity/jumping are behavior-owned. */
  computeCharacterMotion(query: CharacterQuery): CharacterMovement;
  /** Play-session query; costs 1 of 256 shared native-query units per tick. Direction is normalized. */
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
