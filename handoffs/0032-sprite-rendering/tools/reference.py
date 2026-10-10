"""Independent CPU reference for the documented sprite contract (handoff 0032).

Written from the public contract (docs/spikes/sprite-rendering.md, the
SpriteRenderer schema and the display/tone-map shaders), not by running the
engine. It predicts every 8-bit output pixel of an orthographic or pixel-perfect
frame so real engine frames can be compared pixel by pixel:

  * sources: sRGB 8-bit (or 16-bit) texels -> linear -> premultiplied -> f16;
  * geometry: the SpriteRenderer tessellation (simple and nine-slice, flips swap
    asymmetric borders), the entity's inherited Transform, pixel-perfect camera
    snapping (half-pixel ties, odd-size phase) and an orthographic projection;
  * raster: pixel-centre coverage with the top-left fill rule, affine UVs, UV
    clamped to the region's outer texel centres, nearest or bilinear (clamp to
    edge) level-zero sampling;
  * painter order: (layer, order, entity ID); depth test LessEqual against opaque
    meshes, no sprite depth writes; premultiplied "over" blending stored as f16;
  * display: unpremultiply, Incant tone curve (identity below a 0.8 peak), mix
    with the #141519 backdrop by coverage, sRGB encode, 8-bit round.

Pixels whose result depends on sub-1/256 decisions (a pixel centre on an edge,
or a nearest sample on a texel boundary between different texels) are flagged
ambiguous rather than guessed. Standard library only.
"""

import math
import struct
import zlib
from pathlib import Path

EPS_EDGE = 1.0 / 256  # px: GPU vertex snapping precision
# Counterfactual switch for evidence only: filter straight-alpha texels (hidden RGB
# included) and premultiply afterwards. The contract premultiplies before filtering.
STRAIGHT_ALPHA_FILTERING = False
EPS_TEXEL = 1.0 / 256  # texel: sampler sub-texel precision
BACKDROP_SRGB = (20, 21, 25)


def decode_srgb(v):
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def encode_srgb(v):
    return v * 12.92 if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055


BACKDROP = tuple(decode_srgb(c / 255) for c in BACKDROP_SRGB)


def f16(x):
    """Round to the nearest IEEE half (ties to even), saturating at 65504."""
    if x == 0.0:
        return 0.0
    if x > 65504.0:
        return 65504.0
    m, e = math.frexp(x)  # x = m * 2**e, 0.5 <= |m| < 1
    q = math.ldexp(1.0, max(e - 11, -24))
    return round(x / q) * q


def tone_map(c):
    peak = max(c)
    if peak < 0.8:
        return c
    new_peak = 1.0 - 0.04 / (peak + 0.2 - 0.8)
    compressed = [x * (new_peak / peak) for x in c]
    g = 1.0 - 1.0 / (0.15 * (peak - new_peak) + 1.0)
    return [x + (new_peak - x) * g for x in compressed]


def display(rgba):
    """HDR premultiplied pixel -> 8-bit sRGB triple."""
    r, g, b, a = rgba
    coverage = min(max(a, 0.0), 1.0)
    d = max(coverage, 1e-8)
    radiance = [min(max(c / d, 0.0), 65504.0) for c in (r, g, b)]
    mapped = tone_map(radiance)
    out = []
    for k in range(3):
        v = BACKDROP[k] * (1 - coverage) + mapped[k] * coverage
        out.append(int(math.floor(encode_srgb(min(max(v, 0.0), 1.0)) * 255 + 0.5)))
    return tuple(out)


# ----------------------------------------------------------------- textures --

def read_png_float(path):
    """8- or 16-bit RGBA/RGB PNG -> (width, height, rows of normalized (r,g,b,a), bits)."""
    data = Path(path).read_bytes()
    pos, idat, header = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    width, height, depth, ctype, _, _, interlace = header
    assert depth in (8, 16) and ctype in (2, 6) and not interlace, header
    channels = 4 if ctype == 6 else 3
    bpp = channels * depth // 8
    raw = zlib.decompress(idat)
    stride = width * bpp
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = line
        maxv = 255 if depth == 8 else 65535
        vals = list(line) if depth == 8 else list(struct.unpack(f">{stride // 2}H", bytes(line)))
        row = []
        for x in range(width):
            px = vals[x * channels:(x + 1) * channels]
            if channels == 3:
                px = px + [maxv]
            row.append(tuple(v / maxv for v in px))
        rows.append(row)
    return width, height, rows, depth


