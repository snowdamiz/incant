#!/usr/bin/env python3
"""Original diagnostic flipbook art for handoff 0042. Standard library only.

Five animation frames f0..f4 have unequal dimensions (12x12, 14x11, 10x13, 15x10,
11x14). Each has a fully transparent one-texel outer edge whose hidden RGB is
magenta, then the same asymmetric one-texel ring (red top, blue bottom, amber
left, green right; light-grey, cyan, pink and near-black corners), a frame-unique
interior colour, the frame's digit and white 2x2 pips that step clockwise (f0
top-left, f1 top-right, f2 bottom-right, f3 bottom-left, f4 all four). A wrong
frame, flip, row order, crop shift or one-texel offset is therefore visible, and
an interior colour identifies the frame from rendered pixels.

  f0..f4.png        the frames, also packed into the atlas
  t0..t2.png        8x8 opaque tiles for the tilemap, in the same atlas
  sheet.png         ordinary texture: the same five frames at irregular nonzero
                    origins inside an opaque magenta guard (pixel-region flipbook)
  badge.png         ordinary opaque static-sprite texture
  plates.png        board backgrounds
  flip.atlas.json   atlas descriptor: padding 1, so every packed origin is nonzero

All colours stay within sRGB 0..234, the display's measured identity range (0032).

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/fb_art.py OUT_DIR
OUT_DIR must not exist. The committed copy lives in handoffs/0042-native-flipbook/art.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import fb_png  # noqa: E402

HIDDEN = (230, 0, 230, 0)  # zero alpha, garbage magenta RGB
GUARD = (230, 0, 230, 255)  # opaque magenta outside every sheet region
SIDES = {"top": (214, 64, 72), "bottom": (76, 120, 222), "left": (226, 182, 72), "right": (72, 186, 112)}
CORNERS = {"tl": (224, 224, 224), "tr": (64, 200, 210), "bl": (226, 140, 180), "br": (20, 20, 24)}
INK = (200, 196, 220)
PIP = (234, 234, 234)
# name: (width, height, interior, digit, pip corners)
FRAMES = {
    "f0": (12, 12, (60, 44, 110), "0", ("tl",)),
    "f1": (14, 11, (40, 96, 80), "1", ("tr",)),
    "f2": (10, 13, (120, 64, 40), "2", ("br",)),
    "f3": (15, 10, (48, 72, 140), "3", ("bl",)),
    "f4": (11, 14, (104, 104, 40), "4", ("tl", "tr", "br", "bl")),
}
TILES = {
    "t0": ((70, 56, 40), (122, 100, 60), (178, 150, 96)),  # ground: base, top row, top-left mark
    "t1": ((110, 50, 46), (150, 140, 130), (190, 86, 70)),  # brick: base, mortar, top-left mark
    "t2": ((90, 96, 104), (150, 156, 166), (40, 44, 52)),  # stone: base, cross, top-left mark
}
ORDER = ["f0", "f1", "f2", "f3", "f4", "t0", "t1", "t2"]
FRAME_NUMBER = {"f0": 1, "f1": 2, "f2": 3, "f3": 4, "f4": 5, "t0": 11, "t1": 12, "t2": 13}
ATLAS_ID = "01JA42AT000000000000000001"
SHEET_SIZE = (48, 34)
SHEET_ORIGINS = {"f0": (2, 3), "f1": (17, 1), "f2": (34, 2), "f3": (3, 19), "f4": (22, 17)}
BADGE = (10, 10)
PLATES = {"A": (24, 22, 40), "B": (30, 28, 50), "C": (22, 30, 34), "D": (34, 26, 30), "E": (28, 34, 28)}


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
    return ulid("01JA42FR00", FRAME_NUMBER[name])


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def opaque(rgb):
    return (*rgb, 255)


def frame_rows(name):
    w, h, fill, digit, pips = FRAMES[name]
    rows = [[HIDDEN] * w for _ in range(h)]
    x0, y0, x1, y1 = 1, 1, w - 2, h - 2
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            rows[y][x] = opaque(fill)
    for x in range(x0, x1 + 1):
        rows[y0][x], rows[y1][x] = opaque(SIDES["top"]), opaque(SIDES["bottom"])
    for y in range(y0, y1 + 1):
        rows[y][x0], rows[y][x1] = opaque(SIDES["left"]), opaque(SIDES["right"])
    rows[y0][x0], rows[y0][x1] = opaque(CORNERS["tl"]), opaque(CORNERS["tr"])
    rows[y1][x0], rows[y1][x1] = opaque(CORNERS["bl"]), opaque(CORNERS["br"])
    ix0, iy0, iw, ih = 2, 2, w - 4, h - 4
    gx, gy = ix0 + (iw - 3) // 2, iy0 + (ih - 5) // 2
    for r, line in enumerate(fb_png.FONT[digit]):
        for c, ch in enumerate(line):
            if ch == "#":
                rows[gy + r][gx + c] = opaque(INK)
    corners = {"tl": (ix0, iy0), "tr": (ix0 + iw - 2, iy0), "br": (ix0 + iw - 2, iy0 + ih - 2),
               "bl": (ix0, iy0 + ih - 2)}
    for corner in pips:
        px, py = corners[corner]
        for dy in (0, 1):
            for dx in (0, 1):
                assert rows[py + dy][px + dx] == opaque(fill), (name, corner)
                rows[py + dy][px + dx] = opaque(PIP)
    return rows


def tile_rows(name):
    base, line, mark = TILES[name]
    rows = [[opaque(base)] * 8 for _ in range(8)]
    if name == "t0":
        rows[0] = [opaque(line)] * 8
        for i in (2, 5):
            rows[4][i] = opaque(line)
    elif name == "t1":
        for x in range(8):
            rows[3][x] = rows[7][x] = opaque(line)
        for y in range(3):
            rows[y][4] = opaque(line)
        for y in range(4, 7):
            rows[y][1] = opaque(line)
    else:
        for i in range(8):
            rows[i][i] = opaque(line)
            rows[i][7 - i] = opaque(line)
    rows[0][0] = opaque(mark)
    return rows


def write_rows(path, rows):
    fb_png.write_rgba(path, len(rows[0]), len(rows), rows)


def build_sheet(frames):
    w, h = SHEET_SIZE
    rows = [[GUARD] * w for _ in range(h)]
    regions = {}
    for name, (ox, oy) in SHEET_ORIGINS.items():
        source = frames[name]
        fw, fh = len(source[0]), len(source)
        assert ox > 0 and oy > 0 and ox + fw < w and oy + fh < h, name
        for y in range(fh):
            for x in range(fw):
                assert rows[oy + y][ox + x] == GUARD, (name, x, y)
                rows[oy + y][ox + x] = source[y][x]
        regions[name] = [ox, oy, fw, fh]
    return rows, regions


def build_badge():
    w, h = BADGE
    rows = [[opaque((40, 40, 44))] * w for _ in range(h)]
    for x in range(w):
        rows[0][x], rows[h - 1][x] = opaque((230, 140, 40)), opaque((150, 90, 30))
    for y in range(h):
        rows[y][0], rows[y][w - 1] = opaque((230, 140, 40)), opaque((150, 90, 30))
    for y in range(2, 4):
        for x in range(2, 4):
            rows[y][x] = opaque((210, 210, 200))
    for y in range(5, 8):
        for x in range(5, 8):
            rows[y][x] = opaque((230, 140, 40))
    return rows


def build_plates():
    rows = [[opaque((0, 0, 0))] * (4 * len(PLATES)) for _ in range(4)]
    regions = {}
    for i, (board, rgb) in enumerate(PLATES.items()):
        for y in range(4):
            for x in range(4):
                rows[y][4 * i + x] = opaque(rgb)
        regions[board] = [4 * i, 0, 4, 4]
    return rows, regions


def descriptor():
    return {"version": 1, "id": ATLAS_ID, "maximum_size": [256, 256], "padding": 1,
            "frames": {frame_id(n): {"name": n, "path": f"{n}.png"} for n in ORDER}}


def write_json(path, value):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    out = parser.parse_args().out
    if out.exists():
        raise SystemExit("output directory already exists")
    out.mkdir(parents=True)
    frames = {name: frame_rows(name) for name in FRAMES}
    for name, rows in frames.items():
        write_rows(out / f"{name}.png", rows)
    for name in TILES:
        write_rows(out / f"{name}.png", tile_rows(name))
    sheet, regions = build_sheet(frames)
    write_rows(out / "sheet.png", sheet)
    write_rows(out / "badge.png", build_badge())
    plates, plate_regions = build_plates()
    write_rows(out / "plates.png", plates)
    write_json(out / "flip.atlas.json", descriptor())
    manifest = {
        "atlas_id": ATLAS_ID,
        "frames": {n: {"id": frame_id(n), "size": list(FRAMES[n][:2]), "interior": list(FRAMES[n][2]),
                       "digit": FRAMES[n][3], "pips": list(FRAMES[n][4])} for n in FRAMES},
        "tiles": {n: {"id": frame_id(n), "size": [8, 8], "colours": [list(c) for c in TILES[n]]} for n in TILES},
        "sheet": {"size": list(SHEET_SIZE), "regions": regions},
        "badge": list(BADGE),
        "plates": plate_regions,
        "colours": {"sides": SIDES, "corners": CORNERS, "ink": INK, "pip": PIP, "guard": GUARD,
                    "hidden": HIDDEN},
        "files": {},
    }
    for path in sorted(out.iterdir()):
        manifest["files"][path.name] = sha(path)
    write_json(out / "manifest.json", manifest)
    print(json.dumps({"files": len(manifest["files"]), "sheet_regions": regions}))


if __name__ == "__main__":
    main()
