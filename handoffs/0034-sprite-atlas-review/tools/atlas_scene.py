"""Shared board layout for handoff 0034: one scene, two texture bindings.

The atlas binding references every frame by its stable ULID through the cooked
atlas; the control binding draws the same entities from the original source
images with explicit pixel rectangles. Entity IDs, transforms, sizes, pivots,
layers, orders, sampling and modes are identical in both, so nearest-sampled
pixels must agree exactly if frames resolve and pack faithfully.
"""

import math

PPU = 16
BG = (24, 22, 40)
LIGHT_PLATE = (220, 214, 200)
EYE_Z, NEAR, FAR, FOV = 10.0, 0.1, 50.0, 50.0

# (world centre x, y in metres, logical width, height in px). Long strips on X run
# horizontally at |y| <= 20 m and those on Y vertically at 180 <= x <= 220 m, so they
# never cross another board's view. X stays at the origin for f32 position precision.
BOARD_SPEC = {
    "S": (-150.0, -100.0, 320, 180), "T": (-126.0, -100.0, 320, 180), "N": (-102.0, -100.0, 320, 180),
    "O": (-78.0, -100.0, 320, 180), "L": (-54.0, -100.0, 320, 180), "B": (-30.0, -100.0, 320, 180),
    "W": (-100.0, -200.0, 1280, 200), "V": (-300.0, -200.0, 320, 1080),
    "X": (0.0, 0.0, 1024, 832), "Y": (200.0, 100.0, 832, 1024),
}
SMALL_BOARDS = ["S", "T", "N", "O", "L", "B"]
ORIENTATIONS = ["NONE", "FX", "FY", "FXY", "MIR", "R90", "R180", "R270", "R90FX"]
NEAR_TIE = 1.0 / 256.0


def srgb_to_linear(v):
    v /= 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def tint_of(rgb, alpha=1.0):
    return [round(srgb_to_linear(c), 9) for c in rgb] + [alpha]


def quat_z(degrees):
    h = math.radians(degrees) / 2
    return [0.0, 0.0, math.sin(h), math.cos(h)]


IDENT = [0.0, 0.0, 0.0, 1.0]


def transform(t, rotation=IDENT, scale=(1, 1, 1)):
    # Translations are dyadic (multiples of 1/4096 m) and kept exactly.
    return {"translation": list(t), "rotation": [round(v, 12) for v in rotation], "scale": list(scale)}


def wx(board, px):
    cx, _, lw, _ = BOARD_SPEC[board]
    return cx - lw / PPU / 2 + px / PPU


def wy(board, py):
    _, cy, _, lh = BOARD_SPEC[board]
    return cy + lh / PPU / 2 - py / PPU


