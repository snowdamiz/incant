import { describe, expect, it } from 'vitest';
import { NativeBridge } from './native';
import type { EngineRead, Invoke } from './native';
import type { PlayLogBatch, PlayStatus } from './contract';

const id = (n: number) => `01K7QZCY00${n.toString().padStart(16, '0')}`;
const projectId = id(1);
const warningId = id(9_999_999);
function project(name = projectId): EngineRead {
  return { project: { id: name, name: 'Logs', scenes: {} }, revision: 7, schemas: {},
    history: [], applied: 0, can_redo: false, console: [{ id: id(0), level: 'error', message: 'Existing native diagnostic' }],
    viewport_error: null };
}
function status(count = 0, dropped = 0, patch: Partial<PlayStatus> = {}): PlayStatus {
  return { generation: 1, sequence: 1, phase: 'running', tick: 1, discardedTicks: 0, presentedFrames: 0,
    scene: id(2), sourceRevision: 7, spatial: false, suspended: false, preparing: false, error: null,
    logs: { count, dropped }, ...patch };
}
function row(n: number, message = 'same', level: PlayLogBatch['entries'][number]['level'] = 'info') {
  return { id: id(n + 100), tick: 1, message, level };
}
function batch(entries: PlayLogBatch['entries'], from = 0, generation = 1, dropped = 0): PlayLogBatch {
  return { generation, from, next: from + entries.length, dropped, warningId, entries };
}
function deferred<T>() {
  let finish!: (value: T) => void;
  let reject!: (value: unknown) => void;
  const promise = new Promise<T>((yes, no) => { finish = yes; reject = no; });
  return { promise, finish, reject };
}
const scriptRows = (bridge: NativeBridge) => bridge.getSnapshot().console.filter(entry => entry.source === 'Play');

