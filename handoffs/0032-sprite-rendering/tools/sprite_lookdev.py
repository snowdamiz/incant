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

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py OUT_DIR [--binary pre-ties]
OUT_DIR must not exist. The default binary is the active corrected build; pre-ties
selects the preserved round-one build for before/after comparisons.
"""

import argparse
import hashlib
import json
import math
import shutil
import struct
import subprocess
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import sprite_art  # noqa: E402

ROOT = TOOLS.parents[2]
# The active build is the corrected one; the pre-ties build is kept for before/after.
BINARIES = {"corrected": ("incant_headless", "binary.json"),
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

SHOTS = ([(f"pp-{b}", (320, 180)) for b in BOARDS] + [(f"pp-{b}", (960, 540)) for b in BOARDS]
         + [(f"ortho3-{b}", (960, 540)) for b in BOARDS] + [(f"orthof-{b}", (960, 540)) for b in BOARDS]
         + [(f"roll7-{b}", (960, 540)) for b in ("B", "D", "E")]
         + [("pp-A-odd", (319, 179)), ("pp-A-sub", (320, 180))]
         + [("pp-F", (320, 180)), ("pp-F", (960, 540)), ("orthof-F", (960, 540)), ("pp-G", (1024, 640))])


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


def build(out, art):
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
                       ("tone16", "tone16.png")):
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
                       ("tone16", regions["tone16"])):
        for key, rect in table.items():
            s.regions[(tex, key)] = rect
    for builder in (board_a, board_b, board_c, board_d, board_e, board_f, board_g):
        builder(s)
    s.entity("C:occluder", {"Transform": transform((0, 0, 0)),
                            "MeshRenderer": {"mesh": occluder_asset, "materials": [], "cast_shadows": False}})
    cameras = {}
    for name, spec in CAMERAS.items():
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
    for key, (ux, uy) in DIRECTIONS.items():
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
        "cameras": {k: dict(v, id=cameras[k]) for k, v in CAMERAS.items()},
        "live": {"id": live, "wisp": wisp_id, "wisp_t0": wisp_t0, "step": STEP, "ticks": LIVE_TICKS,
                 "directions": DIRECTIONS},
    }
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, journal, scene, scripts


def screenshot(project, path, camera, size, check=True):
    w, h = size
    return engine("screenshot", project, path, "--camera", camera, "--width", w, "--height", h, check=check)


def shoot_pair(project, folder, name, camera, size, record):
    w, h = size
    hashes, info = [], None
    for repeat in (1, 2):
        png = folder / f"{name}-{w}x{h}-r{repeat}.png"
        result = screenshot(project, png, camera, size)
        hashes.append(sha(png))
        if info is None:
            reply = json.loads(result.stdout)
            info = {"adapter": reply.get("adapter"), "geometry": reply.get("geometry")}
    record[f"{name}-{w}x{h}"] = {"r1": hashes[0], "r2": hashes[1], "equal": hashes[0] == hashes[1], **info}


def authored(project, journal):
    return {p.name: sha(p) for p in (project, journal)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--binary", choices=sorted(BINARIES), default="corrected",
                        help="corrected (active) or pre-ties (round-one build) from artifacts/tools")
    args = parser.parse_args()
    global CLI, BINARY_JSON
    CLI = ROOT / "artifacts/tools" / BINARIES[args.binary][0]
    BINARY_JSON = ROOT / "artifacts/tools" / BINARIES[args.binary][1]
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    frames = 1 + LIVE_TICKS
    if frames > MAX_FRAMES or frames * LOGICAL[0] * LOGICAL[1] * 4 > MAX_RAW:
        raise SystemExit("live capture exceeds the budget")
    start = verify_binary()
    out.mkdir(parents=True)
    run(sys.executable, "-I", TOOLS / "sprite_art.py", out / "art")
    art = out / "art"
    project, journal, scene, scripts = build(out, art)
    project_dir = project.parent
    cams = {k: v["id"] for k, v in scene["cameras"].items()}
    metadata = json.loads(BINARY_JSON.read_text(encoding="utf-8"))
    summary = {"binary": args.binary, "binary_file": CLI.name, "binary_source_commit": metadata["source_commit"],
               "binary_sha256_start": start}

    # 1. Static frames, each rendered twice.
    before = authored(project, journal)
    shots_dir = out / "shots"
    shots_dir.mkdir()
    shots = {}
    for name, size in SHOTS:
        shoot_pair(project, shots_dir, name, cams[name], size, shots)
    summary["shots"] = shots
    summary["authored_unchanged_by_rendering"] = before == authored(project, journal)

    # 2. Texture reimport through the public import command.
    re_dir = out / "reimport"
    re_dir.mkdir()
    steps = {}

    def reimport(label, source_name, target_name, usage=None, expect_error=False):
        prior = authored(project, journal)
        shutil.copyfile(art / source_name, project_dir / "art" / target_name)
        args = ["import", project, f"art/{target_name}"] + (["--texture-usage", usage] if usage else [])
        result = engine(*args, check=not expect_error)
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
    reimport("8-sheet-restore", "sheet.png", "sheet.png")
    shoot("8-sheet-restore", "pp-E")
    summary["reimport"] = steps

    # 3. Source-free: delete every source image and model, then render again.
    removed = sorted(str(p.relative_to(project_dir)) for p in project_dir.glob("art/*") if p.is_file())
    removed += sorted(str(p.relative_to(project_dir)) for p in project_dir.glob("models/*") if p.is_file())
    for rel in removed:
        (project_dir / rel).unlink()
    free_dir = out / "source-free"
    free_dir.mkdir()
    free = {}
    for name, size in ([(f"pp-{b}", (320, 180)) for b in BOARDS + ["F"]] + [("pp-G", (1024, 640)), ("ortho3-E", (960, 540)),
                                                                     ("orthof-D", (960, 540)),
                                                                     ("roll7-B", (960, 540))]):
        png = free_dir / f"{name}-{size[0]}x{size[1]}.png"
        screenshot(project, png, cams[name], size)
        free[f"{name}-{size[0]}x{size[1]}"] = sha(png)
    summary["source_free"] = {"removed": removed, "shots": free}

    # 4. Live flipbook + pixel-perfect follow, source-free, twice per direction.
    plays = {}
    live_before = authored(project, journal)
    for key in DIRECTIONS:
        runs = []
        for repeat in (1, 2):
            target = out / f"live-{key}-r{repeat}"
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
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({
        "binary_unchanged": summary["binary_unchanged_every_call"], "engine_calls": CALLS["count"],
        "authored_unchanged": summary["authored_unchanged_by_rendering"] and summary["authored_unchanged_by_play"],
        "screenshot_repeats_equal": all(v["equal"] for v in shots.values()), "screenshots": len(shots),
        "reimport": {k: v.get("exit", v) if isinstance(v, dict) and "exit" in v else
                     {n: e["exit"] for n, e in v.items()} for k, v in steps.items()},
        "source_free_shots": len(free),
        "play_repeats_equal": {k: v["repeat_equal"] for k, v in plays.items()},
        "play_frames": {k: v["count"] for k, v in plays.items()}}))


if __name__ == "__main__":
    main()
