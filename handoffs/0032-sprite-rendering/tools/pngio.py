"""Minimal standard-library PNG I/O for handoff 0032. 8-bit RGBA/RGB, non-interlaced."""

import struct
import zlib
from pathlib import Path

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def encode_rgba(width, height, pixels):
    """pixels: row-major list of (r, g, b, a) 0..255 tuples."""
    assert len(pixels) == width * height
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for p in pixels[y * width:(y + 1) * width]:
            raw.extend(bytes(p))
    return (SIGNATURE + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + _chunk(b"IEND", b""))


def write_rgba(path, width, height, pixels, overwrite=False):
    path = Path(path)
    if path.exists() and not overwrite:
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_bytes(encode_rgba(width, height, pixels))


class Image:
    """Decoded 8-bit image. px(x, y) -> (r, g, b, a)."""

    def __init__(self, width, height, rows):
        self.width, self.height, self.rows = width, height, rows

    def px(self, x, y):
        row = self.rows[y]
        return (row[4 * x], row[4 * x + 1], row[4 * x + 2], row[4 * x + 3])

    def rgb(self, x, y):
        row = self.rows[y]
        return (row[4 * x], row[4 * x + 1], row[4 * x + 2])

    def crop(self, x0, y0, w, h):
        return Image(w, h, [bytes(self.rows[y][4 * x0:4 * (x0 + w)]) for y in range(y0, y0 + h)])

    def pixels(self):
        return [self.px(x, y) for y in range(self.height) for x in range(self.width)]


def read(path):
    data = Path(path).read_bytes()
    if data[:8] != SIGNATURE:
        raise ValueError(f"{path}: not a PNG")
    pos, idat, header = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    width, height, depth, ctype, _, _, interlace = header
    if depth != 8 or ctype not in (2, 6) or interlace:
        raise ValueError(f"{path}: unsupported PNG depth={depth} type={ctype} interlace={interlace}")
    bpp = 4 if ctype == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * bpp
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if f == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + (a + prev[i]) // 2) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                b = prev[i]
                c = prev[i - bpp] if i >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    if bpp == 3:
        rows = [bytes(b for i in range(width) for b in (*r[3 * i:3 * i + 3], 255)) for r in rows]
    return Image(width, height, rows)
