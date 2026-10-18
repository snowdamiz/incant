import { describe, expect, it } from 'vitest';
import type { AgentFailure, AgentSessionSnapshot, AgentTurnSnapshot, BridgeSnapshot, Ulid } from '../../bridge/contract';
import { agentPhase } from './agentPhase';
import {
  argumentsText,
  asStatement,
  failureSentence,
  plainText,
  reveal,
  reviewApproval,
  toolPhrase,
  turnCost,
} from './agentText';
import type { Names } from './agentText';
import { allowanceOf, allowanceText } from './tokenAllowance';

const names: Names = {
  entity: (id) => ({ E1: 'Crate 03', S1: 'Harbor' } as Record<string, string>)[id],
  asset: (id) => (id === 'A1' ? 'crate_albedo' : undefined),
};
const words = (parts: Parameters<typeof plainText>[0]) => plainText(parts);

describe('failure wording', () => {
  it.each([
    ['provider', 'The model provider did not complete the request.'],
    ['step_limit', 'The agent reached its step limit for one request.'],
    ['context_limit', 'The conversation is too long for the model.'],
    ['budget_accounting', 'Token use could not be recorded.'],
  ] as const)('says %s in plain words', (failure, sentence) => {
    expect(failureSentence({ failure, approvals: [] })).toBe(sentence);
  });

  it('does not infer that an earlier denial caused a later tool failure', () => {
    expect(failureSentence({ failure: 'tool', approvals: [{ id: 1, tool: 'doc_patch', bytes: 9, decision: 'denied' }] })).toBe('A tool the agent used failed.');
    expect(failureSentence({ failure: 'tool', approvals: [] })).toBe('A tool the agent used failed.');
  });

  it('falls back for a missing or unknown failure code', () => {
    expect(failureSentence({ failure: null, approvals: [] })).toBe('The request failed.');
    expect(failureSentence({ failure: 'something_new' as AgentFailure, approvals: [] })).toBe('The request failed.');
    expect(failureSentence({ failure: 'toString' as AgentFailure, approvals: [] })).toBe('The request failed.');
  });
});

describe('token accounting', () => {
  it('shows the actual charge, marks an uncertain one as estimated, and never calls missing usage free', () => {
    expect(turnCost({ charged_tokens: 1240, accounting_uncertain: false })).toBe('1,240 tokens');
    expect(turnCost({ charged_tokens: 1, accounting_uncertain: false })).toBe('1 token');
    expect(turnCost({ charged_tokens: 900, accounting_uncertain: true })).toBe('900 tokens (estimated)');
    expect(turnCost({ charged_tokens: 0, accounting_uncertain: true })).toBe('Token use unknown');
  });

  it('states the session allowance and flags estimates and exhaustion', () => {
    const session = (used: number, uncertain = false): AgentSessionSnapshot => ({
      version: 1,
      budget: { max_tokens: 200000, used_tokens: used },
      accounting_uncertain: uncertain,
      worker_running: false,
      turns: [],
    });
    // An older host: no project allowance, so the session's own budget is the allowance.
    const usage = (value: AgentSessionSnapshot) => {
      const allowance = allowanceOf({ connection: { status: 'ready', project: { id: 'P' as Ulid, name: 'P' } }, agentSession: value })!;
      return { text: allowanceText(allowance), level: allowance.level };
    };
    expect(usage(session(6532))).toEqual({ text: '6,532 of 200,000 tokens used', level: 'normal' });
    expect(usage(session(170000, true))).toEqual({ text: '170,000 of 200,000 tokens used (some estimated)', level: 'low' });
    expect(usage(session(200000)).level).toBe('out');
  });
});

