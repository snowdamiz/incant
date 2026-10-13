"""CPU-only image helpers for derived review figures (crops, zooms, diff maps).

Derived figures are never captures: they are computed from existing PNG files
and are labelled as derived wherever they are stored.
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402


class Raster:
    def __init__(self, width, height, rows):
        self.width, self.height = width, height
        self.rows = [bytearray(r) for r in rows]

    @classmethod
    def load(cls, path):
        return cls(*pngio.read(path))

    @classmethod
    def blank(cls, width, height, color=(0, 0, 0, 255)):
        return cls(width, height, [bytes(color) * width for _ in range(height)])

    def get(self, x, y):
        return tuple(self.rows[y][4 * x:4 * x + 4])

    def put(self, x, y, color):
        self.rows[y][4 * x:4 * x + 4] = bytes(color)

    def crop(self, x, y, w, h):
        return Raster(w, h, [bytes(self.rows[yy][4 * x:4 * (x + w)]) for yy in range(y, y + h)])

    def zoom(self, factor):
        rows = []
        for row in self.rows:
            line = b"".join(bytes(row[4 * x:4 * x + 4]) * factor for x in range(self.width))
            rows.extend([line] * factor)
        return Raster(self.width * factor, self.height * factor, rows)

    def over_checker(self, cell=4):
        """Composite straight-alpha RGBA over a grey checkerboard (source art previews only)."""
        out = Raster.blank(self.width, self.height)
        for y in range(self.height):
            for x in range(self.width):
                r, g, b, a = self.get(x, y)
                bg = 200 if ((x // cell) + (y // cell)) % 2 == 0 else 150
                out.put(x, y, tuple(round((c * a + bg * (255 - a)) / 255) for c in (r, g, b)) + (255,))
        return out

    def paste(self, other, x0, y0):
        for y in range(other.height):
            self.rows[y0 + y][4 * x0:4 * (x0 + other.width)] = other.rows[y]

    def save(self, path):
        pngio.write(path, self.width, self.height, [bytes(r) for r in self.rows])


def row_of(images, gap=4, color=(255, 255, 255, 255)):
    width = sum(i.width for i in images) + gap * (len(images) + 1)
    height = max(i.height for i in images) + 2 * gap
    out = Raster.blank(width, height, color)
    x = gap
    for image in images:
        out.paste(image, x, gap)
        x += image.width + gap
    return out


def column_of(images, gap=4, color=(255, 255, 255, 255)):
    width = max(i.width for i in images) + 2 * gap
    height = sum(i.height for i in images) + gap * (len(images) + 1)
    out = Raster.blank(width, height, color)
    y = gap
    for image in images:
        out.paste(image, gap, y)
        y += image.height + gap
    return out


def diff_map(a, b, scale=32):
    """Black where equal; otherwise grey/white by the largest channel delta times scale."""
    out = Raster.blank(a.width, a.height)
    for y in range(a.height):
        for x in range(a.width):
            delta = max(abs(p - q) for p, q in zip(a.get(x, y), b.get(x, y)))
            if delta:
                v = min(255, 64 + delta * scale)
                out.put(x, y, (v, v, v, 255))
    return out
