#!/usr/bin/env python3
"""Replay retained data only; never builds or executes a renderer."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess


HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', metavar='COMMIT')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    archive = (HERE / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['records_sha256'], 'Archive identity changed')
    require(file_digest(__file__) == manifest['verifier_sha256'], 'Verifier identity changed')
    records = json.loads(gzip.decompress(archive))
    require(len(records) == manifest['record_count'], 'Record count changed')
    data = {}
    for row in records:
        require(row['name'] not in data, 'Duplicate record')
        raw = base64.b64decode(row['data'], validate=True)
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Record bytes changed')
        data[row['name']] = raw
        if args.originals:
            require(file_digest(row['original']) == row['sha256'], 'Original changed: ' + row['name'])

    def record(name):
        return json.loads(data[name])

    for attempt in range(1, 7):
        prefix = f'cpu-{attempt:03}'
        result = record(prefix + '/results.json')
        closeout = record(f'cpu{attempt:03}-closeout.json')
        expected = 'complete' if attempt == 6 else 'failed'
        require(result['status'] == expected and result['supervisor_terminal'], 'Attempt outcome changed')
        require(closeout['status'] == expected and closeout['independent_ps_all_absent'], 'Cleanup receipt missing')
        require(all(group['cleanup_confirmed'] for group in result['owned_groups']), 'Unconfirmed cleanup')
        require(digest(data[prefix + '/results.json']) == closeout['results_sha256'], 'Closeout result differs')
        require(result['native_app_player_GPU_window_capture_executions'] == 0, 'Unexpected native execution')
        for name, log in result['logs'].items():
            raw = data[prefix + '/' + name]
            require(len(raw) == log['bytes'] and digest(raw) == log['sha256'], 'Log changed')

    historical = record('cpu-003/results.json')
    require(historical['source'] == manifest['mac_test_source'], 'Historical test source changed')
    require([row['exit'] for row in historical['results']] == [0, 0, 0, 0, 0, 101], 'Historical checks changed')
    for index in (0, 2):
        require(re.findall(rb'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',
                           data[f'cpu-003/{index}.stdout.log']) == [b'23'], 'Mac test count changed')
    final = record('cpu-006/results.json')
    require(final['source'] == manifest['windows_compile_source'], 'Final compile source changed')
    require([row['exit'] for row in final['results']] == [0, 0], 'Final cross-check changed')
    require(all('--target' in row['argv'] and 'x86_64-pc-windows-gnu' in row['argv']
                and '-D' in row['argv'] and 'warnings' in row['argv']
                for row in final['results']), 'Windows compile contract changed')
    require('--all-targets' in final['results'][1]['argv']
            and 'dx12-pipeline-library,cooked-native-shaders' in final['results'][1]['argv'],
            'Full cooked all-target cross-check missing')
    require(record('cpu006-closeout.json')['actual_tests_executed'] == 0, 'Cross-check relabeled native tests')
    for name in ('windows_native_execution', 'driver_cache_hit', 'native_pixels', 'performance', 'shader_stall_gate'):
        require(manifest['claims'][name] is False, 'Unqualified claim enabled')
    for row in manifest['external_metadata']:
        if args.originals:
            require(file_digest(row['path']) == row['sha256'], 'External metadata changed')
    if args.binaries:
        for row in manifest['binaries']:
            require(Path(row['copy']).stat().st_size == row['bytes']
                    and file_digest(row['copy']) == row['sha256'], 'Historical Mac binary changed')

    freeze = record('source-005.json')
    require(freeze['source'] == manifest['windows_compile_source'], 'Source inventory differs')
    if args.source_root:
        for row in freeze['files']:
            require(file_digest(args.source_root / row['path']) == row['sha256'], 'Live source changed: ' + row['path'])
    if args.git_source:
        repo = args.source_root or HERE.parents[3]
        # Batch reads actual Git bytes. No checkout, compiler or native tool runs.
        specs = ''.join(f"{args.git_source}:{row['path']}\n" for row in freeze['files']).encode()
        output = subprocess.run(['git', 'cat-file', '--batch'], cwd=repo, input=specs,
                                capture_output=True, check=True, timeout=60).stdout
        position = 0
        for row in freeze['files']:
            end = output.index(b'\n', position)
            header = output[position:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'Missing Git blob: ' + row['path'])
            length = int(header[2])
            raw = output[end + 1:end + 1 + length]
            require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Git source changed: ' + row['path'])
            position = end + length + 2
        require(position == len(output), 'Unexpected Git batch data')
    print(f"PASS {len(records)} records; 46 historical Mac tests; 2 final Windows GNU compile/lint checks; no native Windows/GPU claim")


if __name__ == '__main__':
    main()
