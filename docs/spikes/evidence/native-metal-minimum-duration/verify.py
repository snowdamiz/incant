#!/usr/bin/env python3
"""Replay saved CPU evidence only; never execute archived code or a player."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
SOURCE = 'c0ba6ba54ae31aae050cd685f27de7655bf76d16'


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'Nonfinite JSON'))


def read_bounded(path, cap):
    with Path(path).open('rb') as stream:
        raw = stream.read(cap + 1)
    require(len(raw) <= cap, 'Input exceeds byte cap: ' + str(path))
    return raw


def load():
    manifest = decode(read_bounded(HERE / 'manifest.json', 128 * 1024))
    packed = read_bounded(HERE / 'records.json.gz', 4 * 1024**2)
    require(len(packed) == manifest['archive_bytes'] and sha(packed) == manifest['archive_sha256'],
            'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:
        raw = stream.read(16 * 1024**2 + 1)
    require(len(raw) == manifest['expanded_bytes'] <= 16 * 1024**2, 'Expanded archive bound')
    packet = decode(raw)
    require(packet['format'] == 'incant-minimum-duration-CPU-records-v1', 'Record format')
    records = {}
    for row in packet['records']:
        name = row['path']
        require(name not in records and not Path(name).is_absolute()
                and '..' not in Path(name).parts, 'Duplicate or unsafe record path')
        content = row['text'].encode()
        require(len(content) == row['bytes'] and sha(content) == row['sha256'], 'Record identity')
        records[name] = content
    require(len(records) == manifest['record_count'], 'Record inventory')
    return manifest, records


def validate(manifest, records):
    get = lambda name: decode(records[name])
    require(manifest['format'] == 'incant-minimum-duration-CPU-packet-v1'
            and manifest['source'] == SOURCE, 'Packet source')
    results = [get(f'cpu-{n:03}/result.json') for n in range(1, 4)]
    for n, result in enumerate(results, 1):
        prefix = f'packet-{n:03}/'
        plan = get(prefix + f'cpu-plan-{n:03}.json')
        grant = get(prefix + f'root-cpu-grant-{n:03}.json')
        require(result['source'] == plan['source'] == grant['source'] == SOURCE,
                'Exact native producer')
        require(result['plan_sha256'] == grant['plan_sha256'] == sha(records[prefix + f'cpu-plan-{n:03}.json'])
                and result['supervisor_sha256'] == grant['supervisor_sha256']
                == sha(records[prefix + f'cpu-supervisor-{n:03}.py']), 'Execution binding')
        require(result['supervisor_terminal'] and result['owned_cleanup_records']
                and all(row['owned_group_cleanup_and_reap_completed'] and row['cleanup_error'] is None
                        for row in result['owned_cleanup_records'])
                and all(row['pid_absent'] and row['process_group_absent']
                        for row in result['independent_process_absence']), 'Owned cleanup')
        require(not result['GPU_execution'] and not result['window_or_player_CLI_execution'], 'CPU-only scope')
        for row in result['logs']:
            name = Path(row['path']).name
            raw = records[f'cpu-{n:03}/' + name]
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Raw log identity')
    first, second, final = results
    require(first['status'] == 'failed' and first['exit'] == 1
            and [row['exit'] for row in first['stages']] == [0, 0, 101]
            and 'CoroutineValue' in records['cpu-001/2.stdout.log'].decode(), 'Original cache failure retained')
    diagnosis = get('cpu001-cache-diagnosis/diagnosis.json')
    require(diagnosis['no_incant_types_feature_section'] and diagnosis['current_export_unconditional']
            and not diagnosis['artifacts'][0]['includes_coroutines']
            and all(row['includes_coroutines'] for row in diagnosis['artifacts'][1:]), 'Saved freshness diagnosis')
    require(second['status'] == 'failed' and second['exit'] == 1
            and [row['exit'] for row in second['stages']] == [0, 0, 0, 0, 0, 101]
            and records['cpu-002/5.stderr.log'].decode().strip()
            == 'error: cannot specify features for packages outside of workspace', 'Cargo selection failure retained')
    close = get('cpu003-closeout.json')
    names = []
    for n, count in enumerate([11, 6, 5]):
        raw = records[f'cpu-002/{n}.stdout.log'].decode()
        found = re.findall(r'^test (\S+) \.\.\. ok$', raw, re.M)
        require(len(found) == len(set(found)) == count
                and re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw)
                == [(str(count), '0')] and second['stages'][n]['tests_passed'] == count,
                'Actual CPU mode membership')
        names.append(found)
    require(names == close['mode_test_names'] and len(set(sum(names, []))) == 11
            and manifest['mode_test_counts'] == [11, 6, 5] and manifest['strict_lint_count'] == 3,
            'CPU coverage claims')
    require(final['status'] == 'complete' and final['exit'] == 0 and len(final['stages']) == 1
            and final['stages'][0]['exit'] == 0 and final['stages'][0]['terminal_reaped'], 'Final HAL lint')
    artifact = final['stages'][0]['HAL_cargo_artifact']
    require(artifact['target']['name'] == 'wgpu_hal' and artifact['features'] == [
        'diagnostic-compilation', 'diagnostic-minimum-duration', 'diagnostic-presentations',
        'metal', 'portable-atomic'], 'Exact primary HAL features')
    copies = [{k: row[k] for k in ['path', 'bytes', 'sha256']} for row in final['copies']]
    require(copies == manifest['binary_copies'] and len(copies) == 18
            and sum(row['bytes'] for row in copies) == close['cumulative_copy_bytes'] == 395296856,
            'Copied artifacts including protected Wasm')
    require(close['cumulative_attempt_seconds'] == final['cumulative_attempt_seconds'] < 2400
            and close['target_original_allocated_bytes'] == 4246511616
            and close['target_peak_and_final_allocated_bytes'] == 6698582016
            <= close['target_effective_ceiling_bytes'] == 8541478912
            < close['target_original_shared_ceiling_bytes'] == 12884901888
            and close['cumulative_log_bytes'] == 4898887, 'Cumulative resource account')
    require(close['wrapper_exit'] == 0 and all(row['pid_absent'] and row['group_absent']
            for row in close['independent_absence'])
            and close['result']['sha256'] == sha(records['cpu-003/result.json']), 'Independent closeout')
    for name in ['native_window_executed', 'shipping_player_executed', 'GPU_executed',
                 'performance_gate_passed', 'pacing_adopted']:
        require(manifest[name] is False, 'Unsupported runtime or performance claim')
    freeze = get('packet-001/source-manifest.json')
    require(freeze['source'] == SOURCE and len(freeze['files']) == manifest['source_inventory_records'] == 4749,
            'Frozen source inventory')
    return freeze


def verify_git(freeze, revision):
    revision = subprocess.check_output(['git', 'rev-parse', revision], cwd=REPO, text=True).strip()
    require(revision == SOURCE, 'Use exact historical native producer')
    # Stream one blob at a time; neither all source text nor all Git output is loaded together.
    with tempfile.TemporaryFile() as requests:
        requests.write(''.join(revision + ':' + name + '\n' for name in freeze['files']).encode())
        requests.seek(0)
        with subprocess.Popen(['git', 'cat-file', '--batch'], cwd=REPO, stdin=requests,
                              stdout=subprocess.PIPE) as process:
            for name, digest in freeze['files'].items():
                header = process.stdout.readline(1024).split()
                require(len(header) == 3 and header[1] == b'blob', 'Missing Git blob')
                size = int(header[2])
                require(0 <= size <= 32 * 1024**2, 'Source blob size bound')
                raw = process.stdout.read(size)
                require(len(raw) == size and sha(raw) == digest and process.stdout.read(1) == b'\n',
                        'Git source changed: ' + name)
            require(process.stdout.read(1) == b'' and process.wait(timeout=30) == 0, 'Trailing Git output')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', metavar='REV')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest, records = load()
    freeze = validate(manifest, records)
    if args.originals:
        for name, raw in records.items():
            require(read_bounded(Path(manifest['original_root']) / name, len(raw)) == raw,
                    'Original changed: ' + name)
    if args.binaries:
        for row in manifest['binary_copies']:
            require(Path(row['path']).stat().st_size == row['bytes']
                    and file_sha(row['path']) == row['sha256'], 'Copied executable changed')
    if args.git_source:
        verify_git(freeze, args.git_source)
    if args.source_root:
        for name, digest in freeze['files'].items():
            require(file_sha(args.source_root / name) == digest, 'Current executable/tool source changed: ' + name)
    print('PASS:65 saved records;22 CPU executions/11 definitions;3 strict lints;both failures retained;no native presentation claim')


if __name__ == '__main__':
    main()