class Texture:
    def __init__(self, path):
        self.width, self.height, rows, self.bits = read_png_float(path)
        self.premul, self.straight = [], []
        for row in rows:
            out, raw = [], []
            for r, g, b, a in row:
                lin = [decode_srgb(c) for c in (r, g, b)]
                out.append((f16(lin[0] * a), f16(lin[1] * a), f16(lin[2] * a), f16(a)))
                raw.append((lin[0], lin[1], lin[2], a))
            self.premul.append(out)
            self.straight.append(raw)


# ----------------------------------------------------------------- geometry --

def mat_mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def local_matrix(t):
    x, y, z, w = t["rotation"]
    sx, sy, sz = t["scale"]
    r = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
         [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
         [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    tx, ty, tz = t["translation"]
    return [[r[0][0] * sx, r[0][1] * sy, r[0][2] * sz, tx],
            [r[1][0] * sx, r[1][1] * sy, r[1][2] * sz, ty],
            [r[2][0] * sx, r[2][1] * sy, r[2][2] * sz, tz],
            [0.0, 0.0, 0.0, 1.0]]


def apply(m, p):
    return [m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3] for i in range(3)]


def apply_vec(m, v):
    return [m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2] for i in range(3)]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def normalize(v):
    n = math.sqrt(dot(v, v))
    return [c / n for c in v]


def world_matrices(entities, overrides=None):
    """entities: list of {id, parent, components}. overrides: id -> Transform."""
    by_id = {e["id"]: e for e in entities}
    cache = {}

    def world(eid):
        if eid in cache:
            return cache[eid]
        e = by_id[eid]
        t = (overrides or {}).get(eid) or e["components"].get("Transform")
        m = local_matrix(t)
        if e.get("parent"):
            m = mat_mul(world(e["parent"]), m)
        cache[eid] = m
        return m

    return {eid: world(eid) for eid in by_id if "Transform" in by_id[eid]["components"]}


def cuts(size, low, high):
    if low + high >= size:
        middle = size * (low / (low + high))
        return [0.0, middle, middle, size]
    return [0.0, low, size - high, size]


def tessellate(sprite, tw, th):
    """Port of incant_2d::tessellate: list of (local x, local y, u, v) triangle vertices."""
    x, y, w, h = sprite["region"]
    sx, sy = sprite["size"]
    flip_x, flip_y = sprite.get("flip_x", False), sprite.get("flip_y", False)
    mode = sprite.get("mode") or {"kind": "simple"}
    if mode["kind"] == "simple":
        px, py, tx, ty = [0.0, sx], [0.0, sy], [0.0, float(w)], [0.0, float(h)]
    else:
        left, right, top, bottom = [float(v) for v in mode["border"]]
        ppu = mode["pixels_per_unit"]
        if flip_x:
            left, right = right, left
        if flip_y:
            bottom, top = top, bottom
        px = cuts(sx, left / ppu, right / ppu)
        py = cuts(sy, bottom / ppu, top / ppu)
        tx = [0.0, left, w - right, float(w)]
        ty = [0.0, bottom, h - top, float(h)]
    pu, pv = sprite["pivot"]
    verts = []
    for j in range(len(py) - 1):
        for i in range(len(px) - 1):
            if px[i] == px[i + 1] or py[j] == py[j + 1]:
                continue
            for a, b in ((i, j), (i + 1, j), (i + 1, j + 1), (i, j), (i + 1, j + 1), (i, j + 1)):
                u = w - tx[a] if flip_x else tx[a]
                v = ty[b] if flip_y else h - ty[b]
                verts.append((px[a] - sx * pu, py[b] - sy * pv, (x + u) / tw, (y + v) / th))
    return verts


# ------------------------------------------------------------------- camera --

