#!/usr/bin/env python3
"""Bounded saved-data replay. Archived source is inert; no compiler is run."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

MANIFEST_CAP = 256 * 1024
PACKED_CAP = 4 * 1024 * 1024
EXPANDED_CAP = 16 * 1024 * 1024
FILE_CAP = 2 * 1024 * 1024
EXPECTED_REFUSALS = {
    'unknown-warning-option', 'missing-profile', 'corrupt-profile',
    'stale-profile', 'unprofiled-function', 'mixed-corrupt-raw',
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def bounded(path, cap):
    with Path(path).open('rb') as stream:
        data = stream.read(cap + 1)
    require(len(data) <= cap, f'Input exceeds bound: {path}')
    return data


def pairs(items):
    result = {}
    for key, value in items:
        require(key not in result, f'Duplicate JSON key: {key}')
        result[key] = value
    return result


def decode(data):
    def nonfinite(value):
        raise ValueError(f'Nonfinite JSON: {value}')
    return json.loads(data, object_pairs_hook=pairs, parse_constant=nonfinite)


def load(root):
    manifest = decode(bounded(root / 'manifest.json', MANIFEST_CAP))
    packed = bounded(root / 'records.json.gz', PACKED_CAP)
    require(digest(packed) == manifest['archive']['sha256'], 'Archive digest')
    require(len(packed) == manifest['archive']['bytes'], 'Archive length')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:
        expanded = stream.read(EXPANDED_CAP + 1)
    require(len(expanded) <= EXPANDED_CAP, 'Expanded archive exceeds bound')
    require(digest(expanded) == manifest['expanded']['sha256'], 'Expanded digest')
    require(len(expanded) == manifest['expanded']['bytes'], 'Expanded length')
    rows = decode(expanded)
    expected = manifest['records']
    require(len(rows) == len(expected) <= 512, 'Record count')
    records = {}
    for row, binding in zip(rows, expected, strict=True):
        name = row['name']
        require(isinstance(name, str) and '\\' not in name, 'Record name')
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts, 'Record traversal')
        require(name not in records and name == binding['name'], 'Record membership')
        raw = row['text'].encode('utf-8')
        require(len(raw) <= FILE_CAP and len(raw) == binding['bytes'], 'Record size')
        require(digest(raw) == binding['sha256'], f'Record digest: {name}')
        records[name] = raw
    return manifest, records


def validate(manifest, records):
    get = lambda name: decode(records[name])
    final = get('final-001.json')
    result = get('run-004/result.json')
    calls = get('run-004/managed-calls.json')
    close = get('closeout-004.json')
    review = get('root-review-004.json')
    require(manifest['scope'] == 'standalone-C-only-feasibility', 'Scope')
    require(final['status'] == 'C-only-feasibility-passed', 'Final status')
    require(final['script_2ms_gate_passed'] is False
            and final['no_speedup_or_adoption_claim'] is True, 'Gate scope')
    for number in range(1, 4):
        old = get(f'run-{number:03}/result.json')
        require(old['status'] == 'failed', 'Historical failure changed')
        old_close = get(f'closeout-{number:03}.json')
        require(old_close['actual_wrapper_exit'] != 0, 'Historical terminal changed')
    require(result['status'] == 'complete' and result['exit'] == 0
            and result['supervisor_terminal'] is True, 'Actual success')
    require(result['performance_claim'] is False
            and result['Incant_runtime_or_SDK_claim'] is False
            and result['Cargo_GPU_window_executions'] == 0, 'Unsupported claim')
    require(len(calls) == 57 and len({call['name'] for call in calls}) == 57,
            'Call membership')
    require({call['name'] for call in calls if call['exit'] != 0} == EXPECTED_REFUSALS,
            'Expected actual refusals')
    require(sum(call['exit'] == 0 for call in calls) == 51, 'Positive count')
    for call in calls:
        require(call['native_process_returned'] is True and call['cleanup_confirmed'] is True,
                'Managed cleanup not observed')
        require(call['absence'] and all(row['PID_absent'] is True and row['group_absent'] is True
                for row in call['absence']), 'Child absence')
        for field in ['stdout', 'stderr']:
            meta = call[field]
            raw = records['run-004/logs/' + Path(meta['path']).name]
            require(len(raw) == meta['bytes'] and digest(raw) == meta['sha256'], 'Raw stream')
    require(close['actual_wrapper_exit'] == 0 and close['session'] == 91449
            and close['scratch_retired'] is True, 'Terminal/scratch evidence')
    absent = close['independent_process_absence']
    require(len(absent) == 58 and len({row['pid'] for row in absent}) == 58
            and all(row['PID_absent'] is True and row['group_absent'] is True for row in absent),
            'Independent wrapper/child absence')
    require(review['actual_terminal_exit'] == 0 and review['calls_rehashed'] == 57
            and review['streams_rehashed'] == 114 and review['artifacts_rehashed'] == 14
            and review['core_gameplay_audio_or_performance_claim'] is False, 'Root review scope')
    require(digest(records['run-004/result.json']) == review['result_sha256']
            and digest(records['run-004/managed-calls.json']) == review['calls_sha256'],
            'Root review bindings')
    by_name = {call['name']: call for call in calls}
    for mode in ['baseline', 'train', 'use']:
        call = by_name[f'{mode}-heldout']
        report = decode(records['run-004/logs/' + Path(call['stdout']['path']).name])
        require(report == result['heldout']['reports'][mode], 'Heldout raw/result mismatch')
        require(report['actual'] == report['expected'] == 97914
                and report['callbacks'] == 771 and report['bytecode_bytes'] == 317
                and report['heldout_controls'] is True
                and report['Incant_runtime_or_SDK_claim'] is False, 'Heldout behavior')
    hashes = result['heldout']['bytecode_sha256']
    require(set(hashes) == {'baseline', 'train', 'use'} and set(hashes.values()) == {
        '3443fd9dabb27f119b41d098856b20623fc8f114002aee026036f436a862e51e'}, 'Bytecode identity')
    train = result['training_raw_inventory']
    require(len(train) == len(result['discarded_heldout_inventory'])
            == len(result['separate_canary_inventory']) == 1, 'Raw isolation')
    argv = by_name['merge-quickjs']['argv']
    require(argv[-1:] == [train[0]['path']] and '--failure-mode=any' in argv
            and '--sparse' not in argv and argv[argv.index('-j') + 1] == '2', 'Merge inventory')
    for name in ['stale-profile', 'unprofiled-function']:
        raw = records['run-004/logs/' + Path(by_name[name]['stderr']['path']).name].decode()
        require('error:' in raw and '[-Werror,-Wbackend-plugin]' in raw, 'Fatal IR safeguard')
    covered = records['run-004/logs/' + Path(by_name['covered-quickjs']['stdout']['path']).name].decode().splitlines()
    required = {'JS_ReadObject', 'JS_EvalFunction',
                str(Path(result['training_raw_inventory'][0]['path']).parent.parent / 'source/quickjs.c') + ';JS_CallInternal'}
    require(required <= set(covered) and len(covered) == len(set(covered)), 'Covered exact names')
    control = get('portable-controls-001/receipt.json')
    require(control['exit'] == 0 and control['controls'] == 19
            and control['native_or_compiler_execution'] is False, 'Pure controls')
    require(b'Ran 19 tests' in records['portable-controls-001/stderr.txt']
            and records['portable-controls-001/stderr.txt'].rstrip().endswith(b'OK'), 'Control raw output')
    require(abs(result['cumulative_elapsed_seconds'] - 79.67278866597917) < 1e-9,
            'Cumulative cost changed')
    return {'records': len(records), 'actual_calls': 57, 'actual_streams': 114,
            'specific_refusals': 6, 'pure_controls': 19,
            'scope': manifest['scope'], 'performance_or_Core_claim': False}


def originals(manifest, records):
    for row in manifest['records']:
        require(bounded(row['original'], row['bytes']) == records[row['name']], 'Original changed')


def artifacts(manifest):
    for row in manifest['external_artifacts']:
        require(digest(bounded(row['path'], row['bytes'])) == row['sha256'], 'External artifact changed')


def git_source(manifest, repo, revision):
    require(re.fullmatch(r'[0-9a-f]{40}', revision) is not None, 'Exact Git revision required')
    for row in manifest['git_source_files']:
        raw = subprocess.check_output(['git', '-C', str(repo), 'show', f'{revision}:{row["path"]}'])
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Git source changed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--artifacts', action='store_true')
    parser.add_argument('--git-source', metavar='EXACT_COMMIT')
    parser.add_argument('--repo', type=Path, default=Path.cwd())
    args = parser.parse_args()
    manifest, records = load(Path(__file__).parent)
    result = validate(manifest, records)
    if args.originals:
        originals(manifest, records)
    if args.artifacts:
        artifacts(manifest)
    if args.git_source:
        git_source(manifest, args.repo, args.git_source)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
