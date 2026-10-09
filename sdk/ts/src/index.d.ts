/** Phase 0 API. Scripts cannot import host modules or access filesystem/network. */
export type Id = string;
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export interface Entity {
  readonly id: Id;
  readonly scene_id: Id;
  readonly name: string;
  readonly parent: Id | null;
  readonly components: Readonly<Record<string, Json>>;
}
export type { Command } from './generated';
import type { Command } from './generated';
export interface ScriptApi {
  query(component?: string): readonly Entity[];
  command(command: Command): void;
}
export interface Behavior<State extends Record<string, Json>> {
  initialState: State;
  update(api: ScriptApi, dt: number, state: State): void;
}
declare global {
  function defineBehavior<State extends Record<string, Json>>(behavior: Behavior<State>): Behavior<State>;
}
