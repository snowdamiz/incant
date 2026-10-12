#!/usr/bin/env python3
"""CPU-only comparison of the 24 captures from the eight 0051 helper children.

Never launches a renderer. Verifies the helper ledger and RELEASED latch, decodes
every PNG with a minimal stdlib decoder, compares default/cooked pairs (file and
decoded bytes), cross-checks the captures against the numerical raw readbacks,
and repeats the tests' analytic probes from the exported pixels. Pixel appearance
review is done separately by inspecting the images themselves.
"""
import hashlib
import json
from pathlib import Path
import struct
import sys
import zlib

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'artifacts/0051-cooked-shader-acceptance'
TOOLS = ROOT / 'artifacts/tools/0051'
NUMERICAL = TOOLS / 'numerical'
CASES = {'output': 1, 'lookdev': 4, 'masks': 5, 'diagnostic': 2}
PNG_SIGNATURE = b'\x89PNG\r\n\x1a\n'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def decode_png(data):
    """Decode a non-interlaced 8-bit RGB/RGBA PNG to (width, height, rgba, chunks)."""
    assert data[:8] == PNG_SIGNATURE, 'not a PNG'
    offset, chunks, idat, header = 8, [], b'', None
    while offset < len(data):
        length, kind = struct.unpack('>I4s', data[offset:offset + 8])
        body = data[offset + 8:offset + 8 + length]
        crc = struct.unpack('>I', data[offset + 8 + length:offset + 12 + length])[0]
        assert zlib.crc32(kind + body) == crc, f'bad CRC in {kind!r}'
        chunks.append(kind.decode('ascii'))
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        offset += 12 + length
    width, height, depth, color, _, _, interlace = header
    assert depth == 8 and interlace == 0 and color in (2, 6), header
    channels = 4 if color == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * channels
    assert len(raw) == height * (stride + 1), 'unexpected decompressed size'
    out = bytearray(height * stride)
    previous = bytearray(stride)
    for y in range(height):
        kind = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            corner = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 255
            elif kind == 2:
                line[i] = (line[i] + up) & 255
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 255
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                line[i] = (line[i] + (left if pa <= pb and pa <= pc else up if pb <= pc else corner)) & 255
            else:
                assert kind == 0, f'filter {kind}'
        out[y * stride:(y + 1) * stride] = line
        previous = line
    if channels == 3:
        rgba = bytearray()
        for i in range(0, len(out), 3):
            rgba += out[i:i + 3] + b'\xff'
        out = rgba
    return width, height, bytes(out), chunks


def px(rgba, width, x, y):
    i = (y * width + x) * 4
    return tuple(rgba[i:i + 4])


def ledger_checks():
    ledger = json.loads((OUTPUT / 'children.json').read_text())
    released = json.loads((TOOLS / 'RELEASED.json').read_text())
    rows = []
    for record in ledger:
        directory = OUTPUT / record['name']
        files = {str(p.relative_to(directory)): sha(p.read_bytes())
                 for p in sorted(directory.rglob('*')) if p.is_file()}
        rows.append({
            'name': record['name'], 'pid': record['pid'], 'exit_code': record['exit_code'],
            'started_at': record['started_at'], 'ended_at': record['ended_at'],
            'timeout': record.get('timeout', False), 'reaped': record['reaped'],
            'capture_count_matches': record['capture_count_matches'],
            'files_unchanged_since_ledger': files == record['files'],
            'binary_sha256': record['binary_sha256'],
        })
    return {
        'children': rows,
        'ledger_sha256': sha((OUTPUT / 'children.json').read_bytes()),
        'released': released,
        'released_ledger_hash_matches': released['ledger_sha256'] == sha((OUTPUT / 'children.json').read_bytes()),
        'gate_absent': not (TOOLS / 'gpu-review-ready.json').exists(),
        'eight_unique_children': len(ledger) == 8 and len({r['name'] for r in ledger}) == 8,
        'all_passed': all(r['exit_code'] == 0 and r['capture_count_matches'] and r['reaped']
                          and not r.get('timeout') for r in ledger),
    }


def captures():
    """Decode every PNG; return {case: {file: {mode: info}}}."""
    table = {}
    for case, expected in CASES.items():
        for mode in ('default', 'cooked'):
            directory = OUTPUT / f'{case}-{mode}'
            pngs = sorted(directory.glob('*.png'))
            assert len(pngs) == expected, (case, mode, len(pngs))
            for path in pngs:
                data = path.read_bytes()
                width, height, rgba, chunks = decode_png(data)
                table.setdefault(case, {}).setdefault(path.name, {})[mode] = {
                    'path': str(path.relative_to(ROOT)), 'file_sha256': sha(data),
                    'bytes': len(data), 'width': width, 'height': height,
                    'chunks': sorted(set(chunks)), 'rgba_sha256': sha(rgba), 'rgba': rgba,
                }
    return table