describe('native Play committed Console tails', () => {
  it('keeps old hosts compatible and preserves nested-metadata snapshot identity at idle polling', async () => {
    const calls: string[] = [];
    let current: PlayStatus = { ...status(), phase: 'stopped' };
    const bridge = new NativeBridge(async <T>(name: string) => {
      calls.push(name);
      return (name === 'engine_read' ? project() : { ...current, ...(current.logs ? { logs: { ...current.logs } } : {}) }) as T;
    });
    await bridge.start();
    await bridge.refreshPlay();
    const first = bridge.getSnapshot();
    for (let i = 0; i < 4; i++) await bridge.refreshPlay();
    expect(bridge.getSnapshot()).toBe(first);
    const { logs: _logs, ...old } = current;
    current = old;
    await bridge.refreshPlay();
    const olderHost = bridge.getSnapshot();
    await bridge.refreshPlay();
    expect(bridge.getSnapshot()).toBe(olderHost);
    expect(calls).not.toContain('play_logs_read');
  });
  it('copies only new rows, retains ordered same-tick duplicates and levels, and merges existing diagnostics', async () => {
    const calls: { name: string; args?: Record<string, unknown> }[] = [];
    let current = status(4);
    const entries = [row(1, 'same', 'debug'), row(2, 'same', 'info'), row(3, 'same', 'warn'), row(4, 'same', 'error')];
    const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
      calls.push({ name, ...(args ? { args } : {}) });
      if (name === 'engine_read') return project() as T;
      if (name === 'play_read') return { ...current, logs: { ...current.logs } } as T;
      return batch(entries.slice(Number(args!.afterIndex)), Number(args!.afterIndex)) as T;
    });
    await bridge.start();
    await bridge.refreshPlay();
    expect(scriptRows(bridge).map(entry => entry.level)).toEqual(['debug', 'info', 'warn', 'error']);
    expect(new Set(scriptRows(bridge).map(entry => entry.id)).size).toBe(4);
    expect(scriptRows(bridge).every(entry => Number.isFinite(Date.parse(entry.at)))).toBe(true);
    expect(new Set(scriptRows(bridge).map(entry => entry.at)).size).toBe(1);
    const before = bridge.getSnapshot();
    await bridge.refreshPlay();
    expect(bridge.getSnapshot()).toBe(before);
    current = status(5, 0, { sequence: 2, phase: 'paused', tick: 2 });
    entries.push(row(5, 'next'));
    await bridge.refreshPlay();
    expect(scriptRows(bridge).map(entry => entry.message)).toEqual(['same', 'same', 'same', 'same', 'next']);
    expect(calls.filter(call => call.name === 'play_logs_read')).toEqual([
      { name: 'play_logs_read', args: { generation: 1, afterIndex: 0 } },
      { name: 'play_logs_read', args: { generation: 1, afterIndex: 4 } },
    ]);
    await bridge.start();
    expect(scriptRows(bridge)).toHaveLength(5);
    expect(bridge.getSnapshot().console.some(entry => entry.message === 'Existing native diagnostic')).toBe(true);
    expect(bridge.getSnapshot().history.entries).toEqual([]);
    expect(calls.filter(call => call.name === 'engine_read')).toHaveLength(2);
  });
  it.each(['failure', 'stop', 'restart', 'project'] as const)('fences an unread late response after %s, retaining delivered history only within the project', async (transition) => {
    const reply = deferred<PlayLogBatch>();
    let document = project();
    let current = status(1);
    let reads = 0;
    const bridge = new NativeBridge(async <T>(name: string) => {
      if (name === 'engine_read') return document as T;
      if (name === 'play_read') return current as T;
      reads++;
      return (reads === 1 ? batch([row(1)]) : await reply.promise) as T;
    });
    await bridge.start();
    await bridge.refreshPlay();
    expect(scriptRows(bridge)).toHaveLength(1);
    current = status(2, 0, { sequence: 2 });
    const pending = bridge.refreshPlay();
    // Wait until the log command is actually pending, rather than only the status read.
    await Promise.resolve(); await Promise.resolve();
    expect(reads).toBe(2);
    if (transition === 'failure') bridge.updatePlay(status(2, 0, { phase: 'error', sequence: 3, error: 'callback failed' }));
    if (transition === 'stop') bridge.updatePlay(status(0, 0, { generation: 2, sequence: 3, phase: 'stopped' }));
    if (transition === 'restart') bridge.updatePlay(status(0, 0, { generation: 3, sequence: 4 }));
    if (transition === 'project') { document = project(id(3)); await bridge.start(); }
    reply.finish(batch([row(2)], 1));
    await pending;
    expect(scriptRows(bridge).map(entry => entry.message)).toEqual(transition === 'project' ? [] : ['same']);
    expect(bridge.getSnapshot().playError).toBeUndefined();
  });
  it('does not reattach old-generation logs to a new project and recovers on a new Play generation', async () => {
    let document = project();
    let current = status(1);
    let reads = 0;
    const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return document as T;
      if (name === 'play_read') return current as T;
      reads++;
      return batch([row(reads, `generation ${current.generation}`)], 0, Number(args!.generation)) as T;
    });
    await bridge.start(); await bridge.refreshPlay();
    document = project(id(3)); await bridge.start();
    await bridge.refreshPlay();
    expect(scriptRows(bridge)).toHaveLength(0);
    expect(reads).toBe(1);
    current = status(1, 0, { generation: 2, sequence: 2 });
    await bridge.refreshPlay();
    expect(scriptRows(bridge).map(entry => entry.message)).toEqual(['generation 2']);
  });
  it('coalesces overlapping reads and discards a stale error behind an authoritative terminal state', async () => {
    const reply = deferred<PlayLogBatch>();
    const calls: string[] = [];
    const bridge = new NativeBridge(async <T>(name: string) => {
      calls.push(name);
      return (name === 'engine_read' ? project() : name === 'play_read' ? status(1) : await reply.promise) as T;
    });
    await bridge.start();
    const pending = bridge.refreshPlay();
    const overlap = bridge.refreshPlay();
    await Promise.resolve(); await Promise.resolve();
    bridge.updatePlay(status(1, 0, { phase: 'error', sequence: 2, error: 'terminal' }));
    reply.reject('credential-like exception must not surface');
    await Promise.all([pending, overlap]);
    expect(calls.filter(name => name === 'play_logs_read')).toHaveLength(1);
    expect(bridge.getSnapshot().playError).toBeUndefined();
    expect(scriptRows(bridge)).toHaveLength(0);
  });
  it('surfaces fixed safe errors and retries the unchanged cursor on recovery', async () => {
    const cursors: unknown[] = [];
    let fail = true;
    const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'play_read') return status(1) as T;
      cursors.push(args!.afterIndex);
      if (fail) throw 'private arbitrary exception';
      return batch([row(1)]) as T;
    });
    await bridge.start(); await bridge.refreshPlay();
    expect(bridge.getSnapshot().playError).toEqual({ code: 'play.logs', message: 'Native Play logs are unavailable. Refresh Play to retry.' });
    expect(JSON.stringify(bridge.getSnapshot())).not.toContain('private arbitrary');
    fail = false;
    await bridge.refreshPlay();
    expect(bridge.getSnapshot().playError).toBeUndefined();
    expect(scriptRows(bridge)).toHaveLength(1);
    expect(cursors).toEqual([0, 0]);
  });
  it.each([
    { ...batch([row(1)]), next: 2 },
    { ...batch([row(1)]), generation: 2 },
    { ...batch([row(1)]), from: 1 },
    { ...batch([row(1)]), dropped: -1 },
    { ...batch([row(1)]), warningId: 'not-an-id' },
    batch([row(1), row(1)]),
    batch([{ ...row(1), tick: Number.MAX_SAFE_INTEGER + 1 }]),
    batch([{ ...row(1), message: 'x'.repeat(1025) }]),
    batch([{ ...row(1), message: '😀'.repeat(257) }]),
    batch([{ ...row(1), level: 'trace' as 'info' }]),
    batch(Array.from({ length: 65 }, (_, i) => row(i))),
    batch(Array.from({ length: 17 }, (_, i) => row(i, 'x'.repeat(1024)))),
    { ...batch([row(1)]), unknown: 'ignored' },
  ])('refuses the entire malformed batch without publishing its valid prefix or advancing the cursor', async (invalid) => {
    let response: unknown = invalid;
    const cursors: unknown[] = [];
    const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'play_read') return status(1) as T;
      cursors.push(args!.afterIndex);
      return response as T;
    });
    await bridge.start(); await bridge.refreshPlay();
    expect(scriptRows(bridge)).toHaveLength(0);
    expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
    response = batch([row(1)]);
    await bridge.refreshPlay();
    expect(scriptRows(bridge)).toHaveLength(1);
    expect(cursors).toEqual([0, 0]);
  });
  it.each([[1024, 256, 16, 385], [1, 1024, 64, 1153]])('keeps Console bounded across restarts (%i-byte rows), warning on source drops and retention', async (size, count, perBatch, omitted) => {
    let current = status(count, 64);
    const reads: number[] = [];
    const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'play_read') return current as T;
      const from = Number(args!.afterIndex);
      reads.push(from);
      const next = Math.min(from + perBatch, count);
      return batch(Array.from({ length: next - from }, (_, i) => row(current.generation * 2000 + from + i, 'x'.repeat(size))),
        from, current.generation, current.logs!.dropped) as T;
    });
    await bridge.start();
    for (let generation = 1; generation <= 2; generation++) {
      current = status(count, 64, { generation, sequence: generation });
      for (let i = 0; i < count / perBatch; i++) await bridge.refreshPlay();
      const rows = scriptRows(bridge);
      expect(rows.length).toBeLessThanOrEqual(1024);
      expect(rows.reduce((sum, entry) => sum + new TextEncoder().encode(entry.message).length, 0)).toBeLessThanOrEqual(256 * 1024);
      expect(rows.filter(entry => entry.id === warningId)).toHaveLength(1);
      expect(rows.at(-1)?.id).toBe(warningId); // Equal timestamp; warning follows accepted order.
    }
    expect(reads.filter(cursor => cursor === 0)).toHaveLength(2);
    expect(scriptRows(bridge).find(entry => entry.id === warningId)?.message).toContain(`${omitted.toLocaleString('en-US')} messages`);
    const stable = bridge.getSnapshot();
    await bridge.refreshPlay();
    expect(bridge.getSnapshot()).toBe(stable);
    current = status(count, 65, { generation: 2, sequence: 3 });
    await bridge.refreshPlay();
    expect(reads.at(-1)).toBe(count);
    expect(scriptRows(bridge).find(entry => entry.id === warningId)?.message).toContain(`${(omitted + 1).toLocaleString('en-US')} messages`);
    expect(scriptRows(bridge).filter(entry => entry.id === warningId)).toHaveLength(1);
  });
  it('accepts a newer copied prefix than the prior status read without reopening or duplicating its cursor', async () => {
    let logReads = 0;
    const bridge = new NativeBridge(async <T>(name: string) => {
      if (name === 'engine_read') return project() as T;
      if (name === 'play_read') return status(1) as T;
      logReads++;
      return batch([row(1), row(2)]) as T;
    });
    await bridge.start(); await bridge.refreshPlay(); await bridge.refreshPlay();
    expect(scriptRows(bridge)).toHaveLength(2);
    expect(logReads).toBe(1);
    expect(bridge.getSnapshot().playError).toBeUndefined();
  });
  it('refuses malformed new host metadata without invoking the log command', async () => {
    const calls: string[] = [];
    const invoke: Invoke = async <T>(name: string) => { calls.push(name); return (name === 'engine_read' ? project() : status(-1)) as T; };
    const bridge = new NativeBridge(invoke);
    await bridge.start(); await bridge.refreshPlay();
    expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
    expect(calls).not.toContain('play_logs_read');
  });
});

