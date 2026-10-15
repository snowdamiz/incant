#!/usr/bin/env python3
"""Verify retained data only. Never import, extract or execute archived code."""
import argparse
import base64
import gzip
import hashlib
import json
import math
import re
import struct
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
CPU = 'prewarm-cpu-009-dyl4ma4j'
GPU = 'prewarm-gpu-001-jn36ppg6'
HEAD = '11d4f5f626319b014656da24064e26478e2e83de'
SOURCE = 'e7eeda90e6fc441c74afa3c0cb10f9cd1115177922386b685521ff3764553307'
STAGES = [f'gpu-{route}-{kind}-001' for route in ('ordinary', 'cooked')
          for kind in ('raster', 'transition', 'player')]


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def png_rgba(raw):
    """Decode only this packet's lossless, unfiltered 96x80 RGBA PNG encoding."""
    require(raw[:8] == b'\x89PNG\r\n\x1a\n', 'PNG signature')
    offset, compressed, names = 8, b'', []
    while offset < len(raw):
        size = struct.unpack('>I', raw[offset:offset + 4])[0]
        kind = raw[offset + 4:offset + 8]
        data = raw[offset + 8:offset + 8 + size]
        crc = raw[offset + 8 + size:offset + 12 + size]
        require(crc == struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff), 'PNG CRC')
        names.append(kind)
        if kind == b'IHDR':
            require(data == struct.pack('>IIBBBBB', 96, 80, 8, 6, 0, 0, 0), 'PNG dimensions')
        if kind == b'IDAT':
            compressed += data
        offset += 12 + size
    require(names == [b'IHDR', b'IDAT', b'IEND'] and offset == len(raw), 'PNG chunk inventory')
    decoder = zlib.decompressobj()
    rows = decoder.decompress(compressed, 80 * 385 + 1)
    require(len(rows) == 80 * 385 and decoder.eof and not decoder.unused_data, 'PNG bounded rows')
    require(all(rows[i] == 0 for i in range(0, len(rows), 385)), 'PNG filter')
    return b''.join(rows[i + 1:i + 385] for i in range(0, len(rows), 385))


