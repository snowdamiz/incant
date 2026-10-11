// Cook-time only. Snapshot local text; never load project configuration or code.
import fs from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

export const LIMITS = Object.freeze({ modules: 64, fileBytes: 256 * 1024,
  sourceBytes: 1024 * 1024, bundleBytes: 1_000_000, mapBytes: 4 * 1024 * 1024 });
export const SDK_IMPORT = '@incant/runtime/native';
export const fail = message => { throw new Error(`Native TypeScript cook: ${message}`); };
export const portable = file => file.split(path.sep).join('/');

/** A catalog is data, never compiler configuration. Stable source order also
 * determines module initialization order when entries share dependencies. */
export function sourceCatalog(value) {
  if (!Array.isArray(value) || !value.length || value.length > LIMITS.modules)
    fail('source catalog must contain 1–64 entries');
  const ids = new Set(), paths = new Set();
  const entries = value.map(item => {
    if (!item || typeof item !== 'object' || Array.isArray(item)
        || Object.keys(item).sort().join(',') !== 'id,path'
        || typeof item.id !== 'string' || !/^[0-7][0-9A-HJKMNP-TV-Z]{25}$/u.test(item.id)
        || typeof item.path !== 'string' || Buffer.byteLength(item.path) > 1024
        || /[\\\x00-\x1f\x7f]/u.test(item.path) || item.path.includes(':')
        || item.path.split('/').some(part => !part || part === '.' || part === '..')
        || !item.path.endsWith('.ts') || item.path.endsWith('.d.ts'))
      fail('source catalog requires canonical ULIDs and portable relative .ts paths');
    if (ids.has(item.id)) fail('duplicate source ID in catalog');
    if (paths.has(item.path)) fail('multiple source IDs alias the same entry path');
    ids.add(item.id); paths.add(item.path);
    return { id: item.id, path: item.path };
  });
  return entries.sort((a, b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
}

function inside(root, file) {
  const relative = path.relative(root, file);
  return relative !== '' && !path.isAbsolute(relative) && relative !== '..'
    && !relative.startsWith(`..${path.sep}`);
}

async function checkedFile(root, file) {
  if (!inside(root, file)) fail('module escapes the selected source root');
  // Refuse links at every component, not only the final file. The final open is
  // no-follow on platforms that provide it; its identity is checked as well.
  let current = root;
  for (const part of path.relative(root, file).split(path.sep)) {
    current = path.join(current, part);
    let stat;
    try { stat = await fs.lstat(current); } catch (error) {
      if (error.code === 'ENOENT' || error.code === 'ENOTDIR') return null;
      throw error;
    }
    if (stat.isSymbolicLink()) fail('symbolic links in the module graph are unsupported');
    if (current === file) return stat.isFile() ? stat : null;
    if (!stat.isDirectory()) return null;
  }
  return null;
}

async function sourceFile(root, file) {
  const before = await checkedFile(root, file);
  if (!before) fail(`missing source module ${portable(path.relative(root, file))}`);
  if (before.size > LIMITS.fileBytes) fail('source module exceeds the file byte limit');
  const handle = await fs.open(file, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const opened = await handle.stat();
    if (!opened.isFile() || before.dev !== opened.dev || before.ino !== opened.ino)
      fail('source module changed during snapshot');
    const data = Buffer.alloc(LIMITS.fileBytes + 1);
    let size = 0;
    while (size < data.length) {
      const { bytesRead } = await handle.read(data, size, data.length - size, null);
      if (!bytesRead) break;
      size += bytesRead;
    }
    if (size > LIMITS.fileBytes) fail('source module exceeds the file byte limit');
    const after = await handle.stat();
    if (after.size !== size || opened.size !== size || opened.mtimeMs !== after.mtimeMs)
      fail('source module changed during snapshot');
    const canonical = await fs.realpath(file);
    if (canonical !== file) fail('source module path changed during snapshot');
    const bytes = data.subarray(0, size);
    return { text: new TextDecoder('utf-8', { fatal: true }).decode(bytes), bytes };
  } finally { await handle.close(); }
}

function imports(source, label) {
  if (source.referencedFiles.length || source.typeReferenceDirectives.length
      || source.libReferenceDirectives.length || source.hasNoDefaultLib)
    fail(`${label}: reference directives are unsupported`);
  const found = new Map();
  const add = (literal, typeOnly) => {
    if (!literal || !ts.isStringLiteral(literal)) fail(`${label}: static module string required`);
    const name = literal.text;
    if (/[\\\x00-\x1f\x7f?#]/u.test(name)) fail(`${label}: unsupported module path`);
    if (name === SDK_IMPORT) {
      if (!typeOnly) fail(`${label}: native SDK imports must use import type or export type`);
    } else if (!name.startsWith('./') && !name.startsWith('../')) {
      fail(`${label}: external modules are unsupported`);
    }
    found.set(name, (found.get(name) ?? true) && typeOnly);
  };
  const visit = node => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) {
      if (node.attributes) fail(`${label}: import attributes are unsupported`);
      if (node.moduleSpecifier) add(node.moduleSpecifier,
        ts.isImportDeclaration(node) ? node.importClause?.isTypeOnly === true : node.isTypeOnly);
    }
    if (ts.isImportTypeNode(node)) {
      if (!ts.isLiteralTypeNode(node.argument)) fail(`${label}: static type import required`);
      add(node.argument.literal, true);
    }
    if (ts.isImportEqualsDeclaration(node) || ts.isExportAssignment(node) && node.isExportEquals)
      fail(`${label}: CommonJS declarations are unsupported; use ES module syntax`);
    if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword)
      fail(`${label}: dynamic imports are unsupported`);
    if (ts.isMetaProperty(node) && node.keywordToken === ts.SyntaxKind.ImportKeyword)
      fail(`${label}: import.meta is unsupported`);
    if (ts.isDecorator(node)) fail(`${label}: decorators are unsupported`);
    if (ts.isModuleDeclaration(node) && ts.isStringLiteral(node.name))
      fail(`${label}: ambient external modules are unsupported`);
    if (ts.isIdentifier(node) && ['require', 'exports', 'module'].includes(node.text))
      fail(`${label}: ${node.text} is reserved by the module cook`);
    ts.forEachChild(node, visit);
  };
  visit(source);
  return found;
}

export async function snapshotGraph(rootArgument, entryArgument) {
  return snapshot(rootArgument, [{ path: entryArgument }]);
}

/** All entries share one captured graph and aggregate source/module quotas. */
export async function snapshotProjectGraph(rootArgument, catalog) {
  return snapshot(rootArgument, sourceCatalog(catalog));
}

async function snapshot(rootArgument, catalog) {
  const root = await fs.realpath(path.resolve(rootArgument));
  if (!(await fs.stat(root)).isDirectory()) fail('source root must be a directory');
  const entries = catalog.map(item => ({ ...item, file: path.resolve(root, item.path) }));
  const modules = new Map();
  let sourceBytes = 0;
  const resolveImport = async (from, name) => {
    const stem = path.resolve(path.dirname(from), name);
    if (!inside(root, stem)) fail('module escapes the selected source root');
    const extension = path.extname(stem);
    if (extension && extension !== '.ts') fail('only TypeScript source modules are supported');
    const candidates = extension ? [stem] : [`${stem}.ts`, path.join(stem, 'index.ts')];
    const matches = [];
    for (const candidate of candidates) if (await checkedFile(root, candidate)) matches.push(candidate);
    if (matches.length !== 1) fail(`${name}: missing or ambiguous local module`);
    return matches[0];
  };
  const visit = async file => {
    if (modules.has(file)) return;
    if (modules.size >= LIMITS.modules) fail('module count limit exceeded');
    if (!file.endsWith('.ts') || file.endsWith('.d.ts')) fail('project modules must be .ts source, not declarations');
    const captured = await sourceFile(root, file);
    sourceBytes += captured.bytes.length;
    if (sourceBytes > LIMITS.sourceBytes) fail('aggregate source byte limit exceeded');
    const label = portable(path.relative(root, file));
    const source = ts.createSourceFile(file, captured.text, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TS);
    if (source.parseDiagnostics.length) fail(`${label}: invalid TypeScript syntax`);
    const record = { ...captured, file, label, source, dependencies: new Map() };
    modules.set(file, record);
    for (const [name] of imports(source, label)) {
      if (name === SDK_IMPORT) continue;
      const dependency = await resolveImport(file, name);
      record.dependencies.set(name, dependency);
      await visit(dependency);
    }
  };
  for (const entry of entries) await visit(entry.file);
  return { root, entry: entries[0].file, entries, sourceBytes, modules: [...modules.values()].sort((a, b) =>
    a.label < b.label ? -1 : a.label > b.label ? 1 : 0) };
}
