#!/usr/bin/env python3
"""Original diagnostic tileset art for handoff 0039. Standard library only.

Every opaque tile frame carries the same asymmetric 1-texel ring (red top, blue
bottom, amber left, green right, four distinct corners), a frame-unique interior
and a 3x5 glyph naming the frame, so a wrong frame, flip, turn, row order or
one-texel offset is visible. Frame sizes differ on purpose; a tilemap stretches
each whole frame across its cell. Two frames have transparent texels whose hidden
RGB is magenta, and every source region in tiles.png is surrounded by an opaque
magenta guard, so sampling outside a frame shows magenta.

Atlas descriptors:
  v1            padding 1, small ballast: every frame packs at a nonzero origin;
  v2            padding 2 and a larger ballast image: frames relocate, IDs stay;
  fail-missing  v2 without frame C, which the scene references: must be rejected.

The PNG writer and the 3x5 glyph shapes are reused read-only from handoff 0032
(pngio.py, sprite_art.FONT); their hashes are recorded in the manifest.

Usage (repo root):
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_art.py OUT_DIR
OUT_DIR must not exist. The committed copy lives in handoffs/0039-native-tilemaps/art.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
ROOT = HELPERS.parents[2]
REUSED = ROOT / "handoffs/0032-sprite-rendering/tools"
sys.path.insert(0, str(REUSED))
import pngio  # noqa: E402
from sprite_art import FONT  # noqa: E402

CLEAR_HIDDEN = (230, 0, 230, 0)  # zero alpha, garbage magenta RGB
GUARD = (230, 0, 230, 255)  # opaque magenta outside every source region
SIDES = {"top": (214, 64, 72), "bottom": (76, 120, 222), "left": (226, 182, 72), "right": (72, 186, 112)}
CORNERS = {"tl": (224, 224, 224), "tr": (64, 200, 210), "bl": (226, 140, 180), "br": (20, 20, 24)}
INK = (200, 196, 220)
# name: (width, height, interior fill). All channels stay within the display's
# exact sRGB range (0..234, measured in handoff 0032).
OPAQUE = {
    "A": (8, 8, (60, 44, 110)), "B": (12, 10, (40, 90, 80)), "C": (10, 14, (110, 60, 40)),
    "D": (16, 16, (50, 70, 130)), "E": (6, 9, (100, 100, 40)), "F": (20, 12, (120, 40, 90)),
    "G": (7, 11, (40, 100, 140)), "H": (13, 7, (90, 110, 60)),
}
RING_T = (12, 12, (130, 80, 40))  # transparent outer ring, coloured inner ring, glyph T
CROSS = (16, 16, (230, 210, 60), (64, 200, 210))  # mostly transparent X with a top-left block
CHECK = (4, 4, (52, 52, 64), (84, 84, 100), (170, 70, 70))  # under-layer checker, marked top-left
RULER_W = (24, 8)  # every column a distinct colour; red top row, blue bottom row
ORDER = ["A", "B", "C", "D", "E", "F", "G", "H", "T", "X", "K", "W"]
ATLAS_ID = "01JA39AT000000000000000001"
BALLAST = {"v1": (30, 6), "v2": (70, 22)}
PLATES = {"board": (24, 22, 40), "board_b": (30, 28, 50), "board_c": (22, 30, 34), "under": (90, 60, 30)}
BADGE = (10, 10)


def ulid(prefix, n):
    """Canonical ULID text: 10-character prefix + 16 Crockford digits of n."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    digits = ""
    for _ in range(16):
        digits = alphabet[n % 32] + digits
        n //= 32
    value = prefix + digits
    assert len(value) == 26 and value[0] in "01234567" and all(c in alphabet for c in value), value
    return value


