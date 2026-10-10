#!/usr/bin/env python3
"""Original pixel art for handoff 0032, generated from code. Standard library only.

Everything here is drawn from scratch for this packet: a 16 px pixel rendition of
the project's own Wisp mascot (four idle frames), an asymmetric nine-slice dialog
panel, an orientation key, a soft glow orb with hidden RGB under zero alpha, a
guarded crate tile, two ground tiles, sky/fog strips, tone and alpha swatches, a
3x5 label font and a gem badge used for reimport tests.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/sprite_art.py OUT_DIR [--preview]

Writes the source PNGs plus regions.json into OUT_DIR, which must not exist.
"""

import argparse
import json
import math
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

CLEAR = (0, 0, 0, 0)
HIDDEN = (255, 0, 255, 0)  # zero-alpha texels carry garbage RGB on purpose
MAGENTA = (255, 0, 255, 255)  # opaque bleed guard outside a region

# Wisp palette (sRGB). Eye whites are deliberately full white.
WISP = {
    ".": CLEAR,
    "o": (54, 32, 132, 255),    # outline, deep purple (reads on dark night backgrounds)
    "s": (72, 46, 178, 255),    # shade
    "b": (96, 64, 216, 255),    # body, close to the mascot purple
    "h": (138, 112, 238, 255),  # highlight
    "l": (190, 174, 248, 255),  # glint and flame tip
    "w": (255, 255, 255, 255),  # eye white (above the tone-curve shoulder on purpose)
}
# Teal variant used to prove whole-sheet reimport. Same structure, new colours.
WISP_TEAL = dict(WISP, o=(16, 70, 76, 255), s=(24, 120, 124, 255), b=(40, 164, 160, 255),
                 h=(96, 208, 192, 255), l=(176, 238, 222, 255))

# 16 x 16 silhouette rasterized from the mascot's vector trace in handoff 0004 at
# 50% coverage, then hand-finished. The flame hooks to the upper right, the eyes
# sit left of centre and the right eye is lower: asymmetric in both axes.
WISP_BODY = [
    "......#######...",
    "....########....",
    "...########.....",
    "...#########....",
    "..###########...",
    "..###########...",
    "..############..",
    "...###########..",
    "...##########...",
    ".....########...",
]
WISP_TIPS = {
    "base": ["...........#....", "...........##...", "...........###..", ".........#####.."],
    "left": ["..........#.....", "..........###...", "...........###..", ".........#####.."],
    "right": ["............#...", "...........##...", "...........###..", ".........#####.."],
}
WISP_TAILS = {
    "base": ["........####....", ".........#......"],
    "left": ["........###.....", "........#......."],
    "right": ["........####....", "..........#....."],
}
WISP_EYES = [(4, 9), (4, 10), (8, 10), (8, 11)]
WISP_BLINK = [(4, 10), (8, 11)]
# Idle loop: rest, sway left, blink, sway right. Every frame differs.
WISP_LOOP = [("base", WISP_EYES), ("left", WISP_EYES), ("base", WISP_BLINK), ("right", WISP_EYES)]


def wisp_frame(pose, eyes):
    rows = WISP_TIPS[pose] + WISP_BODY + WISP_TAILS[pose]
    inside = lambda x, y: 0 <= x < 16 and 0 <= y < 16 and rows[y][x] == "#"  # noqa: E731
    out = [["."] * 16 for _ in range(16)]
    for y in range(16):
        for x in range(16):
            if not inside(x, y):
                continue
            edge = not all(inside(x + dx, y + dy) for dx, dy in ((0, -1), (-1, 0), (0, 1), (1, 0)))
            lit = any(not inside(x + dx, y + dy) for dx, dy in ((-2, -1), (-1, -2), (0, -2), (-2, 0)))
            dark = any(not inside(x + dx, y + dy) for dx, dy in ((2, 1), (1, 2), (0, 2), (2, 0)))
            out[y][x] = "o" if edge else "h" if lit and not dark else "s" if dark and not lit else "b"
    # Flame tip glows: its leading pixels are light, not outline.
    for y in range(3):
        xs = [x for x in range(16) if rows[y][x] == "#"]
        out[y][xs[0]] = "l" if y == 0 else "h"
    out[6][5] = "l"  # glint on the dome
    for x, y in eyes:
        out[y][x] = "w"
    return ["".join(r) for r in out]


