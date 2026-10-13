"""Fixture boards for handoff 0063 (pure data; author.py turns them into bus commands).

Board W: world-space text only, framed so that the orthographic camera maps
100 px per metre (the text contract's 100 layout px per local unit), i.e. one
atlas texel per pixel at scale 1. The perspective camera frames z = 0 the same.
Board U: the 0056 game-UI layout (title, mixed-script body, button row with a
disabled and an Arabic button, caption/field groups) plus a fixed-width clip
strip, long LTR and RTL overflowing fields, and a world-text sign behind the UI.

The strings use only glyphs present in the packaged test fonts (Latin, Arabic
and the CJK subset 中文日本語一). Meshes are generated unit cubes.
"""

import base64
import json
import math
import struct

PREFIX = "000000000000000000000063"
FONTS = ("NotoSans-Regular.ttf", "NotoSansArabic.ttf", "IncantCJKFixture.ttf")


def ids(names):
    return {name: PREFIX + suffix for name, suffix in names.items()}


def quat(axis, degrees):
    half = math.radians(degrees) / 2
    s = math.sin(half)
    return [axis[0] * s, axis[1] * s, axis[2] * s, math.cos(half)]


def mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def transform(at, rotation=(0, 0, 0, 1), scale=(1, 1, 1)):
    return {"translation": list(at), "rotation": list(rotation), "scale": list(scale)}


def cube(color):
    """A unit cube glTF with flat normals and one PBR base colour."""
    positions, normals, indices = [], [], []
    for axis in range(3):
        for sign in (-1, 1):
            normal = [0, 0, 0]
            normal[axis] = sign
            u, v = [(1, 2), (2, 0), (0, 1)][axis]
            base = len(positions)
            for a, b in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                p = [0.0, 0.0, 0.0]
                p[axis], p[u], p[v] = 0.5 * sign, 0.5 * a, 0.5 * b
                positions.append(p)
                normals.append(normal)
            quad = [0, 1, 2, 0, 2, 3] if sign > 0 else [0, 2, 1, 0, 3, 2]
            indices.extend(base + i for i in quad)
    data = b"".join(struct.pack("<3f", *p) for p in positions)
    data += b"".join(struct.pack("<3f", *n) for n in normals)
    data += b"".join(struct.pack("<H", i) for i in indices)
    count = len(positions)
    return {"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": [0]}], "nodes": [{"mesh": 0}],
            "materials": [{"pbrMetallicRoughness": {"baseColorFactor": color,
                                                    "metallicFactor": 0, "roughnessFactor": 0.6}}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
            "buffers": [{"byteLength": len(data),
                         "uri": "data:application/octet-stream;base64," + base64.b64encode(data).decode("ascii")}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": count * 12},
                            {"buffer": 0, "byteOffset": count * 12, "byteLength": count * 12},
                            {"buffer": 0, "byteOffset": count * 24, "byteLength": len(indices) * 2}],
            "accessors": [{"bufferView": 0, "componentType": 5126, "count": count, "type": "VEC3",
                           "min": [-0.5] * 3, "max": [0.5] * 3},
                          {"bufferView": 1, "componentType": 5126, "count": count, "type": "VEC3"},
                          {"bufferView": 2, "componentType": 5123, "count": len(indices), "type": "SCALAR"}]}


MESHES = {"slate": [0.12, 0.13, 0.16, 1], "amber": [0.80, 0.52, 0.20, 1], "teal": [0.18, 0.46, 0.50, 1],
          "floor": [0.15, 0.17, 0.20, 1]}
KEY_LIGHT = mul(quat((0, 1, 0), 30), quat((1, 0, 0), -40))

# ---------------------------------------------------------------- board W
W_IDS = ids({
    "cam_ortho": "10", "cam_persp": "11", "light": "12", "wall": "13", "panel": "14", "crate": "15",
    "flip_parent": "16",
    "w_latin": "20", "w_latin_dup": "21", "w_arabic": "22", "w_cjk": "23", "w_rot": "24", "w_bidi": "25",
    "w_mirror": "26", "w_mirror_ctl": "27", "w_back": "28", "w_mag": "29", "w_child": "2A",
    "w_child_ctl": "2B", "w_min50": "2C", "w_min25": "2D", "w_min12": "2E", "w_yaw": "2F",
    "w_behind": "30", "w_front": "31", "w_wrap": "32", "w_overlap_a": "33", "w_overlap_b": "34",
    "w_moving": "35",
})
# Orthographic view: x in [-4.8, 4.8], y in [-2.7, 2.7] at 100 px/m. Column and row origins
# are on pixel boundaries (multiples of 0.01 m from the frame edge), so the controls below
# sample the same atlas positions as their subjects in exact arithmetic.
COLUMNS = (-4.65, -1.45, 1.75)
ROWS = (2.6, 1.75, 0.9, 0.05, -0.8, -1.65)
MIRROR = "Mirror ffi سلام 中文"
FLIPPED = "Flipped parent 中文 ffi"


