import type { BridgeError, HierarchyNode, HierarchyTree, PlayStatus, Ulid } from '../bridge/contract';

/** The part of a Play status the controls and Inspector present. Running ticks are left out. */
export interface PlaySession {
  readonly generation: number;
  readonly phase: PlayStatus['phase'];
  readonly preparing: boolean;
  readonly suspended: boolean;
  readonly scene: string | null;
  readonly error: string | null;
  /** Shown only while paused, and with an error: a running tick changes four times a second. */
  readonly tick: number | null;
}

export interface PlayFeed {
  readonly session: PlaySession | null;
  /** The Play status could not be read; distinct from a session that failed. */
  readonly transport: BridgeError | null;
}

export function playFeed(status: PlayStatus | undefined, error: BridgeError | undefined): PlayFeed {
  return {
    session: status
      ? {
          generation: status.generation,
          phase: status.phase,
          preparing: status.preparing,
          suspended: status.suspended,
          scene: status.scene,
          error: status.error,
          tick: status.phase === 'paused' || status.phase === 'error' ? status.tick : null,
        }
      : null,
    transport: error ?? null,
  };
}

export function sameFeed(a: PlayFeed, b: PlayFeed): boolean {
  const x = a.session;
  const y = b.session;
  const sessions =
    x === y ||
    (x !== null &&
      y !== null &&
      x.generation === y.generation &&
      x.phase === y.phase &&
      x.preparing === y.preparing &&
      x.suspended === y.suspended &&
      x.scene === y.scene &&
      x.error === y.error &&
      x.tick === y.tick);
  return sessions && a.transport?.code === b.transport?.code && a.transport?.message === b.transport?.message;
}

/**
 * What the Play controls present. `preparing` and `stopping` include the moment between a
 * request and the first status that shows it (the bridge reads Play status at 4 Hz).
 */
export type PlayState =
  | 'unsupported'
  | 'loading'
  | 'unavailable'
  | 'stopped'
  | 'preparing'
  | 'stopping'
  | 'running'
  | 'suspended'
  | 'paused'
  | 'error';

export function playState({
  supported,
  feed,
  startedFrom,
  stopping,
}: {
  supported: boolean;
  feed: PlayFeed;
  /** The generation a pending start was sent from, while its session is not yet visible. */
  startedFrom: number | null;
  /** A stop is in flight, or waits for the starting session's generation. */
  stopping: boolean;
}): PlayState {
  if (!supported) return 'unsupported';
  if (feed.transport) return 'unavailable';
  const session = feed.session;
  if (!session) return 'loading';
  if (startedFrom !== null && session.generation === startedFrom) return stopping ? 'stopping' : 'preparing';
  switch (session.phase) {
    case 'preparing':
      return stopping ? 'stopping' : 'preparing';
    case 'stopped':
      // Stopped while cooking: the cancelled startup is still finishing and a new one is refused.
      return session.preparing ? 'stopping' : 'stopped';
    case 'running':
      return stopping ? 'stopping' : session.suspended ? 'suspended' : 'running';
    case 'paused':
      return stopping ? 'stopping' : 'paused';
    case 'error':
      // Dismissing the error is a Stop of that session.
      return stopping ? 'stopping' : 'error';
  }
}

/** A session exists that Stop can end. */
export function isActive(state: PlayState): boolean {
  return state === 'preparing' || state === 'running' || state === 'suspended' || state === 'paused';
}

export interface PlayCamera {
  readonly id: Ulid;
  readonly name: string;
}

export interface PlayScene {
  readonly id: Ulid;
  readonly name: string;
  /** Camera entities in tree order. */
  readonly cameras: readonly PlayCamera[];
}

function nodeOf(tree: HierarchyTree, id: string): HierarchyNode | undefined {
  return Object.hasOwn(tree.nodes, id) ? tree.nodes[id] : undefined;
}

/** The project's scenes in tree order, each with the camera entities it contains. */
export function playScenes(tree: HierarchyTree | null): readonly PlayScene[] {
  if (!tree) return [];
  const scenes: PlayScene[] = [];
  for (const root of tree.roots) {
    const scene = nodeOf(tree, root);
    if (scene?.kind !== 'scene') continue;
    const cameras: PlayCamera[] = [];
    const seen = new Set<string>([scene.id]);
    const stack = [...scene.children].reverse();
    while (stack.length > 0) {
      const id = stack.pop()!;
      const node = nodeOf(tree, id);
      if (!node || seen.has(id)) continue;
      seen.add(id);
      if (node.kind === 'camera') cameras.push({ id: node.id, name: node.name });
      for (let index = node.children.length - 1; index >= 0; index -= 1) stack.push(node.children[index]!);
    }
    scenes.push({ id: scene.id, name: scene.name, cameras });
  }
  return scenes;
}