def frame_id(name):
    return ulid("01JA39FR00", ORDER.index(name) + 1) if name != "ballast" else ulid("01JA39FR00", 99)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Canvas:
    def __init__(self, width, height, fill):
        self.width, self.height = width, height
        self.data = [fill] * (width * height)

    def put(self, x, y, rgba):
        self.data[y * self.width + x] = rgba

    def fill(self, x0, y0, w, h, rgba):
        for y in range(y0, y0 + h):
            for x in range(x0, x0 + w):
                self.put(x, y, rgba)

    def write(self, path):
        pngio.write_rgba(path, self.width, self.height, self.data)


def opaque(rgb):
    return (*rgb, 255)


def ringed(w, h, fill, glyph, inset=0):
    """Rows of RGBA texels: asymmetric ring at `inset`, interior fill, centred glyph.
    Texels outside the ring (inset > 0) are transparent with hidden magenta RGB."""
    rows = [[CLEAR_HIDDEN] * w for _ in range(h)]
    x0, y0, x1, y1 = inset, inset, w - 1 - inset, h - 1 - inset
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            rows[y][x] = opaque(fill)
    for x in range(x0, x1 + 1):
        rows[y0][x], rows[y1][x] = opaque(SIDES["top"]), opaque(SIDES["bottom"])
    for y in range(y0, y1 + 1):
        rows[y][x0], rows[y][x1] = opaque(SIDES["left"]), opaque(SIDES["right"])
    rows[y0][x0], rows[y0][x1] = opaque(CORNERS["tl"]), opaque(CORNERS["tr"])
    rows[y1][x0], rows[y1][x1] = opaque(CORNERS["bl"]), opaque(CORNERS["br"])
    shape = FONT[glyph]
    gx = x0 + 1 + (x1 - x0 - 1 - 3) // 2
    gy = y0 + 1 + (y1 - y0 - 1 - 5) // 2
    for r, line in enumerate(shape):
        for c, ch in enumerate(line):
            if ch == "#":
                rows[gy + r][gx + c] = opaque(INK)
    return rows


def cross_rows():
    w, h, ink, block = CROSS
    rows = [[CLEAR_HIDDEN] * w for _ in range(h)]
    for i in range(w):
        for d in (0, 1):
            j = min(i + d, w - 1)
            rows[i][j] = opaque(ink)
            rows[i][w - 1 - j] = opaque(ink)
    for y in range(3):
        for x in range(3):
            rows[y][x] = opaque(block)
    return rows


def check_rows():
    w, h, a, b, mark = CHECK
    rows = [[opaque(a if (x + y) % 2 == 0 else b) for x in range(w)] for y in range(h)]
    rows[0][0] = opaque(mark)
    return rows


def ruler_rows():
    w, h = RULER_W
    rows = []
    for y in range(h):
        row = []
        for x in range(w):
            if y == 0:
                row.append(opaque(SIDES["top"]))
            elif y == h - 1:
                row.append(opaque(SIDES["bottom"]))
            else:
                row.append(opaque((30 + 8 * x, 200 - 7 * x, 60 + 100 * (x % 2))))
        rows.append(row)
    return rows


def frame_rows(name):
    if name in OPAQUE:
        w, h, fill = OPAQUE[name]
        return ringed(w, h, fill, name)
    if name == "T":
        w, h, fill = RING_T
        return ringed(w, h, fill, "T", inset=1)
    if name == "X":
        return cross_rows()
    if name == "K":
        return check_rows()
    if name == "W":
        return ruler_rows()
    raise KeyError(name)


def build_tiles():
    """Shelf layout with a 2-texel opaque guard around every region."""
    frames = {name: frame_rows(name) for name in ORDER}
    width, gap = 96, 2
    x, y, shelf = gap, gap, 0
    regions = {}
    for name in ORDER:
        rows = frames[name]
        w, h = len(rows[0]), len(rows)
        if x + w + gap > width:
            x, y, shelf = gap, y + shelf + gap, 0
        regions[name] = [x, y, w, h]
        x += w + gap
        shelf = max(shelf, h)
    height = y + shelf + gap
    canvas = Canvas(width, height, GUARD)
    for name, (rx, ry, w, h) in regions.items():
        for j, row in enumerate(frames[name]):
            for i, texel in enumerate(row):
                canvas.put(rx + i, ry + j, texel)
    return canvas, regions, frames