def verify(data):
    require(data['schema'] == 1, 'Evidence schema')
    records, images = data['records'], data['image_blobs']
    decoded = {}
    for collection, field in ((records, 'text'), (images, 'base64')):
        for path, row in collection.items():
            p = Path(path)
            require(not p.is_absolute() and '..' not in p.parts, 'Relative record path')
            raw = row[field].encode() if field == 'text' else base64.b64decode(row[field], validate=True)
            require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Record identity: ' + path)
            decoded[path] = raw

    def read(path):
        return json.loads(decoded[path])

    def text(path):
        return records[path]['text']

    def clean(path):
        row = read(path + '/result.json')
        require(row['exit_code'] == 0 and row['error'] is None and row['cleanup_confirmed']
                and not row['cleanup_errors'], 'Process success and cleanup: ' + path)
        return row

    outcome = data['outcome']
    for flag in ('eager_default_retained', 'complete_opt_in_model_material_v1_slice',
                 'all_owned_observed_pids_absent', 'source_free_player_cli_executed'):
        require(outcome[flag] is True, 'Qualified scope: ' + flag)
    require(outcome['tested_source_commit'] == HEAD and outcome['joined_main'] ==
            '552baceba079f5d47555047204a865491401442c', 'Tested composition')
    for flag in ('all_nested_pids_individually_observed', 'headless_cli_executed',
                 'windows_or_desktop_capture_executed', 'all_optional_effects_supported',
                 'arbitrary_animation_gpu_qualified', 'direct_depth_buffer_readback',
                 'driver_stall_measured', 'external_driver_service_memory_accounted',
                 'performance_gate_passed', 'adoption_gate_passed', 'shipping_or_reference_phone_qualification'):
        require(outcome[flag] is False, 'Preserve limitation: ' + flag)
    require((outcome['cpu_passed'], outcome['cpu_gpu_ignored'], outcome['native_controls_passed'],
             outcome['offscreen_draw_submissions'], outcome['ordinary_cooked_pairs'],
             outcome['actual_max_channel_difference'], outcome['reviewed_png_count']) ==
            (28, 6, 6, 162, 30, 0, 12), 'Scoped result counts')
    for folder in (CPU, GPU):
        for row in read(folder + '/packet-manifest.json'):
            actual = records[folder + '/' + row['path']]
            require((actual['bytes'], actual['sha256']) == (row['bytes'], row['sha256']), 'Frozen input binding')
        source = read(folder + '/source-manifest.json')
        require(source['head'] == HEAD and len(source['files']) == 22024 and
                records[folder + '/source-manifest.json']['sha256'] == SOURCE, 'Source inventory')
        require(len(read(folder + '/tools.json')) == 5, 'Pinned tools')
        freeze = read(folder + '/evidence/final-freeze.json')
        require(freeze['source_files_verified'] == 22024 and freeze['source_manifest_sha256'] == SOURCE, 'Postflight source')
    require(read(CPU + '/tools.json') == read(GPU + '/tools.json'), 'Same GPU producer tools')

    plan = read(CPU + '/plan.json')
    artifacts = read(CPU + '/evidence/artifacts.json')
    require(len(artifacts) == 13, 'Copied CPU artifacts')
    gpu_artifacts = read(GPU + '/artifacts.json')
    require(set(gpu_artifacts) == {route + '-' + name for route in ('ordinary', 'cooked')
                                  for name in ('player', 'player-controls', 'incant_render')}
            and all(row == artifacts[key] for key, row in gpu_artifacts.items()), 'GPU exact CPU producer artifacts')
    for name, row in artifacts.items():
        relative = CPU + '/retained-artifacts/' + name
        require(data['external_artifacts'][relative] == {k: row[k] for k in ('bytes', 'sha256')}
                and row['source_manifest_sha256'] == SOURCE, 'External artifact binding')
    for route in ('ordinary', 'cooked'):
        features = artifacts[route + '-incant_render']['features']
        require(('cooked-native-shaders' in features) == (route == 'cooked') and
                {'metal-binary-archives', 'native-compilation'} <= set(features), 'Actual feature route')
    totals = dict(build=0., cpu=0., lint=0.)
    passed = ignored = 0
    cpu_pids = set()
    for stage in plan['stages']:
        path = CPU + '/evidence/' + stage['name']
        row = clean(path)
        totals[stage['charge']] += row['elapsed_seconds']
        cpu_pids.update((row['supervisor_pid'], read(path + '/process.json')['pid']))
        cpu_pids.update(item['pid'] for item in row['observed_descendants'])
        if stage['charge'] == 'cpu':
            matches = re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', text(path + '/stdout.log'))
            require(matches == [(str(stage['passed']), str(stage['ignored']))], 'Actual CPU test count')
            passed += stage['passed']
            ignored += stage['ignored']
        elif stage['charge'] == 'lint':
            require(row['argv'][-2:] == ['-D', 'warnings'], 'Strict relevant lint')
    require((passed, ignored) == (28, 6), 'Complete CPU controls')
    terminal = read(CPU + '/evidence/wrapper-terminal.json')
    require(terminal['exit_code'] == 0 and terminal['all_plan_stages_complete'], 'CPU outer terminal')
    for category, total in totals.items():
        require(math.isclose(total, terminal['charged_seconds'][category], abs_tol=1e-8), 'CPU actual charge')
    absence = read(CPU + '/evidence/owned-process-absence.json')
    require(absence['all_absent'] and set(absence['pids']) == cpu_pids and len(cpu_pids) == 478, 'Observed CPU PID absence')

    require(len(data['failures']) == 7, 'Retain seven failed CPU packets')
    diagnostics = ('E0282', 'Camera', 'Build storage limit', 'publication', 'assertion', 'Manifest(Identity)', 'filter_map_bool_then')
    for failure, marker in zip(data['failures'], diagnostics):
        folder, stage = failure['packet'], failure['stage']
        terminal = read(folder + '/evidence/wrapper-terminal.json')
        result = read(folder + '/evidence/' + stage + '/result.json')
        require(terminal['exit_code'] != 0 and result['exit_code'] != 0 and result['cleanup_confirmed'], 'Retained failure')
        combined = text(folder + '/evidence/' + stage + '/stdout.log') + text(folder + '/evidence/' + stage + '/stderr.log') + json.dumps(terminal)
        require(marker in combined, 'Original failure diagnostic')
        require(read(folder + '/evidence/owned-process-absence.json')['all_absent'], 'Failed owner cleanup')

    continuation = read(GPU + '/continuation.json')
    for name, cap in (('build', 1800), ('cpu', 180)):
        total = sum(read(p)['elapsed_seconds'] for p in continuation[name + '_receipts'])
        require(math.isclose(total, continuation['prior_' + name + '_elapsed_seconds'], abs_tol=1e-8)
                and math.isclose(cap - total, continuation['remaining_' + name + '_seconds'], abs_tol=1e-8), 'Original cumulative ' + name)
    for name, paths in continuation['lint_receipts'].items():
        total = sum(read(p)['elapsed_seconds'] for p in paths)
        require(math.isclose(total, continuation['prior_lint_seconds'][name], abs_tol=1e-8)
                and math.isclose(600 - total, continuation['remaining_lint_seconds'][name], abs_tol=1e-8), 'Cumulative lint')
    gpu_plan = read(GPU + '/plan.json')
    phase = read(GPU + '/evidence/gpu-phase.json')
    prior = sum(read(p)['phase_elapsed_seconds'] for p in gpu_plan['prior_gpu_phase_receipts'])
    require(math.isclose(prior, phase['prior_gpu_elapsed_seconds'], abs_tol=1e-8)
            and math.isclose(180 - prior - phase['phase_elapsed_seconds'], phase['remaining_gpu_seconds'], abs_tol=1e-8)
            and phase['phase_elapsed_seconds'] <= 150, 'Original GPU account, no reset')
    require([s['name'] for s in phase['results']] == STAGES and phase['all_six_passed'] and phase['error'] is None, 'Six serial stages')
    gpu_pids = set()
    for stage in gpu_plan['stages']:
        path = GPU + '/evidence/' + stage['name']
        row = clean(path)
        require(row['timeout_seconds'] <= 30 and row['elapsed_seconds'] < 31, 'Bounded native child')
        require(row['peak_rss_bytes'] <= 8 * 1024**3 and row['final_run_bytes'] <= 13 * 1024**3
                and row['final_build_bytes'] <= 10 * 1024**3, 'Original allocated resources')
        require(row['argv'][0] == artifacts[stage['artifact']]['path'] and stage['test'] in row['argv'], 'Exact copied executable and test')
        require('test result: ok. 1 passed; 0 failed' in text(path + '/stdout.log'), 'Native control success')
        gpu_pids.update((row['supervisor_pid'], read(path + '/process.json')['pid']))
        gpu_pids.update(item['pid'] for item in row['observed_descendants'])
    absence = read(GPU + '/evidence/owned-process-absence.json')
    require(absence['all_absent'] and set(absence['pids']) == gpu_pids and len(gpu_pids) == 9, 'Observed GPU PID absence only')
    require(read(GPU + '/evidence/wrapper-terminal.json')['exit_code'] == 0, 'GPU outer terminal')

    for route in ('ordinary', 'cooked'):
        root = GPU + '/pixels/' + route + '/'
        names = [f'pending-{n}-{alpha}-{str(d).lower()}-{str(m).lower()}' for n in (1, 4)
                 for alpha in ('OPAQUE', 'MASK', 'BLEND') for d in (False, True) for m in (False, True)]
        names += [f'pending-{n}-blend-{a}' for n in (1, 4) for a in ('0.25', '0.75')]
        require({Path(p).stem for p in images if p.startswith(root + 'raster/') and p.endswith('.rgba')} == set(names), 'Exact raster cases')
        for name in names:
            meta = read(root + 'raster/' + name + '.json')
            report = meta['prewarm']
            require(meta['samples'] == int(name.split('-')[1]) and report['prewarmed'] == 0
                    and report['real_selections'] == 0 and report['fallback_selections'] > 0, 'Actual pending raster route')
        pending, published = [read(root + 'transition/worker-' + n + '.json') for n in ('pending', 'published')]
        require(pending['prewarm']['fallback_selections'] > 0 and pending['prewarm']['real_selections'] == 0
                and published['prewarm']['real_selections'] > 0, 'Actual pending to published selection')
        require(decoded[root + 'transition/worker-pending.rgba'] != decoded[root + 'transition/worker-published.rgba'], 'Visible publication')
        native = read(root + 'transition/worker-native-compilation.json')
        later = [r for r in native['rows'] if r['sequence'] > native['startup_last_sequence']]
        require(later and any(r['kind'] == 'RenderPipeline' for r in later)
                and all(r['phase'] == 'Background' and r['segment'] > 0 and r['outcome'] == 'Success' for r in later), 'Worker-owned native creation')
        player = read(root + 'player/source-free-player.json')
        report = player['render_input']
        require(report['mode'] == 'gpu-offscreen-recorded-materials' and report['gpu']['adapter']['backend'] == 'Metal'
                and report['gpu']['accepted_frames'] == 3 and report['material_prewarm']['prewarmed'] == 1
                and report['material_prewarm']['fallback_selections'] > 0
                and any(k['key'] == 1 and k['requested'] for k in report['material_prewarm']['keys'])
                and all(c['non_background_pixels'] > 0 for c in report['gpu']['captures'])
                and player['material_prewarm_close']['closed'], 'Actual source-free record/load/mirror fallback')

    pairs = read(GPU + '/evidence/pixel-parity.json')['pairs']
    require(len(pairs) == 30 and len({(p['kind'], p['case']) for p in pairs}) == 30, 'Thirty unique parity pairs')
    for pair in pairs:
        raw = []
        for route in ('ordinary', 'cooked'):
            path = GPU + '/pixels/' + route + '/' + pair['kind'] + '/' + pair['case']
            pixels = decoded[path + '.rgba']
            require(len(pixels) == 96 * 80 * 4 and digest(pixels) == pair[route + '_sha256'], 'Raw image binding')
            meta = read(path + '.json')
            require(meta['size'] == [96, 80] and meta['case'] == pair['case'] and
                    meta['profile'] == ('wgsl' if route == 'ordinary' else 'cooked') and 'backend: Metal' in meta['adapter'], 'Native image identity')
            require(png_rgba(decoded[path + '.png']) == pixels, 'PNG exact raw RGBA representation')
            raw.append(pixels)
        require(raw[0] == raw[1] and pair['max_channel_byte_difference'] == 0
                and pair['alpha_equal'] and pair['red_foreground_coverage_equal'], 'Actual zero-difference shader-route parity')
    review = read(GPU + '/evidence/rendered-review.json')
    require(review['tested_source_commit'] == HEAD and len(review['reviewed_pngs']) == 12
            and review['qualification_manifest_sha256'] == records[GPU + '/evidence/qualification-manifest.json']['sha256'], 'Rendered review binding')
    for row in review['reviewed_pngs']:
        require(images[GPU + '/' + row['path']]['sha256'] == row['sha256'], 'Actually reviewed image identity')
    independent = read(GPU + '/evidence/root-rendered-review.json')
    require(len(independent['images']) == 5 and independent['performance_gate'] is False
            and independent['arbitrary_scene_or_motion_quality'] is False, 'Independent scoped review')
    for row in independent['images']:
        require(images[GPU + '/' + row['path']]['sha256'] == row['sha256'], 'Root reviewed original image')
    closure = read(GPU + '/evidence/audio-feature-closure.json')
    for row in closure['rows']:
        require(records[row['source_record']]['sha256'] == row['source_sha256'], 'Actual compiler feature receipt')
        actual = [v for line in text(row['source_record']).splitlines() if line.startswith('{')
                  for v in [json.loads(line)] if v.get('reason') == 'compiler-artifact'
                  and v.get('target', {}).get('name') in ('incant_audio_runtime', 'kira', 'cpal')]
        require(row['compiler_artifacts'] == [dict(target=v['target']['name'], features=v['features'],
                filenames=v['filenames'], fresh=v['fresh']) for v in actual], 'Actual audio feature closure')
    return {'records': len(records), 'image_blobs': len(images), 'cpu_passed': passed,
            'native_controls': 6, 'pixel_pairs': len(pairs), 'performance_gate_passed': False}