def w_text(content, size, line, width, wrap="none"):
    return {"content": content, "font_size": size, "line_height": line, "width": width, "wrap": wrap}


# key -> (text, transform kwargs[, parent[, velocity]])
W_TEXTS = {
    # Row 0: scripts. Row 1: integer-pixel duplicate, rotation, bidi over the bright panel.
    "w_latin": (w_text("Incant ffi fl ½ 0123", 24, 30, 300), dict(at=(COLUMNS[0], ROWS[0], 0))),
    "w_arabic": (w_text("سلام عليكم يا أصدقاء", 24, 34, 300), dict(at=(COLUMNS[1], ROWS[0], 0))),
    "w_cjk": (w_text("日本語 中文 一 日本", 24, 30, 300), dict(at=(COLUMNS[2], ROWS[0], 0))),
    "w_latin_dup": (w_text("Incant ffi fl ½ 0123", 24, 30, 300), dict(at=(COLUMNS[0], ROWS[1], 0))),
    "w_rot": (w_text("Rotate 30° سلام", 24, 30, 160),
              dict(at=(COLUMNS[1] + 0.1, ROWS[1] - 0.55, 0), rotation=quat((0, 0, 1), 30))),
    "w_bidi": (w_text("Score ffi سلام 中文 42", 24, 30, 300), dict(at=(COLUMNS[2], ROWS[1], 0))),
    # Row 2: reflection by negative scale, its unmirrored control, and the same string seen from behind.
    "w_mirror": (w_text(MIRROR, 24, 30, 300), dict(at=(COLUMNS[0] + 3.0, ROWS[2], 0), scale=(-1, 1, 1))),
    "w_mirror_ctl": (w_text(MIRROR, 24, 30, 300), dict(at=(COLUMNS[1], ROWS[2], 0))),
    "w_back": (w_text(MIRROR, 24, 30, 300), dict(at=(4.75, ROWS[2], 0), rotation=quat((0, 1, 0), 180))),
    # Row 3: magnification, a child of a vertically flipped parent, and its unflipped control.
    "w_mag": (w_text("Ag سلام 語", 16, 22, 120), dict(at=(COLUMNS[0], ROWS[3], 0), scale=(2.5, 2.5, 2.5))),
    "w_child": (w_text(FLIPPED, 24, 30, 300), dict(at=(0, 0, 0)), "flip_parent"),
    "w_child_ctl": (w_text(FLIPPED, 24, 30, 300), dict(at=(COLUMNS[2], ROWS[3], 0))),
    # Row 4: minification ladder (2, 4 and 8.3 texels per pixel), yaw toward the camera, depth around the crate.
    "w_min50": (w_text("Minified 0.5 ffi سلام 中文 0123", 32, 40, 600),
                dict(at=(COLUMNS[0], ROWS[4], 0), scale=(0.5, 0.5, 0.5))),
    "w_min25": (w_text("Minified 0.25 ffi سلام 中文 0123", 48, 60, 1200),
                dict(at=(COLUMNS[0], ROWS[4] - 0.25, 0), scale=(0.25, 0.25, 0.25))),
    "w_min12": (w_text("Minified 0.12 ffi سلام 中文 0123", 64, 80, 2400),
                dict(at=(COLUMNS[0], ROWS[4] - 0.45, 0), scale=(0.12, 0.12, 0.12))),
    "w_yaw": (w_text("Yaw sixty ffi سلام 中文", 28, 34, 400),
              dict(at=(COLUMNS[1], ROWS[4] - 0.05, 0), rotation=quat((0, 1, 0), -60))),
    "w_behind": (w_text("Behind the crate ffi", 28, 34, 400), dict(at=(COLUMNS[2] + 0.1, ROWS[4] - 0.05, -0.6))),
    "w_front": (w_text("In front 中文", 24, 30, 300), dict(at=(COLUMNS[2] + 1.0, ROWS[4] - 0.35, 0.6))),
    # Row 5: word wrap, overlapping coverage, motion.
    "w_wrap": (w_text("Wrapped words keep order: يتم حفظ التقدم تلقائيا 日本語 中文 一", 18, 22, 300, "word"),
               dict(at=(COLUMNS[0], ROWS[5], 0))),
    "w_overlap_a": (w_text("Overlap ffi 中文", 28, 34, 300), dict(at=(COLUMNS[1], ROWS[5] - 0.05, 0))),
    "w_overlap_b": (w_text("Overlap ffi 中文", 28, 34, 300), dict(at=(COLUMNS[1] + 0.04, ROWS[5] - 0.08, 0))),
    "w_moving": (w_text("Moving سلام 中文", 24, 30, 300), dict(at=(COLUMNS[2], ROWS[5] - 0.1, 0)), None,
                 [0.6, 0, 0]),
}