def build_ballast(w, h):
    canvas = Canvas(w, h, opaque((20, 24, 40)))
    for y in range(h):
        for x in range(w):
            canvas.put(x, y, opaque((20 + (x * 5) % 60, 24 + (y * 9) % 80, 40 + ((x + y) * 3) % 90)))
    return canvas


def build_plates():
    canvas = Canvas(4 * len(PLATES), 4, opaque((0, 0, 0)))
    regions = {}
    for i, (name, rgb) in enumerate(PLATES.items()):
        canvas.fill(4 * i, 0, 4, 4, opaque(rgb))
        regions[name] = [4 * i, 0, 4, 4]
    return canvas, regions


def build_badge():
    """Ordinary (non-atlas) texture for interleaved sprites: asymmetric, opaque."""
    w, h = BADGE
    canvas = Canvas(w, h, opaque((40, 40, 44)))
    for x in range(w):
        canvas.put(x, 0, opaque((230, 140, 40)))
        canvas.put(x, h - 1, opaque((150, 90, 30)))
    for y in range(h):
        canvas.put(0, y, opaque((230, 140, 40)))
        canvas.put(w - 1, y, opaque((150, 90, 30)))
    canvas.fill(2, 2, 2, 2, opaque((224, 224, 224)))
    canvas.fill(5, 5, 3, 3, opaque((230, 140, 40)))
    return canvas


def descriptor(regions, padding, maximum=(256, 256), drop=()):
    frames = {frame_id(n): {"name": n, "path": "tiles.png", "region": regions[n]} for n in ORDER if n not in drop}
    frames[frame_id("ballast")] = {"name": "ballast", "path": "ballast.png", "region": None}
    return {"version": 1, "id": ATLAS_ID, "maximum_size": list(maximum), "padding": padding, "frames": frames}


def write_json(path, value):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    out = args.out
    if out.exists():
        raise SystemExit("output directory already exists")
    out.mkdir(parents=True)
    tiles, regions, frames = build_tiles()
    tiles.write(out / "tiles.png")
    for version, (w, h) in BALLAST.items():
        build_ballast(w, h).write(out / f"ballast-{version}.png")
    plates, plate_regions = build_plates()
    plates.write(out / "plates.png")
    build_badge().write(out / "badge.png")
    descs = {"tiles-v1": descriptor(regions, 1), "tiles-v2": descriptor(regions, 2),
             "tiles-fail-missing": descriptor(regions, 2, drop=("C",))}
    for name, value in descs.items():
        write_json(out / f"{name}.atlas.json", value)
    manifest = {
        "atlas_id": ATLAS_ID,
        "frames": {frame_id(n): {"name": n, "path": "tiles.png", "region": regions[n],
                                 "size": [regions[n][2], regions[n][3]]} for n in ORDER},
        "ballast": {"id": frame_id("ballast"), "v1": list(BALLAST["v1"]), "v2": list(BALLAST["v2"])},
        "plates": plate_regions,
        "badge": list(BADGE),
        "colours": {"sides": SIDES, "corners": CORNERS, "ink": INK, "guard": GUARD, "hidden": CLEAR_HIDDEN},
        "reused": {"pngio.py": sha(REUSED / "pngio.py"), "sprite_art.py (FONT only)": sha(REUSED / "sprite_art.py")},
        "files": {},
    }
    for path in sorted(out.iterdir()):
        manifest["files"][path.name] = sha(path)
    write_json(out / "manifest.json", manifest)
    print(json.dumps({"files": len(manifest["files"]), "tiles": [tiles.width, tiles.height], "regions": regions}))


if __name__ == "__main__":
    main()
