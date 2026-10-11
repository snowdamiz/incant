"""Scene description for handoff 0039: three boards, tile and control expansions.

The scene is described once, independent of the engine. `tile_entities` emits the
tilemap project's entities; `control_entities` replaces every map with an empty
parent carrying the map's Transform and one ordinary SpriteRenderer child per
occupied cell, drawn from the original tiles.png with an explicit source
rectangle. A cell (x, y) of map M becomes a child at local (x*cw, y*ch) with pivot
(0, 0) and size (cw, ch), using its layer's layer/order. Its entity ID is
M's ID + 1 + rank, where rank orders M's cells by (layer, order, layer ID, y, x),
so ordinary (layer, order, entity ID) ordering reproduces the documented tile
order (layer, order, map ID, layer ID, y, x). Every entity ID is n << 20, so no
other entity falls between a map and its control cells.

Maps are authored as top-down row strings, exactly as they should appear on
screen; `cells_bottom_up` converts them to the contract's bottom-to-top storage.
"""

import json
import math
from pathlib import Path

PPU = 16
EYE_Z, NEAR, FAR, FOV = 10.0, 0.1, 50.0, 50.0
SLOT_BITS = 20
BOARDS = {"A": (0.0, 0.0), "B": (40.0, 0.0), "C": (80.0, 0.0)}
PLATE_SIZE = (24.0, 14.0)
VIEW_SIZES = {"pp1": (320, 180), "pp3": (960, 540), "of25": (640, 360), "o3": (960, 540)}
OF25_OFFSET = (0.1037, -0.0491)  # +4.15, -1.96 output px at 40 px/m


