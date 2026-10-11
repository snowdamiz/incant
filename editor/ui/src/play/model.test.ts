import { describe, expect, it } from 'vitest';
import type { HierarchyNode, HierarchyTree, PlayStatus, Ulid } from '../bridge/contract';
import type { PlayFeed } from './model';
import { NO_CHOICE, isActive, playFeed, playScenes, playState, resolveTarget, runtimeTarget, sameFeed, targetLabel, targetOptions } from './model';

const id = (name: string) => name as Ulid;
function tree(spec: Record<string, { kind: string; parent?: string; name?: string }>): HierarchyTree {
  const nodes: Record<string, HierarchyNode> = {};
  for (const [key, value] of Object.entries(spec)) {
    nodes[key] = { id: id(key), name: value.name ?? key, kind: value.kind, parent: value.parent ? id(value.parent) : null, children: [] };
  }
  for (const node of Object.values(nodes)) {
    if (node.parent) (nodes[node.parent]!.children as Ulid[]).push(node.id);
  }
  return { roots: Object.values(nodes).filter((node) => node.parent === null).map((node) => node.id), nodes };
}

const PROJECT = tree({
  harbor: { kind: 'scene', name: 'Harbor' },
  rig: { kind: 'group', parent: 'harbor' },
  player: { kind: 'camera', parent: 'rig', name: 'Player Camera' },
  dock: { kind: 'camera', parent: 'rig', name: 'Dock Camera' },
  crate: { kind: 'mesh', parent: 'harbor', name: 'Crate' },
  title: { kind: 'scene', name: 'Title' },
  titleCamera: { kind: 'camera', parent: 'title', name: 'Title Camera' },
  credits: { kind: 'scene', name: 'Credits' },
});

const status = (overrides: Partial<PlayStatus> = {}): PlayStatus => ({
  generation: 2, sequence: 9, phase: 'stopped', tick: 0, discardedTicks: 0, presentedFrames: 0, scene: null,
  sourceRevision: null, spatial: false, suspended: false, preparing: false, error: null, ...overrides,
});
const feed = (overrides: Partial<PlayStatus> = {}): PlayFeed => playFeed(status(overrides), undefined);

describe('Play targets', () => {
  it('lists scenes in tree order with their cameras, depth first', () => {
    expect(playScenes(PROJECT).map((scene) => [scene.name, scene.cameras.map((camera) => camera.name)])).toEqual([
      ['Harbor', ['Player Camera', 'Dock Camera']],
      ['Title', ['Title Camera']],
      ['Credits', []],
    ]);
    expect(playScenes(null)).toEqual([]);
  });

  it('survives a malformed tree with a cycle', () => {
    const looped = tree({ s: { kind: 'scene' }, a: { kind: 'camera', parent: 's' } });
    (looped.nodes.a!.children as Ulid[]).push(id('s'), id('a'));
    expect(playScenes(looped)[0]!.cameras).toHaveLength(1);
  });

  it('uses the only scene and its only camera without asking', () => {
    const single = playScenes(tree({ s: { kind: 'scene' }, c: { kind: 'camera', parent: 's', name: 'Main' } }));
    const target = resolveTarget(single, NO_CHOICE);
    expect(target).toMatchObject({ scene: { id: 's' }, camera: { id: 'c' }, missing: null, choosable: false });
  });

  it('asks for a scene, then a camera when the scene has several, and keeps choices per scene', () => {
    const scenes = playScenes(PROJECT);
    expect(resolveTarget(scenes, NO_CHOICE)).toMatchObject({ scene: null, missing: 'scene', choosable: true });
    const harbor = resolveTarget(scenes, { scene: id('harbor'), cameras: {} });
    expect(harbor).toMatchObject({ missing: 'camera', camera: null });
    expect(targetLabel(harbor, scenes.length)).toBe('Harbor · Choose camera');
    const chosen = resolveTarget(scenes, { scene: id('harbor'), cameras: { harbor: id('dock') } });
    expect(chosen).toMatchObject({ missing: null, camera: { name: 'Dock Camera' } });
    expect(targetLabel(chosen, scenes.length)).toBe('Harbor · Dock Camera');
    // A camera remembered for a scene that no longer has it is not used.
    expect(resolveTarget(scenes, { scene: id('harbor'), cameras: { harbor: id('gone') } }).missing).toBe('camera');
    // A scene without a camera starts without one; the engine decides whether that is allowed.
    expect(resolveTarget(scenes, { scene: id('credits'), cameras: {} })).toMatchObject({ missing: null, camera: null });
  });

  it('offers every scene and camera as one list', () => {
    expect(targetOptions(playScenes(PROJECT)).map((option) => option.label)).toEqual([
      'Harbor · Player Camera',
      'Harbor · Dock Camera',
      'Title · Title Camera',
      'Credits',
    ]);
  });
});

