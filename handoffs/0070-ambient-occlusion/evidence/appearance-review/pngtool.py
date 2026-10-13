"""Scratch PNG decode/crop/diff helper for the appearance review (not committed)."""
import hashlib, json, struct, sys, zlib
from pathlib import Path

def decode(path):
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, header = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    w, h, depth, color, _, _, interlace = header
    assert depth == 8 and color == 6 and interlace == 0, header
    raw, stride, out, prev = zlib.decompress(idat), w * 4, bytearray(), bytearray(w * 4)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b = prev[i]
            c = prev[i - 4] if i >= 4 else 0
            if f == 1: line[i] = (line[i] + a) & 255
            elif f == 2: line[i] = (line[i] + b) & 255
            elif f == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        out += line; prev = line
    return (w, h), bytes(out)

def encode(path, size, rgba):
    w, h = size
    raw = b"".join(b"\0" + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))

def crop(size, rgba, rect, scale):
    x0, y0, cw, ch = rect
    out = bytearray()
    for y in range(ch * scale):
        for x in range(cw * scale):
            i = ((y0 + y // scale) * size[0] + x0 + x // scale) * 4
            out += rgba[i:i + 4]
    return (cw * scale, ch * scale), bytes(out)

def diff(a, b, gain):
    out = bytearray()
    for i in range(0, len(a), 4):
        d = sum(a[i + c] - b[i + c] for c in range(3)) / 3
        v = min(255, int(abs(d) * gain))
        out += bytes((v, 0, 0, 255)) if d > 0 else bytes((0, 0, v, 255)) if d < 0 else bytes((0, 0, 0, 255))
    return bytes(out)
