#!/usr/bin/env python3
"""Real-engine sprite look-dev for handoff 0032. Standard library only.

Builds a disposable project in a NEW output directory using only public engine
interfaces and the supplied frozen binary:

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json
     before and after every engine invocation.
  2. Generates the original art (sprite_art.py), copies the sources into the
     project and registers them with `import`. A mathematical emissive quad (glTF)
     supplies an opaque occluder for the depth checks.
  3. One `rpc command.execute` transaction with provenance creates every entity,
     then `project.save` and `validate`. No project or journal file is edited.
  4. Five 320 x 180 logical "boards" (16 px per metre) expose regions, pivots,
     flips, nine-slice borders, painter order, alpha, depth, sampling, seams, tone
     and a small showcase. Each board is captured by a pixel-perfect camera and by
     ordinary orthographic cameras (exact 3x, fractional 3.75x, rolled). Every
     screenshot is rendered twice into separate new files.
  5. Texture reimport through `import`: whole-sheet recolour, an undersized badge
     that must fail render preparation, restoration, and a linear-usage reimport
     that must be rejected. Sources are then deleted and frames re-rendered.
  6. Strict-TypeScript behaviours move a pixel-perfect camera and the Wisp together
     by 1/8 logical pixel per tick in eight directions while cycling its source
     region (a flipbook made of ordinary set_component commands). `play` captures
     every tick, twice per direction.

  7. Follow-up boards for nearest ties: F holds a 7 x 7 unique-texel ruler in every
     orientation (eight square symmetries, a 180-degree transform and a mirror
     parent) on tie and aligned placements at 1x and 3x; G sweeps sub-pixel offsets
     of 0 to 6/256 px at magnifications 1 to 8, unflipped and flipped.
  8. Round-three falsification boards: H holds 12x-32x strips, quarter turns,
     anisotropic scale and 2-4x minification on tie placements plus offsets; I holds
     a 4096-texel strip magnified 1x-16x so region-local texels sit near 4096; J maps
     exact ties across region widths 512-8192 at 1x-32x.
  9. Round-four span boards (1024 x 1024) for the 512-texel span correction: K and L
     sweep 1x-64x along screen x and y with windows on a span cut and inside a span,
     offsets 0 and +-1/256 px; M holds odd region origins and widths, mirrors, half
     turns, wide and compressed nine-slices, translucent and linear rows; N holds
     isotropic 32x-64x rows, also at 1920 x 1080 and through a rolled camera. The
     strip and column are reimported at another size and every capture is budgeted.
     With --probe, boards O and P (20x-128x, rows and columns) are added and only their
     1024 x 1024 and 1920 x 1080 views are captured.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py OUT_DIR [--binary NAME]
      [--repeats 1|2] [--live DIRS|none] [--static-only] [--probe]
OUT_DIR must not exist. NAME is active (default), pre-wide, pre-magnification or
pre-ties; the earlier builds are preserved in artifacts/tools for before/after
comparisons. Each renderer invocation is limited to 128 frames and 256 MiB raw.
"""

import argparse
import hashlib
import json
import math
import shutil
import struct
import subprocess
import sys
from fractions import Fraction
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import sprite_art  # noqa: E402

ROOT = TOOLS.parents[2]
# The active build is the newest tested one; earlier builds are kept for before/after:
# pre-ties (39e5c38), pre-magnification (c2c7758), pre-wide (f9aa1da) and active
# (0494b40 at round four).
BINARIES = {"active": ("incant_headless", "binary.json"),
            "pre-wide": ("pre-wide-incant_headless", "pre-wide-binary.json"),
            "pre-magnification": ("pre-magnification-incant_headless", "pre-magnification-binary.json"),
            "pre-ties": ("pre-ties-incant_headless", "pre-ties-binary.json")}
CLI = ROOT / "artifacts/tools/incant_headless"
BINARY_JSON = ROOT / "artifacts/tools/binary.json"
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

PPU = 16
LOGICAL = (320, 180)
HALF_W, HALF_H = LOGICAL[0] / PPU / 2, LOGICAL[1] / PPU / 2  # 10 m x 5.625 m
EYE_Z = 10.0
NEAR, FAR, FOV = 0.1, 50.0, 50.0
BOARD_PITCH = 24.0  # metres between board centres
BOARDS = ["A", "B", "C", "D", "E"]
BOARD_X = {b: BOARD_PITCH * i for i, b in enumerate(BOARDS)}
# Board centre (m) and logical size (px). F and G sit near x = 0 so sub-pixel offsets
# stay far above f32 position precision; neither is visible from boards A to E.
BOARD_SPEC = {b: (BOARD_X[b], 0.0, 320, 180) for b in BOARDS}
BOARD_SPEC["F"] = (0.0, 24.0, 320, 180)
BOARD_SPEC["G"] = (0.0, -40.0, 1024, 640)
BOARD_SPEC["H"] = (0.0, 60.0, 1280, 640)  # screenshots allow at most 1920 x 1080
BOARD_SPEC["I"] = (0.0, 110.0, 1024, 640)
BOARD_SPEC["J"] = (0.0, 160.0, 1024, 640)
# Round four: 1024 x 1024 boards (half-extents of 32 m keep projections dyadic). Their
# strips run tens of kilometres, so each board owns a corridor no other board crosses:
# K, M and N rows run along x at y = 400, -400 and -800; L columns along y at x = 400.
BOARD_SPEC["K"] = (0.0, 400.0, 1024, 1024)
BOARD_SPEC["L"] = (400.0, 0.0, 1024, 1024)
BOARD_SPEC["M"] = (0.0, -400.0, 1024, 1024)
BOARD_SPEC["N"] = (0.0, -800.0, 1024, 1024)
PIXEL = {"kind": "pixel_perfect", "pixels_per_unit": PPU, "reference_resolution": list(LOGICAL)}
BG = (24, 22, 40)  # board background, sRGB
STEP = 1.0 / (8 * PPU)  # 1/8 logical pixel per tick
LIVE_TICKS = 33
DIRECTIONS = {"e": (1, 0), "w": (-1, 0), "n": (0, 1), "s": (0, -1),
              "ne": (1, 1), "sw": (-1, -1), "se": (1, -1), "nw": (-1, 1)}


def srgb_to_linear(v):
    v /= 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def tint_of(rgb, alpha=1.0):
    return [round(srgb_to_linear(c), 9) for c in rgb] + [alpha]


# ---------------------------------------------------------------- engine calls --

def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(CLI.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


CALLS = {"count": 0, "unchanged": True}


def run(*args, check=True, stdin=None):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT, input=stdin)
    if check and result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:3]]} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}")
    return result


def engine(*args, check=True, stdin=None):
    before = verify_binary()
    result = run(CLI, *args, check=check, stdin=stdin)
    after = verify_binary()
    CALLS["count"] += 1
    CALLS["unchanged"] &= before == after
    return result


def rpc(project, journal, requests, expect_error=False):
    result = engine("rpc", project, "--journal", journal, stdin="".join(json.dumps(r) + "\n" for r in requests),
                    check=not expect_error)
    replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    if not expect_error:
        for reply in replies:
            if "error" in reply:
                raise SystemExit(f"RPC error: {reply['error']}")
    return replies


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


# ------------------------------------------------------------------- geometry --

def quat_z(degrees):
    h = math.radians(degrees) / 2
    return [0.0, 0.0, math.sin(h), math.cos(h)]


IDENT = [0.0, 0.0, 0.0, 1.0]


def transform(t, rotation=IDENT, scale=(1, 1, 1), exact=False):
    """Rounded to 12 decimals for readable JSON, unless `exact` keeps dyadic offsets
    (board G's 1/512 px steps) bit-for-bit."""
    return {"translation": [v if exact else round(v, 12) for v in t],
            "rotation": [round(v, 12) for v in rotation], "scale": list(scale)}


def wx(board, px):
    cx, _, lw, _ = BOARD_SPEC[board]
    return cx - lw / PPU / 2 + px / PPU


def wy(board, py):
    _, cy, _, lh = BOARD_SPEC[board]
    return cy + lh / PPU / 2 - py / PPU