class View:
    """World -> target pixel mapping for one authored camera and output size."""

    def __init__(self, camera, world, out_w, out_h):
        proj = camera.get("projection") or {"kind": "perspective"}
        eye = apply(world, (0.0, 0.0, 0.0))
        self.pixel = proj["kind"] == "pixel_perfect"
        if proj["kind"] == "perspective":
            raise ValueError("reference covers orthographic and pixel-perfect cameras only")
        if self.pixel:
            ppu = proj["pixels_per_unit"]
            res = proj["reference_resolution"]
            forward = normalize(apply_vec(world, (0.0, 0.0, -1.0)))
            right = normalize(cross(forward, apply_vec(world, (0.0, 1.0, 0.0))))
            up = normalize(cross(right, forward))
            for axis, is_right in ((right, True), (up, False)):
                coordinate = dot(eye, axis)
                pixel = coordinate * ppu
                half = math.floor(pixel) + 0.5
                tol = min(8 * 2.220446049250313e-16 * max(abs(pixel), 1.0), 1e-7)
                if abs(pixel - half) <= tol:
                    pixel = half
                snapped = math.ceil(pixel - 0.5) if is_right else math.floor(pixel + 0.5)
                dim = res[0] if is_right else res[1]
                phase = (0.5 if is_right else -0.5) if dim % 2 == 1 else 0.0
                shift = (snapped + phase) / ppu - coordinate
                eye = [eye[k] + axis[k] * shift for k in range(3)]
            self.half_h = res[1] / ppu / 2
            self.aspect = res[0] / res[1]
            self.width, self.height = res
            scale = min(out_w // res[0], out_h // res[1])
            if scale == 0:
                raise ValueError("viewport smaller than the logical resolution")
            self.output = (out_w, out_h, scale, (out_w - res[0] * scale) // 2, (out_h - res[1] * scale) // 2)
        else:
            self.half_h = proj["vertical_size"] / 2
            self.aspect = out_w / out_h
            self.width, self.height = out_w, out_h
            self.output = None
        self.eye = eye
        f = normalize(apply_vec(world, (0.0, 0.0, -1.0)))
        upv = normalize(apply_vec(world, (0.0, 1.0, 0.0)))
        s = normalize(cross(f, upv))
        u = cross(s, f)
        self.s, self.u, self.f = s, u, f
        self.near, self.far = camera["near"], camera["far"]

    def project(self, p):
        d = [p[k] - self.eye[k] for k in range(3)]
        xv, yv, zv = dot(self.s, d), dot(self.u, d), -dot(self.f, d)
        xn = xv / (self.half_h * self.aspect)
        yn = yv / self.half_h
        depth = (zv + self.near) / (self.near - self.far)
        return ((xn + 1) * 0.5 * self.width, (1 - yn) * 0.5 * self.height, depth)


# --------------------------------------------------------------- rasterizer --

class Frame:
    def __init__(self, w, h):
        self.w, self.h = w, h
        n = w * h
        self.r, self.g, self.b, self.a = [0.0] * n, [0.0] * n, [0.0] * n, [0.0] * n
        self.depth = [1.0] * n
        self.ambiguous = [False] * n
        self.linear = [False] * n
        self.owner = [None] * n  # top-most contributing entity name


def _edges(p0, p1, p2):
    area = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0])
    if area == 0:
        return None
    sign = 1.0 if area > 0 else -1.0
    edges = []
    for a, b in ((p0, p1), (p1, p2), (p2, p0)):
        # E(p) = (b.x-a.x)(p.y-a.y) - (b.y-a.y)(p.x-a.x), oriented positive inside.
        A = -(b[1] - a[1]) * sign
        B = (b[0] - a[0]) * sign
        C = -(A * a[0] + B * a[1])
        top_left = A > 0 or (A == 0 and B > 0)
        edges.append((A, B, C, top_left, math.hypot(A, B)))
    return edges, area * sign


