#!/usr/bin/env python3
"""Deterministic fixture art for handoff 0062 (drawn from code; standard library only).

  python3 -I -B art.py <new-output-directory>

Every transparent texel stores hidden magenta RGB (255, 0, 255) under alpha 0,
so any missing premultiplication or region/padding bleed shows as magenta.
Writes PNG sources, one sprite atlas descriptor, an emissive glTF occluder and
manifest.json (file hashes, frame IDs, sheet regions and palette rules).
"""

import hashlib
import json
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

HIDDEN = (255, 0, 255, 0)
CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"


def ulid(prefix, n):
    digits = ""
    for _ in range(26 - len(prefix)):
        digits = CROCKFORD[n % 32] + digits
        n //= 32
    value = prefix + digits
    assert len(value) == 26 and value[0] in "01234567" and all(c in CROCKFORD for c in value), value
    return value


def ulid_hex(text):
    value = 0
    for ch in text:
        value = value * 32 + CROCKFORD.index(ch)
    return f"{value:032x}"


class Image:
    def __init__(self, width, height, fill=HIDDEN):
        self.width, self.height = width, height
        self.pixels = [[tuple(fill)] * width for _ in range(height)]

    def set(self, x, y, color):
        color = tuple(color)
        self.pixels[y][x] = color if len(color) == 4 else color + (255,)

    def rect(self, x0, y0, x1, y1, color):
        for y in range(y0, y1):
            for x in range(x0, x1):
                self.set(x, y, color)

    def save(self, path):
        rows = [bytes(c for p in row for c in p) for row in self.pixels]
        pngio.write(path, self.width, self.height, rows)


def checker():
    """8x8, every texel unique: r encodes x, g encodes y, b alternates."""
    image = Image(8, 8)
    for y in range(8):
        for x in range(8):
            image.set(x, y, (40 + 28 * x, 40 + 28 * y, 220 if (x + y) % 2 == 0 else 90))
    return image


def ring():
    """10x10: hidden transparent edge, distinct ring sides/corners, grey interior, dark diagonal."""
    image = Image(10, 10)
    image.rect(1, 1, 9, 2, (230, 40, 40))
    image.rect(1, 8, 9, 9, (40, 80, 230))
    image.rect(1, 2, 2, 8, (240, 170, 30))
    image.rect(8, 2, 9, 8, (40, 200, 80))
    for (x, y), color in {(1, 1): (250, 250, 250), (8, 1): (60, 60, 60), (1, 8): (40, 220, 220),
                          (8, 8): (240, 240, 60)}.items():
        image.set(x, y, color)
    image.rect(2, 2, 8, 8, (200, 200, 200))
    for i in range(2, 8):
        image.set(i, i, (70, 70, 90))
    return image


def alpha():
    """16x8: rows 0-3 alpha ramp 0..255 (orange); rows 4-7 alternate alpha 128/255 (cyan)."""
    image = Image(16, 8)
    for x in range(16):
        a = 17 * x
        for y in range(4):
            image.set(x, y, (255, 150, 40, a) if a else HIDDEN)
        for y in range(4, 8):
            image.set(x, y, HIDDEN if x == 0 else (40, 200, 255, 128 if x % 2 == 0 else 255))
    return image


def arrow():
    """12x12 asymmetric 'F' glyph on hidden transparency; green texel marks bottom-right."""
    image = Image(12, 12)
    image.rect(2, 1, 4, 11, (245, 245, 245))
    image.rect(2, 1, 10, 3, (230, 50, 50))
    image.rect(4, 5, 8, 7, (240, 210, 40))
    image.set(10, 10, (40, 200, 80))
    image.set(2, 10, (60, 120, 230))
    return image


def nine():
    """9x9 nine-slice source with 3-texel borders and a distinct centre."""
    image = Image(9, 9)
    for y in range(9):
        for x in range(9):
            border_x, border_y = x < 3 or x > 5, y < 3 or y > 5
            if border_x and border_y:
                color = (150, 30, 30) if (x + y) % 2 == 0 else (110, 20, 20)
            elif border_y:
                color = (230, 120, 30) if x % 2 == 0 else (200, 90, 20)
            elif border_x:
                color = (230, 200, 60) if y % 2 == 0 else (190, 160, 40)
            else:
                color = (60, 190, 90)
            image.set(x, y, color)
    image.set(4, 4, (250, 250, 250))
    image.set(0, 0, (250, 250, 250))
    return image


FRAME_SIZES = [(12, 12), (14, 11), (10, 13), (15, 10), (11, 14)]
FRAME_COLORS = [(220, 60, 60), (240, 160, 40), (230, 220, 60), (60, 190, 90), (60, 120, 230)]


