/**
 * DETERMINISTIC AGENT FIXTURE DATA for handoff 0045. Invented content, not engine or
 * provider output.
 *
 * The project is the evidence double from handoff 0033 (an invented harbor scene with
 * the real component schemas), renamed "(agent fixture)". Sessions are written in the
 * native session's snake_case wire form, so the real compiled bridge validates them
 * exactly as it would a native reply. Account labels are invented example addresses.
 */
import { reviewRead } from '../../0033-editor-design-review/tools/review-double.mjs';

const { read } = reviewRead();
const scene = Object.values(read.project.scenes)[0];
const byName = (name) => Object.values(scene.entities).find((entity) => entity.name === name).id;
export const IDS = {
  scene: scene.id,
  environment: byName('Environment'),
  lantern: byName('Lantern'),
  gateSpot: byName('Gate Spot'),
  closeUp: byName('Close-up Camera'),
  crates: byName('Crate Stack'),
  navmesh: byName('Harbor NavMesh'),
};

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
/** A ULID whose time part is `iso`, so History shows a sensible time. */
function ulidAt(iso, serial) {
  let time = Date.parse(iso);
  let prefix = '';
  for (let i = 0; i < 10; i += 1) {
    prefix = CROCKFORD[time % 32] + prefix;
    time = Math.floor(time / 32);
  }
  return `${prefix}${String(serial).padStart(16, '0')}`;
}

/** Agent edits in History, attributed to the fixture conversation. */
const agentEdit = (iso, serial, description) => ({
  id: ulidAt(iso, serial),
  description,
  actor: { origin: 'agent', actor: 'agent', model: 'gpt-6-astra', conversation_id: 'agent-fixture' },
});

export function fixtureEngine() {
  return {
    ...read,
    project: { ...read.project, name: 'Harbor (agent fixture)' },
    revision: 7,
    history: [
      { id: ulidAt('2026-10-11T13:58:00Z', 1), description: 'Rename Lantern', actor: { origin: 'user', actor: 'user', model: null, conversation_id: null } },
      agentEdit('2026-10-11T14:02:00Z', 2, 'Raise Crate Stack and hang Lantern above it'),
    ],
    applied: 2,
    viewport_error: 'Agent fixture: a browser page has no native surface.',
  };
}

export const ADA = { id: 'acct-fixture-1', label: 'ada@example.com' };
export const SIGNED_IN = { status: 'connected', provider: 'openai', method: 'oauth', accountLabel: ADA.label, accounts: [ADA], activeAccount: ADA.id };
export const SIGNED_OUT = { status: 'not-connected', provider: 'openai', accounts: [ADA] };

const request = (prompt, approval = 'destructive', model = 'gpt-6-astra') => ({ prompt, model, approval });
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
  transaction_ids: Array.from({ length: edits }, (_, n) => ulidAt('2026-10-11T14:02:00Z', 2 + n)),
  tool_calls: [],
  truncated: false,
});
const bytes = (value) => new TextEncoder().encode(JSON.stringify(value)).length;

/* ---- Turns ---- */

const crates = turn(1, 'Stack the crates two high by the gate and hang a warm lantern above them.', {
  text: 'I raised Crate Stack onto the upper pallet (y 1.02 → 2.04) and moved Lantern 1.8 m above it with a warmer colour. Both edits are one transaction in History, so a single Undo reverts them.',
  approvals: [{ id: 1, tool: 'doc_patch', bytes: 1184, decision: 'allowed' }],
  report: report(1, 3),
  charged_tokens: 6532,
});

const navmesh = turn(2, 'Bake a navigation mesh for the whole harbor and tell me which areas are unreachable.', {
  status: 'failed',
  text: 'I’ll start from the Harbor NavMesh settings. Its cell size is outside the supported range, so I’ll',
  failure: 'provider',
  charged_tokens: 0,
  accounting_uncertain: true,
});

const rename = turn(3, 'Rename every crate to follow the Crate NN pattern.', {
  status: 'cancelled',
  text: 'Renaming the crates in hierarchy order: Crate Stack → Crate 01, ',
  approvals: [{ id: 2, tool: 'doc_patch', bytes: 912, decision: 'cancelled' }],
  charged_tokens: 1180,
  accounting_uncertain: true,
});

