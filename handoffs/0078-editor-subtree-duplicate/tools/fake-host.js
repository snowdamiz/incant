/**
 * SUBTREE DUPLICATE FIXTURE for handoff 0078. NOT a native host, NOT an engine and NOT
 * the command bus: "the project" is an object in this page.
 *
 * A scripted stand-in for the editor host's IPC, installed as window.__TAURI__ before the
 * page's modules run, so the real compiled bridge (editor/bridge/native.generated.js,
 * loaded next) runs unchanged on top of it. It answers only calls that bridge makes.
 * engine_duplicate_subtree follows the shape of editor/app/src/entity.rs (receipt plus
 * snapshot) and imitates crates/incant_cmd/src/subtree.rs only far enough to drive the
 * UI: fresh time-ordered ULIDs, the same names, the root under the source's parent, the
 * typed entity references it lists remapped, one History entry. Its refusals use the
 * bus's Display strings. engine_history undoes and redoes whole project states, and
 * engine_execute accepts only delete_entity (to compare focus after Delete with focus
 * after Undo). Captures made with it show bridge and UI behaviour only.
 *
 * Configuration (window.__DUPLICATE_FIXTURE_CONFIG__):
 *   engine      an engine_read response (an invented project)
 *   delay       ms before engine_duplicate_subtree answers
 *   unverified  true: answer with a receipt for the previous revision (the bridge must
 *               report the result as unknown)
 *   chrome      a window_read answer; absent leaves the window unread, as in a browser
 * window.__DUPLICATE_FIXTURE__ exposes the state and `calls` to the capture script.
 */
