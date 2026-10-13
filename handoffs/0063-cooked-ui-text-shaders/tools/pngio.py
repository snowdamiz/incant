"""Minimal 8-bit RGBA PNG reader/writer and raster helpers for handoff 0063.

Standard library only. The reader accepts non-interlaced 8-bit RGBA or RGB
images with any of the five scanline filters and rejects everything else
explicitly. The writer emits only IHDR/IDAT/IEND with filter 0 and zlib level 9,
so derived figures are deterministic. Figures are crops and nearest-neighbour
zooms (pixel replication) of captures, never re-rendered or resampled.
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


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    return a if pa <= pb and pa <= pc else (b if pb <= pc else c)


def decode(data):
    """Return (width, height, rgba bytes)."""
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
                c = previous[i - channels] if i >= channels else 0
                line[i] = (line[i] + _paeth(a, previous[i], c)) & 255
        elif kind != 0:
            raise ValueError(f"unknown filter {kind}")
        previous = line
        if channels == 3:
            line = bytearray(b"".join(bytes(line[i:i + 3]) + b"\xff" for i in range(0, stride, 3)))
        out += line
    return width, height, bytes(out)


def encode(width, height, rgba):
    if len(rgba) != width * height * 4:
        raise ValueError("pixel geometry mismatch")
    stride = width * 4
    raw = b"".join(b"\x00" + rgba[y * stride:(y + 1) * stride] for y in range(height))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    return (SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


class Raster:
    def __init__(self, width, height, rgba):
        self.width, self.height, self.rgba = width, height, bytes(rgba)

    @classmethod
    def load(cls, path):
        with open(path, "rb") as source:
            return cls(*decode(source.read()))

    @classmethod
    def filled(cls, width, height, color):
        return cls(width, height, bytes(color) * (width * height))

    def save(self, path):
        data = encode(self.width, self.height, self.rgba)
        with open(path, "xb") as out:
            out.write(data)
        return data

    def pixel(self, x, y):
        i = 4 * (y * self.width + x)
        return tuple(self.rgba[i:i + 4])

    def crop(self, x, y, w, h):
        w, h = min(w, self.width - x), min(h, self.height - y)
        stride = self.width * 4
        return Raster(w, h, b"".join(self.rgba[(y + r) * stride + 4 * x:(y + r) * stride + 4 * (x + w)]
                                     for r in range(h)))

    def zoom(self, factor):
        rows = []
        for y in range(self.height):
            row = self.rgba[y * self.width * 4:(y + 1) * self.width * 4]
            wide = b"".join(row[i:i + 4] * factor for i in range(0, len(row), 4))
            rows.extend([wide] * factor)
        return Raster(self.width * factor, self.height * factor, b"".join(rows))

    def paste(self, other, x, y):
        out = bytearray(self.rgba)
        for r in range(other.height):
            start = ((y + r) * self.width + x) * 4
            out[start:start + other.width * 4] = other.rgba[r * other.width * 4:(r + 1) * other.width * 4]
        return Raster(self.width, self.height, out)


def row_of(tiles, gap=6, color=(255, 255, 255, 255)):
    width = sum(t.width for t in tiles) + gap * (len(tiles) - 1)
    height = max(t.height for t in tiles)
    canvas, x = Raster.filled(width, height, color), 0
    for tile in tiles:
        canvas = canvas.paste(tile, x, 0)
        x += tile.width + gap
    return canvas


def column_of(tiles, gap=6, color=(255, 255, 255, 255)):
    width = max(t.width for t in tiles)
    height = sum(t.height for t in tiles) + gap * (len(tiles) - 1)
    canvas, y = Raster.filled(width, height, color), 0
    for tile in tiles:
        canvas = canvas.paste(tile, 0, y)
        y += tile.height + gap
    return canvas


def diff(a, b, region=None):
    """Per-channel comparison of two equal-size rasters inside region [x0, y0, x1, y1) (default: full frame).
    Returns differing pixel count, max channel delta, histogram of max deltas and the bounding box."""
    if (a.width, a.height) != (b.width, b.height):
        raise ValueError("size mismatch")
    x0, y0, x1, y1 = region or (0, 0, a.width, a.height)
    count, worst, histogram, box = 0, 0, {}, None
    for y in range(y0, y1):
        base = (y * a.width) * 4
        ra, rb = a.rgba[base + 4 * x0:base + 4 * x1], b.rgba[base + 4 * x0:base + 4 * x1]
        if ra == rb:
            continue
        for i in range(0, len(ra), 4):
            if ra[i:i + 4] != rb[i:i + 4]:
                delta = max(abs(ra[i + k] - rb[i + k]) for k in range(4))
                count += 1
                worst = max(worst, delta)
                histogram[delta] = histogram.get(delta, 0) + 1
                x = x0 + i // 4
                box = [min(box[0], x), min(box[1], y), max(box[2], x + 1), max(box[3], y + 1)] if box else \
                    [x, y, x + 1, y + 1]
    return {"differing_pixels": count, "max_channel_delta": worst,
            "delta_histogram": {str(k): histogram[k] for k in sorted(histogram)}, "bounding_box": box}
