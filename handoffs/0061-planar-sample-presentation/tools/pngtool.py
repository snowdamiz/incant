#!/usr/bin/env python3
"""Standard-library PNG helpers for pixel review (no PIL on this host).

  pngtool.py zoom IN.png ZOOM OUT.png            nearest-neighbour enlargement;
                                                 transparency over a checkerboard
  pngtool.py crop IN.png X Y W H ZOOM OUT.png    crop, then enlarge
  pngtool.py diff A.png B.png                    count differing pixels, max delta
  pngtool.py stats IN.png                        size, unique colors, hash
Reads 8-bit RGB/RGBA non-interlaced PNGs, the formats the engine and art write.
"""
import hashlib
import struct
import sys
import zlib


def read(path):
    data = open(path, 'rb').read()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError(f'{path}: not a PNG')
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
    if depth != 8 or color not in (2, 6) or interlace:
        raise ValueError(f'{path}: expects 8-bit RGB/RGBA, non-interlaced')
    channels = 4 if color == 6 else 3
    raw, stride = zlib.decompress(idat), width * channels
    previous, pixels = bytearray(stride), []
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = previous[i]
            c = previous[i - channels] if i >= channels else 0
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
        previous = line
        row = []
        for x in range(width):
            px = line[x * channels:(x + 1) * channels]
            row.append(tuple(px) if channels == 4 else (*px, 255))
        pixels.append(row)
    return pixels


def write(path, pixels):
    height, width = len(pixels), len(pixels[0])
    raw = b''.join(b'\x00' + bytes(v for p in row for v in p) for row in pixels)

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))
    with open(path, 'wb') as out:
        out.write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
                  + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def flatten(pixels, checker=8):
    out = []
    for y, row in enumerate(pixels):
        line = []
        for x, (r, g, b, a) in enumerate(row):
            bg = 70 if ((x // checker) + (y // checker)) % 2 else 92
            k = a / 255
            line.append((round(r * k + bg * (1 - k)), round(g * k + bg * (1 - k)),
                         round(b * k + bg * (1 - k)), 255))
        out.append(line)
    return out


def zoom(pixels, factor):
    return [[p for p in row for _ in range(factor)] for row in pixels for _ in range(factor)]


def main(argv):
    command = argv[0]
    if command == 'zoom':
        source, factor, target = argv[1], int(argv[2]), argv[3]
        write(target, zoom(flatten(read(source)), factor))
    elif command == 'crop':
        source, x, y, w, h, factor, target = argv[1], *map(int, argv[2:7]), argv[7]
        pixels = [row[x:x + w] for row in read(source)[y:y + h]]
        write(target, zoom(flatten(pixels), factor))
    elif command == 'diff':
        a, b = read(argv[1]), read(argv[2])
        if (len(a), len(a[0])) != (len(b), len(b[0])):
            print('size mismatch')
            return 1
        deltas = [max(abs(u - v) for u, v in zip(p, q)) for ra, rb in zip(a, b) for p, q in zip(ra, rb)]
        print({'pixels': len(deltas), 'differing': sum(d > 0 for d in deltas), 'max_delta': max(deltas)})
    elif command == 'stats':
        pixels = read(argv[1])
        colors = {p for row in pixels for p in row}
        flat = bytes(v for row in pixels for p in row for v in p)
        print({'size': [len(pixels[0]), len(pixels)], 'unique_colors': len(colors),
               'rgba_sha256': hashlib.sha256(flat).hexdigest()})
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
