/**
 * DETERMINISTIC ALLOWANCE FIXTURE DATA for handoff 0046. Invented content, not engine or
 * provider output.
 *
 * The project is handoff 0045's agent fixture (0033's invented harbor scene with the real
 * component schemas). Hosts with a project allowance add what the real bridge requires
 * before it offers `agent.budget.set`: `agent_budget: { available: true }`, the published
 * schemas/Project.schema.json and valid settings. The older host omits all three, so the
 * same bridge falls back to the session's own budget. Sessions use the native snake_case
 * wire form, so the bridge validates them exactly as it would a native reply.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PATCH, SIGNED_IN, SIGNED_OUT, fixtureEngine } from '../../0045-native-agent-chat/tools/scenarios.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const projectSchema = JSON.parse(readFileSync(join(resolve(here, '../../..'), 'schemas/Project.schema.json'), 'utf8'));

/** A host with the durable allowance. `cap` undefined leaves the schema's 1,000,000 default in force. */
function budgetEngine(cap) {
  const engine = fixtureEngine();
  return {
    ...engine,
    project: { ...engine.project, name: 'Harbor (allowance fixture)', settings: { tick_rate: 60, ...(cap === undefined ? {} : { agent_budget: { max_tokens: cap } }) } },
    schemas: { ...engine.schemas, Project: projectSchema },
    agent_budget: { available: true },
  };
}
/** An older host: no allowance support, no Project schema, no settings. */
function olderEngine() {
  const engine = fixtureEngine();
  return { ...engine, project: { ...engine.project, name: 'Harbor (older host fixture)' } };
}

const request = (prompt, approval = 'destructive') => ({ prompt, model: 'gpt-6-astra', approval });
const turn = (id, prompt, fields = {}) => ({
  id,
  request: request(prompt),
  status: 'completed',
  text: '',
  approvals: [],
  pending: null,
  report: null,
  failure: null,
  charged_tokens: 0,
  accounting_uncertain: false,
  ...fields,
});
const report = (edits, tools) => ({
  steps: 4,
  usage: { input_tokens: 6012, output_tokens: 520 },
  transaction_count: edits,
  tool_call_count: tools,
  transaction_ids: Array.from({ length: edits }, (_, n) => `01JA45FX${String(n).padStart(18, '0')}`),
  tool_calls: [],
  truncated: false,
});
const bytes = (value) => new TextEncoder().encode(JSON.stringify(value)).length;

const crates = turn(1, 'Stack the crates two high by the gate and hang a warm lantern above them.', {
  text: 'I raised Crate Stack onto the upper pallet and moved Lantern 1.8 m above it with a warmer colour. Both edits are one transaction in History.',
  approvals: [{ id: 1, tool: 'doc_patch', bytes: 1184, decision: 'allowed' }],
  report: report(1, 3),
  charged_tokens: 6532,
});
const rename = turn(2, 'Rename every crate to follow the Crate NN pattern.', {
  status: 'cancelled',
  text: 'Renaming the crates in hierarchy order: Crate Stack → Crate 01, ',
  approvals: [{ id: 2, tool: 'doc_patch', bytes: 912, decision: 'cancelled' }],
  charged_tokens: 1180,
  accounting_uncertain: true,
});
const awaiting = turn(3, 'Make the lantern visible from the close-up camera, and drop the spot light by the gate.', {
  status: 'awaiting_approval',
  text: 'The lantern sits just outside the close-up camera’s view. I’ll move it toward the gate, add a soft fill light and remove Gate Spot.',
  approvals: [{ id: 4, tool: 'doc_patch', bytes: bytes(PATCH), decision: 'pending' }],
  pending: { turn: 3, id: 4, tool: 'doc_patch', arguments: PATCH },
});

const session = (turns, { used, max, uncertain, worker = false, version = 12 }) => ({
  version,
  budget: { max_tokens: max, used_tokens: used ?? turns.reduce((sum, item) => sum + item.charged_tokens, 0) },
  accounting_uncertain: uncertain ?? turns.some((item) => item.accounting_uncertain),
  worker_running: worker,
  turns,
});

const held = [crates, rename];
const USED = 7712;
const base = (engine, sessionValue, extra = {}) => ({ engine, provider: SIGNED_IN, session: sessionValue, ...extra });

export const MAX = Number.MAX_SAFE_INTEGER;
export const SCENARIOS = {
  // The schema's default allowance: the bridge reads it from the published Project schema.
  'ready-default': base(budgetEngine(undefined), session([], { used: 0, max: 1_000_000, version: 0 })),
  ready: base(budgetEngine(250_000), session(held, { max: 250_000 })),
  'ready-max': base(budgetEngine(MAX), session(held, { max: MAX })),
  approval: base(budgetEngine(250_000), session([crates, awaiting], { used: 6532, max: 250_000, worker: true })),
  exhausted: base(budgetEngine(250_000), session(held, { used: 250_000, max: 250_000 })),
  'over-cap': base(budgetEngine(5_000), session(held, { max: 5_000 })),
  zero: base(budgetEngine(0), session(held, { max: 0 })),
  // After an unclean exit the whole remaining allowance stays reserved, marked uncertain.
  reserved: base(budgetEngine(1_000_000), session(held, { used: 1_000_000, max: 1_000_000, uncertain: true })),
  'signed-out': base(budgetEngine(250_000), session(held, { max: 250_000 }), { provider: SIGNED_OUT }),
  'older-host': base(olderEngine(), session(held, { max: 200_000 })),
  'older-exhausted': base(olderEngine(), session(held, { used: 200_000, max: 200_000 })),
  // Flows.
  'flow-slow-save': base(budgetEngine(250_000), session(held, { max: 250_000 }), { executeDelay: 60_000 }),
  'flow-save': base(budgetEngine(250_000), session(held, { used: 250_000, max: 250_000 }), { executeDelay: 300 }),
  'flow-save-fails': base(budgetEngine(250_000), session(held, { max: 250_000 }), { executeError: { code: 'engine.revision-mismatch', message: 'Project revision changed.' } }),
  'flow-lag': base(budgetEngine(250_000), session(held, { max: 250_000 }), { sessionLag: 1_000 }),
  'flow-undo': base(budgetEngine(250_000), session(held, { max: 250_000 }), { executeDelay: 150, historyDelay: 150 }),
  'flow-running': base(budgetEngine(250_000), session([crates, awaiting], { used: 6532, max: 250_000, worker: true }), { executeDelay: 400 }),
  'flow-offline': base(budgetEngine(250_000), session(held, { max: 250_000 }), { provider: SIGNED_OUT, executeDelay: 150 }),
};
export { USED, SIGNED_IN, SIGNED_OUT };
