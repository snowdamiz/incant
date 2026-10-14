#!/usr/bin/env python3
"""Read-only negative-result replay; never builds, executes a test, or grants quality."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import zlib


def sha(value):
    return hashlib.sha256(value).hexdigest()


def png_rgba(value):
    assert value[:8] == b'\x89PNG\r\n\x1a\n'
    offset, chunks, header = 8, [], None
    while offset < len(value):
        length = int.from_bytes(value[offset:offset + 4], 'big')
        kind = value[offset + 4:offset + 8]
        body = value[offset + 8:offset + 8 + length]
        assert len(body) == length
        assert zlib.crc32(kind + body) == int.from_bytes(value[offset + 8 + length:offset + 12 + length], 'big')
        offset += length + 12
        if kind == b'IHDR':
            assert header is None
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            chunks.append(body)
        elif kind == b'IEND':
            assert length == 0 and offset == len(value)
            break
    assert header == (960, 540, 8, 6, 0, 0, 0)
    decoder = zlib.decompressobj()
    packed = decoder.decompress(b''.join(chunks), 540 * (960 * 4 + 1) + 1)
    assert decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail
    assert len(packed) == 540 * (960 * 4 + 1)
    result, previous, stride = bytearray(), bytearray(960 * 4), 960 * 4
    for y in range(540):
        kind = packed[y * (stride + 1)]
        row = bytearray(packed[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        assert kind in range(5)
        if kind:
            for x in range(stride):
                left, up, corner = (row[x - 4] if x >= 4 else 0), previous[x], (previous[x - 4] if x >= 4 else 0)
                if kind == 1:
                    predictor = left
                elif kind == 2:
                    predictor = up
                elif kind == 3:
                    predictor = (left + up) // 2
                else:
                    p = left + up - corner
                    a, b, c = abs(p - left), abs(p - up), abs(p - corner)
                    predictor = left if a <= b and a <= c else up if b <= c else corner
                row[x] = (row[x] + predictor) & 255
        result.extend(row)
        previous = row
    return bytes(result)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_text())
    assert sha(Path(__file__).read_bytes()) == manifest['verifier_sha256']
    archive = (here / 'records.json.gz').read_bytes()
    assert sha(archive) == manifest['archive_sha256']
    with gzip.open(here / 'records.json.gz', 'rb') as file:
        payload = file.read(64 * 1024 * 1024 + 1)
    assert len(payload) <= 64 * 1024 * 1024
    rows = json.loads(payload)
    assert len(rows) == len(manifest['records'])
    data = {}
    for row, identity in zip(rows, manifest['records']):
        assert {k: row[k] for k in ('path', 'bytes', 'sha256')} == identity
        name = row['path']
        assert name not in data and not Path(name).is_absolute() and '..' not in Path(name).parts
        value = base64.b64decode(row['data'], validate=True)
        assert len(value) == row['bytes'] and sha(value) == row['sha256'], name
        data[name] = value
        if args.originals:
            assert (Path(manifest['external_root']) / name).read_bytes() == value, name
    def doc(name):
        return json.loads(data[name])
    for generation in ('001', '002'):
        freeze = doc(f'source-{generation}/freeze.json')
        for row in freeze['files']:
            value = data[f"source-{generation}/inputs/{row['path']}"]
            assert len(value) == row['bytes'] and sha(value) == row['sha256']
        assert sha(data[f'source-{generation}/source-inventory.json']) == freeze['source_inventory_sha256']
        result = doc(f'qualify-{generation}/result.json')
        assert result['source_freeze_sha256'] == sha(data[f'source-{generation}/freeze.json'])
        assert result['plan_sha256'] == sha(data[f'plan-{generation}.json'])
        assert result['runner_sha256'] == sha(data[f'qualify-{generation}.py'])
        assert result['native_process_sha256'] == sha(data['native_process.py'])
        assert result['processes_returned_after_cleanup'] and not result['gpu_execution']
        for command in result['commands']:
            for stream in ('stdout', 'stderr'):
                assert sha(data[f"qualify-{generation}/{command['name']}.{stream}"]) == command[stream + '_sha256']
    failed, cpu = doc('qualify-001/result.json'), doc('qualify-002/result.json')
    assert failed['outcome'] == 'failed' and [(c['name'], c['exit_code']) for c in failed['commands']] == [('image-cpu', 101)]
    assert b'E0432' in data['qualify-001/image-cpu.stdout']
    assert cpu['outcome'] == 'passed' and [(c['name'], c['exit_code']) for c in cpu['commands']] == [('image-cpu', 0), ('native-clippy', 0)]
    assert b'test result: ok. 2 passed; 0 failed; 1 ignored;' in data['qualify-002/image-cpu.stdout']
    for row in doc('source-002/freeze.json')['files']:
        if row['path'] != 'crates/incant_render/src/lib.rs':
            assert data['source-002/inputs/' + row['path']] == data['source-001/inputs/' + row['path']]
    old = data['source-001/inputs/crates/incant_render/src/lib.rs']
    new = data['source-002/inputs/crates/incant_render/src/lib.rs']
    addition = b'#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]\nextern crate self as incant_render;\n'
    assert new.count(addition) == 1 and new.replace(addition, b'', 1) == old
    gpu, plan, grant = doc('gpu-001/result.json'), doc('gpu-plan.json'), doc('root-gpu-grant-001.json')
    assert gpu['outcome'] == 'passed' and gpu['child_exit_code'] == 0
    assert grant['authorized'] and grant['exclusive_gpu_lease']
    for key, name in [('plan_sha256', 'gpu-plan.json'), ('runner_sha256', 'run-gpu.py'), ('source_freeze_sha256', 'source-002/freeze.json')]:
        assert gpu[key] == grant[key] == sha(data[name])
    assert gpu['copied_binary_sha256_after'] == plan['binary']['sha256']
    assert plan['cpu_result_sha256'] == sha(data['qualify-002/result.json'])
    assert plan['binary_list_sha256'] == sha(data['qualify-002/binary-list.txt'])
    test = b'metalfx_image::actual_full_half_three_quarter_material_images_and_motion'
    assert test + b': test' in data['qualify-002/binary-list.txt'].splitlines()
    assert b'test ' + test + b' ...' in data['gpu-001/stdout.log']
    assert b'test result: ok. 1 passed; 0 failed; 0 ignored;' in data['gpu-001/stdout.log']
    assert all(gpu[k] is False for k in ('appearance_acceptance', 'performance_acceptance', 'built_in_upscaler_acceptance'))
    for row in gpu['inventory']:
        assert sha(data['gpu-001/' + row['path']]) == row['sha256']
    report = doc('gpu-001/scene/report.json')
    budget = {'queue_submits': 111, 'maps': 15, 'native_raw_encodes': 64, 'explicit_command_buffers': 239,
              'vendor_internal_work_counted': False, 'frames_per_arm': 32, 'captures': 15, 'width': 960, 'height': 540}
    assert report['budget'] == budget and 'backend: Metal' in report['adapter']
    assert report['native_admission'] == 'Some(Ok([1.0, 3.0]))'
    frames = report['frames']
    assert [(r['arm'], r['frame']) for r in frames] == [(a, f) for a in ('full', 'mfx50', 'mfx75') for f in range(1, 33)]
    jitter = [[-.375, -.125], [.125, .375], [.375, -.375], [-.125, .125], [.375, .125], [-.125, -.375], [-.375, .375], [.125, -.125]]
    for row in frames:
        frame, arm = row['frame'], row['arm']
        assert row['input'] == frames[frame - 1]['input']
        assert row['output'] == [960, 540] and row['cut'] == (frame == 25)
        roles = {n['role']: n for n in row['input']['nodes']}
        assert set(roles) == {'static-stage', 'environment', 'camera', 'sun', 'detail-sign', 'mover'}
        assert len({n['id'] for n in roles.values()}) == 6
        assert all(not n['diagnostic'] for n in roles.values())
        camera_x = 0 if frame <= 16 else (frame - 16) * .05 if frame <= 24 else (frame + 4) * .05
        assert abs(roles['camera']['world'][12] - camera_x) < 1e-12
        assert abs(roles['mover']['world'][12] - (2.02 + max(frame - 16, 0) * .12)) < 1e-12
        if arm == 'full':
            assert row['motion'] is None and row['selection'] == 'None'
        else:
            m = row['motion']
            assert row['selection'] == 'Some(MetalFx)' and m['sequence'] == frame
            assert m['epoch'] == (1 if frame < 25 else 2) and m['previous'] == (frame not in (1, 25))
            assert m['jitter_phase'] == (frame - 1) % 8 and m['jitter_internal_pixels'] == jitter[(frame - 1) % 8]
            assert m['content'] == ([480, 270] if arm == 'mfx50' else [720, 405])
    image_names = {f'{arm}-{frame:02}.{ext}' for arm in ('full', 'mfx50', 'mfx75') for frame in (1, 16, 24, 25, 32) for ext in ('png', 'rgba8')}
    assert {r['path'] for r in report['files'] if not r['path'].startswith('fixture/')} == image_names
    for row in report['files']:
        value = data['gpu-001/scene/' + row['path']]
        assert len(value) == row['bytes'] and sha(value) == row['sha256']
        if row['path'].endswith('.png') and not row['path'].startswith('fixture/'):
            raw = data['gpu-001/scene/' + row['path'][:-4] + '.rgba8']
            assert len(raw) == 960 * 540 * 4 and png_rgba(value) == raw
            assert all(a == 255 for a in raw[3::4])
    assert data['gpu-001/scene/full-01.rgba8'] == data['gpu-001/scene/full-16.rgba8']
    review = doc('reviews/root-pixels.json')
    assert review['decision'].startswith('FAIL appearance') and len(review['reviewed_images']) == 15
    for row in review['reviewed_images']:
        assert sha(data['gpu-001/scene/' + Path(row['path']).name]) == row['sha256']
    assert manifest['appearance_acceptance'] is False and manifest['default_enabled'] is False
    if args.binaries:
        binary = plan['binary']
        value = Path(binary['path']).read_bytes()
        assert len(value) == binary['bytes'] and sha(value) == binary['sha256']
    if args.git_source:
        commit = manifest['source_commit']
        assert re.fullmatch('[0-9a-f]{40}', commit)
        inventory = doc('source-002/source-inventory.json')
        assert len(inventory) == 3798 and sum(r['bytes'] for r in inventory) < 64 * 1024 * 1024
        query = ''.join(commit + ':' + r['path'] + '\n' for r in inventory).encode()
        result = subprocess.run(['git', 'cat-file', '--batch'], input=query, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=60)
        offset = 0
        for row in inventory:
            end = result.stdout.index(b'\n', offset)
            header = result.stdout[offset:end].split()
            assert len(header) == 3 and header[1] == b'blob' and int(header[2]) == row['bytes'], row['path']
            offset = end + 1
            assert sha(result.stdout[offset:offset + row['bytes']]) == row['sha256'], row['path']
            offset += row['bytes']
            assert result.stdout[offset:offset + 1] == b'\n'
            offset += 1
        assert offset == len(result.stdout)
    print(f'PASS {len(rows)} records / 3798 compiler inputs / 2 CPU commands / 96 scene frames / 15 PNG-raw pairs; appearance FAIL, no performance acceptance')


if __name__ == '__main__':
    main()
