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


def graph(raw):
    rows = {}
    for line in raw.decode().splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v([^ |]+)(?: \([^|]+\))?\|([^|]*)',
                             line.removesuffix(' (*)'))
        require(match is not None, 'Dependency graph syntax')
        name, version, flags = match.groups()
        rows.setdefault((name, version), set()).add(tuple(sorted(filter(None, flags.split(',')))))
    return rows


def compiler_subset(path):
    # cargo.jsonl includes retained verbose text as well as JSON events.
    names = {'incant_game_player', 'incant_render', 'wgpu_hal', 'objc2_metal'}
    selected = []
    with Path(path).open('rb') as stream:
        for line in stream:
            if not line.lstrip().startswith(b'{'):
                continue
            value = json.loads(line)
            if value.get('reason') == 'build-finished' or (
                    value.get('reason') == 'compiler-artifact' and
                    value.get('target', {}).get('name') in names):
                selected.append(line)
    return b''.join(selected)


def shipping(manifest, records):
    read = lambda name: json.loads(records['shipping/' + name])
    meta = manifest['shipping']
    plan, freeze = read('packet/build-plan.json'), read('packet/freeze.json')
    result, attempt, children = (read('build/' + name) for name in
                                  ('diagnostic-build.json', 'attempt.json', 'children.json'))
    replay_plan, replay_result = read('packet/cpu-replay-plan.json'), read('functional/result.json')
    source = read('packet/source-manifest.json')
    require(plan['source'] == freeze['source'] == result['source'] == attempt['source'] ==
            replay_plan['source'] == replay_result['source'] == source['source'] == meta['source'], 'Shipping source')
    require(len(source['files']) == freeze['source_files'] == meta['source_files'] == 4539, 'Compiler inventory')
    require(freeze['authored_files'] == meta['authored_files'] == 74, 'Authored inventory')
    for row in read_source_freeze(records)['rows']:
        require(source['files'][row['path']] == row['sha256'], 'Qualified CPU files unchanged')
    for name, row in freeze['files'].items():
        require(sha(records['shipping/packet/' + name]) == row['sha256'] and
                len(records['shipping/packet/' + name]) == row['bytes'], 'Frozen shipping input')
    require(sha(records['shipping/packet/source-manifest.json']) == plan['source_manifest']['sha256'], 'Source inventory binding')
    require(sha(records['shipping/reference/authoring-freeze.json']) == plan['authoring_manifest']['sha256'], 'Authored inventory binding')
    authored = read('reference/authoring-freeze.json')['authoring_files']
    require(len(authored) == 74, 'Exact authored count')
    require(result['plan_sha256'] == attempt['plan_sha256'] == sha(records['shipping/packet/build-plan.json']), 'Executed build plan')
    grant = read('packet/root-build-grant.json')
    require(grant['plan_sha256'] == result['plan_sha256'] and grant['build_sha256'] == sha(records['shipping/packet/build.py']) and
            grant['action'] == 'one-private-build-and-offline-startup' and
            sha(records['shipping/packet/root-build-grant.json']) == attempt['grant_sha256'], 'Build authorization identity')
    require(attempt['status'] == 'complete' and attempt['retries'] == 0 and attempt['scratch_retained'] is False and
            result['build_storage_removed_before_startup'] is True, 'Build terminal/cleanup')
    require(attempt['elapsed_seconds'] <= plan['stage_seconds'] == 3000 and
            attempt['minimum_sampled_free_bytes'] >= plan['minimum_free_bytes'] == 6 * 1024**3, 'Sampled shipping bounds')
    require(len(children) == attempt['children'] == 22 and all(c['exit_code'] == 0 and
            c['managed_returned_after_cleanup'] is True for c in children), 'All22 build children')
    for i, child in enumerate(children, 1):
        for stream in ('stdout', 'stderr'):
            key = 'shipping/build/children/' + f'{i:03}.{stream}'
            if key in records and stream + '_sha256' in child:
                require(sha(records[key]) == child[stream + '_sha256'], 'Raw managed child stream')
    for name, digest in result['artifacts'].items():
        key = 'shipping/build/' + name
        if key in records:
            require(sha(records[key]) == digest, 'Portable build artifact')
    baseline = read('reference/baseline-export.json')
    require(result['codegen'] == baseline['codegen'] and result['data'] == baseline['data'] and
            result['physics']['features'] == baseline['physics']['features'] and result['physics_mode'] == 'fast', 'Codegen/data/physics baseline')
    require(result['startup'] == baseline['startup'] and result['startup']['tick'] == 1 and
            result['startup']['render_input']['gpu_submitted'] is False, 'CPU one-tick startup')
    before, after = graph(records['shipping/build/baseline-mode-dependencies.txt']), graph(records['shipping/build/normal-dependencies.txt'])
    require(before == graph(records['shipping/reference/baseline-normal.txt']) and
            after == graph(records['shipping/build/diagnostic-dependencies.txt']) and before.keys() == after.keys(), 'Normal dependency closure')
    extra = {'incant_game_player': {'native-presentations', 'native-compilation'},
             'incant_render': {'native-compilation'}, 'wgpu-hal': {'diagnostic-presentations', 'diagnostic-compilation'},
             'objc2-metal': {'objc2-core-foundation'}}
    for (name, version), rows in before.items():
        require(after[name, version] == {tuple(sorted(set(r) | extra.get(name, set()))) for r in rows}, 'Only declared feature delta')
    units = [json.loads(line) for line in records[meta['compiler_subset_record']].splitlines()]
    require([u.get('success') for u in units if u['reason'] == 'build-finished'] == [True], 'Actual compiler success')
    actual = []
    for unit in (u for u in units if u['reason'] == 'compiler-artifact'):
        name = unit['target']['name'].replace('wgpu_hal', 'wgpu-hal').replace('objc2_metal', 'objc2-metal')
        require(unit['target']['kind'] in (['lib'], ['bin']), 'Compiler unit kind')
        require(all('/aarch64-apple-darwin/shipping/' in f for f in unit['filenames']), 'Actual target output')
        actual.append({'package': name, 'kind': unit['target']['kind'][0], 'features': sorted(unit['features'])})
    actual.sort(key=lambda u: (u['package'], u['kind']))
    require(actual == result['observer_compiler_units']['target_units'] and len(actual) == 5, 'Five actual target feature recipients')
    require({(u['package'], u['kind']) for u in actual} == {(n, 'lib') for n in extra} | {('incant_game_player', 'bin')}, 'Exact receiver units')
    for unit in actual:
        require(unit['features'] == result['graph_delta'][unit['package']]['after'], 'Actual compiler feature equality')
    old, new = read('reference/baseline-project-build.json'), read('build/project-build.json')
    old_id, new_id = old['script'].pop('build_id'), new['script'].pop('build_id')
    require(old == new and result['compatibility']['prior_build_id'] == old_id and
            result['compatibility']['actual_build_id'] == new_id, 'Honest compiler/payload correspondence')
    require(result['player_sha256'] == replay_result['player_sha256'] == meta['player']['sha256'] and
            result['artifacts']['player/incant_game_player'] == meta['player']['sha256'], 'Copied shipping executable identity')
    require(result['diagnostic_only'] is True and result['eligible_for_distribution'] is False and
            result['eligible_for_performance_gate'] is False and result['window_launches'] == attempt['window_launches'] == 0, 'Diagnostic-only CPU scope')
    policy = read('packet/policy-results.json')
    require(policy['exit_code'] == 0 and policy['tests'] == 18 and policy['native_children'] == 0 and
            re.search(rb'Ran 18 tests in [0-9.]+s\s+OK\s*$', records['shipping/packet/policy-tests.stderr']), 'Packet policy controls')
    require(sha(records['shipping/packet/policy-tests.stderr']) == policy['stderr_sha256'], 'Packet policy raw log')
    for name, digest in policy['source_files'].items():
        require(sha(records['shipping/packet/' + name]) == digest, 'Policy-tested source')
    replay_grant = read('packet/root-cpu-replay-grant.json')
    require(replay_grant['action'] == 'one-cpu-gameplay-replay' and
            replay_grant['plan_sha256'] == replay_result['plan_sha256'] == sha(records['shipping/packet/cpu-replay-plan.json']) and
            replay_grant['runner_sha256'] == sha(records['shipping/packet/cpu-replay.py']) and
            replay_grant['build_sha256'] == replay_result['build_sha256'] == sha(records['shipping/build/diagnostic-build.json']) and
            replay_grant['player_sha256'] == meta['player']['sha256'], 'Full replay actual artifact binding')
    require(replay_result['status'] == 'complete' and replay_result['exit_code'] == 0 and
            replay_result['managed_call_active'] is False and replay_result['retries'] == 0 and
            replay_result['gpu_launches'] == replay_result['window_launches'] == 0 and
            replay_result['performance_claim'] is False, 'Full replay terminal CPU-only scope')
    require(replay_result['elapsed_seconds'] <= replay_plan['stage_seconds'] == 90 and
            replay_result['minimum_sampled_free_bytes'] >= replay_plan['minimum_free_bytes'], 'Replay bounds')
    require(len(replay_result['children']) == 1 and replay_result['children'][0]['exit_code'] == 0 and
            replay_result['children'][0]['managed_returned_after_cleanup'] is True and
            replay_result['children'][0]['elapsed_seconds'] <= replay_plan['child_seconds'] == 45 and
            replay_result['children'][0]['argv'] == replay_plan['argv'], 'Exactly one bounded replay child')
    for name, digest in replay_result['artifacts'].items():
        if name != 'output.pcm':
            require(sha(records['shipping/functional/' + name]) == digest, 'Raw replay receipt')
    observed, expected = read('functional/stdout.json'), read('reference/cpu-stdout.json')
    require(observed == expected and observed['tick'] == 2156 and observed['observations']['passed'] is True and
            len(observed['observations']['checks']) == 8 and len(observed['observations']['entities']) == 17 and
            all(c['passed'] is True for c in observed['observations']['checks']), 'Complete logical parity')
    require(not records['shipping/functional/stderr.log'] and observed['pcm_sha256'] == replay_plan['baseline_pcm_sha256'] ==
            replay_result['artifacts']['output.pcm'] and replay_plan['pcm_bytes'] == 13798400, 'Recorded full PCM parity')
    require(all(meta[k] is False for k in ('actual_host_stage_observation', 'distribution_eligible', 'performance_gate_passed')) and
            meta['window_launches'] == meta['gpu_launches'] == 0, 'No live or gate acceptance')