WISP_FRAMES = [wisp_frame(pose, eyes) for pose, eyes in WISP_LOOP]


def grid(rows, palette):
    return [[palette[c] for c in row] for row in rows]


class Canvas:
    def __init__(self, width, height, fill=CLEAR):
        self.width, self.height = width, height
        self.data = [[fill] * width for _ in range(height)]
        self.regions = {}

    def put(self, x, y, colour):
        if not (0 <= x < self.width and 0 <= y < self.height):
            raise ValueError(f"pixel {x},{y} outside {self.width}x{self.height}")
        self.data[y][x] = colour

    def blit(self, x0, y0, cells):
        for j, row in enumerate(cells):
            for i, colour in enumerate(row):
                self.put(x0 + i, y0 + j, colour)

    def fill(self, x0, y0, w, h, colour):
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w):
                self.put(x, y, colour)

    def region(self, name, x, y, w, h):
        for other, (ox, oy, ow, oh) in self.regions.items():
            if x < ox + ow and ox < x + w and y < oy + oh and oy < y + h:
                raise ValueError(f"region {name} overlaps {other}")
        if x < 0 or y < 0 or x + w > self.width or y + h > self.height:
            raise ValueError(f"region {name} outside the sheet")
        self.regions[name] = [x, y, w, h]

    def pixels(self):
        return [p for row in self.data for p in row]


# ---- nine-slice dialog panel: 24 x 24, borders left 3, right 6, top 4, bottom 8 --
PANEL_BORDER = [3, 6, 4, 8]
PANEL_COLOURS = {
    ".": CLEAR,
    "o": (24, 18, 44, 255),     # outline
    "l": (190, 178, 222, 255),  # bevel light
    "r": (132, 122, 176, 255),  # rim
    "d": (86, 78, 128, 255),    # rim dark / groove
    "k": (52, 46, 86, 255),     # inner shadow
    "c": (34, 36, 66, 255),     # centre fill
    "R": (214, 64, 72, 255),    # top-left gem (red)
    "G": (72, 186, 112, 255),   # top-right gem (green)
    "B": (76, 120, 222, 255),   # bottom-left gem (blue)
    "Y": (226, 182, 72, 255),   # bottom-right tab (gold)
    "y": (164, 120, 40, 255),   # gold shade
}
# Each edge is uniform along its stretch axis; each corner is unique.
# Columns: left 0-2 | centre 3-17 | right 18-23. Rows: top 0-3 | centre 4-15 | bottom 16-23.
PANEL = (
    ["o" * 24,
     "oRl" + "l" * 15 + "lGGllo",
     "olr" + "r" * 15 + "rGGrro",
     "olr" + "k" * 15 + "kdrdro"]
    + ["olr" + "c" * 15 + "kdrdro"] * 12
    + ["olr" + "k" * 15 + "kdrdro",
       "olr" + "r" * 15 + "rdrdro",
       "oBd" + "d" * 15 + "ddddYo",
       "oBy" + "y" * 15 + "yyyYYo",
       "old" + "d" * 15 + "ddYYyo",
       "old" + "d" * 15 + "dYYyyo",
       "orr" + "r" * 15 + "rrrrro",
       "o" * 24]
)


def panel_cells(palette=PANEL_COLOURS):
    rows = PANEL
    assert len(rows) == 24 and all(len(r) == 24 for r in rows), [len(r) for r in rows]
    return grid(rows, palette)


# ---- orientation key: 8 x 8 quadrants with a white notch in the top-left -----
KEY = {
    "r": (200, 62, 62, 255), "g": (66, 178, 92, 255), "b": (68, 104, 212, 255), "y": (220, 178, 60, 255),
    "w": (226, 226, 226, 255),
}
KEY_ROWS = ["wrrrgggg", "rrrrgggg", "rrrrgggg", "rrrrgggg", "bbbbyyyy", "bbbbyyyy", "bbbbyyyy", "bbbbyyyy"]

# ---- odd-size ruler: 7 x 7, every texel unique --------------------------------
# Texel (i, j) = (60 + 25 i, 60 + 25 j, 150): red encodes the column, green the row,
# all inside the tone curve's identity range. A rendered pixel names its texel.
def odd_ruler():
    return [[(60 + 25 * i, 60 + 25 * j, 150, 255) for i in range(7)] for j in range(7)]