def frame(index):
    """Flipbook frame: hidden edge, darker ring, unique fill, index+1 white pips, black origin marker."""
    w, h = FRAME_SIZES[index]
    fill = FRAME_COLORS[index]
    image = Image(w, h)
    dark = tuple(c * 3 // 5 for c in fill)
    image.rect(1, 1, w - 1, h - 1, dark)
    image.rect(2, 2, w - 2, h - 2, fill)
    image.set(2, 2, (20, 20, 20))
    for k in range(index + 1):
        image.set(4 + k, 3, (250, 250, 250))
    return image


TILE_RULES = {
    "t0": ((70, 160, 60), (120, 200, 90), (50, 120, 45)),
    "t1": ((130, 130, 140), (175, 175, 185), (90, 90, 100)),
    "t2": ((50, 110, 200), (120, 170, 240), (30, 70, 150)),
    "t3": ((170, 70, 50), (210, 190, 170), (120, 45, 35)),
}


def tile(name):
    """8x8 opaque tile: light top row, dark left column, unique corners, inner motif."""
    base, light, dark = TILE_RULES[name]
    image = Image(8, 8, base + (255,))
    image.rect(0, 0, 8, 1, light)
    image.rect(0, 1, 1, 8, dark)
    image.set(0, 0, (250, 250, 250))
    image.set(7, 7, (20, 20, 20))
    index = int(name[1])
    image.rect(3, 3, 5, 5, tuple(255 - c for c in base))
    image.set(2 + index, 6, (250, 250, 250))
    return image


def sheet():
    """48x16 plain sheet: solid red | cyan-bordered checker | solid blue, no padding."""
    image = Image(48, 16)
    image.rect(0, 0, 16, 16, (255, 0, 0))
    image.rect(32, 0, 48, 16, (0, 0, 255))
    image.rect(16, 0, 32, 16, (40, 220, 220))
    for y in range(1, 15):
        for x in range(17, 31):
            block = ((x - 17) // 4 + (y - 1) // 4) % 2
            image.set(x, y, (240, 240, 240) if block == 0 else (60, 60, 70))
    return image


def strip_color(i):
    return (24 + (i % 32) * 7, 24 + ((i // 32) % 16) * 14, 24 + (i // 512) * 30)


def strips():
    horizontal = Image(4096, 1)
    vertical = Image(1, 4096)
    for i in range(4096):
        horizontal.set(i, 0, strip_color(i))
        vertical.set(0, i, strip_color(i))
    return horizontal, vertical


def occluder(directory):
    """2 m x 1.5 m +Z quad, double-sided opaque emissive teal material."""
    positions = [-1., -.75, 0., 1., -.75, 0., 1., .75, 0., -1., -.75, 0., 1., .75, 0., -1., .75, 0.]
    uv = [0., 1., 1., 1., 1., 0., 0., 1., 1., 0., 0., 0.]
    buffer = struct.pack("<18f12f", *positions, *uv)
    (directory / "occluder.bin").write_bytes(buffer)
    document = {
        "asset": {"version": "2.0"}, "buffers": [{"uri": "occluder.bin", "byteLength": len(buffer)}],
        "bufferViews": [{"buffer": 0, "byteLength": 72}, {"buffer": 0, "byteOffset": 72, "byteLength": 48}],
        "accessors": [{"bufferView": 0, "componentType": 5126, "count": 6, "type": "VEC3",
                       "min": [-1, -0.75, 0], "max": [1, 0.75, 0]},
                      {"bufferView": 1, "componentType": 5126, "count": 6, "type": "VEC2"}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "TEXCOORD_0": 1}, "material": 0}]}],
        "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [0, 0, 0, 1], "metallicFactor": 1},
                       "emissiveFactor": [0.05, 0.25, 0.45], "alphaMode": "OPAQUE", "doubleSided": True}],
        "nodes": [{"mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0,
    }
    (directory / "occluder.gltf").write_text(json.dumps(document, indent=1) + "\n")


ATLAS_ID = ulid("01JA62AT00", 1)
FRAME_IDS = {name: ulid("01JA62FR00", i + 1) for i, name in enumerate(
    ["checker", "ring", "alpha", "arrow", "nine", "f0", "f1", "f2", "f3", "f4", "t0", "t1", "t2", "t3"])}
SHEET_REGIONS = {"red": [0, 0, 16, 16], "middle": [16, 0, 16, 16], "blue": [32, 0, 16, 16]}


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    out = Path(sys.argv[1])
    out.mkdir(parents=True)
    images = {"checker": checker(), "ring": ring(), "alpha": alpha(), "arrow": arrow(), "nine": nine()}
    images.update({f"f{i}": frame(i) for i in range(5)})
    images.update({name: tile(name) for name in TILE_RULES})
    for name, image in images.items():
        image.save(out / f"{name}.png")
    sheet().save(out / "sheet.png")
    horizontal, vertical = strips()
    horizontal.save(out / "strip-h.png")
    vertical.save(out / "strip-v.png")
    occluder(out)
    descriptor = {"version": 1, "id": ATLAS_ID, "maximum_size": [256, 256], "padding": 2,
                  "frames": {FRAME_IDS[name]: {"name": name, "path": f"{name}.png"} for name in images}}
    (out / "sprites.atlas.json").write_text(json.dumps(descriptor, indent=1, sort_keys=True) + "\n")
    files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(out.iterdir())}
    manifest = {"files": files, "atlas_id": ATLAS_ID, "frames": FRAME_IDS,
                "frame_sizes": {name: [images[name].width, images[name].height] for name in images},
                "sheet_regions": SHEET_REGIONS, "hidden_rgb": list(HIDDEN[:3]),
                "strip_rule": "texel i -> (24 + 7*(i%32), 24 + 14*((i//32)%16), 24 + 30*(i//512)); 4096 unique"}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print(json.dumps({"files": len(files), "atlas_frames": len(FRAME_IDS)}))


if __name__ == "__main__":
    main()
