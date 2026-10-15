import fs from 'node:fs';

// Fixed compiler milestones only: never source text, filenames, or diagnostics.
export const PHASES = Object.freeze(['driver_entry', 'typescript_ready', 'swc_ready', 'compiler_ready', 'input_ready',
  'snapshot_ready', 'typecheck_done', 'transform_done', 'artifact_ready', 'publish_done']);

export function phaseRecorder(file) {
  if (!file) return { mark() {}, close() {} };
  const started = process.hrtime.bigint();
  const fd = fs.openSync(file, 'a');
  let next = 0;
  return {
    mark(phase) {
      if (next >= PHASES.length || phase !== PHASES[next])
        throw new Error('invalid native compiler phase order');
      const elapsed_us = Number((process.hrtime.bigint() - started) / 1000n);
      fs.writeSync(fd, `${JSON.stringify({ phase, elapsed_us })}\n`);
      next++;
    },
    close() { fs.closeSync(fd); },
  };
}