def orb():
    """16 x 16 warm glow: straight alpha, hidden magenta RGB where alpha is zero."""
    cells = []
    for y in range(16):
        row = []
        for x in range(16):
            r = math.hypot(x + 0.5 - 8, y + 0.5 - 8)
            a = max(0.0, min(1.0, (7.0 - r) / 4.0))
            a = round(255 * a * a * (3 - 2 * a))
            row.append(HIDDEN if a == 0 else (255, 214, 128, a))
        cells.append(row)
    return cells


def glow():
    """16 x 16 white aura for tinting: soft quadratic falloff, hidden RGB at zero alpha."""
    cells = []
    for y in range(16):
        row = []
        for x in range(16):
            r = math.hypot(x + 0.5 - 8, y + 0.5 - 8) / 8.0
            a = round(255 * max(0.0, 1.0 - r) ** 2)
            row.append(HIDDEN if a == 0 else (255, 255, 255, a))
        cells.append(row)
    return cells


MOON = {"m": (238, 230, 198, 255), "n": (206, 196, 168, 255), "k": (150, 142, 132, 255)}
MOON_ROWS = [
    "....mmmm....",
    "..mmmmn.....",
    ".mmmmn......",
    ".mmmn.......",
    "mmmmn.......",
    "mmmkn.......",
    "mmmmn.......",
    "mmmmn.......",
    ".mmmmn......",
    ".mmmmmn.....",
    "..mmmmmnn...",
    "....mmmm....",
]


CRATE = {
    "o": (40, 26, 20, 255), "p": (150, 96, 54, 255), "q": (118, 72, 40, 255), "n": (196, 142, 84, 255),
}
CRATE_ROWS = [
    "oooooooooooo",
    "onnnnnnnnnno",
    "onqpppppppqo",
    "onpqpppppqpo",
    "onppqpppqppo",
    "onpppqpqpppo",
    "onppppqppppo",
    "onpppqpqpppo",
    "onppqpppqppo",
    "onpqpppppqpo",
    "onqpppppppqo",
    "oooooooooooo",
]

# ---- ground tiles: tile horizontally (left/right columns continue) ------------
TILE = {
    "g": (62, 150, 84, 255), "G": (118, 196, 104, 255), "k": (34, 94, 64, 255),
    "d": (92, 62, 54, 255), "D": (66, 44, 42, 255), "p": (132, 104, 88, 255), "f": (226, 120, 160, 255),
    "F": (230, 206, 96, 255),
}
TILE_ROWS = [
    [
        "GGgGGGgGGGGgGGGg",
        "gggggggggggggggg",
        "gkggggkgggggkggg",
        "kkkgkkkkkgkkkkgk",
        "dddkdddddkddddkd",
        "dddddddddddddddd",
        "ddpdddddddddDddd",
        "dddddDddddpddddd",
        "dddddddddddddddd",
        "dDddddddpdddddDd",
        "dddddddddddddddd",
        "ddddpdddDddddddd",
        "dddddddddddddpdd",
        "DdddddDddddddddd",
        "dddddddddddddddd",
        "ddpdddddddddDddd",
    ],
    [
        "GGGgGGGGfGGgGGGg",
        "ggggggggFfgggggg",
        "gkggggkggfggkggg",
        "kkkgkkkkkgkkkkgk",
        "dddkdddddkddddkd",
        "dddddddddddddddd",
        "dddddDdddddddddd",
        "ddddddddpddddDdd",
        "dpdddddddddddddd",
        "ddddddDddddddddd",
        "dddddddddddpdddd",
        "dddDdddddddddddd",
        "ddddddddddDddddd",
        "dddddpdddddddddd",
        "dDdddddddddddddd",
        "dddddddddddpdddd",
    ],
]

# ---- tone and alpha swatches -------------------------------------------------
TONE_SWATCHES = [
    (0, 0, 0), (64, 64, 64), (128, 128, 128), (200, 200, 200), (231, 231, 231), (232, 232, 232),
    (240, 240, 240), (255, 255, 255), (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 214, 128),
    (92, 60, 212), (200, 80, 40), (231, 120, 20), (240, 200, 40),
]
ALPHAS = [0, 32, 64, 96, 128, 160, 192, 224, 255]

