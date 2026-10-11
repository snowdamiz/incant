"""Minimal 8-bit PNG reader/writer (standard library only) for handoff 0040.

Reads non-interlaced 8-bit gray, RGB and RGBA images (the engine writes RGBA8) and
writes RGB or RGBA images with filter 0, so generated art is byte-reproducible.
"""
import struct
import zlib
from pathlib import Path

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def write(path, width, height, pixels, channels=3):
    """pixels: flat sequence of width*height*channels ints in 0..255, top row first."""
    if len(pixels) != width * height * channels or channels not in (3, 4):
        raise ValueError("pixel buffer does not match the image size")
    stride = width * channels
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw += bytes(pixels[y * stride:(y + 1) * stride])
    color_type = 2 if channels == 3 else 6
    data = (SIGNATURE + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, color_type, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + _chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)
    return data


def dimensions(data):
    if data[:8] != SIGNATURE or data[12:16] != b"IHDR":
        raise ValueError("not a PNG")
    return struct.unpack(">II", data[16:24])


def read(path_or_bytes):
    """Return (width, height, channels, bytearray of pixels, top row first)."""
    data = path_or_bytes if isinstance(path_or_bytes, (bytes, bytearray)) else Path(path_or_bytes).read_bytes()
    if data[:8] != SIGNATURE:
        raise ValueError("not a PNG")
    pos, idat, header = 8, bytearray(), None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
        pos += 12 + length
    width, height, depth, color_type, _, _, interlace = header
    channels = {0: 1, 2: 3, 6: 4}.get(color_type)
    if depth != 8 or channels is None or interlace != 0:
        raise ValueError(f"unsupported PNG layout depth={depth} type={color_type} interlace={interlace}")
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    out = bytearray(height * stride)
    prev = bytearray(stride)
    for y in range(height):
        filt = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if filt == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 255
        elif filt == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif filt == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 255
        elif filt == 4:
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 255
        elif filt != 0:
            raise ValueError(f"bad PNG filter {filt}")
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return width, height, channels, out
