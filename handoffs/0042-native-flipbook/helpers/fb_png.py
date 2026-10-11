"""Minimal standard-library PNG I/O and a 3x5 glyph font for handoff 0042.

Reads 8-bit RGB/RGBA non-interlaced PNGs (all five filter types) and writes 8-bit
RGBA. Images are row-major lists of bytes rows; px(x, y) returns (r, g, b, a).
"""

import struct
import zlib
from pathlib import Path

SIGNATURE = b"\x89PNG\r\n\x1a\n"

FONT = {
    "0": ["###", "#.#", "#.#", "#.#", "###"], "1": [".#.", "##.", ".#.", ".#.", "###"],
    "2": ["###", "..#", "###", "#..", "###"], "3": ["###", "..#", "###", "..#", "###"],
    "4": ["#.#", "#.#", "###", "..#", "..#"], "5": ["###", "#..", "###", "..#", "###"],
    "6": ["###", "#..", "###", "#.#", "###"], "7": ["###", "..#", "..#", "..#", "..#"],
    "8": ["###", "#.#", "###", "#.#", "###"], "9": ["###", "#.#", "###", "..#", "###"],
    "A": ["###", "#.#", "###", "#.#", "#.#"], "B": ["##.", "#.#", "##.", "#.#", "##."],
    "C": ["###", "#..", "#..", "#..", "###"], "D": ["##.", "#.#", "#.#", "#.#", "##."],
    "E": ["###", "#..", "##.", "#..", "###"], "F": ["###", "#..", "##.", "#..", "#.."],
    "G": ["###", "#..", "#.#", "#.#", "###"], "H": ["#.#", "#.#", "###", "#.#", "#.#"],
    "I": ["###", ".#.", ".#.", ".#.", "###"], "K": ["#.#", "#.#", "##.", "#.#", "#.#"],
    "L": ["#..", "#..", "#..", "#..", "###"], "M": ["#.#", "###", "###", "#.#", "#.#"],
    "N": ["##.", "#.#", "#.#", "#.#", "#.#"], "O": ["###", "#.#", "#.#", "#.#", "###"],
    "P": ["###", "#.#", "###", "#..", "#.."], "R": ["##.", "#.#", "##.", "#.#", "#.#"],
    "S": ["###", "#..", "###", "..#", "###"], "T": ["###", ".#.", ".#.", ".#.", ".#."],
    "U": ["#.#", "#.#", "#.#", "#.#", "###"], "V": ["#.#", "#.#", "#.#", "#.#", ".#."],
    "W": ["#.#", "#.#", "###", "###", "#.#"], "X": ["#.#", "#.#", ".#.", "#.#", "#.#"],
    "Y": ["#.#", "#.#", ".#.", ".#.", ".#."], "Z": ["###", "..#", ".#.", "#..", "###"],
    "-": ["...", "...", "###", "...", "..."], ".": ["...", "...", "...", "...", ".#."],
    "/": ["..#", "..#", ".#.", "#..", "#.."], ":": ["...", ".#.", "...", ".#.", "..."],
    "=": ["...", "###", "...", "###", "..."], "+": ["...", ".#.", "###", ".#.", "..."],
    " ": ["...", "...", "...", "...", "..."], "*": ["#.#", ".#.", "###", ".#.", "#.#"],
}


def _chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def encode_rgba(width, height, rows):
    """rows: height sequences of width*4 bytes (or lists of RGBA tuples)."""
    raw = bytearray()
    for row in rows:
        raw.append(0)
        if row and isinstance(row[0], tuple):
            for p in row:
                raw.extend(bytes(p))
        else:
            raw.extend(row)
    assert len(raw) == height * (4 * width + 1)
    return (SIGNATURE + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + _chunk(b"IEND", b""))


def write_rgba(path, width, height, rows):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_bytes(encode_rgba(width, height, rows))


class Image:
    def __init__(self, width, height, rows):
        self.width, self.height, self.rows = width, height, rows

    def px(self, x, y):
        row = self.rows[y]
        return tuple(row[4 * x:4 * x + 4])

    def crop(self, x0, y0, w, h):
        """Crop clipped to the image; outside pixels are transparent black."""
        out = []
        for y in range(y0, y0 + h):
            if 0 <= y < self.height:
                lo, hi = max(x0, 0), min(x0 + w, self.width)
                row = bytes(4 * (lo - x0)) + self.rows[y][4 * lo:4 * hi] + bytes(4 * (x0 + w - hi)) \
                    if hi > lo else bytes(4 * w)
            else:
                row = bytes(4 * w)
            out.append(row)
        return Image(w, h, out)

    def key(self):
        return b"".join(self.rows)


def _unfilter(kind, line, prev, bpp):
    n = len(line)
    if kind == 0:
        return line
    if kind == 2:
        return bytearray((a + b) & 255 for a, b in zip(line, prev))
    if kind == 1:
        for i in range(bpp, n):
            line[i] = (line[i] + line[i - bpp]) & 255
        return line
    if kind == 3:
        for i in range(n):
            left = line[i - bpp] if i >= bpp else 0
            line[i] = (line[i] + ((left + prev[i]) >> 1)) & 255
        return line
    if kind == 4:
        for i in range(n):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            p = a + b - c
            pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
            line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        return line
    raise ValueError(f"unknown PNG filter {kind}")


def read(path):
    data = Path(path).read_bytes()
    if data[:8] != SIGNATURE:
        raise ValueError(f"{path}: not a PNG")
    pos, idat, header = 8, bytearray(), None
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
    raw = zlib.decompress(bytes(idat))
    stride = width * bpp
    rows, prev = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        line = _unfilter(raw[start], bytearray(raw[start + 1:start + 1 + stride]), prev, bpp)
        rows.append(bytes(line))
        prev = line
    if bpp == 3:
        rows = [bytes(b for i in range(width) for b in (*r[3 * i:3 * i + 3], 255)) for r in rows]
    return Image(width, height, rows)
