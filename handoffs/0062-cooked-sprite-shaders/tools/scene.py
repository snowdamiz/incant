"""Scene descriptions for handoff 0062. Pure data; no I/O beyond the caller.

Units: positions/sizes are authored in logical pixels (lp) of the pixel-perfect
camera, 1 lp = 1/16 m, then converted to metres. The pixel-perfect camera shows
320x180 lp at 3x (960x540), so 1 lp = 3 output pixels and 1/6 lp = half a pixel.

Three boards, each its own project:
  A  sampling/filtering, atlas edges, transparency, reflection/rotation,
     nine-slice, painter order and depth against an opaque emissive mesh
  W  wide horizontal/vertical strips (4096 texels) with trailing-end controls
  B  tilemaps (plain and rotated+mirrored at 1.5x) and flipbooks with static
     first/final-frame controls

Controls placed in the same scene let CPU analysis compare crops that must be
pixel-identical within one capture, independently of the display curve.
"""

import math

PPU = 16.0
CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
HALF = 1.0 / 6.0  # half an output pixel at the pixel-perfect 3x scale, in lp
FINAL_TICK = 48


def ulid(prefix, n):
    digits = ""
    for _ in range(26 - len(prefix)):
        digits = CROCKFORD[n % 32] + digits
        n //= 32
    return prefix + digits


def ulid_hex(text):
    value = 0
    for ch in text:
        value = value * 32 + CROCKFORD.index(ch)
    return f"{value:032x}"


def m(lp):
    return lp / PPU


def quat_z(degrees):
    h = math.radians(degrees) / 2
    return [0.0, 0.0, math.sin(h), math.cos(h)]


def quat_y(degrees):
    h = math.radians(degrees) / 2
    return [0.0, math.sin(h), 0.0, math.cos(h)]


class Board:
    def __init__(self, key, prefix):
        self.key, self.prefix = key, prefix
        self.items = []
        self.by_name = {}

    def add(self, name, kind, *, at=(0.0, 0.0), z=0.0, rotation=None, scale=(1.0, 1.0), parent=None, **data):
        assert name not in self.by_name, name
        item = {"name": name, "kind": kind, "slot": len(self.items) + 1, "at": [float(at[0]), float(at[1])],
                "z": float(z), "rotation": rotation or [0.0, 0.0, 0.0, 1.0], "scale": [float(scale[0]),
                                                                                        float(scale[1])],
                "parent": parent, **data}
        self.items.append(item)
        self.by_name[name] = item
        return item

    def sprite(self, name, texture, region, size, *, sampling="nearest", flip=(False, False), tint=(1, 1, 1, 1),
               layer=0, order=0, mode=None, **kw):
        return self.add(name, "sprite", texture=texture, region=region, size=[float(size[0]), float(size[1])],
                        sampling=sampling, flip=list(flip), tint=[float(c) for c in tint], layer=layer,
                        order=order, mode=mode or {"kind": "simple"}, **kw)

    def id(self, name):
        return ulid(self.prefix, self.by_name[name]["slot"])


def camera_pp():
    return {"kind": "pixel_perfect", "pixels_per_unit": PPU, "reference_resolution": [320, 180]}


