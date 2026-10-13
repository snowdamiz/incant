"""Minimal PNG reader for handoff 0072's CPU comparison. Standard library only.

Adapted read-only from handoff 0067's pngio.read (itself from 0066, 0064 and 0062):
non-interlaced 8-bit RGB or RGBA with any of the five scanline filters; every other
format is refused explicitly, as are bad CRCs and truncated data.
"""

import struct
import zlib

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _chunks(data):
    if data[:8] != SIGNATURE:
        raise ValueError("not a PNG file")
    offset = 8
    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError("truncated chunk header")
        length, kind = struct.unpack(">I4s", data[offset:offset + 8])
        end = offset + 12 + length
        if end > len(data):
            raise ValueError("truncated chunk")
        body = data[offset + 8:offset + 8 + length]
        crc = struct.unpack(">I", data[offset + 8 + length:end])[0]
        if zlib.crc32(kind + body) & 0xFFFFFFFF != crc:
            raise ValueError(f"bad CRC in {kind!r}")
        yield kind, body
        offset = end


def read(path):
    """Return (width, height, rgba) with rgba a bytes object of width*height*4."""
    with open(path, "rb") as source:
        data = source.read()
    header, idat = None, []
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
    out = bytearray()
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
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else (b if pb <= pc else c))) & 255
        elif kind != 0:
            raise ValueError(f"unknown filter {kind}")
        previous = line
        if channels == 3:
            line = bytearray(b"".join(bytes(line[i:i + 3]) + b"\xff" for i in range(0, stride, 3)))
        out += line
    return width, height, bytes(out)
