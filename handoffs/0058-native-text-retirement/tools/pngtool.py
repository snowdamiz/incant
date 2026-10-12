#!/usr/bin/env python3
"""Stdlib-only PNG helper for the 0058 review (8-bit RGB/RGBA, non-interlaced).

A copy of the 0057 helper (reading only; no 0057 file is modified).
Usage: python3 -I -B pngtool.py <command> ...
  info  IMG                              size, colour type, ICC profile name
  px    IMG X Y [X Y ...]                pixel values
  crop  IMG OUT X Y W H [ZOOM]           crop (nearest zoom), keeps the source iCCP chunk
  stack OUT IMG [IMG ...]                vertical stack with 8 px gaps (sizes may differ)
  diff  A B [AX AY BX BY W H] [--tol N]  count differing pixels in two equal regions; bounding box
  rows  IMG X0 X1 Y0 Y1                  per-row first/last x whose colour differs from (X0, row)
Values are the stored code values; nothing is colour converted.
"""
import struct
import sys
import zlib


def read(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG'
    pos, chunks, idat = 8, [], b''
    while pos < len(data):
        length, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        chunks.append((kind, body))
        if kind == b'IDAT':
            idat += body
        pos += 12 + length
    width, height, depth, colour, _, _, interlace = struct.unpack('>IIBBBBB', chunks[0][1])
    assert depth == 8 and colour in (2, 6) and interlace == 0, (depth, colour, interlace)
    channels = 4 if colour == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    pixels = [[tuple(row[x * channels:x * channels + 3]) for x in range(width)] for row in rows]
    icc = next((body for kind, body in chunks if kind == b'iCCP'), None)
    return {'w': width, 'h': height, 'px': pixels, 'icc': icc, 'colour': colour}


def write(path, pixels, icc=None):
    height, width = len(pixels), len(pixels[0])

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)
    raw = b''.join(b'\x00' + bytes(v for p in row for v in p) for row in pixels)
    out = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    if icc:
        out += chunk(b'iCCP', icc)
    out += chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')
    with open(path, 'xb') as f:
        f.write(out)


def main(argv):
    command = argv[0]
    if command == 'info':
        image = read(argv[1])
        name = image['icc'].split(b'\0', 1)[0].decode('latin-1') if image['icc'] else None
        print({'size': [image['w'], image['h']], 'colour_type': image['colour'], 'icc_profile': name})
    elif command == 'px':
        image = read(argv[1])
        coords = list(map(int, argv[2:]))
        for x, y in zip(coords[::2], coords[1::2]):
            print(x, y, image['px'][y][x])
    elif command == 'crop':
        image = read(argv[1])
        x, y, w, h = map(int, argv[3:7])
        zoom = int(argv[7]) if len(argv) > 7 else 1
        rows = [[p for p in image['px'][yy][x:x + w] for p in [p] * zoom] for yy in range(y, y + h)]
        write(argv[2], [row for row in rows for _ in range(zoom)], image['icc'])
    elif command == 'stack':
        images = [read(p) for p in argv[2:]]
        width = max(i['w'] for i in images)
        rows = []
        for n, image in enumerate(images):
            if n:
                rows.extend([[(255, 255, 255)] * width for _ in range(8)])
            rows.extend(row + [(255, 255, 255)] * (width - image['w']) for row in image['px'])
        write(argv[1], rows, images[0]['icc'])
    elif command == 'diff':
        tol = 0
        if '--tol' in argv:
            tol = int(argv[argv.index('--tol') + 1])
            argv = argv[:argv.index('--tol')]
        a, b = read(argv[1]), read(argv[2])
        if len(argv) > 3:
            ax, ay, bx, by, w, h = map(int, argv[3:9])
        else:
            ax = ay = bx = by = 0
            w, h = min(a['w'], b['w']), min(a['h'], b['h'])
        count, box, worst = 0, None, 0
        for y in range(h):
            for x in range(w):
                pa, pb = a['px'][ay + y][ax + x], b['px'][by + y][bx + x]
                d = max(abs(i - j) for i, j in zip(pa, pb))
                worst = max(worst, d)
                if d > tol:
                    count += 1
                    box = [x, y, x, y] if box is None else [min(box[0], x), min(box[1], y),
                                                            max(box[2], x), max(box[3], y)]
        print({'region': [w, h], 'differing': count, 'tolerance': tol, 'max_delta': worst, 'bbox': box})
    elif command == 'rows':
        image = read(argv[1])
        x0, x1, y0, y1 = map(int, argv[2:6])
        for y in range(y0, y1):
            base = image['px'][y][x0]
            xs = [x for x in range(x0, x1) if image['px'][y][x] != base]
            print(y, base, (xs[0], xs[-1]) if xs else None)
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main(sys.argv[1:])
