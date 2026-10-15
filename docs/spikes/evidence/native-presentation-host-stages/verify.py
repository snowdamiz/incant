#!/usr/bin/env python3
"""Read-only replay of CPU/source evidence. Never executes an archived program."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
MAX_BYTES = 32 * 1024 * 1024


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def replay(manifest, records):
    read = lambda name: json.loads(records[name])
    freeze, result = read('source-001/freeze.json'), read('cpu-001/result.json')
    receipt, plan = read('qualification-receipt.json'), read('cpu-plan-001.json')
    require(freeze['source'] == result['source'] == receipt['source'] == manifest['source'], 'Source binding')
    require(sha(records['source-001/freeze.json']) == receipt['freeze_sha256'], 'Frozen source receipt')
    require(sha(records['cpu-plan-001.json']) == receipt['plan_sha256'] == result['plan_sha256'], 'Plan binding')
    require(sha(records['qualify-001.py']) == receipt['runner_sha256'] == result['runner_sha256'], 'Runner binding')
    require(sha(records['cpu-001/result.json']) == receipt['result_sha256'], 'Actual result binding')
    require(sha(records['source-review.json']) == freeze['review_sha256'], 'Independent source review binding')
    require(len(freeze['rows']) == 13, 'Exact source inventory')
    for row in freeze['rows']:
        raw = records['source-001/inputs/' + row['path']]
        require(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'Frozen source bytes')
    require(result['outcome'] == 'success' and result['error'] is None and
            result['managed_call_active'] is False, 'Terminal CPU success')
    require(not any(result[k] for k in ('gpu_execution', 'window_execution', 'listener_execution',
                                       'native_player_execution')), 'CPU-only scope')
    require(len(result['commands']) == 8 and receipt['source_unchanged'] is True and
            receipt['managed_children_returned_after_cleanup'] is True, 'Command and source closure')
    actual_counts = {}
    for command in result['commands']:
        require(command['exit_code'] == 0 and command['managed_returned_after_cleanup'] is True, 'Child result')
        for stream in ('stdout', 'stderr'):
            raw = records[f'cpu-001/{command["name"]}.{stream}']
            require(sha(raw) == command[stream + '_sha256'], 'Raw CPU stream')
        if command['name'] in ('observed-lib', 'ordinary-lib'):
            raw = records[f'cpu-001/{command["name"]}.stdout']
            counts = re.findall(rb'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', raw)
            require(len(counts) == 1, 'Exactly one successful test harness')
            actual_counts[command['name']] = int(counts[0])
        if command['name'].endswith(('lib', 'clippy')):
            argv = command['argv']
            require('--locked' in argv and '--offline' in argv and
                    argv[argv.index('--target') + 1] == 'aarch64-apple-darwin' and
                    argv[argv.index('--profile') + 1] == 'ci' and argv[argv.index('-j') + 1] == '2', 'CPU selections')
    require(actual_counts == receipt['counts'] == {'observed-lib': 148, 'ordinary-lib': 92}, 'Actual test counts')
    require(result['elapsed_seconds'] <= 2400 and result['minimum_sampled_free_bytes'] >= 6 * 1024**3 and
            result['target_last_sampled_bytes'] - result['target_initial_bytes'] <= 2 * 1024**3, 'Sampled CPU bounds')
    require(plan['source'] == manifest['source'], 'Plan source')
    python = read('analyzer-001/result.json')
    require(python['exit_code'] == 0 and python['tests'] == 18 and python['native_execution'] is False, 'Pure-data controls')
    for stream in ('stdout', 'stderr'):
        require(sha(records['analyzer-001/tests.' + stream]) == python[stream + '_sha256'], 'Python raw log')
    require(re.search(rb'Ran 18 tests in [0-9.]+s\s+OK\s*$', records['analyzer-001/tests.stderr']) is not None,
            'Actual Python test summary')
    for path, digest in python['source_files'].items():
        require(sha(records['analyzer-001/source/' + path]) == digest, 'Analyzer source identity')
    # Independent arithmetic on two retained synthetic examples, not live data.
    sample, observed = read('analyzer-001/catchup-input.json'), read('analyzer-001/catchup-analysis.json')
    first, second = sample['host']['attempts']
    s = first['host_timing']['schedule']
    require(s['pulse_ns'] == second['host_timing']['schedule']['pulse_ns'] == 1000 and
            s['deadline_ns'] == 995 and s['discarded_ticks'] == 3, 'Synthetic batch inputs')
    require(len(observed['scheduler_batches']) == 1 and observed['scheduler_batches'][0]['ordinals'] == [0, 1] and
            observed['discarded_ticks']['valid_batches_sum'] == 3 and
            observed['summary']['batch_wake_lateness']['valid_subset_sum_ns'] == 5 and
            observed['attempts'][1]['metrics']['pre_attempt']['ns'] == 1020, 'Batch arithmetic')
    partial = read('analyzer-001/partial-analysis.json')
    require(partial['complete_requested_host_timing'] is False and
            partial['summary']['finished_total']['missing'] == 1 and
            partial['summary']['adjacent_host_finish_intervals']['valid'] == 0, 'Partial example remains partial')
    require(all(not a['performance_gate_passed'] and not a['display_cadence_measured']
                for a in (observed, partial)), 'No display/performance verdict')
    require(len(manifest['external_binaries']) == 2 and
            [b['sha256'] for b in manifest['external_binaries']] ==
            [b['sha256'] for b in receipt['executed_copied_binaries']], 'Copied test identities')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(0 < manifest['archive_bytes'] <= MAX_BYTES and 0 < manifest['expanded_bytes'] <= MAX_BYTES, 'Packet bounds')
    with (HERE / 'records.json.gz').open('rb') as stream:
        packed = stream.read(MAX_BYTES + 1)
    require(len(packed) == manifest['archive_bytes'] and sha(packed) == manifest['archive_sha256'], 'Archive identity')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        expanded = stream.read(manifest['expanded_bytes'] + 1)
    require(len(expanded) == manifest['expanded_bytes'], 'Expanded bound')
    records = {name: base64.b64decode(value, validate=True) for name, value in json.loads(expanded).items()}
    require(set(records) == {row['name'] for row in manifest['records']}, 'Exact record inventory')
    for row in manifest['records']:
        raw = records[row['name']]
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Record identity')
        if args.originals:
            require(Path(row['original']).read_bytes() == raw, 'Retained original identity')
    replay(manifest, records)
    if args.binaries:
        for row in manifest['external_binaries']:
            require(Path(row['path']).stat().st_size == row['bytes'] and
                    file_sha(row['path']) == row['sha256'], 'Copied executable identity')
    print(f'PASS: {len(records)} records; Rust148+92 and Python18 CPU controls. '
          'No engine launch, live host-stage coverage, display or performance claim.')


if __name__ == '__main__':
    main()