const lights = turn(4, 'Remove the lights we are not using.', {
  status: 'failed',
  text: 'Gate Spot is not seen by any authored camera and nothing references it. I’d like to delete it.',
  approvals: [{ id: 3, tool: 'doc_patch', bytes: 288, decision: 'denied' }],
  failure: 'tool',
  charged_tokens: 940,
});

export const PATCH = {
  description: 'Move Lantern into the close-up view, add a fill light and remove Gate Spot',
  expected_revision: 7,
  ops: [
    { op: 'set_component', scene_id: IDS.scene, entity_id: IDS.lantern, component: 'Transform', value: { translation: [2.65, 2.4, -1.5], rotation: [0, 0, 0, 1], scale: [1, 1, 1] } },
    { op: 'set_component', scene_id: IDS.scene, entity_id: IDS.lantern, component: 'PointLight', value: { color: [1, 0.62, 0.3], intensity: 800, range: 7.5 } },
    {
      op: 'create_entity',
      scene_id: IDS.scene,
      entity: {
        id: '01JA45FX00000000000000FILL',
        name: 'Lantern Fill',
        parent: IDS.environment,
        components: { Transform: { translation: [2.4, 1.2, -1.1], rotation: [0, 0, 0, 1], scale: [1, 1, 1] }, PointLight: { color: [1, 0.7, 0.45], intensity: 120, range: 3 } },
      },
    },
    { op: 'delete_entity', scene_id: IDS.scene, entity_id: IDS.gateSpot },
  ],
};

const awaitingPatch = (id = 5) =>
  turn(id, 'Make the lantern visible from the close-up camera, and drop the spot light by the gate.', {
    status: 'awaiting_approval',
    text: 'The lantern sits just outside the close-up camera’s view. I’ll move it 0.6 m toward the gate, shorten its range so it stops lighting the water, add a soft fill light and remove Gate Spot.',
    approvals: [{ id: 4, tool: 'doc_patch', bytes: bytes(PATCH), decision: 'pending' }],
    pending: { turn: id, id: 4, tool: 'doc_patch', arguments: PATCH },
  });

const streaming = (id = 5) =>
  turn(id, 'Check that the lantern is visible from the close-up camera.', {
    status: 'running',
    text: 'I’ll capture the close-up camera’s view and check whether the lantern falls inside its frustum. The camera looks along −Z from',
  });

const session = (turns, { worker = false, used, max = 200000, uncertain, version = 12 } = {}) => ({
  version,
  budget: { max_tokens: max, used_tokens: used ?? turns.reduce((sum, item) => sum + item.charged_tokens, 0) },
  accounting_uncertain: uncertain ?? turns.some((item) => item.accounting_uncertain),
  worker_running: worker,
  turns,
});

/* ---- Long content ---- */

const longPrompt = [
  'Tidy the harbor scene before the playtest:',
  ...Array.from({ length: 14 }, (_, n) => `${n + 1}. Check prop group ${n + 1}, snap anything floating to the pier, and keep the existing names unless they collide with another entity in the scene.`),
  'Leave the cameras alone.',
].join('\n');
const longPath = 'assets/textures/harbor/props/very/deeply/nested/folder/structure/crate_albedo_weathered_variant_with_a_long_unbroken_name_v2_final.png';
const longReply = [
  'Here is what I found, prop group by prop group.',
  ...Array.from({ length: 10 }, (_, n) => `Group ${n + 1}: ${n % 3 === 0 ? 'two crates float 0.04 m above the pier; I snapped them down.' : n % 3 === 1 ? 'nothing to change.' : `the albedo for one crate points at ${longPath}, which is not registered, so I left it.`}`),
  'A long unbroken token, to check wrapping: ' + 'x'.repeat(180),
].join('\n\n');
const scriptSource = `// Harbor buoy bobbing behaviour (fixture source, never executed)\n${Array.from({ length: 2400 }, (_, n) => `export const wave${n} = Math.sin(${n} * 0.0125) * 0.08; // amplitude sample ${n}`).join('\n')}\n`;
const scriptWrite = {
  description: 'Replace the buoy script with a sampled wave table',
  expected_revision: 7,
  script_id: '01JA45FX00000000000000BUOY',
  expected_source: { state: 'present', sha256: 'f'.repeat(64) },
  source: scriptSource,
};

