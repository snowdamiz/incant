#!/usr/bin/env python3
"""Replay saved reports and original PNGs; never run a player, GPU or capture API."""
import argparse
import base64
import gzip
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import zlib

def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def json_bytes(data):
    def pairs(rows):
        result = {}
        for key, value in rows:
            require(key not in result, 'Duplicate JSON member')
            result[key] = value
        return result
    def invalid(value):
        raise ValueError('Nonfinite JSON: ' + value)
    def finite(value):
        parsed = float(value)
        require(math.isfinite(parsed), 'Nonfinite JSON number')
        return parsed
    return json.loads(data, object_pairs_hook=pairs, parse_constant=invalid, parse_float=finite)


def exact(left, right):
    return json.dumps(left,sort_keys=True,allow_nan=False,separators=(',',':')) == json.dumps(right,sort_keys=True,allow_nan=False,separators=(',',':'))


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def png_rgba(data, size):
    """Decode bounded PNG RGBA8 noninterlaced bytes, including all five filters."""
    width, height = size
    require(integer(width, 1) and integer(height, 1) and max(size) <= 4096,
            'Invalid expected image dimensions')
    raw_size = width * height * 4
    require(len(data) <= raw_size + 1024**2 and data[:8] == b'\x89PNG\r\n\x1a\n', 'PNG signature/byte cap')
    position, header, ended, seen_idat, idat_closed = 8, False, False, False, False
    compressed = bytearray()
    while position < len(data):
        require(position + 12 <= len(data) and not ended, 'Truncated/trailing PNG chunk')
        count = struct.unpack_from('>I', data, position)[0]
        kind = data[position + 4:position + 8]
        require(position + 12 + count <= len(data), 'PNG chunk length')
        value = data[position + 8:position + 8 + count]
        crc = struct.unpack_from('>I', data, position + 8 + count)[0]
        require(zlib.crc32(kind + value) == crc, 'PNG CRC')
        require(header or kind == b'IHDR', 'IHDR must be first')
        if kind == b'IHDR':
            require(not header and len(value) == 13 and struct.unpack('>IIBBBBB', value)
                    == (width, height, 8, 6, 0, 0, 0), 'Expected one RGBA8 noninterlaced IHDR')
            header = True
        elif kind == b'IDAT':
            require(not idat_closed, 'Noncontiguous IDAT')
            seen_idat = True
            compressed.extend(value)
        elif kind == b'IEND':
            require(seen_idat and not value, 'PNG IEND')
            ended = True
        else:
            require(kind[0] & 32, 'Unsupported PNG critical chunk')
            if seen_idat:
                idat_closed = True
        position += count + 12
    require(header and ended and position == len(data), 'Incomplete PNG')
    expected = height * (width * 4 + 1)
    decoder = zlib.decompressobj()
    filtered = decoder.decompress(compressed, expected + 1)
    require(len(filtered) == expected and decoder.eof and not decoder.unused_data
            and not decoder.unconsumed_tail, 'Bounded PNG deflate length/termination')
    stride = width * 4
    rgba = bytearray(raw_size)
    def paeth(a, b, c):
        p = a + b - c
        pa, pb, pc = abs(p-a), abs(p-b), abs(p-c)
        return a if pa <= pb and pa <= pc else b if pb <= pc else c
    for y in range(height):
        start = y * (stride + 1)
        mode = filtered[start]
        require(mode <= 4, 'PNG filter')
        if mode == 0:
            rgba[y * stride:(y+1) * stride] = filtered[start + 1:start + 1 + stride]
            continue
        for x, v in enumerate(filtered[start + 1:start + 1 + stride]):
            at = y * stride + x
            left = rgba[at - 4] if x >= 4 else 0
            up = rgba[at - stride] if y else 0
            corner = rgba[at - stride - 4] if y and x >= 4 else 0
            predictor = left if mode == 1 else up if mode == 2 else (left+up)//2 if mode == 3 else paeth(left,up,corner)
            rgba[at] = (v + predictor) & 255
    return bytes(rgba)


