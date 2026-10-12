/**
 * AGENT FIXTURE for handoff 0045. NOT a native host, NOT a provider.
 *
 * A scripted stand-in for the editor host's IPC, installed as window.__TAURI__ before the
 * page's modules run, so the real compiled bridge (editor/bridge/native.generated.js,
 * loaded next) runs unchanged on top of it. Everything is deterministic and local: no
 * network, no account, no engine, no model. Captures and logs made with it are bridge
 * and UI behaviour only, never live-provider or native-host evidence.
 *
 * Configuration (window.__AGENT_FIXTURE_CONFIG__, set by an earlier init script):
 *   engine    an engine_read response (an invented project)
 *   provider  the saved-account state (invented labels only)
 *   session   the native agent session snapshot to start from
 *   script    steps a started turn plays: {wait, text} | {wait, approval: {tool, arguments}}
 *             | {wait, complete: {charged, edits, uncertain?}} | {wait, fail: {failure, charged}}
 *   stopDelay / reapDelay  ms from Stop to "cancelled", and on to the worker's exit
 *   decideDelay  ms before an approval decision is answered
 * window.__AGENT_FIXTURE__ exposes the state and `calls` (every IPC name and argument).
 */
(() => {
  const config = window.__AGENT_FIXTURE_CONFIG__;
  if (!config) throw new Error('Agent fixture: missing configuration.');
  const clone = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
  const listeners = new Map();
  const fixture = {
    engine: config.engine,
    provider: config.provider,
    session: config.session,
    calls: [],
    /** When set, engine_agent_read throws it (a transport failure). */
    readError: null,
    /** When set, engine_agent_start throws it. */
    startError: config.startError ?? null,
    resume: null,
  };
  let nextApproval = 1 + Math.max(0, ...config.session.turns.flatMap((turn) => turn.approvals.map((approval) => approval.id)));

  const later = (ms, run) => setTimeout(run, ms);
  const turnOf = (session, id) => session.turns.find((turn) => turn.id === id);
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

  function play(id, steps, index) {
    const step = steps[index];
    if (!step) return;
    later(step.wait ?? 150, () => {
      const turn = turnOf(fixture.session, id);
      if (!turn || turn.status !== 'running') return;
      if (step.text !== undefined) {
        fixture.change((session) => {
          turnOf(session, id).text += step.text;
        });
        play(id, steps, index + 1);
      } else if (step.approval) {
        const approvalId = nextApproval++;
        const { tool, arguments: args } = step.approval;
        fixture.change((session) => {
          const current = turnOf(session, id);
          current.status = 'awaiting_approval';
          current.approvals.push({ id: approvalId, tool, bytes: new TextEncoder().encode(JSON.stringify(args)).length, decision: 'pending' });
          current.pending = { turn: id, id: approvalId, tool, arguments: args };
        });
        fixture.resume = { id, approvalId, steps, index: index + 1 };
      } else if (step.complete) {
        fixture.change((session) => {
          const current = turnOf(session, id);
          current.status = 'completed';
          current.charged_tokens = step.complete.charged;
          current.accounting_uncertain = step.complete.uncertain ?? false;
          current.report = {
            steps: 3,
            usage: { input_tokens: Math.round(step.complete.charged * 0.92), output_tokens: Math.round(step.complete.charged * 0.08) },
            transaction_count: step.complete.edits,
            tool_call_count: step.complete.edits + 1,
            transaction_ids: Array.from({ length: step.complete.edits }, (_, n) => `01JA45FX${String(n).padStart(18, '0')}`),
            tool_calls: [],
            truncated: false,
          };
          session.budget.used_tokens += step.complete.charged;
          session.worker_running = false;
        });
      } else if (step.fail) {
        fixture.change((session) => {
          const current = turnOf(session, id);
          current.status = 'failed';
          current.failure = step.fail.failure;
          current.charged_tokens = step.fail.charged;
          session.budget.used_tokens += step.fail.charged;
          session.worker_running = false;
        });
      }
    });
  }

  function start(request) {
    if (fixture.startError) throw clone(fixture.startError);
    const id = (fixture.session.turns.at(-1)?.id ?? 0) + 1;
    fixture.change((session) => {
      session.worker_running = true;
      session.turns.push({ id, request, status: 'running', text: '', approvals: [], pending: null, report: null, failure: null, charged_tokens: 0, accounting_uncertain: false });
      if (session.turns.length > 16) session.turns.shift();
    });
    play(id, config.script ?? [], 0);
    return id;
  }

  function cancel(turnId) {
    const turn = fixture.session.turns.at(-1);
    if (!turn || turn.id !== turnId) throw { code: 'agent.stale-turn', message: 'This agent turn is no longer active.' };
    fixture.resume = null;
    fixture.change((session) => {
      const current = turnOf(session, turnId);
      current.status = 'stopping';
      if (current.pending) {
        current.approvals.find((approval) => approval.id === current.pending.id).decision = 'cancelled';
        current.pending = null;
      }
    });
    const stopDelay = config.stopDelay ?? 900;
    later(stopDelay, () =>
      fixture.change((session) => {
        const current = turnOf(session, turnId);
        current.status = 'cancelled';
        current.charged_tokens = 1180;
        current.accounting_uncertain = true;
        session.budget.used_tokens += 1180;
        session.accounting_uncertain = true;
      }),
    );
    later(stopDelay + (config.reapDelay ?? 700), () =>
      fixture.change((session) => {
        session.worker_running = false;
      }),
    );
    return null;
  }

  async function decide(turnId, approvalId, approved) {
    // A slow host, so a decision in flight can be seen.
    if (config.decideDelay) await new Promise((done) => setTimeout(done, config.decideDelay));
    const resume = fixture.resume;
    if (!resume || resume.id !== turnId || resume.approvalId !== approvalId) {
      throw { code: 'agent.stale-approval', message: 'This agent approval is no longer pending.' };
    }
    fixture.resume = null;
    fixture.change((session) => {
      const current = turnOf(session, turnId);
      current.pending = null;
      current.status = 'running';
      current.approvals.find((approval) => approval.id === approvalId).decision = approved ? 'allowed' : 'denied';
    });
    if (approved) play(turnId, resume.steps, resume.index);
    else
      later(250, () =>
        fixture.change((session) => {
          const current = turnOf(session, turnId);
          current.status = 'failed';
          current.failure = 'tool';
          current.charged_tokens = 940;
          session.budget.used_tokens += 940;
          session.worker_running = false;
        }),
      );
    return null;
  }

  async function invoke(name, args) {
    fixture.calls.push({ name, args: clone(args ?? null) });
    switch (name) {
      case 'engine_read':
        return clone(fixture.engine);
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
      case 'engine_agent_start':
        return start(args.request);
      case 'engine_agent_cancel':
        return cancel(args.turnId);
      case 'engine_agent_decide':
        return decide(args.turnId, args.approvalId, args.approved);
      default:
        throw { code: 'fixture.unsupported', message: 'The agent fixture does not implement this host call.' };
    }
  }

  const listen = async (event, handler) => {
    if (!listeners.has(event)) listeners.set(event, new Set());
    listeners.get(event).add(handler);
    return () => listeners.get(event)?.delete(handler);
  };

  window.__TAURI__ = { core: { invoke }, event: { listen } };
  window.__AGENT_FIXTURE__ = fixture;
})();