def pair_report(table):
    rows = []
    for case, files in table.items():
        for name, sides in sorted(files.items()):
            a, b = sides['default'], sides['cooked']
            differing = sum(1 for i in range(0, len(a['rgba']), 4) if a['rgba'][i:i + 4] != b['rgba'][i:i + 4])
            max_delta = max((abs(x - y) for x, y in zip(a['rgba'], b['rgba'])), default=0)
            rows.append({
                'case': case, 'file': name, 'width': a['width'], 'height': a['height'],
                'default_file_sha256': a['file_sha256'], 'cooked_file_sha256': b['file_sha256'],
                'file_bytes_equal': a['file_sha256'] == b['file_sha256'],
                'rgba_sha256': a['rgba_sha256'], 'decoded_equal': a['rgba'] == b['rgba'],
                'differing_pixels': differing, 'max_channel_delta': max_delta,
                'chunks': a['chunks'],
            })
    return rows


def numerical_cross_checks(table):
    """The PNG captures should equal the raw sRGB-format readbacks of the same scene."""
    checks = []
    pairs = [('diagnostic', 'diagnostic-base.png', 'diagnostic/diagnostic-base-Rgba8UnormSrgb.rgba'),
             ('diagnostic', 'diagnostic-raised.png', 'diagnostic/diagnostic-raised-Rgba8UnormSrgb.rgba'),
             ('output', 'hdr-reference-swatches.png', 'output/hdr-swatches-Rgba8UnormSrgb.rgba')]
    for case, name, raw in pairs:
        for mode, folder in (('default', 'default'), ('cooked', 'cooked-geometry')):
            raw_bytes = (NUMERICAL / folder / raw).read_bytes()
            capture = table[case][name][mode]['rgba']
            checks.append({'capture': f'{case}-{mode}/{name}', 'numerical': f'numerical/{folder}/{raw}',
                           'raw_sha256': sha(raw_bytes), 'equal': raw_bytes == capture,
                           'note': 'output-v1 capture vs geometry-v2 numerical run (unchanged output sources)'
                           if case == 'output' else 'same binary and scene, separate process'})
    return checks


def swatches(table):
    expected = [0, 25, 118, 188, 231, 243, 252, 254, 255, 255, (254, 158, 158), (255, 255, 255)]
    info = table['output']['hdr-reference-swatches.png']['default']
    width, rgba = info['width'], info['rgba']
    rows = []
    for i, value in enumerate(expected):
        want = value if isinstance(value, tuple) else (value,) * 3
        band = {px(rgba, width, x, y) for y in range(info['height']) for x in range(i * 32, i * 32 + 32)}
        centre = px(rgba, width, i * 32 + 16, 64)
        rows.append({'swatch': i, 'centre_rgba': centre, 'expected_rgb': want,
                     'within_one': all(abs(c - w) <= 1 for c, w in zip(centre[:3], want)),
                     'uniform_band': len(band) == 1})
    return rows