(() => {
  const config = window.__DUPLICATE_FIXTURE_CONFIG__;
  if (!config) throw new Error('Duplicate fixture: missing configuration.');
  const clone = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
  const delay = (ms) => new Promise((done) => setTimeout(done, ms));
  const listeners = new Map();
  const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  let serial = 0;
  /** Canonical ULIDs newer than every invented one, so copies sort last as in the engine's maps. */
  const ulid = () => {
    serial += 1;
    let time = Date.parse('2026-10-15T09:00:00Z') + serial * 1000;
    let prefix = '';
    for (let i = 0; i < 10; i += 1) {
      prefix = CROCKFORD[time % 32] + prefix;
      time = Math.floor(time / 32);
    }
    return `${prefix}${String(serial).padStart(16, '0')}`;
  };
  const user = { origin: 'user', actor: 'editor', model: null, conversation_id: null };
  // Components incant_cmd copies as literals, plus the three whose entity edges it remaps.
  const LITERAL = new Set(['AngularVelocity', 'AngularVelocity2D', 'Animation', 'AnimationGraph', 'AnimationIk', 'AnimationRootMotion', 'AudioListener', 'Camera', 'Collider', 'Collider2D', 'DirectionalLight', 'EnvironmentLight', 'MeshRenderer', 'NavigationGrid', 'PointLight', 'RigidBody', 'RigidBody2D', 'Script', 'SpotLight', 'SpriteFlipbook', 'SpriteRenderer', 'Text', 'Tilemap', 'Transform', 'UiNode', 'Velocity']);
  const REMAPPED = new Set(['AudioBus', 'AudioSource', 'NavigationMesh']);

  const fixture = { engine: clone(config.engine), calls: [], receipts: [], undo: [], redo: [] };
  const invalid = (message) => `invalid command: duplicate subtree: ${message}`;

  function duplicate({ sceneId, entityId, expectedRevision }) {
    const engine = fixture.engine;
    if (expectedRevision !== engine.revision) throw `document revision conflict: expected ${expectedRevision}, current ${engine.revision}`;
    const scene = engine.project.scenes[sceneId];
    if (!scene) throw invalid('scene absent');
    if (!scene.entities[entityId]) throw invalid('source root absent');
    const children = new Map();
    for (const entity of Object.values(scene.entities)) {
      if (entity.parent) children.set(entity.parent, [...(children.get(entity.parent) ?? []), entity.id]);
    }
    const mapping = new Map();
    const pending = [entityId];
    while (pending.length) {
      const id = pending.pop();
      if (mapping.size === 1024) throw invalid('identity mapping requires 1..1024 nodes');
      mapping.set(id, ulid());
      pending.push(...(children.get(id) ?? []));
    }
    for (const source of [...mapping.keys()].sort()) {
      const entity = scene.entities[source];
      if (entity.prefab) throw invalid('detach linked instances explicitly before duplication');
      const c = entity.components;
      if (c.AudioListener || c.EnvironmentLight || c.UiNode?.kind?.role === 'root') throw invalid('a subtree containing a singleton cannot be duplicated');
    }
    const copies = {};
    for (const [source, target] of mapping) {
      const original = scene.entities[source];
      const components = clone(original.components);
      for (const kind of Object.keys(components)) {
        if (!LITERAL.has(kind) && !REMAPPED.has(kind)) throw invalid('component reference semantics are not registered');
      }
      const edge = (id) => mapping.get(id) ?? id;
      if (components.AudioBus?.parent) components.AudioBus.parent = edge(components.AudioBus.parent);
      if (components.AudioSource?.bus) components.AudioSource.bus = edge(components.AudioSource.bus);
      for (const item of components.NavigationMesh?.sources ?? []) item.entity = edge(item.entity);
      copies[target] = { id: target, name: original.name, parent: source === entityId ? original.parent : mapping.get(original.parent), components };
    }
    fixture.undo.push(clone(engine.project));
    fixture.redo = [];
    scene.entities = { ...scene.entities, ...copies };
    const transaction = ulid();
    engine.history = [...engine.history.slice(0, engine.applied), { id: transaction, description: 'Duplicate subtree', actor: user }];
    engine.applied = engine.history.length;
    engine.can_redo = false;
    engine.revision += 1;
    return { project: engine.project.id, entity: mapping.get(entityId), revision: engine.revision, transaction };
  }

  /** Only `delete_entity`, for comparing focus after Delete with focus after Undo. */
  function execute({ commands, description, expectedRevision }) {
    const engine = fixture.engine;
    if (expectedRevision !== engine.revision) throw `document revision conflict: expected ${expectedRevision}, current ${engine.revision}`;
    if (commands.some((command) => command.op !== 'delete_entity')) throw { code: 'fixture.unsupported', message: 'The duplicate fixture only deletes.' };
    fixture.undo.push(clone(engine.project));
    fixture.redo = [];
    for (const { scene_id: sceneId, entity_id: entityId } of commands) {
      const scene = engine.project.scenes[sceneId];
      if (!scene?.entities[entityId]) throw `invalid command: entity ${entityId} does not exist`;
      const gone = new Set([entityId]);
      for (let grew = true; grew; ) {
        grew = false;
        for (const entity of Object.values(scene.entities)) {
          if (entity.parent && gone.has(entity.parent) && !gone.has(entity.id)) {
            gone.add(entity.id);
            grew = true;
          }
        }
      }
      scene.entities = Object.fromEntries(Object.entries(scene.entities).filter(([id]) => !gone.has(id)));
    }
    engine.history = [...engine.history.slice(0, engine.applied), { id: ulid(), description, actor: user }];
    engine.applied = engine.history.length;
    engine.can_redo = false;
    engine.revision += 1;
    return clone(engine);
  }

  function history(redo) {
    const engine = fixture.engine;
    const [from, to] = redo ? [fixture.redo, fixture.undo] : [fixture.undo, fixture.redo];
    if (from.length === 0) throw `nothing to ${redo ? 'redo' : 'undo'}`;
    to.push(clone(engine.project));
    engine.project = from.pop();
    engine.applied += redo ? 1 : -1;
    engine.can_redo = fixture.redo.length > 0;
    engine.revision += 1;
    return clone(engine);
  }

  async function invoke(name, args) {
    fixture.calls.push({ name, args: clone(args ?? null) });
    switch (name) {
      case 'engine_read':
        return clone(fixture.engine);
      case 'engine_duplicate_subtree': {
        if (config.delay) await delay(config.delay);
        const receipt = duplicate(args);
        fixture.receipts.push(clone(receipt));
        // A lost or mismatched receipt: the bridge cannot verify what happened.
        if (config.unverified) receipt.revision -= 1;
        return { receipt, snapshot: clone(fixture.engine) };
      }
      case 'engine_history':
        return history(args?.redo === true);
      case 'engine_execute':
        return execute(args);
      case 'provider_read':
        // No account: nothing in a capture identifies anyone.
        return { status: 'not-connected', provider: 'openai' };
      case 'engine_agent_read':
        return { session: null, project_revision: fixture.engine.revision };
      case 'window_read':
        return config.chrome ? clone(config.chrome) : new Promise(() => {});
      // A browser page has no Play worker; the status is never read.
      case 'play_read':
        return new Promise(() => {});
      case 'viewport_bounds':
        return null;
      default:
        throw { code: 'fixture.unsupported', message: `The duplicate fixture does not implement ${name}.` };
    }
  }

  const listen = async (event, handler) => {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event).add(handler);
    return () => listeners.get(event)?.delete(handler);
  };

  window.__TAURI__ = { core: { invoke }, event: { listen } };
  window.__DUPLICATE_FIXTURE__ = fixture;
})();
