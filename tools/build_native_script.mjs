#!/usr/bin/env node
import path from 'node:path';
import { cookNativeScript } from './native_script/cook.mjs';
import { publishArtifact } from './native_script/publish.mjs';

try {
  const [root, entry, output, ...extra] = process.argv.slice(2);
  if (!root || !entry || !output || extra.length)
    throw new Error('Usage: node tools/build_native_script.mjs source-root entry.ts new-output-directory');
  const artifact = await cookNativeScript(root, entry);
  // Only a new output directory is accepted; existing assets are never replaced.
  const destination = path.resolve(output);
  await publishArtifact(destination, artifact);
  process.stdout.write(`${JSON.stringify(artifact.manifest)}\n`);
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