def lookdev(table):
    files = table['lookdev']
    reference = files['lookdev-unshadowed.png']['default']['rgba']
    width, height = 640, 480
    out = {}
    for name in ('lookdev-sun-12.png', 'lookdev-sun-38.png', 'lookdev-sun-70.png'):
        rgba = files[name]['default']['rgba']
        changed = [i // 4 for i in range(0, len(rgba), 4) if rgba[i:i + 4] != reference[i:i + 4]]
        darker = sum(1 for p in changed if sum(rgba[p * 4:p * 4 + 3]) < sum(reference[p * 4:p * 4 + 3]))
        brighter = sum(1 for p in changed if sum(rgba[p * 4:p * 4 + 3]) > sum(reference[p * 4:p * 4 + 3]))
        below = sum(1 for p in changed if p // width >= 360)
        ys = [p // width for p in changed]
        xs = [p % width for p in changed]
        out[name] = {'changed_vs_unshadowed': len(changed), 'darker': darker, 'brighter': brighter,
                     'changed_rows_360_plus': below,
                     'changed_bbox': [min(xs), min(ys), max(xs), max(ys)] if changed else None}
    out['alpha_all_255'] = all(files[n]['default']['rgba'][3::4] == b'\xff' * (width * height) for n in files)
    return out


def mask_probe(rgba, world):
    """Same receiver-plane projection as the test's pixel() helper."""
    import math
    x = int((0.5 + world[0] / (2 * 8 * math.tan(math.radians(30)) * 640 / 480)) * 640)
    y = int((0.5 - world[1] / (2 * 8 * math.tan(math.radians(30)))) * 480)
    return px(rgba, 640, x, y)


def masks(table):
    files = table['masks']
    reference = files['shadow-single-sided-back.png']['default']['rgba']
    out = {}
    for name in sorted(files):
        rgba = files[name]['default']['rgba']
        changed = [i // 4 for i in range(0, len(rgba), 4) if rgba[i:i + 4] != reference[i:i + 4]]
        xs, ys = [p % 640 for p in changed], [p // 640 for p in changed]
        out[name] = {'left_-2.4': mask_probe(rgba, (-2.4, 0.)), 'right_-1.6': mask_probe(rgba, (-1.6, 0.)),
                     'changed_vs_single_sided_back': len(changed),
                     'changed_bbox': [min(xs), min(ys), max(xs), max(ys)] if changed else None}
    unique = {}
    for name in sorted(files):
        unique.setdefault(files[name]['default']['rgba_sha256'], []).append(name)
    out['identical_groups'] = [names for names in unique.values() if len(names) > 1]
    return out


def diagnostic(table):
    out = {}
    width, height = 160, 96
    for name in ('diagnostic-base.png', 'diagnostic-raised.png'):
        rgba = table['diagnostic'][name]['default']['rgba']
        background = rgba[:4]
        drawn = [i // 4 for i in range(0, len(rgba), 4) if rgba[i:i + 4] != background]
        colours = {}
        for p in drawn:
            colours[tuple(rgba[p * 4:p * 4 + 4])] = colours.get(tuple(rgba[p * 4:p * 4 + 4]), 0) + 1
        halves = {}
        for side, test in (('left', lambda x: x < width // 2), ('right', lambda x: x >= width // 2)):
            pts = [p for p in drawn if test(p % width)]
            xs, ys = [p % width for p in pts], [p // width for p in pts]
            halves[side] = {'pixels': len(pts), 'bbox': [min(xs), min(ys), max(xs), max(ys)] if pts else None}
        out[name] = {'background_rgba': tuple(background), 'drawn_pixels': len(drawn),
                     'mean_row': sum(p // width for p in drawn) / len(drawn),
                     'top_colours': sorted(colours.items(), key=lambda kv: -kv[1])[:6], 'halves': halves}
    return out


def observations(table):
    """Measured appearance observations from pixel review; identical on both sides."""
    files = {name: info['default']['rgba'] for name, info in table['lookdev'].items()}
    red = lambda name, x, y: files[name][(y * 640 + x) * 4]
    reference = 'lookdev-unshadowed.png'
    # Thin 2 cm wall: columns left of the sphere where its dark face meets the ground.
    wall = {}
    for name in ('lookdev-sun-12.png', 'lookdev-sun-38.png', 'lookdev-sun-70.png'):
        rows = {}
        for x in range(212, 281):
            base = max(y for y in range(200, 230) if red(reference, x, y) == 82)
            first, lit = red(name, x, base + 1), red(name, 600, 470)
            rows[x] = (first, lit)
        leaks = [x for x, (first, lit) in rows.items() if first >= 0.75 * lit]
        wall[name] = {'columns': len(rows), 'first_ground_row_lit_columns': len(leaks),
                      'example_x240_first_row_vs_open_ground': rows[240]}
    diff = lambda name, box: [red(reference, x, y) - red(name, x, y)
                              for y in range(box[1], box[3]) for x in range(box[0], box[2])]
    histogram = lambda values: {d: values.count(d) for d in sorted(set(values)) if d}
    sphere = [red(reference, x, y) - red('lookdev-sun-38.png', x, y)
              for y in range(194, 293) for x in range(284, 383)
              if (x - 333) ** 2 + (y - 243) ** 2 <= 47 ** 2]
    return {
        'thin_wall_contact_gap': wall,
        'sphere_lit_side_sun38_minus_unshadowed': histogram(sphere),
        'ramp_interior_sun38_minus_unshadowed': histogram(diff('lookdev-sun-38.png', (400, 140, 450, 175))),
        'open_ground_rows_360_plus_changed': sum(1 for d in diff('lookdev-sun-38.png', (0, 360, 640, 480)) if d),
        'cube_top_light_facing_edge_sun12': {
            'interior': red('lookdev-sun-12.png', 170, 220),
            'back_edge_row_210': [red('lookdev-sun-12.png', x, 210) for x in range(146, 195, 6)],
            'left_edge': [red('lookdev-sun-12.png', x, y) for x, y in ((146, 210), (140, 216), (134, 222), (128, 228))]},
    }


def main():
    table = captures()
    report = {
        'ledger': ledger_checks(),
        'pairs': pair_report(table),
        'numerical_cross_checks': numerical_cross_checks(table),
        'swatches': swatches(table),
        'lookdev': lookdev(table),
        'masks': masks(table),
        'diagnostic': diagnostic(table),
        'observations': observations(table),
    }
    report['summary'] = {
        'captures': sum(len(s) for f in table.values() for s in f.values()),
        'pairs': len(report['pairs']),
        'pairs_file_equal': sum(r['file_bytes_equal'] for r in report['pairs']),
        'pairs_decoded_equal': sum(r['decoded_equal'] for r in report['pairs']),
        'numerical_cross_checks_equal': sum(c['equal'] for c in report['numerical_cross_checks']),
        'numerical_cross_checks': len(report['numerical_cross_checks']),
    }
    destination = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    text = json.dumps(report, indent=2, default=list) + '\n'
    if destination:
        with destination.open('x') as out:
            out.write(text)
    print(text)


if __name__ == '__main__':
    main()
