#!/usr/bin/env python3
"""Offline source/CPU/shipping diagnostic replay. Never launches an engine or compiler."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / 'manifest.json').read_text())
    archive = (packet / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive']['sha256'], 'archive hash')
    require(len(archive) == manifest['archive']['bytes'], 'archive bytes')
    records = json.loads(gzip.decompress(archive))['records']
    require(manifest['schema'] == 2 and len(records) == manifest['archive']['records'] == 94,
            'record count/schema')
    require(len({row['path'] for row in records}) == len(records), 'duplicate record')
    retained = {}
    for row, expected in zip(records, manifest['records'], strict=True):
        require({key: value for key, value in row.items() if key != 'data'} == expected,
                'record inventory')
        data = base64.b64decode(row['data'], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], row['path'])
        if args.originals:
            require(digest(Path(row['path']).read_bytes()) == row['sha256'], 'original ' + row['path'])
        retained[row['path']] = data

    def raw(suffix):
        matches = [data for path, data in retained.items() if path.endswith('/' + suffix)]
        require(len(matches) == 1, 'exact record ' + suffix)
        return matches[0].decode()

    def obj(suffix):
        return json.loads(raw(suffix))

    first, final = obj('cpu-001/results.json'), obj('cpu-002/results.json')
    require([row['exit_code'] for row in first['results']] == [0, 0, 101], 'first failure retained')
    require(not obj('cpu-001/terminal.json')['success'], 'first attempt cannot pass')
    require('E0659' in raw('cpu-001/2.raw.log'), 'original ambiguous import')
    require('test result:' not in raw('cpu-001/2.raw.log'), 'no original Rust test claim')
    for index, count in enumerate((22, 2)):
        require(re.search(rf'^Ran {count} tests in ', raw(f'cpu-001/{index}.raw.log'), re.M), 'Python count')
        require(raw(f'cpu-001/{index}.raw.log').endswith('\nOK\n'), 'Python outcome')
    rows = final['results']
    require(len(rows) == 7, 'remaining stages')
    require(all(row['exit_code'] == 0 and row['owned_cleanup_completed'] and 'error' not in row
                for row in rows), 'terminal outcomes/cleanup')
    require([row['original_stage_index'] for row in rows] == list(range(2, 9)), 'no repeated Python stages')
    for index, (passed, ignored) in enumerate(((13, 0), (18, 0), (6, 0), (156, 1), (151, 1))):
        require(f'test result: ok. {passed} passed; 0 failed; {ignored} ignored;' in raw(f'cpu-002/{index}.raw.log'),
                'exact feature-configuration result')
    for index in (0, 1):
        require('64,000 check/clock annotations in 1,000 complete frames allocate/reallocate nothing'
                in raw(f'cpu-002/{index}.raw.log'), 'diagnostic allocation control')
    for index in (0, 1, 2):
        require('allocation controls: hot scopes no allocation;' in raw(f'cpu-002/{index}.raw.log'),
                'dedicated allocation executable')
    require('budget_attribution_observes_real_vm_interrupt_and_explicit_checks_without_changing_writes ... ok'
            in raw('cpu-002/3.raw.log'), 'actual VM/rollback/detachment control')
    for index in (5, 6):
        require(rows[index]['argv'][-3:] == ['--', '-D', 'warnings'], 'strict lint')
        require('--all-targets' in rows[index]['argv'], 'all-target lint')
    terminal = obj('cpu-002/terminal.json')
    require(terminal['success'] and terminal['all_returned_children_cleanup_completed'], 'final cleanup')
    require(terminal['target_growth_bytes'] == 74090239 < 2 * 1024**3, 'cumulative target bound')
    require(final['copied_artifacts'] == manifest['binaries'] and len(manifest['binaries']) == 8,
            'copied executable inventory')
    for binary in manifest['binaries']:
        if args.binaries:
            data = Path(binary['copy']).read_bytes()
            require(len(data) == binary['bytes'] and digest(data) == binary['sha256'], 'binary copy')
    scope = manifest['scope']
    require(not scope['measurement_unrun'] and scope['shipping_diagnostic_complete']
            and scope['normal_performance_measurement_unrun']
            and not scope['eligible_for_performance_gate']
            and scope['no_capture_or_CUA'] and scope['no_gpu_or_native_window'], 'diagnostic-only scope')
    root = (args.source_root or packet.parents[3]).resolve()
    if args.git_source or args.source_root:
        for row in manifest['source_inputs']:
            if not row['composition_check']:
                continue  # The prior source document is retained; final prose is docs-only.
            if args.git_source:
                data = subprocess.check_output(['git', 'show', args.git_source + ':' + row['path']], cwd=root)
                require(digest(data) == row['sha256'], 'Git source ' + row['path'])
            if args.source_root:
                require(digest((root / row['path']).read_bytes()) == row['sha256'], 'live source ' + row['path'])
    # Bind the pure parser implementations before importing them. This does not execute
    # source admission, native commands, toolchain probes or the build entry point.
    inputs = obj('shipping-source-001.json')
    require(inputs['source_commit'] == manifest['shipping_source_sha']
            == '4109b15c357d6377a10266f0a902d220cb9ad60f'
            and len(inputs['files']) == 4545, 'producing source identity')
    parsers = ['tools/probes/core-script-attribution/' + name for name in
               ('common.py', 'analyze.py', 'budget_analysis.py')]
    parsers += ['tools/native_package.py', 'tools/mac_performance.py', 'tools/export_native.py',
                'tools/native_process.py', 'tools/native_profiling.py', 'tools/physics_export.py',
                'tools/physics_features.py', 'tools/shipping.py', 'tools/shipping_symbols.py']
    for name in parsers:
        require(digest((root / name).read_bytes()) == inputs['files'][name], 'parser source ' + name)
    sys.path[:0] = [str(root / 'tools/probes/core-script-attribution'), str(root / 'tools')]
    from analyze import analyze
    from common import BUDGET_FEATURES, TARGET, compiler_features
    from export_native import verify_codegen

    require(re.search(r'^Ran 19 tests in ', raw('adapter-source-001/python-controls.raw.log'), re.M)
            and raw('adapter-source-001/python-controls.raw.log').endswith('\nOK\n'),
            'private analyzer controls')
    plan = obj('shipping-plan-001.json')
    build = obj('private-build-001/diagnostic-build.json')
    command = obj('private-build-001/command.json')
    supervision = obj('shipping-supervision-001/result.json')
    require(build['source'] == command['source']
            and build['source']['commit'] == inputs['source_commit']
            and build['source']['files'] == 4545
            and build['source']['manifest_sha256'] == digest(raw('shipping-source-001.json').encode()),
            'fresh source inventory binding')
    plan_hash = digest(raw('shipping-plan-001.json').encode())
    require(build['plan_sha256'] == supervision['plan_sha256'] == plan_hash
            and build['diagnostic_only'] and not build['eligible_for_distribution']
            and not build['eligible_for_performance_gate'] and build['profile_version'] == 4
            and build['build_storage_removed'] and build['player_launches'] == 0, 'private build scope')
    require(plan['features'] == BUDGET_FEATURES and plan['ticks'] == 2156
            and plan['retry_count'] == 0 and plan['physics_mode'] == 'fast', 'private plan')
    require(supervision['status'] == 'complete' and supervision['exit'] == 0
            and supervision['terminal_reaped'] and supervision['remaining_owned_scratch'] == []
            and supervision['player_launches'] == 0 and supervision['retries'] == 0
            and not supervision['gpu_window_capture'] and len(supervision['children']) == 17
            and all(c['exit'] == 0 and c['terminal_reaped'] for c in supervision['children'])
            and supervision['minimum_sampled_free_bytes'] >= 6 * 1024**3, 'build lifecycle')
    require(command['environment']['CARGO_BUILD_JOBS'] == '2'
            and command['environment']['RAYON_NUM_THREADS'] == '2'
            and command['environment']['CARGO_NET_OFFLINE'] == 'true'
            and '--locked' in command['argv'] and '--offline' in command['argv']
            and '--no-default-features' in command['argv'], 'actual bounded build flags')
    baseline = obj('package/export.json')
    compiler = obj('outcome-001/compiler-records.json')
    actual = compiler_features(compiler['messages'], Path(command['target']),
                               baseline['profiling']['compiler']['features'], BUDGET_FEATURES)
    require(actual == build['features'], 'actual target feature graph')
    require(verify_codegen(raw('private-build-001/build.log'), TARGET, 'packed') == build['codegen'],
            'actual shipping codegen')
    require(build['baseline_export_sha256'] == digest(raw('package/export.json').encode())
            and build['data'] == baseline['data'] and len(build['data']['files']) == 28
            and build['codegen'] == baseline['codegen']
            and build['compatibility']['all_other_script_metadata_equal']
            and build['compatibility']['baseline_build_id'] != build['compatibility']['diagnostic_build_id'],
            'historical package/conservative identity distinction')
    if args.originals:
        cargo = Path(compiler['raw_cargo']['path']).read_bytes()
        require(len(cargo) == compiler['raw_cargo']['bytes']
                and digest(cargo) == compiler['raw_cargo']['sha256'], 'original Cargo stream')
        messages = [json.loads(line) for line in cargo.decode().splitlines() if line.startswith('{')]
        names = set(actual['features'])
        selected = [m for m in messages if m.get('reason') == 'build-finished'
                    or m.get('reason') == 'compiler-artifact' and m['target']['name'] in names]
        require(selected == compiler['messages'], 'exact original compiler selection')

    functional = obj('private-functional-001/stdout.json')
    diagnostic = obj('private-diagnostic-001/stdout.json')
    historical = obj('functional-001/maintained/stdout.json')
    def logical(report):
        return {key: value for key, value in report.items() if key != 'profile'}
    require('profile' not in functional and logical(diagnostic) == functional == logical(historical),
            'entire gameplay report parity')
    require(functional['tick'] == 2156 and len(functional['observations']['checks']) == 8
            and functional['observations']['passed'] is True
            and all(row['passed'] is True for row in functional['observations']['checks']), 'authored observations')
    pcm_hash = 'ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3'
    for stage in ('functional', 'diagnostic'):
        prefix = 'private-' + stage + '-001/'
        result = obj(prefix + 'result.json')
        require(result['stage'] == stage and result['status'] == 'complete' and result['exit'] == 0
                and result['terminal_reaped'] and result['logical_and_pcm_equal'] and result['retries'] == 0
                and not result['eligible_for_performance_gate'] and result['child_timeout_seconds'] == 45
                and result['execution_deadline_seconds'] == 90 and result['source'] == build['source']
                and result['plan_sha256'] == plan_hash
                and result['build_identity'] == digest(raw('private-build-001/diagnostic-build.json').encode()),
                'child source/lifecycle ' + stage)
        require(result['artifacts']['output.pcm'] == {'bytes': 13798400, 'sha256': pcm_hash}
                and functional['pcm_sha256'] == pcm_hash and raw(prefix + 'stderr.log') == '', 'PCM/stderr parity')
        for name in ('stdout.json', 'stderr.log'):
            data = raw(prefix + name).encode()
            require(result['artifacts'][name] == {'bytes': len(data), 'sha256': digest(data)}, 'child raw binding')
    sources = [row['source'] for row in historical['profile']['scripts']]
    observation = analyze(diagnostic, sources, budget=True)
    require(observation == obj('private-diagnostic-001/attribution.json'), 'full schema3 attribution replay')
    summary = obj('outcome-001/summary.json')
    require(summary == manifest['outcomes']['shipping'], 'summary inventory')
    expected = [('all_frames', 2156, [2967958, 4287912, 0, 2110812], [11478, 25822, 0, 10780]),
                ('same_frame_tail', 108, [346738, 382745, 0, 177465], [602, 1376, 0, 540])]
    for name, count, ns, calls in expected:
        budget, phase, stated = observation['budget_checks'][name], observation[name], summary[name]
        require(budget['count'] == stated['frames'] == count
                and budget['ns'] == stated['ns'] == ns
                and budget['calls'] == budget['clock_reads'] == stated['calls'] == stated['clock_reads'] == calls
                and budget['refused'] == stated['refused'] == [0] * 4, 'exact measured aggregates')
        require(stated['check_wall_ns_per_frame'] == {'num_ns': sum(ns), 'den_frames': count}
                and stated['check_wall_share_of_whole'] == {'num_ns': sum(ns), 'den_ns': phase['whole_tick_total_ns']}
                and stated['callback_check_wall_share'] == {'num_ns': sum(ns[:2]),
                    'den_ns': sum(phase['callback_nested_totals_ns'].values())}, 'same-frame integer ratios')
    require(summary['instrumented_p95_context'] == observation['instrumented_whole_p95_interval_ns']
            and summary['raw_row_bytes'] == diagnostic['profile']['script_phase_diagnostic']['raw_row_bytes'] == 912,
            'instrumented-only context')
    for row in manifest['retained_files']:
        if args.binaries:
            data = Path(row['path']).read_bytes()
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'retained ' + row['path'])
    print('PASS: 94 records; 24 qualification + 19 analyzer Python cases; 344 Rust feature-configuration cases; '
          'three allocation executables; two strict lints; original compile refusal retained. '
          'Fresh private shipping build and 2156-frame gameplay/PCM/schema3 replay; '
          'check wall 9366682ns/2156 and tail 906948ns/108. Scoped negative result, no performance gate.')


if __name__ == '__main__':
    main()
