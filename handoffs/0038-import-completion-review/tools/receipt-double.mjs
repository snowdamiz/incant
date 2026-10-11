/**
 * EVIDENCE TEST DOUBLE for handoff 0038 (import completion review). Not engine data, and
 * not native command execution.
 *
 * The project is the 0035 union double's (`unionRead`, itself the 0033 project), converted
 * by the same real compiled bridge function `snapshotFromEngine`
 * (editor/bridge/native.generated.js). New here:
 *   - an existing edit history in the native read shape ({ id, description, actor }) with
 *     ULID time prefixes, so the real conversion maps origins and times. Descriptions are
 *     the strings the native host writes: "Import assets" (editor/app/src/assets.rs),
 *     "Rename entity to …" and "Delete entity" (editor/bridge/native.ts), and source
 *     watch's "Reimport changed sources" (crates/incant_import/src/watch.rs). The last edit
 *     is undone, so there is a redo tail.
 *   - a live bridge. Every asset.import is HELD until the review calls
 *     `window.__incantReview.release(outcome)`, so the pending state can be observed.
 *     Outcomes are MOCKED host results:
 *       unchanged  publishes the unchanged history, then resolves
 *                  { ok: true, assetImport: { revision, transaction: null } };
 *       changed    appends "Import assets" (clearing the redo tail, as incant_cmd does),
 *                  publishes, then resolves the receipt with that transaction;
 *       missing    behaves like a host without receipts: it may append, then resolves
 *                  { ok: true } with no assetImport;
 *       failure    resolves { ok: false, error } with a message copied from Rust.
 *     `concurrent` adds another transaction after the commit and before the read, which
 *     the receipt must not be inferred from. As in NativeBridge.dispatch, the snapshot is
 *     published before the result resolves.
 *   - history.undo/redo move the cursor, removing or restoring the assets a batch added;
 *     entity.rename renames the entity and appends
 *     "Rename entity to <name>", clearing the redo tail. Also MOCKED; native journal,
 *     Undo and Redo semantics are covered by the Rust tests named in
 *     docs/spikes/editor-import-receipts.md.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import { unionRead } from '../../0035-editor-union-review/tools/union-double.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '../../..');
const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/** A ULID whose time prefix is `iso` (as the engine's are) and whose suffix is `serial`. */
export function ulidAt(iso, serial) {
  let time = Date.parse(iso);
  let prefix = '';
  for (let i = 0; i < 10; i += 1) {
    prefix = CROCKFORD.charAt(time % 32) + prefix;
    time = Math.floor(time / 32);
  }
  let suffix = '';
  let rest = serial;
  for (let i = 0; i < 16; i += 1) {
    suffix = CROCKFORD.charAt(rest % 32) + suffix;
    rest = Math.floor(rest / 32);
  }
  return prefix + suffix;
}

const USER = { origin: 'user', actor: 'editor', model: null, conversation_id: null };
const SOURCE_WATCH = { origin: 'import', actor: 'source-watch', model: null, conversation_id: null };

/** Native read-shaped history, oldest first. Entries past APPLIED are the redo tail. */
const HISTORY = [
  ['2026-10-10T13:02:11.000Z', 'Import assets', USER],
  ['2026-10-10T13:09:47.000Z', 'Rename entity to Crate Stack', USER],
  ['2026-10-10T13:41:05.000Z', 'Reimport changed sources', SOURCE_WATCH],
  ['2026-10-10T14:16:32.000Z', 'Delete entity', USER],
  ['2026-10-10T14:22:58.000Z', 'Rename entity to Lantern', USER],
  ['2026-10-10T14:23:40.000Z', 'Rename entity to Gate Spot', USER],
].map(([at, description, actor], index) => ({ id: ulidAt(at, 7000 + index), description, actor }));
const APPLIED = 5;
const REVISION = 41;

/** A UI history entry, shaped as snapshotFromEngine maps a native one. */
export function entry(iso, serial, description, origin = { kind: 'user' }) {
  return { transaction: ulidAt(iso, serial), description, origin, at: new Date(Date.parse(iso)).toISOString() };
}

function loadSnapshotFromEngine() {
  const source = readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
  // Same capture as the 0033 and 0035 doubles: the IIFE's exports, without installing a bridge.
  const wrapped = source.replace(/\}\)\(\{\}\);\s*$/, '})(globalThis.__exports = {});');
  if (wrapped === source) throw new Error('native.generated.js has an unexpected shape');
  const sandbox = { window: {}, console, globalThis: undefined };
  sandbox.globalThis = sandbox;
  vm.runInNewContext(wrapped, sandbox, { filename: 'native.generated.js' });
  const fn = sandbox.__exports?.snapshotFromEngine;
  if (typeof fn !== 'function') throw new Error('snapshotFromEngine was not exported');
  return fn;
}

/** The converted snapshot: the union project with an existing history and import available. */
export function receiptSnapshot() {
  const { read, diagnostics } = unionRead();
  read.history = HISTORY;
  read.applied = APPLIED;
  read.can_redo = APPLIED < HISTORY.length;
  read.revision = REVISION;
  read.asset_import = { available: true };
  const converted = loadSnapshotFromEngine()(read);
  return {
    ...converted,
    diagnostics: [...converted.diagnostics, ...diagnostics],
    viewport: { status: 'not-attached', reason: 'Evidence test double: a browser page has no engine surface.' },
  };
}

