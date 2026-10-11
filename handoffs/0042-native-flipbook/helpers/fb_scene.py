"""Scene description, contract model and control expansion for handoff 0042.

One scene holds five boards, viewed by a 16 px/m pixel-perfect camera at 640x360
(1x) and a 2.5x orthographic camera with a sub-pixel offset:

  A  clock and sampling: nearest, linear, flips, pixel-region (sheet) frames,
     a reversed frame list, tint, 1x/2x/3x magnification and nine-slice
     (nearest, linear, mirrored, ppu 8 and horizontally compressed borders)
  B  inherited transforms: rotated, reflected and sheared parents, a two-level
     chain with scale 2, a moving parent with a static sibling, a moving sprite
  C  typed controls: pause/play phase, seek while paused, restart, once hold
     and completion, initially paused, same-tick ordering, seek on completed
  D  painter order with a retained two-layer tilemap, static and translucent
     sprites, equal (layer, order) ID ties, a sprite under the map seen through
     holes and a moving paused/resumed sprite passing under a static sprite
  E  clock arithmetic: rates above and below 60 Hz, 1000 fps, single-frame and
     a once sequence that completes within two ticks

Every animated SpriteRenderer's otherwise-unused static region deliberately
names a different frame (the t2 stone tile, or the sheet's magenta guard), so
a renderer that showed the static region would be visible at tick 0.

`expected_states` is an independent closed-form model written from the brief:
a playing sequence's cursor is base + numerator*(ticks since the segment began)
over denominator*tick_rate frames; controls start new segments. It does not step
the incremental clock the engine uses. `position` replicates the runtime's
documented per-tick f64 integration (translation += velocity * (1/60)) so a
static control can place a moving entity at exactly the same world position.

`animated_entities` emits the animated project. `control_entities(t)` replaces
every flipbook with a static SpriteRenderer showing the model's frame at tick t,
places moving entities at their exact tick-t positions without Velocity, and
keeps every entity ID, so painter-order ties are unchanged.
"""

import json
import math
from fractions import Fraction
from pathlib import Path

PPU = 16
VIEW = (640, 360)
TICK_RATE = 60
EYE_Z, NEAR, FAR, FOV = 10.0, 0.1, 50.0, 50.0
ORTHO_CENTRE = (-7.0 + 0.1037, 6.5 - 0.0491)  # +4.148, -1.964 output px at 40 px/m
ORTHO_VERTICAL = 9.0
RATE_A = (50, 7)  # 50/7 fps: 8.4 ticks per frame; tick 42 is an exact boundary
ATLAS_FRAMES = ["f0", "f1", "f2", "f3", "f4"]
NINE = {"kind": "nine_slice", "border": [3, 2, 2, 3], "pixels_per_unit": 16.0}

# Main animated capture clock: both sides of every board-A boundary (9, 17, 26,
# 34, 42, 51), every control edge, high-rate ticks 1-3, the 1/7 fps boundary at
# 420 and 50 loops later (2100 = 50 x 42) to show there is no drift.
MAIN_TICKS = [0, 1, 2, 3, 8, 9, 10, 12, 16, 17, 20, 24, 25, 26, 33, 34, 41, 42, 50, 51, 59, 60, 68, 69,
              419, 420, 2099, 2100]
MOMENTS = [0, 9, 17, 25, 34, 42, 50, 60, 69, 420, 2100]  # full-frame static controls
ORTHO_TICKS = [0, 9, 17, 42]
SUBSET_TICKS = [0, 9, 25, 42, 60, 69, 420, 2100]


