#!/usr/bin/env python3
"""Offline CPU-evidence replay; no compiler, player, GPU or capture launch."""
import argparse
import base64
import gzip
import hashlib
import importlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'Archive hash differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) <= 32 * 1024**2, 'Archive exceeds replay bound')
    records = json.loads(payload)
    require(set(records) == {row['key'] for row in manifest['records']}, 'Record membership differs')
    raw = {}
    source_root = args.source_root.resolve() if args.source_root else here.parents[3]
    for row in manifest['records']:
        data = base64.b64decode(records[row['key']], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original differs: ' + row['key'])
        if row.get('code_input'):
            if args.source_root:
                require((source_root / row['source_path']).read_bytes() == data,
                        'Live source differs: ' + row['source_path'])
            if args.git_source:
                result = subprocess.run(['git', 'show', args.git_source + ':' + row['source_path']],
                                        cwd=source_root, check=True, capture_output=True)
                require(result.stdout == data, 'Git source differs: ' + row['source_path'])
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'], 'Binary size differs')
            require(digest(path.read_bytes()) == row['sha256'], 'Binary hash differs')

    def data(key):
        return json.loads(raw[key])

    for index in (1, 2, 3):
        receipt = data(f'cpu-00{index}/results.json')
        require(receipt['source'] == manifest['source'] and receipt['supervisor_terminal'], 'CPU identity/cleanup differs')
        require(receipt['native_engine_launches'] == 0 and not receipt['gpu_window_capture'], 'Unexpected native execution')
        plan = f'cpu-proposal-00{index}.json'
        runner = f'cpu-supervisor-00{index}.py'
        require(receipt['proposal_sha256'] == digest(raw[plan]), 'Plan binding differs')
        require(receipt['supervisor_sha256'] == digest(raw[runner]), 'Supervisor binding differs')
        for name, row in receipt['logs'].items():
            log = raw[f'cpu-00{index}/' + name]
            require(len(log) == row['bytes'] and digest(log) == row['sha256'], 'CPU log binding differs')
        for child in receipt['results']:
            if 'argv' not in child:
                continue
            if index == 2 and child.get('error') == 'Storage stop threshold':
                # Cancellation raises after managed cleanup, so this retained
                # attempt has no child return code/terminal flag to relabel.
                require(child['status'] == 'failed' and 'exit' not in child
                        and 'terminal_reaped' not in child, 'Storage-stop schema differs')
            else:
                require(child.get('terminal_reaped'), 'Unreaped child')
    one, two, three = (data(f'cpu-00{i}/results.json') for i in (1, 2, 3))
    require(one['exit'] == 1 and [r['exit'] for r in one['results']] == [0, 101], 'Original fixture failure missing')
    require(b'185 passed; 3 failed; 54 ignored' in raw['cpu-001/1.stdout.log']
            and raw['cpu-001/1.stdout.log'].count(b'requested: Deterministic, compiled: Fast') == 3,
            'Original failure diagnosis differs')
    require(two['exit'] == 1 and two['error'] == "ValueError('Storage stop threshold')", 'Storage refusal missing')
    require(two['results'][0]['exit'] == 0 and b'188 passed; 0 failed; 54 ignored' in raw['cpu-002/0.stdout.log'],
            'Corrected renderer result differs')
    require(three['exit'] == 0 and three['status'] == 'complete'
            and all(r['status'] == 'complete' and r.get('exit', 0) == 0 for r in three['results']), 'CPU continuation differs')
    require(three['final_target_bytes'] - three['initial_target_bytes'] == 2856601093
            and three['final_target_bytes'] - three['initial_target_bytes'] <= three['bounds']['target_growth_bytes'],
            'Cumulative resource bound differs')
    require(three['minimum_sampled_free_bytes'] >= three['bounds']['free_floor_bytes'], 'Free-space floor violated')
    cooker = re.findall(rb'test result: ok\. (\d+) passed; 0 failed;', raw['cpu-001/0.stdout.log'])
    require(sum(map(int, cooker)) == 36, 'Cooker count differs')
    counts = [int(re.search(rb'Ran (\d+) tests', raw[f'pure-003/{i}.raw.log'])[1]) for i in range(5)]
    require(counts == [19, 3, 6, 2, 9] and all(r['exit'] == 0 for r in data('pure-003/results.json')),
            'Python coverage differs')
    closeout = data('cpu-003/closeout.json')
    require(closeout['target_released'] and closeout['supervisor_terminal_reaped']
            and closeout['matching_live_owners'] == [], 'Final cleanup differs')

    # Relocate only compiler-output absolute paths into a private data tree. This
    # reuses the exact frozen audit implementation; it launches no native code.
    with tempfile.TemporaryDirectory(prefix='incant-cooked-evidence-') as temporary:
        root = Path(temporary).resolve()
        for row in manifest['records']:
            if row.get('source_path'):
                path = root / row['source_path']
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw[row['key']])
            elif row['key'].startswith('cpu-003/generated/'):
                path = root / 'target/aarch64-apple-darwin/ci/build/renderer/out' / row['key'].removeprefix('cpu-003/generated/')
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw[row['key']])
        sys.path.insert(0, str(root / 'tools'))
        audit = importlib.import_module('native_shaders')
        messages = [json.loads(line) for line in raw['cpu-003/0.stdout.log'].splitlines()]
        original_target = data('cpu-proposal-003.json')['target']
        renderer = next(r['package_id'] for r in messages if r.get('reason') == 'compiler-artifact'
                        and r['target']['name'] == 'incant_render')
        for row in messages:
            if row.get('reason') == 'compiler-artifact':
                row['filenames'] = [str(root / 'target' / Path(p).relative_to(original_target)) for p in row['filenames']]
            if row.get('reason') == 'build-script-executed' and row.get('package_id') == renderer:
                row['out_dir'] = str(root / 'target/aarch64-apple-darwin/ci/build/renderer/out')
        actual = audit.audit(messages, 'cooked', target_dir=root / 'target', target='aarch64-apple-darwin',
                             profile='ci', physics_mode='fast', root=root)
        require(actual == data('cpu-003/actual-shaders.json'), 'Actual family audit differs')
        families = actual['families']
        require(len(families) == 8 and sum(len(f['artifacts']) for f in families.values()) == 45
                and sum(f['artifact_bytes'] for f in families.values()) == 198123, 'Catalog coverage differs')
    print(f"PASS {len(records)} records; Python39/cooker36/renderer188; retained fixture+storage failures; "
          'actual8 families/45 artifacts; player compile-only; zero native/GPU/window/capture runs')


if __name__ == '__main__':
    main()