def board_a():
    b = Board("A", "01JA62AE00")
    b.add("camera-pp", "camera", z=10.0, projection=camera_pp())
    b.add("camera-ortho", "camera", at=(-4.3137 * PPU, 1.9213 * PPU), z=10.0,
          projection={"kind": "orthographic", "vertical_size": 6.17})
    b.add("camera-persp", "camera", z=12.0, rotation=quat_y(5.0), projection={"kind": "perspective"})
    f = ("frame", "checker")
    # Row 1: sampling and filtering (y = 70 lp).
    y = 70.0
    b.sprite("A1-nearest-1x", "atlas", f, (8, 8), at=(-148, y))
    b.sprite("A2-nearest-2x", "atlas", f, (16, 16), at=(-128, y))
    b.sprite("A3-nearest-2.5x", "atlas", f, (20, 20), at=(-102, y))
    b.sprite("A4-linear-4x", "atlas", f, (32, 32), at=(-68, y), sampling="linear")
    b.sprite("A5-linear-0.5x", "atlas", f, (4, 4), at=(-44, y), sampling="linear")
    b.sprite("A6-nearest-2x-tie", "atlas", f, (16, 16), at=(-28 + HALF, y - HALF))
    b.sprite("A7-nearest-2x-tie-flipped", "atlas", f, (16, 16), at=(-8 + HALF, y - HALF), flip=(True, True))
    b.sprite("A8-nearest-3x-rot90", "atlas", f, (24, 24), at=(16, y), rotation=quat_z(90))
    b.sprite("A9-nearest-3x-rot30", "atlas", f, (24, 24), at=(48, y), rotation=quat_z(30))
    b.sprite("A10-linear-3x-rot30", "atlas", f, (24, 24), at=(84, y), rotation=quat_z(30), sampling="linear")
    b.sprite("A11-nearest-3x-yaw60", "atlas", f, (24, 24), at=(112, y), rotation=quat_y(60))
    b.sprite("A12-linear-3x-yaw60", "atlas", f, (24, 24), at=(136, y), rotation=quat_y(60), sampling="linear")
    # Row 2: atlas edges, plain-sheet neighbours, premultiplied transparency, tint (y = 30 lp).
    y = 30.0
    ring = ("frame", "ring")
    b.sprite("B1-ring-nearest-3x", "atlas", ring, (30, 30), at=(-140, y))
    b.sprite("B2-ring-linear-3x", "atlas", ring, (30, 30), at=(-104, y), sampling="linear")
    b.sprite("B3-sheet-middle-linear-2x", "sheet", ("pixels", "middle"), (32, 32), at=(-68, y),
             sampling="linear")
    b.sprite("B4-sheet-middle-nearest-2x", "sheet", ("pixels", "middle"), (32, 32), at=(-30, y))
    b.sprite("B5-sheet-red-edge-linear", "sheet", ("pixels", "red"), (16, 16), at=(-2, y), sampling="linear")
    b.sprite("B6-backdrop", "atlas", f, (32, 16), at=(36, y), layer=-1)
    b.sprite("B6-alpha-linear-2x", "atlas", ("frame", "alpha"), (32, 16), at=(36, y), sampling="linear")
    b.sprite("B7-backdrop", "atlas", f, (32, 16), at=(76, y), layer=-1)
    b.sprite("B7-alpha-nearest-2x", "atlas", ("frame", "alpha"), (32, 16), at=(76, y))
    b.sprite("B8-tint-color", "atlas", f, (16, 16), at=(106, y), tint=(1.0, 0.6, 0.3, 1.0))
    b.sprite("B9-backdrop-ring", "atlas", ring, (30, 30), at=(136, y), layer=-1)
    b.sprite("B9-tint-alpha-half", "atlas", f, (16, 16), at=(136, y), tint=(1.0, 1.0, 1.0, 0.5))
    # Row 3: reflection and rotation (y = -8 lp).
    y = -8.0
    arrow = ("frame", "arrow")
    b.sprite("C1-arrow", "atlas", arrow, (24, 24), at=(-144, y))
    b.sprite("C2-arrow-flip-x", "atlas", arrow, (24, 24), at=(-116, y), flip=(True, False))
    b.sprite("C3-arrow-flip-y", "atlas", arrow, (24, 24), at=(-88, y), flip=(False, True))
    b.sprite("C4-arrow-flip-xy", "atlas", arrow, (24, 24), at=(-60, y), flip=(True, True))
    b.add("C5-mirror-parent", "group", at=(-32, y), scale=(-1.0, 1.0))
    b.sprite("C5-arrow-mirrored", "atlas", arrow, (24, 24), parent="C5-mirror-parent")
    b.add("C6-mirror-parent", "group", at=(-4, y), scale=(-1.0, 1.0))
    b.sprite("C6-arrow-mirrored-flip-x", "atlas", arrow, (24, 24), parent="C6-mirror-parent", flip=(True, False))
    b.sprite("C7-arrow-rot90", "atlas", arrow, (24, 24), at=(24, y), rotation=quat_z(90))
    b.sprite("C8-arrow-rot180", "atlas", arrow, (24, 24), at=(52, y), rotation=[0.0, 0.0, 1.0, 0.0])
    b.sprite("C9-arrow-rot30-nearest", "atlas", arrow, (24, 24), at=(82, y), rotation=quat_z(30))
    b.sprite("C10-arrow-rot45-linear", "atlas", arrow, (24, 24), at=(114, y), rotation=quat_z(45),
             sampling="linear")
    b.add("C11-shear-parent", "group", at=(142, y), scale=(1.25, 0.75))
    b.sprite("C11-arrow-sheared", "atlas", arrow, (24, 24), parent="C11-shear-parent", rotation=quat_z(20))
    # Row 4: nine-slice, painter order and depth (y = -50 lp).
    y = -50.0
    nine = {"kind": "nine_slice", "border": [3, 3, 3, 3], "pixels_per_unit": PPU}
    b.sprite("D1-nine-nearest", "atlas", ("frame", "nine"), (40, 16), at=(-136, y), mode=nine)
    b.sprite("D2-nine-linear", "atlas", ("frame", "nine"), (40, 16), at=(-90, y), mode=nine, sampling="linear")
    b.sprite("D3-nine-compressed", "atlas", ("frame", "nine"), (4, 12), at=(-62, y), mode=nine)
    b.sprite("E1-order-blue-l0-o0", "sheet", ("pixels", "blue"), (16, 16), at=(-34, y - 6))
    b.sprite("E1-order-red-l1", "sheet", ("pixels", "red"), (16, 16), at=(-46, y + 6), layer=1)
    b.sprite("E1-order-green-l0-o5", "atlas", ("frame", "t0"), (16, 16), at=(-40, y), order=5)
    b.sprite("E2-tie-first", "atlas", ("frame", "t1"), (12, 12), at=(-14, y + 4))
    b.sprite("E2-tie-second", "atlas", ("frame", "t3"), (12, 12), at=(-8, y - 2))
    b.add("E3-occluder", "mesh", at=(52, y), z=0.5)
    b.sprite("E3-behind", "atlas", f, (24, 24), at=(32, y), z=0.0)
    b.sprite("E3-front", "atlas", arrow, (24, 24), at=(66, y), z=1.0)
    b.sprite("E3-piercing-yaw70", "atlas", ring, (30, 30), at=(52, y), z=0.5, rotation=quat_y(70))
    b.sprite("E3-translucent-front", "atlas", f, (16, 16), at=(38, y + 12), z=1.0, tint=(1.0, 1.0, 1.0, 0.5))
    b.sprite("E4-linear-behind-mesh", "atlas", ("frame", "t2"), (16, 16), at=(84, y + 8), z=0.0,
             sampling="linear")
    return b


