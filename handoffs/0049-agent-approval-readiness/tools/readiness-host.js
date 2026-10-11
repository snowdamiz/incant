/**
 * READINESS FIXTURE for handoff 0049. NOT a native host, NOT a provider, NOT an engine.
 *
 * Installed after handoff 0046's tools/fake-host.js (unchanged) and before the real compiled
 * bridge, which captures window.__TAURI__.core.invoke when it installs. It wraps that
 * fixture's scripted IPC with the three calls this review needs:
 *   engine_history       Undo or Redo of one of the fixture's ordinary History entries. The
 *                        allowance does not change, so the session and its pending approval
 *                        are left alone, as the native host leaves them. While `holdHistory`
 *                        is true the reply waits for release('history').
 *   engine_agent_read    while `holdReads` is true, each session read waits for
 *                        release('reads'); `heldReads` counts them.
 *   engine_agent_decide  records the decision on the pending approval; the next read shows it.
 * Every call is recorded in the 0046 fixture's `calls`. Exposed as window.__READINESS__.
 */
(() => {
  const fixture = window.__ALLOWANCE_FIXTURE__;
  const tauri = window.__TAURI__;
  if (!fixture || !tauri?.core?.invoke) throw new Error('Readiness fixture: install handoff 0046 tools/fake-host.js first.');
  const base = tauri.core.invoke;
  const clone = (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
  const gate = () => {
    let open;
    const promise = new Promise((done) => {
      open = done;
    });
    return { promise, open };
  };
  const gates = { history: gate(), reads: gate() };
  const readiness = {
    holdHistory: true,
    holdReads: false,
    heldReads: 0,
    /** Answers what waits at `name` and stops holding it. */
    release(name) {
      if (name === 'history') readiness.holdHistory = false;
      else if (name === 'reads') readiness.holdReads = false;
      else throw new Error(`Readiness fixture: no gate ${name}`);
      const opened = gates[name];
      gates[name] = gate();
      opened.open();
    },
  };

  async function history(args) {
    fixture.calls.push({ name: 'engine_history', args: clone(args ?? null) });
    if (readiness.holdHistory) await gates.history.promise;
    const engine = fixture.engine;
    const redo = args?.redo === true;
    if (redo ? engine.applied >= engine.history.length : engine.applied <= 0) throw { code: 'engine.history', message: redo ? 'Nothing to redo.' : 'Nothing to undo.' };
    engine.applied += redo ? 1 : -1;
    engine.can_redo = engine.applied < engine.history.length;
    engine.revision += 1;
    return clone(engine);
  }

  function decide(args) {
    fixture.calls.push({ name: 'engine_agent_decide', args: clone(args ?? null) });
    const turn = fixture.session.turns.at(-1);
    if (!turn || turn.id !== args?.turnId || turn.status !== 'awaiting_approval' || turn.pending?.id !== args?.approvalId) {
      throw { code: 'agent.stale-approval', message: 'This agent approval is no longer pending.' };
    }
    fixture.change((session) => {
      const current = session.turns.at(-1);
      current.approvals.find((approval) => approval.id === current.pending.id).decision = args.approved ? 'allowed' : 'denied';
      current.pending = null;
      current.status = 'running';
    });
    return null;
  }

  async function invoke(name, args) {
    if (name === 'engine_history') return history(args);
    if (name === 'engine_agent_decide') return decide(args);
    if (name === 'engine_agent_read' && readiness.holdReads) {
      readiness.heldReads += 1;
      await gates.reads.promise;
    }
    return base(name, args);
  }

  tauri.core.invoke = invoke;
  window.__READINESS__ = readiness;
})();