def new_ulid(n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA32SP00" + suffix


class Scene:
    """Collects entities in creation order. IDs ascend with creation order."""

    def __init__(self, textures):
        self.textures = textures  # name -> asset id
        self.regions = {}  # (texture name, region name) -> [x, y, w, h]
        self.entities = []
        self.ids = {}
        self.meta = {}
        self.counter = 0

    def entity(self, name, components, parent=None, meta=None):
        self.counter += 1
        entity_id = new_ulid(self.counter)
        if name in self.ids:
            raise ValueError(f"duplicate entity name {name}")
        self.entities.append({"id": entity_id, "name": name, "parent": parent, "components": components})
        self.ids[name] = entity_id
        if meta is not None:
            self.meta[name] = meta
        return entity_id

    def sprite(self, name, board, tex, region, rect=None, *, pivot=(0.5, 0.5), center=None, size=None, angle=0.0,
               flip_x=False, flip_y=False, tint=(1, 1, 1, 1), layer=0, order=0, z=0.0, sampling="nearest",
               mode=None, parent=None, local=None, scale=(1, 1, 1), exact=False):
        """`rect` = (x, y, w, h) in board logical pixels (top-left origin), placed via pivot.
        `center` + `size` (logical px) place a rotated sprite about its centre (pivot 0.5)."""
        reg = self.regions[(tex, region)] if isinstance(region, str) else list(region)
        if rect is not None:
            x, y, w, h = rect
            translation = (wx(board, x) + pivot[0] * w / PPU, wy(board, y + h) + pivot[1] * h / PPU, z)
        elif center is not None:
            w, h = size
            translation = (wx(board, center[0]), wy(board, center[1]), z)
        else:
            w, h = size
            translation = local
        value = {"texture": self.textures[tex], "region": reg, "size": [w / PPU, h / PPU], "pivot": list(pivot),
                 "tint": list(tint), "flip_x": flip_x, "flip_y": flip_y, "layer": layer, "order": order,
                 "sampling": sampling, "mode": mode or {"kind": "simple"}}
        components = {"Transform": transform(translation, quat_z(angle), scale, exact), "SpriteRenderer": value}
        meta = {"board": board, "texture": tex, "region_name": region if isinstance(region, str) else None,
                "rect": list(rect) if rect is not None else None,
                "center": list(center) if center is not None else None}
        return self.entity(name, components, parent=self.ids[parent] if parent else None, meta=meta)

    def label(self, board, text, x, y, layer=50):
        reg = self.regions[("labels", f"label:{text}")]
        return self.sprite(f"{board}:label:{text}:{x},{y}", board, "labels", f"label:{text}",
                           (x, y, reg[2], reg[3]), pivot=(0, 1), layer=layer)

    def plate(self, name, board, rect, rgb, layer=-100, z=0.0, alpha=1.0):
        return self.sprite(name, board, "sheet", "solid", rect, pivot=(0.5, 0.5), tint=tint_of(rgb, alpha),
                           layer=layer, z=z)


# --------------------------------------------------------------- scene layout --

def nine(border, density):
    return {"kind": "nine_slice", "border": list(border), "pixels_per_unit": density}


def board_a(s):
    b = "A"
    s.plate("A:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "A", 312, 172)
    s.label(b, "SOURCE STRIP", 8, 6)
    s.sprite("A:strip", b, "sheet", (0, 0, 64, 16), (8, 14, 64, 16), pivot=(0, 1))
    s.label(b, "FRAMES", 84, 6)
    for k in range(4):
        s.sprite(f"A:frame{k}", b, "sheet", f"wisp{k}", (84 + 20 * k, 14, 16, 16), pivot=(0, 0))
    s.label(b, "GUARDED", 172, 6)
    s.sprite("A:crate1x", b, "sheet", "crate", (172, 16, 12, 12), pivot=(0, 1))
    s.sprite("A:crate2x", b, "sheet", "crate", (192, 12, 24, 24), pivot=(1, 1))
    s.sprite("A:crate2x-linear", b, "sheet", "crate", (222, 12, 24, 24), pivot=(0, 0), sampling="linear")
    s.sprite("A:crate-rot30-linear", b, "sheet", "crate", None, center=(276, 24), size=(24, 24), angle=30,
             sampling="linear")
    s.label(b, "PIVOT", 8, 44)
    pivots = [(0, 0), (0.5, 0), (0.5, 0.5), (1, 1), (0.25, 0.75)]
    for k, p in enumerate(pivots):
        ox, oy = 28 + 40 * k, 70
        # Place the sprite so its pivot lies on the origin (ox, oy).
        rect = (ox - p[0] * 16, oy - (1 - p[1]) * 16, 16, 16)
        s.sprite(f"A:pivot{k}", b, "sheet", "key", rect, pivot=p)
        s.sprite(f"A:pivot{k}:marker", b, "sheet", "marker", (ox - 1, oy - 1, 2, 2), layer=5)
    s.label(b, "ODD 7 PX", 220, 44)
    # 7 x 7 ruler on a centred pivot. Top row: pivot on a grid point, so every edge and
    # texel boundary lies on a pixel centre. Bottom row: pivot on a pixel centre (aligned).
    for k, (fx, fy) in enumerate(((False, False), (True, False), (False, True), (True, True))):
        cx = 228 + 12 * k
        s.sprite(f"A:odd-grid{k}", b, "sheet", "odd", (cx - 3.5, 56.5, 7, 7), flip_x=fx, flip_y=fy)
        s.sprite(f"A:odd-half{k}", b, "sheet", "odd", (cx - 3, 70, 7, 7), flip_x=fx, flip_y=fy)
    # Transform-based orientation on the same grid-point pivot: 180 degrees, mirror parent.
    s.sprite("A:odd-grid-rot180", b, "sheet", "odd", None, center=(276, 60), size=(7, 7), angle=180)
    s.entity("A:odd-mirror-parent", {"Transform": transform((wx(b, 288), wy(b, 60), 0), IDENT, (-1, 1, 1))})
    s.sprite("A:odd-grid-mirror", b, "sheet", "odd", None, size=(7, 7), local=(0, 0, 0), parent="A:odd-mirror-parent")
    s.label(b, "FLIP", 8, 92)
    names = ["normal", "flip-x", "flip-y", "both", "scale-1", "rot90", "rot180"]
    captions = [None, "FLIP X", "FLIP Y", "BOTH", "SCALE -1", "ROT 90", "ROT 180"]
    for k, (name, caption) in enumerate(zip(names, captions)):
        cx = 24 + 40 * k
        rect = (cx - 16, 100, 32, 32)
        if name == "scale-1":
            s.entity("A:mirror-parent", {"Transform": transform((wx(b, cx), wy(b, 116), 0), IDENT, (-1, 1, 1))})
            s.sprite(f"A:flip:{name}", b, "sheet", "wisp0", None, size=(32, 32), local=(0, 0, 0),
                     parent="A:mirror-parent")
            s.meta[f"A:flip:{name}"]["rect"] = list(rect)
        elif name.startswith("rot"):
            s.sprite(f"A:flip:{name}", b, "sheet", "wisp0", None, center=(cx, 116), size=(32, 32),
                     angle=90 if name == "rot90" else 180)
        else:
            s.sprite(f"A:flip:{name}", b, "sheet", "wisp0", rect, flip_x=name in ("flip-x", "both"),
                     flip_y=name in ("flip-y", "both"))
        if caption:
            w = s.regions[("labels", f"label:{caption}")][2]
            s.label(b, caption, cx - w // 2, 136)
    s.label(b, "TINT", 8, 148)
    tints = [(1, 1, 1, 1), (1, 0.45, 0.45, 1), (0.45, 1, 0.45, 1), (0.45, 0.45, 1, 1), (0.3, 0.3, 0.3, 1),
             (1, 1, 1, 0.5)]
    for k, t in enumerate(tints):
        s.sprite(f"A:tint{k}", b, "sheet", "wisp0", (8 + 20 * k, 158, 16, 16), tint=t)


def board_b(s):
    b = "B"
    border = sprite_art.PANEL_BORDER
    s.plate("B:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "B", 312, 172)
    s.label(b, "SIMPLE", 8, 6)
    s.sprite("B:simple24", b, "sheet", "panel", (8, 14, 24, 24))
    s.label(b, "9-SLICE", 44, 6)
    s.sprite("B:nine72x44", b, "sheet", "panel", (44, 14, 72, 44), mode=nine(border, 16))
    s.label(b, "STRETCHED", 128, 6)
    s.sprite("B:simple72x44", b, "sheet", "panel", (128, 14, 72, 44))
    s.label(b, "SMALL", 212, 6)
    s.sprite("B:nine8x10", b, "sheet", "panel", (212, 14, 8, 10), mode=nine(border, 16))
    s.sprite("B:nine6x30", b, "sheet", "panel", (226, 14, 6, 30), mode=nine(border, 16))
    s.sprite("B:nine44x9", b, "sheet", "panel", (238, 14, 44, 9), mode=nine(border, 16))
    s.sprite("B:nine24", b, "sheet", "panel", (290, 14, 24, 24), mode=nine(border, 16))
    for k, (caption, fx, fy) in enumerate([(None, False, False), ("FLIP X", True, False), ("FLIP Y", False, True),
                                           ("BOTH", True, True)]):
        x = 8 + 74 * k
        if caption:
            s.label(b, caption, x, 66)
        else:
            s.label(b, "9-SLICE", x, 66)
        s.sprite(f"B:flip{k}", b, "sheet", "panel", (x, 74, 64, 40), mode=nine(border, 16), flip_x=fx, flip_y=fy)
    s.label(b, "DENSITY 8", 8, 120)
    s.sprite("B:density8", b, "sheet", "panel", (8, 128, 80, 48), mode=nine(border, 8))
    s.label(b, "DENSITY 32", 96, 120)
    s.sprite("B:density32", b, "sheet", "panel", (96, 128, 64, 40), mode=nine(border, 32))
    s.label(b, "LINEAR", 168, 120)
    s.sprite("B:linear8", b, "sheet", "panel", (168, 128, 64, 44), mode=nine(border, 8), sampling="linear")
    s.label(b, "ROTATED", 240, 120)
    s.sprite("B:rot12", b, "sheet", "panel", None, center=(276, 154), size=(48, 32), angle=12,
             mode=nine(border, 16))


SQUARE = 24
RGB = {"red": (214, 64, 72), "green": (72, 186, 112), "blue": (76, 120, 222)}


def board_c(s):
    b = "C"
    s.plate("C:background", b, (-16, -16, 352, 156), BG)  # y >= 140 stays empty: display backdrop
    s.label(b, "C", 312, 130)
    clusters = [
        ("LAYER", 8, {"red": {"layer": 2}, "green": {"layer": 1}, "blue": {"layer": 0}}),
        ("ORDER", 64, {"red": {"order": 5}, "green": {"order": -3}, "blue": {"order": 0}}),
        ("ID TIE", 120, {"red": {}, "green": {}, "blue": {}}),
    ]
    for caption, x0, spec in clusters:
        s.label(b, caption, x0, 6)
        for k, colour in enumerate(("red", "green", "blue")):
            s.plate(f"C:{caption}:{colour}", b, (x0 + 8 * k, 14 + 8 * k, SQUARE, SQUARE), RGB[colour],
                    layer=spec[colour].get("layer", 0))
            if "order" in spec[colour]:
                s.entities[-1]["components"]["SpriteRenderer"]["order"] = spec[colour]["order"]
    s.label(b, "Z IGNORED", 176, 6)
    s.plate("C:Z:red-far", b, (176, 14, SQUARE, SQUARE), RGB["red"], layer=1, z=-3.0)
    s.plate("C:Z:green-near", b, (188, 26, SQUARE, SQUARE), RGB["green"], layer=0, z=3.0)
    s.label(b, "BATCH", 232, 6)
    for k in range(4):
        tex, region = ("sheet", "key") if k % 2 == 0 else ("badge", "badge")
        s.sprite(f"C:batch{k}", b, tex, region, (232 + 10 * k, 14 + 10 * k, 16, 16), order=k)
    s.label(b, "ALPHA", 8, 72)
    for origin, plate in (((8, 80), None), ((76, 80), (72, 76, 64, 60))):
        if plate:
            s.plate("C:alpha-plate", b, plate, (200, 200, 200), layer=-50)
        for k, (colour, dx, dy) in enumerate((("red", 0, 0), ("green", 16, 0), ("blue", 8, 14))):
            s.plate(f"C:venn{origin[0]}:{colour}", b, (origin[0] + dx, origin[1] + dy, 32, 32), RGB[colour],
                    layer=0, alpha=0.5)
            s.entities[-1]["components"]["SpriteRenderer"]["order"] = k
    s.plate("C:gray-plate", b, (148, 76, 80, 28), (128, 128, 128), layer=-50)
    s.sprite("C:alpha-white-gray", b, "sheet", "alpha_white", (152, 80, 72, 8))
    s.sprite("C:alpha-black-gray", b, "sheet", "alpha_black", (152, 92, 72, 8))
    s.label(b, "DEPTH", 236, 72)
    s.sprite("C:depth:behind", b, "sheet", "wisp0", (236, 84, 32, 32), z=0.0)
    s.sprite("C:depth:front", b, "sheet", "key", (282, 102, 16, 16), z=2.0)
    s.sprite("C:depth:layer100-behind", b, "sheet", "key", (290, 80, 16, 16), z=-1.0, layer=100)
    # Display-backdrop band: nothing opaque behind these sprites.
    s.sprite("C:void:alpha-white", b, "sheet", "alpha_white", (8, 148, 72, 8))
    s.sprite("C:void:alpha-black", b, "sheet", "alpha_black", (8, 162, 72, 8))
    s.sprite("C:void:orb", b, "sheet", "orb", (96, 144, 32, 32), sampling="linear")
    s.sprite("C:void:wisp-half", b, "sheet", "wisp0", (140, 152, 16, 16), tint=(1, 1, 1, 0.5))
    for k, (colour, dx, dy) in enumerate((("red", 0, 0), ("green", 12, 0), ("blue", 6, 10))):
        s.plate(f"C:void:venn:{colour}", b, (176 + dx, 144 + dy, 24, 24), RGB[colour], layer=0, alpha=0.5)
        s.entities[-1]["components"]["SpriteRenderer"]["order"] = k
    # All 256 grays at 2x: the display transform's full input/output table.
    s.label(b, "GRAY 0-255", 232, 140)
    s.sprite("C:gray-ramp", b, "sheet", "ramp", (232, 146, 32, 32))


def board_d(s):
    b = "D"
    s.plate("D:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "D", 312, 172)
    s.label(b, "NEAREST", 8, 6)
    s.sprite("D:orb-nearest", b, "sheet", "orb", (8, 14, 64, 64))
    s.label(b, "LINEAR", 80, 6)
    s.sprite("D:orb-linear", b, "sheet", "orb", (80, 14, 64, 64), sampling="linear")
    s.plate("D:light-plate", b, (152, 10, 72, 72), (200, 200, 200), layer=-50)
    s.sprite("D:orb-linear-plate", b, "sheet", "orb", (156, 14, 64, 64), sampling="linear")
    s.sprite("D:wisp2x-nearest", b, "sheet", "wisp0", (236, 14, 32, 32))
    s.sprite("D:wisp2x-linear", b, "sheet", "wisp0", (276, 14, 32, 32), sampling="linear")
    s.sprite("D:crate-rot-nearest", b, "sheet", "crate", None, center=(252, 66), size=(24, 24), angle=30)
    s.sprite("D:crate-rot-linear", b, "sheet", "crate", None, center=(292, 66), size=(24, 24), angle=30,
             sampling="linear")
    s.label(b, "SEAM", 8, 84)
    for row, (y, sampling) in enumerate(((92, "nearest"), (110, "linear"))):
        for k in range(18):
            s.sprite(f"D:tiles{row}:{k}", b, "sheet", f"tile{k % 2}", (8 + 16 * k, y, 16, 16), pivot=(0, 0),
                     sampling=sampling)
    s.label(b, "TONE", 8, 130)
    s.sprite("D:tone8", b, "sheet", "tone", (8, 138, 128, 8))
    s.label(b, "16-BIT", 140, 148)
    s.sprite("D:tone16", b, "tone16", "tone16", (8, 148, 128, 8))
    s.sprite("D:tone8-tint", b, "sheet", "tone", (8, 158, 128, 8), tint=(0.5, 0.5, 0.5, 1))
    s.label(b, "TINT", 140, 159)
    s.label(b, "SKY", 168, 130)
    s.sprite("D:sky-nearest", b, "sheet", "sky", (168, 138, 20, 38))
    s.sprite("D:sky-linear", b, "sheet", "sky", (192, 138, 20, 38), sampling="linear")
    s.entity("D:tile-row-parent", {"Transform": transform((wx(b, 264), wy(b, 156), 0), quat_z(8))})
    for k in range(5):
        s.sprite(f"D:tiles-rot:{k}", b, "sheet", f"tile{k % 2}", None, size=(16, 16),
                 local=((k - 2) * 1.0, 0, 0), parent="D:tile-row-parent")


def board_e(s):
    b = "E"
    s.sprite("E:sky", b, "sheet", "sky", (-16, -16, 352, 164), sampling="linear", layer=-100)
    stars = [(14, 12, 1.0), (40, 30, 0.6), (66, 8, 0.8), (92, 44, 0.5), (120, 18, 1.0), (150, 52, 0.6),
             (30, 64, 0.5), (300, 70, 0.7), (284, 92, 0.5), (238, 74, 0.8), (210, 98, 0.5), (136, 84, 0.6),
             (60, 96, 0.7), (312, 112, 0.5), (18, 108, 0.6)]
    for k, (x, y, t) in enumerate(stars):
        s.sprite(f"E:star{k}", b, "sheet", "star", (x, y, 3, 3), tint=(t, t, min(1.0, t * 1.1), 1), layer=-90)
    s.sprite("E:moon", b, "sheet", "moon", (36, 22, 12, 12), layer=-80)
    s.sprite("E:moon-halo", b, "sheet", "glow", (18, 4, 48, 48), sampling="linear", layer=-85,
             tint=(0.85, 0.80, 0.62, 0.22))
    s.sprite("E:fog", b, "sheet", "fog", (-16, 120, 352, 28), sampling="linear", layer=-60, tint=(1, 1, 1, 0.55))
    for k in range(-1, 21):
        s.sprite(f"E:ground{k}", b, "sheet", f"tile{(k * 7) % 3 % 2}", (16 * k, 148, 16, 16), pivot=(0, 0),
                 layer=-40)
        for row in (1, 2):
            s.sprite(f"E:soil{k}:{row}", b, "sheet", "dirt", (16 * k, 148 + 16 * row, 16, 16), pivot=(0, 0),
                     layer=-40, tint=(0.8, 0.8, 0.8, 1) if row == 2 else (1, 1, 1, 1))
    s.sprite("E:glow", b, "sheet", "glow", (80, 96, 48, 48), sampling="linear", layer=-20,
             tint=(0.50, 0.36, 1.0, 0.50))
    s.sprite("E:wisp", b, "sheet", "wisp0", (96, 112, 16, 16), layer=0)
    s.sprite("E:crate", b, "sheet", "crate", (176, 136, 12, 12), layer=-30)
    s.sprite("E:crate2", b, "sheet", "crate", (188, 136, 12, 12), layer=-30)
    s.sprite("E:crate3", b, "sheet", "crate", (182, 124, 12, 12), layer=-30)
    s.sprite("E:gem-glow", b, "sheet", "glow", (222, 92, 32, 32), sampling="linear", layer=-21,
             tint=(0.25, 0.95, 0.90, 0.45))
    s.sprite("E:gem", b, "badge", "badge", (230, 100, 16, 16), layer=-10)
    s.sprite("E:dialog", b, "sheet", "panel", (150, 10, 160, 46), mode=nine(sprite_art.PANEL_BORDER, 16),
             layer=10)
    s.label(b, "A WISP STIRS.", 160, 20, layer=20)
    s.label(b, "FOLLOW THE LIGHT", 160, 30, layer=20)


# The eight symmetries of the square, plus a 180-degree transform and a mirror parent.
# name: (flip_x, flip_y, rotation degrees, mirror parent)
D4 = [("none", False, False, 0, False), ("flip-x", True, False, 0, False), ("flip-y", False, True, 0, False),
      ("both", True, True, 0, False), ("rot90", False, False, 90, False), ("rot270", False, False, -90, False),
      ("rot90-flip-x", True, False, 90, False), ("rot90-flip-y", False, True, 90, False),
      ("rot180", False, False, 180, False), ("mirror", False, False, 0, True)]


def board_f(s):
    """7 x 7 rulers in every orientation: centred pivots on grid points (all edges and
    texel boundaries on pixel centres) and on pixel centres (aligned), at 1x and 3x."""
    b = "F"
    s.plate("F:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "F", 312, 172)
    rows = [("TIES 1X", 1, 0.0, 6, 24, 16), ("ALIGNED 1X", 1, 0.5, 36, 54, 16),
            ("TIES 3X", 3, 0.0, 70, 100, 30), ("ALIGNED 3X", 3, 0.5, 120, 150, 30)]
    for caption, m, phase, label_y, centre_y, pitch in rows:
        s.label(b, caption, 8, label_y)
        for k, (name, fx, fy, angle, mirror) in enumerate(D4):
            cx = 20 + pitch * k
            if caption.startswith("TIES"):
                s.label(b, str(k), cx - 1, label_y + 8)
            centre = (cx + phase, centre_y + phase)
            key = f"F:{caption}:{k}:{name}"
            if mirror:
                s.entity(f"{key}:parent", {"Transform": transform((wx(b, centre[0]), wy(b, centre[1]), 0), IDENT,
                                                                  (-1, 1, 1))})
                s.sprite(key, b, "sheet", "odd", None, size=(7 * m, 7 * m), local=(0, 0, 0), parent=f"{key}:parent")
                s.meta[key]["center"] = list(centre)
            else:
                s.sprite(key, b, "sheet", "odd", None, center=centre, size=(7 * m, 7 * m), angle=angle,
                         flip_x=fx, flip_y=fy)


# Offsets toward increasing screen coordinate, in px, applied to both axes from a
# tie placement (left/top edges on pixel centres). The rasterizer snaps to 1/256 px.
SWEEP_M = [1, 2, 3, 4, 5, 6, 8]
# 0.625/256 replaces an exact half step, where the snap direction itself is a knife edge.
SWEEP_D = [0.0, 1 / 2048, 1 / 1024, 5 / 2048, 1 / 256, 2 / 256, 3 / 256, 4 / 256, 6 / 256, -1 / 256]
SWEEP_HEAD = ["0", ".125", ".25", ".625", "1", "2", "3", "4", "6", "-1"]


def sweep_layout():
    """(magnification, flipped, delta index, cell x, cell y) for every board G ruler."""
    cells, y = [], 24
    for m in SWEEP_M:
        for flipped in (False, True):
            for c in range(len(SWEEP_D)):
                cells.append((m, flipped, c, 44 + 62 * c, y))
            y += 7 * m + 5
    return cells


def board_g(s):
    b = "G"
    s.plate("G:background", b, (-16, -16, 1056, 672), BG)
    s.label(b, "G", 1016, 632)
    s.label(b, "OFFSET IN 1/256 PX", 44, 4)
    for c, head in enumerate(SWEEP_HEAD):
        s.label(b, head, 44 + 62 * c, 14)
    for m, flipped, c, x, y in sweep_layout():
        if c == 0:
            s.label(b, f"{m}X FLIP" if flipped else f"{m}X", 4, y + 1)
        d = SWEEP_D[c]
        key = f"G:{m}x:{'flip' if flipped else 'plain'}:{c}"
        s.sprite(key, b, "sheet", "odd", None, size=(7 * m, 7 * m), pivot=(0, 1),
                 local=(wx(b, x + 0.5 + d), wy(b, y + 0.5 + d), 0.0), flip_x=flipped, flip_y=flipped, exact=True)
        s.meta[key].update(cell=[x, y], delta_px=d, magnification=m, flipped=flipped)


# Round three: cases chosen to falsify the derivative-scaled tolerance.
HIGH_M = [12, 16, 24, 32]
HIGH_D = [0.0, 5 / 2048, 1 / 256, 2 / 256, 8 / 256]
HIGH_HEAD = ["0", ".625", "1", "2", "8"]
TURN_M = [4, 8]
TURN_D = [0.0, 5 / 2048, 1 / 256, 2 / 256]
MIN_K = [(2, "ruler64", 64), (3, "ruler63", 63), (4, "ruler64", 64)]
MIN_PLACE = [("aligned", 0.0, 0.0), ("tie", 0.5, 0.0), ("tie+.625", 0.5, 5 / 2048), ("tie+1", 0.5, 1 / 256),
             ("tie+2", 0.5, 2 / 256)]
WIDE_M = [1, 2, 4, 8, 16]
WIDE_D = [0.0, 5 / 2048, 1 / 256, 3 / 256]


def board_h(s):
    """High magnification (12x-32x strips), quarter turns, anisotropic scale and
    minification, each on tie placements moved by sub-pixel offsets."""
    b = "H"
    rx, ry = s.regions[("sheet", "odd")][:2]
    s.plate("H:background", b, (-16, -16, 1312, 672), BG)
    s.label(b, "H", 1240, 632)
    s.label(b, "HIGH MAGNIFICATION", 70, 4)
    for c, head in enumerate(HIGH_HEAD):
        s.label(b, head, 70 + 240 * c, 14)
    y = 24
    for m in HIGH_M:
        for flipped in (False, True):
            s.label(b, f"{m}X FLIP" if flipped else f"{m}X", 4, y + 1)
            for c, d in enumerate(HIGH_D):
                x = 70 + 240 * c
                key = f"H:high:{m}x:{'flip' if flipped else 'plain'}:{c}"
                # The ruler's top row as a 7 x 1 strip; left edge on a pixel centre + d.
                s.sprite(key, b, "sheet", (rx, ry, 7, 1), None, size=(7 * m, m), pivot=(0, 1),
                         local=(wx(b, x + 0.5 + d), wy(b, y), 0.0), flip_x=flipped, exact=True)
                s.meta[key].update(cell=[x, y], delta_px=d, magnification=m, flipped=flipped, kind="high")
            y += m + 6
    s.label(b, "QUARTER TURNS 4X 8X", 70, 250)
    k = 0
    for m in TURN_M:
        for angle in (90, -90):
            for c, d in enumerate(TURN_D):
                x0 = 70 + 68 * k
                # Centre on a pixel centre: every edge and texel boundary on a pixel centre, then + d.
                centre = (x0 + 28.5 + d, 262 + 28.5 + d)
                key = f"H:turn:{m}x:{angle}:{c}"
                s.sprite(key, b, "sheet", "odd", None, center=centre, size=(7 * m, 7 * m), angle=angle, exact=True)
                s.meta[key].update(cell=[x0, 262], delta_px=d, magnification=m, angle=angle, kind="turn")
                k += 1
    s.label(b, "ANISOTROPIC", 70, 330)
    k = 0
    for mx, my in ((1, 8), (8, 1)):
        for flipped in (False, True):
            for c, d in enumerate(TURN_D):
                x0 = 70 + 64 * k
                key = f"H:aniso:{mx}x{my}:{'flip' if flipped else 'plain'}:{c}"
                s.sprite(key, b, "sheet", "odd", None, size=(7 * mx, 7 * my), pivot=(0, 1),
                         local=(wx(b, x0 + 0.5 + d), wy(b, 342 + 0.5 + d), 0.0), flip_x=flipped, flip_y=flipped,
                         exact=True)
                s.meta[key].update(cell=[x0, 342], delta_px=d, magnification=[mx, my], flipped=flipped,
                                   kind="aniso")
                k += 1
    s.label(b, "MINIFY", 70, 410)
    y = 422
    for k_texels, region, n in MIN_K:
        s.label(b, f"1/{k_texels}", 4, y + 1)
        size = n // k_texels
        c = 0
        for flipped in (False, True):
            for name, phase, d in MIN_PLACE:
                x0 = 70 + 40 * c
                key = f"H:min:{k_texels}:{'flip' if flipped else 'plain'}:{name}"
                s.sprite(key, b, "ruler64", region, None, size=(size, size), pivot=(0, 1),
                         local=(wx(b, x0 + phase + d), wy(b, y + phase + d), 0.0), flip_x=flipped,
                         flip_y=flipped, exact=True)
                s.meta[key].update(cell=[x0, y], delta_px=d, phase=phase, texels_per_pixel=k_texels,
                                   flipped=flipped, placement=name, size_px=size, kind="minify")
                c += 1
        y += size + 6


def board_i(s):
    """A 4096 x 1 texel strip magnified 1x-16x, so visible region-local texel
    coordinates sit near 4096. Plain strips end on screen; flipped ones start on it."""
    b = "I"
    s.plate("I:background", b, (-16, -16, 1056, 672), BG)
    s.label(b, "I", 1016, 632)
    s.label(b, "WIDE STRIP NEAR T 4096", 100, 4)
    y = 16
    for m in WIDE_M:
        s.label(b, f"{m}X", 4, y)
        y += 8
        for flipped in (False, True):
            for c, d in enumerate(WIDE_D):
                key = f"I:{m}x:{'flip' if flipped else 'plain'}:{c}"
                if flipped:
                    pivot, local = (0, 1), (wx(b, 100 + 0.5 + d), wy(b, y), 0.0)
                else:
                    pivot, local = (1, 1), (wx(b, 900 + 0.5 + d), wy(b, y), 0.0)
                s.sprite(key, b, "strip", "strip", None, size=(4096 * m, m), pivot=pivot, local=local,
                         flip_x=flipped, exact=True)
                s.meta[key].update(row_y=y, delta_px=d, magnification=m, flipped=flipped, kind="wide")
                y += m + 4


MAP_W = [512, 1024, 2048, 4096, 8192]
MAP_M = [1, 2, 4, 8, 16, 32]


def board_j(s):
    """Exact ties across region widths and magnifications: each row holds a plain
    strip ending at x = 780.5 and a flipped one starting at x = 820.5, so visible
    region-local texels sit near the region width."""
    b = "J"
    s.plate("J:background", b, (-16, -16, 1056, 672), BG)
    s.label(b, "J", 1016, 632)
    s.label(b, "TIES ACROSS REGION WIDTH", 100, 4)
    y = 16
    for w in MAP_W:
        s.label(b, f"W{w}", 784, y)
        for m in MAP_M:
            for flipped in (False, True):
                key = f"J:w{w}:{m}x:{'flip' if flipped else 'plain'}"
                pivot, edge = ((0, 1), 820.5) if flipped else ((1, 1), 780.5)
                s.sprite(key, b, "strip8192", f"w{w}", None, size=(w * m, m), pivot=pivot,
                         local=(wx(b, edge), wy(b, y), 0.0), flip_x=flipped, exact=True)
                s.meta[key].update(row_y=y, width=w, magnification=m, flipped=flipped, kind="map")
            y += m + 3


# Round four: interior spans of the wide-region correction (source 0494b40), which
# cuts nearest geometry every 512 source texels. Every row is a long strip whose
# visible window is centred on one region-local texel boundary b, placed on the pixel
# centre 512.5 plus an offset d, so every texel boundary in a row is an exact tie when
# d = 0. Each row records its exact screen-to-texel mapping (rational "segments"), so
# the analysis can name the expected texel of every pixel without the CPU reference.
SPAN_M = [1, 2, 3, 4, 5, 6, 8, 12, 16, 24, 32, 48, 64]
SPAN_D = [Fraction(0), Fraction(1, 256), Fraction(-1, 256)]
SPAN_LOCAL = {"cut": 0, "mid": 256, "upper": 448}  # span texel at the window centre
SPAN_BASE = 5120  # [5120, 5632] is a span of an 8192-texel region in either direction
CENTRE = Fraction(1025, 2)  # pixel centre 512.5 of a 1024-px board
THICK = 3  # rows are 3 px across, 1 px apart


def rational(v):
    v = Fraction(v)
    return f"{v.numerator}/{v.denominator}"


def strip_row(s, board, key, *, tex, region, m, orient, b, d, at, thick=THICK, along="x", decode=True,
              tint=(1, 1, 1, 1), sampling="nearest", meta=None):
    """One straight strip whose region-local texel boundary b sits on screen at 512.5 + d.

    `along` is the screen axis of the row ("x": a row at screen y `at`; "y": a column
    at screen x `at`). The texture axis follows from the texture: strip8192 names texels
    along x, column8192 along y. `orient` is plain, flip, mirror (parent scale -1),
    rot180, rot+90 or rot-90.
    """
    tex_axis = "y" if tex == "column8192" else "x"
    n = region[3] if tex_axis == "y" else region[2]
    length = n * m
    # Texel coordinates at the screen-start and screen-end of the row.
    if along == "x":
        increasing = {"plain": True, "flip": False, "mirror": False, "rot180": False,
                      "rot+90": True, "rot-90": False}[orient]
    else:
        increasing = {"plain": True, "flip": False, "rot180": False, "rot+90": False, "rot-90": True}[orient]
    t0, t1 = (0, n) if increasing else (n, 0)
    target = CENTRE + d
    start = target - abs(b - t0) * m  # screen coordinate of the row's first end
    cross0, cross1 = at, at + thick
    flip_x = flip_y = False
    angle, parent = 0.0, None
    pivot = (0, 0)
    # Strips are long in local x and columns in local y, whatever the screen axis.
    size = (length, thick) if tex_axis == "x" else (thick, length)
    if along == "x":
        if orient in ("plain", "flip"):
            flip_x = orient == "flip"
            local = (wx(board, start), wy(board, cross1), 0.0)
        elif orient == "mirror":
            parent = f"{key}:parent"
            s.entity(parent, {"Transform": transform((wx(board, start + length), wy(board, cross1), 0.0), IDENT,
                                                     (-1, 1, 1), exact=True)})
            local = (0.0, 0.0, 0.0)
        elif orient == "rot180":
            angle, local = 180, (wx(board, start + length), wy(board, cross0), 0.0)
        elif orient == "rot-90":  # local +y runs toward screen +x
            angle, local = -90, (wx(board, start), wy(board, cross0), 0.0)
        else:  # rot+90: local +y runs toward screen -x
            angle, local = 90, (wx(board, start + length), wy(board, cross1), 0.0)
    else:
        if orient in ("plain", "flip"):  # columns: local bottom at the far screen-y end
            flip_y = orient == "flip"
            local = (wx(board, cross0), wy(board, start + length), 0.0)
        elif orient == "rot180":
            angle, local = 180, (wx(board, cross1), wy(board, start), 0.0)
        elif orient == "rot+90":  # local +x runs toward screen -y
            angle, local = 90, (wx(board, cross1), wy(board, start + length), 0.0)
        else:  # rot-90: local +x runs toward screen +y
            angle, local = -90, (wx(board, cross0), wy(board, start), 0.0)
    s.sprite(key, board, tex, list(region), None, size=size, pivot=pivot, local=local, angle=angle,
             flip_x=flip_x, flip_y=flip_y, tint=tint, sampling=sampling, parent=parent, exact=True)
    s.meta[key].update(kind="span", along=along, line=at + thick // 2, texture_axis=tex_axis, region=list(region),
                       magnification=m, orient=orient, b=b, d=rational(d), decode=decode and tint[3] == 1
                       and sampling == "nearest",
                       segments=[[rational(start), rational(start + length), t0, t1]], **(meta or {}))


def nine_row(s, board, key, *, mb, mc, flipped, window, b, at, compressed=False, d=Fraction(0)):
    """A 2000 x 1 nine-slice strip, borders 600 (left) and 700 (right) texels at mb px
    per texel and a 700-texel centre at mc, or compressed to half size (no centre)."""
    left, right, n = 600, 700, 2000
    if compressed:
        # Borders at half density, centre collapsed: 300 mb + 350 mb px.
        quads = [(right, mb // 2, n, n - right), (left, mb // 2, left, 0)] if flipped else \
                [(left, mb // 2, 0, left), (right, mb // 2, n - right, n)]
    else:
        quads = ([(right, mb, n, n - right), (n - left - right, mc, n - right, left), (left, mb, left, 0)]
                 if flipped else [(left, mb, 0, left), (n - left - right, mc, left, n - right), (right, mb, n - right, n)])
    segments, x, offset = [], Fraction(0), None
    for texels, m, ta, tb in quads:
        segments.append([x, x + texels * m, ta, tb])
        lo, hi = min(ta, tb), max(ta, tb)
        if offset is None and lo <= b <= hi:
            offset = x + abs(b - ta) * m
        x += texels * m
    if compressed:  # the junction where the two borders meet
        offset = segments[0][1] if window == "junction" else offset
    start = CENTRE + d - offset
    for seg in segments:
        seg[0], seg[1] = rational(seg[0] + start), rational(seg[1] + start)
    s.sprite(key, board, "strip8192", [0, 0, n, 1], None, size=(int(x), THICK), pivot=(0, 0),
             local=(wx(board, start), wy(board, at + THICK), 0.0), flip_x=flipped,
             mode=nine((left, right, 0, 0), PPU / mb), exact=True)
    s.meta[key].update(kind="span", along="x", line=at + THICK // 2, texture_axis="x", region=[0, 0, n, 1],
                       magnification=[mb, mc], orient="flip" if flipped else "plain", b=b, d=rational(d),
                       window=window, decode=True, segments=segments, nine_slice=True, compressed=compressed)


def label4(s, board, text, x, y):
    reg = s.regions[("labels4", f"label:{text}")]
    return s.sprite(f"{board}:label:{text}:{x},{y}", board, "labels4", f"label:{text}", (x, y, reg[2], reg[3]),
                    pivot=(0, 1), layer=50)


def span_windows(m):
    return ["cut"] if m <= 2 else list(SPAN_LOCAL)


def board_k(s):
    """Spans along screen x: every magnification, both directions, offsets 0 and +-1/256,
    windows on a cut and inside a span, plus quarter-turned columns (texture y on screen x)."""
    b = "K"
    s.plate("K:background", b, (-16, -16, 1056, 1056), BG)
    label4(s, b, "K", 1012, 4)
    label4(s, b, "SPANS ALONG X", 8, 4)
    y = 16
    for m in SPAN_M:
        for orient in ("plain", "flip"):
            for k, d in enumerate(SPAN_D):
                for window in span_windows(m):
                    bnd = SPAN_BASE + SPAN_LOCAL[window]
                    strip_row(s, b, f"K:{m}x:{orient}:{k}:{window}", tex="strip8192", region=(0, 0, 8192, 1),
                              m=m, orient=orient, b=bnd, d=d, at=y, meta={"window": window, "family": "x"})
                    y += THICK + 1
    for m in (8, 48):
        for orient in ("rot-90", "rot+90"):
            for k, d in enumerate(SPAN_D[:2]):
                for window in ("cut", "mid"):
                    bnd = SPAN_BASE + SPAN_LOCAL[window]
                    strip_row(s, b, f"K:col:{m}x:{orient}:{k}:{window}", tex="column8192", region=(0, 0, 1, 8192),
                              m=m, orient=orient, b=bnd, d=d, at=y, meta={"window": window, "family": "column turned"})
                    y += THICK + 1
    assert y <= 1024, y


def board_l(s):
    """Spans along screen y: the same sweep on the 1 x 8192 column (source y), plus
    quarter-turned strips (texture x on screen y) and a half-turned column."""
    b = "L"
    s.plate("L:background", b, (-16, -16, 1056, 1056), BG)
    label4(s, b, "L", 1012, 1012)
    label4(s, b, "SPANS ALONG Y", 960, 4)
    x = 16
    for m in SPAN_M:
        for orient in ("plain", "flip"):
            for k, d in enumerate(SPAN_D):
                for window in span_windows(m):
                    bnd = SPAN_BASE + SPAN_LOCAL[window]
                    strip_row(s, b, f"L:{m}x:{orient}:{k}:{window}", tex="column8192", region=(0, 0, 1, 8192),
                              m=m, orient=orient, b=bnd, d=d, at=x, along="y", meta={"window": window, "family": "y"})
                    x += THICK + 1
    for m in (8, 48):
        for orient in ("rot-90", "rot+90", "rot180"):
            tex, region = ("column8192", (0, 0, 1, 8192)) if orient == "rot180" else ("strip8192", (0, 0, 8192, 1))
            for k, d in enumerate(SPAN_D[:2]):
                for window in ("cut", "mid"):
                    bnd = SPAN_BASE + SPAN_LOCAL[window]
                    strip_row(s, b, f"L:turn:{m}x:{orient}:{k}:{window}", tex=tex, region=region, m=m, orient=orient,
                              b=bnd, d=d, at=x, along="y", meta={"window": window, "family": "y turned"})
                    x += THICK + 1
    assert x <= 958, x


# Region variants: (name, region, windows as {orientation: {window: boundary b}}).
# R4097 ends in a 1-texel span; R8191 starts at texel 1; R1000 crosses texel 4096.
SPAN_REGIONS = [
    ("r4097", (300, 0, 4097, 1), {"plain": {"cut": 2048, "end": 4097}, "flip": {"cut": 2049, "end": 0}}),
    ("r8191", (1, 0, 8191, 1), {"plain": {"cut": 4096, "end": 8191}, "flip": {"cut": 4095, "end": 0}}),
    ("r1000", (4000, 0, 1000, 1), {"plain": {"cut": 512, "end": 1000}, "flip": {"cut": 488, "end": 0}}),
]


def board_m(s):
    """Span variants: region origins and widths that are not multiples of 512, mirrors,
    half turns, wide nine-slice borders (also compressed), translucent and linear rows."""
    b = "M"
    s.plate("M:background", b, (-16, -16, 1056, 1056), BG)
    label4(s, b, "M", 1012, 4)
    label4(s, b, "SPAN VARIANTS", 8, 4)
    y = 16
    for name, region, windows in SPAN_REGIONS:
        for orient in ("plain", "flip"):
            for m in (1, 8, 48):
                for k, d in enumerate(SPAN_D[:2]):
                    for window, bnd in windows[orient].items():
                        strip_row(s, b, f"M:{name}:{m}x:{orient}:{k}:{window}", tex="strip8192", region=region, m=m,
                                  orient=orient, b=bnd, d=d, at=y, meta={"window": window, "family": name})
                        y += THICK + 1
    for name, region, cut in (("w8192", (0, 0, 8192, 1), SPAN_BASE), ("r4097", (300, 0, 4097, 1), 2048)):
        for orient in ("mirror", "rot180"):
            if orient == "rot180" and name != "w8192":
                continue
            for m in (8, 48):
                for k, d in enumerate(SPAN_D[:2]):
                    for window, bnd in (("cut", cut), ("mid", cut + 256)):
                        strip_row(s, b, f"M:{name}:{m}x:{orient}:{k}:{window}", tex="strip8192", region=region, m=m,
                                  orient=orient, b=bnd, d=d, at=y, meta={"window": window, "family": orient})
                        y += THICK + 1
    nine_windows = {False: {"left cut": 512, "left|centre": 600, "centre cut": 1112, "centre|right": 1300,
                            "right cut": 1812},
                    True: {"left cut": 1488, "left|centre": 1300, "centre cut": 788, "centre|right": 600,
                           "right cut": 88}}
    for mb, mc in ((8, 3), (32, 12)):
        for flipped in (False, True):
            for window, bnd in nine_windows[flipped].items():
                nine_row(s, b, f"M:nine:{mb}x{mc}:{'flip' if flipped else 'plain'}:{window}", mb=mb, mc=mc,
                         flipped=flipped, window=window, b=bnd, at=y)
                y += THICK + 1
    for flipped in (False, True):
        for window, bnd in (("left cut", 1488 if flipped else 512), ("junction", None)):
            nine_row(s, b, f"M:nine-compressed:8x:{'flip' if flipped else 'plain'}:{window}", mb=8, mc=0,
                     flipped=flipped, window=window, b=bnd if bnd is not None else (1300 if flipped else 600), at=y,
                     compressed=True)
            y += THICK + 1
    for m in (1, 8, 48):
        for orient in ("plain", "flip"):
            strip_row(s, b, f"M:translucent:{m}x:{orient}", tex="strip8192", region=(0, 0, 8192, 1), m=m,
                      orient=orient, b=SPAN_BASE, d=Fraction(0), at=y, tint=(1, 1, 1, 0.5),
                      meta={"window": "cut", "family": "translucent"})
            y += THICK + 1
    for orient in ("plain", "flip"):
        strip_row(s, b, f"M:linear:8x:{orient}", tex="strip8192", region=(0, 0, 8192, 1), m=8, orient=orient,
                  b=SPAN_BASE, d=Fraction(0), at=y, sampling="linear", meta={"window": "cut", "family": "linear"})
        y += THICK + 1
    assert y <= 1024, y


def board_n(s):
    """Isotropic rows (1 texel tall, M px high) at 32x, 48x and 64x on cut, mid and upper
    windows, unflipped and flipped. Also viewed at 1920 x 1080 and through a rolled camera."""
    b = "N"
    s.plate("N:background", b, (-16, -16, 1056, 1056), BG)
    label4(s, b, "N", 1012, 4)
    label4(s, b, "ISOTROPIC 32X 48X 64X", 8, 4)
    y = 16
    for orient in ("plain", "flip"):
        for m in (32, 48, 64):
            for window in SPAN_LOCAL:
                strip_row(s, b, f"N:{m}x:{orient}:{window}", tex="strip8192", region=(0, 0, 8192, 1), m=m,
                          orient=orient, b=SPAN_BASE + SPAN_LOCAL[window], d=Fraction(0), at=y, thick=m,
                          meta={"window": window, "family": "isotropic"})
                y += m + 4
    assert y <= 1024, y


# Opt-in probe (--probe): where exact and near ties start to fail between 20x and 128x,
# by span texel, along screen x (O) and y (P), on the dyadic 1024 x 1024 view and at
# 1920 x 1080. The default scene, and so every other frame, is unchanged without it.
PROBE_M = [20, 24, 28, 32, 36, 40, 44, 48, 56, 64, 80, 96, 128]
PROBE_LOCAL = [32, 288, 480]
PROBE_SPEC = {"O": (0.0, 1200.0, 1024, 1024), "P": (800.0, 0.0, 1024, 1024)}


def board_probe(s, b):
    along, tex, region = ("x", "strip8192", (0, 0, 8192, 1)) if b == "O" else ("y", "column8192", (0, 0, 1, 8192))
    s.plate(f"{b}:background", b, (-16, -16, 1056, 1056), BG)
    at = 16
    for m in PROBE_M:
        for orient in ("plain", "flip"):
            for k, d in enumerate(SPAN_D[:2]):
                for local in PROBE_LOCAL:
                    strip_row(s, b, f"{b}:{m}x:{orient}:{k}:{local}", tex=tex, region=region, m=m, orient=orient,
                              b=SPAN_BASE + local, d=d, at=at, along=along,
                              meta={"window": f"span texel {local}", "family": f"probe {along}"})
                    at += THICK + 1
    assert at <= 1024, at


def occluder_gltf(models):
    """One emissive quad: board C, logical x 252..300, y 86..118, z = 1 (in front of z = 0)."""
    x0, x1, y0, y1 = wx("C", 252), wx("C", 300), wy("C", 118), wy("C", 86)
    positions = [(x0, y0, 1.0), (x1, y0, 1.0), (x1, y1, 1.0), (x0, y1, 1.0)]
    indices = [0, 1, 2, 0, 2, 3]
    blob = b"".join(struct.pack("<3f", *p) for p in positions)
    blob += b"".join(struct.pack("<3f", 0, 0, 1) for _ in positions)
    blob += b"".join(struct.pack("<I", i) for i in indices)
    document = {
        "asset": {"version": "2.0", "generator": "handoff 0032 sprite_lookdev.py"},
        "buffers": [{"uri": "occluder.bin", "byteLength": len(blob)}],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962},
                        {"buffer": 0, "byteOffset": 48, "byteLength": 48, "target": 34962},
                        {"buffer": 0, "byteOffset": 96, "byteLength": 24, "target": 34963}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
             "min": [x0, y0, 1.0], "max": [x1, y1, 1.0]},
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5125, "count": 6, "type": "SCALAR"}],
        "meshes": [{"name": "occluder", "primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2,
                                                        "material": 0}]}],
        "materials": [{"name": "occluder", "pbrMetallicRoughness": {"baseColorFactor": [0, 0, 0, 1],
                                                                     "metallicFactor": 1},
                       "emissiveFactor": [0.10, 0.22, 0.24]}],
        "nodes": [{"name": "occluder", "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0}
    (models / "occluder.bin").write_bytes(blob)
    (models / "occluder.gltf").write_text(json.dumps(document, indent=1), encoding="utf-8")
    return {"positions": positions, "emissive": [0.10, 0.22, 0.24]}


# -------------------------------------------------------------------- cameras --

def camera_value(projection):
    value = {"fov_degrees": FOV, "near": NEAR, "far": FAR}
    if projection is not None:
        value["projection"] = projection
    return value


def ortho(vertical):
    return {"kind": "orthographic", "vertical_size": vertical}


CAMERAS = {}
for _b in BOARDS:
    CAMERAS[f"pp-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0, "projection": PIXEL}
    CAMERAS[f"ortho3-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0, "projection": ortho(11.25)}
    # 60 px/m: 3.75 output px per texel, offset (+6.22, -2.95) output px from the board centre.
    CAMERAS[f"orthof-{_b}"] = {"board": _b, "offset": (0.1037, -0.0491), "roll": 0, "projection": ortho(9.0)}
for _b in ("B", "D", "E"):
    CAMERAS[f"roll7-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 7, "projection": ortho(9.0)}
CAMERAS["pp-A-odd"] = {"board": "A", "offset": (0, 0), "roll": 0,
                       "projection": dict(PIXEL, reference_resolution=[319, 179])}
CAMERAS["pp-A-sub"] = {"board": "A", "offset": (0.6 / PPU, 0.2 / PPU), "roll": 0, "projection": PIXEL}
CAMERAS["pp-F"] = {"board": "F", "offset": (0, 0), "roll": 0, "projection": PIXEL}
CAMERAS["orthof-F"] = {"board": "F", "offset": (0.1037, -0.0491), "roll": 0, "projection": ortho(9.0)}
CAMERAS["pp-G"] = {"board": "G", "offset": (0, 0), "roll": 0,
                   "projection": dict(PIXEL, reference_resolution=[1024, 640])}
CAMERAS["roll7-G"] = {"board": "G", "offset": (-10.0, 3.6), "roll": 7, "projection": ortho(32.0)}
CAMERAS["pp-H"] = {"board": "H", "offset": (0, 0), "roll": 0,
                   "projection": dict(PIXEL, reference_resolution=[1280, 640])}
CAMERAS["pp-I"] = {"board": "I", "offset": (0, 0), "roll": 0,
                   "projection": dict(PIXEL, reference_resolution=[1024, 640])}
CAMERAS["pp-J"] = {"board": "J", "offset": (0, 0), "roll": 0,
                   "projection": dict(PIXEL, reference_resolution=[1024, 640])}
for _b in ("K", "L", "M", "N"):
    CAMERAS[f"pp-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0,
                           "projection": dict(PIXEL, reference_resolution=[1024, 1024])}
# The same board N at a common display size, whose 60 m half-width is not dyadic, and rolled.
CAMERAS["pp-N-hd"] = {"board": "N", "offset": (0, 0), "roll": 0,
                      "projection": dict(PIXEL, reference_resolution=[1920, 1080])}
CAMERAS["roll7-N"] = {"board": "N", "offset": (0, 0), "roll": 7, "projection": ortho(64.0)}

SHOTS = ([(f"pp-{b}", (320, 180)) for b in BOARDS] + [(f"pp-{b}", (960, 540)) for b in BOARDS]
         + [(f"ortho3-{b}", (960, 540)) for b in BOARDS] + [(f"orthof-{b}", (960, 540)) for b in BOARDS]
         + [(f"roll7-{b}", (960, 540)) for b in ("B", "D", "E")]
         + [("pp-A-odd", (319, 179)), ("pp-A-sub", (320, 180))]
         + [("pp-F", (320, 180)), ("pp-F", (960, 540)), ("orthof-F", (960, 540)), ("pp-G", (1024, 640))]
         + [("roll7-G", (960, 540)), ("pp-H", (1280, 640)), ("pp-I", (1024, 640)), ("pp-J", (1024, 640))]
         + [(f"pp-{b}", (1024, 1024)) for b in "KLMN"] + [("pp-N-hd", (1920, 1080)), ("roll7-N", (1024, 1024))])
SPAN_SHOTS = [(f"pp-{b}", (1024, 1024)) for b in "KLMN"] + [("pp-N-hd", (1920, 1080))]
PROBE_CAMERAS = {}
for _b in ("O", "P"):
    PROBE_CAMERAS[f"pp-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0,
                                 "projection": dict(PIXEL, reference_resolution=[1024, 1024])}
    PROBE_CAMERAS[f"pp-{_b}-hd"] = {"board": _b, "offset": (0, 0), "roll": 0,
                                    "projection": dict(PIXEL, reference_resolution=[1920, 1080])}
PROBE_SHOTS = [("pp-O", (1024, 1024)), ("pp-P", (1024, 1024)), ("pp-O-hd", (1920, 1080)), ("pp-P-hd", (1920, 1080))]


# ---------------------------------------------------------------- live script --

BEHAVIOR = """
export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const t = state.tick;
    state.tick++;
    if (t >= STEPS.length) return;
    const step = STEPS[t]!;
    const wisp = api.query('SpriteRenderer').find((e) => e.id === WISP_ID)!;
    const sprite = wisp.components.SpriteRenderer as SpriteRenderer;
    const value: SpriteRenderer = { ...sprite, region: step.region };
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Transform', value: step.camera });
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: WISP_ID, component: 'Transform', value: step.wisp });
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: WISP_ID, component: 'SpriteRenderer', value });
    api.log(JSON.stringify({ t, x: step.camera.translation[0], y: step.camera.translation[1], region: step.region[0] }));
  },
});
"""


def build(out, art, directions, probe=False):
    project_dir = out / "project"
    project = project_dir / "sprites.incant.json"
    journal = project_dir / "sprites.incant.journal.jsonl"
    (project_dir / "art").mkdir(parents=True)
    (project_dir / "models").mkdir()
    engine("init", project, "--name", "Sprite Look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))
    regions = json.loads((art / "regions.json").read_text(encoding="utf-8"))
    imports = {}

    def importer(source_name, target_name, usage=None):
        shutil.copyfile(art / source_name, project_dir / "art" / target_name)
        args = ["import", project, f"art/{target_name}"] + (["--texture-usage", usage] if usage else [])
        return engine(*args)

    textures = {}
    for name, file in (("sheet", "sheet.png"), ("labels", "labels.png"), ("badge", "badge.png"),
                       ("tone16", "tone16.png"), ("ruler64", "ruler64.png"), ("strip", "strip4096.png"),
                       ("strip8192", "strip8192.png"), ("column8192", "column8192.png"), ("labels4", "labels4.png")):
        result = importer(file, file, usage="color")
        reply = json.loads(result.stdout)
        textures[name] = reply["asset"]["id"]
        imports[name] = {"asset": reply["asset"], "details": reply["details"], "changed": reply["changed"]}
    occluder = occluder_gltf(project_dir / "models")
    reply = json.loads(engine("import", project, "models/occluder.gltf").stdout)
    occluder_asset = reply["asset"]["id"]
    imports["occluder"] = {"asset": reply["asset"], "details": reply["details"]}

    s = Scene(textures)
    for tex, table in (("sheet", regions["sheet"]), ("labels", regions["labels"]), ("badge", regions["badge"]),
                       ("tone16", regions["tone16"]), ("ruler64", regions["ruler64"]),
                       ("strip", regions["strip4096"]), ("strip8192", regions["strip8192"]),
                       ("column8192", regions["column8192"]), ("labels4", regions["labels4"])):
        for key, rect in table.items():
            s.regions[(tex, key)] = rect
    for builder in (board_a, board_b, board_c, board_d, board_e, board_f, board_g, board_h, board_i, board_j,
                    board_k, board_l, board_m, board_n):
        builder(s)
    if probe:
        for b in PROBE_SPEC:
            board_probe(s, b)
    s.entity("C:occluder", {"Transform": transform((0, 0, 0)),
                            "MeshRenderer": {"mesh": occluder_asset, "materials": [], "cast_shadows": False}})
    cameras = {}
    for name, spec in (dict(CAMERAS, **PROBE_CAMERAS) if probe else CAMERAS).items():
        bx, by = BOARD_SPEC[spec["board"]][:2]
        t = (bx + spec["offset"][0], by + spec["offset"][1], EYE_Z)
        cameras[name] = s.entity(f"camera:{name}", {"Camera": camera_value(spec["projection"]),
                                                     "Transform": transform(t, quat_z(spec["roll"]))})
    live = s.entity("camera:live", {"Camera": camera_value(PIXEL), "Transform": transform((BOARD_X["E"], 0, EYE_Z))})

    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in s.entities],
            "expected_revision": revision, "description": "Sprite look-dev scene (handoff 0032)"}},
        {"id": 3, "method": "project.save"}])
    engine("validate", project)

    wisp_id = s.ids["E:wisp"]
    wisp_t0 = s.entities[[e["id"] for e in s.entities].index(wisp_id)]["components"]["Transform"]["translation"]
    scripts = {}
    for key, (ux, uy) in directions.items():
        steps = []
        for t in range(LIVE_TICKS):
            k = (t + 1) * STEP  # commanded at tick t
            frame = ((t + 1) // 4) % 4
            steps.append({"camera": transform((BOARD_X["E"] + ux * k, uy * k, EYE_Z)),
                          "wisp": transform((wisp_t0[0] + ux * k, wisp_t0[1] + uy * k, wisp_t0[2])),
                          "region": s.regions[("sheet", f"wisp{frame}")]})
        source = out / "scripts" / f"live_{key}.ts"
        source.parent.mkdir(exist_ok=True)
        source.write_text(
            "import type { ScriptApi, SpriteRenderer, Transform } from "
            + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n"
            + f"const SCENE_ID = {json.dumps(scene_id)};\nconst LIVE_ID = {json.dumps(live)};\n"
            + f"const WISP_ID = {json.dumps(wisp_id)};\n"
            + "const STEPS: { camera: Transform; wisp: Transform; region: [number, number, number, number] }[] = "
            + json.dumps(steps) + ";\n" + BEHAVIOR, encoding="utf-8")
        run("node", ROOT / "node_modules/typescript/bin/tsc", "--strict", "--noEmit", "--target", "ES2022",
            "--module", "ESNext", "--moduleResolution", "bundler", source)
        scripts[key] = out / "scripts" / f"live_{key}.js"
        run("node", ROOT / "tools/build_script.mjs", source, scripts[key])

    scene = {
        "scene": scene_id, "ppu": PPU, "logical": LOGICAL, "eye_z": EYE_Z, "near": NEAR, "far": FAR, "fov": FOV,
        "board_x": BOARD_X, "board_spec": BOARD_SPEC, "background": BG, "textures": textures, "imports": imports,
        "d4": [d[0] for d in D4], "sweep": {"magnifications": SWEEP_M, "deltas_px": SWEEP_D},
        "regions": {f"{t}/{k}": v for (t, k), v in s.regions.items()},
        "entities": s.entities, "meta": s.meta, "ids": s.ids, "occluder": occluder,
        "cameras": {k: dict(v, id=cameras[k]) for k, v in (dict(CAMERAS, **PROBE_CAMERAS) if probe
                                                             else CAMERAS).items()},
        "live": {"id": live, "wisp": wisp_id, "wisp_t0": wisp_t0, "step": STEP, "ticks": LIVE_TICKS,
                 "directions": directions},
    }
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, journal, scene, scripts


BUDGET = {"frames": 0, "raw_bytes": 0, "invocations": 0, "largest_invocation_frames": 0,
          "largest_invocation_raw_bytes": 0}


def account(frames, raw):
    """Record one renderer invocation; refuse anything beyond the per-invocation limit."""
    if frames > MAX_FRAMES or raw > MAX_RAW:
        raise SystemExit(f"invocation of {frames} frames, {raw} raw bytes exceeds the budget")
    BUDGET["frames"] += frames
    BUDGET["raw_bytes"] += raw
    BUDGET["invocations"] += 1
    BUDGET["largest_invocation_frames"] = max(BUDGET["largest_invocation_frames"], frames)
    BUDGET["largest_invocation_raw_bytes"] = max(BUDGET["largest_invocation_raw_bytes"], raw)


def screenshot(project, path, camera, size, check=True):
    w, h = size
    account(1, w * h * 4)
    return engine("screenshot", project, path, "--camera", camera, "--width", w, "--height", h, check=check)


def shoot_pair(project, folder, name, camera, size, record, repeats=2):
    w, h = size
    hashes, info = [], None
    for repeat in range(1, repeats + 1):
        png = folder / f"{name}-{w}x{h}-r{repeat}.png"
        result = screenshot(project, png, camera, size)
        hashes.append(sha(png))
        if info is None:
            reply = json.loads(result.stdout)
            info = {"adapter": reply.get("adapter"), "geometry": reply.get("geometry")}
    record[f"{name}-{w}x{h}"] = {"r1": hashes[0], "r2": hashes[-1] if repeats > 1 else None,
                                 "equal": hashes[0] == hashes[-1] if repeats > 1 else None, **info}


def authored(project, journal):
    return {p.name: sha(p) for p in (project, journal)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--binary", choices=sorted(BINARIES), default="active",
                        help="active (artifacts/tools/incant_headless), pre-wide, pre-magnification or pre-ties")
    parser.add_argument("--repeats", type=int, choices=(1, 2), default=2, help="renders of each static frame")
    parser.add_argument("--live", default=",".join(DIRECTIONS),
                        help="comma-separated live directions to play twice each, or 'none'")
    parser.add_argument("--static-only", action="store_true", help="static frames only: no reimport, "
                        "source-free or live captures")
    parser.add_argument("--probe", action="store_true", help="add probe boards O and P and capture only their "
                        "four views (implies --static-only)")
    args = parser.parse_args()
    global SHOTS
    if args.probe:
        args.static_only = True
        BOARD_SPEC.update(PROBE_SPEC)
        SHOTS = PROBE_SHOTS
    global CLI, BINARY_JSON
    CLI = ROOT / "artifacts/tools" / BINARIES[args.binary][0]
    BINARY_JSON = ROOT / "artifacts/tools" / BINARIES[args.binary][1]
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    keys = [] if args.live == "none" or args.static_only else args.live.split(",")
    if any(k not in DIRECTIONS for k in keys):
        raise SystemExit(f"unknown live direction in {args.live}")
    directions = {k: DIRECTIONS[k] for k in keys}
    start = verify_binary()
    out.mkdir(parents=True)
    run(sys.executable, "-I", TOOLS / "sprite_art.py", out / "art")
    art = out / "art"
    project, journal, scene, scripts = build(out, art, directions, args.probe)
    project_dir = project.parent
    cams = {k: v["id"] for k, v in scene["cameras"].items()}
    metadata = json.loads(BINARY_JSON.read_text(encoding="utf-8"))
    summary = {"binary": args.binary, "binary_file": CLI.name, "binary_source_commit": metadata["source_commit"],
               "binary_sha256_start": start, "plan": {"repeats": args.repeats, "live": keys,
                                                      "static_only": args.static_only, "probe": args.probe}}

    # 1. Static frames, each rendered `repeats` times.
    before = authored(project, journal)
    shots_dir = out / "shots"
    shots_dir.mkdir()
    shots = {}
    for name, size in SHOTS:
        shoot_pair(project, shots_dir, name, cams[name], size, shots, args.repeats)
    summary["shots"] = shots
    summary["authored_unchanged_by_rendering"] = before == authored(project, journal)
    summary["authored_unchanged_by_play"] = True

    if not args.static_only:
        # 2. Texture reimport through the public import command.
        re_dir = out / "reimport"
        re_dir.mkdir()
        steps = {}

        def reimport(label, source_name, target_name, usage=None, expect_error=False):
            prior = authored(project, journal)
            shutil.copyfile(art / source_name, project_dir / "art" / target_name)
            cmd = ["import", project, f"art/{target_name}"] + (["--texture-usage", usage] if usage else [])
            result = engine(*cmd, check=not expect_error)
            entry = {"exit": result.returncode, "stderr": result.stderr.strip()[-600:],
                     "authored_unchanged": prior == authored(project, journal)}
            if result.returncode == 0:
                reply = json.loads(result.stdout)
                entry.update(asset=reply["asset"], changed=reply["changed"], cache_hit=reply["cache_hit"],
                             details=reply["details"])
            steps[label] = entry
            return entry

        def shoot(label, name, size=(320, 180)):
            """Never aborts: the analysis judges each recorded exit status and image."""
            png = re_dir / f"{label}-{name}-{size[0]}x{size[1]}.png"
            result = screenshot(project, png, cams[name], size, check=False)
            entry = {"exit": result.returncode, "image_written": png.exists(),
                     "sha256": sha(png) if png.exists() else None, "stderr": result.stderr.strip()[-600:]}
            steps.setdefault(f"{label}:shots", {})[f"{name}-{size[0]}x{size[1]}"] = entry
            return entry

        reimport("1-sheet-teal", "sheet-teal.png", "sheet.png")
        for b in BOARDS:
            shoot("1-sheet-teal", f"pp-{b}")
        reimport("2-badge-orange", "badge-orange.png", "badge.png")
        shoot("2-badge-orange", "pp-C")
        shoot("2-badge-orange", "pp-E")
        reimport("3-badge-small", "badge-small.png", "badge.png")
        shoot("3-badge-small", "pp-C")  # expected: explicit source-region failure, no image
        shoot("3-badge-small", "pp-A")  # board A does not draw the badge; is the whole project prepared?
        reimport("4-badge-restore", "badge.png", "badge.png")
        shoot("4-badge-restore", "pp-C")
        reimport("5-sheet-linear", "sheet-teal.png", "sheet.png", usage="linear", expect_error=True)
        reimport("6-sheet-restore", "sheet.png", "sheet.png")
        for b in BOARDS:
            shoot("6-sheet-restore", f"pp-{b}")
        shoot("6-sheet-restore", "ortho3-E", (960, 540))
        # Same texels and regions in a taller image: frames must not change at all.
        reimport("7-sheet-tall", "sheet-tall.png", "sheet.png")
        for b in BOARDS + ["F"]:
            shoot("7-sheet-tall", f"pp-{b}")
        shoot("7-sheet-tall", "orthof-D", (960, 540))
        shoot("7-sheet-tall", "pp-G", (1024, 640))
        shoot("7-sheet-tall", "pp-H", (1280, 640))
        reimport("8-sheet-restore", "sheet.png", "sheet.png")
        shoot("8-sheet-restore", "pp-E")
        # Round four: the wide strip twice as tall and the column twice as wide, with
        # red texels outside every region. Span frames must not change at all.
        reimport("9-strip-tall", "strip8192-tall.png", "strip8192.png")
        shoot("9-strip-tall", "pp-J", (1024, 640))
        for b in "KMN":
            shoot("9-strip-tall", f"pp-{b}", (1024, 1024))
        reimport("10-column-wide", "column8192-wide.png", "column8192.png")
        shoot("10-column-wide", "pp-L", (1024, 1024))
        shoot("10-column-wide", "pp-K", (1024, 1024))
        reimport("11-strip-restore", "strip8192.png", "strip8192.png")
        reimport("12-column-restore", "column8192.png", "column8192.png")
        shoot("12-column-restore", "pp-K", (1024, 1024))
        summary["reimport"] = steps

        # 3. Source-free: delete every source image and model, then render again.
        removed = sorted(str(p.relative_to(project_dir)) for p in project_dir.glob("art/*") if p.is_file())
        removed += sorted(str(p.relative_to(project_dir)) for p in project_dir.glob("models/*") if p.is_file())
        for rel in removed:
            (project_dir / rel).unlink()
        free_dir = out / "source-free"
        free_dir.mkdir()
        free = {}
        for name, size in ([(f"pp-{b}", (320, 180)) for b in BOARDS + ["F"]]
                           + [("pp-G", (1024, 640)), ("pp-H", (1280, 640)), ("pp-I", (1024, 640)),
                              ("pp-J", (1024, 640)), ("ortho3-E", (960, 540)), ("orthof-D", (960, 540)),
                              ("roll7-B", (960, 540))] + SPAN_SHOTS):
            png = free_dir / f"{name}-{size[0]}x{size[1]}.png"
            screenshot(project, png, cams[name], size)
            free[f"{name}-{size[0]}x{size[1]}"] = sha(png)
        summary["source_free"] = {"removed": removed, "shots": free}

        # 4. Live flipbook + pixel-perfect follow, source-free, twice per direction.
        plays = {}
        live_before = authored(project, journal)
        for key in directions:
            runs = []
            for repeat in (1, 2):
                target = out / f"live-{key}-r{repeat}"
                account(1 + LIVE_TICKS, (1 + LIVE_TICKS) * LOGICAL[0] * LOGICAL[1] * 4)
                result = engine("play", project, "--ticks", LIVE_TICKS, "--compiled-script", scripts[key],
                                "--camera", scene["live"]["id"], "--capture-every", 1, "--width", LOGICAL[0],
                                "--height", LOGICAL[1], "--output", target,
                                "--log-output", out / f"live-{key}-r{repeat}.logs.jsonl")
                (out / f"live-{key}-r{repeat}.stdout.json").write_text(result.stdout, encoding="utf-8")
                runs.append({p.name: sha(p) for p in sorted(target.glob("*.png"))})
            plays[key] = {"frames": runs[0], "repeat_equal": runs[0] == runs[1], "count": len(runs[0])}
        summary["plays"] = plays
        summary["authored_unchanged_by_play"] = live_before == authored(project, journal)
    summary["binary_sha256_end"] = verify_binary()
    summary["engine_calls"] = CALLS["count"]
    summary["binary_unchanged_every_call"] = CALLS["unchanged"] and start == summary["binary_sha256_end"]
    # Planned captures, checked per invocation; the written PNGs are counted as well.
    written = [p for p in out.rglob("*.png") if "art" not in p.parts and "project" not in p.parts]
    summary["budget"] = dict(BUDGET, written_pngs=len(written),
                             written_raw_bytes=sum(4 * w * h for w, h in (png_size(p) for p in written)))
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({
        "binary_unchanged": summary["binary_unchanged_every_call"], "engine_calls": CALLS["count"],
        "authored_unchanged": summary["authored_unchanged_by_rendering"] and summary["authored_unchanged_by_play"],
        "screenshot_repeats_equal": all(v["equal"] for v in shots.values()) if args.repeats > 1 else None,
        "screenshots": len(shots),
        "reimport": {k: v.get("exit", v) if isinstance(v, dict) and "exit" in v else
                     {n: e["exit"] for n, e in v.items()} for k, v in summary.get("reimport", {}).items()},
        "source_free_shots": len(summary.get("source_free", {}).get("shots", {})),
        "play_repeats_equal": {k: v["repeat_equal"] for k, v in summary.get("plays", {}).items()},
        "play_frames": {k: v["count"] for k, v in summary.get("plays", {}).items()},
        "budget": summary["budget"]}))


def png_size(path):
    """(width, height) from a PNG header."""
    with open(path, "rb") as f:
        head = f.read(24)
    return struct.unpack(">II", head[16:24])


if __name__ == "__main__":
    main()
