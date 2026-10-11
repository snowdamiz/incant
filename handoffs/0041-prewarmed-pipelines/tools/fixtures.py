"""Original fixture art, glTF sources and scene specs for handoff 0041.

Standard library only. Every image and model here is authored by this file; no
third-party asset is copied. Reused patterns, identified by origin:

  * PNG encode/decode: handoffs/0032-sprite-rendering/tools/pngio.py, imported in
    place (sha256 5b73bef8...9e86 at commit 716ccc8), unmodified.
  * glTF writer, closed box and engine call shape: adapted from
    handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py (write_gltf, box).
  * ULID-shaped entity IDs: same scheme as 0030/0032 (fixed prefix + counter).

Four disposable projects exercise every pipeline variant the candidate prepares:
materials (3 alpha modes x 2 culling choices x mirrored placement, normal-map
orientation, depth/blend order, emissive tone ramp), shadows (opaque and masked
casters x both culling choices x mirrored, facing toward and away from the sun,
plus BLEND non-casters), lights (point/spot, HDR shoulder) and sprites
(ordinary/atlas, nine-slice, flips, reflection, translucency, sprite/model depth).
"""

import json
import math
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # never write caches into handoff 0032
TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import pngio  # noqa: E402  (reused unmodified from handoff 0032)

GENERATOR = "handoff 0041 fixtures.py (original fixture art)"

# ------------------------------------------------------------------ math --


def norm(v):
    length = math.sqrt(sum(c * c for c in v))
    return tuple(c / length for c in v)


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def quat_axis(axis, degrees):
    s = math.sin(math.radians(degrees) / 2)
    return [axis[0] * s, axis[1] * s, axis[2] * s, math.cos(math.radians(degrees) / 2)]


