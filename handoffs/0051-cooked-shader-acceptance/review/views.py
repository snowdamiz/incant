#!/usr/bin/env python3
"""Magnified crops and difference maps of existing 0051 captures for pixel inspection.

CPU-only and derived: every output is labelled `derived-*`, records its source
capture hash and crop, and is written create-new under the ignored review folder.
No renderer is launched and no capture is modified. Crops use the default side;
the analysis report proves each cooked file is byte-identical.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import zlib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUTPUT = ROOT / 'artifacts/0051-cooked-shader-acceptance'
DERIVED = OUTPUT / 'review/derived'
spec = importlib.util.spec_from_file_location('analyze_0051', HERE / 'analyze.py')
analyze = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analyze)

# name: (capture relative to OUTPUT, [x0, y0, x1, y1) crop, integer magnification)
VIEWS = {
    'lookdev38-cube-contact': ('lookdev-default/lookdev-sun-38.png', (115, 195, 275, 315), 4),
    'lookdev38-sphere-terminator-contact': ('lookdev-default/lookdev-sun-38.png', (270, 185, 480, 345), 3),
    'lookdev38-wall-and-distant-posts': ('lookdev-default/lookdev-sun-38.png', (100, 25, 400, 235), 3),
    'lookdev38-ramp-post': ('lookdev-default/lookdev-sun-38.png', (380, 110, 590, 245), 3),
    'lookdev12-sphere-shadow-edge': ('lookdev-default/lookdev-sun-12.png', (330, 280, 560, 400), 4),
    'lookdev12-distant-post-shadows': ('lookdev-default/lookdev-sun-12.png', (180, 28, 420, 140), 3),
    'lookdev12-cube-wall-contact': ('lookdev-default/lookdev-sun-12.png', (115, 150, 345, 300), 3),
    'lookdev12-ramp-right-edge': ('lookdev-default/lookdev-sun-12.png', (380, 110, 640, 260), 3),
    'lookdev70-contacts': ('lookdev-default/lookdev-sun-70.png', (115, 180, 410, 325), 3),
    'lookdev70-ramp-post': ('lookdev-default/lookdev-sun-70.png', (380, 110, 540, 245), 3),
    'unshadowed-sphere': ('lookdev-default/lookdev-unshadowed.png', (270, 185, 400, 300), 4),
    'mask-alpha-shadow': ('masks-default/shadow-alpha-mask.png', (150, 185, 290, 295), 4),
    'mask-reflected-shadow': ('masks-default/shadow-reflected-mask.png', (150, 185, 290, 295), 4),
    'mask-double-sided-shadow': ('masks-default/shadow-double-sided-back.png', (150, 185, 290, 295), 4),
    'mask-single-sided-region': ('masks-default/shadow-single-sided-back.png', (150, 185, 290, 295), 4),
    'mask-blended-region': ('masks-default/shadow-blended-noncaster.png', (150, 185, 290, 295), 4),
    'diagnostic-base': ('diagnostic-default/diagnostic-base.png', (0, 0, 160, 96), 6),
    'diagnostic-raised': ('diagnostic-default/diagnostic-raised.png', (0, 0, 160, 96), 6),
}


DIFF_SOURCE = 'lookdev-sun-38.png minus lookdev-unshadowed.png (default side)'
DIFF_METHOD = ('R-channel unshadowed minus sun-38: black=0, red=1..10 (brighter red = larger), '
               'gray=>10 (cast shadow), blue=sun-38 brighter')


def diffmap():
    """Amplified difference between the 38-degree capture and its unshadowed control."""
    lit = analyze.decode_png((OUTPUT / 'lookdev-default/lookdev-sun-38.png').read_bytes())[2]
    reference = analyze.decode_png((OUTPUT / 'lookdev-default/lookdev-unshadowed.png').read_bytes())[2]
    out = bytearray()
    for i in range(0, len(lit), 4):
        d = reference[i] - lit[i]
        out += bytes((0, 0, 0, 255) if d == 0 else (0, 120, 255, 255) if d < 0
                     else (min(255, 90 + d * 25), 0, 0, 255) if d <= 10 else (150, 150, 150, 255))
    return bytes(out)


def encode_png(width, height, rgba):
    rows = b''.join(b'\x00' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))
    return (analyze.PNG_SIGNATURE + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))


def magnify(width, rgba, box, scale):
    x0, y0, x1, y1 = box
    out = bytearray()
    for y in range(y0, y1):
        line = bytearray()
        for x in range(x0, x1):
            line += rgba[(y * width + x) * 4:(y * width + x) * 4 + 4] * scale
        out += bytes(line) * scale
    return (x1 - x0) * scale, (y1 - y0) * scale, bytes(out)


def main():
    DERIVED.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for name, (relative, box, scale) in VIEWS.items():
        data = (OUTPUT / relative).read_bytes()
        width, height, rgba, _ = analyze.decode_png(data)
        assert 0 <= box[0] < box[2] <= width and 0 <= box[1] < box[3] <= height, name
        w, h, pixels = magnify(width, rgba, box, scale)
        encoded = encode_png(w, h, pixels)
        target = DERIVED / f'derived-{name}-x{scale}.png'
        with target.open('xb') as out:
            out.write(encoded)
        manifest[target.name] = {'source': relative, 'source_sha256': analyze.sha(data),
                                 'crop_xyxy': list(box), 'magnification': scale,
                                 'method': 'nearest-neighbour, no filtering or colour change',
                                 'sha256': analyze.sha(encoded)}
    difference = diffmap()
    for name, (w, h, pixels), box in (
            ('derived-diffmap-sun38-vs-unshadowed.png', (640, 480, difference), None),
            ('derived-diffmap-sun38-sphere-x4.png', magnify(640, difference, (270, 185, 400, 300), 4),
             [270, 185, 400, 300])):
        encoded = encode_png(w, h, pixels)
        with (DERIVED / name).open('xb') as out:
            out.write(encoded)
        manifest[name] = {'source': DIFF_SOURCE, 'method': DIFF_METHOD, 'crop_xyxy': box,
                          'magnification': 4 if box else 1, 'sha256': analyze.sha(encoded)}
    with (DERIVED / 'derived-manifest.json').open('x') as out:
        out.write(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({k: v['sha256'][:12] for k, v in manifest.items()}, indent=1))


if __name__ == '__main__':
    main()