/** The user's choices this editor session: a scene, and a camera per scene. */
export interface PlayChoice {
  readonly scene: Ulid | null;
  readonly cameras: Readonly<Record<string, Ulid>>;
}

export const NO_CHOICE: PlayChoice = { scene: null, cameras: {} };

export interface PlayTarget {
  readonly scene: PlayScene | null;
  readonly camera: PlayCamera | null;
  /** What must still be chosen before Play can start. */
  readonly missing: 'scene' | 'camera' | null;
  /** There is something to choose: several scenes, or several cameras in the scene. */
  readonly choosable: boolean;
}

/**
 * The scene and camera Play starts with. A single scene, and a scene's only camera, are
 * used without asking; anything ambiguous waits for the user's choice. A scene without a
 * camera starts without one: the engine refuses it if the scene has visual content.
 */
export function resolveTarget(scenes: readonly PlayScene[], choice: PlayChoice): PlayTarget {
  const scene = scenes.length === 1 ? scenes[0]! : (scenes.find((item) => item.id === choice.scene) ?? null);
  const cameras = scene?.cameras ?? [];
  const chosen = scene && Object.hasOwn(choice.cameras, scene.id) ? choice.cameras[scene.id] : undefined;
  const camera = cameras.length === 1 ? cameras[0]! : (cameras.find((item) => item.id === chosen) ?? null);
  return {
    scene,
    camera,
    missing: scenes.length > 0 && !scene ? 'scene' : cameras.length > 1 && !camera ? 'camera' : null,
    choosable: scenes.length > 1 || cameras.length > 1,
  };
}

export interface PlayTargetOption {
  readonly key: string;
  readonly scene: PlayScene;
  readonly camera: PlayCamera | null;
  readonly label: string;
}

/** Every scene and camera Play can start with, as one list. */
export function targetOptions(scenes: readonly PlayScene[]): readonly PlayTargetOption[] {
  const several = scenes.length > 1;
  return scenes.flatMap((scene): PlayTargetOption[] =>
    scene.cameras.length === 0
      ? [{ key: scene.id, scene, camera: null, label: scene.name }]
      : scene.cameras.map((camera) => ({
          key: `${scene.id}/${camera.id}`,
          scene,
          camera,
          label: several ? `${scene.name} · ${camera.name}` : camera.name,
        })),
  );
}

/** The picker's label: the chosen scene and camera, or what is still to choose. */
export function targetLabel(target: PlayTarget, sceneCount: number): string {
  const { scene, camera } = target;
  if (!scene) return 'Choose scene';
  const prefix = sceneCount > 1 ? `${scene.name} · ` : '';
  if (target.missing === 'camera') return `${prefix}Choose camera`;
  if (!camera) return scene.name;
  return `${prefix}${camera.name}`;
}

/** What the selection is in the running world. */
export type RuntimeTarget =
  | { readonly kind: 'none' }
  | { readonly kind: 'scene' }
  | { readonly kind: 'elsewhere'; readonly scene: string }
  | { readonly kind: 'entity'; readonly id: Ulid };

/**
 * A selected entity has a runtime counterpart only in the scene that is playing. A scene
 * node has none of its own. `playing` is null while no scene is known.
 */
export function runtimeTarget(tree: HierarchyTree | null, selection: Ulid | null, playing: string | null): RuntimeTarget {
  if (!tree || selection === null) return { kind: 'none' };
  const node = nodeOf(tree, selection);
  if (!node) return { kind: 'none' };
  if (node.kind === 'scene') return { kind: 'scene' };
  let root: HierarchyNode = node;
  for (let depth = 0; root.parent !== null && depth < 4096; depth += 1) {
    const parent = nodeOf(tree, root.parent);
    if (!parent) break;
    root = parent;
  }
  if (playing !== null && root.kind === 'scene' && root.id !== playing) return { kind: 'elsewhere', scene: root.name };
  return { kind: 'entity', id: node.id };
}