def raster_triangle(frame, pts, attrs, shade, outer=(True, True, True)):
    """pts: 3 x (sx, sy). attrs: 3 x tuple. shade(i, attr, ambiguous_edge, inside) per pixel.

    `outer` marks which edges (01, 12, 20) lie on the drawable's outer boundary. Only
    those can make coverage ambiguous: internal diagonals and nine-slice cut lines
    are shared by neighbours with continuous UVs, so either side gives the same colour.
    """
    # Snap to the rasterizer's 1/256 px fixed-point grid, as GPUs do. Edge values
    # are then exact in f64, so shared edges are watertight under the fill rule.
    pts = [(round(x * 256) / 256, round(y * 256) / 256) for x, y in pts]
    res = _edges(*pts)
    if res is None:
        return
    edges, area = res
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    x0, x1 = max(0, int(math.floor(min(xs) - 0.5))), min(frame.w - 1, int(math.ceil(max(xs) + 0.5)))
    y0, y1 = max(0, int(math.floor(min(ys) - 0.5))), min(frame.h - 1, int(math.ceil(max(ys) + 0.5)))
    if x0 > x1 or y0 > y1:
        return
    # Barycentric weights: lambda_k = E_opposite / area, edges are (01, 12, 20).
    (A0, B0, C0, t0, n0), (A1, B1, C1, t1, n1), (A2, B2, C2, t2, n2) = edges
    o0, o1, o2 = outer
    na = len(attrs[0])
    for py in range(y0, y1 + 1):
        cy = py + 0.5
        for px in range(x0, x1 + 1):
            cx = px + 0.5
            e0 = A0 * cx + B0 * cy + C0
            e1 = A1 * cx + B1 * cy + C1
            e2 = A2 * cx + B2 * cy + C2
            amb = ((o0 and abs(e0) < EPS_EDGE * n0) or (o1 and abs(e1) < EPS_EDGE * n1)
                   or (o2 and abs(e2) < EPS_EDGE * n2))
            inside = ((e0 > 0 or (e0 == 0 and t0)) and (e1 > 0 or (e1 == 0 and t1))
                      and (e2 > 0 or (e2 == 0 and t2)))
            if not inside and not amb:
                continue
            l0, l1, l2 = e1 / area, e2 / area, e0 / area  # weights of vertices 0, 1, 2
            attr = tuple(l0 * attrs[0][k] + l1 * attrs[1][k] + l2 * attrs[2][k] for k in range(na))
            shade(py * frame.w + px, attr, amb, inside)


def sample(tex, u, v, sampling, bounds):
    """Premultiplied linear sample; returns (rgba, ambiguous)."""
    u = min(max(u, bounds[0]), bounds[2])
    v = min(max(v, bounds[1]), bounds[3])
    tw, th = tex.width, tex.height
    if sampling == "nearest":
        fx, fy = u * tw, v * th
        ix, iy = min(int(math.floor(fx)), tw - 1), min(int(math.floor(fy)), th - 1)
        texel = tex.premul[iy][ix]
        amb = False
        for f, i, axis in ((fx, ix, 0), (fy, iy, 1)):
            frac = f - math.floor(f)
            for j in ((i - 1) if frac < EPS_TEXEL else None, (i + 1) if frac > 1 - EPS_TEXEL else None):
                if j is None:
                    continue
                if axis == 0 and 0 <= j < tw and tex.premul[iy][j] != texel:
                    amb = True
                if axis == 1 and 0 <= j < th and tex.premul[j][ix] != texel:
                    amb = True
        return texel, amb
    x, y = u * tw - 0.5, v * th - 0.5
    x0, y0 = math.floor(x), math.floor(y)
    fx, fy = x - x0, y - y0
    xa, xb = min(max(x0, 0), tw - 1), min(max(x0 + 1, 0), tw - 1)
    ya, yb = min(max(y0, 0), th - 1), min(max(y0 + 1, 0), th - 1)
    src = tex.straight if STRAIGHT_ALPHA_FILTERING else tex.premul
    t00, t10, t01, t11 = src[ya][xa], src[ya][xb], src[yb][xa], src[yb][xb]
    out = tuple((t00[k] * (1 - fx) + t10[k] * fx) * (1 - fy) + (t01[k] * (1 - fx) + t11[k] * fx) * fy
                for k in range(4))
    if STRAIGHT_ALPHA_FILTERING:
        out = (out[0] * out[3], out[1] * out[3], out[2] * out[3], out[3])
    return out, False