describe('approval review', () => {
  it('identifies registered Rust source edits without suggesting compilation or file deletion', () => {
    const review = reviewApproval('doc_patch', { expected_revision: 2, description: 'Rust source', ops: [
      { op: 'upsert_module_source', module: { id: 'M1', name: 'Movement', path: 'movement/main.rs' } },
      { op: 'write_module_source', module_id: 'M1', source: 'é', expected_source: { state: 'missing' } },
      { op: 'remove_module_source', module_id: 'M1' },
    ] }, names);
    expect(review.rows.map((row) => [row.kind, words(row.parts)])).toEqual([
      ['change', 'Register Rust source movement/main.rs'],
      ['change', 'Replace Rust source M1 (2 bytes)'],
      ['remove', 'Unregister Rust source M1 (file retained)'],
    ]);
  });

  it('reads document operations from the exact arguments, naming known entities', () => {
    const review = reviewApproval(
      'doc_patch',
      {
        description: 'Tidy the crates',
        expected_revision: 3,
        ops: [
          { op: 'set_component', scene_id: 'S1', entity_id: 'E1', component: 'Transform', value: {} },
          { op: 'rename_entity', scene_id: 'S1', entity_id: 'E1', name: 'Crate 04' },
          { op: 'create_entity', scene_id: 'S1', entity: { id: 'N1', name: 'Lantern', components: { Transform: {}, PointLight: {} } } },
          { op: 'delete_entity', scene_id: 'S1', entity_id: 'UNKNOWN' },
          { op: 'remove_asset', asset_id: 'A1' },
          { op: 'teleport_everything' },
        ],
      },
      names,
    );
    expect(words(review.title)).toBe('Apply 6 changes to the project?');
    expect(review.description).toBe('Tidy the crates');
    expect(review.rows.map((row) => [row.kind, words(row.parts)])).toEqual([
      ['change', 'Set Transform on Crate 03'],
      ['change', 'Rename Crate 03 to Crate 04'],
      ['add', 'Create Lantern with Transform, PointLight'],
      ['remove', 'Delete UNKNOWN'],
      ['remove', 'Remove asset crate_albedo'],
      ['unknown', 'Unrecognised operation teleport_everything'],
    ]);
    // An ID the project does not know is shown as the ID, in monospace.
    expect(review.rows[3]!.parts[1]).toEqual({ name: 'UNKNOWN', mono: true });
    expect(words(asStatement(review.title))).toBe('Apply 6 changes to the project');
  });

  it('lists a bounded number of operations and counts the rest', () => {
    const ops = Array.from({ length: 30 }, (_, n) => ({ op: 'delete_entity', scene_id: 'S1', entity_id: `X${n}` }));
    const review = reviewApproval('doc_patch', { description: 'Clear', expected_revision: 1, ops }, names);
    expect(review.rows).toHaveLength(8);
    expect(review.more).toBe(22);
  });

  it('says plainly when the arguments do not match the tool', () => {
    const review = reviewApproval('doc_patch', { ops: 'not a list' }, names);
    expect(words(review.title)).toBe('Change the project?');
    expect(review.rows.map((row) => row.kind)).toEqual(['unknown']);
  });

  it.each([
    ['asset_import', { description: 'Add', expected_revision: 1, sources: [{ source: 'textures/a.png', texture_usage: 'linear' }] }, 'Import 1 asset?'],
    ['script_write', { description: 'Swap', expected_revision: 1, script_id: 'SC1', source: 'x'.repeat(2048), expected_source: { state: 'missing' } }, 'Replace the source of script SC1?'],
    ['view_screenshot', { camera: 'E1', width: 640, height: 360 }, 'Capture a 640 × 360 screenshot from Crate 03?'],
    ['view_screenshot', { camera: null, width: 64, height: 64 }, 'Capture a 64 × 64 screenshot from the editor preview?'],
    ['doc_query', { path: '' }, 'Read the whole project?'],
    ['play_run', { seconds: 2.5, scene: null }, 'Run the game for 2.5 s?'],
    ['asset_inspect', { id: 'A1' }, 'Inspect asset crate_albedo?'],
    ['mystery_tool', { anything: true }, 'Use the tool mystery_tool?'],
  ] as const)('summarises %s', (tool, args, title) => {
    expect(words(reviewApproval(tool, args, names).title)).toBe(title);
  });

  it('treats every argument string as untrusted: texture usage and component names are revealed and bounded', () => {
    const imported = reviewApproval('asset_import', { description: 'x', expected_revision: 1, sources: [{ source: 'a.png', texture_usage: 'color\u202E' + 'z'.repeat(400) }] }, names);
    const usage = imported.rows[0]!.parts[3] as { name: string };
    expect(usage.name.startsWith('color\\u202E')).toBe(true);
    expect(usage.name.length).toBeLessThanOrEqual(120);
    const components = Object.fromEntries(Array.from({ length: 9 }, (_, n) => [`Part${n}${'q'.repeat(n === 0 ? 300 : 0)}`, {}]));
    const created = reviewApproval('doc_patch', { description: 'x', expected_revision: 1, ops: [{ op: 'create_entity', scene_id: 'S1', entity: { id: 'N', name: 'Box', components } }] }, names);
    const text = words(created.rows[0]!.parts);
    expect(text).toMatch(/ and 3 more$/);
    expect(text.length).toBeLessThan(220);
  });

  it('names a decided tool without its arguments', () => {
    expect(words(toolPhrase('doc_patch'))).toBe('Project changes');
    expect(words(toolPhrase('custom_tool'))).toBe('Tool custom_tool');
  });

  it('reveals bidirectional overrides and zero-width characters instead of letting them reorder a name', () => {
    expect(reveal('crate‮gnp.exe')).toBe('crate\\u202Egnp.exe');
    expect(reveal('a​b﻿')).toBe('a\\u200Bb\\uFEFF');
    const review = reviewApproval('doc_patch', { description: 'x', expected_revision: 1, ops: [{ op: 'create_entity', scene_id: 'S1', entity: { id: 'N', name: 'Evil‮txt.exe', components: {} } }] }, names);
    expect(words(review.rows[0]!.parts)).toBe('Create Evil\\u202Etxt.exe');
  });

  it('bounds the exact arguments it renders until all are asked for', () => {
    const args = { source: 'y'.repeat(100_000) };
    const preview = argumentsText(args, 64 * 1024);
    expect(preview.truncated).toBe(true);
    expect(preview.text).toHaveLength(64 * 1024);
    expect(argumentsText(args, Number.POSITIVE_INFINITY)).toMatchObject({ truncated: false, length: preview.length });
  });
});

