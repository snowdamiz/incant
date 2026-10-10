/** Native QuickJS migration API. Separate from the document-backed ScriptApi. */
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
 * Errors reject numeric writes and the complete batch and fail the host.
 * Functions belong to one tick only; retained functions never reactivate.
 * Vector/transform inputs are copied as fixed numeric fields, never JSON. */
export interface NativeCommands {
  spawn(id: NativeId, transform: NativeTransform, parent: NativeId | null, velocity: NativeVector3 | null): void;
  /** No cascading deletion. Reparent/remove children in the same batch. */
  despawn(id: NativeId): void;
  /** Keeps local Transform. Parent may be spawned later in this batch. */
  setParent(id: NativeId, parent: NativeId | null): void;
  /** Add, replace or remove the actual Velocity component. */
  setVelocity(id: NativeId, velocity: NativeVector3 | null): void;
}
export interface NativeContext {
  readonly chunks: readonly NativeChunk[];
  readonly dt: number;
  readonly commands: NativeCommands;
}
export interface NativeBehavior {
  /** Synchronous only; no promises, deferred work or non-undefined return. */
  update(context: NativeContext): undefined;
}
