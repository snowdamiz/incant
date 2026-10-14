#!/usr/bin/env node
import path from 'node:path';
import { publishArtifact } from './native_script/publish.mjs';
import { phaseRecorder } from './native_script/phases.mjs';

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

const phases = phaseRecorder(process.env.INCANT_NATIVE_COOK_PHASE_FILE);
try {
  phases.mark('driver_entry');
  // Keep dependency loading observable, including the pinned SWC native carrier.
  const { cookNativeScript, cookNativeProjectScripts } = await import('./native_script/cook.mjs');
  phases.mark('compiler_ready');
  const [first, second, output, ...extra] = process.argv.slice(2);
  if (!first || !second || !output || extra.length)
    throw new Error('Usage: node tools/build_native_script.mjs source-root entry.ts new-output-directory; or --project source-root new-output-directory with a source catalog on stdin');
  const catalog = first === '--project' ? await readCatalog() : null;
  phases.mark('input_ready');
  const artifact = first === '--project'
    ? await cookNativeProjectScripts(second, catalog, phases.mark)
    : await cookNativeScript(first, second, phases.mark);
  phases.mark('artifact_ready');
  // Only a new output directory is accepted; existing assets are never replaced.
  const destination = path.resolve(output);
  await publishArtifact(destination, artifact);
  phases.mark('publish_done');
  process.stdout.write(`${JSON.stringify(artifact.manifest)}\n`);
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
} finally {
  phases.close();
}