def read_source_freeze(records):
    return json.loads(records['source-001/freeze.json'])


def shipping_external(manifest, records, originals, binaries, source_root):
    meta = manifest['shipping']
    if originals:
        for row in meta['external_originals']:
            require(Path(row['path']).stat().st_size == row['bytes'] and
                    file_sha(row['path']) == row['sha256'], 'External raw/cooked/PCM artifact')
        require(compiler_subset(meta['compiler_stream']['path']) == records[meta['compiler_subset_record']], 'Exact raw compiler subset')
        plan = json.loads(records['shipping/packet/build-plan.json'])
        files = json.loads(records['shipping/reference/authoring-freeze.json'])['authoring_files']
        for name, digest in files.items():
            path = Path(name)
            require(not path.is_absolute() and '..' not in path.parts, 'Authored relative path')
            require(file_sha(Path(plan['authoring_root']) / path) == digest, 'Authored original bytes')
    if binaries:
        row = meta['player']
        require(Path(row['path']).stat().st_size == row['bytes'] and file_sha(row['path']) == row['sha256'], 'Copied shipping player bytes')
    if source_root:
        files = json.loads(records['shipping/packet/source-manifest.json'])['files']
        for name, digest in files.items():
            path = Path(name)
            require(not path.is_absolute() and '..' not in path.parts, 'Source relative path')
            require(file_sha(source_root / path) == digest, 'Producing source bytes: ' + name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--source-root', type=Path, help='Preserved producing source checkout, not current main')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(file_sha(__file__) == manifest['verifier_sha256'], 'Verifier identity')
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
    shipping(manifest, records)
    shipping_external(manifest, records, args.originals, args.binaries, args.source_root)
    if args.binaries:
        for row in manifest['external_binaries']:
            require(Path(row['path']).stat().st_size == row['bytes'] and
                    file_sha(row['path']) == row['sha256'], 'Copied executable identity')
    print(f'PASS: {len(records)} records; Rust148+92, analyzer18 and packet18 controls; '
          'fresh shipping build and complete2156-tick CPU logical/recorded PCM parity. '
          'No engine launch by verifier; no live host-stage, visual or performance acceptance.')


if __name__ == '__main__':
    main()