def quat_from_basis(x, y, z):
    """Unit quaternion [x, y, z, w] whose rotation matrix has columns x, y, z."""
    m00, m01, m02 = x[0], y[0], z[0]
    m10, m11, m12 = x[1], y[1], z[1]
    m20, m21, m22 = x[2], y[2], z[2]
    trace = m00 + m11 + m22
    if trace > 0:
        s = math.sqrt(trace + 1.0) * 2
        q = [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s]
    elif m00 > m11 and m00 > m22:
        s = math.sqrt(1.0 + m00 - m11 - m22) * 2
        q = [0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    elif m11 > m22:
        s = math.sqrt(1.0 + m11 - m00 - m22) * 2
        q = [(m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s]
    else:
        s = math.sqrt(1.0 + m22 - m00 - m11) * 2
        q = [(m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s]
    n = math.sqrt(sum(c * c for c in q))
    return [c / n for c in q]


def look_rotation(forward, up=(0.0, 1.0, 0.0)):
    """Rotation whose local -Z points along `forward` (cameras and lights)."""
    z = norm(tuple(-c for c in forward))
    x = norm(cross(up, z))
    y = cross(z, x)
    return quat_from_basis(x, y, z)


def rotate(q, v):
    x, y, z, w = q
    u = (x, y, z)
    t = tuple(2 * c for c in cross(u, v))
    return tuple(v[i] + w * t[i] + cross(u, t)[i] for i in range(3))


def transform(translation, rotation=(0.0, 0.0, 0.0, 1.0), scale=(1.0, 1.0, 1.0)):
    return {"translation": [round(v, 9) for v in translation], "rotation": [round(v, 12) for v in rotation],
            "scale": [float(s) for s in scale]}


def srgb_to_linear(byte):
    v = byte / 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def tint(rgb, alpha=1.0):
    return [round(srgb_to_linear(c), 9) for c in rgb] + [alpha]


class Ids:
    """Canonical ULID-shaped IDs: fixed prefix plus a Crockford base32 counter."""

    ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

    def __init__(self, prefix):
        assert len(prefix) == 10 and all(c in self.ALPHABET for c in prefix)
        self.prefix, self.counter = prefix, 0

    def next(self):
        self.counter += 1
        n, suffix = self.counter, ""
        for _ in range(16):
            suffix = self.ALPHABET[n % 32] + suffix
            n //= 32
        return self.prefix + suffix


# ------------------------------------------------------------- projection --

class View:
    """Engine camera model: local -Z forward, +Y up, vertical FOV, NDC y up."""

    def __init__(self, name, eye, rotation, fov=None, ortho_half_height=None, size=(960, 540), near=0.1, far=80.0,
                 pixel=None):
        self.name, self.eye, self.rotation, self.fov = name, tuple(eye), list(rotation), fov
        self.ortho, self.size, self.near, self.far, self.pixel = ortho_half_height, size, near, far, pixel

    def camera_component(self):
        value = {"fov_degrees": self.fov or 40.0, "near": self.near, "far": self.far}
        if self.pixel:
            value["projection"] = {"kind": "pixel_perfect", "pixels_per_unit": self.pixel[0],
                                   "reference_resolution": list(self.pixel[1])}
        elif self.ortho:
            value["projection"] = {"kind": "orthographic", "vertical_size": 2 * self.ortho}
        else:
            value["projection"] = {"kind": "perspective"}
        return value

    def project(self, p):
        q = self.rotation
        inverse = [-q[0], -q[1], -q[2], q[3]]
        v = rotate(inverse, sub(p, self.eye))
        w, h = self.size
        aspect = w / h
        if self.ortho:
            half = self.ortho
            x_ndc, y_ndc = v[0] / (half * aspect), v[1] / half
        else:
            if v[2] >= -self.near:
                return None
            t = math.tan(math.radians(self.fov) / 2)
            x_ndc, y_ndc = v[0] / (-v[2] * t * aspect), v[1] / (-v[2] * t)
        return ((x_ndc + 1) * w / 2, (1 - y_ndc) * h / 2, -v[2])


# ------------------------------------------------------------------ art --

PAPER = (226, 208, 172, 255)
INK = (52, 60, 84, 255)
TEAL_HOLE = (40, 170, 160, 0)
STRIPE_RGB = (190, 40, 50)
STRIPE_ALPHA = (51, 102, 153, 204)  # 0.2, 0.4, 0.6, 0.8
DOME_CENTER, DOME_RADIUS = (48.0, 16.0), 13.0


def f_glyph(x, y):
    """Bold F in the card's upper-left quadrant (texel coordinates, y down)."""
    return (8 <= x <= 12 and 6 <= y <= 27) or (8 <= x <= 25 and 6 <= y <= 10) or (8 <= x <= 21 and 15 <= y <= 18)


def triangle_hole(x, y):
    """Right triangle, right angle at the bottom-left of the lower-left quadrant."""
    cx, cy = x + 0.5, y + 0.5
    return 6 <= cx <= 26 and 38 <= cy <= 58 and (cx - 6) <= (cy - 38)


def stripe(x, y):
    if not 36 <= y <= 60:
        return None
    for k, x0 in enumerate((34, 41, 48, 55)):
        if x0 <= x <= x0 + 6:
            return k
    return None


def card_base_pixels(glyph_only=False):
    pixels = []
    for y in range(64):
        for x in range(64):
            if f_glyph(x, y):
                pixels.append(INK)
            elif not glyph_only and triangle_hole(x, y):
                pixels.append(TEAL_HOLE)
            elif not glyph_only and stripe(x, y) is not None:
                pixels.append((*STRIPE_RGB, STRIPE_ALPHA[stripe(x, y)]))
            else:
                pixels.append(PAPER)
    return pixels


def dome_normal(x, y):
    """glTF tangent-space normal (+X right, +Y up the image) of a raised dome."""
    cx, cy = x + 0.5, y + 0.5
    dx, dy = cx - DOME_CENTER[0], cy - DOME_CENTER[1]
    r2 = (dx * dx + dy * dy) / (DOME_RADIUS * DOME_RADIUS)
    if r2 >= 1:
        return (0.0, 0.0, 1.0)
    height = 0.65 * DOME_RADIUS
    # h = H (1 - r2)^2; image y grows downward, so +Y (up) = -image y.
    common = height * 2 * (1 - r2) * (-2) / (DOME_RADIUS * DOME_RADIUS)
    hx, hy_image = common * dx, common * dy
    return norm((-hx, hy_image, 1.0))


def card_normal_pixels():
    out = []
    for y in range(64):
        for x in range(64):
            n = dome_normal(x, y)
            out.append(tuple(max(0, min(255, round(127.5 + 127.5 * c))) for c in n) + (255,))
    return out


def uniform_pixels(value):
    return [(value, value, value, 255)] * 2


def glyph_mask(kind, x, y):
    if kind == "F":
        return (3 <= x <= 5 and 2 <= y <= 13) or (3 <= x <= 12 and 2 <= y <= 4) or (3 <= x <= 10 and 7 <= y <= 9)
    # R: stem, bowl and diagonal leg.
    if 3 <= x <= 5 and 2 <= y <= 13:
        return True
    if 3 <= x <= 10 and 2 <= y <= 3:
        return True
    if 3 <= x <= 10 and 7 <= y <= 8:
        return True
    if 10 <= x <= 12 and 3 <= y <= 7:
        return True
    if 9 <= y <= 13:
        lead = 6 + (y - 9) * 1.3
        return lead <= x <= lead + 2.2
    return False


def outlined_glyph(kind, fill, outline):
    rows = []
    for y in range(16):
        for x in range(16):
            if glyph_mask(kind, x, y):
                rows.append(fill)
            elif any(glyph_mask(kind, x + i, y + j) for i in (-1, 0, 1) for j in (-1, 0, 1)
                     if 0 <= x + i < 16 and 0 <= y + j < 16):
                rows.append(outline)
            else:
                rows.append((0, 0, 0, 0))
    return rows


def panel_pixels(corners, edge, line, centre, border=5, size=24):
    out = []
    for y in range(size):
        for x in range(size):
            left, right, top, bottom = x < border, x >= size - border, y < border, y >= size - border
            if x in (0, size - 1) or y in (0, size - 1):
                out.append((24, 24, 30, 255))
            elif top and left:
                out.append(corners[0])
            elif top and right:
                out.append(corners[1])
            elif bottom and left:
                out.append(corners[2])
            elif bottom and right:
                out.append(corners[3])
            elif top or bottom:
                out.append(line if y in (2, size - 3) else edge)
            elif left or right:
                out.append(line if x in (2, size - 3) else edge)
            else:
                out.append(centre)
    return out


def orb_pixels():
    out = []
    for y in range(16):
        for x in range(16):
            r = math.hypot(x + 0.5 - 8, y + 0.5 - 8) / 8
            a = max(0.0, 1 - r * r)
            out.append((255, 226, 140, round(255 * a)))
    return out


def sheet_pixels():
    """64 x 32 ordinary sheet: F glyph, panel A, orb, solid white."""
    canvas = [[(0, 0, 0, 0)] * 64 for _ in range(32)]

    def blit(x0, y0, w, pixels):
        for i, p in enumerate(pixels):
            canvas[y0 + i // w][x0 + i % w] = p

    blit(0, 0, 16, outlined_glyph("F", (240, 150, 40, 255), (40, 28, 20, 255)))
    blit(16, 0, 24, panel_pixels(((220, 60, 60, 255), (60, 190, 90, 255), (70, 110, 230, 255), (235, 200, 60, 255)),
                                 (200, 200, 210, 255), (90, 90, 100, 255), (60, 90, 100, 255)))
    blit(40, 0, 16, orb_pixels())
    blit(56, 0, 4, [(255, 255, 255, 255)] * 16)
    return [p for row in canvas for p in row]


SHEET_REGIONS = {"glyph": [0, 0, 16, 16], "panel": [16, 0, 24, 24], "orb": [40, 0, 16, 16], "solid": [56, 0, 4, 4]}
ATLAS_ID = "01JA41PWPA0000000000000001"
FRAME_FLAG = "01JA41PWPF0000000000000001"
FRAME_PANEL = "01JA41PWPF0000000000000002"


def atlas_descriptor():
    return {"version": 1, "id": ATLAS_ID, "maximum_size": [64, 64], "padding": 2,
            "frames": {FRAME_FLAG: {"name": "flag", "path": "flag.png"},
                       FRAME_PANEL: {"name": "panel", "path": "panel_b.png"}}}


# ----------------------------------------------------------------- glTF --

def card_quad(size, tangent_w=None):
    h = size / 2
    positions = [(-h, -h, 0.0), (h, -h, 0.0), (h, h, 0.0), (-h, h, 0.0)]
    prim = {"positions": positions, "normals": [(0.0, 0.0, 1.0)] * 4,
            "uvs": [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)], "indices": [0, 1, 2, 0, 2, 3]}
    if tangent_w is not None:
        prim["tangents"] = [(1.0, 0.0, 0.0, float(tangent_w))] * 4
    return prim


def rect_xy(x0, x1, y0, y1, z):
    return {"positions": [(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)], "normals": [(0.0, 0.0, 1.0)] * 4,
            "indices": [0, 1, 2, 0, 2, 3]}


def merge(prims):
    out = {"positions": [], "normals": [], "indices": []}
    for p in prims:
        base = len(out["positions"])
        out["positions"] += p["positions"]
        out["normals"] += p["normals"]
        out["indices"] += [base + i for i in p["indices"]]
    return out


def floor_xz(x0, x1, z0, z1, y=0.0):
    return {"positions": [(x0, y, z1), (x1, y, z1), (x1, y, z0), (x0, y, z0)], "normals": [(0.0, 1.0, 0.0)] * 4,
            "indices": [0, 1, 2, 0, 2, 3]}


def box(x0, x1, y0, y1, z0, z1):
    """Closed box with outward counter-clockwise faces (from 0030 ortho_lookdev.py)."""
    positions, normals, indices = [], [], []
    c = ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2)
    half = ((x1 - x0) / 2, (y1 - y0) / 2, (z1 - z0) / 2)
    for n, u in (((1, 0, 0), (0, 1, 0)), ((-1, 0, 0), (0, 1, 0)), ((0, 1, 0), (0, 0, 1)),
                 ((0, -1, 0), (0, 0, 1)), ((0, 0, 1), (0, 1, 0)), ((0, 0, -1), (0, 1, 0))):
        v = cross(n, u)
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple(c[i] + (n[i] + su * u[i] + sv * v[i]) * half[i] for i in range(3)))
            normals.append(tuple(float(a) for a in n))
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return {"positions": positions, "normals": normals, "indices": indices}


def sphere(radius, centre=(0.0, 0.0, 0.0), rings=32, segments=64):
    positions, normals, indices = [], [], []
    for i in range(rings + 1):
        theta = math.pi * i / rings
        for j in range(segments + 1):
            phi = 2 * math.pi * j / segments
            n = (math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi))
            normals.append(n)
            positions.append(tuple(centre[k] + radius * n[k] for k in range(3)))
    for i in range(rings):
        for j in range(segments):
            a, b = i * (segments + 1) + j, (i + 1) * (segments + 1) + j
            for tri in ((a, b, a + 1), (a + 1, b, b + 1)):
                p0, p1, p2 = (positions[t] for t in tri)
                if sum(x * x for x in cross(sub(p1, p0), sub(p2, p0))) < 1e-18:
                    continue  # pole slivers
                outward = dot(cross(sub(p1, p0), sub(p2, p0)), sub(p0, centre))
                indices += list(tri) if outward > 0 else [tri[0], tri[2], tri[1]]
    return {"positions": positions, "normals": normals, "indices": indices}


