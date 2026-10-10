#!/usr/bin/env python3
"""Original atlas look-dev art and descriptors for handoff 0034. Standard library only.

New art is drawn from code here so atlas failures are easy to see:
  * twelve ID cards (digits 0-9, A, B), each a unique colour inside an opaque
    magenta guard, so a wrong or shifted frame shows a wrong digit or magenta;
  * an 11 x 9 edge probe with a different colour on every side and corner and an
    F glyph, so flips, rotations and one-texel offsets are unambiguous;
  * 1100-texel horizontal and vertical strips whose every texel names itself by
    colour, so rendered pixels identify texels across the internal 512- and
    1024-texel span boundaries;
  * a soft halo with garbage green RGB hidden under zero alpha (transparent edges);
  * a night-panorama "ballast" in two sizes whose dimension change relocates
    every other frame;
  * label words in the 3 x 5 font of handoff 0032.

Reused with provenance: handoff 0032's committed sheet.png (Wisp frames, dialog
panel, key, 7 x 7 ruler, orb, glow, crate, tiles, solid, marker, star) and badge.png
are copied byte for byte from handoffs/0032-sprite-rendering/evidence/art, and its
font and PNG writer are imported from handoffs/0032-sprite-rendering/tools. The
SHA-256 of every reused file is recorded in manifest.json.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_art.py OUT_DIR
OUT_DIR must not exist.
"""

import argparse
import hashlib
import json
import math
import shutil
import struct
import sys
import zlib
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
ART_0032 = ROOT / "handoffs/0032-sprite-rendering/evidence/art"
TOOLS_0032 = ROOT / "handoffs/0032-sprite-rendering/tools"
sys.path.insert(0, str(TOOLS_0032))
import pngio  # noqa: E402
import sprite_art  # noqa: E402

CLEAR = (0, 0, 0, 0)
MAGENTA = (255, 0, 255, 255)        # opaque guard just outside a source rectangle
HIDDEN_GREEN = (0, 255, 0, 0)       # garbage RGB under zero alpha
INK = (28, 22, 44, 255)
LIGHT = (222, 220, 230, 255)

# Every authored colour stays at or below 226 per channel, inside the display
# transform's measured identity range (0032: sRGB 0-234 round-trip exactly).
CARD_COLOURS = [
    (196, 64, 64), (204, 120, 48), (196, 172, 52), (120, 172, 56), (56, 160, 88), (48, 156, 152),
    (52, 128, 196), (72, 88, 200), (128, 72, 196), (180, 64, 168), (150, 110, 80), (110, 120, 130),
]
CARD_GLYPHS = "0123456789AB"
CARD_W, CARD_H = 12, 14

PROBE_SIDES = {"top": (214, 64, 72), "right": (72, 186, 112), "bottom": (76, 120, 222), "left": (226, 182, 72)}
PROBE_CORNERS = {"tl": (224, 224, 224), "tr": (64, 200, 210), "bl": (226, 140, 180), "br": (20, 20, 24)}
PROBE_CHECK = ((44, 40, 70), (58, 54, 88))
PROBE_GLYPH = (206, 202, 222)
PROBE_W, PROBE_H = 11, 9

STRIP_LEN, STRIP_THICK = 1100, 4
HALO = (226, 180, 110)

ATLAS_ID = "01JA34AA000000000000000001"
SHEET_FRAMES = ["wisp0", "wisp1", "wisp2", "wisp3", "orb", "key", "odd", "marker", "solid", "star", "crate",
                "glow", "tile0", "tile1", "panel"]
