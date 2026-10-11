#!/usr/bin/env python3
"""Independent CPU reference for handoff 0039 views. Standard library only.

From the scene description and the original source images (never the cooked atlas
or engine code), it predicts every pixel of a view under the documented contract:
cell (x, y) of a map covers local [x*cw, (x+1)*cw] x [y*ch, (y+1)*ch] with +y up,
shows its whole frame with image rows top-down, nearest sampling, white tint and
binary alpha; inherited transforms compose parent * T*R*S; painter order is
(layer, order, entity ID, layer ID, y, x) for cells and (layer, order, entity ID)
for sprites.

Exact ties follow the documented rules: top-left coverage for edges through a pixel
centre, and the nearest texel on the side of increasing screen coordinate along the
dominant axis (1/1024 px bias, X wins equal slopes). Any pixel centre within DELTA px
of a relevant coverage edge or texel boundary is reported as ambiguous, with every
colour reachable by moving the sample DELTA px in any direction.

Pixel-perfect views are modelled at their logical resolution; the integer upscale
is checked separately against the engine's own 1x frame.

Usage (repo root), a preview of a predicted view:
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_model.py RUN STATE VIEW OUT.png
"""

import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
ROOT = HELPERS.parents[2]
sys.path.insert(0, str(HELPERS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import pngio  # noqa: E402

DELTA = 0.01
BIAS = 1.0 / 1024.0
COVER = (1e-7, 1e-10)  # emulate the top-left rule for exact hits: x decides, then y
JITTER = [(dx, dy) for dx in (-DELTA, 0.0, DELTA) for dy in (-DELTA, 0.0, DELTA) if dx or dy]
BUCKET = 16


# ----------------------------------------------------------------- affine --

def quat_matrix(degrees, scale):
    """glam DMat4::from_scale_rotation_translation for a z-axis quaternion, normalized."""
    h = math.radians(degrees) / 2
    z, w = math.sin(h), math.cos(h)
    n = math.sqrt(z * z + w * w)
    z, w = z / n, w / n
    zz, wz = z * (z + z), w * (z + z)
    sx, sy = scale
    return (1.0 - zz) * sx, -wz * sy, wz * sx, (1.0 - zz) * sy


def local_matrix(item):
    a, b, d, e = quat_matrix(item["r"], item["s"])
    return (a, b, item["t"][0], d, e, item["t"][1])


def compose(p, q):
    """p * q for affines (a, b, c, d, e, f): x' = a x + b y + c, y' = d x + e y + f."""
    a1, b1, c1, d1, e1, f1 = p
    a2, b2, c2, d2, e2, f2 = q
    return (a1 * a2 + b1 * d2, a1 * b2 + b1 * e2, a1 * c2 + b1 * f2 + c1,
            d1 * a2 + e1 * d2, d1 * b2 + e1 * e2, d1 * c2 + e1 * f2 + f1)


def invert(m):
    a, b, c, d, e, f = m
    det = a * e - b * d
    ia, ib, id_, ie = e / det, -b / det, -d / det, a / det
    return (ia, ib, -(ia * c + ib * f), id_, ie, -(id_ * c + ie * f))


def apply(m, x, y):
    return m[0] * x + m[1] * y + m[2], m[3] * x + m[4] * y + m[5]


# ----------------------------------------------------------------- inputs --

class Textures:
    def __init__(self, art, manifest):
        self.images = {name: pngio.read(art / name) for name in ("tiles.png", "plates.png", "badge.png")}
        self.frames = {f["name"]: f["region"] for f in manifest["frames"].values()}
        self.plates = manifest["plates"]
        self.badge = [0, 0, *manifest["badge"]]

    def source(self, texture):
        """(image, region) for a scene texture tuple or a frame name."""
        if isinstance(texture, str):
            return self.images["tiles.png"], self.frames[texture]
        if texture[0] == "plate":
            return self.images["plates.png"], self.plates[texture[1]]
        if texture[0] == "badge":
            return self.images["badge.png"], self.badge
        return self.images["tiles.png"], self.frames[texture[1]]


def cells_bottom_up(rows):
    return [None if ch == "." else ch for row in reversed(rows) for ch in row]


def draw_items(description, textures):
    """Every drawable quad with world affine, local rectangle, source and painter key."""
    items = {i["name"]: i for i in description["items"]}
    ids = description["ids"]
    worlds = {}

    def world(name):
        if name not in worlds:
            item = items[name]
            m = local_matrix(item)
            worlds[name] = compose(world(item["parent"]), m) if item["parent"] else m
        return worlds[name]

    out = []
    for item in description["items"]:
        if item["kind"] == "sprite":
            w, h = item["size"]
            px, py = item["pivot"]
            image, region = textures.source(item["texture"])
            out.append({"key": (item["layer"], item["order"], ids[item["name"]], 0, "", 0, 0),
                        "world": world(item["name"]), "rect": (-px * w, -py * h, (1 - px) * w, (1 - py) * h),
                        "image": image, "region": region, "name": item["name"]})
        elif item["kind"] == "map":
            cw, ch = item["cell"]
            width = item["size"][0]
            for lay in item["layers"]:
                for index, frame in enumerate(cells_bottom_up(lay["rows"])):
                    if frame is None:
                        continue
                    x, y = index % width, index // width
                    image, region = textures.source(frame)
                    out.append({"key": (lay["layer"], lay["order"], ids[item["name"]], 1, lay["id"], y, x),
                                "world": world(item["name"]),
                                "rect": (x * cw, y * ch, (x + 1) * cw, (y + 1) * ch),
                                "image": image, "region": region,
                                "name": f"{item['name']}:{lay['label']}:{x},{y}:{frame}"})
    out.sort(key=lambda d: d["key"], reverse=True)
    return out


# ------------------------------------------------------------------ views --

def view_affine(camera, width, height):
    """Screen pixel coordinates (x right, y down; pixel centre at +0.5) -> world.
    Pixel-perfect views return their logical resolution instead of the output size."""
    ex, ey = camera["t"]
    projection = camera["projection"]
    if projection["kind"] == "pixel_perfect":
        ppu = projection["pixels_per_unit"]
        lw, lh = projection["reference_resolution"]
        # Snap exactly as the camera does (right ties down, up ties up, odd-size phase).
        snapped = []
        for coordinate, is_right, dim in ((ex, True, lw), (ey, False, lh)):
            pixel = coordinate * ppu
            half = math.floor(pixel) + 0.5
            if abs(pixel - half) <= min(8 * sys.float_info.epsilon * max(abs(pixel), 1.0), 1e-7):
                pixel = half
            s = math.ceil(pixel - 0.5) if is_right else math.floor(pixel + 0.5)
            phase = (0.5 if is_right else -0.5) if dim % 2 else 0.0
            snapped.append((s + phase) / ppu)
        ex, ey = snapped
        return (1.0 / ppu, 0.0, ex - lw / ppu / 2, 0.0, -1.0 / ppu, ey + lh / ppu / 2), (lw, lh)
    half_h = projection["vertical_size"] / 2
    half_w = half_h * width / height
    return (2 * half_w / width, 0.0, ex - half_w, 0.0, -2 * half_h / height, ey + half_h), (width, height)


class Prepared:
    """Screen -> local affine, screen bbox and texel scales for one item in one view."""

    def __init__(self, item, screen):
        self.item = item
        self.m = compose(invert(item["world"]), screen)
        x0, y0, x1, y1 = item["rect"]
        rx, ry, rw, rh = item["region"]
        self.rect, self.size = (x0, y0, x1, y1), (rw, rh)
        a, b, _, d, e, _ = self.m
        self.gu, self.gv = math.hypot(a, b), math.hypot(d, e)
        # Texel coordinates: tu grows with u; tv grows downward in the image (top-down rows).
        self.su, self.sv = rw / (x1 - x0), rh / (y1 - y0)
        self.du = (a * self.su, b * self.su)
        self.dv = (-d * self.sv, -e * self.sv)
        inverse = invert(self.m)
        corners = [apply(inverse, u, v) for u, v in ((x0, y0), (x1, y0), (x0, y1), (x1, y1))]
        self.bbox = (min(c[0] for c in corners), min(c[1] for c in corners),
                     max(c[0] for c in corners), max(c[1] for c in corners))
        image = item["image"]
        self.texels = [[image.px(rx + i, ry + j) for i in range(rw)] for j in range(rh)]

    def local(self, x, y):
        return apply(self.m, x, y)

    def texel(self, u, v):
        x0, y0, x1, y1 = self.rect
        rw, rh = self.size
        return (min(max(math.floor((u - x0) * self.su), 0), rw - 1),
                min(max(math.floor((y1 - v) * self.sv), 0), rh - 1))

    def inside(self, u, v):
        x0, y0, x1, y1 = self.rect
        return x0 <= u < x1 and y0 <= v < y1

    def edge_margin(self, u, v):
        """Signed screen-px distance to the nearest edge line: > 0 inside, < 0 outside."""
        x0, y0, x1, y1 = self.rect
        return min((u - x0) / self.gu, (x1 - u) / self.gu, (v - y0) / self.gv, (y1 - v) / self.gv)

    def biased_texel(self, x, y):
        """Texel at a pixel centre under the 1/1024 px dominant-axis tie rule."""
        out = []
        for axis, (dx, dy) in enumerate((self.du, self.dv)):
            ox, oy = (BIAS, 0.0) if abs(dx) >= abs(dy) else (0.0, BIAS)
            u, v = self.local(x + ox, y + oy)
            out.append(self.texel(u, v)[axis])
        return tuple(out)


def evaluate(stack, x, y, rule=False):
    """Composite one sample: topmost opaque texel; binary alpha. rule=True applies
    the top-left coverage offset and the biased texel choice."""
    cx, cy = (x + COVER[0], y + COVER[1]) if rule else (x, y)
    for p in stack:
        u, v = p.local(cx, cy)
        if not p.inside(u, v):
            continue
        i, j = p.biased_texel(x, y) if rule else p.texel(*p.local(x, y))
        texel = p.texels[j][i]
        if texel[3] == 255:
            return texel[:3]
        if texel[3] != 0:
            raise ValueError("model supports binary alpha only")
    return None


def near_tie(stack, x, y):
    """True if a coverage edge or a colour-changing texel boundary of any item that
    can influence this pixel lies within DELTA px of the sample."""
    for p in stack:
        u, v = p.local(x, y)
        margin = p.edge_margin(u, v)
        if -DELTA < margin < DELTA:
            return True
        if margin <= -DELTA:
            continue
        i, j = p.texel(u, v)
        tu, tv = (u - p.rect[0]) * p.su, (p.rect[3] - v) * p.sv
        for t, g, axis in ((tu, math.hypot(*p.du), 0), (tv, math.hypot(*p.dv), 1)):
            boundary = round(t)
            if 0 < boundary < p.size[axis] and abs(t - boundary) / g < DELTA:
                current = i if axis == 0 else j
                other = boundary if current == boundary - 1 else boundary - 1
                a = p.texels[j][i]
                b = p.texels[j][other] if axis == 0 else p.texels[other][i]
                if a != b:
                    return True
        if p.texels[j][i][3] == 255:
            return False
    return False


def predict(description, textures, view):
    """Returns (size, rule colours, ambiguous {index: candidate colour set})."""
    camera_name, (width, height) = description["views"][view]
    screen, (lw, lh) = view_affine(description["cameras"][camera_name], width, height)
    prepared = [Prepared(item, screen) for item in draw_items(description, textures)]
    buckets = {}
    for p in prepared:
        bx0, by0, bx1, by1 = p.bbox
        if bx1 < -1 or by1 < -1 or bx0 > lw + 1 or by0 > lh + 1:
            continue
        for gy in range(max(0, int((by0 - 1) // BUCKET)), min((lh - 1) // BUCKET, int((by1 + 1) // BUCKET)) + 1):
            for gx in range(max(0, int((bx0 - 1) // BUCKET)), min((lw - 1) // BUCKET, int((bx1 + 1) // BUCKET)) + 1):
                buckets.setdefault((gx, gy), []).append(p)
    colours, ambiguous = [None] * (lw * lh), {}
    for py in range(lh):
        for px in range(lw):
            stack = buckets.get((px // BUCKET, py // BUCKET), [])
            x, y = px + 0.5, py + 0.5
            index = py * lw + px
            colours[index] = evaluate(stack, x, y, rule=True)
            if near_tie(stack, x, y):
                ambiguous[index] = {evaluate(stack, x + dx, y + dy) for dx, dy in JITTER} | \
                                   {evaluate(stack, x, y), colours[index]}
    return (lw, lh), colours, ambiguous


def load(run, state):
    run = Path(run)
    description = json.loads((run / f"scene-{state}.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    return description, Textures(run / "art", manifest)


def main():
    run, state, view, out = sys.argv[1:5]
    description, textures = load(run, state)
    (w, h), colours, ambiguous = predict(description, textures, view)
    pixels = [(*(c if c is not None else (255, 0, 0)), 255) for c in colours]
    for index in ambiguous:
        r, g, b, _ = pixels[index]
        pixels[index] = (r, g, b, 255)
    pngio.write_rgba(out, w, h, pixels)
    print(json.dumps({"view": view, "size": [w, h], "ambiguous": len(ambiguous),
                      "uncovered": sum(c is None for c in colours)}))


if __name__ == "__main__":
    main()