def write_gltf(directory, name, primitives, materials, images=(), sampler=None):
    """primitives: dicts with positions, normals, indices, material and optional uvs/tangents."""
    blob, views, accessors, prims = b"", [], [], []

    def add(data, target, accessor):
        nonlocal blob
        views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(data), "target": target})
        blob += data
        accessor["bufferView"] = len(views) - 1
        accessors.append(accessor)
        return len(accessors) - 1

    for p in primitives:
        pos = p["positions"]
        attributes = {
            "POSITION": add(b"".join(struct.pack("<3f", *v) for v in pos), 34962,
                            {"componentType": 5126, "count": len(pos), "type": "VEC3",
                             "min": [min(v[i] for v in pos) for i in range(3)],
                             "max": [max(v[i] for v in pos) for i in range(3)]}),
            "NORMAL": add(b"".join(struct.pack("<3f", *v) for v in p["normals"]), 34962,
                          {"componentType": 5126, "count": len(pos), "type": "VEC3"}),
        }
        if "uvs" in p:
            attributes["TEXCOORD_0"] = add(b"".join(struct.pack("<2f", *v) for v in p["uvs"]), 34962,
                                           {"componentType": 5126, "count": len(pos), "type": "VEC2"})
        if "tangents" in p:
            attributes["TANGENT"] = add(b"".join(struct.pack("<4f", *v) for v in p["tangents"]), 34962,
                                        {"componentType": 5126, "count": len(pos), "type": "VEC4"})
        index = add(b"".join(struct.pack("<I", i) for i in p["indices"]), 34963,
                    {"componentType": 5125, "count": len(p["indices"]), "type": "SCALAR"})
        prims.append({"attributes": attributes, "indices": index, "material": p["material"]})
    (directory / f"{name}.bin").write_bytes(blob)
    document = {"asset": {"version": "2.0", "generator": GENERATOR},
                "buffers": [{"uri": f"{name}.bin", "byteLength": len(blob)}],
                "bufferViews": views, "accessors": accessors,
                "meshes": [{"name": name, "primitives": prims}], "materials": materials,
                "nodes": [{"name": name, "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0}
    if images:
        document["images"] = [{"uri": uri} for uri in images]
        document["samplers"] = [sampler or {"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071}]
        document["textures"] = [{"source": i, "sampler": 0} for i in range(len(images))]
    path = directory / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1), encoding="utf-8")
    return path


def card_material(alpha, double_sided, *, normal=True, factor=(1.0, 1.0, 1.0, 1.0), cutoff=0.5, name=None):
    material = {"name": name or f"card {alpha.lower()} {'double' if double_sided else 'single'}",
                "pbrMetallicRoughness": {"baseColorFactor": list(factor), "baseColorTexture": {"index": 0},
                                         "metallicFactor": 0.0, "roughnessFactor": 0.65},
                "alphaMode": alpha, "doubleSided": double_sided}
    if normal:
        material["normalTexture"] = {"index": 1, "scale": 1.0}
    if alpha == "MASK":
        material["alphaCutoff"] = cutoff
    return material


def flat_material(name, rgb, roughness=0.8, metallic=0.0, alpha=None, emissive=None):
    m = {"name": name, "pbrMetallicRoughness": {"baseColorFactor": [*rgb, 1.0 if alpha is None else alpha],
                                                "metallicFactor": metallic, "roughnessFactor": roughness}}
    if alpha is not None:
        m["alphaMode"] = "BLEND"
    if emissive is not None:
        m["emissiveFactor"] = list(emissive)
    return m


def with_material(prim, index):
    return {**prim, "material": index}


# ------------------------------------------------------------- projects --
# Each spec returns sources to write, imports to run, a scene builder and views.

ALPHAS = ("OPAQUE", "MASK", "BLEND")
SIDED = (False, True)
KEY_L = norm((-0.55, 0.6, 0.58))  # toward the materials key light: upper left, in front


class Project:
    def __init__(self, name, prefix):
        self.name, self.ids, self.entities, self.names = name, Ids(prefix), [], {}
        self.views, self.meta = {}, {}

    def entity(self, name, components, parent=None):
        eid = self.ids.next()
        assert name not in self.names, name
        self.entities.append({"id": eid, "name": name, "parent": self.names[parent] if parent else None,
                              "components": components})
        self.names[name] = eid
        return eid

    def camera(self, view):
        self.views[view.name] = view
        self.entity(f"Camera {view.name}", {"Camera": view.camera_component(),
                                             "Transform": transform(view.eye, view.rotation)})


def mesh(asset, shadows=False):
    return {"mesh": asset, "materials": [], "cast_shadows": shadows}


# --- materials ---------------------------------------------------------

M_CARD = 1.5
M_CELLS = {"single": (-5.85, False, False), "single-mirrored": (-1.95, False, True),
           "double": (1.95, True, False), "double-mirrored": (5.85, True, True)}
M_ROWS = {"OPAQUE": 2.0, "MASK": 0.0, "BLEND": -2.0}
M_FRONT_DX, M_BACK_DX = -0.85, 0.85
M_GLASS = (-7.7, 7.7, -2.25, -1.45, 0.15)
M_BAR = (-7.7, 7.7, -2.55, -2.15, 0.30)
M_SWATCHES = [("white-0.05", (0.05,) * 3), ("white-0.2", (0.2,) * 3), ("white-0.5", (0.5,) * 3),
              ("white-0.8", (0.8,) * 3), ("white-1.0", (1.0,) * 3), ("red-1.0", (1.0, 0.0, 0.0)),
              ("green-1.0", (0.0, 1.0, 0.0)), ("blue-1.0", (0.0, 0.0, 1.0))]
M_SWATCH_Y, M_SWATCH_H, M_SWATCH_W, M_SWATCH_PITCH = -3.4, 0.6, 1.6, 1.8
N_OFFSET, N_CARD = (60.0, 0.0), 2.6
N_SLOTS = {"generated": (-3.0, 1.5), "tangent+1": (0.0, 1.5), "tangent-1": (3.0, 1.5),
           "generated-mirrored": (-3.0, -1.5), "generated-double-back": (0.0, -1.5), "tangent+1-mirrored": (3.0, -1.5)}


def card_model_name(alpha, double_sided):
    return f"card_{alpha.lower()}_{'double' if double_sided else 'single'}"


def materials_sources(directory, reimport=False):
    """Write materials sources. With reimport=True, only the changed card is rewritten."""
    if reimport:
        # Accepted material change: OPAQUE single becomes a tinted, double-sided MASK
        # with a lower cutoff, so the 0.4 stripe survives and back faces appear.
        material = card_material("MASK", True, factor=(0.62, 0.82, 1.0, 1.0), cutoff=0.3,
                                 name="card opaque single (reimported as tinted double-sided mask 0.3)")
        return [write_gltf(directory, card_model_name("OPAQUE", False),
                           [with_material(card_quad(M_CARD), 0)], [material], ["card_base.png", "card_normal.png"])]
    pngio.write_rgba(directory / "card_base.png", 64, 64, card_base_pixels())
    pngio.write_rgba(directory / "card_normal.png", 64, 64, card_normal_pixels())
    pngio.write_rgba(directory / "nrm_base.png", 64, 64, card_base_pixels(glyph_only=True))
    pngio.write_rgba(directory / "env_uniform.png", 2, 1, uniform_pixels(64))
    written = []
    for alpha in ALPHAS:
        for double_sided in SIDED:
            written.append(write_gltf(directory, card_model_name(alpha, double_sided),
                                      [with_material(card_quad(M_CARD), 0)], [card_material(alpha, double_sided)],
                                      ["card_base.png", "card_normal.png"]))
    nrm = [("nrm_generated", None, False), ("nrm_generated_double", None, True),
           ("nrm_tangent_pos", 1.0, False), ("nrm_tangent_neg", -1.0, False)]
    for name, w, double_sided in nrm:
        written.append(write_gltf(directory, name, [with_material(card_quad(N_CARD, w), 0)],
                                  [card_material("OPAQUE", double_sided, name=name)],
                                  ["nrm_base.png", "card_normal.png"]))
    stripes_a, stripes_b = [], []
    for k in range(40):
        x0 = -10.0 + 0.5 * k
        (stripes_a if k % 2 == 0 else stripes_b).append(rect_xy(x0, x0 + 0.5, -6.0, 5.0, -1.5))
    swatches = []
    for k, (_, _) in enumerate(M_SWATCHES):
        cx = (k - 3.5) * M_SWATCH_PITCH
        swatches.append(rect_xy(cx - M_SWATCH_W / 2, cx + M_SWATCH_W / 2, M_SWATCH_Y - M_SWATCH_H / 2,
                                M_SWATCH_Y + M_SWATCH_H / 2, 0.0))
    gx0, gx1, gy0, gy1, gz = M_GLASS
    bx0, bx1, by0, by1, bz = M_BAR
    prims = [with_material(merge(stripes_a), 0), with_material(merge(stripes_b), 1),
             with_material(rect_xy(bx0, bx1, by0, by1, bz), 2), with_material(rect_xy(gx0, gx1, gy0, gy1, gz), 3)]
    materials = [flat_material("backdrop dark", (0.06, 0.07, 0.09), 0.9),
                 flat_material("backdrop light", (0.11, 0.12, 0.15), 0.9),
                 flat_material("front bar", (0.12, 0.30, 0.16), 0.5),
                 flat_material("glass strip", (0.25, 0.5, 0.95), 0.25, alpha=0.45)]
    for k, (label, emissive) in enumerate(M_SWATCHES):
        prims.append(with_material(swatches[k], len(materials)))
        materials.append(flat_material(f"swatch {label}", (0.0, 0.0, 0.0), 1.0, emissive=emissive))
    written.append(write_gltf(directory, "props", prims, materials))
    return written


def materials_imports():
    imports = [("env_uniform.png", "linear")]
    imports += [(f"{card_model_name(a, d)}.gltf", None) for a in ALPHAS for d in SIDED]
    imports += [(f"{n}.gltf", None) for n in ("nrm_generated", "nrm_generated_double", "nrm_tangent_pos",
                                             "nrm_tangent_neg", "props")]
    return imports


def materials_scene(assets):
    p = Project("materials", "01JA41PWM0")
    key = look_rotation(tuple(-c for c in KEY_L))
    p.entity("Key light", {"DirectionalLight": {"color": [1.0, 1.0, 1.0], "intensity": 3.0},
                           "Transform": transform((0.0, 6.0, 6.0), key)})
    p.entity("Uniform environment", {"EnvironmentLight": {"texture": assets["env_uniform.png"], "intensity": 1.0,
                                                          "rotation_degrees": 0.0}})
    p.entity("Props", {"MeshRenderer": mesh(assets["props.gltf"]), "Transform": transform((0.0, 0.0, 0.0))})
    back = quat_axis((0, 1, 0), 180)
    for alpha, y in M_ROWS.items():
        for cell, (cx, double_sided, mirrored) in M_CELLS.items():
            asset = assets[f"{card_model_name(alpha, double_sided)}.gltf"]
            scale = (-1.0, 1.0, 1.0) if mirrored else (1.0, 1.0, 1.0)
            for facing, dx, rotation in (("front", M_FRONT_DX, (0.0, 0.0, 0.0, 1.0)), ("back", M_BACK_DX, back)):
                name = f"{alpha} {cell} {facing}"
                p.entity(name, {"MeshRenderer": mesh(asset), "Transform": transform((cx + dx, y, 0.0), rotation,
                                                                                     scale)})
                p.meta[name] = {"centre": [cx + dx, y, 0.0], "size": M_CARD, "alpha": alpha,
                                "double_sided": double_sided, "mirrored": mirrored, "facing": facing}
    ox, oy = N_OFFSET
    models = {"generated": "nrm_generated", "tangent+1": "nrm_tangent_pos", "tangent-1": "nrm_tangent_neg",
              "generated-mirrored": "nrm_generated", "generated-double-back": "nrm_generated_double",
              "tangent+1-mirrored": "nrm_tangent_pos"}
    for slot, (x, y) in N_SLOTS.items():
        rotation = back if slot.endswith("back") else (0.0, 0.0, 0.0, 1.0)
        scale = (-1.0, 1.0, 1.0) if slot.endswith("mirrored") else (1.0, 1.0, 1.0)
        name = f"Normals {slot}"
        p.entity(name, {"MeshRenderer": mesh(assets[f"{models[slot]}.gltf"]),
                        "Transform": transform((ox + x, oy + y, 0.0), rotation, scale)})
        p.meta[name] = {"centre": [ox + x, oy + y, 0.0], "size": N_CARD, "model": models[slot]}
    p.camera(View("board", (0.0, -0.45, 14.5), [0.0, 0.0, 0.0, 1.0], fov=34.0, far=60.0))
    p.camera(View("normals", (ox, oy, 12.0), [0.0, 0.0, 0.0, 1.0], fov=34.0, far=60.0))
    return p


# --- shadows -----------------------------------------------------------

S_CARD, S_HEIGHT = 1.2, 1.4
S_COLUMNS = {"single": (-6.0, False, False), "single-mirrored": (-2.0, False, True),
             "double": (2.0, True, False), "double-mirrored": (6.0, True, True)}
S_ROWS = {"OPAQUE up": (3.0, "OPAQUE", "up"), "OPAQUE down": (0.6, "OPAQUE", "down"),
          "MASK up": (-1.8, "MASK", "up"), "MASK down": (-4.2, "MASK", "down")}
S_BLEND = {"BLEND single up (non-caster)": (-2.0, 5.6, False), "BLEND double up (non-caster)": (2.0, 5.6, True)}
SUN_L = norm((-0.72, 0.643, 0.262))  # toward the sun: left, 40 degrees up, slightly in front
S_FLOOR = (-21.0, 23.0, -19.0, 9.0)


def shadows_sources(directory):
    pngio.write_rgba(directory / "card_base.png", 64, 64, card_base_pixels())
    pngio.write_rgba(directory / "env_uniform.png", 2, 1, uniform_pixels(51))
    written = []
    for alpha in ALPHAS:
        for double_sided in SIDED:
            written.append(write_gltf(directory, card_model_name(alpha, double_sided),
                                      [with_material(card_quad(S_CARD), 0)],
                                      [card_material(alpha, double_sided, normal=False)], ["card_base.png"]))
    x0, x1, z0, z1 = S_FLOOR
    written.append(write_gltf(directory, "floor", [with_material(floor_xz(x0, x1, z0, z1), 0)],
                              [flat_material("floor", (0.55, 0.55, 0.55), 0.9)]))
    return written


def shadows_imports():
    return [("env_uniform.png", "linear")] + [(f"{card_model_name(a, d)}.gltf", None) for a in ALPHAS
                                              for d in SIDED] + [("floor.gltf", None)]


def shadows_scene(assets):
    p = Project("shadows", "01JA41PWS0")
    p.entity("Sun", {"DirectionalLight": {"color": [1.0, 1.0, 1.0], "intensity": 3.0, "shadows": {"distance": 30.0}},
                     "Transform": transform((0.0, 8.0, 0.0), look_rotation(tuple(-c for c in SUN_L)))})
    p.entity("Uniform environment", {"EnvironmentLight": {"texture": assets["env_uniform.png"], "intensity": 1.0,
                                                          "rotation_degrees": 0.0}})
    p.entity("Floor", {"MeshRenderer": mesh(assets["floor.gltf"]), "Transform": transform((0.0, 0.0, 0.0))})
    up, down = quat_axis((1, 0, 0), -90), quat_axis((1, 0, 0), 90)
    for row, (z, alpha, facing) in S_ROWS.items():
        for column, (x, double_sided, mirrored) in S_COLUMNS.items():
            name = f"{row} {column}"
            p.entity(name, {"MeshRenderer": mesh(assets[f"{card_model_name(alpha, double_sided)}.gltf"], True),
                            "Transform": transform((x, S_HEIGHT, z), up if facing == "up" else down,
                                                   (-1.0, 1.0, 1.0) if mirrored else (1.0, 1.0, 1.0))})
            p.meta[name] = {"centre": [x, S_HEIGHT, z], "alpha": alpha, "facing": facing,
                            "double_sided": double_sided, "mirrored": mirrored, "casts": True}
    for name, (x, z, double_sided) in S_BLEND.items():
        p.entity(name, {"MeshRenderer": mesh(assets[f"{card_model_name('BLEND', double_sided)}.gltf"], False),
                        "Transform": transform((x, S_HEIGHT, z), up)})
        p.meta[name] = {"centre": [x, S_HEIGHT, z], "alpha": "BLEND", "facing": "up", "double_sided": double_sided,
                        "mirrored": False, "casts": False}
    eye, target = (1.0, 12.0, 13.5), (1.0, 0.0, 0.0)
    p.camera(View("board", eye, look_rotation(norm(sub(target, eye))), fov=40.0, far=80.0))
    return p


def shadow_offset():
    """Floor displacement of a shadow cast from height S_HEIGHT."""
    return (-S_HEIGHT * SUN_L[0] / SUN_L[1], -S_HEIGHT * SUN_L[2] / SUN_L[1])


# --- lights ------------------------------------------------------------

L_ROOM = (-8.0, 8.0, -4.0, 6.0, 6.0)
L_OBJECTS = {"white sphere": (-3.6, 0.75, 0.3), "metal sphere": (0.0, 0.75, 0.3), "red cube": (3.6, 0.55, 0.3)}
L_LIGHTS = {
    "Warm point": ("PointLight", (-4.8, 2.0, 1.6), None,
                   {"color": [1.0, 0.72, 0.42], "intensity": 14.0, "range": 8.0}),
    "Cool point": ("PointLight", (-0.9, 2.1, 1.8), None,
                   {"color": [0.42, 0.62, 1.0], "intensity": 14.0, "range": 8.0}),
    "White spot": ("SpotLight", (3.6, 4.2, 1.8), (3.6, 0.0, 0.6),
                   {"color": [1.0, 1.0, 1.0], "intensity": 220.0, "range": 12.0, "inner_degrees": 16.0,
                    "outer_degrees": 24.0}),
    "Magenta spot": ("SpotLight", (-2.5, 4.5, 3.0), (-4.5, 3.2, -4.0),
                     {"color": [0.95, 0.3, 0.85], "intensity": 160.0, "range": 14.0, "inner_degrees": 7.0,
                      "outer_degrees": 12.0}),
}


def lights_sources(directory):
    pngio.write_rgba(directory / "env_uniform.png", 2, 1, uniform_pixels(10))
    x0, x1, z0, z1, wall = L_ROOM
    written = [write_gltf(directory, "room", [with_material(floor_xz(x0, x1, z0, z1), 0),
                                              with_material(rect_xy(x0, x1, 0.0, wall, z0), 0)],
                          [flat_material("room", (0.5, 0.5, 0.5), 0.85)])]
    written.append(write_gltf(directory, "sphere_white", [with_material(sphere(0.75), 0)],
                              [flat_material("white", (0.8, 0.8, 0.8), 0.35)]))
    written.append(write_gltf(directory, "sphere_metal", [with_material(sphere(0.75), 0)],
                              [flat_material("metal", (0.92, 0.9, 0.86), 0.28, metallic=1.0)]))
    written.append(write_gltf(directory, "cube_red", [with_material(box(-0.55, 0.55, -0.55, 0.55, -0.55, 0.55), 0)],
                              [flat_material("red", (0.7, 0.08, 0.06), 0.55)]))
    return written


def lights_imports():
    return [("env_uniform.png", "linear"), ("room.gltf", None), ("sphere_white.gltf", None),
            ("sphere_metal.gltf", None), ("cube_red.gltf", None)]


def lights_scene(assets):
    p = Project("lights", "01JA41PWG0")
    p.entity("Dim environment", {"EnvironmentLight": {"texture": assets["env_uniform.png"], "intensity": 1.0,
                                                      "rotation_degrees": 0.0}})
    p.entity("Room", {"MeshRenderer": mesh(assets["room.gltf"]), "Transform": transform((0.0, 0.0, 0.0))})
    models = {"white sphere": "sphere_white.gltf", "metal sphere": "sphere_metal.gltf", "red cube": "cube_red.gltf"}
    for name, at in L_OBJECTS.items():
        rotation = quat_axis((0, 1, 0), 35) if name == "red cube" else (0.0, 0.0, 0.0, 1.0)
        p.entity(name, {"MeshRenderer": mesh(assets[models[name]]), "Transform": transform(at, rotation)})
    for name, (kind, at, target, value) in L_LIGHTS.items():
        rotation = look_rotation(norm(sub(target, at))) if target else (0.0, 0.0, 0.0, 1.0)
        p.entity(name, {kind: value, "Transform": transform(at, rotation)})
    eye, target = (0.0, 2.8, 9.0), (0.0, 1.1, -0.5)
    p.camera(View("board", eye, look_rotation(norm(sub(target, eye))), fov=46.0, far=60.0))
    return p


# --- sprites -----------------------------------------------------------

PPU, LOGICAL = 16, (320, 180)


def wx(px):
    return -LOGICAL[0] / PPU / 2 + px / PPU


def wy(py):
    return LOGICAL[1] / PPU / 2 - py / PPU


def sprites_sources(directory):
    pngio.write_rgba(directory / "sheet.png", 64, 32, sheet_pixels())
    pngio.write_rgba(directory / "flag.png", 16, 16, outlined_glyph("R", (60, 200, 230, 255), (16, 30, 44, 255)))
    pngio.write_rgba(directory / "panel_b.png", 24, 24, panel_pixels(
        ((215, 70, 200, 255), (60, 210, 220, 255), (150, 220, 60, 255), (245, 245, 245, 255)),
        (80, 80, 92, 255), (200, 200, 215, 255), (50, 40, 70, 255)))
    (directory / "sprites.atlas.json").write_text(json.dumps(atlas_descriptor(), indent=1), encoding="utf-8")
    occluder = write_gltf(directory, "occluder", [with_material(rect_xy(-1.0, 1.0, -1.0, 1.0, 0.0), 0)],
                          [flat_material("occluder", (0.0, 0.0, 0.0), 0.9, emissive=(0.08, 0.32, 0.18))])
    return [directory / "sheet.png", directory / "sprites.atlas.json", occluder]


def sprites_imports():
    return [("sheet.png", "color"), ("sprites.atlas.json", "color"), ("occluder.gltf", None)]


def sprites_scene(assets):
    p = Project("sprites", "01JA41PWP0")
    sheet, atlas = assets["sheet.png"], assets["sprites.atlas.json"]

    def sprite(name, texture, region, rect, *, z=0.0, flip=(False, False), angle=0.0, tint_value=(1, 1, 1, 1),
               layer=0, order=0, mode=None, parent=None):
        x, y, w, h = rect
        value = {"texture": texture, "region": region, "size": [w / PPU, h / PPU], "pivot": [0.5, 0.5],
                 "tint": list(tint_value), "flip_x": flip[0], "flip_y": flip[1], "layer": layer, "order": order,
                 "sampling": "nearest", "mode": mode or {"kind": "simple"}}
        if parent:
            at = (0.0, 0.0, 0.0)
        else:
            at = (wx(x + w / 2), wy(y + h / 2), z)
        p.entity(name, {"SpriteRenderer": value, "Transform": transform(at, quat_axis((0, 0, 1), angle))},
                 parent=parent)
        p.meta[name] = {"rect": list(rect), "z": z}

    def mirror(name, rect, z=0.0):
        x, y, w, h = rect
        p.entity(name, {"Transform": transform((wx(x + w / 2), wy(y + h / 2), z), scale=(-1.0, 1.0, 1.0))})

    solid = SHEET_REGIONS["solid"]
    sprite("Background", sheet, solid, (-16, -16, 352, 212), tint_value=tint((30, 32, 46)), layer=-100)
    for k in range(5):  # stripes behind the translucent column (x 270..310)
        sprite(f"Stripe light {k}", sheet, solid, (270 + 8 * k, 4, 4, 78), tint_value=tint((210, 210, 220)),
               layer=-50)
        sprite(f"Stripe dark {k}", sheet, solid, (274 + 8 * k, 4, 4, 78), tint_value=tint((60, 64, 80)), layer=-50)
    rows = {"A": (sheet, SHEET_REGIONS["glyph"], 8), "B": (atlas, {"frame": FRAME_FLAG}, 46)}
    for row, (texture, region, y) in rows.items():
        for i, variant in enumerate(("normal", "flip-x", "flip-y", "flip-xy", "rot90", "reflection", "tint",
                                     "translucent")):
            rect = (8 + 38 * i, y, 32, 32)
            name = f"{row} {variant}"
            if variant == "reflection":
                mirror(f"{name} parent", rect)
                sprite(name, texture, region, rect, parent=f"{name} parent")
                continue
            flips = {"flip-x": (True, False), "flip-y": (False, True), "flip-xy": (True, True)}
            sprite(name, texture, region, rect, flip=flips.get(variant, (False, False)),
                   angle=90.0 if variant == "rot90" else 0.0,
                   tint_value={"tint": (0.35, 0.8, 1.0, 1.0), "translucent": (1.0, 1.0, 1.0, 0.5)}.get(
                       variant, (1, 1, 1, 1)))
    nine_a = {"kind": "nine_slice", "border": [5, 5, 5, 5], "pixels_per_unit": PPU}
    sprite("C ordinary nine-slice", sheet, SHEET_REGIONS["panel"], (8, 86, 64, 40), mode=nine_a)
    sprite("C atlas nine-slice", atlas, {"frame": FRAME_PANEL}, (80, 86, 64, 40), mode=nine_a)
    mirror("C reflected nine-slice parent", (152, 86, 64, 40))
    sprite("C reflected nine-slice", sheet, SHEET_REGIONS["panel"], (152, 86, 64, 40), mode=nine_a,
           parent="C reflected nine-slice parent")
    sprite("C atlas nine-slice flip-xy", atlas, {"frame": FRAME_PANEL}, (224, 86, 64, 40), mode=nine_a,
           flip=(True, True))
    sprite("C atlas nine-slice compressed", atlas, {"frame": FRAME_PANEL}, (300, 86, 8, 40), mode=nine_a)
    for k in range(4):
        sprite(f"D stripe {k}", sheet, solid, (8 + 16 * k, 138, 8, 36), tint_value=tint((200, 205, 215)), layer=-50)
    for k in range(3):
        sprite(f"D orb {k}", sheet, SHEET_REGIONS["orb"], (10 + 14 * k, 144, 24, 24), layer=k)
    sprite("D glyph behind occluder", sheet, SHEET_REGIONS["glyph"], (160, 138, 32, 32), z=0.0)
    sprite("D atlas glyph in front", atlas, {"frame": FRAME_FLAG}, (232, 138, 32, 32), z=2.0)
    sprite("D translucent atlas over occluder", atlas, {"frame": FRAME_FLAG}, (196, 150, 24, 24), z=2.0,
           tint_value=(1.0, 1.0, 1.0, 0.55))
    occluder_rect = (180, 140, 70, 30)
    ox, oy, ow, oh = occluder_rect
    p.entity("D occluder model", {"MeshRenderer": mesh(assets["occluder.gltf"]),
                                  "Transform": transform((wx(ox + ow / 2), wy(oy + oh / 2), 1.0),
                                                         scale=(ow / PPU / 2, oh / PPU / 2, 1.0))})
    p.meta["D occluder model"] = {"rect": list(occluder_rect), "z": 1.0}
    p.camera(View("pixel", (0.0, 0.0, 10.0), [0.0, 0.0, 0.0, 1.0], fov=40.0, far=50.0,
                  ortho_half_height=LOGICAL[1] / PPU / 2, pixel=(PPU, LOGICAL)))
    eye, target = (8.0, -4.0, 19.0), (0.0, 0.0, 0.0)
    p.camera(View("perspective", eye, look_rotation(norm(sub(target, eye))), fov=40.0, far=50.0))
    return p


PROJECTS = {
    "materials": (materials_sources, materials_imports, materials_scene),
    "shadows": (shadows_sources, shadows_imports, shadows_scene),
    "lights": (lights_sources, lights_imports, lights_scene),
    "sprites": (sprites_sources, sprites_imports, sprites_scene),
}

# ----------------------------------------------------------- framing check --


def framing(project):
    """Analytic pixel bounds of key content per view; raises if anything leaves the frame."""
    report = {}
    p = project
    if p.name == "materials":
        board, normals = p.views["board"], p.views["normals"]
        pts = []
        for name, m in p.meta.items():
            if name.startswith("Normals"):
                continue
            cx, cy, cz = m["centre"]
            h = m["size"] / 2
            pts += [(cx + sx * h, cy + sy * h, cz) for sx in (-1, 1) for sy in (-1, 1)]
        for k in range(len(M_SWATCHES)):
            cx = (k - 3.5) * M_SWATCH_PITCH
            pts += [(cx + sx * M_SWATCH_W / 2, M_SWATCH_Y + sy * M_SWATCH_H / 2, 0.0) for sx in (-1, 1)
                    for sy in (-1, 1)]
        report["board"] = bounds(board, pts)
        corners = [(-10.0, -6.0, -1.5), (10.0, 5.0, -1.5)]
        report["board backdrop covers frame"] = covers(board, corners)
        pts = []
        for name, m in p.meta.items():
            if name.startswith("Normals"):
                cx, cy, cz = m["centre"]
                h = m["size"] / 2
                pts += [(cx + sx * h, cy + sy * h, cz) for sx in (-1, 1) for sy in (-1, 1)]
        report["normals"] = bounds(normals, pts)
    elif p.name == "shadows":
        view = p.views["board"]
        dx, dz = shadow_offset()
        pts = []
        for m in p.meta.values():
            x, y, z = m["centre"]
            h = S_CARD / 2
            pts += [(x + sx * h, y, z + sz * h) for sx in (-1, 1) for sz in (-1, 1)]
            if m["casts"]:
                pts += [(x + dx + sx * h, 0.0, z + dz + sz * h) for sx in (-1, 1) for sz in (-1, 1)]
        report["board"] = bounds(view, pts)
        x0, x1, z0, z1 = S_FLOOR
        report["floor covers frame"] = covers(view, [(x0, 0.0, z0), (x1, 0.0, z1)], floor=True)
    elif p.name == "lights":
        view = p.views["board"]
        pts = [(x + sx * 0.75, y + sy * 0.75, z) for (x, y, z) in L_OBJECTS.values() for sx in (-1, 1)
               for sy in (-1, 1)]
        report["board"] = bounds(view, pts)
    elif p.name == "sprites":
        view = p.views["perspective"]
        pts = [(wx(x), wy(y), 0.0) for x in (-16, 336) for y in (-16, 196)]
        report["perspective background corners"] = [list(view.project(q)[:2]) for q in pts]
        inner = [(wx(x), wy(y), 0.0) for x in (0, 320) for y in (0, 180)]
        report["perspective board"] = bounds(view, inner)
    return report


def bounds(view, pts, margin=4):
    xs, ys = [], []
    for q in pts:
        r = view.project(q)
        if r is None:
            raise SystemExit(f"{view.name}: point behind camera {q}")
        xs.append(r[0])
        ys.append(r[1])
    w, h = view.size
    box_ = [min(xs), min(ys), max(xs), max(ys)]
    if box_[0] < margin or box_[1] < margin or box_[2] > w - margin or box_[3] > h - margin:
        raise SystemExit(f"{view.name}: content leaves the frame {box_}")
    return [round(v, 1) for v in box_]


def covers(view, corners, floor=False):
    """True when the given axis-aligned rectangle covers the whole frame (checked by sampling rays)."""
    (x0, y0, z0), (x1, y1, z1) = corners
    w, h = view.size
    q = view.rotation
    t = math.tan(math.radians(view.fov) / 2)
    for px in (0, w):
        for py in (0, h):
            xn, yn = 2 * px / w - 1, 1 - 2 * py / h
            d = rotate(q, (xn * t * w / h, yn * t, -1.0))
            if floor:
                if d[1] >= 0:
                    return False
                s = -view.eye[1] / d[1]
                hit = (view.eye[0] + s * d[0], view.eye[2] + s * d[2])
                if not (x0 <= hit[0] <= x1 and z0 <= hit[1] <= z1):
                    return False
            else:
                s = (z0 - view.eye[2]) / d[2]
                hit = (view.eye[0] + s * d[0], view.eye[1] + s * d[1])
                if not (x0 <= hit[0] <= x1 and y0 <= hit[1] <= y1):
                    return False
    return True
