// transpile.mjs IN.ts OUT.js: type-strip the unmodified gameplay controller to CommonJS for the
// offscreen harness guest (QuickJS CompiledScript). Uses the repository's pinned TypeScript only;
// no bundling, no project configuration. The shipped package uses the cook's own bytecode.
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const ts = require('typescript');
const [input, output] = process.argv.slice(2);
const source = readFileSync(input, 'utf8');
const result = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, removeComments: false },
  fileName: 'gameplay.ts', reportDiagnostics: true,
});
if (result.diagnostics && result.diagnostics.length) { console.error(JSON.stringify(result.diagnostics)); process.exit(1); }
writeFileSync(output, result.outputText, { flag: 'wx' });
console.log(JSON.stringify({ input, output, typescript: ts.version, bytes: result.outputText.length }));
