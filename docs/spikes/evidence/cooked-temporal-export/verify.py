#!/usr/bin/env python3
"""Offline fixed motion/history evidence replay; no compiler, player or GPU."""
import argparse
import base64
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    source_root = args.source_root.resolve() if args.source_root else here.parents[3]
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) <= 32 * 1024**2, 'Archive exceeds bound')
    encoded = json.loads(payload)
    require(len(encoded) == len(manifest['records']) and set(encoded) == {r['key'] for r in manifest['records']}, 'Record membership differs')
    raw = {}
    for row in manifest['records']:
        data = base64.b64decode(encoded[row['key']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original differs: ' + row['key'])
        if row.get('code_input'):
            if args.source_root:
                require((source_root / row['source_path']).read_bytes() == data, 'Live source differs: ' + row['source_path'])
            if args.git_source:
                actual = subprocess.run(['git', 'show', args.git_source + ':' + row['source_path']], cwd=source_root, check=True, capture_output=True).stdout
                require(actual == data, 'Git source differs: ' + row['source_path'])
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['path'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'], 'Binary differs: ' + row['path'])

    def data(key):
        return json.loads(raw[key])

    freeze = data('source-003/freeze.json')
    require(freeze['base'] == manifest['native_source'] and len(freeze['files']) == 163 and not freeze['untracked_paths'], 'Native source scope differs')
    require(sha(raw['source-003/source.diff']) == freeze['review_patch_sha256'], 'Review patch differs')
    for row in freeze['files']:
        require(len(raw['source/' + row['path']]) == row['bytes'] and sha(raw['source/' + row['path']]) == row['sha256'], 'Frozen source differs: ' + row['path'])
    successor = data('final-source.json')
    require(successor['native_source'] == manifest['native_source'] and successor['python_source'] == manifest['python_source'], 'Source identity differs')
    require(successor['changed_code_paths'] == ['tools/temporal_parity.py', 'tools/tests/test_temporal_parity.py'], 'Native executable input changed')
    require(sha(raw['gpu001-diagnosis-001/proposed-comparator.diff']) == data('gpu001-diagnosis-001/proposal.json')['patch_sha256'], 'Comparator proposal differs')

    cpu = data('cpu-003/results.json')
    require(cpu['source'] == manifest['native_source'] and cpu['status'] == 'complete' and cpu['exit'] == 0 and cpu['supervisor_terminal'], 'CPU result differs')
    require(cpu['proposal_sha256'] == sha(raw['cpu-proposal-003.json']) and cpu['supervisor_sha256'] == sha(raw['cpu-supervisor-003.py']), 'CPU protocol differs')
    require(cpu['native_engine_launches'] == 0 and not cpu['gpu_window_capture'], 'Unexpected CPU scope')
    require(all(r['exit'] == 0 and r['terminal_reaped'] and r['cleanup_confirmed'] for r in cpu['tool_probes'] + cpu['results'] if 'argv' in r), 'CPU child unfinished')
    require(all(g['cleanup_confirmed'] for g in cpu['owned_groups']) and data('cpu003-closeout.json')['independent_ps_all_absent'], 'CPU cleanup unconfirmed')
    for name, row in cpu['logs'].items():
        require(len(raw['cpu-003/' + name]) == row['bytes'] and sha(raw['cpu-003/' + name]) == row['sha256'], 'CPU log differs')
    def counts(name):
        pairs = re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw[name])
        return sum(int(a) for a, _ in pairs), sum(int(b) for _, b in pairs)
    require(counts('cpu-003/0.stdout.log') == (48, 0), 'Cooker coverage differs')
    require(counts('cpu-003/1.stdout.log') == (186, 57) and counts('cpu-003/2.stdout.log') == (218, 57), 'Renderer coverage differs')
    require(counts('cpu-003/3.stdout.log') == (1, 0) and counts('cpu-003/4.stdout.log') == (1, 0), 'Partial refusal coverage differs')
    require(cpu['final_target_bytes'] - cpu['initial_target_bytes'] == 165512928 and cpu['final_target_bytes'] - cpu['initial_target_bytes'] <= 4 * 1024**3, 'Target growth differs')
    require(b'Ran 32 tests' in raw['pure-controls-002.log'] and b'Ran 37 tests' in raw['gpu001-diagnosis-001/final-python-controls.log'], 'Pure control coverage differs')

    gpu = data('gpu-001/results.json')
    require(gpu['status'] == 'failed' and gpu['exit'] == 1 and gpu['error'] == "ValueError('Stationary history missing/changed')", 'Original GPU failure was lost')
    require(gpu['plan_sha256'] == sha(raw['gpu-proposal-002.json']) and gpu['supervisor_sha256'] == sha(raw['gpu-supervisor-002.py']), 'GPU protocol differs')
    require(gpu['supervisor_terminal'] and len(gpu['arms']) == 6 and not gpu['capture'] and gpu['native_player_window_launches'] == 0, 'GPU scope differs')
    require(all(g['cleanup_confirmed'] for g in gpu['owned_groups']) and data('gpu001-closeout.json')['independent_ps_all_absent'], 'GPU cleanup unconfirmed')
    plan = data('gpu-proposal-002.json')
    for index, arm in enumerate(gpu['arms']):
        require(arm['status'] == 'complete' and arm['exit'] == 0 and arm['cleanup_confirmed'], 'Native GPU child failed')
        require(all(arm[k] == v for k, v in plan['arms'][index].items()), 'GPU child binding differs')
        require(b'test result: ok. 1 passed; 0 failed; 0 ignored;' in raw[f'gpu-001/{index}.stdout.log'] and b'backend: Metal' in raw[f'gpu-001/{index}.stderr.log'] and b'Apple M5 Pro' in raw[f'gpu-001/{index}.stderr.log'], 'Actual Metal control missing')
        if arm['role'] == 'cluster':
            require(len(arm['assignment_intervals']) == 4 and all(r['start_ticks'] < r['end_ticks'] and r['nanoseconds'] > 0 for r in arm['assignment_intervals']), 'Actual jittered light-assignment dispatch missing')
    for name, row in gpu['retained_files'].items():
        require(len(raw['gpu-001/' + name]) == row['bytes'] and sha(raw['gpu-001/' + name]) == row['sha256'], 'GPU retained file differs')

    # Relocate only saved inputs/generated outputs. Imports execute the reviewed
    # pure audit/comparator, never a compiler, native binary, device or window.
    with tempfile.TemporaryDirectory(prefix='incant-temporal-evidence-') as temporary:
        root = Path(temporary).resolve()
        generated = root / 'target/aarch64-apple-darwin/ci/build/renderer/out'
        for row in manifest['records']:
            key = row['key']
            if key.startswith('source/'):
                path = root / key.removeprefix('source/')
            elif key.startswith('generated/'):
                path = generated / key.removeprefix('generated/')
            elif key.startswith('gpu-001/'):
                path = root / key
            else:
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw[key])
        sys.path.insert(0, str(root / 'tools'))
        audit = module('incant_temporal_saved_audit', root / 'tools/native_shaders.py')
        messages = [json.loads(line) for line in raw['cpu-003/7.stdout.log'].splitlines() if line.startswith(b'{')]
        target = data('cpu-proposal-003.json')['target']
        renderer = next(m['package_id'] for m in messages if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'incant_render')
        for message in messages:
            if message.get('reason') == 'compiler-artifact':
                message['filenames'] = [str(root / 'target' / Path(p).relative_to(target)) for p in message['filenames']]
            if message.get('reason') == 'build-script-executed' and message.get('package_id') == renderer:
                message['out_dir'] = str(generated)
        actual = audit.audit(messages, 'cooked', target_dir=root / 'target', target='aarch64-apple-darwin', profile='ci', physics_mode='fast', root=root)
        require(actual == data('cpu-003/actual-shaders.json'), 'Actual target-output audit differs')
        require(len(actual['families']) == 23 and sum(len(v['artifacts']) for v in actual['families'].values()) == 132 and sum(v['artifact_bytes'] for v in actual['families'].values()) == 573004, 'Catalog differs')
        paths = [root / 'gpu-001' / f'{mode}-{kind}' for kind in ('owner', 'resolve', 'cluster') for mode in ('default', 'aggregate')]
        original = module('incant_temporal_original_compare', root / 'tools/temporal_parity.py')
        try:
            original.compare(*paths)
        except ValueError as error:
            require(str(error) == 'Stationary history missing/changed', 'Original comparator failed differently')
        else:
            raise ValueError('Original failing assertion no longer reproduced')
        (root / 'tools/temporal_parity.py').write_bytes(raw['comparator/tools/temporal_parity.py'])
        corrected = module('incant_temporal_reviewed_compare', root / 'tools/temporal_parity.py')
        replay = corrected.compare(*paths)
        require(replay == data('gpu001-diagnosis-001/replay-001.json'), 'Reviewed saved-byte replay differs')
        require(replay['tolerance_codes'] == 0 and replay['stationary_motion_precision_bound'] is None and replay['nonzero_motion_pixels'] == [18, 7], 'Parity/state witness differs')
        require(data('gpu001-diagnosis-001/exact-pairs.json')['pair_count'] == 60, 'Independent pair replay differs')
    print(f'PASS {len(raw)} records; cooker48/default186/full218/partial1+1;23families/132artifacts;6Metal children; original GPU wrapper FAIL retained;60exact pairs/reviewed saved replay PASS; no player/window/capture or gate claim')


if __name__ == '__main__':
    main()