describe('Play state', () => {
  const at = (overrides: Partial<PlayStatus>, extra: { startedFrom?: number | null; stopping?: boolean } = {}) =>
    playState({ supported: true, feed: feed(overrides), startedFrom: extra.startedFrom ?? null, stopping: extra.stopping ?? false });

  it('follows the engine phase', () => {
    expect(at({ phase: 'stopped' })).toBe('stopped');
    expect(at({ phase: 'preparing', preparing: true })).toBe('preparing');
    expect(at({ phase: 'running' })).toBe('running');
    expect(at({ phase: 'running', suspended: true })).toBe('suspended');
    expect(at({ phase: 'paused', suspended: true })).toBe('paused');
    expect(at({ phase: 'error', error: 'boom' })).toBe('error');
  });

  it('shows a stopped session whose cancelled startup still cooks as stopping', () => {
    expect(at({ phase: 'stopped', preparing: true })).toBe('stopping');
  });

  it('is preparing from the moment Play is pressed, and stopping when Stop waits for that session', () => {
    expect(at({ generation: 2 }, { startedFrom: 2 })).toBe('preparing');
    expect(at({ generation: 2 }, { startedFrom: 2, stopping: true })).toBe('stopping');
    // Once a status shows the new session, the engine's phase speaks.
    expect(at({ generation: 3, phase: 'running' }, { startedFrom: 2 })).toBe('running');
  });

  it('never pretends a session runs without a status, and keeps a transport failure distinct', () => {
    expect(playState({ supported: true, feed: playFeed(undefined, undefined), startedFrom: null, stopping: false })).toBe('loading');
    expect(playState({ supported: true, feed: playFeed(status({ phase: 'running' }), { code: 'engine.request', message: 'gone' }), startedFrom: null, stopping: false })).toBe('unavailable');
    expect(playState({ supported: false, feed: feed({ phase: 'running' }), startedFrom: null, stopping: false })).toBe('unsupported');
    expect(isActive('suspended')).toBe(true);
    expect(isActive('error')).toBe(false);
  });

  it('ignores running ticks and counters, so a status read alone does not re-render', () => {
    expect(sameFeed(feed({ phase: 'running', tick: 10, sequence: 1, presentedFrames: 3 }), feed({ phase: 'running', tick: 900, sequence: 99, presentedFrames: 80, discardedTicks: 4 }))).toBe(true);
    expect(sameFeed(feed({ phase: 'paused', tick: 10 }), feed({ phase: 'paused', tick: 11 }))).toBe(false);
    expect(sameFeed(feed({ phase: 'running' }), feed({ phase: 'running', suspended: true }))).toBe(false);
  });
});

describe('runtime counterpart of the selection', () => {
  it('is the entity in the playing scene; a scene node has none; another scene is named', () => {
    expect(runtimeTarget(PROJECT, id('crate'), 'harbor')).toEqual({ kind: 'entity', id: 'crate' });
    expect(runtimeTarget(PROJECT, id('player'), 'harbor')).toEqual({ kind: 'entity', id: 'player' });
    expect(runtimeTarget(PROJECT, id('harbor'), 'harbor')).toEqual({ kind: 'scene' });
    expect(runtimeTarget(PROJECT, id('titleCamera'), 'harbor')).toEqual({ kind: 'elsewhere', scene: 'Title' });
    expect(runtimeTarget(PROJECT, id('missing'), 'harbor')).toEqual({ kind: 'none' });
    expect(runtimeTarget(PROJECT, null, 'harbor')).toEqual({ kind: 'none' });
  });
});
