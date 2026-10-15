#!/usr/bin/env python3
"""Saved-byte material refresh replay. Never builds or starts an engine/GPU."""
import argparse
import ast
import base64
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import tempfile


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = args.source_root.resolve() if args.source_root else here.parents[3]
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'], 'Archive changed')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(12 * 1024**2 + 1)
    require(len(payload) <= 12 * 1024**2, 'Decoded archive exceeds bound')
    encoded = json.loads(payload)
    require(set(encoded) == {r['key'] for r in manifest['records']}
            and len(encoded) == len(manifest['records']), 'Record membership differs')
    raw = {}
    for row in manifest['records']:
        data = base64.b64decode(encoded[row['key']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record changed: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original changed: ' + row['key'])
    if args.originals:
        for row in manifest['unbundled_ancestry_diffs']:
            path = Path(row['original'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'],
                    'Unbundled ancestry diff changed')

    def data(key):
        return json.loads(raw[key])

    final = data('source-006/freeze.json')
    require(final['base'] == manifest['native_source'] and len(final['files']) == 160,
            'Final source identity differs')
    for index in range(1, 7):
        freeze = data(f'source-{index:03d}/freeze.json')
        require(not freeze['untracked_paths'], 'Uncommitted source inventory')
        require(sha(raw[f'source-{index:03d}/source.diff']) == freeze['review_patch_sha256'],
                'Frozen review diff differs')
        for row in freeze['files']:
            key = manifest['source_blobs'][row['sha256']]
            require(sha(raw[key]) == row['sha256'] and len(raw[key]) == row['bytes'],
                    'Frozen source bytes differ')
    for row in final['files']:
        expected = raw[manifest['source_blobs'][row['sha256']]]
        if args.source_root:
            require((root / row['path']).read_bytes() == expected, 'Live source differs: ' + row['path'])
        if args.git_source:
            actual = subprocess.check_output(['git', 'show', args.git_source + ':' + row['path']],
                                             cwd=root, timeout=10)
            require(actual == expected, 'Git source differs: ' + row['path'])
    composition = data('main355-composition/receipt.json')
    require(composition['native_producing_source'] == manifest['native_source']
            and composition['final_composition_source'] == manifest['final_composition_source']
            and composition['bounded_native_source_inventory160_unchanged']
            and not composition['native_rebuild_or_GPU_rerun'], 'Composition claim differs')
    require({r['path'] for r in composition['compiled_or_build_source_delta']} == {
        'crates/incant_cook/src/typescript/phases.rs', 'tools/build_native_script.mjs',
        'tools/native_script/phases.mjs'}, 'Unexercised compiler delta differs')
    for row in composition['compiled_or_build_source_delta']:
        expected = raw[row['record']]
        require(sha(expected) == row['sha256'], 'Joined source snapshot changed')
        if args.source_root and row['source'] == manifest['final_composition_source']:
            require((root / row['path']).read_bytes() == expected, 'Live composition input differs')
        if args.git_source:
            actual = subprocess.check_output(['git', 'show', row['source'] + ':' + row['path']],
                                             cwd=root, timeout=10)
            require(actual == expected, 'Named historical/composition Git input differs')
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['copy'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'], 'Executable differs')

    cpus = [data(f'cpu-{i:03d}/results.json') for i in range(1, 6)]
    for i, cpu in enumerate(cpus, 1):
        proposal = manifest['CPU_protocols'][str(i)]
        require(cpu['proposal_sha256'] == sha(raw[proposal['plan']])
                and cpu['supervisor_sha256'] == sha(raw[proposal['runner']]), 'CPU protocol differs')
        require(cpu['source'] == manifest['CPU_sources'][str(i)] and cpu['supervisor_terminal']
                and cpu['native_app_player_GPU_window_capture_executions'] == 0, 'CPU execution scope differs')
        require(all(g['cleanup_confirmed'] for g in cpu['owned_groups'])
                and data(f'cpu{i:03d}-closeout.json')['independent_ps_all_absent'], 'CPU cleanup unconfirmed')
        for name, row in cpu['logs'].items():
            require(sha(raw[f'cpu-{i:03d}/' + name]) == row['sha256']
                    and len(raw[f'cpu-{i:03d}/' + name]) == row['bytes'], 'CPU log changed')
    require(cpus[0]['status'] == 'failed' and len(cpus[0]['results']) == 1
            and b'8 passed; 1 failed;' in raw['cpu-001/0.stdout.log'], 'Tangent failure lost')
    require(cpus[1]['status'] == 'complete' and all(r['exit'] == 0 for r in cpus[1]['results']),
            '37-control result differs')

    def counts(key):
        return [int(n) for n in re.findall(rb'test result: ok\. (\d+) passed; 0 failed;', raw[key])]

    require(counts('cpu-002/0.stdout.log') == [9, 11, 7]
            and counts('cpu-002/1.stdout.log') == [1, 4, 5], 'CPU coverage differs')
    for name in (b'prior_geometry_divergence_survives_material_changes_restoration_and_unrelated_texture_refresh',
                 b'a_signed_zero_geometry_change_is_not_admitted_as_material_only_for_baked_navigation',
                 b'material_only_navigation_accepts_before_and_after_stage_but_geometry_still_refuses'):
        require(name + b' ... ok' in raw['cpu-002/0.stdout.log'], 'Navigation behavior evidence missing')
    require(b'real_material_import_undo_redo_and_table_resize_preserve_world_vm_and_old_resources ... ok'
            in raw['cpu-002/1.stdout.log'], 'Real authoring/history evidence missing')
    require(cpus[2]['status'] == 'failed' and counts('cpu-003/0.stdout.log') == [1]
            and b'clippy::duplicate_mod' in raw['cpu-003/1.stderr.log'], 'Module lint failure lost')
    for i in (4, 5):
        cpu = cpus[i - 1]
        require(cpu['status'] == 'complete' and len(cpu['results']) == 2
                and all(r['exit'] == 0 for r in cpu['results'])
                and counts(f'cpu-{i:03d}/0.stdout.log') == [1], 'Final fixture/lint result differs')
        require(cpu['copied_artifacts'][0]['features'] == ['default', 'physics-deterministic'],
                'Actual executable feature route differs')
    require(sum(r['bytes'] for r in manifest['binaries']) == 313245720 < 320 * 1024**2,
            'Cumulative copy charge differs')
    require(cpus[-1]['final_target_bytes'] == 14520147163
            and cpus[-1]['final_target_bytes'] - 12020734905 <= 4 * 1024**3,
            'Cumulative shared target charge differs')

    for i in (1, 2):
        gpu = data(f'gpu-{i:03d}/results.json')
        require(gpu['proposal_sha256'] == sha(raw[f'gpu-proposal-{i:03d}.json'])
                and gpu['supervisor_sha256'] == sha(raw[f'gpu-supervisor-{i:03d}.py']), 'GPU protocol differs')
        require(gpu['supervisor_terminal'] and gpu['no_build_window_capture']
                and all(g['cleanup_confirmed'] for g in gpu['owned_groups'])
                and data(f'gpu{i:03d}-closeout.json')['independent_ps_all_absent'], 'GPU cleanup/scope differs')
        for row in gpu['retained_files']:
            require(sha(raw[f'gpu-{i:03d}/' + row['path']]) == row['sha256']
                    and len(raw[f'gpu-{i:03d}/' + row['path']]) == row['bytes'], 'GPU bytes changed')
        require(b'Apple M5 Pro' in raw[f'gpu-{i:03d}/stderr.log']
                and b'backend: Metal' in raw[f'gpu-{i:03d}/stderr.log'], 'Actual Metal identity missing')
    old, gpu = data('gpu-001/results.json'), data('gpu-002/results.json')
    require(old['status'] == 'failed' and old['child_exit'] == 101
            and b'Option::unwrap()' in raw['gpu-001/stderr.log'], 'History observer failure lost')
    require(raw['gpu-001/pixels/initial.rgba'] == raw['gpu-001/pixels/late_refusal.rgba'],
            'Initial refusal retention differs')
    require(gpu['status'] == 'complete' and gpu['child_exit'] == 0
            and b'test result: ok. 1 passed;' in raw['gpu-002/stdout.log'], 'Final native result differs')
    plan = data('gpu-proposal-002.json')
    require(plan['binary'] == cpus[-1]['copied_artifacts'][0]
            and plan['bounds']['child_seconds'] == 118 and plan['bounds']['sequence_seconds'] == 178
            and plan['predecessor']['prior_log_bytes'] == 3043
            and plan['predecessor']['prior_pixel_bytes'] == 123767, 'Successor/cumulative budget differs')
    controls = data('gpu-comparator-controls-001.json')
    require(len(controls['controls']) == 6 and all(r['passed'] for r in controls['controls']),
            'Pure comparison controls differ')
    # Import only the saved pure comparator. Its engine launch lives behind main().
    with tempfile.TemporaryDirectory(prefix='incant-material-replay-') as temporary:
        path = Path(temporary) / 'saved_compare.py'
        path.write_bytes(raw['gpu-supervisor-002.py'])
        tree = ast.parse(path.read_text())
        function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'assess')
        require(sha(ast.dump(function, include_attributes=False).encode()) == controls['comparator_AST_sha256'],
                'Comparator semantics changed')
        spec = importlib.util.spec_from_file_location('saved_material_compare', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        frames = {name: raw['gpu-002/pixels/' + name + '.rgba'] for name in module.STAGES}
        compared = module.assess(frames, data('gpu-002/pixels/result.json'))
        require(compared == gpu['comparison'] and compared['covered_primary_pixels'] == 612,
                'Exact pixel replay differs')
    print(f"PASS {len(raw)} records; 37 runtime/Play +1 fixture; both failures and GPU001 retained; "
          "Metal612 pixels/7 frames/VM continuity; no window/appearance/performance claim")


if __name__ == '__main__':
    main()
