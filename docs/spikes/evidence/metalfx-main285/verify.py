#!/usr/bin/env python3
"""Offline receipt/readback replay. Never builds or runs a renderer/player."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_text())
    assert sha(Path(__file__).read_bytes()) == manifest['verifier_sha256']
    archive = here / 'records.json.gz'
    assert sha(archive.read_bytes()) == manifest['archive_sha256']
    with gzip.open(archive, 'rb') as file:
        payload = file.read(32 * 1024 * 1024 + 1)
    assert len(payload) <= 32 * 1024 * 1024
    rows = json.loads(payload)
    assert len(rows) == len(manifest['records'])
    data = {}
    for row, identity in zip(rows, manifest['records']):
        assert {key: row[key] for key in ('path', 'bytes', 'sha256')} == identity
        name = row['path']
        assert name not in data and not Path(name).is_absolute() and '..' not in Path(name).parts
        value = base64.b64decode(row['data'], validate=True)
        assert len(value) == row['bytes'] and sha(value) == row['sha256']
        data[name] = value
        if args.originals:
            assert (Path(manifest['external_root']) / name).read_bytes() == value, name
    def doc(name):
        return json.loads(data[name])
    for generation in ('001', '002', '003'):
        freeze = doc(f'source-{generation}/freeze.json')
        for row in freeze['files']:
            value = data[f"source-{generation}/inputs/{row['path']}"]
            assert len(value) == row['bytes'] and sha(value) == row['sha256']
    final = doc('source-003/freeze.json')
    assert final['changed_from_source002'] == [
        'crates/incant_render/src/metalfx/owner.rs',
        'crates/incant_render/src/metalfx/prewarm_tests.rs',
        'crates/incant_render_gpu_host/src/lib.rs',
        'crates/incant_render_gpu_host/src/metalfx_tests.rs',
    ]
    for row in doc('source-002/freeze.json')['files']:
        if row['path'] not in final['changed_from_source002']:
            assert data['source-002/inputs/' + row['path']] == data['source-003/inputs/' + row['path']]
    old, current = doc('qualify-001/result.json'), doc('qualify-003/result.json')
    assert old['outcome'] == 'failed'
    assert [(v['name'], v['exit_code']) for v in old['commands']] == [
        ('native-lib', 0), ('default-lib', 0), ('cooked-native', 0), ('native-clippy', 101)]
    assert current['outcome'] == 'passed' and len(current['commands']) == 6
    assert all(v['exit_code'] == 0 for v in current['commands'])
    for generation, result in [('001', old), ('003', current)]:
        assert result['source_freeze_sha256'] == sha(data[f'source-{generation}/freeze.json'])
        assert result['plan_sha256'] == sha(data[f'plan-{generation}.json'])
        for command in result['commands']:
            for stream in ('stdout', 'stderr'):
                assert sha(data[f"qualify-{generation}/{command['name']}.{stream}"]) == command[stream + '_sha256']
    for name, counts in [('native-lib', [174, 4]), ('default-lib', [173, 4])]:
        text = data[f'qualify-003/{name}.stdout'].decode()
        assert [int(n) for n in re.findall(r'test result: ok\. (\d+) passed;', text)] == counts
    gpu = doc('gpu-003/result.json')
    plan = doc('gpu-plan-003.json')
    assert gpu['outcome'] == 'passed' and len(gpu['children']) == 5
    assert gpu['appearance_acceptance'] is False and gpu['performance_acceptance'] is False
    assert sha(data['gpu-plan-003.json']) == gpu['plan_sha256']
    assert sha(data['run-gpu-003.py']) == gpu['runner_sha256']
    assert sha(data['source-003/freeze.json']) == gpu['source_freeze_sha256']
    grant = doc('root-gpu-grant-003.json')
    assert grant['authorized'] is True and grant['exclusive_gpu_lease'] is True
    assert grant['plan_sha256'] == gpu['plan_sha256'] and grant['runner_sha256'] == gpu['runner_sha256']
    assert grant['source_freeze_sha256'] == gpu['source_freeze_sha256']
    for child, declared in zip(gpu['children'], plan['sequence']):
        assert child['exit_code'] == 0 and child['test'] == declared['exact_test']
        assert child['binary_sha256'] == child['binary_sha256_after'] == declared['binary']['sha256']
        folder = f"gpu-003/{child['index']:02d}"
        for stream in ('stdout', 'stderr'):
            assert sha(data[f'{folder}/{stream}.log']) == child[stream + '_sha256']
        assert b'test result: ok. 1 passed; 0 failed; 0 ignored;' in data[folder + '/stdout.log']
    assert sum(c['declared_queue_submit_calls'] for c in gpu['children']) == 9
    assert sum(c['declared_maps'] for c in gpu['children']) == 1
    assert sum(c['declared_raw_native_encodes'] for c in gpu['children']) == 10
    boundary = 'gpu-003/01/boundary/'
    report, raw = doc(boundary + 'report.json'), data[boundary + 'raw.bin']
    names = ['opaque', 'clear', 'partial', 'silhouette', 'mixed-depth-full-coverage', 'fresh',
             'invalid-motion', 'native-nan', 'native-inf', 'native-negative-inf', 'current-nan',
             'current-inf', 'motion-nan', 'motion-inf', 'motion-overflow', 'static-jitter', 'moving-jitter']
    assert report['cases'] == names and 'backend: Metal' in report['adapter']
    assert len(raw) == 87040 and sha(raw) == report['raw_sha256']
    src = 'source-003/inputs/crates/incant_render/src/metalfx/'
    for name, leaf, key in [('pack.wgsl', 'prepare.wgsl', 'pack_sha256'),
                            ('normalize.wgsl', 'normalize.wgsl', 'normalize_sha256')]:
        assembled = b'\n'.join(data[src + f] for f in ['common.wgsl', 'alpha.wgsl', leaf])
        assert data[boundary + name] == assembled and sha(assembled) == report[key]
    ordinary = (.5, .25, .125, 1.)
    empty = {'clear', 'current-nan', 'current-inf'}
    native_invalid = {'native-nan', 'native-inf', 'native-negative-inf'}
    reactive = empty | {'partial', 'fresh', 'invalid-motion', 'motion-nan', 'motion-inf', 'motion-overflow'}
    for index, name in enumerate(names):
        start = index * 5120
        expected = ((0.,) * 4 if name in empty else (.25, .125, .0625, .5) if name == 'partial'
                    else tuple(v * 9 / 16 for v in ordinary) if name == 'silhouette'
                    else ordinary if name in native_invalid | {'mixed-depth-full-coverage'}
                    else (4., 2., 1., 1.))
        assert struct.unpack_from('<4e', raw, start + 4096 + 264) == expected, name
        packed = name not in empty | {'partial'}
        assert struct.unpack_from('<4e', raw, start) == (ordinary if packed else (0.,) * 4), name
        assert struct.unpack_from('<f', raw, start + 1024)[0] == (.25 if packed else 1.), name
        assert struct.unpack_from('<2e', raw, start + 2048) == ((.5, -1.) if name == 'moving-jitter' else (0., 0.)), name
        assert raw[start + 3072] == (255 if name in reactive else 0), name
        for y in range(4):
            for x in range(4):
                rgba = struct.unpack_from('<4e', raw, start + 4096 + y * 256 + x * 8)
                assert all(0 <= v <= 65504 for v in rgba) and rgba[3] <= 1
                assert rgba[3] != 0 or rgba[:3] == (0., 0., 0.)
    if args.binaries:
        for binary in manifest['binaries']:
            path = Path(binary['path'])
            assert path.stat().st_size == binary['bytes'] and sha(path.read_bytes()) == binary['sha256']
    if args.git_source:
        commit = manifest['source_commit']
        assert re.fullmatch('[0-9a-f]{40}', commit)
        inventory = doc('source-003/source-inventory.json')
        assert len(inventory) == 3794
        assert sum(r['bytes'] for r in inventory) < 64 * 1024 * 1024
        query = ''.join(commit + ':' + r['path'] + '\n' for r in inventory).encode()
        result = subprocess.run(['git', 'cat-file', '--batch'], input=query, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, check=True, timeout=60)
        offset = 0
        for row in inventory:
            end = result.stdout.index(b'\n', offset)
            header = result.stdout[offset:end].split()
            assert len(header) == 3 and header[1] == b'blob' and int(header[2]) == row['bytes'], row['path']
            offset = end + 1
            value = result.stdout[offset:offset + row['bytes']]
            assert sha(value) == row['sha256'], row['path']
            offset += row['bytes']
            assert result.stdout[offset:offset + 1] == b'\n'
            offset += 1
        assert offset == len(result.stdout)
    print(f"PASS {len(rows)} records, source003 / 3794 compiler inputs, 6 CPU commands, 5 GPU children, 17 raw boundary cases; no appearance/performance verdict")


if __name__ == '__main__':
    main()