def crockford(prefix, n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    digits = ""
    for _ in range(16):
        digits = alphabet[n % 32] + digits
        n //= 32
    value = prefix + digits
    assert len(value) == 26 and value[0] in "01234567" and all(c in alphabet for c in value), value
    return value


def entity_id(n):
    return crockford("01JA39EN00", n)


def layer_id(n):
    return crockford("01JA39KY00", n)


def quat_z(degrees):
    h = math.radians(degrees) / 2
    return [0.0, 0.0, math.sin(h), math.cos(h)]


def cells_bottom_up(rows):
    """Top-down strings -> bottom-to-top row-major list of frame names or None."""
    width = len(rows[0])
    assert all(len(r) == width for r in rows), rows
    return [None if ch == "." else ch for row in reversed(rows) for ch in row]


class Scene:
    def __init__(self, state):
        self.state = state
        self.items = []
        self.by_name = {}

    def add(self, name, kind, *, parent=None, t=(0.0, 0.0), r=0.0, s=(1.0, 1.0), **data):
        item = {"name": name, "kind": kind, "slot": len(self.items) + 1, "parent": parent,
                "t": [float(t[0]), float(t[1])], "r": float(r), "s": [float(s[0]), float(s[1])], **data}
        assert name not in self.by_name, name
        self.items.append(item)
        self.by_name[name] = item
        return item

    def plate(self, board, colour):
        cx, cy = BOARDS[board]
        self.add(f"{board}:plate", "sprite", t=(cx, cy), texture=("plate", colour), size=PLATE_SIZE,
                 pivot=(0.5, 0.5), layer=-100, order=0)

    def sprite(self, name, texture, centre, size, layer=0, order=0, parent=None, r=0.0):
        return self.add(name, "sprite", parent=parent, t=centre, r=r, texture=texture, size=size,
                        pivot=(0.5, 0.5), layer=layer, order=order)

    def map(self, name, size, cell, layers, *, parent=None, t=(0.0, 0.0), r=0.0, s=(1.0, 1.0)):
        for layer in layers:
            assert len(layer["rows"]) == size[1] and all(len(row) == size[0] for row in layer["rows"]), name
        return self.add(name, "map", parent=parent, t=t, r=r, s=s, size=list(size), cell=list(cell),
                        layers=layers)

    def group(self, name, *, parent=None, t=(0.0, 0.0), r=0.0, s=(1.0, 1.0)):
        return self.add(name, "group", parent=parent, t=t, r=r, s=s)


def layer(n, rows, layer_value=0, order=0, label=""):
    return {"id": layer_id(n), "rows": list(rows), "layer": layer_value, "order": order, "label": label}


# ------------------------------------------------------------------ boards --

def board_a(sc, state):
    """Grid order, full-frame identity, holes, unequal frames, cell shapes and seams."""
    sc.plate("A", "board")
    a1 = (["ABCDEF", "GH.TWK", "X.FE.D", "CBAHGW"] if state == "s0"
          else ["ABCDE.", "GHXTWK", "X.FEAD", "DBAHGW"])
    sc.map("A1", (6, 4), (1.0, 1.0), [layer(1, a1, label="main")], t=(-7.5, 0.0))
    a2 = (["DCBA", "W.TE", "FGHX"] if state == "s0" else ["DCBAK", "W.TEG", "FGHXA"])
    sc.map("A2", (len(a2[0]), 3), (1.25, 0.75), [layer(2, a2, label="main")], t=(0.5, 1.75))
    sc.map("A3", (6, 1), (1.0, 1.0), [layer(3, ["ABCDEF"], label="half-pixel")],
           t=(0.5 + 1 / 32, 0.25 + 1 / 32))
    a4 = ["ABCDEFGH", "TWKXABCD", "EFGH.TWK", "XABC.DEF", "GHTWKXAB", "CDE..FGH", "TWKXABCD"]
    sc.map("A4", (8, 7), (0.7, 0.6), [layer(4, a4, label="non-dyadic")], t=(-7.5, -4.4))
    sc.map("A5", (6, 3), (0.5, 0.5), [layer(5, ["DFWABC", "TXHGED", "KKDDFF"], label="minified")],
           t=(0.5, -1.75))
    sc.map("A6", (3, 1), (2.0, 2.0), [layer(6, ["AEK"], label="magnified")], t=(0.5, -4.25))
    sc.map("A7", (12, 1), (1.0, 0.5), [layer(7, ["ABCDEFGHTWKX"], label="view-clipped")], t=(-6.0, 4.3))


def board_b(sc, state):
    """Overlapping layers, equal-tie stable layer IDs, interleaved ordinary sprites and a second map."""
    sc.plate("B", "board_b")
    # Lower than B1's ID: drawn before every (0, 0) cell of B1, visible through holes.
    sc.sprite("B:below", ("badge",), (35.5, -2.0), (4.0, 1.0))
    ground_order = 0 if state == "s0" else 4
    sc.map("B1", (6, 4), (1.0, 1.0), [
        # Document order is not painter order: decor precedes ground here, but ground's
        # stable ID is lower, so ground draws first at the equal (0, 0) key.
        layer(0x30, ["T....X", ".T.X..", "X.T...", "...T.X"], label="decor"),
        layer(0x20, ["ABCDEF", "G.HABC", "DE..FG", "H.ABC."], order=ground_order, label="ground"),
        layer(0x40, ["KKKKKK"] * 4, layer_value=-1, label="under"),
        layer(0x50, ["......", "....X.", "..W...", "......"], order=3, label="mid"),
        layer(0x10, [".X....", "......", "......", "......"], layer_value=2, order=-7, label="top"),
    ], t=(33.0, -2.5))
    # Higher than B1, lower than B2: above B1's (0, 0) cells, below B1's (0, 3) cell.
    sc.sprite("B:tie-after", ("frame", "F"), (35.5, -1.0), (2.0, 1.0))
    sc.map("B2", (3, 2), (1.0, 1.0), [layer(0x60, ["CTD", "EFX"], label="main")], t=(37.0, -1.5))
    sc.sprite("B:order5", ("badge",), (38.0, 0.25), (1.0, 1.0), order=5)
    sc.sprite("B:layer1", ("frame", "W"), (34.5, 1.25), (2.5, 0.5), layer=1)
    sc.sprite("B:layer2-low", ("badge",), (34.25, 1.25), (0.5, 0.5), layer=2, order=-8)
    sc.sprite("B:layer2-high", ("badge",), (34.75, 0.75), (0.5, 0.5), layer=2, order=-7)


def board_c(sc, state):
    """Inherited rotation, reflections, nonuniform scale, shear and a doubled-transform probe."""
    sc.plate("C", "board_c")
    sc.group("C1:parent", t=(74.5, 2.25), r=30.0)
    sc.map("C1", (3, 2), (1.0, 1.0), [layer(0x71, ["ABC", "DEF"], label="rotation")], parent="C1:parent",
           t=(-1.5, -1.0))
    sc.sprite("C:over-c1", ("badge",), (75.5, 2.75), (1.0, 1.0))
    sc.sprite("C:under-c2", ("badge",), (81.75, 2.25), (1.0, 1.0))
    sc.map("C2", (3, 2), (1.0, 1.0), [layer(0x72, ["ABC", "DEF"], label="reflect-x")], t=(81.5, 1.25),
           s=(-1.0, 1.0))
    sc.group("C3:parent", t=(85.5, 2.25), s=(1.0, -1.0))
    sc.map("C3", (3, 2), (1.0, 1.0), [layer(0x73, ["ABC", "DEF"], label="reflect-y")], parent="C3:parent",
           t=(-1.5, -1.0))
    sc.map("C4", (3, 2), (1.0, 1.0), [layer(0x74, ["GHT", "WKX"], label="scale+rotation")], t=(73.0, -3.0),
           r=15.0, s=(1.25, 0.6))
    sc.group("C5:parent", t=(80.0, -3.25), s=(1.3, 0.8))
    sc.map("C5", (2, 2), (1.0, 1.0), [layer(0x75, ["GH", "TE"], label="shear")], parent="C5:parent",
           t=(-0.2, 0.0), r=40.0)
    sc.group("C6:parent", t=(83.0, -3.5), s=(2.0, 2.0))
    sc.map("C6", (2, 2), (0.5, 0.5), [layer(0x76, ["AB", "CD"], label="inherited")], parent="C6:parent",
           t=(0.5, 0.25))


CAMERAS = {}
for _b, (_x, _y) in BOARDS.items():
    CAMERAS[f"pp-{_b}"] = {"board": _b, "t": (_x, _y),
                           "projection": {"kind": "pixel_perfect", "pixels_per_unit": PPU,
                                          "reference_resolution": [320, 180]}}
    CAMERAS[f"of-{_b}"] = {"board": _b, "t": (_x + OF25_OFFSET[0], _y + OF25_OFFSET[1]),
                           "projection": {"kind": "orthographic", "vertical_size": 9.0}}
CAMERAS["o3-C"] = {"board": "C", "t": BOARDS["C"], "projection": {"kind": "orthographic", "vertical_size": 11.25}}
# Named views: camera and output size. pp3 renders the pp camera at 3x integer scale.
VIEWS = {}
for _b in BOARDS:
    VIEWS[f"pp1-{_b}"] = (f"pp-{_b}", VIEW_SIZES["pp1"])
    VIEWS[f"pp3-{_b}"] = (f"pp-{_b}", VIEW_SIZES["pp3"])
    VIEWS[f"of25-{_b}"] = (f"of-{_b}", VIEW_SIZES["of25"])
VIEWS["o3-C"] = ("o3-C", VIEW_SIZES["o3"])


def build(state):
    sc = Scene(state)
    board_a(sc, state)
    board_b(sc, state)
    board_c(sc, state)
    for name, spec in CAMERAS.items():
        sc.add(f"camera:{name}", "camera", t=spec["t"], camera=name)
    return sc


# --------------------------------------------------------------- expansion --

def transform(item, translation=None):
    tx, ty = item["t"] if translation is None else translation
    z = EYE_Z if item["kind"] == "camera" else 0.0
    return {"translation": [tx, ty, z], "rotation": quat_z(item["r"]), "scale": [item["s"][0], item["s"][1], 1.0]}


def sprite_value(texture_id, region, size, pivot, layer_value, order):
    return {"texture": texture_id, "region": region, "size": list(size), "pivot": list(pivot),
            "tint": [1.0, 1.0, 1.0, 1.0], "flip_x": False, "flip_y": False, "layer": layer_value, "order": order,
            "sampling": "nearest", "mode": {"kind": "simple"}}


def camera_value(name):
    return {"fov_degrees": FOV, "near": NEAR, "far": FAR, "projection": CAMERAS[name]["projection"]}


class Assets:
    """Asset IDs and regions of one project. kind 'tile' references atlas frames by stable
    ID; kind 'control' references tiles.png with the frame's explicit source rectangle."""

    def __init__(self, kind, ids, manifest):
        self.kind, self.ids, self.manifest = kind, ids, manifest
        self.frames = {f["name"]: (fid, f["region"]) for fid, f in manifest["frames"].items()}

    def frame(self, name):
        fid, region = self.frames[name]
        if self.kind == "tile":
            return self.ids["atlas"], {"frame": fid}
        return self.ids["tiles"], list(region)

    def texture(self, texture):
        if texture[0] == "plate":
            return self.ids["plates"], list(self.manifest["plates"][texture[1]])
        if texture[0] == "badge":
            return self.ids["badge"], [0, 0, *self.manifest["badge"]]
        return self.frame(texture[1])


def eid(sc, name):
    return entity_id(sc.by_name[name]["slot"] << SLOT_BITS)


def ranked_cells(item):
    """Occupied cells of a map in documented painter order (layer, order, layer ID, y, x)."""
    w, h = item["size"]
    cells = []
    for lay in item["layers"]:
        for index, frame in enumerate(cells_bottom_up(lay["rows"])):
            if frame is not None:
                cells.append((lay["layer"], lay["order"], lay["id"], index // w, index % w, frame, lay))
    cells.sort(key=lambda c: c[:5])
    return cells


def tilemap_value(item, assets):
    return {"tileset": assets.ids["atlas"], "size": list(item["size"]), "cell_size": list(item["cell"]),
            "layers": [{"id": lay["id"], "cells": [None if f is None else assets.frames[f][0]
                                                   for f in cells_bottom_up(lay["rows"])],
                        "layer": lay["layer"], "order": lay["order"], "solid": False}
                       for lay in item["layers"]]}


def common(sc, item):
    return {"id": eid(sc, item["name"]), "name": item["name"],
            "parent": eid(sc, item["parent"]) if item["parent"] else None}


def tile_entities(sc, assets):
    out = []
    for item in sc.items:
        entity = common(sc, item)
        components = {"Transform": transform(item)}
        if item["kind"] == "sprite":
            tex, region = assets.texture(item["texture"])
            components["SpriteRenderer"] = sprite_value(tex, region, item["size"], item["pivot"], item["layer"],
                                                        item["order"])
        elif item["kind"] == "map":
            components["Tilemap"] = tilemap_value(item, assets)
        elif item["kind"] == "camera":
            components["Camera"] = camera_value(item["camera"])
        entity["components"] = components
        out.append(entity)
    return out


def control_entities(sc, assets):
    out = []
    for item in sc.items:
        entity = common(sc, item)
        components = {"Transform": transform(item)}
        if item["kind"] == "sprite":
            tex, region = assets.texture(item["texture"])
            components["SpriteRenderer"] = sprite_value(tex, region, item["size"], item["pivot"], item["layer"],
                                                        item["order"])
        elif item["kind"] == "camera":
            components["Camera"] = camera_value(item["camera"])
        entity["components"] = components
        out.append(entity)
        if item["kind"] != "map":
            continue
        base = item["slot"] << SLOT_BITS
        cw, ch = item["cell"]
        for rank, (lv, order, lid, y, x, frame, lay) in enumerate(ranked_cells(item)):
            tex, region = assets.frame(frame)
            out.append({"id": entity_id(base + 1 + rank), "name": f"{item['name']}:{lay['label']}:{x},{y}",
                        "parent": entity["id"],
                        "components": {"Transform": {"translation": [x * cw, y * ch, 0.0],
                                                     "rotation": [0.0, 0.0, 0.0, 1.0], "scale": [1.0, 1.0, 1.0]},
                                       "SpriteRenderer": sprite_value(tex, region, (cw, ch), (0.0, 0.0), lv,
                                                                      order)}})
    ids = [e["id"] for e in out]
    assert len(ids) == len(set(ids))
    return out


def camera_ids(sc):
    return {name: eid(sc, f"camera:{name}") for name in CAMERAS}


def describe(sc):
    """JSON-serializable description used by the analysis and recorded as evidence."""
    return {"state": sc.state, "items": sc.items, "cameras": CAMERAS,
            "views": {k: [v[0], list(v[1])] for k, v in VIEWS.items()},
            "ids": {item["name"]: eid(sc, item["name"]) for item in sc.items}}


def write_description(path, sc):
    Path(path).write_text(json.dumps(describe(sc), indent=1) + "\n", encoding="utf-8")