LABELS = [
    "S", "T", "N", "O", "L", "B", "W", "V", "X", "Y",
    "STABLE FRAMES", "CARDS 1X", "CARDS 2X", "WISP", "KEY", "RULER", "CRATE", "PROBE", "TILES", "STAR",
    "TRANSFORMS", "PROBE 1X", "PROBE 2X", "RULER TIES 1X", "RULER TIES 3X", "RULER ALIGNED", "PIVOTS",
    "NONE", "FX", "FY", "FXY", "MIR", "R90", "R180", "R270", "R90FX",
    "9-SLICE", "NATIVE", "SIMPLE", "STRETCH 72X44", "SIMPLE 72X44", "SMALL", "FLIPS", "ROT 180", "MIRROR",
    "DENSITY 8", "DENSITY 32", "ROT 12", "PROBE 1-1-1-1",
    "ORDER", "LAYER", "ID TIE", "Z IGNORED", "BATCH", "ALPHA 0.5", "LIGHT",
    "LINEAR", "ORB 4X", "ORB ON LIGHT", "HALO 3X", "HALO 2.5X", "GLOW", "WISP 2X", "CRATE 30", "PANEL", "PROBE 3X",
    "BALLAST", "PIXEL RECT", "STABLE ID",
    "WIDE 1100 1X", "ALIGNED", "TIE", "FLIP", "FLIP TIE", "TIE +1/256", "FLIP +1/256", "TIE -1/256",
    "FLIP -1/256", "SPAN 512", "SPAN 1024", "2X", "4X", "8X", "16X",
    "32X", "MID", "MID TIE", "MID FLIP", "MID FLIP TIE", "ORB 2X", "GLOW 2X", "CARD 7", "TILES TIE",
    "SCOPED ID",
]
# Glyphs the 0032 font lacks; added here, not in 0032's tools.
EXTRA_GLYPHS = {"+": ["...", ".#.", "###", ".#.", "..."]}


