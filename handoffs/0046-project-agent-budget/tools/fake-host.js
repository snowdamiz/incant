/**
 * ALLOWANCE FIXTURE for handoff 0046. NOT a native host, NOT a provider, NOT an engine.
 *
 * A scripted stand-in for the editor host's IPC, installed as window.__TAURI__ before the
 * page's modules run, so the real compiled bridge (editor/bridge/native.generated.js,
 * loaded next) runs unchanged on top of it. It answers only calls that production bridge
 * makes. Its capabilities are the bridge's own: `agent.budget.set` appears only because
 * the engine read carries `agent_budget: { available: true }`, the published Project
 * schema and valid settings, exactly as the bridge requires of a native host.
 *
 * Budget behaviour follows docs/spikes/agent-project-budget.md and its review record:
 * an allowance edit is one `engine_execute` with a `set_agent_budget` operation at the
 * expected revision, journaled in History; Undo and Redo restore the cap and never the
 * usage; the session's cap follows the project (optionally after `sessionLag` ms); and a
 * cap change ends an observed active turn as `project_changed`. Everything is local and
 * deterministic. Captures made with it are bridge and UI behaviour only.
 *
 * Configuration (window.__ALLOWANCE_FIXTURE_CONFIG__):
 *   engine        an engine_read response (an invented project)
 *   provider      the saved-account state (invented labels only)
 *   session       the native agent session snapshot to start from
 *   executeDelay  ms before an allowance edit is answered
 *   historyDelay  ms before Undo or Redo is answered
 *   executeError  thrown by the next allowance edit
 *   sessionLag    ms before the session's cap follows an edited cap (absent: at once)
 * window.__ALLOWANCE_FIXTURE__ exposes the state and `calls` (every IPC name and argument).
 */
