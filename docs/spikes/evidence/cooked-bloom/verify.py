#!/usr/bin/env python3
"""Offline bloom CPU evidence replay; never starts a compiler/player/device."""
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


def require(ok, message):
    if not ok:
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
    require(digest(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) <= 32 * 1024**2, 'Replay archive exceeds bound')
    records = json.loads(payload)
    require(set(records) == {r['key'] for r in manifest['records']}, 'Record membership differs')
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
            require(path.stat().st_size == row['bytes'] and digest(path.read_bytes()) == row['sha256'],
                    'Retained executable differs: ' + row['path'])

    def data(key):
        return json.loads(raw[key])

    receipts = {}
    for index in (3, 4, 5):
        prefix = f'cpu-00{index}/'
        receipt = receipts[index] = data(prefix + 'results.json')
        plan = data(f'cpu-proposal-00{index}.json')
        require(receipt['source'] == plan['source'] and receipt['supervisor_terminal'], 'CPU source/cleanup differs')
        require(receipt['proposal_sha256'] == digest(raw[f'cpu-proposal-00{index}.json'])
                and receipt['supervisor_sha256'] == digest(raw[f'cpu-supervisor-00{index}.py']), 'CPU protocol differs')
        require(receipt['native_engine_launches'] == 0 and not receipt['gpu_window_capture'], 'Unexpected execution')
        for name, row in receipt['logs'].items():
            require(len(raw[prefix + name]) == row['bytes'] and digest(raw[prefix + name]) == row['sha256'],
                    'Raw CPU log differs')
        require(all(r.get('terminal_reaped') for r in receipt['tool_probes']), 'Unreaped tool probe')
        require(all(r.get('terminal_reaped') for r in receipt['results'] if 'argv' in r), 'Unreaped CPU child')
        require(receipt['final_free_bytes'] >= receipt['bounds']['free_floor_bytes'], 'Free floor violated')
    require([r['exit'] for r in receipts[3]['results']] == [101]
            and b'2 passed; 1 failed;' in raw['cpu-003/0.stdout.log']
            and b'bloom.rs:108' in raw['cpu-003/0.stdout.log'], 'Original HLSL assertion failure missing')
    require([r['exit'] for r in receipts[4]['results']] == [0, 101], 'Visibility failure chronology differs')
    require(b'function `display` is private' in raw['cpu-004/1.stdout.log'], 'Original visibility error missing')
    final = receipts[5]
    require(final['source'] == manifest['source'] and final['status'] == 'complete' and final['exit'] == 0
            and all(r['status'] == 'complete' and r.get('exit', 0) == 0 for r in final['results']), 'Final CPU failed')
    for index in (4, 5):
        passed = re.findall(rb'test result: ok\. (\d+) passed; 0 failed;', raw[f'cpu-00{index}/0.stdout.log'])
        require(sum(map(int, passed)) == 41, 'Actual cooker coverage differs')
    require(b'209 passed; 0 failed; 54 ignored' in raw['cpu-005/1.stdout.log']
            and b'12 passed; 0 failed; 3 ignored; 0 measured; 223 filtered out' in raw['cpu-005/2.stdout.log'],
            'Actual renderer/refusal coverage differs')
    require(all(data(f'cpu-proposal-00{i}.json')['steps'] == data('cpu-proposal-003.json')['steps']
                for i in (4, 5)), 'Successor changed qualified command sequence')
    require(final['final_target_bytes'] - receipts[3]['initial_target_bytes'] == 221256083
            and final['final_target_bytes'] - receipts[3]['initial_target_bytes'] <= 4 * 1024**3,
            'Cumulative target bound differs')
    closeout = data('cpu-005/closeout.json')
    require(closeout['terminal_reaped'] and closeout['resources']['CORE_target_and_CPU_slot_released']
            and closeout['resources']['matching_live_target_owners'] == [], 'Final ownership differs')
    counts = [int(re.search(rb'Ran (\d+) tests', raw[f'python-004-{i}.log'])[1]) for i in range(5)]
    require(counts == [21, 3, 6, 2, 9] and all(r['exit'] == 0 for r in data('python-004-results.json')),
            'Actual Python coverage differs')

    # Relocate compiler-output paths only; replay the exact trusted audit using
    # retained generated bytes and frozen source, without native execution.
    with tempfile.TemporaryDirectory(prefix='incant-bloom-evidence-') as temporary:
        root = Path(temporary).resolve()
        generated = root / 'target/aarch64-apple-darwin/ci/build/renderer/out'
        for row in manifest['records']:
            if row.get('source_path'):
                path = root / row['source_path']
            elif row['key'].startswith('cpu-005/generated/'):
                path = generated / row['key'].removeprefix('cpu-005/generated/')
            else:
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw[row['key']])
        sys.path.insert(0, str(root / 'tools'))
        audit = importlib.import_module('native_shaders')
        messages = [json.loads(line) for line in raw['cpu-005/4.stdout.log'].splitlines() if line.startswith(b'{')]
        target = data('cpu-proposal-005.json')['target']
        renderer = next(m['package_id'] for m in messages if m.get('reason') == 'compiler-artifact'
                        and m['target']['name'] == 'incant_render')
        for row in messages:
            if row.get('reason') == 'compiler-artifact':
                row['filenames'] = [str(root / 'target' / Path(p).relative_to(target)) for p in row['filenames']]
            if row.get('reason') == 'build-script-executed' and row.get('package_id') == renderer:
                row['out_dir'] = str(generated)
        actual = audit.audit(messages, 'cooked', target_dir=root / 'target', target='aarch64-apple-darwin',
                             profile='ci', physics_mode='fast', root=root)
        require(actual == data('cpu-005/actual-shaders.json'), 'Actual generated audit differs')
        families = actual['families']
        require(len(families) == 12 and sum(len(f['artifacts']) for f in families.values()) == 69
                and sum(f['artifact_bytes'] for f in families.values()) == 268438, 'Catalog coverage differs')
        for row in closeout['bloom_actual_emitted_hlsl']:
            shader = generated / ('bloom-' + row['family'] + '-shaders') / ('Dx12-' + row['stage'] + '.shader')
            require(digest(shader.read_bytes()) == row['sha256'], 'Actual HLSL differs')
            lines = shader.read_text().splitlines()
            uniforms = [s.split(' : ', 1)[1].split(' { ', 1)[0] for s in lines if s.startswith('cbuffer ')]
            special = [s for s in lines if s.startswith('ConstantBuffer<')]
            require(uniforms == row['uniform_cbvs'] and special == row['special_constants'], 'HLSL ABI differs')
    print(f'PASS {len(records)} records; Python41/cooker41/renderer209+partial12; retained two CPU failures; '
          'actual12 families/69 artifacts/268438 bytes; player compile-only; zero GPU/window/capture runs')


if __name__ == '__main__':
    main()
