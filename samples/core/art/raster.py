"""Float image helpers for texture generation: low-res fields, bilinear sampling,
sRGB conversion and normal maps. Images are lists of rows of float tuples.
"""
import math


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def linear_to_srgb(c):
    c = max(0.0, min(1.0, c))
    return c * 12.92 if c <= 0.0031308 else 1.055 * c ** (1.0 / 2.4) - 0.055


def hex_rgb(value):
    value = value.lstrip('#')
    return tuple(int(value[i:i + 2], 16) / 255.0 for i in (0, 2, 4))


def hex_linear(value):
    return tuple(srgb_to_linear(c) for c in hex_rgb(value))


def field(width, height, fn):
    """Evaluate fn(u, v) on a width x height grid of cell centres (u, v in 0..1)."""
    return [[fn((x + 0.5) / width, (y + 0.5) / height) for x in range(width)] for y in range(height)]


def sample(grid, u, v, wrap_u=True, wrap_v=True):
    """Bilinear sample of a scalar grid at normalized (u, v)."""
    h, w = len(grid), len(grid[0])
    x, y = u * w - 0.5, v * h - 0.5
    x0, y0 = math.floor(x), math.floor(y)
    fx, fy = x - x0, y - y0

    def at(ix, iy):
        ix = ix % w if wrap_u else min(max(ix, 0), w - 1)
        iy = iy % h if wrap_v else min(max(iy, 0), h - 1)
        return grid[iy][ix]
    a = at(x0, y0) + (at(x0 + 1, y0) - at(x0, y0)) * fx
    b = at(x0, y0 + 1) + (at(x0 + 1, y0 + 1) - at(x0, y0 + 1)) * fx
    return a + (b - a) * fy


def to_rows(image, channels=3, encode_srgb=True):
    """Float linear image (rows of tuples) to PNG byte rows."""
    rows = []
    for line in image:
        row = bytearray()
        for px in line:
            for c in range(channels):
                value = px[c]
                if encode_srgb and c < 3:
                    value = linear_to_srgb(value)
                row.append(max(0, min(255, int(value * 255.0 + 0.5))))
        rows.append(bytes(row))
    return rows


def normal_rows(height_grid, strength, wrap_u=True, wrap_v=True):
    """Tangent-space normal map rows from a height grid (texel units)."""
    h, w = len(height_grid), len(height_grid[0])
    rows = []
    for y in range(h):
        row = bytearray()
        for x in range(w):
            def at(ix, iy):
                ix = ix % w if wrap_u else min(max(ix, 0), w - 1)
                iy = iy % h if wrap_v else min(max(iy, 0), h - 1)
                return height_grid[iy][ix]
            dx = (at(x + 1, y) - at(x - 1, y)) * 0.5 * strength
            dy = (at(x, y + 1) - at(x, y - 1)) * 0.5 * strength
            nx, ny, nz = -dx, -dy, 1.0
            n = math.sqrt(nx * nx + ny * ny + nz * nz)
            for c in (nx / n, ny / n, nz / n):
                row.append(max(0, min(255, int((c * 0.5 + 0.5) * 255.0 + 0.5))))
        rows.append(bytes(row))
    return rows
