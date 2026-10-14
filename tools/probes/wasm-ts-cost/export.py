#!/usr/bin/env python3
"""Fresh ordinary shipping exports. Invoke only with an assigned CPU build slot."""
import argparse
import contextlib
import json
from pathlib import Path
import time

from cost_workload import ROOT, prepared, require, write_json, file_hash, run
import export_native


def export_pair(inputs, output, wabt):
    inputs, output = inputs.absolute(), output.absolute()
    before = prepared(inputs)
    require(not output.exists() and output.parent.is_dir(), 'Choose new package output')
    output.mkdir()
    require(not run(['git', 'status', '--porcelain'], output / 'source-clean', cwd=ROOT),
            'Commit/freeze all source before shipping experiments')
    engine = run(['git', 'rev-parse', 'HEAD'], output / 'source-head', cwd=ROOT).decode().strip()
    lock = file_hash(ROOT / 'Cargo.lock')
    results = {}
    # In-process exporter retains ownership of its managed build/tool sessions;
    # no outer timeout can abandon its separately owned process groups.
    for kind in ('typescript', 'wasm'):
        receipt = {'entrypoint': 'export_native.export', 'kind': kind, 'started_ns': time.time_ns(), 'exit': None}
        try:
            with (output / f'{kind}.stdout').open('x') as out, (output / f'{kind}.stderr').open('x') as err, \
                    contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                results[kind] = export_native.export(inputs / kind / 'project.json', output / kind,
                                                    wabt=wabt.absolute(), profiling='summary')
            require(prepared(inputs) == before, 'Workload changed during shipping export')
            require(not run(['git', 'status', '--porcelain'], output / f'{kind}-source-clean', cwd=ROOT),
                    'Source changed during export')
            require(run(['git', 'rev-parse', 'HEAD'], output / f'{kind}-source-head', cwd=ROOT).decode().strip() == engine
                    and file_hash(ROOT / 'Cargo.lock') == lock, 'Engine/lock changed between exports')
            receipt['exit'] = 0
        except BaseException as error:
            receipt['error'] = repr(error)
            raise
        finally:
            receipt['ended_ns'] = time.time_ns()
            receipt['logs'] = {suffix: file_hash(output / f'{kind}.{suffix}', 128 * 1024**2)
                               for suffix in ('stdout', 'stderr')}
            write_json(output / f'{kind}.command.json', receipt)
    result = {'format': 'incant-wasm-ts-exports', 'version': 1,
              'engine_commit': engine, 'cargo_lock_sha256': lock,
              'prepared_sha256': file_hash(inputs / 'prepared.json'),
              'project_sha256': {kind: before['files'][f'{kind}/project.json'] for kind in results},
              'export_sha256': {kind: file_hash(output / kind / 'export.json') for kind in results}}
    write_json(output / 'pair.json', result)
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('inputs', type=Path); parser.add_argument('output', type=Path)
    parser.add_argument('--wabt', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(export_pair(args.inputs, args.output, args.wabt), indent=2))
