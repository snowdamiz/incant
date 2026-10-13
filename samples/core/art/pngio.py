"""Deterministic 8-bit PNG encoding and decoding with the standard library only.

Images are row-major lists of rows; every pixel is a tuple of 3 (RGB) or 4
(RGBA) integers in 0..255. The encoder uses filter 0 and zlib level 9, so equal
pixels always produce equal bytes. The decoder accepts the PNGs this toolkit and
the engine's review tools write (8-bit RGB/RGBA, non-interlaced, any filter).
"""
import struct
import zlib


def _chunk(kind, data):
    body = kind + data
    return struct.pack('>I', len(data)) + body + struct.pack('>I', zlib.crc32(body) & 0xFFFFFFFF)


def encode(width, height, channels, rows):
    """Encode raw rows (bytes-like, width*channels each) as PNG bytes."""
    if channels not in (3, 4):
        raise ValueError('PNG channels must be 3 or 4')
    raw = bytearray()
    for row in rows:
        if len(row) != width * channels:
            raise ValueError('PNG row length mismatch')
        raw.append(0)
        raw.extend(row)
    if len(raw) != height * (width * channels + 1):
        raise ValueError('PNG row count mismatch')
    header = struct.pack('>IIBBBBB', width, height, 8, 6 if channels == 4 else 2, 0, 0, 0)
    return (b'\x89PNG\r\n\x1a\n' + _chunk(b'IHDR', header)
            + _chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + _chunk(b'IEND', b''))


def write(path, width, height, channels, rows):
    data = encode(width, height, channels, rows)
    with open(path, 'wb') as stream:
        stream.write(data)
    return data


def from_float(width, height, pixel, channels=3):
    """Rows from a callback pixel(x, y) returning floats in 0..1 (display-encoded)."""
    rows = []
    for y in range(height):
        row = bytearray()
        for x in range(width):
            value = pixel(x, y)
            for c in range(channels):
                row.append(max(0, min(255, int(value[c] * 255.0 + 0.5))))
        rows.append(bytes(row))
    return rows


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def decode(data):
    """Return (width, height, channels, rows) for 8-bit RGB/RGBA PNG bytes."""
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('not a PNG')
    offset, idat, header = 8, bytearray(), None
    while offset < len(data):
        length, kind = struct.unpack('>I4s', data[offset:offset + 8])
        body = data[offset + 8:offset + 8 + length]
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat.extend(body)
        elif kind == b'IEND':
            break
        offset += 12 + length
    width, height, depth, color, _, _, interlace = header
    if depth != 8 or color not in (2, 6) or interlace:
        raise ValueError('unsupported PNG layout')
    channels = 4 if color == 6 else 3
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    rows, previous = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            corner = previous[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + left) & 255
            elif kind == 2:
                line[i] = (line[i] + up) & 255
            elif kind == 3:
                line[i] = (line[i] + ((left + up) >> 1)) & 255
            elif kind == 4:
                line[i] = (line[i] + _paeth(left, up, corner)) & 255
        rows.append(bytes(line))
        previous = line
    return width, height, channels, rows
