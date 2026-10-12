#!/usr/bin/env python3
"""Crop (and optionally nearest-neighbour enlarge) an 8-bit RGB/RGBA capture.

Usage: python3 -I -B pngcrop.py SOURCE OUT X Y W H [SCALE]
       python3 -I -B pngcrop.py --stats SOURCE X Y W H
Colour-space chunks (iCCP, sRGB, gAMA, cHRM) are copied unchanged, so a Display
P3 window capture stays Display P3; pixel values are never converted. --stats
prints per-channel min/max/mean of the region for numeric review notes.
"""
import struct
import sys
import zlib

KEEP = (b'iCCP', b'sRGB', b'gAMA', b'cHRM')


def chunks(data):
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('not a PNG')
    at = 8
    while at < len(data):
        size = struct.unpack('>I', data[at:at + 4])[0]
        yield data[at + 4:at + 8], data[at + 8:at + 8 + size]
        at += 12 + size


def read(path):
    data = open(path, 'rb').read()
    idat, keep = b'', []
    for kind, body in chunks(data):
        if kind == b'IHDR':
            width, height, depth, color, _, _, interlace = struct.unpack('>IIBBBBB', body)
            if depth != 8 or color not in (2, 6) or interlace:
                raise ValueError('needs 8-bit RGB/RGBA, non-interlaced')
        elif kind == b'IDAT':
            idat += body
        elif kind in KEEP:
            keep.append((kind, body))
    bpp = 4 if color == 6 else 3
    raw, stride, rows, prev, at = zlib.decompress(idat), width * bpp, [], bytearray(width * bpp), 0
    for _ in range(height):
        kind, line = raw[at], bytearray(raw[at + 1:at + 1 + stride])
        at += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b, c = prev[x], prev[x - bpp] if x >= bpp else 0
            if kind == 1:
                line[x] = (line[x] + a) & 255
            elif kind == 2:
                line[x] = (line[x] + b) & 255
            elif kind == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return width, height, bpp, color, rows, keep


def chunk(kind, body):
    return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body) & 0xffffffff)


def main():
    if sys.argv[1] == '--stats':
        _, _, bpp, _, rows, _ = read(sys.argv[2])
        x, y, w, h = map(int, sys.argv[3:7])
        values = [[rows[j][(i * bpp) + c] for j in range(y, y + h) for i in range(x, x + w)] for c in range(3)]
        print({name: (min(v), max(v), round(sum(v) / len(v), 2)) for name, v in zip('RGB', values)})
        return
    source, out = sys.argv[1], sys.argv[2]
    x, y, w, h = map(int, sys.argv[3:7])
    scale = int(sys.argv[7]) if len(sys.argv) > 7 else 1
    width, height, bpp, color, rows, keep = read(source)
    if x < 0 or y < 0 or w <= 0 or h <= 0 or x + w > width or y + h > height or not 1 <= scale <= 16:
        raise ValueError('region outside image or bad scale')
    raw = bytearray()
    for j in range(y, y + h):
        line = rows[j][x * bpp:(x + w) * bpp]
        wide = b''.join(line[i * bpp:(i + 1) * bpp] * scale for i in range(w))
        for _ in range(scale):
            raw += b'\0' + wide
    header = struct.pack('>IIBBBBB', w * scale, h * scale, 8, color, 0, 0, 0)
    body = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header)
    body += b''.join(chunk(kind, data) for kind, data in keep)
    body += chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + chunk(b'IEND', b'')
    open(out, 'wb').write(body)


if __name__ == '__main__':
    main()
