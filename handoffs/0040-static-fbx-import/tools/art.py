"""Original review art for handoff 0040 (Claude, generated; no external sources).

The Maya cube fixture uses Maya's default cube UV layout. In this 512x512 image
(top-left origin, FBX v up -> image row (1 - v) * 512) each face owns a 128x128
island. Every island carries an asymmetric glyph ("F" in v1, "G" in v2), the face's
outward axis in Maya space, a red top-left and a yellow top-right corner mark, so a
mirrored, rotated or flipped UV mapping is visible at a glance.

The normal map uses the OpenGL/glTF convention (R = +U to the image right, G = toward
the image top, B = out of the surface). Each island holds the same raised frustum
("button"): a flat top and four 35-degree bevels. Under a light from the upper right,
a correctly decoded button is brightest on its top bevel and darkest on its bottom
bevel; an inverted green channel or tangent handedness swaps those.
"""
import math

import pngio

SIZE = 512
ISLAND = 128
# (name, outward axis label in Maya space, image x0, image y0, colour)
ISLANDS = [
    ("front", "+Z", 192, 384, (236, 140, 62)),
    ("top", "+Y", 192, 256, (64, 170, 160)),
    ("back", "-Z", 192, 128, (150, 110, 200)),
    ("bottom", "-Y", 192, 0, (160, 160, 70)),
    ("right", "+X", 320, 384, (80, 130, 220)),
    ("left", "-X", 64, 384, (110, 180, 90)),
]

FONT = {  # 5x7 bitmap glyphs, top row first
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "G": ["01110", "10001", "10000", "10111", "10001", "10001", "01111"],
    "X": ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
    "Y": ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
    "Z": ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
    "+": ["00000", "00100", "00100", "11111", "00100", "00100", "00000"],
    "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
    "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
}


class Canvas:
    def __init__(self, width, height, color):
        self.width, self.height = width, height
        self.pixels = bytearray(bytes(color) * (width * height))

    def rect(self, x0, y0, w, h, color):
        for y in range(max(0, y0), min(self.height, y0 + h)):
            row = (y * self.width) * 3
            for x in range(max(0, x0), min(self.width, x0 + w)):
                self.pixels[row + x * 3:row + x * 3 + 3] = bytes(color)

    def text(self, x, y, text, scale, color):
        for glyph in text:
            for gy, line in enumerate(FONT[glyph]):
                for gx, bit in enumerate(line):
                    if bit == "1":
                        self.rect(x + gx * scale, y + gy * scale, scale, scale, color)
            x += 6 * scale

    def save(self, path):
        return pngio.write(path, self.width, self.height, self.pixels, 3)


def face_art(path, glyph="F", tint=None):
    """Base-colour art. `tint` replaces every island colour (used for the v2 art)."""
    canvas = Canvas(SIZE, SIZE, (40, 40, 44))
    for _, label, x0, y0, color in ISLANDS:
        color = tint or color
        canvas.rect(x0, y0, ISLAND, ISLAND, (245, 245, 240))           # seam border
        canvas.rect(x0 + 3, y0 + 3, ISLAND - 6, ISLAND - 6, color)
        canvas.rect(x0 + 8, y0 + 8, 14, 14, (220, 30, 30))              # top-left mark
        canvas.rect(x0 + ISLAND - 22, y0 + 8, 14, 14, (250, 220, 40))   # top-right mark
        canvas.text(x0 + 34, y0 + 10, label, 4, (20, 20, 20))           # axis label
        canvas.text(x0 + 46, y0 + 48, glyph, 10, (15, 15, 15))          # big glyph
    return canvas.save(path)


BEVEL_DEGREES = 35.0
TOP_HALF = 24      # flat top half-size in texels (island units)
OUTER_HALF = 56    # bevel outer half-size in texels


def _encode(n):
    return tuple(max(0, min(255, int(round((c * 0.5 + 0.5) * 255)))) for c in n)


def bevel_normal(dx, dy):
    """Tangent-space normal at island offset (dx right, dy up) from the island centre."""
    if max(abs(dx), abs(dy)) > OUTER_HALF or max(abs(dx), abs(dy)) <= TOP_HALF:
        return (0.0, 0.0, 1.0)
    s, c = math.sin(math.radians(BEVEL_DEGREES)), math.cos(math.radians(BEVEL_DEGREES))
    if abs(dx) > abs(dy):
        return (s if dx > 0 else -s, 0.0, c)
    return (0.0, s if dy > 0 else -s, c)


def normal_map(path):
    pixels = bytearray()
    owner = {}
    for _, _, x0, y0, _ in ISLANDS:
        for y in range(y0, y0 + ISLAND):
            for x in range(x0, x0 + ISLAND):
                owner[(x, y)] = (x0, y0)
    flat = _encode((0.0, 0.0, 1.0))
    for y in range(SIZE):
        for x in range(SIZE):
            island = owner.get((x, y))
            if island is None:
                pixels += bytes(flat)
                continue
            # Texel centre offsets; image rows grow downward, so "up" is -y.
            dx = x + 0.5 - (island[0] + ISLAND / 2)
            dy = (island[1] + ISLAND / 2) - (y + 0.5)
            pixels += bytes(_encode(bevel_normal(dx, dy)))
    return pngio.write(path, SIZE, SIZE, pixels, 3)