def ulid(prefix, n):
    """Canonical ULID text: 10-character prefix + 16 Crockford digits of n."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    digits = ""
    for _ in range(16):
        digits = alphabet[n % 32] + digits
        n //= 32
    value = prefix + digits
    assert len(value) == 26 and value[0] in "01234567" and all(c in alphabet for c in value)
    return value


def frame_id(n):
    return ulid("01JA34FR00", n)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Canvas(sprite_art.Canvas):
    def ring(self, x, y, w, h, colour=MAGENTA):
        """One-texel guard ring directly around the rectangle (x, y, w, h)."""
        for i in range(x - 1, x + w + 1):
            self.put(i, y - 1, colour)
            self.put(i, y + h, colour)
        for j in range(y - 1, y + h + 1):
            self.put(x - 1, j, colour)
            self.put(x + w, j, colour)


def glyph_rows(ch):
    return EXTRA_GLYPHS.get(ch) or sprite_art.FONT[ch]


def card_cells(k):
    colour = CARD_COLOURS[k] + (255,)
    notch = tuple(min(226, c + 40) for c in CARD_COLOURS[k]) + (255,)
    cells = [[colour] * CARD_W for _ in range(CARD_H)]
    for x in range(CARD_W):
        cells[0][x] = cells[CARD_H - 1][x] = INK
    for y in range(CARD_H):
        cells[y][0] = cells[y][CARD_W - 1] = INK
    cells[1][1] = notch  # top-left orientation notch
    for r, row in enumerate(glyph_rows(CARD_GLYPHS[k])):
        for c, bit in enumerate(row):
            if bit == "#":
                for dy in range(2):
                    for dx in range(2):
                        cells[2 + 2 * r + dy][3 + 2 * c + dx] = LIGHT
    return cells


def build_cards():
    c = Canvas(4 * (CARD_W + 2) + 2, 3 * (CARD_H + 2) + 2, MAGENTA)
    for k in range(12):
        x, y = 2 + (CARD_W + 2) * (k % 4), 2 + (CARD_H + 2) * (k // 4)
        c.blit(x, y, card_cells(k))
        c.region(f"card{CARD_GLYPHS[k]}", x, y, CARD_W, CARD_H)
    return c


def probe_cells():
    w, h = PROBE_W, PROBE_H
    cells = [[PROBE_CHECK[(x + y) % 2] + (255,) for x in range(w)] for y in range(h)]
    for x in range(1, w - 1):
        cells[0][x] = PROBE_SIDES["top"] + (255,)
        cells[h - 1][x] = PROBE_SIDES["bottom"] + (255,)
    for y in range(1, h - 1):
        cells[y][0] = PROBE_SIDES["left"] + (255,)
        cells[y][w - 1] = PROBE_SIDES["right"] + (255,)
    for name, (x, y) in {"tl": (0, 0), "tr": (w - 1, 0), "bl": (0, h - 1), "br": (w - 1, h - 1)}.items():
        cells[y][x] = PROBE_CORNERS[name] + (255,)
    for r, row in enumerate(sprite_art.FONT["F"]):
        for col, bit in enumerate(row):
            if bit == "#":
                cells[2 + r][3 + col] = PROBE_GLYPH + (255,)
    return cells


def build_probe():
    c = Canvas(PROBE_W + 2, PROBE_H + 2, CLEAR)
    c.blit(1, 1, probe_cells())
    c.ring(1, 1, PROBE_W, PROBE_H)
    c.region("probe", 1, 1, PROBE_W, PROBE_H)
    return c


STRIP_PHASE = 32  # ramp wraps at texel 32 mod 64, so span cuts at multiples of 512 lie mid-ramp


def strip_colour(i, j, base):
    """Texel i along the strip, j across it: R, G name i; B names j."""
    k = i + STRIP_PHASE
    return (60 + 2 * (k % 64), 60 + 2 * (k // 64), base + j, 255)


H_BASE = (100, 120, 140, 160)
V_BASE = (180, 190, 200, 210)


def build_hstrip():
    c = Canvas(STRIP_LEN + 6, STRIP_THICK + 4, CLEAR)
    for j in range(STRIP_THICK):
        for i in range(STRIP_LEN):
            c.put(3 + i, 2 + j, strip_colour(i, 0, H_BASE[j]))
    c.ring(3, 2, STRIP_LEN, STRIP_THICK)
    c.region("hstrip", 3, 2, STRIP_LEN, STRIP_THICK)
    return c


def build_vstrip():
    c = Canvas(STRIP_THICK + 4, STRIP_LEN + 6, CLEAR)
    for j in range(STRIP_LEN):
        for i in range(STRIP_THICK):
            c.put(2 + i, 3 + j, strip_colour(j, 0, V_BASE[i]))
    c.ring(2, 3, STRIP_THICK, STRIP_LEN)
    c.region("vstrip", 2, 3, STRIP_THICK, STRIP_LEN)
    return c


def build_halo():
    c = Canvas(26, 26, CLEAR)
    for y in range(24):
        for x in range(24):
            r = math.hypot(x + 0.5 - 12, y + 0.5 - 12)
            a = max(0.0, min(1.0, (10.5 - r) / 7.0))
            a = round(255 * a * a * (3 - 2 * a))
            c.put(1 + x, 1 + y, HIDDEN_GREEN if a == 0 else HALO + (a,))
    c.ring(1, 1, 24, 24)
    c.region("halo", 1, 1, 24, 24)
    return c


def build_ballast(w, h):
    """Night panorama: sky gradient, deterministic stars, two hill ridges, a moon."""
    c = Canvas(w, h, CLEAR)
    top, bottom = (18, 22, 58), (74, 52, 112)
    for y in range(h):
        t = y / max(1, h - 1)
        sky = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom))
        for x in range(w):
            c.put(x, y, sky + (255,))
    seed = 0x34A7
    for _ in range(w * h // 90):
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF
        x, y = seed % w, (seed >> 11) % max(1, int(h * 0.7))
        c.put(x, y, (196, 200, 222, 255))
    for x in range(w):
        far = h - round(h * (0.28 + 0.10 * math.sin(x / 37.0) + 0.05 * math.sin(x / 11.0)))
        near = h - round(h * (0.16 + 0.07 * math.sin(x / 23.0 + 1.3)))
        for y in range(max(0, far), h):
            c.put(x, y, (38, 34, 72, 255))
        for y in range(max(0, near), h):
            c.put(x, y, (24, 30, 40, 255))
    cx, cy, rad = round(w * 0.82), round(h * 0.3), max(3, h // 6)
    for y in range(max(0, cy - rad), min(h, cy + rad + 1)):
        for x in range(cx - rad, cx + rad + 1):
            if math.hypot(x - cx, y - cy) <= rad:
                c.put(x, y, (222, 214, 184, 255))
    c.region("ballast", 0, 0, w, h)
    return c


def build_labels():
    c = Canvas(320, 96, CLEAR)
    x, y = 0, 0
    for text in LABELS:
        ink = sprite_art.LABEL_MUTED if len(text) == 1 else sprite_art.LABEL_INK
        cells = [[] for _ in range(5)]
        for i, ch in enumerate(text):
            for r in range(5):
                if i:
                    cells[r].append(CLEAR)
                cells[r].extend(ink if bit == "#" else CLEAR for bit in glyph_rows(ch)[r])
        w = len(cells[0])
        if x + w > c.width:
            x, y = 0, y + 6
        c.blit(x, y, cells)
        c.region(f"label:{text}", x, y, w, 5)
        x += w + 2
    return c


def descriptor(frames, maximum, padding, atlas_id=ATLAS_ID):
    return {"version": 1, "id": atlas_id, "maximum_size": list(maximum), "padding": padding,
            "frames": {fid: {"name": f["name"], "path": f["path"], "region": f["region"]}
                       for fid, f in frames.items()}}


def write_rows(path, width, height, row):
    """8-bit RGBA PNG from a row callback, for images too large for tuple lists."""
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    raw = b"".join(b"\x00" + row(y) for y in range(height))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    path.write_bytes(pngio.SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def write_json(path, value):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    out.mkdir(parents=True)
    reused = {}
    for name in ("sheet.png", "badge.png"):
        shutil.copyfile(ART_0032 / name, out / name)
        reused[name] = {"from": str((ART_0032 / name).relative_to(ROOT)), "sha256": sha(out / name)}
    regions_0032 = json.loads((ART_0032 / "regions.json").read_text(encoding="utf-8"))
    canvases = {"cards.png": build_cards(), "probe.png": build_probe(), "hstrip.png": build_hstrip(),
                "vstrip.png": build_vstrip(), "halo.png": build_halo(), "labels34.png": build_labels(),
                "ballast-v1.png": build_ballast(1200, 40), "ballast-v2.png": build_ballast(1240, 150)}
    for name, canvas in canvases.items():
        pngio.write_rgba(out / name, canvas.width, canvas.height, canvas.pixels())

    # Frame table: stable ID -> name, source path (relative to the descriptor) and rectangle.
    frames, n = {}, 0

    def add(name, path, region):
        nonlocal n
        n += 1
        frames[frame_id(n)] = {"name": name, "path": path, "region": list(region)}

    for key in SHEET_FRAMES:
        add(key, "sheet.png", regions_0032["sheet"][key])
    for key, rect in canvases["cards.png"].regions.items():
        add(key, "cards.png", rect)
    add("probe", "probe.png", canvases["probe.png"].regions["probe"])
    add("hstrip", "hstrip.png", canvases["hstrip.png"].regions["hstrip"])
    add("vstrip", "vstrip.png", canvases["vstrip.png"].regions["vstrip"])
    # One-texel rows/columns for magnified boards, and 700-texel sub-rectangles that
    # start at strip texel 100 (nonzero source origin, packed away from the corner).
    add("hrow", "hstrip.png", [3, 3, STRIP_LEN, 1])
    add("hmid", "hstrip.png", [103, 4, 700, 1])
    add("vcol", "vstrip.png", [3, 3, 1, STRIP_LEN])
    add("vmid", "vstrip.png", [4, 103, 1, 700])
    add("halo", "halo.png", canvases["halo.png"].regions["halo"])
    ballast_id = frame_id(n + 1)
    add("ballast", "ballast.png", [0, 0, 1200, 40])
    for key, rect in canvases["labels34.png"].regions.items():
        add(key, "labels34.png", rect)
    # The ballast frame selects the whole image: its region is omitted on purpose so
    # the v2 image's new dimensions are taken from the file, exercising "absent selects
    # the whole image". Every other frame uses an optional source rectangle.
    whole = dict(frames)
    whole[ballast_id] = {"name": "ballast", "path": "ballast.png", "region": None}

    def desc(maximum, padding, table=whole):
        d = descriptor(table, maximum, padding)
        for fid, f in d["frames"].items():
            if f["region"] is None:
                del f["region"]
        return d

    descs = {
        "atlas-v1": desc((2048, 2048), 2),
        "atlas-v3": desc((1600, 2600), 5),
        # --- rejected replacements (import must fail and change nothing) ---
        "fail-nofit": desc((1300, 1000), 2),
        "fail-bounds-area": desc((4096, 2048), 2),
        "fail-bounds-axis": desc((8193, 256), 2),
        "fail-padding0": desc((2048, 2048), 0),
        "fail-padding17": desc((2048, 2048), 17),
    }
    bad_id = dict(whole)
    lower = frame_id(1).lower()
    bad_id[lower] = bad_id.pop(frame_id(1))
    descs["fail-identity"] = desc((2048, 2048), 2, bad_id)
    bad_rect = dict(whole)
    bad_rect[frame_id(1)] = dict(bad_rect[frame_id(1)], region=[120, 0, 16, 16])  # 136 > 128 px sheet
    descs["fail-rect"] = desc((2048, 2048), 2, bad_rect)
    many = {frame_id(1000 + i): {"name": f"dot{i}", "path": "sheet.png", "region": [i % 128, i // 128, 1, 1]}
            for i in range(257)}
    descs["fail-frames257"] = desc((2048, 2048), 2, many)
    exactly = {frame_id(2000 + i): {"name": f"big{i}", "path": "big2048.png", "region": [16 * i, 0, 16, 16]}
               for i in range(4)}
    descs["limit-texels-exact"] = desc((2048, 2048), 2, exactly)
    over = dict(exactly)
    over[frame_id(2100)] = {"name": "one", "path": "one.png", "region": [0, 0, 1, 1]}
    descs["fail-texels"] = desc((2048, 2048), 2, over)
    # --- accepted imports that make the scene fail to prepare ---
    shrink = dict(whole)
    panel = next(k for k, f in whole.items() if f["name"] == "panel")
    shrink[panel] = dict(whole[panel], region=[0, 32, 8, 8])  # borders 3+6 > 8
    descs["render-fail-shrink-panel"] = desc((2048, 2048), 2, shrink)
    drop = dict(whole)
    dropped = next(k for k, f in whole.items() if f["name"] == "cardB")
    del drop[dropped]
    descs["render-fail-drop-cardB"] = desc((2048, 2048), 2, drop)
    # A second atlas that reuses two stable frame IDs with swapped content: lookup must be
    # scoped to the sprite's own texture, so these IDs show different cards per atlas.
    card = {f["name"]: (fid, f) for fid, f in whole.items()}
    descs["alt"] = descriptor({card["card0"][0]: dict(card["card1"][1], name="alt-card0"),
                               card["card1"][0]: dict(card["card0"][1], name="alt-card1")},
                              (256, 256), 1, atlas_id="01JA34AA000000000000000002")
    for name, d in descs.items():
        write_json(out / f"{name}.atlas.json", d)
    # Limit probes: one 2048 x 2048 source (exactly 4194304 texels) and a 1 x 1 extra.
    write_rows(out / "big2048.png", 2048, 2048,
               lambda y: b"".join(bytes((40 + (x // 128) * 8, 40 + (y // 128) * 8, 120, 255)) for x in range(2048)))
    pngio.write_rgba(out / "one.png", 1, 1, [(200, 200, 200, 255)])

    manifest = {
        "atlas_id": ATLAS_ID,
        "frames": {fid: dict(f) for fid, f in whole.items()},
        "ballast_frame": ballast_id,
        "panel_frame": panel,
        "dropped_frame": dropped,
        "regions": {name: c.regions for name, c in canvases.items()},
        "regions_0032_sheet": regions_0032["sheet"],
        "panel_border": regions_0032["panel_border"],
        "card_colours": CARD_COLOURS, "card_glyphs": CARD_GLYPHS,
        "probe": {"sides": PROBE_SIDES, "corners": PROBE_CORNERS, "check": PROBE_CHECK, "glyph": PROBE_GLYPH},
        "strip": {"length": STRIP_LEN, "thickness": STRIP_THICK, "h_base": H_BASE, "v_base": V_BASE,
                  "phase": STRIP_PHASE},
        "halo": HALO,
        "reused": reused,
        "reused_code": {p.name: {"from": str(p.relative_to(ROOT)), "sha256": sha(p)}
                        for p in (TOOLS_0032 / "pngio.py", TOOLS_0032 / "sprite_art.py")},
        "files": {p.name: sha(p) for p in sorted(out.iterdir()) if p.is_file()},
    }
    write_json(out / "manifest.json", manifest)
    print(json.dumps({"frames": len(whole), "files": len(manifest["files"])}))


if __name__ == "__main__":
    main()
