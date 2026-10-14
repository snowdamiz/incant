#!/usr/bin/env python3
"""Verify retained CPU diagnostic bytes and recompute every reported interval."""
import argparse
import base64
import gzip
import hashlib
import json
import math
from pathlib import Path
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def analyze(raw):
    text = raw.decode()
    marker = 'CORE_SCRIPT_PHASE '
    assert text.count(marker) == 1
    value, _ = json.JSONDecoder().raw_decode(text.split(marker, 1)[1])
    assert '1 passed; 0 failed; 0 ignored;' in text
    assert value['diagnostic_only'] and value['ticks'] == 128 and value['rows'] == 9999
    assert value['negative_controls'] == ['omit-x', 'omit-y', 'omit-z']
    names = value['region_names']
    assert names == ['enter', 'initial_checks', 'snapshots', 'group_setup', 'capabilities',
                     'undo_capture', 'view_exposure', 'callbacks', 'revoke_checks',
                     'numeric_validation', 'transaction_stage', 'scope_cleanup',
                     'publication', 'heap_telemetry']
    frames = value['frames']
    assert len(frames) == 128
    for tick, frame in enumerate(frames, 1):
        assert frame['tick'] == tick and len(frame['regions_ns']) == 14
        assert all(type(x) is int and x >= 0 for x in frame['regions_ns'])
        assert frame['tail_ns'] >= 0
        assert sum(frame['regions_ns']) + frame['tail_ns'] == frame['outer_ns']
    regions = []
    for index, name in enumerate(names):
        values = [f['regions_ns'][index] for f in frames]
        regions.append(dict(name=name, samples=128, sum_ns=sum(values),
                            mean_ns=statistics.mean(values),
                            p95_order_statistic_ns=sorted(values)[math.ceil(.95 * 128) - 1],
                            max_ns=max(values)))
    outer = [f['outer_ns'] for f in frames]
    profile = value['profile']
    assert profile['quickjs_heap']['samples'] == 128
    assert len(profile['scripts']) == 1
    zones = {z['name']: z['totals'] for z in profile['zones']}
    for totals in [zones['script.tick'], zones['script.source'], profile['scripts'][0]['totals']]:
        assert totals['calls'] == 128 and totals['unwound'] == 0
    assert profile['completed_frames'] == profile['failed_frames'] == 0
    assert all(p['samples'] == 0 for p in profile['frame_wall']['phases'])
    return dict(regions=regions, outer=dict(samples=128, sum_ns=sum(outer),
                mean_ns=statistics.mean(outer), p95_order_statistic_ns=sorted(outer)[121],
                max_ns=max(outer)), tail_mean_ns=statistics.mean([f['tail_ns'] for f in frames]))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true', help='Also rehash original local evidence and binary')
    parser.add_argument('--git-source', action='store_true', help='Verify executed inputs against immutable source commit')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    packed = (HERE / 'records.json.gz').read_bytes()
    assert sha(packed) == manifest['archive_sha256']
    records = json.loads(gzip.decompress(packed))
    payloads = {}
    for row in records:
        if 'payload' in row:
            data = base64.b64decode(row['payload'], validate=True)
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
            payloads[row['path']] = data
        if args.external:
            data = Path(row['original']).read_bytes()
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
    run = json.loads(payloads['run-001.json'])
    assert run['exit_code'] == 0 and run['source_unchanged'] and run['managed_child_tree_reaped']
    assert run['argv'] == ['./tools/cargo', 'test', '--offline', '--locked', '--profile', 'ci',
                           '-j2', '--target', 'aarch64-apple-darwin', '-p', 'incant_script',
                           '--no-default-features', '--features',
                           'compiler,profile,physics-fast-simd8', '--lib',
                           'tick_probe::tests::actual_native_stress_regions_128', '--',
                           '--exact', '--ignored', '--nocapture', '--test-threads=1']
    assert run['environment']['RUSTUP_TOOLCHAIN'] == '1.99.0'
    assert run['environment']['RAYON_NUM_THREADS'] == '2'
    assert run['stdout_sha256'] == sha(payloads['run-001.stdout'])
    assert run['stderr_sha256'] == sha(payloads['run-001.stderr'])
    assert run['source_freeze_sha256'] == sha(payloads['source-001/freeze.json'])
    assert run['build_inputs_sha256'] == sha(payloads['build-inputs-001.json'])
    assert run['supervisor_sha256'] == sha(payloads['run-001.py'])
    assert len(run['binaries']) == 1 and run['binaries'][0]['mode'] == '0o755'
    result = analyze(payloads['run-001.stdout'])
    stored = json.loads(payloads['analysis-001.json'])
    for key, value in result.items():
        assert value == stored[key], key
    freeze = json.loads(payloads['source-001/freeze.json'])
    for row in freeze['inputs']:
        assert sha(payloads['source-001/files/' + row['path']]) == row['sha256']
    prepared = json.loads(payloads['prepared-001/manifest.json'])
    assert sha(payloads['prepared-001/manifest.json']) == freeze['prepared_manifest_sha256']
    assert sha(payloads['prepared-001/stress.ts']) == prepared['source_sha256']
    for path, digest in prepared['files'].items():
        assert sha(payloads['prepared-001/' + path]) == digest
    if args.git_source:
        for row in json.loads(payloads['build-inputs-001.json']):
            data = subprocess.check_output(['git', 'show', manifest['source_commit'] + ':' + row['path']], cwd=ROOT)
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
    print(json.dumps(dict(verified=True, records=len(records), payloads=len(payloads),
                         ticks=128, rows_per_tick=9999, negative_controls=3,
                         diagnostic_only=True, recomputed=result), indent=2))


if __name__ == '__main__':
    main()