/* ---- Phases ---- */

const turn = (id: number, status: string, fields: Partial<AgentTurnSnapshot> = {}): AgentTurnSnapshot => ({
  id,
  request: { prompt: 'p', model: 'gpt-6-astra', approval: 'destructive' },
  status: status as AgentTurnSnapshot['status'],
  text: '',
  approvals: [],
  pending: null,
  report: null,
  failure: null,
  charged_tokens: 0,
  accounting_uncertain: false,
  ...fields,
});
const snapshotWith = (agent: BridgeSnapshot['agent'], turns: AgentTurnSnapshot[] | null, worker = false) =>
  ({ agent, ...(turns ? { agentSession: { version: 1, budget: { max_tokens: 10, used_tokens: 0 }, accounting_uncertain: false, worker_running: worker, turns } } : {}) }) as unknown as BridgeSnapshot;
const idle = { sending: false, cancelling: new Set<number>(), startedTurn: null };
const running = { status: 'running', conversation: 'c' } as const;

describe('agent phase', () => {
  it('follows the session: running, approval, stopping and finished', () => {
    expect(agentPhase(snapshotWith(running, [turn(1, 'running')], true), idle)).toEqual({ phase: 'running', stoppable: 1 });
    expect(agentPhase(snapshotWith(running, [turn(1, 'awaiting_approval')], true), idle)).toEqual({ phase: 'approval', stoppable: 1 });
    expect(agentPhase(snapshotWith(running, [turn(1, 'stopping')], true), idle)).toEqual({ phase: 'stopping', stoppable: null });
    expect(agentPhase(snapshotWith({ status: 'idle' }, [turn(1, 'completed')]), idle)).toEqual({ phase: 'idle', stoppable: null });
  });

  it('keeps a new turn unavailable until the native worker has actually stopped', () => {
    expect(agentPhase(snapshotWith(running, [turn(1, 'completed')], true), idle).phase).toBe('finishing');
    expect(agentPhase(snapshotWith(running, [turn(1, 'cancelled')], true), idle).phase).toBe('stopping');
  });

  it('treats a stop requested here as stopping, even before the session says so', () => {
    expect(agentPhase(snapshotWith(running, [turn(4, 'running')], true), { ...idle, cancelling: new Set([4]) })).toEqual({ phase: 'stopping', stoppable: null });
  });

  it('can stop a turn reserved by its start receipt before the session lists it', () => {
    expect(agentPhase(snapshotWith(running, [turn(3, 'completed')]), { ...idle, startedTurn: 4 })).toEqual({ phase: 'starting', stoppable: 4 });
    expect(agentPhase(snapshotWith({ status: 'idle' }, [turn(3, 'completed')]), { ...idle, sending: true }).phase).toBe('starting');
  });

  it('keeps an active turn stoppable when the host reports the agent unavailable (signed out)', () => {
    expect(agentPhase(snapshotWith({ status: 'unavailable', reason: 'Connect an OpenAI account to use the agent.' }, [turn(2, 'running')], true), idle)).toEqual({ phase: 'running', stoppable: 2 });
  });

  it('calls a host that is busy for a reason its session does not show "running", with nothing to stop', () => {
    expect(agentPhase(snapshotWith(running, [turn(3, 'completed')]), idle)).toEqual({ phase: 'running', stoppable: null });
  });

  it('works on older hosts without a session', () => {
    expect(agentPhase(snapshotWith(running, null), idle)).toEqual({ phase: 'running', stoppable: null });
    expect(agentPhase(snapshotWith({ status: 'idle' }, null), idle).phase).toBe('idle');
    expect(agentPhase(snapshotWith({ status: 'unavailable', reason: 'x' }, null), idle).phase).toBe('unavailable');
    expect(agentPhase(null, idle).phase).toBe('unavailable');
  });

  it('does not treat an unknown status as active', () => {
    expect(agentPhase(snapshotWith({ status: 'idle' }, [turn(1, 'paused_by_host')]), idle).phase).toBe('idle');
  });
});