(() => {
  const config = window.__ALLOWANCE_FIXTURE_CONFIG__;
  if (!config) throw new Error('Allowance fixture: missing configuration.');
  const clone = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
  const delay = (ms) => new Promise((done) => setTimeout(done, ms));
  const listeners = new Map();
  const ACTIVE = ['running', 'awaiting_approval', 'stopping'];
  const fixture = {
    engine: clone(config.engine),
    provider: clone(config.provider),
    session: clone(config.session),
    calls: [],
    /** When set, engine_agent_read throws it (a transport failure). */
    readError: config.readError ?? null,
    /** When set, the next allowance edit throws it. */
    executeError: config.executeError ?? null,
    /** Allowance edits made here, oldest first, for Undo and Redo: { before, after }. */
    edits: [],
    /** History entries the scenario started with; Undo stops there. */
    base: config.engine.applied,
    sessionLag: config.sessionLag ?? null,
  };

  const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  const ulid = (serial) => {
    let time = Date.parse('2026-10-11T15:20:00Z') + serial * 60000;
    let prefix = '';
    for (let i = 0; i < 10; i += 1) {
      prefix = CROCKFORD[time % 32] + prefix;
      time = Math.floor(time / 32);
    }
    return `${prefix}${String(serial).padStart(16, '0')}`;
  };
  fixture.change = (mutate) => {
    const next = clone(fixture.session);
    mutate(next);
    next.version = fixture.session.version + 1;
    fixture.session = next;
  };
  fixture.emit = (event, payload) => {
    for (const handler of listeners.get(event) ?? []) handler({ event, payload });
  };
  fixture.setProvider = (provider) => {
    fixture.provider = provider;
    fixture.emit('incant:provider-changed', provider);
  };
  /** Engine-side changes (another window, a script) reach the editor as a change event. */
  fixture.setCap = (max) => {
    setCap(max);
    fixture.engine.revision += 1;
    fixture.emit('incant:engine-changed', null);
  };

  function cap() {
    return fixture.engine.project.settings.agent_budget?.max_tokens ?? 1000000;
  }

  /** The native session follows the project's cap, and ends a turn the change interrupts. */
  function sessionFollows(max) {
    const apply = () =>
      fixture.change((session) => {
        session.budget.max_tokens = max;
      });
    if (fixture.sessionLag === null) apply();
    else setTimeout(apply, fixture.sessionLag);
    const last = fixture.session.turns.at(-1);
    if (last && ACTIVE.includes(last.status)) {
      fixture.change((session) => {
        const turn = session.turns.at(-1);
        if (turn.pending) {
          turn.approvals.find((approval) => approval.id === turn.pending.id).decision = 'cancelled';
          turn.pending = null;
        }
        turn.status = 'failed';
        turn.failure = 'project_changed';
      });
      setTimeout(() => fixture.change((session) => { session.worker_running = false; }), 600);
    }
  }

  function setCap(max) {
    fixture.engine.project.settings = { ...fixture.engine.project.settings, agent_budget: { max_tokens: max } };
    sessionFollows(max);
  }

  async function execute(args) {
    if (config.executeDelay) await delay(config.executeDelay);
    if (fixture.executeError) {
      const error = fixture.executeError;
      fixture.executeError = null;
      throw clone(error);
    }
    const [command] = args.commands ?? [];
    if (args.commands?.length !== 1 || command?.op !== 'set_agent_budget') throw { code: 'fixture.unsupported', message: 'The allowance fixture implements only allowance edits.' };
    if (args.expectedRevision !== fixture.engine.revision) throw { code: 'engine.revision-mismatch', message: 'Project revision changed.' };
    const before = cap();
    const after = command.settings.max_tokens;
    const engine = fixture.engine;
    engine.history = [...engine.history.slice(0, engine.applied), { id: ulid(engine.history.length + 1), description: args.description, actor: { origin: 'user', actor: 'editor', model: null, conversation_id: null } }];
    fixture.edits = [...fixture.edits.slice(0, engine.applied - fixture.base), { before, after }];
    engine.applied = engine.history.length;
    engine.can_redo = false;
    engine.revision += 1;
    setCap(after);
    return clone(engine);
  }

  async function history(redo) {
    if (config.historyDelay) await delay(config.historyDelay);
    const engine = fixture.engine;
    const index = engine.applied - fixture.base;
    const edit = redo ? fixture.edits[index] : fixture.edits[index - 1];
    if (!edit) throw { code: 'fixture.unsupported', message: 'The allowance fixture can undo only its own allowance edits.' };
    engine.applied += redo ? 1 : -1;
    engine.can_redo = engine.applied < engine.history.length;
    engine.revision += 1;
    setCap(redo ? edit.after : edit.before);
    return clone(engine);
  }

  async function invoke(name, args) {
    fixture.calls.push({ name, args: clone(args ?? null) });
    switch (name) {
      case 'engine_read':
        return clone(fixture.engine);
      case 'engine_execute':
        return execute(args);
      case 'engine_history':
        return history(args.redo === true);
      case 'provider_read':
        return clone(fixture.provider);
      case 'provider_action':
        return null;
      // A browser page has no native window or Play worker; these never answer.
      case 'window_read':
      case 'play_read':
        return new Promise(() => {});
      case 'viewport_bounds':
        return null;
      case 'engine_agent_read': {
        if (fixture.readError) throw clone(fixture.readError);
        const unchanged = args && args.afterVersion === fixture.session.version;
        return { session: unchanged ? null : clone(fixture.session), project_revision: fixture.engine.revision };
      }
      case 'engine_agent_cancel': {
        const turn = fixture.session.turns.at(-1);
        if (!turn || turn.id !== args.turnId || !ACTIVE.includes(turn.status)) throw { code: 'agent.stale-turn', message: 'This agent turn is no longer active.' };
        fixture.change((session) => {
          const current = session.turns.at(-1);
          if (current.pending) {
            current.approvals.find((approval) => approval.id === current.pending.id).decision = 'cancelled';
            current.pending = null;
          }
          current.status = 'cancelled';
        });
        setTimeout(() => fixture.change((session) => { session.worker_running = false; }), 500);
        return null;
      }
      default:
        throw { code: 'fixture.unsupported', message: 'The allowance fixture does not implement this host call.' };
    }
  }

  const listen = async (event, handler) => {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event).add(handler);
    return () => listeners.get(event)?.delete(handler);
  };

  window.__TAURI__ = { core: { invoke }, event: { listen } };
  window.__ALLOWANCE_FIXTURE__ = fixture;
})();
