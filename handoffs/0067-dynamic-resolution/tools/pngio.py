"""Minimal 8-bit PNG reader/writer for handoff 0067. Standard library only.

Copied unchanged in behaviour from handoff 0066 (itself from 0064 and 0062). The
reader accepts non-interlaced 8-bit RGBA or RGB images with any of the five
scanline filters and rejects everything else explicitly. The writer emits only
IHDR/IDAT/IEND with filter 0 and zlib level 9, so output is deterministic.
"""

import struct
import zlib

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _chunks(data):
    if data[:8] != SIGNATURE:
        raise ValueError("not a PNG file")
    offset = 8
    while offset < len(data):
        length, kind = struct.unpack(">I4s", data[offset:offset + 8])
        body = data[offset + 8:offset + 8 + length]
        crc = struct.unpack(">I", data[offset + 8 + length:offset + 12 + length])[0]
        if zlib.crc32(kind + body) & 0xFFFFFFFF != crc:
            raise ValueError(f"bad CRC in {kind!r}")
        yield kind, body
        offset += 12 + length


def chunk_types(data):
    return [kind.decode("ascii") for kind, _ in _chunks(data)]


def read(path_or_bytes):
    """Return (width, height, rows) where rows[y] is a bytes object of RGBA."""
    if isinstance(path_or_bytes, (bytes, bytearray)):
        data = path_or_bytes
    else:
        with open(path_or_bytes, "rb") as source:
            data = source.read()
    header = None
    idat = []
    for kind, body in _chunks(data):
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
    if header is None:
        raise ValueError("missing IHDR")
    width, height, depth, color, compression, filtering, interlace = header
    if depth != 8 or color not in (2, 6) or compression or filtering or interlace:
        raise ValueError(f"unsupported PNG format {header}")
    channels = 4 if color == 6 else 3
    stride = width * channels
    raw = zlib.decompress(b"".join(idat))
    if len(raw) != height * (stride + 1):
        raise ValueError("unexpected decompressed size")
    rows = []
    previous = bytearray(stride)
    for y in range(height):
        kind = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if kind == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 255
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 255
        elif kind == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 255
        elif kind == 4:
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = previous[i]
                c = previous[i - channels] if i >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                predictor = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + predictor) & 255
        elif kind != 0:
            raise ValueError(f"unknown filter {kind}")
        previous = line
        if channels == 3:
            line = bytearray(b"".join(bytes(line[i:i + 3]) + b"\xff" for i in range(0, stride, 3)))
        rows.append(bytes(line))
    return width, height, rows


def encode(width, height, rows):
    """Encode RGBA rows (bytes-like, width*4 each) as a deterministic PNG byte string."""
    if len(rows) != height or any(len(r) != width * 4 for r in rows):
        raise ValueError("row geometry mismatch")
    raw = b"".join(b"\x00" + bytes(r) for r in rows)

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    return (SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def write(path, width, height, rows):
    """Write RGBA rows. Refuses to overwrite."""
    data = encode(width, height, rows)
    with open(path, "xb") as out:
        out.write(data)
    return data