def board_w(fonts, meshes, entity):
    commands = [
        entity("cam_ortho", "Orthographic camera", {
            "Transform": transform((0, 0, 10)),
            "Camera": {"fov_degrees": 45, "near": 0.1, "far": 100,
                       "projection": {"kind": "orthographic", "vertical_size": 5.4}}}),
        entity("cam_persp", "Perspective camera", {
            "Transform": transform((0, 0, 2.7 / math.tan(math.radians(22.5)))),
            "Camera": {"fov_degrees": 45, "near": 0.1, "far": 100, "projection": {"kind": "perspective"}}}),
        entity("light", "Key light", {"Transform": transform((0, 0, 0), KEY_LIGHT),
                                      "DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.4,
                                                           "shadows": None}}),
        entity("wall", "Dark wall", {"Transform": transform((0, 0, -1.2), scale=(10.4, 6.2, 0.2)),
                                     "MeshRenderer": {"mesh": meshes["slate"], "materials": [],
                                                      "cast_shadows": False}}),
        # Bright backdrop behind the CJK and bidi samples only (x 1.6..4.78, y 0.95..2.68).
        entity("panel", "Bright panel", {"Transform": transform((3.19, 1.815, -0.6), scale=(3.18, 1.73, 0.1)),
                                         "MeshRenderer": {"mesh": meshes["amber"], "materials": [],
                                                          "cast_shadows": False}}),
        entity("crate", "Occluding crate", {"Transform": transform((3.25, -1.1, 0), scale=(1.2, 0.7, 0.5)),
                                            "MeshRenderer": {"mesh": meshes["teal"], "materials": [],
                                                             "cast_shadows": False}}),
        # Children of this parent extend right and up from y = -0.6 (an upside-down reflection).
        entity("flip_parent", "Vertically flipped parent",
               {"Transform": transform((COLUMNS[1], -0.6, 0), scale=(1, -1, 1))}),
    ]
    for key, spec in W_TEXTS.items():
        text, place = spec[0], spec[1]
        parent = spec[2] if len(spec) > 2 else None
        velocity = spec[3] if len(spec) > 3 else None
        components = {"Transform": transform(**place), "Text": {**text, "fonts": fonts}}
        if velocity:
            components["Velocity"] = {"linear": velocity}
        commands.append(entity(key, key, components, parent))
    return commands


# ---------------------------------------------------------------- board U
U_IDS = ids({
    "camera": "40", "light": "41", "floor": "42", "crate": "43", "pillar": "44", "sign": "45",
    "root": "50", "title": "51", "body": "52", "actions": "53", "resume": "54", "options": "55",
    "online": "56", "continue": "57", "clip": "58", "clip_a": "59", "clip_b": "5A", "clip_c": "5B",
    "fields": "5C", "name_group": "5D", "name_label": "5E", "name": "5F", "long_group": "60",
    "long_label": "61", "long": "62", "arabic_group": "63", "arabic_label": "64", "arabic": "65",
    "cjk_group": "66", "cjk_label": "67", "cjk": "68",
})


def style(**values):
    """UiStyle with explicit defaults; lengths are logical pixels (as in 0056)."""
    base = {"size": ["auto", "auto"], "min_size": [0, 0], "max_size": [None, None], "padding": [0, 0, 0, 0],
            "gap": [0, 0], "direction": "column", "wrap": False, "align": "stretch", "justify": "start",
            "grow": 0, "shrink": 0, "basis": "auto"}
    base.update(values)
    return base