def crockford(prefix, n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    digits = ""
    for _ in range(16):
        digits = alphabet[n % 32] + digits
        n //= 32
    value = prefix + digits
    assert len(value) == 26 and value[0] in "01234567" and all(c in alphabet for c in value), value
    return value


def ulid_hex(text):
    """32 hexadecimal digits of the ULID's 128-bit big-endian value (cooked stable ID)."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    value = 0
    for ch in text:
        value = value * 32 + alphabet.index(ch)
    assert value < 1 << 128
    return f"{value:032x}"


def entity_id(n):
    return crockford("01JA42EN00", n)


def layer_id(n):
    return crockford("01JA42KY00", n)


def scene_id(n):
    return crockford("01JA42SC00", n)


def quat_z(degrees):
    h = math.radians(degrees) / 2
    return [0.0, 0.0, math.sin(h), math.cos(h)]


class Scene:
    def __init__(self):
        self.items = []
        self.by_name = {}

    def add(self, name, kind, *, parent=None, t=(0.0, 0.0), r=0.0, s=(1.0, 1.0), v=None, **data):
        assert name not in self.by_name, name
        item = {"name": name, "kind": kind, "slot": len(self.items) + 1, "parent": parent,
                "t": [float(t[0]), float(t[1])], "r": float(r), "s": [float(s[0]), float(s[1])],
                "v": None if v is None else [float(v[0]), float(v[1])], **data}
        self.items.append(item)
        self.by_name[name] = item
        return item

    def group(self, name, **kw):
        return self.add(name, "group", **kw)

    def sprite(self, name, texture, region, size, *, layer=0, order=0, tint=(1.0, 1.0, 1.0, 1.0),
               sampling="nearest", flip=(False, False), mode=None, pivot=(0.5, 0.5), **kw):
        return self.add(name, "sprite", texture=texture, region=region, size=[float(size[0]), float(size[1])],
                        pivot=[float(pivot[0]), float(pivot[1])], tint=[float(c) for c in tint],
                        layer=layer, order=order, sampling=sampling, flip=list(flip),
                        mode=mode or {"kind": "simple"}, **kw)

    def anim(self, name, frames, size, *, texture="atlas", rate=RATE_A, looping=True, playing=True,
             controls=(), static=None, **kw):
        static = static or (("frame", "t2") if texture == "atlas" else ("pixels", [0, 0, 4, 4]))
        item = self.sprite(name, texture, static, size, **kw)
        item.update(kind="anim", frames=list(frames), rate=list(rate), looping=looping, playing=playing,
                    controls=[list(c) for c in controls])
        return item


def build():
    sc = Scene()
    plates = {"A": ((-10.0, 7.0), (19.5, 8.25)), "B": ((10.0, 7.0), (19.5, 8.25)),
              "C": ((-10.0, -0.75), (19.5, 6.5)), "E": ((-10.0, -7.75), (19.5, 6.5)),
              "D": ((10.0, -4.25), (19.5, 13.5))}
    for board, (centre, size) in plates.items():
        sc.sprite(f"plate-{board}", "plates", ("plate", board), size, t=centre, layer=-100)
    seq = ATLAS_FRAMES

    # Board A: clock and sampling (all at RATE_A, looping, never controlled).
    row1 = ["A1-nearest", "A2-linear", "A3-flip-x", "A4-flip-y", "A5-flip-xy", "A6-sheet", "A7-reversed"]
    for i, name in enumerate(row1):
        kw = {"t": (-18.0 + 2.5 * i, 9.5)}
        if name == "A2-linear":
            kw["sampling"] = "linear"
        if name.startswith("A3"):
            kw["flip"] = (True, False)
        if name.startswith("A4"):
            kw["flip"] = (False, True)
        if name.startswith("A5"):
            kw["flip"] = (True, True)
        frames = list(reversed(seq)) if name == "A7-reversed" else seq
        sc.anim(name, frames, (1.5, 1.5), texture="sheet" if name == "A6-sheet" else "atlas", **kw)
    sc.anim("A8-tint", seq, (1.5, 1.5), t=(-18.0, 6.75), tint=(1.0, 0.7, 0.45, 1.0))
    sc.anim("A9-one-to-one", seq, (0.75, 0.75), t=(-15.5, 6.75))
    sc.anim("A10-nearest-3x", seq, (2.25, 2.25), t=(-12.75, 6.75))
    sc.anim("A15-linear-3x", seq, (2.25, 2.25), t=(-9.5, 6.75), sampling="linear")
    sc.anim("A16-sheet-linear", seq, (1.5, 1.5), t=(-6.5, 6.75), texture="sheet", sampling="linear")
    sc.anim("A17-wide", seq, (3.0, 1.5), t=(-2.5, 6.75))
    sc.anim("A11-nine-nearest", seq, (3.0, 2.0), t=(-17.5, 4.25), mode=NINE)
    sc.anim("A12-nine-linear", seq, (3.0, 2.0), t=(-13.75, 4.25), mode=NINE, sampling="linear")
    sc.anim("A13-nine-flip-x", seq, (3.0, 2.0), t=(-10.0, 4.25), mode=NINE, flip=(True, False))
    sc.anim("A14-nine-ppu8", seq, (3.5, 2.25), t=(-6.0, 4.25), mode=dict(NINE, pixels_per_unit=8.0))
    sc.anim("A18-nine-compressed", seq, (0.25, 1.5), t=(-2.75, 4.25), mode=NINE)

    # Board B: inherited transforms and motion.
    sc.group("B1-parent", t=(2.5, 8.75), r=30.0)
    sc.anim("B1-rotated", seq, (1.5, 1.5), parent="B1-parent")
    sc.group("B2-parent", t=(6.0, 8.75), s=(-1.0, 1.0))
    sc.anim("B2-reflect-x-flip-y", seq, (1.5, 1.5), parent="B2-parent", flip=(False, True))
    sc.group("B3-parent", t=(9.75, 8.75), s=(1.5, 0.75))
    sc.anim("B3-shear-linear", seq, (1.5, 1.5), parent="B3-parent", r=20.0, sampling="linear")
    sc.group("B6-parent", t=(14.0, 8.75), r=12.0)
    sc.anim("B6-nine-rotated", seq, (3.0, 2.0), parent="B6-parent", mode=NINE)
    sc.anim("B7-rotated-linear", seq, (1.5, 1.5), t=(18.0, 8.75), r=45.0, sampling="linear")
    sc.group("B4-moving-parent", t=(2.5, 4.75), v=(0.75, 0.0))
    sc.anim("B4-moving-child", seq, (1.5, 1.5), parent="B4-moving-parent")
    sc.sprite("B4-static-sibling", "badge", ("badge",), (1.0, 1.0), parent="B4-moving-parent", t=(2.0, 0.0))
    sc.group("B8-parent", t=(11.0, 4.75), s=(2.0, 2.0))
    sc.group("B8-inner", parent="B8-parent", t=(0.25, 0.0), r=-15.0)
    sc.anim("B8-chain", seq, (0.75, 0.75), parent="B8-inner")
    sc.anim("B5-moving", seq, (1.5, 1.5), t=(17.5, 4.75), v=(-0.5, 0.0))

    # Board C: typed controls, all RATE_A.
    controls = {
        "C1-pause-play": [(12, "pause"), (20, "play")],
        "C2-pause-seek-play-restart": [(10, "pause"), (16, "seek", 3), (25, "play"), (41, "restart")],
        "C3-once-hold-replay": [(60, "play")],
        "C4-initially-paused": [(33, "play")],
        "C5-seek-playing": [(17, "seek", 3)],
        "C6-restart-playing": [(33, "restart")],
        "C7-pause-then-restart": [(16, "pause"), (16, "restart")],
        "C8-restart-then-pause": [(16, "restart"), (16, "pause"), (50, "play")],
        "C9-once-seek-completed": [(50, "seek", 1), (59, "play")],
        "C10-reversed-seek": [(17, "seek", 1)],
        "C11-sheet-pause": [(9, "pause")],
    }
    for i, (name, ctl) in enumerate(controls.items()):
        x, y = (-18.0 + 3.0 * i, 0.75) if i < 6 else (-18.0 + 3.0 * (i - 6), -2.25)
        sc.anim(name, list(reversed(seq)) if name == "C10-reversed-seek" else seq, (1.5, 1.5), t=(x, y),
                texture="sheet" if name == "C11-sheet-pause" else "atlas",
                looping=name not in ("C3-once-hold-replay", "C9-once-seek-completed"),
                playing=name != "C4-initially-paused", controls=ctl)

    # Board E: clock arithmetic.
    clocks = [("E1-150fps", (150, 1), True, seq), ("E2-13-over-3", (13, 3), True, seq),
              ("E3-1-over-7", (1, 7), True, seq), ("E4-1000fps", (1000, 1), True, seq),
              ("E5-60fps", (60, 1), True, seq), ("E6-7-over-11", (7, 11), True, seq),
              ("E7-once-150fps", (150, 1), False, seq), ("E8-single-loop", (10, 1), True, ["f2"]),
              ("E9-single-once", (2, 1), False, ["f3"])]
    for i, (name, rate, looping, frames) in enumerate(clocks):
        sc.anim(name, frames, (1.0, 1.0), t=(-18.0 + 2.0 * i, -6.5), rate=rate, looping=looping)

    # Board D: painter order with a retained tilemap.
    sc.anim("D0-under-map", seq, (2.25, 2.25), t=(4.0, -6.0), layer=-1)
    sc.add("D-map", "map", t=(1.5, -10.5), size=[16, 7], cell=[1.0, 1.0], layers=map_layers())
    sc.sprite("D-static-low", "badge", ("badge",), (2.0, 2.0), t=(8.0, -5.25), order=1)
    sc.anim("D1-tie-middle", seq, (2.25, 2.25), t=(9.0, -5.75), order=1)
    sc.sprite("D-static-high", "badge", ("badge",), (1.5, 1.5), t=(10.25, -6.5), order=1)
    sc.anim("D2-layer1", seq, (2.25, 2.25), t=(13.5, -5.0), layer=1)
    sc.sprite("D-glass", "badge", ("badge",), (1.5, 1.5), t=(14.25, -4.5), layer=2, tint=(1.0, 1.0, 1.0, 0.5))
    sc.anim("D5-above-ground", seq, (1.5, 1.5), t=(16.0, -8.0))
    sc.sprite("D-static-top", "badge", ("badge",), (1.5, 1.5), t=(7.0, -8.25), layer=2)
    sc.anim("D3-moving-paused", seq, (2.25, 2.25), t=(3.5, -8.25), v=(3.0, 0.0), order=3,
            controls=[(25, "pause"), (50, "play")])
    sc.sprite("D-static-frame", "atlas", ("frame", "f2"), (1.5, 1.5), t=(18.5, -9.5), order=1)
    sc.anim("D6-linear-beside-static", seq, (1.5, 1.5), t=(18.5, -7.5), sampling="linear", order=1)
    sc.anim("D7-nearest-4x", seq, (3.0, 3.0), t=(4.5, -0.5))
    sc.anim("D8-linear-4x", seq, (3.0, 3.0), t=(9.0, -0.5), sampling="linear")
    sc.anim("D9-nine-linear-big", seq, (4.0, 3.0), t=(15.0, -0.5), mode=NINE, sampling="linear")

    sc.add("camera-pp", "camera", t=(0.0, 0.0),
           projection={"kind": "pixel_perfect", "pixels_per_unit": float(PPU), "reference_resolution": list(VIEW)})
    sc.add("camera-ortho", "camera", t=ORTHO_CENTRE,
           projection={"kind": "orthographic", "vertical_size": ORTHO_VERTICAL})
    return sc


def map_layers():
    ground = ["1111111111111111",
              "0000000000000000",
              "00..000000000.00",
              "00..0000..000000",
              "00..00000000..00",
              "0..0000000000000",
              "1111111111111111"]
    overlay = ["................",
               "..2.........2...",
               "................",
               ".....2....2.....",
               "................",
               "........2.....2.",
               "................"]
    return [{"id": layer_id(1), "rows": ground, "layer": 0, "order": 0, "label": "ground"},
            {"id": layer_id(2), "rows": overlay, "layer": 0, "order": 2, "label": "overlay"}]


def cells_bottom_up(rows):
    width = len(rows[0])
    assert all(len(r) == width for r in rows)
    names = {"0": "t0", "1": "t1", "2": "t2"}
    return [None if ch == "." else names[ch] for row in reversed(rows) for ch in row]


# ------------------------------------------------------------------- model --

def segments_model(item, ticks):
    """Closed-form playback per the brief. Returns {tick: (frame index, playing, completed)}.

    A segment is (start tick, cursor at start, playing). While playing, the cursor
    after k further ticks is start_cursor + numerator*k in units of 1/(den*rate)
    frames; frame = cursor // D. Looping wraps; a once sequence completes on the
    first tick whose cursor reaches n*D, holding frame n-1. Controls apply after
    their tick's step, in submitted order.
    """
    num, den = item["rate"]
    d = den * TICK_RATE
    n = len(item["frames"])
    looping = item["looping"]
    controls = {}
    for c in item["controls"]:
        controls.setdefault(c[0], []).append(c[1:])
    start, cursor0, playing, completed, frozen = 0, 0, item["playing"], False, 0

    def cursor_at(t):
        if completed:
            return (n - 1) * d
        if not playing:
            return frozen
        c = cursor0 + num * (t - start)
        return c

    def completion_tick():
        if looping or not playing or completed:
            return None
        need = n * d - cursor0
        return start + -(-need // num)

    out = {}
    events = sorted(set(controls) | set(ticks))
    for t in events:
        done = completion_tick()
        if done is not None and done <= t:
            completed, playing = True, False
        c = cursor_at(t)
        for action in controls.get(t, []):
            kind = action[0]
            if kind == "pause":
                if playing:
                    frozen, playing = c, False
            elif kind == "play":
                if completed:
                    completed, playing, start, cursor0, c = False, True, t, 0, 0
                elif not playing:
                    playing, start, cursor0 = True, t, c
            elif kind == "restart":
                completed, playing, start, cursor0, c = False, True, t, 0, 0
            elif kind == "seek":
                frame = action[1]
                assert 0 <= frame < n
                completed, c = False, frame * d
                if playing:
                    start, cursor0 = t, c
                else:
                    frozen = c
            else:
                raise ValueError(kind)
        if t in ticks:
            if completed:
                out[t] = (n - 1, False, True)
            else:
                frame = c // d
                if looping:
                    frame %= n
                out[t] = (frame, playing, False)
    return out


def fraction_check(item, tick):
    """Uncontrolled sequences only: frame = floor(tick * rate / 60) by exact Fraction."""
    assert not item["controls"] and item["playing"]
    k = Fraction(item["rate"][0], item["rate"][1]) * tick / TICK_RATE
    frame = math.floor(k)
    n = len(item["frames"])
    if item["looping"]:
        return frame % n, True, False
    return (n - 1, False, True) if frame >= n else (frame, True, False)


def expected_states(sc, ticks):
    return {item["name"]: segments_model(item, ticks) for item in sc.items if item["kind"] == "anim"}


def position(item, tick):
    """Exact f64 translation after `tick` steps of translation += velocity * (1/60)."""
    x, y = item["t"]
    if item["v"] is None:
        return [x, y]
    step = 1.0 / float(TICK_RATE)
    vx, vy = item["v"]
    for _ in range(tick):
        x += vx * step
        y += vy * step
    return [x, y]


# --------------------------------------------------------------- expansion --

def transform(item, translation=None):
    x, y = item["t"] if translation is None else translation
    z = EYE_Z if item["kind"] == "camera" else 0.0
    return {"translation": [x, y, z], "rotation": quat_z(item["r"]), "scale": [item["s"][0], item["s"][1], 1.0]}


class Assets:
    def __init__(self, ids, manifest):
        self.ids, self.manifest = ids, manifest

    def region(self, texture, region):
        kind = region[0]
        if kind == "frame":
            assert texture == "atlas"
            return self.ids["atlas"], {"frame": self.manifest["frames"].get(region[1], self.manifest["tiles"].get(
                region[1]))["id"]}
        if kind == "pixels":
            return self.ids[texture], list(region[1])
        if kind == "plate":
            return self.ids["plates"], list(self.manifest["plates"][region[1]])
        if kind == "badge":
            return self.ids["badge"], [0, 0, *self.manifest["badge"]]
        raise KeyError(region)

    def frame_region(self, texture, name):
        if texture == "atlas":
            return {"frame": self.manifest["frames"][name]["id"]}
        assert texture == "sheet"
        return list(self.manifest["sheet"]["regions"][name])


def sprite_value(assets, item, region_override=None):
    texture_id, region = assets.region(item["texture"], item["region"])
    if region_override is not None:
        region = region_override
    return {"texture": texture_id, "region": region, "size": list(item["size"]), "pivot": list(item["pivot"]),
            "tint": list(item["tint"]), "flip_x": item["flip"][0], "flip_y": item["flip"][1],
            "layer": item["layer"], "order": item["order"], "sampling": item["sampling"], "mode": item["mode"]}


def flipbook_value(assets, item):
    return {"frames": [assets.frame_region(item["texture"], f) for f in item["frames"]],
            "rate": {"numerator": item["rate"][0], "denominator": item["rate"][1]},
            "looping": item["looping"], "playing": item["playing"]}


def tilemap_value(assets, item):
    return {"tileset": assets.ids["atlas"], "size": list(item["size"]), "cell_size": list(item["cell"]),
            "layers": [{"id": lay["id"], "cells": [None if f is None else assets.manifest["tiles"][f]["id"]
                                                   for f in cells_bottom_up(lay["rows"])],
                        "layer": lay["layer"], "order": lay["order"], "solid": False}
                       for lay in item["layers"]]}


def camera_value(item):
    return {"fov_degrees": FOV, "near": NEAR, "far": FAR, "projection": item["projection"]}


def eid(sc, name):
    return entity_id(sc.by_name[name]["slot"])


def entity(sc, item, components):
    return {"id": eid(sc, item["name"]), "name": item["name"],
            "parent": eid(sc, item["parent"]) if item["parent"] else None, "components": components}


def animated_entities(sc, assets):
    out = []
    for item in sc.items:
        components = {"Transform": transform(item)}
        if item["v"] is not None:
            components["Velocity"] = {"linear": [item["v"][0], item["v"][1], 0.0]}
        if item["kind"] in ("sprite", "anim"):
            components["SpriteRenderer"] = sprite_value(assets, item)
        if item["kind"] == "anim":
            components["SpriteFlipbook"] = flipbook_value(assets, item)
        elif item["kind"] == "map":
            components["Tilemap"] = tilemap_value(assets, item)
        elif item["kind"] == "camera":
            components["Camera"] = camera_value(item)
        out.append(entity(sc, item, components))
    return out


def control_entities(sc, assets, tick):
    states = expected_states(sc, [tick])
    out = []
    for item in sc.items:
        components = {"Transform": transform(item, position(item, tick))}
        if item["kind"] == "sprite":
            components["SpriteRenderer"] = sprite_value(assets, item)
        elif item["kind"] == "anim":
            frame = item["frames"][states[item["name"]][tick][0]]
            components["SpriteRenderer"] = sprite_value(assets, item, assets.frame_region(item["texture"], frame))
        elif item["kind"] == "map":
            components["Tilemap"] = tilemap_value(assets, item)
        elif item["kind"] == "camera":
            components["Camera"] = camera_value(item)
        out.append(entity(sc, item, components))
    return out


def request(sc, ticks, width=VIEW[0], height=VIEW[1], with_controls=True):
    controls = []
    if with_controls:
        for item in sc.items:
            for c in item.get("controls", []):
                action = {"kind": c[1]} if c[1] != "seek" else {"kind": "seek", "frame": c[2]}
                controls.append((c[0], item["slot"], len(controls),
                                 {"tick": c[0], "entity": ulid_hex(eid(sc, item["name"])), "action": action}))
        controls = [c[3] for c in sorted(controls)]  # by tick; same-tick order preserved per entity
        controls = [c for c in controls if c["tick"] <= ticks[-1]]
    value = {"width": width, "height": height, "ticks": list(ticks)}
    if controls:
        value["controls"] = controls
    return value


def describe(sc, manifest):
    """JSON-serializable description recorded as evidence."""
    return {"items": sc.items, "ppu": PPU, "view": list(VIEW), "tick_rate": TICK_RATE,
            "ortho": {"centre": list(ORTHO_CENTRE), "vertical_size": ORTHO_VERTICAL},
            "ticks": {"main": MAIN_TICKS, "moments": MOMENTS, "ortho": ORTHO_TICKS, "subset": SUBSET_TICKS},
            "ids": {item["name"]: eid(sc, item["name"]) for item in sc.items},
            "hex": {item["name"]: ulid_hex(eid(sc, item["name"])) for item in sc.items},
            "art": manifest["files"]}


def write_json(path, value):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")
