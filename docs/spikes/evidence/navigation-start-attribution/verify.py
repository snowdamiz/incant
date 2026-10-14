#!/usr/bin/env python3
"""Read-only replay of retained actual observations; no native execution."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(manifest, records):
    read = lambda name: json.loads(records[name])
    source = manifest['source']
    plan = read('shipping-plan-001.json')
    freeze = read('shipping-source-001.json')
    build = read('private-build-001/diagnostic-build.json')
    require(source == plan['source_commit'] == freeze['source_commit']
            == build['source']['commit'], 'Producing source identity')
    require(len(freeze['files']) == build['source']['files'] == 4410, 'Source closure')
    require(sha(records['shipping-plan-001.json']) == build['plan_sha256']
            and sha(records['shipping-source-001.json']) == build['source']['manifest_sha256'],
            'Build admission identities')
    for path, digest in manifest['source_files'].items():
        require(sha(records['source/' + path]) == digest == freeze['files'][path], 'Source bytes')
    require(build['eligible_for_distribution'] is False
            and build['eligible_for_performance_gate'] is False
            and build['player_launches'] == 0, 'Private diagnostic scope')
    expected = {'incant_game_player', 'incant_runtime', 'incant_profile',
                'incant_script', 'incant_nav'}
    require(set(build['graph_delta']) == expected, 'Only five diagnostic recipients')
    for name, delta in build['graph_delta'].items():
        require(len(delta['before']) == len(delta['after']) == 1, 'Unambiguous feature rows')
        added = {'diagnostic-navigation'}
        if name != 'incant_nav':
            added.add('diagnostic-script-phases')
        require(set(delta['after'][0]) == set(delta['before'][0]) | added, 'Exact feature additions')
    for row in build['codegen'].values():
        require(row['opt-level'] == '3' and row['codegen-units'] == '1'
                and row['panic'] == 'abort', 'Shipping codegen')
    require(build['codegen']['incant_game_player']['lto'] == 'fat'
            and build['physics']['mode'] == 'fast', 'Shipping LTO and physics')
    for path, digest in build['artifacts'].items():
        key = 'private-build-001/' + path
        if key in records:
            require(sha(records[key]) == digest, 'Actual compiler/cook records')
    supervise = read('shipping-supervision-002/result.json')
    require(supervise['status'] == 'complete' and supervise['exit'] == 0
            and supervise['terminal_reaped'] is True and len(supervise['children']) == 13
            and all(c['exit'] == 0 and c['terminal_reaped'] for c in supervise['children']),
            'Completed and reaped build children')
    cpu = read('cpu-result-001.json')
    require(cpu['source'] == source and len(cpu['records']) == 7, 'CPU source and commands')
    tests = 0
    for index, row in enumerate(cpu['records']):
        require(row['exit'] == 0 and row['owned_group_cleaned_and_reaped'] is True, 'CPU success/reap')
        raw = records[f'cpu-001-{index}.log']
        require(sha(raw) == row['log_sha256'], 'CPU raw log')
        tests += sum(int(n) for n in re.findall(rb'test result: ok\. (\d+) passed; 0 failed', raw))
    require(tests == 71, 'Actual Rust control count')
    baseline = read('baseline/functional_report')
    reports = []
    identity = None
    for stage in ('functional', 'diagnostic'):
        prefix = 'private-' + stage + '-001/'
        result, actual = read(prefix + 'result.json'), read(prefix + 'stdout.json')
        require(result['status'] == 'complete' and result['stage'] == stage
                and result['exit'] == 0 and result['terminal_reaped'] is True
                and result['logical_and_pcm_equal'] is True and result['retries'] == 0,
                'Actual single child outcome')
        require(result['machine_before'] == result['machine_after'], 'Stable machine settings')
        if identity is None:
            identity = result['build_identity']
        require(result['build_identity'] == identity == sha(records['private-build-001/diagnostic-build.json'])
                and result['plan_sha256'] == build['plan_sha256'],
                'Same admitted build')
        require({k: v for k, v in actual.items() if k != 'profile'} == baseline,
                'Complete historical logical parity')
        require(result['artifacts']['output.pcm']['sha256'] == baseline['pcm_sha256']
                and result['artifacts']['output.pcm']['bytes'] == 13_798_400, 'PCM identity')
        for name, artifact in result['artifacts'].items():
            if prefix + name in records:
                raw = records[prefix + name]
                require(len(raw) == artifact['bytes'] and sha(raw) == artifact['sha256'], 'Actual child bytes')
        require(not records[prefix + 'stderr.log'], 'No child diagnostics')
        reports.append(actual)
    require('profile' not in reports[0], 'Unprofiled control')
    profile = reports[1]['profile']
    diagnostic = profile['script_phase_diagnostic']
    require(profile['version'] == 4 and profile['completed_frames'] == 2156
            and profile['failed_frames'] == profile['dropped_source_samples'] == 0,
            'Complete diagnostic profile')
    require(diagnostic['schema'] == 2 and diagnostic['complete'] is True
            and diagnostic['eligible_for_performance_gate'] is False, 'Diagnostic schema and scope')
    rows = diagnostic['rows']
    require([r['frame'] for r in rows] == list(range(1, 2157)), 'All frames retained')
    require(diagnostic['navigation_regions'] == ['path_query', 'initial_start', 'repeated_start'],
            'Fixed aggregate order')
    totals = {'ns': [0, 0, 0], 'calls': [0, 0, 0], 'failed': [0, 0, 0]}
    hypothetical, whole = [], []
    for row in rows:
        require(row['valid'] and row['success'] and not row['unwound']
                and row['whole_tick_calls'] == 1, 'Valid successful whole tick')
        nav = row['navigation']
        for key in totals:
            require(len(nav[key]) == 3 and all(type(n) is int and n >= 0 for n in nav[key]),
                    'Nonnegative exact aggregate')
            totals[key] = [a + b for a, b in zip(totals[key], nav[key])]
        require(nav['ns'][1] + nav['ns'][2] <= nav['ns'][0] <= sum(row['source_ns'])
                and nav['ns'][0] <= row['whole_tick_ns'], 'Same-frame containment')
        whole.append(row['whole_tick_ns'])
        hypothetical.append(row['whole_tick_ns'] - nav['ns'][2])
    require(totals == {'ns': [197088280, 6016213, 14370827],
                       'calls': [414, 338, 1026], 'failed': [0, 0, 0]}, 'Measured totals')
    # Rank after same-frame subtraction, never subtract separate quantiles.
    rank = (len(rows) * 95 + 99) // 100 - 1
    p95 = sorted(hypothetical)[rank]
    require(rank == 2048 and p95 == 2544041 and p95 > 2_000_000, 'Negative feasibility result')
    require(2_580_001 <= sorted(whole)[rank] <= 2_590_000, 'Instrumented whole-tick bucket')
    analysis = read('private-diagnostic-001/attribution.json')
    require(analysis['navigation']['totals'] == totals
            and analysis['navigation']['zero_cost_same_frame_p95_ns'] == p95
            and analysis['navigation']['stop_hypothesis'] is True, 'Recorded conclusion matches raw')
    return p95


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    packed = (HERE / 'records.json.gz').read_bytes()
    require(len(packed) == manifest['archive_bytes'] and sha(packed) == manifest['archive_sha256'], 'Archive')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(manifest['expanded_bytes'] + 1)
    require(len(raw) == manifest['expanded_bytes'], 'Expanded bound')
    records = {k: base64.b64decode(v, validate=True) for k, v in json.loads(raw).items()}
    require(set(records) == {r['name'] for r in manifest['records']}, 'Exact inventory')
    for row in manifest['records']:
        raw = records[row['name']]
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Record identity')
        if args.originals and 'original' in row:
            require(Path(row['original']).read_bytes() == raw, 'Retained original identity')
    if args.binaries:
        for row in manifest['external_artifacts']:
            raw = Path(row['path']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Retained binary/data/PCM')
    p95 = replay(manifest, records)
    pcm_scope = 'PCM bytes verified' if args.binaries else 'recorded PCM identity'
    print(f'PASS: {len(records)} retained records; full logical parity and {pcm_scope}; '
          f'hypothetical repeated-scan p95 {p95} ns remains above 2 ms. No speedup or gate claim.')


if __name__ == '__main__':
    main()
