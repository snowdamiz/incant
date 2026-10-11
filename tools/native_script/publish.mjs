import fs from 'node:fs/promises';
import path from 'node:path';

const same = (a, b) => a.dev === b.dev && a.ino === b.ino;

/** Create-only output. On failure remove only this invocation's own objects.
 * Consumers use the directory only after this call/CLI succeeds. The manifest
 * is last; a failed write never leaves a valid complete manifest behind. */
export async function publishArtifact(destination, artifact, io = fs) {
  await io.mkdir(destination, { mode: 0o700 });
  const directory = await io.lstat(destination);
  const created = [];
  try {
    for (const [name, data] of [['behavior.cjs', artifact.code], ['behavior.cjs.map', artifact.map],
      ['manifest.json', `${JSON.stringify(artifact.manifest, null, 2)}\n`]]) {
      const file = path.join(destination, name);
      const handle = await io.open(file, 'wx', 0o600);
      try {
        created.push({ file, identity: await handle.stat() });
        await handle.writeFile(data);
      } finally { await handle.close(); }
    }
  } catch (error) {
    const failures = [];
    // Never recursively delete this directory: another actor may have added
    // unrelated content. Identity checks also preserve replaced files/directories.
    for (const { file, identity } of created.reverse()) {
      try {
        if (same(directory, await io.lstat(destination)) && same(identity, await io.lstat(file)))
          await io.unlink(file);
      } catch (cleanup) { if (cleanup.code !== 'ENOENT') failures.push(cleanup); }
    }
    try {
      if (same(directory, await io.lstat(destination))) await io.rmdir(destination);
    } catch (cleanup) { if (cleanup.code !== 'ENOENT') failures.push(cleanup); }
    if (failures.length) throw new Error(`${error.message}; incomplete cleanup: ${failures.map(e => e.code ?? e.message).join(', ')}`, { cause: error });
    throw error;
  }
}
