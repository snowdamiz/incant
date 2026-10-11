#!/usr/bin/env node
/**
 * Mutation checks for handoff 0049. Each mutation changes one part of the final source,
 * runs the approval readiness tests, records which of them fail, and restores the file;
 * the restored bytes are compared with the original before the next mutation.
 *
 * Usage (worktree root): node handoffs/0049-agent-approval-readiness/tools/mutations.mjs [out.json]
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const uiRoot = join(repoRoot, 'editor/ui');
const TESTS = 'src/components/agent/AgentApprovalReadiness.test.tsx';
const PANEL = join(uiRoot, 'src/components/AgentPanel.tsx');
const TURN = join(uiRoot, 'src/components/agent/AgentTurn.tsx');
const UPDATING = "'Waiting for the agent to update its session. You can still stop the agent.'";

/** The pinned vitest of this checkout or, for a handoff worktree, of the checkout above it. */
function findVitest() {
  for (let dir = uiRoot; dir !== dirname(dir); dir = dirname(dir)) {
    const bin = join(dir, 'node_modules/.bin/vitest');
    if (existsSync(bin)) return bin;
  }
  throw new Error('No node_modules/.bin/vitest above editor/ui.');
}

const MUTATIONS = [
  {
    name: 'no hold: the decision chain without approvalReady (the uncorrected behaviour)',
    file: PANEL,
    from: `            : approvalHeld\n              ? ${UPDATING}\n              : null;`,
    to: '            : null;',
  },
  {
    name: 'hold first: approvalReady before every other reason',
    file: PANEL,
    from: "  const decisionBlocked = !capabilities.has('agent.approve')",
    to: `  const decisionBlocked = approvalHeld ? ${UPDATING} : !capabilities.has('agent.approve')`,
  },
  {
    name: 'hold before the allowance reason',
    file: PANEL,
    from: '          : savingAllowance || allowancePending\n',
    to: `          : approvalHeld ? ${UPDATING} : savingAllowance || allowancePending\n`,
  },
  {
    name: 'omission holds: an older host without the field treated as not ready',
    file: PANEL,
    from: "agent.approvalReady === false",
    to: 'agent.approvalReady !== true',
  },
  {
    name: 'buttons ignore the hold: activation no longer checks it (marking unchanged)',
    file: TURN,
    from: '    if (unavailable) return;\n',
    to: '',
  },
];

const vitest = findVitest();
const scratch = mkdtempSync(join(tmpdir(), 'incant-0049-mutations-'));
const results = [];
for (const [index, mutation] of MUTATIONS.entries()) {
  const original = readFileSync(mutation.file);
  const text = original.toString('utf8');
  if (text.split(mutation.from).length !== 2) throw new Error(`${mutation.name}: the target text must occur exactly once`);
  const report = join(scratch, `mutation-${index + 1}.json`);
  try {
    writeFileSync(mutation.file, text.replace(mutation.from, mutation.to));
    spawnSync(vitest, ['run', '--maxWorkers=4', '--reporter=json', `--outputFile=${report}`, TESTS], { cwd: uiRoot, stdio: 'ignore' });
  } finally {
    writeFileSync(mutation.file, original);
  }
  if (!readFileSync(mutation.file).equals(original)) throw new Error(`${mutation.name}: the file was not restored byte for byte`);
  const json = JSON.parse(readFileSync(report, 'utf8'));
  const tests = json.testResults.flatMap((file) => file.assertionResults);
  const failed = tests.filter((test) => test.status === 'failed').map((test) => test.fullName);
  results.push({ mutation: mutation.name, file: relative(repoRoot, mutation.file), tests: tests.length, failed: failed.length, failedTests: failed, restored: true });
  console.log(`${failed.length ? 'CAUGHT' : 'MISSED'}  ${failed.length}/${tests.length} failed  ${mutation.name}`);
}
const out = process.argv[2];
if (out) writeFileSync(out, `${JSON.stringify({ tests: TESTS, results }, null, 2)}\n`);
if (results.some((result) => result.failed === 0)) process.exitCode = 1;
