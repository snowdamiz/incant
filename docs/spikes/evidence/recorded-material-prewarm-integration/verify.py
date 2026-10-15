#!/usr/bin/env python3
"""Verify portable integration records as data; never import or run retained code."""
import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
HEAD = '00dc34c5c7913d1fcae5e2191e6db62e10172115'
JOINED = 'c25c6f507f21ff5b2b576f252c147cef941d5cef'
GPU_HEAD = '11d4f5f626319b014656da24064e26478e2e83de'
SOURCE = '83fc0e2b287cf1640f6ad39dc73a1331b02c3e3762a2c7317a444fb65da021f6'
LEGACY_SHA = '2bd228140bee06e6b223c74cedfdc790558f6fab7a3a0bfd008ca1aa57c1ebad'
GIB = 1024**3


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(data, legacy):
    require(data['schema'] == 1, 'Schema')
    records, patches = data['records'], data['external_patch_inputs']
    for name, row in records.items():
        path = Path(name)
        require(not path.is_absolute() and '..' not in path.parts, 'Relative record')
        raw = row['text'].encode()
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Record bytes: ' + name)
    def read(name):
        return json.loads(records[name]['text'])
    def binding(name):
        return records[name] if name in records else patches[name]
    outcome = data['outcome']
    require(outcome == {'CPU_source': HEAD, 'integrated_source': JOINED,
                       'original_GPU_source': GPU_HEAD, 'CPU_passed': 14,
                       'GPU_ignored': 6, 'Mac_strict_lints': 2,
                       'Windows_GNU_strict_compile_stages': 2,
                       'new_GPU_executions': 0, 'Windows_runtime_executed': False,
                       'performance_or_phase_gate_passed': False}, 'Scoped outcome')
    require(legacy['outcome']['tested_source_commit'] == GPU_HEAD and
            legacy['outcome']['native_controls_passed'] == 6, 'Original GPU identity')
    for prefix in ('cpu', 'windows'):
        for row in read(prefix + '/packet-manifest.json'):
            actual = binding(prefix + '/' + row['path'])
            require((actual['bytes'], actual['sha256']) == (row['bytes'], row['sha256']), 'Frozen input binding')
    source = read('cpu/source-manifest.json')
    require(source['head'] == HEAD and len(source['files']) == 22067 and
            records['cpu/source-manifest.json']['sha256'] == SOURCE, 'CPU source')
    require(read('windows/source.json')['files'] == source['files'] and
            read('windows/source.json')['source'] == HEAD, 'Same crosscheck source')
    plan = read('cpu/plan.json')
    require(plan['head'] == HEAD and len(plan['stages']) == 12 and
            (plan['target_limit_bytes'], plan['aggregate_storage_limit_bytes'], plan['free_floor_bytes']) ==
            (10*GIB, 13*GIB, 20*GIB), 'Mac original allocations')
    totals = {'build': 0., 'cpu': 0., 'lint': 0.}
    passed = ignored = 0
    pids = set()
    for stage in plan['stages']:
        folder = 'cpu/evidence/' + stage['name']
        row = read(folder + '/result.json')
        require(row['exit_code'] == 0 and row['error'] is None and row['cleanup_confirmed']
                and not row['cleanup_errors'], 'CPU success/cleanup')
        totals[stage['charge']] += row['elapsed_seconds']
        pids.update((row['supervisor_pid'], read(folder + '/process.json')['pid']))
        pids.update(v['pid'] for v in row['observed_descendants'])
        if 'passed' in stage:
            matches = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', records[folder + '/stdout.log']['text'])
            require(matches == [(str(stage['passed']), '0', str(stage['ignored']))], 'Actual test counts')
            passed += stage['passed']; ignored += stage['ignored']
    require((passed, ignored) == (14, 6), 'Focused control counts')
    close = read('cpu/evidence/closeout.json')
    require(read('cpu/evidence/wrapper-terminal.json')['actual_exec_terminal_exit'] == 0 and
            close['actual_wrapper_exit'] == 0 and close['spent_seconds'] == totals, 'Mac wrapper/cost')
    absent = read('cpu/evidence/owned-process-absence.json')
    require(set(absent['observed_pids']) == pids and len(pids) == 198 and
            absent['all_observed_PIDs_and_leader_groups_absent'] and not absent['ps_matching_rows'], 'Observed PID absence')
    continuation = read('cpu/continuation.json')
    def historical(path):
        return json.loads(legacy['records'][path]['text'])
    for kind, original in (('build', 1800), ('cpu', 180)):
        prior = sum(historical(path)['elapsed_seconds'] for path in continuation[kind + '_receipts'])
        require(prior == continuation['prior_' + kind + '_elapsed_seconds'] and
                continuation['remaining_' + kind + '_seconds'] == original-prior, 'Original CPU charges')
        key = 'remaining_CPU_seconds' if kind == 'cpu' else 'remaining_build_seconds'
        require(close[key] == original-prior-totals[kind], 'New CPU charge preserved')
    prior_lint = sum(historical(path)['elapsed_seconds'] for path in continuation['lint_receipts']['renderer'])
    require(prior_lint == continuation['prior_lint_seconds']['renderer'] and
            close['remaining_renderer_lint_seconds'] == 600-prior_lint-totals['lint'], 'Original lint charges')
    old_gpu = historical('prewarm-gpu-001-jn36ppg6/evidence/gpu-phase.json')
    require(close['remaining_GPU_seconds'] == old_gpu['remaining_gpu_seconds'] == 150.2278788749827 and
            not close['GPU_executed'], 'GPU budget and producer unchanged')
    artifacts = read('cpu/evidence/artifacts.json')
    require(set(artifacts) == {r+'-'+k for r in ('ordinary', 'cooked') for k in ('player', 'incant_render', 'player-controls')}, 'Six fresh artifacts')
    for key, row in artifacts.items():
        expected = data['external_artifacts'][key]
        require(row['source_manifest_sha256'] == SOURCE and
                all(expected[field] == row[field] for field in ('bytes', 'sha256')), 'Fresh artifact source')
    closure = read('cpu/evidence/actual-feature-closure.json')
    for row in closure['routes']:
        name = 'cpu/evidence/' + row['stage'] + '/stdout.log'
        require(records[name]['sha256'] == row['stdout_sha256'], 'Actual feature graph binding')
        actual = {}
        for line in records[name]['text'].splitlines():
            try:
                item = json.loads(line)
            except ValueError:
                continue
            if item.get('reason') == 'compiler-artifact' and item['target']['name'] in (
                    'incant_audio_runtime', 'kira', 'cpal', 'incant_render', 'wgpu_hal', 'incant_game_player'):
                actual[item['target']['name']] = {'features': item['features'], 'package_id': item['package_id']}
        require(actual == row['actual_compiler_artifact_features'], 'Feature graph from build rows')
        if 'player' in row['stage']:
            cooked = 'cooked' in row['stage']
            require(('device' in actual['incant_audio_runtime']['features']) == cooked and
                    ('cpal' in actual) == cooked and ('cpal' in actual['kira']['features']) == cooked and
                    ('cooked-native-shaders' in actual['incant_render']['features']) == cooked, 'Audio/cooked closure')
    reused = read('cpu/source-reuse-proof.json')
    old_files = {row['path']: row for row in historical('prewarm-cpu-009-dyl4ma4j/source-manifest.json')['files']}
    changes = []
    for row in reused['all_inventoried_source_paths']:
        equal = row['before'] == row['after']
        require(row['identical'] == equal and row['after'] in source['files'] and
                row['before'] == old_files.get(row['path']), 'Source reuse bytes')
        if not equal:
            changes.append(row['path'])
    require(changes == ['crates/incant_render/src/material_prewarm/mod.rs'], 'Scoped unchanged implementation')
    composition = read('windows/composition-reuse.json')
    before = {row['path']: row for row in composition['qualified_files']}
    after = {row['path']: row for row in composition['integrated_files']}
    require(composition['qualified_files'] == source['files'] and composition['qualified_source'] == HEAD and
            composition['integrated_source'] == JOINED, 'Composition identities')
    changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
    require(changed == composition['changed_paths'] == ['STATUS.md', 'docs/spikes/dx12-pipeline-library.md',
            'docs/spikes/evidence/dx12-pipeline-library/hosted-first-failure.json',
            'tools/probes/dx12-pipeline-library.py', 'tools/tests/test_dx12_pipeline_library.py'], 'No native input changes after00dc')
    windows = read('windows/run-001/results.json'); winplan = read('windows/plan.json'); winclose = read('windows/closeout.json')
    require(windows['status'] == 'complete' and windows['exit'] == 0 and windows['supervisor_terminal'] and
            winclose['supervisor_terminal_reaped_exit'] == 0 and not windows['copied_artifacts'] and
            winclose['actual_tests_executed'] == 0, 'Windows compile-only result')
    require(len(windows['results']) == 2 and len(winplan['steps']) == 2, 'Two Windows stages')
    for i, row in enumerate(windows['results']):
        require(row['status'] == 'complete' and row['exit'] == 0 and row['cleanup_confirmed'] and
                row['terminal_reaped'] and row['timeout_seconds'] == 120 and
                row['argv'] == winplan['steps'][i]['argv'] and 'clippy' in row['argv'] and
                '--tests' in row['argv'] and row['argv'][-2:] == ['-D', 'warnings'], 'Strict Windows stage')
    for name, row in windows['logs'].items():
        require(records['windows/run-001/' + name]['sha256'] == row['sha256'], 'Windows log binding')
    groups = [v['pid_pgid'] for v in windows['owned_groups']]
    require(all(v['cleanup_confirmed'] for v in windows['owned_groups']) and len(groups) == 7 and
            groups == read('windows/owned-process-absence.json')['owned_PID_groups'] and
            not read('windows/owned-process-absence.json')['ps_matching_rows'], 'Windows owned cleanup')
    require((winplan['cumulative_target_baseline_bytes'], winplan['cumulative_target_ceiling_bytes']) ==
            (12020734905, 16315702201) and winclose['final_target_bytes'] == windows['final_target_bytes'] and
            winclose['remaining_target_bytes'] == 16315702201-windows['final_target_bytes'] and
            winclose['prior_charge_seconds'] == 369 and winclose['elapsed_conservative_charge_seconds'] == 26 and
            winclose['total_charge_seconds'] == 395 and winclose['remaining_original_sequence_seconds'] == 3205 and
            winclose['cumulative_copy_bytes'] == 380898056, 'CORE original resource ledger')
    restored = read('windows/checkout-restored.json')
    require(restored['branch'] == 'codex/native-frame-capture' and restored['head'] ==
            '5b2fa73d5c4ddac035b8be4166cbde782f7f8454' and restored['worktree_clean'] and
            restored['after_actual_wrapper_terminal_and_independent_PID_group_absence'], 'Borrowed checkout restored')
    return {'records': len(records), 'CPU_passed': passed, 'GPU_ignored': ignored,
            'Mac_strict_lints': 2, 'Windows_compile_stages': 2, 'new_GPU_executions': 0}


