#!/usr/bin/env python3
"""Raw RGBA frames from the 0065 adapter to PNG, unedited (CPU only, no PIL).

  png.py convert REPORT.json OUT_DIR           every retained frame, pixels unchanged
  png.py crop RGBA WIDTH X Y W H ZOOM OUT.png  nearest-neighbour enlargement for review

`convert` writes 8-bit RGBA PNGs of exactly the adapter's sRGB-encoded bytes and checks
each source against the report's SHA-256 first. Crops are review aids only; they are
labelled as crops in the evidence and never replace the unedited frame.
"""
import hashlib
import json
from pathlib import Path
import struct
import sys
import zlib


def png(path, rgba, width, height):
    assert len(rgba) == width * height * 4
    rows = b''.join(b'\x00' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))

    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    data = (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'sRGB', b'\x00') + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))
    with open(path, 'xb') as stream:
        stream.write(data)


def convert(report_path, out):
    report = json.loads(Path(report_path).read_text(encoding='utf-8'))
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    for capture in report['captures']:
        if 'rgba' not in capture:
            continue
        source = Path(report_path).parent / capture['rgba']
        rgba = source.read_bytes()
        if hashlib.sha256(rgba).hexdigest() != capture['rgba_sha256']:
            raise SystemExit(f'{source} differs from its report hash')
        width, height = capture['size']
        target = out / (Path(report_path).parent.name + '-' + capture['rgba'][3:-5] + '.png')
        png(target, rgba, width, height)
        rows.append({'png': target.name, 'png_sha256': hashlib.sha256(target.read_bytes()).hexdigest(),
                     'rgba': str(source), 'rgba_sha256': capture['rgba_sha256'], 'size': [width, height],
                     'host_step': capture['host_step'], 'simulation_tick': capture['simulation_tick'],
                     'generation': capture['generation']})
    print(json.dumps(rows, indent=1))


def crop(source, width, x, y, w, h, zoom, target):
    rgba = Path(source).read_bytes()
    out = bytearray()
    for row in range(y, y + h):
        line = bytearray()
        for col in range(x, x + w):
            i = (row * width + col) * 4
            line += rgba[i:i + 4] * zoom
        out += bytes(line) * zoom
    png(target, bytes(out), w * zoom, h * zoom)


def main():
    args = sys.argv[1:]
    if args[:1] == ['convert'] and len(args) == 3:
        convert(args[1], Path(args[2]))
    elif args[:1] == ['crop'] and len(args) == 9:
        crop(args[1], *map(int, args[2:8]), args[8])
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == '__main__':
    main()
