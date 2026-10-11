/**
 * NATIVE-BRIDGE DOUBLE for the handoff 0044 browser review. Not engine data, not native
 * Play, not a native window.
 *
 * Builds a page init script that installs a simulated Tauri IPC (`window.__TAURI__`) and
 * then runs the REAL compiled native bridge, editor/bridge/native.generated.js, on top
 * of it. The bridge polls play_read at 4 Hz and sends play_command exactly as it does in
 * the editor; the double behind it is editor/ui/src/test/nativePlayDouble.ts (the same
 * code the UI tests use), transpiled here with the workspace's pinned TypeScript. It
 * mirrors the editor worker's rules and messages, never renders, and advances ticks only
 * when the review calls `window.__playDouble.frame()`.
 *
 * The project's component schemas are the repository's generated schemas/*.schema.json,
 * so the Inspector presents authored values through the real snapshotFromEngine.
 */
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
export const repoRoot = resolve(here, '../../..');
const require = createRequire(join(repoRoot, 'package.json'));
const ts = require('typescript');

/** Component schemas the double's project uses, as the native host would register them. */
const COMPONENTS = ['Transform', 'Camera', 'RigidBody', 'Collider', 'Velocity', 'PointLight'];

export function schemas() {
  return Object.fromEntries(
    COMPONENTS.map((name) => [name, JSON.parse(readFileSync(join(repoRoot, 'schemas', `${name}.schema.json`), 'utf8'))]),
  );
}

/** The shared double as a script that defines `window.__nativePlayDoubleModule`. */
export function doubleSource() {
  const source = readFileSync(join(repoRoot, 'editor/ui/src/test/nativePlayDouble.ts'), 'utf8');
  const { outputText, diagnostics } = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS, verbatimModuleSyntax: false },
    reportDiagnostics: true,
  });
  if (diagnostics?.length) throw new Error(`Transpiling the double failed: ${diagnostics.map((d) => d.messageText).join('; ')}`);
  if (/\brequire\(/.test(outputText)) throw new Error('The double must not have runtime imports.');
  return `(() => { const exports = {}; const module = { exports }; ${outputText}\n window.__nativePlayDoubleModule = exports; })();`;
}

export function bridgeSource() {
  return readFileSync(join(repoRoot, 'editor/bridge/native.generated.js'), 'utf8');
}

/**
 * Init script: the double as `window.__TAURI__` (variant `harbor` or `single`), then the
 * real native bridge. `saved: false` makes Play refuse as for an unsaved project; `drop`
 * removes advertised capabilities, as an older host would lack them.
 */
export function initScript({ variant = 'harbor', saved = true, holdStart = true, drop = [] } = {}) {
  const config = JSON.stringify({ variant, saved, holdStart, schemas: schemas() });
  return `${doubleSource()}
(() => {
  const config = ${config};
  const module = window.__nativePlayDoubleModule;
  const double = module.createNativePlayDouble({ read: module.doubleRead(config.variant, config.schemas), saved: config.saved, holdStart: config.holdStart });
  window.__playDouble = double;
  window.__playDoubleIds = module.IDS;
  window.__TAURI__ = { core: { invoke: double.invoke }, event: { listen: double.listen } };
})();
${bridgeSource()}
// The real bridge calls itself "Native engine"; every capture must say what it is.
window.__INCANT_BRIDGE__.label = '${DOUBLE_LABEL}';
window.__INCANT_BRIDGE__.capabilities = window.__INCANT_BRIDGE__.capabilities.filter((c) => !${JSON.stringify(drop)}.includes(c));`;
}

export const DOUBLE_LABEL = 'Native-bridge double (simulated Play, not engine)';
