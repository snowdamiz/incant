#!/usr/bin/env node
import path from 'node:path';
import { cookNativeScript, cookNativeProjectScripts } from './native_script/cook.mjs';
import { publishArtifact } from './native_script/publish.mjs';

async function readCatalog() {
  const chunks = [];
  let size = 0;
  for await (const chunk of process.stdin) {
    size += chunk.length;
    if (size > 96 * 1024) throw new Error('Native source catalog exceeds 96 KiB');
    chunks.push(chunk);
  }
  return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks)));
}

try {
  const [first, second, output, ...extra] = process.argv.slice(2);
  if (!first || !second || !output || extra.length)
    throw new Error('Usage: node tools/build_native_script.mjs source-root entry.ts new-output-directory; or --project source-root new-output-directory with a source catalog on stdin');
  const artifact = first === '--project'
    ? await cookNativeProjectScripts(second, await readCatalog())
    : await cookNativeScript(first, second);
  // Only a new output directory is accepted; existing assets are never replaced.
  const destination = path.resolve(output);
  await publishArtifact(destination, artifact);
  process.stdout.write(`${JSON.stringify(artifact.manifest)}\n`);
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