# ---- 3 x 5 label font ---------------------------------------------------------
FONT = {
    "A": ["###", "#.#", "###", "#.#", "#.#"], "B": ["##.", "#.#", "##.", "#.#", "##."],
    "C": ["###", "#..", "#..", "#..", "###"], "D": ["##.", "#.#", "#.#", "#.#", "##."],
    "E": ["###", "#..", "##.", "#..", "###"], "F": ["###", "#..", "##.", "#..", "#.."],
    "G": ["###", "#..", "#.#", "#.#", "###"], "H": ["#.#", "#.#", "###", "#.#", "#.#"],
    "I": ["###", ".#.", ".#.", ".#.", "###"], "J": ["..#", "..#", "..#", "#.#", "###"],
    "K": ["#.#", "#.#", "##.", "#.#", "#.#"], "L": ["#..", "#..", "#..", "#..", "###"],
    "M": ["#.#", "###", "###", "#.#", "#.#"], "N": ["##.", "#.#", "#.#", "#.#", "#.#"],
    "O": ["###", "#.#", "#.#", "#.#", "###"], "P": ["###", "#.#", "###", "#..", "#.."],
    "Q": ["###", "#.#", "#.#", "###", "..#"], "R": ["##.", "#.#", "##.", "#.#", "#.#"],
    "S": ["###", "#..", "###", "..#", "###"], "T": ["###", ".#.", ".#.", ".#.", ".#."],
    "U": ["#.#", "#.#", "#.#", "#.#", "###"], "V": ["#.#", "#.#", "#.#", "#.#", ".#."],
    "W": ["#.#", "#.#", "###", "###", "#.#"], "X": ["#.#", "#.#", ".#.", "#.#", "#.#"],
    "Y": ["#.#", "#.#", ".#.", ".#.", ".#."], "Z": ["###", "..#", ".#.", "#..", "###"],
    "0": ["###", "#.#", "#.#", "#.#", "###"], "1": [".#.", "##.", ".#.", ".#.", "###"],
    "2": ["###", "..#", "###", "#..", "###"], "3": ["###", "..#", "###", "..#", "###"],
    "4": ["#.#", "#.#", "###", "..#", "..#"], "5": ["###", "#..", "###", "..#", "###"],
    "6": ["###", "#..", "###", "#.#", "###"], "7": ["###", "..#", "..#", "..#", "..#"],
    "8": ["###", "#.#", "###", "#.#", "###"], "9": ["###", "#.#", "###", "..#", "###"],
    "-": ["...", "...", "###", "...", "..."], ".": ["...", "...", "...", "...", ".#."],
    ",": ["...", "...", "...", ".#.", "#.."], "/": ["..#", "..#", ".#.", "#..", "#.."],
    " ": ["...", "...", "...", "...", "..."],
    "!": [".#.", ".#.", ".#.", "...", ".#."], "'": [".#.", ".#.", "...", "...", "..."],
}
LABEL_INK = (206, 200, 226, 255)
LABEL_MUTED = (140, 134, 170, 255)
LABELS = [
    "REGION", "SOURCE STRIP", "FRAMES", "GUARDED", "PIVOT", "ODD 7 PX", "FLIP", "FLIP X", "FLIP Y",
    "BOTH", "SCALE -1", "ROT 90", "ROT 180", "TINT", "SIMPLE", "9-SLICE", "STRETCHED", "SMALL",
    "DENSITY 8", "DENSITY 32", "LINEAR", "NEAREST", "ROTATED", "LAYER", "ORDER", "ID TIE", "Z IGNORED",
    "BATCH", "ALPHA", "DEPTH", "SEAM", "TONE", "16-BIT", "SKY", "A WISP STIRS.", "FOLLOW THE LIGHT",
    "GRAY 0-255",
    "A", "B", "C", "D", "E",
    # Follow-up tie boards (appended, so earlier label regions keep their places).
    "TIES 1X", "ALIGNED 1X", "TIES 3X", "ALIGNED 3X", "OFFSET IN 1/256 PX",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ".125", ".25", ".5", "-1",
    "1X", "2X", "3X", "4X", "5X", "6X", "8X", "1X FLIP", "2X FLIP", "3X FLIP", "4X FLIP", "5X FLIP",
    "6X FLIP", "8X FLIP", "F", "G", ".625",
]