export const CAPABILITIES = ['asset.import', 'history.undo', 'history.redo', 'entity.rename'];

/** Init script installing the live double as window.__INCANT_BRIDGE__. */
export function receiptInitScript() {
  const snapshot = receiptSnapshot();
  return `
    (() => {
      let snapshot = ${JSON.stringify(snapshot)};
      let revision = ${REVISION};
      const listeners = new Set();
      const refused = { ok: false, error: { code: 'test-double', message: 'Evidence test double: read-only.' } };
      const review = window.__incantReview = { commands: [], results: [], held: [] };
      // Assets each committed import added, by transaction, so Undo and Redo can revert them.
      const added = new Map();
      const publish = (patch) => {
        snapshot = { ...snapshot, ...patch };
        listeners.forEach((listener) => listener());
      };
      review.publish = publish;
      review.snapshot = () => snapshot;
      // A committed transaction drops the redo tail and becomes the applied head.
      const appended = (history, item) => ({ entries: [...history.entries.slice(0, history.applied), item], applied: history.applied + 1 });
      review.release = (outcome) => {
        const settle = review.held.shift();
        if (!settle) throw new Error('No import is waiting.');
        settle(outcome);
      };
      window.__INCANT_BRIDGE__ = {
        protocolVersion: 1, capabilities: ${JSON.stringify(CAPABILITIES)}, label: 'Evidence test double (not engine)', isFixture: false,
        getSnapshot: () => snapshot,
        subscribe: (listener) => { listeners.add(listener); return () => listeners.delete(listener); },
        dispatch: (command) => {
          review.commands.push(command);
          if (command.type === 'asset.import') {
            return new Promise((resolve) => review.held.push((outcome) => {
              let result;
              if (outcome.kind === 'failure') {
                result = { ok: false, error: outcome.error };
              } else {
                let history = snapshot.history;
                let assets = snapshot.assets;
                if (outcome.entry) {
                  history = appended(history, outcome.entry);
                  revision += 1;
                  if (outcome.assets) added.set(outcome.entry.transaction, outcome.assets);
                }
                if (outcome.assets) assets = { status: 'ready', value: [...assets.value, ...outcome.assets] };
                const commit = { revision, transaction: outcome.kind === 'changed' ? outcome.entry.transaction : null };
                if (outcome.concurrent) {
                  history = appended(history, outcome.concurrent);
                  revision += 1;
                }
                publish({ history, assets });
                result = outcome.kind === 'missing' ? { ok: true } : { ok: true, assetImport: commit };
              }
              review.results.push(result);
              resolve(result);
            }));
          }
          if (command.type === 'history.undo' || command.type === 'history.redo') {
            const { entries, applied } = snapshot.history;
            const redo = command.type === 'history.redo';
            const next = applied + (redo ? 1 : -1);
            if (next < 0 || next > entries.length) return Promise.resolve(refused);
            revision += 1;
            // Undoing a batch that added assets removes them again; redo restores them.
            const batch = added.get(entries[redo ? applied : applied - 1].transaction);
            const ids = new Set((batch ?? []).map((asset) => asset.id));
            const assets = !batch ? snapshot.assets : redo
              ? { status: 'ready', value: [...snapshot.assets.value, ...batch] }
              : { status: 'ready', value: snapshot.assets.value.filter((asset) => !ids.has(asset.id)) };
            publish({ history: { entries, applied: next }, assets });
            return Promise.resolve({ ok: true });
          }
          if (command.type === 'entity.rename') {
            const node = snapshot.hierarchy.value.nodes[command.entity];
            if (!node) return Promise.resolve(refused);
            revision += 1;
            const nodes = { ...snapshot.hierarchy.value.nodes, [command.entity]: { ...node, name: command.name } };
            const detail = snapshot.entities[command.entity];
            publish({
              hierarchy: { status: 'ready', value: { ...snapshot.hierarchy.value, nodes } },
              entities: detail ? { ...snapshot.entities, [command.entity]: { ...detail, name: command.name } } : snapshot.entities,
              history: appended(snapshot.history, review.nextRename(command.name)),
            });
            return Promise.resolve({ ok: true });
          }
          return Promise.resolve(refused);
        },
        request: () => Promise.resolve(refused),
      };
      // The rename's history entry, at a fixed time so screenshots are deterministic.
      review.nextRename = (name) => ({ ...${JSON.stringify(entry('2026-10-10T14:33:02.000Z', 7100, ''))}, description: 'Rename entity to ' + name });
    })();
  `;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const snapshot = receiptSnapshot();
  const { entries, applied } = snapshot.history;
  entries.forEach((item, index) => console.log(`${index < applied ? 'applied' : 'redo   '}  ${item.at}  ${item.origin.kind.padEnd(6)}  ${item.transaction}  ${item.description}`));
  console.log(`assets: ${snapshot.assets.value.map((a) => `${a.name} (${a.path})`).join(', ')}`);
  console.log(`assetImport: ${JSON.stringify(snapshot.assetImport)}`);
}