def board_w():
    b = Board("W", "01JA62WE00")
    b.add("camera-pp", "camera", z=10.0, projection=camera_pp())
    b.add("camera-ortho", "camera", at=(0.0137 * PPU, -0.0219 * PPU), z=10.0,
          projection={"kind": "orthographic", "vertical_size": 7.77})
    whole_h, whole_v = ("pixels", [0, 0, 4096, 1]), ("pixels", [0, 0, 1, 4096])
    tail_h, tail_v = ("pixels", [4080, 0, 16, 1]), ("pixels", [0, 4080, 1, 16])
    long, thick = 4096 * 3.0, 6.0
    # Horizontal strip extending left, trailing texel 4095 at its right end x = -8.
    b.sprite("W1-h-trailing-right", "strip-h", whole_h, (long, thick), at=(-8 - long / 2, 60))
    b.sprite("W1c-h-tail-control", "strip-h", tail_h, (48, thick), at=(-8 - 24, 50))
    # Flipped horizontal strip extending right, texel 4095 at its left end x = 8.
    b.sprite("W2-h-flipped-trailing-left", "strip-h", whole_h, (long, thick), at=(8 + long / 2, -60),
             flip=(True, False))
    b.sprite("W2c-h-flipped-tail-control", "strip-h", tail_h, (48, thick), at=(8 + 24, -50), flip=(True, False))
    # Vertical strip extending up, last row 4095 at its bottom end y = 8.
    b.sprite("W3-v-trailing-bottom", "strip-v", whole_v, (thick, long), at=(60, 8 + long / 2))
    b.sprite("W3c-v-tail-control", "strip-v", tail_v, (thick, 48), at=(50, 8 + 24))
    # Flipped vertical strip extending down, row 4095 at its top end y = -8.
    b.sprite("W4-v-flipped-trailing-top", "strip-v", whole_v, (thick, long), at=(-60, -8 - long / 2),
             flip=(False, True))
    b.sprite("W4c-v-flipped-tail-control", "strip-v", tail_v, (thick, 48), at=(-50, -8 - 24), flip=(False, True))
    # Linear horizontal strip; the control's first texel clamps differently by design.
    b.sprite("W5-h-linear-trailing-right", "strip-h", whole_h, (long, thick), at=(-8 - long / 2, 30),
             sampling="linear")
    b.sprite("W5c-h-linear-tail-control", "strip-h", tail_h, (48, thick), at=(-8 - 24, 20), sampling="linear")
    # Half-pixel offset magnified strip: trailing end at a pixel-centre tie.
    b.sprite("W6-h-trailing-tie", "strip-h", whole_h, (long, thick), at=(110 + HALF - long / 2, -20 - HALF))
    b.sprite("W6c-h-tail-tie-control", "strip-h", tail_h, (48, thick), at=(110 + HALF - 24, -30 - HALF))
    return b