def draw_sprite(frame, view, world, sprite, tex, name):
    reg = sprite["region"]
    tw, th = tex.width, tex.height
    bounds = ((reg[0] + 0.5) / tw, (reg[1] + 0.5) / th, (reg[0] + reg[2] - 0.5) / tw, (reg[1] + reg[3] - 0.5) / th)
    tint = sprite["tint"]
    sampling = sprite.get("sampling", "nearest")
    verts = tessellate(sprite, tw, th)
    sx_, sy_ = sprite["size"]
    pu, pv = sprite["pivot"]
    xmin, xmax, ymin, ymax = -sx_ * pu, sx_ * (1 - pu), -sy_ * pv, sy_ * (1 - pv)

    def on(a, b):
        return abs(a - b) <= 1e-9 * max(1.0, abs(b))

    projected = []
    for lx, ly, u, v in verts:
        sx, sy, depth = view.project(apply(world, (lx, ly, 0.0)))
        projected.append(((sx, sy), (u, v, depth), (lx, ly)))
    R, G, B, A, D = frame.r, frame.g, frame.b, frame.a, frame.depth
    tr, tg, tb, ta = tint

    def shade(i, attr, amb_edge, inside):
        u, v, depth = attr
        if amb_edge:
            frame.ambiguous[i] = True
        if not inside:
            return
        if depth > D[i] + 1e-6:
            return
        texel, amb = sample(tex, u, v, sampling, bounds)
        if amb:
            frame.ambiguous[i] = True
        sr, sg, sb, sa = texel[0] * tr * ta, texel[1] * tg * ta, texel[2] * tb * ta, texel[3] * ta
        if sa == 0 and sr == 0 and sg == 0 and sb == 0:
            return
        k = 1.0 - sa
        R[i] = f16(sr + R[i] * k)
        G[i] = f16(sg + G[i] * k)
        B[i] = f16(sb + B[i] * k)
        A[i] = f16(sa + A[i] * k)
        if sampling == "linear":
            frame.linear[i] = True
        frame.owner[i] = name

    for t in range(0, len(projected), 3):
        tri = projected[t:t + 3]
        outer = []
        for a, b in ((0, 1), (1, 2), (2, 0)):
            (ax, ay), (bx, by) = tri[a][2], tri[b][2]
            outer.append((on(ax, bx) and (on(ax, xmin) or on(ax, xmax)))
                         or (on(ay, by) and (on(ay, ymin) or on(ay, ymax))))
        raster_triangle(frame, [p[0] for p in tri], [p[1] for p in tri], shade, tuple(outer))


def draw_mesh_quad(frame, view, world, positions, radiance, name):
    pts = [view.project(apply(world, p)) for p in positions]
    R, G, B, A, D = frame.r, frame.g, frame.b, frame.a, frame.depth

    def shade(i, attr, amb_edge, inside):
        if amb_edge:
            frame.ambiguous[i] = True
        if not inside or attr[0] >= D[i]:
            return
        D[i] = attr[0]
        R[i], G[i], B[i], A[i] = f16(radiance[0]), f16(radiance[1]), f16(radiance[2]), 1.0
        frame.owner[i] = name

    for tri, outer in (((0, 1, 2), (True, True, False)), ((0, 2, 3), (False, True, True))):
        raster_triangle(frame, [pts[k][:2] for k in tri], [(pts[k][2],) for k in tri], shade, outer)


def render(scene_entities, textures_by_asset, camera_id, out_w, out_h, meshes=(), overrides=None):
    """Returns (rows of 8-bit RGB tuples at output size, frame, view).

    scene_entities: the authored entity list. textures_by_asset: asset id -> Texture.
    meshes: list of (entity id, positions, radiance) for opaque emissive quads.
    overrides: entity id -> replacement component dict (Transform/SpriteRenderer).
    """
    overrides = overrides or {}
    entities = []
    for e in scene_entities:
        if e["id"] in overrides:
            e = dict(e, components=dict(e["components"], **overrides[e["id"]]))
        entities.append(e)
    worlds = world_matrices(entities)
    by_id = {e["id"]: e for e in entities}
    view = View(by_id[camera_id]["components"]["Camera"], worlds[camera_id], out_w, out_h)
    frame = Frame(view.width, view.height)
    for mesh_id, positions, radiance in meshes:
        draw_mesh_quad(frame, view, worlds[mesh_id], positions, radiance, by_id[mesh_id]["name"])
    sprites = [e for e in entities if "SpriteRenderer" in e["components"]]
    sprites.sort(key=lambda e: (e["components"]["SpriteRenderer"].get("layer", 0),
                                e["components"]["SpriteRenderer"].get("order", 0), e["id"]))
    for e in sprites:
        sprite = e["components"]["SpriteRenderer"]
        draw_sprite(frame, view, worlds[e["id"]], sprite, textures_by_asset[sprite["texture"]], e["name"])
    logical = [display((frame.r[i], frame.g[i], frame.b[i], frame.a[i])) for i in range(frame.w * frame.h)]
    if view.output is None:
        rows = [logical[y * frame.w:(y + 1) * frame.w] for y in range(frame.h)]
        return rows, frame, view
    out_w, out_h, s, ox, oy = view.output
    back = BACKDROP_SRGB
    rows = []
    for y in range(out_h):
        ly = (y - oy) // s if y >= oy else -1
        row = []
        for x in range(out_w):
            lx = (x - ox) // s if x >= ox else -1
            if 0 <= lx < frame.w and 0 <= ly < frame.h:
                row.append(logical[ly * frame.w + lx])
            else:
                row.append(back)
        rows.append(row)
    return rows, frame, view