def entity_id(n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    digits = ""
    for _ in range(16):
        digits = alphabet[n % 32] + digits
        n //= 32
    return "01JA34EN00" + digits


class Binding:
    """Maps a frame name to (texture asset ID, region value) for one project."""

    def __init__(self, kind, textures, manifest, raw_rect=None):
        self.kind = kind
        self.textures = textures  # "atlas", "badge" and source file names -> asset IDs
        self.by_name = {f["name"]: (fid, f) for fid, f in manifest["frames"].items()}
        self.raw_rect = raw_rect

    def source(self, frame):
        fid, f = self.by_name[frame]
        if self.kind == "atlas":
            return self.textures["atlas"], {"frame": fid}
        region = f["region"] if f["region"] is not None else self.textures["ballast_v1_rect"]
        return self.textures[f["path"]], list(region)

    def raw(self, frame):
        """Atlas: texture with an explicit pixel rectangle (the v1 packed rectangle);
        control: the frame's source rectangle. Equal content only while v1 is current."""
        if self.kind == "atlas":
            return self.textures["atlas"], list(self.raw_rect)
        return self.source(frame)

    def scoped(self, frame, shown):
        """Atlas: the alt atlas's entry for `frame`'s stable ID (whose content is `shown`);
        control: `shown` from its source image."""
        if self.kind == "atlas":
            return self.textures["alt"], {"frame": self.by_name[frame][0]}
        return self.source(shown)

    def region_size(self, frame):
        f = self.by_name[frame][1]
        return f["region"][2:] if f["region"] is not None else self.textures["ballast_v1_rect"][2:]


class Scene:
    def __init__(self, binding):
        self.binding = binding
        self.entities, self.ids, self.meta = [], {}, {}

    def entity(self, name, components, parent=None, meta=None):
        if name in self.ids:
            raise ValueError(f"duplicate entity name {name}")
        eid = entity_id(len(self.entities) + 1)
        self.entities.append({"id": eid, "name": name, "parent": parent, "components": components})
        self.ids[name] = eid
        if meta is not None:
            self.meta[name] = meta
        return eid

    def sprite(self, name, board, frame, rect=None, *, pivot=(0.5, 0.5), center=None, size=None, angle=0.0,
               flip_x=False, flip_y=False, tint=(1, 1, 1, 1), layer=0, order=0, z=0.0, sampling="nearest",
               mode=None, parent=None, scale=(1, 1, 1), local=None, raw=False, texture=None):
        """rect = (x, y, w, h) in board logical px (top-left origin), placed via pivot;
        center + size place a sprite about its pivot point (logical px)."""
        if texture is not None:
            tex, region = texture
        else:
            tex, region = (self.binding.raw if raw else self.binding.source)(frame)
        if local is not None:
            w, h = size
            translation = local
        elif rect is not None:
            x, y, w, h = rect
            translation = (wx(board, x) + pivot[0] * w / PPU, wy(board, y + h) + pivot[1] * h / PPU, z)
        else:
            w, h = size
            translation = (wx(board, center[0]), wy(board, center[1]), z)
        value = {"texture": tex, "region": region, "size": [w / PPU, h / PPU], "pivot": list(pivot),
                 "tint": list(tint), "flip_x": flip_x, "flip_y": flip_y, "layer": layer, "order": order,
                 "sampling": sampling, "mode": mode or {"kind": "simple"}}
        components = {"Transform": transform(translation, quat_z(angle), scale), "SpriteRenderer": value}
        meta = {"board": board, "frame": frame, "rect": list(rect) if rect is not None else None,
                "center": list(center) if center is not None else None, "size_px": [w, h], "angle": angle,
                "flip": [flip_x, flip_y], "sampling": sampling, "raw": raw, "pivot": list(pivot),
                "mode": value["mode"], "layer": layer, "order": order, "mirror": scale[0] < 0}
        return self.entity(name, components, parent=self.ids[parent] if parent else None, meta=meta)

    def label(self, board, text, x, y, layer=50):
        w, h = self.binding.region_size(f"label:{text}")
        return self.sprite(f"{board}:label:{text}:{x},{y}", board, f"label:{text}", (x, y, w, h), pivot=(0, 1),
                           layer=layer)

    def words(self, board, words, x, y):
        for word in words:
            self.label(board, word, x, y)
            x += self.binding.region_size(f"label:{word}")[0] + 4

    def plate(self, name, board, rect, rgb, layer=-100, alpha=1.0):
        return self.sprite(name, board, "solid", rect, tint=tint_of(rgb, alpha), layer=layer)

    def oriented(self, name, board, frame, centre, size, orientation, layer=0):
        """One of the nine ORIENTATIONS about an exact centre. MIR uses a parent with
        scale -1 on X, so the reflection comes from the inherited transform."""
        flips = {"FX": (True, False), "FY": (False, True), "FXY": (True, True), "R90FX": (True, False)}
        angles = {"R90": 90.0, "R180": 180.0, "R270": 270.0, "R90FX": 90.0}
        fx, fy = flips.get(orientation, (False, False))
        if orientation == "MIR":
            self.entity(f"{name}:mirror", {"Transform": transform(
                (wx(board, centre[0]), wy(board, centre[1]), 0.0), IDENT, (-1, 1, 1))})
            eid = self.sprite(name, board, frame, size=size, local=(0.0, 0.0, 0.0), parent=f"{name}:mirror",
                              layer=layer)
        else:
            eid = self.sprite(name, board, frame, center=centre, size=size, angle=angles.get(orientation, 0.0),
                              flip_x=fx, flip_y=fy, layer=layer)
        self.meta[name].update(center=list(centre), orientation=orientation)
        return eid


def nine(border, density):
    return {"kind": "nine_slice", "border": list(border), "pixels_per_unit": density}


PANEL_BORDER = [3, 6, 4, 8]
CARDS = "0123456789AB"


def board_s(s):
    b = "S"
    s.plate("S:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "S", 312, 172)
    s.label(b, "STABLE FRAMES", 6, 4)
    s.label(b, "CARDS 1X", 6, 12)
    for k, g in enumerate(CARDS):
        s.sprite(f"S:card{g}:1x", b, f"card{g}", (6 + 26 * k, 19, 12, 14), pivot=(0, 0))
    s.label(b, "CARDS 2X", 6, 38)
    for k, g in enumerate(CARDS[:10]):
        s.sprite(f"S:card{g}:2x", b, f"card{g}", (6 + 30 * k, 45, 24, 28), pivot=(0, 0))
    s.label(b, "WISP", 6, 80)
    for k in range(4):
        s.sprite(f"S:wisp{k}", b, f"wisp{k}", (6 + 20 * k, 87, 16, 16), pivot=(0, 0))
    s.label(b, "KEY", 90, 80)
    s.sprite("S:key", b, "key", (90, 87, 8, 8), pivot=(0, 0))
    s.label(b, "RULER", 106, 80)
    s.sprite("S:odd", b, "odd", (106, 87, 7, 7), pivot=(0, 0))
    s.sprite("S:odd3x", b, "odd", (116, 87, 21, 21), pivot=(0, 0))
    s.label(b, "CRATE", 144, 80)
    s.sprite("S:crate", b, "crate", (144, 87, 12, 12), pivot=(0, 0))
    s.label(b, "PROBE", 166, 80)
    s.sprite("S:probe", b, "probe", (166, 87, 11, 9), pivot=(0, 0))
    s.sprite("S:probe3x", b, "probe", (182, 87, 33, 27), pivot=(0, 0))
    s.label(b, "STAR", 222, 80)
    s.sprite("S:star", b, "star", (222, 87, 3, 3), pivot=(0, 0))
    s.sprite("S:marker", b, "marker", (230, 87, 2, 2), pivot=(0, 0))
    s.sprite("S:cardA:2x", b, "cardA", (246, 87, 24, 28), pivot=(0, 0))
    s.sprite("S:cardB:2x", b, "cardB", (276, 87, 24, 28), pivot=(0, 0))
    s.label(b, "TILES", 6, 120)
    for k in range(18):
        s.sprite(f"S:tile{k}", b, f"tile{k % 2}", (16 + 16 * k, 127, 16, 16), pivot=(0, 0))
    s.label(b, "TILES TIE", 6, 146)
    for k in range(18):
        s.sprite(f"S:tietile{k}", b, f"tile{(k + 1) % 2}", (16.5 + 16 * k, 153.5, 16, 16), pivot=(0, 0))


def board_t(s):
    b = "T"
    s.plate("T:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "T", 312, 172)
    s.label(b, "TRANSFORMS", 6, 4)
    for k, o in enumerate(ORIENTATIONS):
        x = 6 + 34 * k
        s.label(b, o, x, 12)
        s.oriented(f"T:probe1x:{o}", b, "probe", (x + 5.5, 24.5), (11, 9), o)
        s.oriented(f"T:probe2x:{o}", b, "probe", (x + 11, 45), (22, 18), o)
        s.oriented(f"T:ruler-tie1x:{o}", b, "odd", (x + 4, 68), (7, 7), o)
        s.oriented(f"T:ruler-tie3x:{o}", b, "odd", (x + 11, 91), (21, 21), o)
        s.oriented(f"T:ruler-aligned:{o}", b, "odd", (x + 3.5, 116.5), (7, 7), o)
    s.label(b, "RULER TIES 1X", 6, 58)
    s.label(b, "RULER TIES 3X", 6, 74)
    s.label(b, "RULER ALIGNED", 6, 105)
    s.label(b, "PIVOTS", 6, 128)
    for k, p in enumerate([(0, 0), (0.5, 0), (0.5, 0.5), (1, 1), (0.25, 0.75)]):
        ox, oy = 26 + 40 * k, 156
        x = ox - p[0] * 12
        y = oy - (1 - p[1]) * 14
        s.sprite(f"T:pivot{k}", b, "card3", (x, y, 12, 14), pivot=p)
        s.sprite(f"T:pivot{k}:origin", b, "marker", (ox - 1, oy - 1, 2, 2), pivot=(0.5, 0.5), layer=10)


def board_n(s):
    b = "N"
    s.plate("N:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "N", 312, 172)
    s.label(b, "9-SLICE", 6, 4)
    n16 = nine(PANEL_BORDER, PPU)
    s.label(b, "NATIVE", 6, 12)
    s.sprite("N:native", b, "panel", (6, 19, 24, 24), pivot=(0, 0), mode=n16)
    s.label(b, "SIMPLE", 36, 12)
    s.sprite("N:simple24", b, "panel", (36, 19, 24, 24), pivot=(0, 0))
    s.label(b, "STRETCH 72X44", 66, 12)
    s.sprite("N:stretch", b, "panel", (66, 19, 72, 44), pivot=(0, 0), mode=n16)
    s.label(b, "SIMPLE 72X44", 146, 12)
    s.sprite("N:simple-stretch", b, "panel", (146, 19, 72, 44), pivot=(0, 0))
    s.label(b, "SMALL", 226, 12)
    for k, (x, w, h) in enumerate([(226, 8, 10), (238, 6, 6), (248, 12, 7), (264, 9, 12)]):
        s.sprite(f"N:small{k}", b, "panel", (x, 19, w, h), pivot=(0, 0), mode=n16)
    s.label(b, "FLIPS", 6, 70)
    s.sprite("N:flipx", b, "panel", (6, 77, 48, 32), pivot=(0, 0), mode=n16, flip_x=True)
    s.sprite("N:flipy", b, "panel", (60, 77, 48, 32), pivot=(0, 0), mode=n16, flip_y=True)
    s.sprite("N:flipxy", b, "panel", (114, 77, 48, 32), pivot=(0, 0), mode=n16, flip_x=True, flip_y=True)
    s.label(b, "ROT 180", 168, 70)
    s.sprite("N:rot180", b, "panel", None, center=(192, 93), size=(48, 32), angle=180.0, mode=n16)
    s.label(b, "MIRROR", 222, 70)
    s.entity("N:mirror:parent", {"Transform": transform((wx(b, 246), wy(b, 93), 0.0), IDENT, (-1, 1, 1))})
    s.sprite("N:mirror", b, "panel", size=(48, 32), local=(0.0, 0.0, 0.0), parent="N:mirror:parent", mode=n16)
    s.meta["N:mirror"].update(center=[246, 93], mirror=True)
    s.label(b, "DENSITY 8", 6, 116)
    s.sprite("N:density8", b, "panel", (6, 123, 72, 44), pivot=(0, 0), mode=nine(PANEL_BORDER, 8))
    s.label(b, "DENSITY 32", 84, 116)
    s.sprite("N:density32", b, "panel", (84, 123, 72, 44), pivot=(0, 0), mode=nine(PANEL_BORDER, 32))
    s.label(b, "ROT 12", 164, 116)
    s.sprite("N:rot12", b, "panel", None, center=(196, 147), size=(60, 36), angle=12.0, mode=n16)
    s.label(b, "PROBE 1-1-1-1", 232, 116)
    s.sprite("N:probe9", b, "probe", (232, 123, 40, 24), pivot=(0, 0), mode=nine([1, 1, 1, 1], PPU))
    s.sprite("N:probe9fx", b, "probe", (276, 123, 40, 24), pivot=(0, 0), mode=nine([1, 1, 1, 1], PPU),
             flip_x=True)


def board_o(s, badge):
    b = "O"
    s.plate("O:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "O", 312, 172)
    s.label(b, "ORDER", 6, 4)
    s.label(b, "LAYER", 6, 12)
    s.sprite("O:layer-top", b, "card0", (6, 19, 24, 28), pivot=(0, 0), layer=1, order=-5)
    s.sprite("O:layer-under", b, "card1", (18, 27, 24, 28), pivot=(0, 0), layer=0, order=10)
    s.label(b, "ORDER", 56, 12)
    s.sprite("O:order-top", b, "card2", (56, 19, 24, 28), pivot=(0, 0), order=2)
    s.sprite("O:order-under", b, "card3", (68, 27, 24, 28), pivot=(0, 0), order=1)
    s.label(b, "ID TIE", 106, 12)
    s.sprite("O:tie-first", b, "card4", (106, 19, 24, 28), pivot=(0, 0))
    s.sprite("O:tie-second", b, "card5", (118, 27, 24, 28), pivot=(0, 0))
    s.label(b, "Z IGNORED", 156, 12)
    s.sprite("O:z-near", b, "card6", (156, 19, 24, 28), pivot=(0, 0), z=2.0)
    s.sprite("O:z-far-top", b, "card7", (168, 27, 24, 28), pivot=(0, 0), z=-2.0, layer=1)
    s.label(b, "BATCH", 206, 12)
    for k, (frame, ordinary) in enumerate([("card8", False), (None, True), ("card9", False), (None, True),
                                           ("cardA", False)]):
        rect = (206 + 9 * k, 19 + 7 * k, 16, 16) if ordinary else (206 + 9 * k, 19 + 7 * k, 12, 14)
        s.sprite(f"O:batch{k}", b, frame, rect, pivot=(0, 0), layer=5, order=k,
                 texture=(badge, [0, 0, 16, 16]) if ordinary else None)
    s.label(b, "ALPHA 0.5", 6, 70)
    for k, g in enumerate("012"):
        s.sprite(f"O:alpha-dark{k}", b, f"card{g}", (6 + 10 * k, 77 + 8 * k, 24, 28), pivot=(0, 0),
                 tint=(1, 1, 1, 0.5), order=k)
    s.label(b, "LIGHT", 80, 70)
    s.plate("O:light-plate", b, (80, 77, 70, 60), LIGHT_PLATE, layer=-50)
    for k, g in enumerate("456"):
        s.sprite(f"O:alpha-light{k}", b, f"card{g}", (86 + 10 * k, 81 + 8 * k, 24, 28), pivot=(0, 0),
                 tint=(1, 1, 1, 0.5), order=k)
    s.label(b, "ORB 2X", 166, 70)
    s.sprite("O:orb-near", b, "orb", (166, 77, 32, 32), pivot=(0, 0))
    s.label(b, "GLOW 2X", 206, 70)
    s.sprite("O:glow-near", b, "glow", (206, 77, 32, 32), pivot=(0, 0), tint=tint_of((150, 110, 230)))
    s.plate("O:light-plate2", b, (246, 77, 70, 40), LIGHT_PLATE, layer=-50)
    s.sprite("O:orb-on-light", b, "orb", (250, 81, 32, 32), pivot=(0, 0))
    s.sprite("O:halo-on-light", b, "halo", (284, 85, 24, 24), pivot=(0, 0))
    s.label(b, "WISP", 166, 120)
    for k in range(4):
        s.sprite(f"O:wisp-stack{k}", b, f"wisp{k}", (166 + 10 * k, 127 + 4 * k, 32, 32), pivot=(0, 0),
                 order=-k)


def board_l(s):
    b = "L"
    s.plate("L:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "L", 312, 172)
    s.label(b, "LINEAR", 6, 4)
    s.label(b, "ORB 4X", 6, 12)
    s.sprite("L:orb4x", b, "orb", (6, 19, 64, 64), pivot=(0, 0), sampling="linear")
    s.label(b, "ORB ON LIGHT", 80, 12)
    s.plate("L:light-orb", b, (80, 19, 72, 72), LIGHT_PLATE, layer=-50)
    s.sprite("L:orb-light", b, "orb", (84, 23, 64, 64), pivot=(0, 0), sampling="linear")
    s.label(b, "HALO 3X", 160, 12)
    s.sprite("L:halo3x", b, "halo", (160, 19, 72, 72), pivot=(0, 0), sampling="linear")
    s.label(b, "HALO 2.5X", 240, 12)
    s.plate("L:light-halo", b, (238, 19, 64, 64), LIGHT_PLATE, layer=-50)
    s.sprite("L:halo2.5x", b, "halo", (240, 21, 60, 60), pivot=(0, 0), sampling="linear")
    s.label(b, "GLOW", 6, 100)
    s.sprite("L:glow", b, "glow", (6, 107, 48, 48), pivot=(0, 0), sampling="linear",
             tint=tint_of((150, 110, 230)))
    s.label(b, "WISP 2X", 60, 100)
    s.sprite("L:wisp2x", b, "wisp0", (60, 107, 32, 32), pivot=(0, 0), sampling="linear")
    s.label(b, "CRATE 30", 98, 100)
    s.sprite("L:crate30", b, "crate", None, center=(116, 132), size=(24, 24), angle=30.0, sampling="linear")
    s.label(b, "PANEL", 140, 100)
    s.sprite("L:panel", b, "panel", (140, 107, 72, 44), pivot=(0, 0), sampling="linear",
             mode=nine(PANEL_BORDER, PPU))
    s.label(b, "PROBE 3X", 220, 100)
    s.sprite("L:probe3x", b, "probe", (220, 107, 33, 27), pivot=(0, 0), sampling="linear")
    s.sprite("L:probe3x-fx", b, "probe", (258, 107, 33, 27), pivot=(0, 0), sampling="linear", flip_x=True)
    s.label(b, "CARD 7", 220, 138)
    s.sprite("L:card7-2.5x", b, "card7", (220, 145, 30, 35), pivot=(0, 0), sampling="linear")
    s.sprite("L:card7-1x", b, "card7", (256, 150, 12, 14), pivot=(0, 0), sampling="linear")


def board_b(s):
    b = "B"
    s.plate("B:background", b, (-16, -16, 352, 212), BG)
    s.label(b, "B", 312, 172)
    s.label(b, "BALLAST", 6, 4)
    s.sprite("B:ballast", b, "ballast", (10, 12, 300, 10), pivot=(0, 0))
    s.label(b, "PIXEL RECT", 6, 60)
    s.sprite("B:raw-card7", b, "card7", (10, 67, 36, 42), pivot=(0, 0), raw=True)
    s.label(b, "STABLE ID", 70, 60)
    s.sprite("B:stable-card7", b, "card7", (74, 67, 36, 42), pivot=(0, 0))
    s.label(b, "SCOPED ID", 140, 60)
    s.sprite("B:scoped-card0", b, "card1", (144, 67, 36, 42), pivot=(0, 0), texture=s.binding.scoped("card0", "card1"))
    s.sprite("B:scoped-card1", b, "card0", (190, 67, 36, 42), pivot=(0, 0), texture=s.binding.scoped("card1", "card0"))


def board_w(s):
    """1x horizontal strips. Span cuts lie 512 and 1024 screen px right of each left edge."""
    b = "W"
    s.plate("W:background", b, (-16, -16, 1312, 232), BG)
    s.label(b, "W", 1268, 190)
    s.label(b, "WIDE 1100 1X", 6, 2)
    rows = [("ALIGNED", "hstrip", False, 0.0), ("TIE", "hstrip", False, 0.5), ("FLIP", "hstrip", True, 0.0),
            ("FLIP TIE", "hstrip", True, 0.5), ("TIE +1/256", "hstrip", False, 0.5 + NEAR_TIE),
            ("FLIP +1/256", "hstrip", True, 0.5 + NEAR_TIE), ("TIE -1/256", "hstrip", False, 0.5 - NEAR_TIE),
            ("FLIP -1/256", "hstrip", True, 0.5 - NEAR_TIE), ("MID", "hmid", False, 0.0),
            ("MID TIE", "hmid", False, 0.5), ("MID FLIP", "hmid", True, 0.0), ("MID FLIP TIE", "hmid", True, 0.5)]
    for r, (label, frame, flip, dx) in enumerate(rows):
        y = 14 + 12 * r
        s.label(b, label, 4, y)
        h = 4 if frame == "hstrip" else 1
        w = 1100 if frame == "hstrip" else 700
        s.sprite(f"W:{label}", b, frame, (100 + dx, y, w, h), pivot=(0, 1), flip_x=flip)
    for cut in (512, 1024):
        s.plate(f"W:tick{cut}", b, (100 + cut, 160, 1, 8), (200, 200, 200))
        s.label(b, f"SPAN {cut}", 100 + cut - 16, 172)


def board_v(s):
    """1x vertical strips; the board shows strip rows 10 to 1089."""
    b = "V"
    s.plate("V:background", b, (-16, -16, 352, 1112), BG)
    s.label(b, "V", 300, 1070)
    cols = [("ALIGNED", "vstrip", False, 0.0), ("TIE", "vstrip", False, 0.5), ("FLIP", "vstrip", True, 0.0),
            ("FLIP TIE", "vstrip", True, 0.5), ("TIE +1/256", "vstrip", False, 0.5 + NEAR_TIE),
            ("FLIP +1/256", "vstrip", True, 0.5 + NEAR_TIE), ("TIE -1/256", "vstrip", False, 0.5 - NEAR_TIE),
            ("FLIP -1/256", "vstrip", True, 0.5 - NEAR_TIE), ("MID", "vmid", False, 0.0),
            ("MID TIE", "vmid", False, 0.5), ("MID FLIP", "vmid", True, 0.0), ("MID FLIP TIE", "vmid", True, 0.5)]
    for c, (label, frame, flip, dy) in enumerate(cols):
        x = 60 + 20 * c
        w = 4 if frame == "vstrip" else 1
        h = 1100 if frame == "vstrip" else 700
        top = -10 + dy if frame == "vstrip" else 40 + dy
        s.sprite(f"V:{label}", b, frame, (x, top, w, h), pivot=(0, 1), flip_y=flip)
    for cut, label in ((502, "SPAN 512"), (1014, "SPAN 1024")):
        s.plate(f"V:tick{cut}", b, (40, cut, 8, 1), (200, 200, 200))
        s.label(b, label, 4, cut + 4)


def magnified_rows():
    """(frame, magnification, flip, placement offset px, span boundary in texels).
    Boundary "lead"/"trail" instead puts the strip's screen-leading/trailing edge at
    board 100 (+offset) / 900 (+offset): at 32x, 1100 x 32 = 35,200 > 32,768, where 0032
    found exact ties lost at large region-local coordinates before span splitting."""
    rows = []
    for m in (2, 4, 8, 16, 32):
        for flip in (False, True):
            for dx in (0.0, 0.5):
                rows.append(("row", m, flip, dx, 512))
    for m in (4, 16):
        for flip in (False, True):
            for dx in (0.0, 0.5):
                rows.append(("mid", m, flip, dx, 512))
    for m in (2, 4):
        for flip in (False, True):
            rows.append(("row", m, flip, 0.0, 1024))
    for flip in (False, True):
        for dx in (0.5 + NEAR_TIE, 0.5 - NEAR_TIE):
            rows.append(("row", 8, flip, dx, 512))
    for edge in ("lead", "trail"):
        for flip in (False, True):
            for dx in (0.0, 0.5):
                rows.append(("row", 32, flip, dx, edge))
    return rows


def magnified_start(cut, m, length, dx):
    """Screen-low edge of a magnified strip so that `cut` lands on board 512 (+dx)."""
    if cut == "lead":
        return 100 + dx
    if cut == "trail":
        return 900 + dx - length * m
    return 512 - cut * m + dx


def board_x(s):
    """Magnified 1-texel rows. Each row's span cut lands on board x = 512 (+ offset)."""
    b = "X"
    s.plate("X:background", b, (-16, -16, 1056, 864), BG)
    s.label(b, "X", 1010, 818)
    s.label(b, "SPAN 512", 496, 818)
    y = 4
    for k, (kind, m, flip, dx, cut) in enumerate(magnified_rows()):
        frame, length = ("hrow", 1100) if kind == "row" else ("hmid", 700)
        s.sprite(f"X:{k}:{kind}:{m}x:{'flip' if flip else 'none'}:{dx}:{cut}", b, frame,
                 (magnified_start(cut, m, length, dx), y, length * m, m), pivot=(0, 1), flip_x=flip)
        y += m + 4


def board_y(s):
    b = "Y"
    s.plate("Y:background", b, (-16, -16, 864, 1056), BG)
    s.label(b, "Y", 816, 1012)  # every other column of this board carries a strip
    x = 4
    for k, (kind, m, flip, dy, cut) in enumerate(magnified_rows()):
        frame, length = ("vcol", 1100) if kind == "row" else ("vmid", 700)
        s.sprite(f"Y:{k}:{kind}:{m}x:{'flip' if flip else 'none'}:{dy}:{cut}", b, frame,
                 (x, magnified_start(cut, m, length, dy), m, length * m), pivot=(0, 1), flip_y=flip)
        x += m + 4


def build(binding, badge):
    s = Scene(binding)
    board_s(s)
    board_t(s)
    board_n(s)
    board_o(s, badge)
    board_l(s)
    board_b(s)
    board_w(s)
    board_v(s)
    board_x(s)
    board_y(s)
    return s


# ------------------------------------------------------------------ cameras --

def pixel(reference):
    return {"kind": "pixel_perfect", "pixels_per_unit": PPU, "reference_resolution": list(reference)}


def ortho(vertical):
    return {"kind": "orthographic", "vertical_size": vertical}


CAMERAS = {}
for _b in SMALL_BOARDS:
    CAMERAS[f"pp-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0, "projection": pixel((320, 180))}
    # 40 px/m = 2.5 output px per texel, offset (+4.15, -1.96) output px from the board centre.
    CAMERAS[f"of-{_b}"] = {"board": _b, "offset": (0.1037, -0.0491), "roll": 0, "projection": ortho(9.0)}
    CAMERAS[f"rl-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 7, "projection": ortho(9.0)}
for _b in ("S", "N"):
    CAMERAS[f"o3-{_b}"] = {"board": _b, "offset": (0, 0), "roll": 0, "projection": ortho(11.25)}
CAMERAS["pp-W"] = {"board": "W", "offset": (0, 0), "roll": 0, "projection": pixel((1280, 200))}
CAMERAS["of-W"] = {"board": "W", "offset": (0.0371, -0.0123), "roll": 0, "projection": ortho(200 / 18)}
CAMERAS["rl-W"] = {"board": "W", "offset": (0, 0), "roll": 3, "projection": ortho(18.0)}
CAMERAS["pp-V"] = {"board": "V", "offset": (0, 0), "roll": 0, "projection": pixel((320, 1080))}
CAMERAS["of-V"] = {"board": "V", "offset": (0.0371, -0.0123), "roll": 0, "projection": ortho(1080 / 14)}
CAMERAS["rl-V"] = {"board": "V", "offset": (0, 0), "roll": 3, "projection": ortho(67.5)}
CAMERAS["pp-X"] = {"board": "X", "offset": (0, 0), "roll": 0, "projection": pixel((1024, 832))}
CAMERAS["pp-Y"] = {"board": "Y", "offset": (0, 0), "roll": 0, "projection": pixel((832, 1024))}

SUBSET = ([(f"pp-{b}", (320, 180)) for b in SMALL_BOARDS]
          + [("pp-W", (1280, 200)), ("pp-V", (320, 1080)), ("pp-X", (1024, 832)), ("pp-Y", (832, 1024))])
FULL = (SUBSET + [(f"of-{b}", (640, 360)) for b in SMALL_BOARDS] + [(f"rl-{b}", (640, 360)) for b in SMALL_BOARDS]
        + [("of-W", (1280, 200)), ("rl-W", (1280, 288)), ("of-V", (320, 1080)), ("rl-V", (360, 1080))])
VIEW = [(f"pp-{b}", (960, 540)) for b in SMALL_BOARDS] + [("o3-S", (960, 540)), ("o3-N", (960, 540))]
PP_SMALL = [(f"pp-{b}", (320, 180)) for b in SMALL_BOARDS]


def camera_entities(s):
    ids = {}
    for name, spec in CAMERAS.items():
        bx, by = BOARD_SPEC[spec["board"]][:2]
        value = {"fov_degrees": FOV, "near": NEAR, "far": FAR, "projection": spec["projection"]}
        t = (bx + spec["offset"][0], by + spec["offset"][1], EYE_Z)
        ids[name] = s.entity(f"camera:{name}", {"Camera": value, "Transform": transform(t, quat_z(spec["roll"]))})
    return ids
