#!/usr/bin/env python3
"""Tile engine PNG frames into one review sheet (stdlib only; 8-bit RGB/RGBA PNGs).

Usage: python3 -I contact_sheet.py OUT.png COLUMNS FRAME.png [FRAME.png ...]

Pixels are copied unchanged from the engine's frames; nothing is resampled or
drawn. OUT.png must not exist.
"""
import struct
import sys
import zlib
from pathlib import Path


def read_png(path):
    data = Path(path).read_bytes()
    i, idat, width, height, kind = 8, b"", 0, 0, 0
    while i < len(data):
        n = struct.unpack(">I", data[i:i + 4])[0]
        tag, chunk = data[i + 4:i + 8], data[i + 8:i + 8 + n]
        i += 12 + n
        if tag == b"IHDR":
            width, height, depth, kind = struct.unpack(">IIBB", chunk[:10])
            if depth != 8 or kind not in (2, 6) or chunk[12] != 0:
                raise SystemExit(f"{path}: only non-interlaced 8-bit RGB/RGBA PNGs are supported")
        elif tag == b"IDAT":
            idat += chunk
    bpp = 4 if kind == 6 else 3
    raw, stride, rows, prev, o = zlib.decompress(idat), width * bpp, [], bytearray(width * bpp), 0
    for _ in range(height):
        kind_filter, line = raw[o], bytearray(raw[o + 1:o + 1 + stride])
        o += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if kind_filter == 1:
                line[x] = (line[x] + a) & 255
            elif kind_filter == 2:
                line[x] = (line[x] + b) & 255
            elif kind_filter == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind_filter == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line) if bpp == 3 else bytes(v for k, v in enumerate(line) if k % 4 != 3))
        prev = line
    return width, height, rows


def write_png(path, width, height, rows):
    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)
    raw = b"".join(b"\x00" + row for row in rows)
    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def main():
    if len(sys.argv) < 4:
        raise SystemExit(__doc__)
    out, columns, frames = Path(sys.argv[1]), int(sys.argv[2]), sys.argv[3:]
    if out.exists():
        raise SystemExit(f"{out} already exists")
    images = [read_png(f) for f in frames]
    width, height = images[0][0], images[0][1]
    if any(img[0] != width or img[1] != height for img in images):
        raise SystemExit("frames differ in size")
    blank = bytes(width * 3)
    sheet = []
    for r in range((len(images) + columns - 1) // columns):
        for y in range(height):
            sheet.append(b"".join(images[r * columns + c][2][y] if r * columns + c < len(images) else blank
                                  for c in range(columns)))
    write_png(out, width * columns, len(sheet), sheet)


if __name__ == "__main__":
    main()