def text_cells(text, ink):
    cells = [[] for _ in range(5)]
    for i, ch in enumerate(text):
        glyph = FONT[ch]
        for r in range(5):
            if i:
                cells[r].append(CLEAR)
            cells[r].extend(ink if c == "#" else CLEAR for c in glyph[r])
    return cells


def build_sheet(wisp_palette=WISP):
    c = Canvas(128, 64)
    for k, frame in enumerate(WISP_FRAMES):
        assert len(frame) == 16 and all(len(r) == 16 for r in frame), (k, [len(r) for r in frame])
        c.blit(16 * k, 0, grid(frame, wisp_palette))
        c.region(f"wisp{k}", 16 * k, 0, 16, 16)
    c.blit(64, 0, orb())
    c.region("orb", 64, 0, 16, 16)
    c.blit(80, 0, grid(KEY_ROWS, KEY))
    c.region("key", 80, 0, 8, 8)
    c.blit(88, 0, odd_ruler())
    c.region("odd", 88, 0, 7, 7)
    c.fill(80, 8, 2, 2, (236, 236, 236, 255))
    c.region("marker", 80, 8, 2, 2)
    c.put(82, 8, (255, 255, 255, 255))
    c.region("solid", 82, 8, 1, 1)
    star = [[CLEAR, (170, 180, 236, 255), CLEAR],
            [(170, 180, 236, 255), (255, 255, 255, 255), (170, 180, 236, 255)],
            [CLEAR, (170, 180, 236, 255), CLEAR]]
    c.blit(83, 8, star)
    c.region("star", 83, 8, 3, 3)
    # Guarded crate: the opaque magenta ring lies outside the region on all sides.
    c.fill(96, 0, 14, 14, MAGENTA)
    c.blit(97, 1, grid(CRATE_ROWS, CRATE))
    c.region("crate", 97, 1, 12, 12)
    # Sky gradient column with an opaque magenta guard column to its left.
    top, bottom = (14, 12, 36), (86, 54, 120)
    for y in range(32):
        t = y / 31
        c.put(127, y, tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,))
        c.put(126, y, MAGENTA)
    c.region("sky", 127, 0, 1, 32)
    for k, rows in enumerate(TILE_ROWS):
        assert len(rows) == 16 and all(len(r) == 16 for r in rows)
        c.blit(16 * k, 16, grid(rows, TILE))
        c.region(f"tile{k}", 16 * k, 16, 16, 16)
    # Soil continues below the grass tiles: the dirt rows of both tiles, interleaved.
    dirt = [TILE_ROWS[k % 2][5 + (k * 5) % 11] for k in range(16)]
    c.blit(32, 32, grid(dirt, TILE))
    c.region("dirt", 32, 32, 16, 16)
    c.blit(48, 32, grid(MOON_ROWS, MOON | {".": CLEAR}))
    c.region("moon", 48, 32, 12, 12)
    # Every 8-bit gray once: texel (i, j) = 16 j + i.
    c.blit(64, 32, [[(16 * j + i,) * 3 + (255,) for i in range(16)] for j in range(16)])
    c.region("ramp", 64, 32, 16, 16)
    # White aura between two opaque magenta guards (crate ring left, sky guard right).
    c.blit(110, 0, glow())
    c.region("glow", 110, 0, 16, 16)
    # Fog band: soft vertical alpha profile, uniform along x.
    for y, a in enumerate((40, 96, 96, 40)):
        for x in range(16):
            c.put(32 + x, 16 + y, (196, 186, 232, a))
    c.region("fog", 32, 16, 16, 4)
    for k, rgb in enumerate(TONE_SWATCHES):
        c.fill(32 + 4 * k, 20, 4, 4, rgb + (255,))
    c.region("tone", 32, 20, 64, 4)
    for k, a in enumerate(ALPHAS):
        c.fill(32 + 4 * k, 24, 4, 4, (255, 255, 255, a) if a else (255, 255, 255, 0))
        c.fill(68 + 4 * k, 24, 4, 4, (0, 0, 0, a))
    c.region("alpha_white", 32, 24, 36, 4)
    c.region("alpha_black", 68, 24, 36, 4)
    c.blit(0, 32, panel_cells())
    c.region("panel", 0, 32, 24, 24)
    # Opaque guards right of and below the panel; tiles already sit above it.
    c.fill(24, 32, 1, 25, MAGENTA)
    c.fill(0, 56, 24, 1, MAGENTA)
    return c