const longTurn = turn(1, longPrompt, { text: longReply, report: report(2, 6), charged_tokens: 18240, approvals: [{ id: 1, tool: 'doc_patch', bytes: 2210, decision: 'allowed' }] });
const longPending = turn(2, 'Replace the buoy script with a precomputed wave table so it costs nothing at runtime.', {
  status: 'awaiting_approval',
  text: 'A precomputed table removes the per-frame sine calls. I’ll replace the buoy script’s source.',
  approvals: [{ id: 2, tool: 'script_write', bytes: bytes(scriptWrite), decision: 'pending' }],
  pending: { turn: 2, id: 2, tool: 'script_write', arguments: scriptWrite },
});

const screenshotArgs = { camera: IDS.closeUp, width: 640, height: 360 };
const alwaysTurn = turn(1, 'Is the lantern visible from the close-up camera?', {
  request: request('Is the lantern visible from the close-up camera?', 'always'),
  status: 'awaiting_approval',
  text: 'I’ll look through the close-up camera first.',
  approvals: [{ id: 1, tool: 'view_screenshot', bytes: bytes(screenshotArgs), decision: 'pending' }],
  pending: { turn: 1, id: 1, tool: 'view_screenshot', arguments: screenshotArgs },
});

/** Sixteen held turns (the session's bound), the earliest already dropped, the last streaming. */
function manyTurns() {
  const turns = [];
  for (let id = 5; id <= 19; id += 1) {
    turns.push(
      turn(id, `Nudge prop ${id - 4} onto the pier and report its final height.`, {
        text: `Prop ${id - 4} now rests on the pier at y ${(0.98 + id / 100).toFixed(2)} m. I checked the collider overlap after the move and nothing intersects.`,
        approvals: [{ id, tool: 'doc_patch', bytes: 640, decision: 'allowed' }],
        report: report(1, 2),
        charged_tokens: 2400 + id * 10,
      }),
    );
  }
  turns.push(streaming(20));
  return turns;
}

/* ---- Scenarios ---- */

const history = [crates, navmesh, rename, lights];
const base = (sessionValue, extra = {}) => ({ engine: fixtureEngine(), provider: SIGNED_IN, session: sessionValue, ...extra });

/** Text a started turn streams, an approval, then completion: for the interactive flows. */
const approveScript = [
  { wait: 200, text: 'Reading the lantern and camera transforms. ' },
  { wait: 200, text: 'The lantern sits just outside the close-up view; ' },
  { wait: 200, text: 'I’ll move it and remove Gate Spot.' },
  { wait: 250, approval: { tool: 'doc_patch', arguments: PATCH } },
  { wait: 250, text: '\n\nDone. The lantern is now inside the close-up camera’s frustum.' },
  { wait: 300, complete: { charged: 7310, edits: 1 } },
];
const streamScript = Array.from({ length: 200 }, (_, n) => ({ wait: 220, text: `Checking prop ${n + 1}… ` }));

export const SCENARIOS = {
  'ready-empty': base(session([], { version: 0 })),
  'send-fails': base(session([], { version: 0 }), { startError: { code: 'agent.provider', message: 'The selected OpenAI account is unavailable.' } }),
  streaming: base(session([crates, streaming()], { worker: true })),
  approval: base(session([crates, awaitingPatch()], { worker: true })),
  'approval-signed-out': base(session([crates, awaitingPatch()], { worker: true }), { provider: SIGNED_OUT }),
  'approval-slow': base(session([crates, awaitingPatch()], { worker: true }), { decideDelay: 60000 }),
  'approval-always': base(session([alwaysTurn], { worker: true })),
  history: base(session(history)),
  stopping: base(session([crates, turn(5, 'Check that the lantern is visible from the close-up camera.', { status: 'stopping', text: streaming().text })], { worker: true })),
  finishing: base(session([crates, turn(5, 'Check that the lantern is visible from the close-up camera.', { text: 'The lantern is inside the close-up camera’s frustum, 2.1 m from its near plane. No edits were needed.', report: report(0, 2), charged_tokens: 3120 })], { worker: true })),
  exhausted: base(session(history, { used: 200000 })),
  'signed-out': base(session(history), { provider: SIGNED_OUT }),
  long: base(session([longTurn, longPending], { worker: true })),
  many: base(session(manyTurns(), { worker: true }), { script: [] }),
  // Interactive flows.
  'flow-send': base(session([], { version: 0 }), { script: approveScript }),
  'flow-stop': base(session([crates]), { script: streamScript, stopDelay: 1200, reapDelay: 900 }),
  'flow-many': base(session(manyTurns(), { worker: true })),
};