def file_digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def archive(directory, manifest):
    zipped = (directory / 'records.json.gz').read_bytes()
    require(len(zipped) == manifest['archive_bytes'] and digest(zipped) == manifest['archive_sha256'], 'Archive identity')
    with gzip.open(directory / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) == manifest['decoded_archive_bytes'] <= 32 * 1024**2, 'Archive bound')
    encoded = json_bytes(payload)
    require(len(manifest['records']) == len(encoded) and set(encoded) == {r['key'] for r in manifest['records']}, 'Record membership')
    result = {}
    for row in manifest['records']:
        value = base64.b64decode(encoded[row['key']], validate=True)
        require(len(value) == row['bytes'] and digest(value) == row['sha256'], 'Record identity: ' + row['key'])
        result[row['key']] = value
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='Also rehash all retained records, 37 original PNGs and ten full PCM files')
    parser.add_argument('--binaries', action='store_true', help='Also replay the package packet external executable/data identities')
    parser.add_argument('--git-source', help='Also check all producing source hashes through the package packet')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json_bytes((here / 'manifest.json').read_bytes())
    raw = archive(here, manifest)
    data = lambda key: json_bytes(raw[key])
    packages = here.parent / 'native-player-capture-packages'
    for path, expected in manifest['package_packet'].items():
        require(file_digest(packages / path) == expected, 'Package packet changed: ' + path)
    package_manifest = json_bytes((packages / 'manifest.json').read_bytes())
    prior = archive(packages, package_manifest)
    source = '50b5408b1d6be66dbb1dcbca501d4380add9d656'
    require(manifest['producer'] == package_manifest['producer'] == source, 'Producing source')
    plan, result, life, close, review = [data(k) for k in [
        'capture-proposal-001.json', 'captures-001/result.json', 'captures-001/lifecycle.json',
        'capture-closeout-001.json', 'root-pixel-review-001.json']]
    require(plan['source'] == result['source'] == close['source'] == source, 'Actual source binding')
    require(result['passed'] is True and result['mode'] == 'capture' and result['window_or_performance_claim'] is False,
            'Actual capture outcome/scope')
    require(result['proposal_sha256'] == digest(raw['capture-proposal-001.json']), 'Frozen proposal binding')
    require(close['passed'] is True and close['wrapper_terminal_observed'] is True and close['wrapper_terminal_exit'] == 0
            and close['positive_owned_group_absence'] is True and close['actual_ps_matches'] == []
            and close['GPU_player_children'] == 5 and close['retained_PNGs'] == 37, 'Wrapper and child closure')
    require(close['all_full_reports_and_PCM_exact'] and close['all_PNG_CRC_dimensions_decoded_RGBA_receipts_exact'], 'Original numerical audit')
    require(len(life['calls']) == 5 and not life['cleanup_errors']
            and all(c['returncode'] == 0 and c['stop_returned'] and c['call_returned'] and c['pid_group_absent'] for c in life['calls']), 'Managed lifecycle')
    require(close['all_owned_PIDs'] == [result['wrapper_pid']] + [c['pid'] for c in life['calls']]
            == review['native_lifecycle']['root_independently_absent_pids_and_groups'], 'Independent ownership closeout')
    require(review['closeout_sha256'] == digest(raw['capture-closeout-001.json']), 'Pixel review closeout binding')
    require(plan['stage_seconds'] == 650 and plan['child_seconds'] == 120
            and life['elapsed_seconds'] <= 650 and life['peak_output_bytes'] <= 512 * 1024**2, 'Frozen sequence bounds')
    names = ['Core-en', 'zh-Hans', 'ar', 'en-XA', 'en-XA-narrow']
    require([c['name'] for c in plan['cases']] == [c['name'] for c in result['cases']] == names, 'Complete route membership')
    receipts = {}
    for i, case in enumerate(plan['cases']):
        name = case['name']
        report = data(f'captures-001/{name}/report.json')
        baseline = json_bytes(prior[f'baselines-001/{name}/report.json'])
        require(set(report) == set(baseline) | {'frame_capture'}
                and exact({k:v for k,v in report.items() if k != 'frame_capture'}, baseline), 'Entire CPU report parity: ' + name)
        require(report['tick'] == 2156 and report['frames'] == case['host_steps'] * 800, 'Final tick and PCM sample clocks')
        require(report['render_input']['ui']['sources'] == case['labels'] and report['render_input']['ui']['viewport'] == case['logical_viewport'], 'Authored HUD sources/extent')
        if name != 'Core-en':
            require(report['session'] == {'generation':2, 'host_tick':3056, 'simulation_tick':2156,
                    'restarts':[{'completed_generation':1,'host_step':900,'output_frames':'720000','simulation_tick':900}]}
                    and report['observations']['passed'] is True, 'Complete restarted route')
        check = data(f'captures-001/{name}/check.json')
        require(exact(check, result['cases'][i]['check']) and check['pcm_exact'] is True and check['full_report_exact'] is True
                and check['appearance_review'] is False and check['window_presentation_or_performance'] is False, 'Numerical comparator scope')
        frame_manifest = data(f'captures-001/{name}/frames/manifest.json')
        summary = report['frame_capture']
        require(frame_manifest['complete'] is True and frame_manifest['format'] == 'incant-player-frame-capture'
                and frame_manifest['version'] == 1 and exact(frame_manifest['player'], report)
                and exact(frame_manifest['capture'], summary), 'Complete manifest/report correspondence')
        require(summary == {'mode':'selected-offscreen-frames', 'on_screen':False, 'physical_size':case['physical_output'],
                'selected_host_steps':case['samples'], 'captured_frames':len(case['samples']),
                'continuous_gpu_submission':False, 'temporal_history':False,
                'adapter':{'backend':'Metal','name':'Apple M5 Pro'}}, 'Actual sampled Metal route')
        require(life['calls'][i]['argv'] == case['capture_command'], 'Actual frozen child command')
        require(len(frame_manifest['frames']) == len(case['samples']) == check['selected_frames'], 'Exact frame membership')
        require(check['decoded_rgba_sha256'] == [f['rgba_sha256'] for f in frame_manifest['frames']], 'Decoded numerical audit')
        partial = data(f'captures-001/{name}/frames/partial.json')
        require(partial['complete'] is False and partial['selected_host_steps'] == case['samples'], 'Initial create-only authority')
        cpu_pcm = captured_pcm = None
        if args.originals:
            cpu_pcm = Path(case['cpu_command'][case['cpu_command'].index('--output') + 1]).read_bytes()
            captured_pcm = Path(case['capture_command'][case['capture_command'].index('--output') + 1]).read_bytes()
            require(cpu_pcm == captured_pcm and len(cpu_pcm) == case['pcm_bytes']
                    and digest(cpu_pcm) == report['pcm_sha256'], 'Full retained PCM parity')
        for step, f in zip(case['samples'], frame_manifest['frames']):
            gen = 2 if name != 'Core-en' and step > 900 else 1
            tick = step - 900 if gen == 2 else step
            relative = f'{name}/frames/frame-{step:04}.png'
            require(f['host_step'] == step and f['generation'] == f['world_epoch'] == gen and f['simulation_tick'] == tick
                    and f['pcm_frames'] == step * 800 and f['physical_size'] == case['physical_output']
                    and f['logical_viewport'] == case['logical_viewport'] and f['ui_widgets'] > 0 and f['ui_glyph_quads'] > 0,
                    'Selected clock/extent/UI: ' + relative)
            require(f['png'] == f'frame-{step:04}.png'
                    and exact(f, data(f'captures-001/{name}/frames/frame-{step:04}.json')), 'Individual receipt correspondence')
            if args.originals:
                png = Path(plan['output']) / relative
                require(digest(png_rgba(png.read_bytes(), f['physical_size'])) == f['rgba_sha256']
                        and digest(cpu_pcm[:step * 6400]) == f['pcm_prefix_sha256'], 'Original RGBA/PCM prefix')
            receipts[relative] = f
    require(len(receipts) == 37 and len(review['images']) == len(manifest['images']) == 13, 'Sampled/reviewed frame totals')
    reviewed = {row['relative']:row for row in review['images']}
    require(len(reviewed) == 13 and {row['relative'] for row in manifest['images']} == set(reviewed), 'Exact reviewed image set')
    for row in manifest['images']:
        f = receipts[row['relative']]
        original = reviewed[row['relative']]
        png = (here / row['path']).read_bytes()
        require(row['original'] == original['path'] and row['sha256'] == original['sha256'] == f['png_sha256'] == digest(png)
                and row['bytes'] == original['bytes'] == f['png_bytes'] == len(png), 'Reviewed original PNG identity')
        require(digest(png_rgba(png, f['physical_size'])) == f['rgba_sha256'], 'Checked-in decoded RGBA identity')
    if args.originals:
        for row in manifest['records']:
            require(Path(row['original']).read_bytes() == raw[row['key']], 'Original archived record: ' + row['key'])
        for row in manifest['original_files']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'] and file_digest(path) == row['sha256'], 'Original external file: ' + str(path))
    if args.binaries or args.git_source:
        command = [sys.executable, str(packages / 'verify.py')]
        if args.binaries: command.append('--binaries')
        if args.git_source: command.extend(['--git-source', args.git_source])
        subprocess.run(command, check=True, timeout=180)
    print(f'PASS {len(raw)} records; 5 full CPU/report/PCM-matched routes; 37 Metal frames; 13 reviewed original PNGs; restart/RTL/CJK/pseudo/narrow sampled scope; no window/FPS/phase approval')


if __name__ == '__main__':
    main()
