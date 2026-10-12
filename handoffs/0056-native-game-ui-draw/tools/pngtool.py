#!/usr/bin/env python3
"""Minimal RGBA8 PNG crop/zoom/stack for pixel review, standard library only.

Usage:
  python3 -I pngtool.py crop IN.png X Y W H ZOOM OUT.png      (nearest-neighbour)
  python3 -I pngtool.py stack OUT.png GAP IN1.png IN2.png ...  (vertical, dark gaps)
Reads only 8-bit RGBA, non-interlaced PNGs (what the review harness writes).
"""
import struct
import sys
import zlib


def read(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG'
    at, idat, header = 8, b'', None
    while at < len(data):
        length, kind = struct.unpack('>I4s', data[at:at + 8])
        body = data[at + 8:at + 8 + length]
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
        at += 12 + length
    width, height, depth, color, _, _, interlace = header
    assert (depth, color, interlace) == (8, 6, 0), 'expects 8-bit RGBA, non-interlaced'
    raw, stride, rows, previous = zlib.decompress(idat), width * 4, [], bytearray(width * 4)
    for y in range(height):
        kind, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b, c = previous[i], previous[i - 4] if i >= 4 else 0
            if kind == 1:
                line[i] = (line[i] + a) & 255
            elif kind == 2:
                line[i] = (line[i] + b) & 255
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        previous = line
    return width, height, rows


def write(path, width, height, rows):
    raw = b''.join(b'\x00' + row for row in rows)
    chunk = lambda kind, body: struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))
    with open(path, 'wb') as out:
        out.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
                  + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def main():
    if sys.argv[1] == 'crop':
        _, _, source, x, y, w, h, zoom, target = sys.argv
        x, y, w, h, zoom = map(int, (x, y, w, h, zoom))
        width, height, rows = read(source)
        assert x + w <= width and y + h <= height, 'crop outside image'
        out = []
        for row in rows[y:y + h]:
            pixels = [row[i:i + 4] for i in range(x * 4, (x + w) * 4, 4)]
            line = b''.join(p * zoom for p in pixels)
            out.extend([line] * zoom)
        write(target, w * zoom, h * zoom, out)
    elif sys.argv[1] == 'stack':
        target, gap, sources = sys.argv[2], int(sys.argv[3]), sys.argv[4:]
        images = [read(s) for s in sources]
        width = max(i[0] for i in images)
        out = []
        for index, (w, _, rows) in enumerate(images):
            if index:
                out.extend([bytes([20, 21, 25, 255]) * width] * gap)
            out.extend(row + bytes([20, 21, 25, 255]) * (width - w) for row in rows)
        write(target, width, len(out), out)
    else:
        raise SystemExit(__doc__)


if __name__ == '__main__':
    main()