def build_labels():
    c = Canvas(256, 64)
    x, y = 0, 0
    for text in LABELS:
        cells = text_cells(text, LABEL_MUTED if len(text) == 1 else LABEL_INK)
        w = len(cells[0])
        if x + w > c.width:
            x, y = 0, y + 6
        c.blit(x, y, cells)
        c.region(f"label:{text}", x, y, w, 5)
        x += w + 2
    return c


def build_badge(colours, size=16):
    """Gem badge. `colours` is (light, mid, dark)."""
    c = Canvas(size, size)
    light, mid, dark = colours
    outline = (28, 22, 44, 255)
    centre = (size - 1) / 2
    for y in range(size):
        for x in range(size):
            d = abs(x - centre) + abs(y - centre)
            if d <= size / 2 - 0.5:
                if d > size / 2 - 1.6:
                    col = outline
                elif x + y < size - 1:
                    col = light if d < size / 4 else mid
                else:
                    col = dark
                c.put(x, y, col)
    c.region("badge", 0, 0, size, size)
    return c


def write_png16(path, width, height, pixels16):
    """16-bit RGBA PNG (big-endian samples)."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for p in pixels16[y * width:(y + 1) * width]:
            raw.extend(struct.pack(">4H", *p))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_bytes(pngio.SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 16, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b""))


def preview(canvas, path, scale=8, back=(28, 26, 40)):
    """Upscaled contact preview with a checker behind transparency."""
    w, h = canvas.width * scale, canvas.height * scale
    out = []
    for y in range(h):
        for x in range(w):
            r, g, b, a = canvas.data[y // scale][x // scale]
            chk = 46 if ((x // 4 + y // 4) % 2) else 36
            bg = (back[0] + chk - 40, back[1] + chk - 40, back[2] + chk - 40)
            al = a / 255
            out.append(tuple(round(c * al + k * (1 - al)) for c, k in zip((r, g, b), bg)) + (255,))
    pngio.write_rgba(path, w, h, out)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--preview", action="store_true")
    args = parser.parse_args()
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    out.mkdir(parents=True)
    sheet, teal, labels = build_sheet(), build_sheet(WISP_TEAL), build_labels()
    assert sheet.regions == teal.regions
    # Same texels and regions in a taller image: normalized UVs must be recomputed.
    tall = Canvas(sheet.width, sheet.height + 16)
    tall.data[:sheet.height] = [list(r) for r in sheet.data]
    tall.regions = dict(sheet.regions)
    badge = build_badge(((120, 226, 214, 255), (40, 170, 176, 255), (20, 98, 120, 255)))
    badge_orange = build_badge(((250, 206, 120, 255), (232, 128, 48, 255), (150, 64, 30, 255)))
    badge_small = build_badge(((120, 226, 214, 255), (40, 170, 176, 255), (20, 98, 120, 255)), size=8)
    files = {
        "sheet.png": sheet, "sheet-teal.png": teal, "sheet-tall.png": tall, "labels.png": labels,
        "badge.png": badge, "badge-orange.png": badge_orange, "badge-small.png": badge_small,
    }
    for name, canvas in files.items():
        pngio.write_rgba(out / name, canvas.width, canvas.height, canvas.pixels())
    # High-precision twin of the tone swatches: 16-bit samples = 8-bit value x 257.
    tone16 = []
    for y in range(4):
        for rgb in TONE_SWATCHES:
            tone16 += [tuple(v * 257 for v in rgb) + (65535,)] * 4
    write_png16(out / "tone16.png", 64, 4, tone16)
    regions = {
        "sheet": sheet.regions, "labels": labels.regions, "badge": badge.regions,
        "badge_small": badge_small.regions, "tone16": {"tone16": [0, 0, 64, 4]},
        "panel_border": PANEL_BORDER, "tone_swatches": TONE_SWATCHES, "alphas": ALPHAS,
    }
    (out / "regions.json").write_text(json.dumps(regions, indent=1), encoding="utf-8")
    if args.preview:
        for name, canvas in files.items():
            preview(canvas, out / f"preview-{name}", scale=8 if canvas.width <= 128 else 4)
    print(json.dumps({"written": sorted(files) + ["tone16.png", "regions.json"]}))


if __name__ == "__main__":
    main()