it('fences an old project status-read failure without publishing it in the new project', async () => {
  const old = deferred<PlayStatus>();
  let document = project();
  const bridge = new NativeBridge(async <T>(name: string) => (name === 'engine_read' ? document : await old.promise) as T);
  await bridge.start();
  const pending = bridge.refreshPlay();
  document = project(id(3));
  await bridge.start();
  old.reject('obsolete project failure');
  await pending;
  expect(bridge.getSnapshot().playError).toBeUndefined();
  expect(scriptRows(bridge)).toHaveLength(0);
});
it('refuses repeated retained IDs across restart without duplicating Console keys', async () => {
  let current = status(1);
  const bridge = new NativeBridge(async <T>(name: string) => (name === 'engine_read' ? project() : name === 'play_read' ? current : batch([row(1)], 0, current.generation)) as T);
  await bridge.start(); await bridge.refreshPlay();
  current = status(1, 0, { generation: 2, sequence: 2 });
  await bridge.refreshPlay();
  expect(scriptRows(bridge)).toHaveLength(1);
  expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
});
it('enforces the cumulative native-prefix byte bound before accepting a later batch', async () => {
  const cursors: number[] = [];
  const bridge = new NativeBridge(async <T>(name: string, args?: Record<string, unknown>) => {
    if (name === 'engine_read') return project() as T;
    if (name === 'play_read') return status(272) as T;
    const from = Number(args!.afterIndex);
    cursors.push(from);
    return batch(Array.from({ length: 16 }, (_, i) => row(from + i, 'x'.repeat(1024))), from) as T;
  });
  await bridge.start();
  for (let i = 0; i < 16; i++) await bridge.refreshPlay();
  const before = bridge.getSnapshot().console;
  await bridge.refreshPlay();
  expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
  expect(bridge.getSnapshot().console).toBe(before);
  await bridge.refreshPlay();
  expect(cursors.slice(-2)).toEqual([256, 256]);
});
it('keeps a newer malformed-status error visible instead of accepting an older pending log packet', async () => {
  const reply = deferred<PlayLogBatch>();
  const bridge = new NativeBridge(async <T>(name: string) => (name === 'engine_read' ? project() : name === 'play_read' ? status(1) : await reply.promise) as T);
  await bridge.start();
  const pending = bridge.refreshPlay();
  await Promise.resolve(); await Promise.resolve();
  bridge.updatePlay(status(-1, 0, { sequence: 2 }));
  expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
  reply.finish(batch([row(1)]));
  await pending;
  expect(scriptRows(bridge)).toHaveLength(0);
  expect(bridge.getSnapshot().playError?.code).toBe('play.logs');
});