TILE_ROWS_BACK = ["t0 t1 t0 t1 t0 t1 t0 t1", "t1 t0 t1 t0 t1 t0 t1 t0", "t0 t1 t0 t1 t0 t1 t0 t1",
                  "t1 t0 t1 t0 t1 t0 t1 t0"]
TILE_ROWS_FRONT = ["t2 t2 . . t3 . . t2", ". t3 . t2 t2 . t3 .", ". . t3 . . t2 . .", "t3 . . . t2 t3 . t3"]


def cells(rows_top_down):
    """Bottom-to-top row-major cell list from rows written top-down."""
    out = []
    for row in reversed(rows_top_down):
        out.extend(None if c == "." else c for c in row.split())
    return out


def board_b():
    b = Board("B", "01JA62BE00")
    b.add("camera-pp", "camera", z=10.0, projection=camera_pp())
    b.add("camera-ortho", "camera", at=(-4.1 * PPU, 2.2 * PPU), z=10.0,
          projection={"kind": "orthographic", "vertical_size": 5.11})
    layers = [{"name": "back", "rows": TILE_ROWS_BACK, "layer": -1, "order": 0},
              {"name": "front", "rows": TILE_ROWS_FRONT, "layer": 2, "order": 0}]
    b.add("T1-map", "map", at=(-150, 20), size=[8, 4], cell=[8.0, 8.0], layers=layers)
    # Raw cell sprites reproducing T1 50 lp lower (cell (i, j) centre at origin + 8i + 4, 8j + 4).
    for layer in layers:
        for j, row in enumerate(reversed(layer["rows"])):
            for i, name in enumerate(row.split()):
                if name != ".":
                    b.sprite(f"T1c-{layer['name']}-{i}-{j}", "atlas", ("frame", name), (8, 8),
                             at=(-150 + 8 * i + 4, -30 + 8 * j + 4), layer=layer["layer"], order=layer["order"])
    b.add("T2-parent", "group", at=(-20, 88), rotation=quat_z(90), scale=(-1.0, 1.0))
    b.add("T2-map", "map", parent="T2-parent", size=[8, 4], cell=[12.0, 12.0], layers=layers)
    frames = ["f0", "f1", "f2", "f3", "f4"]

    def anim(name, at, seq, rate, *, looping=True, playing=True, texture="atlas", **kw):
        first = seq[0]
        region = ("frame", first) if texture == "atlas" else ("pixels", first)
        item = b.sprite(name, texture, region, (24, 24), at=at, **kw)
        item.update(kind="anim", frames=list(seq), rate=list(rate), looping=looping, playing=playing)
        return item

    def control(name, at, frame_name, texture="atlas", **kw):
        region = ("frame", frame_name) if texture == "atlas" else ("pixels", frame_name)
        return b.sprite(name, texture, region, (24, 24), at=at, **kw)

    anim("F1-loop-nearest-5fps", (20, 60), frames, (5, 1))
    control("F1c-first", (20, 30), "f0")
    control("F1c-final", (20, 0), "f4")
    anim("F2-loop-linear-6fps", (52, 60), frames, (6, 1), sampling="linear")
    control("F2c-first", (52, 30), "f0", sampling="linear")
    control("F2c-final", (52, 0), "f4", sampling="linear")
    anim("F3-once-flip-x-10fps", (84, 60), frames, (10, 1), looping=False, flip=(True, False))
    control("F3c-first", (84, 30), "f0", flip=(True, False))
    control("F3c-final", (84, 0), "f4", flip=(True, False))
    anim("F4-reversed-5fps", (116, 60), list(reversed(frames)), (5, 1))
    control("F4c-first", (116, 30), "f4")
    control("F4c-final", (116, 0), "f0")
    anim("F5-sheet-regions-2.5fps", (148, 60), ["red", "middle", "blue"], (5, 2), texture="sheet")
    control("F5c-first", (148, 30), "red", texture="sheet")
    control("F5c-final", (148, 0), "blue", texture="sheet")
    anim("F6-paused", (20, -40), frames, (5, 1), playing=False)
    control("F6c-held", (52, -40), "f0")
    anim("F7-rot90-5fps", (84, -40), frames, (5, 1), rotation=quat_z(90))
    control("F7c-first", (116, -40), "f0", rotation=quat_z(90))
    control("F7c-final", (148, -40), "f4", rotation=quat_z(90))
    anim("F8-once-linear-rot30-10fps", (20, -75), frames, (10, 1), looping=False, sampling="linear",
         rotation=quat_z(30))
    control("F8c-final", (52, -75), "f4", sampling="linear", rotation=quat_z(30))
    return b