def load():
    manifest = json.loads((HERE / 'manifest.json').read_text())
    compressed = (HERE / manifest['data_file']).read_bytes()
    require(len(compressed) == manifest['compressed_bytes'] and digest(compressed) == manifest['sha256'], 'Portable compressed identity')
    with gzip.open(HERE / manifest['data_file'], 'rb') as stream:
        raw = stream.read(64 * 1024**2 + 1)
    require(len(raw) == manifest['uncompressed_bytes'] and len(raw) <= 64 * 1024**2, 'Bounded data-only packet')
    data = json.loads(raw)
    require(len(data['records']) == manifest['record_count'] and len(data['image_blobs']) == manifest['image_blob_count']
            and len(data['external_artifacts']) == manifest['external_artifact_count'], 'Manifest inventories')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external-root', type=Path, help='Optionally hash retained native files; never execute them')
    args = parser.parse_args()
    data = load()
    result = verify(data)
    if args.external_root:
        for relative, expected in data['external_artifacts'].items():
            path = args.external_root / relative
            require(not Path(relative).is_absolute() and '..' not in Path(relative).parts, 'External relative path')
            require(path.is_file() and not path.is_symlink() and path.stat().st_size == expected['bytes'], 'External artifact size')
            with path.open('rb') as stream:
                require(hashlib.file_digest(stream, 'sha256').hexdigest() == expected['sha256'], 'External artifact hash')
        result['external_artifacts_hashed'] = len(data['external_artifacts'])
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