def board_u(fonts, meshes, entity):
    def text(content, size, line, wrap="word"):
        return {"content": content, "fonts": fonts, "font_size": size, "line_height": line, "wrap": wrap}

    def ui(key, name, kind, node_style, order, parent=None, disabled=False):
        return entity(key, name, {"UiNode": {"kind": kind, "style": node_style, "order": order,
                                             "disabled": disabled}}, parent)

    def button(key, label, order, parent, disabled=False):
        return ui(key, label, {"role": "button", "text": text(label, 16, 20)},
                  style(padding=[16, 9, 16, 9]), order, parent, disabled)

    def group(key, caption, value, order):
        return [
            ui(f"{key}_group", f"{key} group", {"role": "container"}, style(gap=[0, 4]), order, "fields"),
            ui(f"{key}_label", f"{key} caption", {"role": "label", "text": text(caption, 13, 18)},
               style(), 1, f"{key}_group"),
            ui(key, f"{key} field", {"role": "text_input", "text": text(value, 16, 22, "none")},
               style(size=[{"fraction": 1}, "auto"], padding=[10, 7, 10, 7]), 2, f"{key}_group"),
        ]

    pitch = quat((1, 0, 0), -8)
    return [
        entity("camera", "Camera", {"Transform": transform((0, 0.9, 7), pitch),
                                    "Camera": {"fov_degrees": 45, "near": 0.1, "far": 100,
                                               "projection": {"kind": "perspective"}}}),
        entity("light", "Key light", {"Transform": transform((0, 0, 0), KEY_LIGHT),
                                      "DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.4,
                                                           "shadows": None}}),
        entity("floor", "Floor", {"Transform": transform((0, -1.25, -2), scale=(16, 0.1, 12)),
                                  "MeshRenderer": {"mesh": meshes["floor"], "materials": [],
                                                   "cast_shadows": False}}),
        entity("crate", "Moving crate", {"Transform": transform((2.2, -0.6, 0.4)),
                                         "MeshRenderer": {"mesh": meshes["amber"], "materials": [],
                                                          "cast_shadows": False},
                                         "Velocity": {"linear": [-0.4, 0, 0]}}),
        entity("pillar", "Pillar", {"Transform": transform((2.6, 0.2, -2.5), scale=(1.6, 3, 0.9)),
                                    "MeshRenderer": {"mesh": meshes["teal"], "materials": [],
                                                     "cast_shadows": False}}),
        entity("sign", "World text sign", {"Transform": transform((1.9, 1.3, -2.0)),
                                           "Text": {"content": "World سلام 中文 ffi", "fonts": fonts,
                                                    "font_size": 28, "line_height": 34, "width": 400,
                                                    "wrap": "none"}}),
        ui("root", "UI root", {"role": "root"},
           style(padding=[32, 28, 32, 28], gap=[0, 14], align="start", shrink=1), 0),
        ui("title", "Title", {"role": "label", "text": text("Paused", 30, 36, "none")}, style(), 1, "root"),
        ui("body", "Body", {"role": "label", "text": text(
            "Progress is saved at every checkpoint. يتم حفظ التقدم تلقائيا عند كل نقطة. 日本語 中文", 16, 23)},
           style(size=[{"fraction": 1}, "auto"], max_size=[520, None]), 2, "root"),
        ui("actions", "Actions", {"role": "container"},
           style(size=[{"fraction": 1}, "auto"], max_size=[560, None], direction="row", wrap=True,
                 gap=[10, 10], align="start"), 3, "root"),
        button("resume", "Resume", 1, "actions"),
        button("options", "Options", 2, "actions"),
        button("online", "Online play", 3, "actions", disabled=True),
        button("continue", "متابعة", 4, "actions"),
        # A fixed 250 px strip: its third button overflows and is clipped by the strip's content box.
        ui("clip", "Clip strip", {"role": "container"},
           style(size=[{"pixels": 250}, "auto"], direction="row", gap=[10, 0], align="start"), 4, "root"),
        button("clip_a", "Alpha", 1, "clip"),
        button("clip_b", "Beta", 2, "clip"),
        button("clip_c", "Gamma 中文 سلام", 3, "clip"),
        ui("fields", "Fields", {"role": "container"},
           style(size=[{"fraction": 1}, "auto"], max_size=[360, None], gap=[0, 12]), 5, "root"),
        *group("name", "Player name", "Ada Lovelace", 1),
        *group("long", "Long value", "This field holds a long value that overflows its box ffi", 2),
        *group("arabic", "الاسم", "سلام عليكم ورحمة الله وبركاته يا أصدقاء الأعزاء في كل مكان وزمان", 3),
        *group("cjk", "日本語", "日本", 4),
    ]


BOARDS = {"W": (W_IDS, board_w), "U": (U_IDS, board_u)}


def describe():
    return {"W": {"ids": W_IDS, "texts": {k: v[0] for k, v in W_TEXTS.items()}}, "U": {"ids": U_IDS}}


if __name__ == "__main__":
    print(json.dumps(describe(), indent=1, ensure_ascii=False))