def load():
    manifest = json.loads((HERE/'manifest.json').read_text())
    path = HERE/'records.json.gz'
    require(path.stat().st_size == manifest['compressed_bytes'] and path.stat().st_size <= 16*1024**2, 'Compressed bound')
    require(digest(path.read_bytes()) == manifest['sha256'], 'Portable identity')
    with gzip.open(path, 'rb') as stream:
        raw = stream.read(64*1024**2+1)
    require(len(raw) == manifest['uncompressed_bytes'] and len(raw) <= 64*1024**2, 'Expanded bound')
    data = json.loads(raw)
    require((len(data['records']), len(data['external_patch_inputs']), len(data['external_artifacts'])) ==
            (manifest['record_count'], manifest['external_patch_input_count'], manifest['external_artifact_count']), 'Manifest inventories')
    legacy_path = HERE.parent/'recorded-material-prewarm'/'records.json.gz'
    require(legacy_path.stat().st_size <= 16*1024**2, 'Historical compressed bound')
    require(digest(legacy_path.read_bytes()) == LEGACY_SHA, 'Original GPU packet unchanged')
    with gzip.open(legacy_path, 'rb') as stream:
        old = stream.read(64*1024**2+1)
    require(len(old) <= 64*1024**2, 'Historical bound')
    return data, json.loads(old)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external-root', type=Path, help='Optionally hash retained files beneath the local evidence root; never execute')
    args = parser.parse_args(); data, legacy = load(); result = verify(data, legacy)
    if args.external_root:
        rows = list(data['external_artifacts'].values()) + list(data['external_patch_inputs'].values())
        for row in rows:
            relative = Path(row['external_relative_path'])
            require(not relative.is_absolute() and '..' not in relative.parts, 'External relative path')
            path = args.external_root/relative
            require(path.is_file() and not path.is_symlink() and path.stat().st_size == row['bytes'], 'External size')
            with path.open('rb') as stream:
                require(hashlib.file_digest(stream, 'sha256').hexdigest() == row['sha256'], 'External hash')
        result['external_files_hashed'] = len(rows)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