BOARDS = {"A": board_a, "W": board_w, "B": board_b}


# ------------------------------------------------------------------ expansion

def entity_values(board, ids, manifest):
    """Entity JSON for create_entity commands. ids: texture key -> asset ID; 'occluder' -> model ID."""

    def region(texture, value):
        kind, name = value
        if kind == "frame":
            return {"frame": manifest["frames"][name]}
        if isinstance(name, str):
            return list(manifest["sheet_regions"][name])
        return list(name)

    out = []
    for item in board.items:
        transform = {"translation": [m(item["at"][0]), m(item["at"][1]), item["z"]],
                     "rotation": item["rotation"], "scale": [item["scale"][0], item["scale"][1], 1.0]}
        components = {"Transform": transform}
        kind = item["kind"]
        if kind in ("sprite", "anim"):
            components["SpriteRenderer"] = {
                "texture": ids[item["texture"]], "region": region(item["texture"], item["region"]),
                "size": [m(item["size"][0]), m(item["size"][1])], "pivot": [0.5, 0.5], "tint": item["tint"],
                "flip_x": item["flip"][0], "flip_y": item["flip"][1], "layer": item["layer"],
                "order": item["order"], "sampling": item["sampling"],
                "mode": item["mode"]}
        if kind == "anim":
            frames = [region(item["texture"], ("frame" if item["texture"] == "atlas" else "pixels", f))
                      for f in item["frames"]]
            components["SpriteFlipbook"] = {"frames": frames, "rate": {"numerator": item["rate"][0],
                                                                       "denominator": item["rate"][1]},
                                            "looping": item["looping"], "playing": item["playing"]}
        elif kind == "camera":
            components["Camera"] = {"fov_degrees": 60.0, "near": 0.1, "far": 100.0,
                                    "projection": item["projection"]}
        elif kind == "mesh":
            components["MeshRenderer"] = {"mesh": ids["occluder"], "materials": [], "cast_shadows": False}
        elif kind == "map":
            components["Tilemap"] = {
                "tileset": ids["atlas"], "size": item["size"],
                "cell_size": [m(item["cell"][0]), m(item["cell"][1])],
                "layers": [{"id": ulid("01JA62MY00", 1 + index + 10 * item["slot"]),
                            "cells": [None if c is None else manifest["frames"][c] for c in cells(layer["rows"])],
                            "layer": layer["layer"], "order": layer["order"], "solid": False}
                           for index, layer in enumerate(item["layers"])]}
        out.append({"id": board.id(item["name"]), "name": item["name"],
                    "parent": board.id(item["parent"]) if item["parent"] else None, "components": components})
    return out


def textures_used(board):
    keys = {item["texture"] for item in board.items if item["kind"] in ("sprite", "anim")}
    if any(item["kind"] == "map" for item in board.items):
        keys.add("atlas")
    return sorted(keys)


def describe(board):
    return {"key": board.key, "items": board.items,
            "ids": {item["name"]: board.id(item["name"]) for item in board.items},
            "hex": {item["name"]: ulid_hex(board.id(item["name"])) for item in board.items},
            "units": "lp = 1/16 m; pixel-perfect camera 320x180 lp at 3x", "final_tick": FINAL_TICK}
