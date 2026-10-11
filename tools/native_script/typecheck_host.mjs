import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';
import { fail, SDK_IMPORT } from './graph.mjs';

/** Only snapshotted modules, fixed SDK text and pinned TypeScript libraries are
 * visible. TypeScript uses forward slashes even on Windows; normalize callback
 * lookups to the same native spelling and case policy as the captured keys.
 * The optional adapters are internal test dependencies, never project options. */
export function snapshotCompilerHost(options, snapshot, io = fs, paths = path) {
  const host = ts.createCompilerHost(options, true);
  const key = file => host.getCanonicalFileName(paths.normalize(file));
  const texts = new Map();
  for (const [file, text] of snapshot.texts) {
    const name = key(file);
    if (texts.has(name)) fail('ambiguous compiler snapshot path');
    texts.set(name, text);
  }
  const records = new Map(snapshot.records.map(record => [key(record.file), record]));
  const library = key(snapshot.lib);
  const libraryFile = file => key(paths.dirname(file)) === library
    && /^lib\.[a-z0-9.]+\.d\.ts$/u.test(paths.basename(file));
  const allowed = file => texts.has(key(file)) || libraryFile(file);
  host.fileExists = file => {
    const native = paths.normalize(file);
    return allowed(native) && (texts.has(key(native)) || io.existsSync(native));
  };
  host.readFile = file => {
    const native = paths.normalize(file), name = key(native);
    if (!allowed(native)) return undefined;
    if (!texts.has(name)) texts.set(name, io.readFileSync(native, 'utf8'));
    return texts.get(name);
  };
  host.getSourceFile = (file, languageVersion) => {
    const text = host.readFile(file);
    return text === undefined ? undefined : ts.createSourceFile(file, text, languageVersion, true);
  };
  host.resolveModuleNames = (names, from) => names.map(name => {
    const containing = key(from);
    const file = name === SDK_IMPORT ? snapshot.native
      : containing === key(snapshot.virtual) ? snapshot.entryImports.get(name)
      : containing === key(snapshot.native) && name === './generated' ? snapshot.generated
      : records.get(containing)?.dependencies.get(name);
    return file ? { resolvedFileName: file, extension: file.endsWith('.d.ts') ? ts.Extension.Dts : ts.Extension.Ts } : undefined;
  });
  host.getDefaultLibLocation = () => snapshot.lib;
  host.getDefaultLibFileName = () => paths.join(snapshot.lib, ts.getDefaultLibFileName(options));
  return host;
}
