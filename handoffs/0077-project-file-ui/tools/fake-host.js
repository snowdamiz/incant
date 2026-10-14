/**
 * PROJECT FILE FIXTURE for handoff 0077. NOT a native host, NOT an engine, and it writes
 * NO file: "the file" is a string in this page.
 *
 * A scripted stand-in for the editor host's IPC, installed as window.__TAURI__ before the
 * page's modules run, so the real compiled bridge (editor/bridge/native.generated.js,
 * loaded next) runs unchanged on top of it. It answers only calls that bridge makes. Its
 * file requests mimic the host contract read as background (crates/incant_cmd/src/
 * project_checkpoint.rs and project_file.rs): one retained review, retired by a newer
 * review or a save; apply only by that review's ID at its revision, adding one History
 * entry; save adds no revision. Refusal texts are copied from the backend's error Display
 * strings. Checksums are real SHA-256 of the strings here. Captures made with it are
 * bridge and UI behaviour only.
 *
 * Configuration (window.__FILE_FIXTURE_CONFIG__):
 *   engine       an engine_read response (an invented project) with project_file
 *   other        a second invented project, opened by `openOther()`
 *   requirement  what a review proposes, e.g. "=0.1.0"
 *   unsaved      true: the document differs from the file (Save writes; review refuses)
 *   pinned       true: the project already declares an engine (review refuses)
 *   delays       { save, review_pin, apply_pin }: ms before answering
 *   errors       { save, review_pin, apply_pin }: thrown once instead of answering
 * window.__FILE_FIXTURE__ exposes the state, `calls` and helpers for the capture script.
 */
(() => {
  const config = window.__FILE_FIXTURE_CONFIG__;
  if (!config) throw new Error('Project file fixture: missing configuration.');
  const clone = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
  const delay = (ms) => new Promise((done) => setTimeout(done, ms));
  const listeners = new Map();
  const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  let serial = 900;
  const ulid = () => {
    let time = Date.parse('2026-10-14T09:00:00Z') + (serial += 1) * 1000;
    let prefix = '';
    for (let i = 0; i < 10; i += 1) {
      prefix = CROCKFORD[time % 32] + prefix;
      time = Math.floor(time / 32);
    }
    return `${prefix}${String(serial).padStart(16, '0')}`;
  };
  /** Sorted keys and two-space indentation, as the engine's canonical project text. */
  const sorted = (value) =>
    Array.isArray(value) ? value.map(sorted)
      : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map((key) => [key, sorted(value[key])]))
        : value;
  const canonical = (project) => `${JSON.stringify(sorted(project), null, 2)}\n`;
  const sha256 = async (text) =>
    [...new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)))].map((byte) => byte.toString(16).padStart(2, '0')).join('');
  const user = { origin: 'user', actor: 'editor', model: null, conversation_id: null };

  const fixture = {
    engine: clone(config.engine),
    calls: [],
    review: null,
    pinned: config.pinned === true,
    errors: { ...(config.errors ?? {}) },
    delays: { ...(config.delays ?? {}) },
  };
  // The file on "disk": the document as last saved. An unsaved document differs from it.
  const unsavedName = (project) => ({ ...project, name: `${project.name} (before rename)` });
  fixture.file = canonical(config.unsaved ? unsavedName(fixture.engine.project) : fixture.engine.project);

  fixture.emit = (event, payload) => {
    for (const handler of listeners.get(event) ?? []) handler({ event, payload });
  };
  /** An edit made elsewhere (another window, the agent): one more History entry. */
  fixture.change = (description = 'Rename Lantern') => {
    const engine = fixture.engine;
    engine.history = [...engine.history.slice(0, engine.applied), { id: ulid(), description, actor: user }];
    engine.applied = engine.history.length;
    engine.can_redo = false;
    engine.revision += 1;
    fixture.emit('incant:engine-changed', null);
  };
  /** Another project opens in this window. */
  fixture.openOther = () => {
    fixture.engine = clone(config.other);
    fixture.file = canonical(fixture.engine.project);
    fixture.review = null;
    fixture.emit('incant:engine-changed', null);
  };

  async function projectFile({ operation, expected_revision: expected, review_id: reviewId }) {
    if (fixture.delays[operation]) await delay(fixture.delays[operation]);
    const error = fixture.errors[operation];
    if (error !== undefined && error !== null) {
      fixture.errors[operation] = null;
      throw clone(error);
    }
    const engine = fixture.engine;
    if (expected !== engine.revision) throw `document revision conflict: expected ${expected}, current ${engine.revision}`;
    const current = canonical(engine.project);
    if (operation === 'save') {
      const before = fixture.file;
      fixture.file = current;
      // A save retires the retained review.
      fixture.review = null;
      return { operation: 'save', receipt: { revision: engine.revision, changed: before !== current, source_sha256: await sha256(before), candidate_sha256: await sha256(current) } };
    }
    if (operation === 'review_pin') {
      if (fixture.file !== current) throw 'source project differs from the currently opened document; reload or save explicitly';
      if (fixture.pinned) throw 'engine pin review applies only to a legacy project without an engine declaration';
      const candidate = canonical({ ...engine.project, engine_version: config.requirement });
      fixture.review = { id: ulid(), revision: engine.revision, candidate };
      return {
        operation: 'review_pin',
        review: {
          review_id: fixture.review.id,
          expected_revision: engine.revision,
          source_sha256: await sha256(fixture.file),
          candidate_sha256: await sha256(candidate),
          candidate_text: candidate,
          command: { op: 'set_engine_version', requirement: config.requirement },
        },
      };
    }
    if (operation === 'apply_pin') {
      const review = fixture.review;
      if (!review || review.id !== reviewId) throw 'invalid command: unknown or retired project pin review';
      if (review.revision !== engine.revision) throw `document revision conflict: expected ${review.revision}, current ${engine.revision}`;
      fixture.review = null;
      fixture.pinned = true;
      engine.project = { ...engine.project, engine_version: config.requirement };
      fixture.file = review.candidate;
      const transaction = ulid();
      engine.history = [...engine.history.slice(0, engine.applied), { id: transaction, description: 'Pin opened project to the current engine', actor: user }];
      engine.applied = engine.history.length;
      engine.can_redo = false;
      engine.revision += 1;
      return { operation: 'apply_pin', transaction_id: transaction, revision: engine.revision };
    }
    throw `unknown variant \`${String(operation)}\``;
  }

  async function invoke(name, args) {
    fixture.calls.push({ name, args: clone(args ?? null) });
    switch (name) {
      case 'engine_read':
        return clone(fixture.engine);
      case 'engine_project_file': {
        const response = await projectFile(args.request);
        return { response, snapshot: clone(fixture.engine) };
      }
      case 'provider_read':
        // No account: nothing in a capture identifies anyone.
        return { status: 'not-connected', provider: 'openai' };
      case 'engine_agent_read':
        return { session: null, project_revision: fixture.engine.revision };
      // A browser page has no native window or Play worker; these never answer.
      case 'window_read':
      case 'play_read':
        return new Promise(() => {});
      case 'viewport_bounds':
        return null;
      default:
        throw { code: 'fixture.unsupported', message: `The project file fixture does not implement ${name}.` };
    }
  }

  const listen = async (event, handler) => {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event).add(handler);
    return () => listeners.get(event)?.delete(handler);
  };

  window.__TAURI__ = { core: { invoke }, event: { listen } };
  window.__FILE_FIXTURE__ = fixture;
})();
