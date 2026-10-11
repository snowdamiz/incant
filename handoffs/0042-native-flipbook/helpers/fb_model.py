"""Screen geometry and an approximate CPU preview for handoff 0042. No engine calls.

`quads` maps every drawable entity's destination rectangle through its inherited
2D transform and a camera to screen pixels; the analysis uses these quads to
crop entities and to bound where pixels may change between ticks. `preview` is a
coarse nearest-sampling CPU raster used only to check the layout before any GPU
capture. It approximates nine-slice and linear sampling as simple nearest and is
never evidence.
"""

import math

import fb_png
import fb_scene as fs


def local_matrix(item, tick):
    x, y = fs.position(item, tick)
    c, s = math.cos(math.radians(item["r"])), math.sin(math.radians(item["r"]))
    sx, sy = item["s"]
    return [[c * sx, -s * sy, x], [s * sx, c * sy, y], [0.0, 0.0, 1.0]]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def global_matrices(sc, tick):
    out = {}
    for item in sc.items:
        m = local_matrix(item, tick)
        if item["parent"]:
            m = mul(out[item["parent"]], m)
        out[item["name"]] = m
    return out


class Camera:
    def __init__(self, sc, name, width, height):
        item = sc.by_name[name]
        self.cx, self.cy = item["t"]
        projection = item["projection"]
        if projection["kind"] == "pixel_perfect":
            ref_w, ref_h = projection["reference_resolution"]
            scale = max(1, min(width // ref_w, height // ref_h))
            self.ppu = projection["pixels_per_unit"] * scale
        else:
            self.ppu = height / projection["vertical_size"]
        self.width, self.height = width, height

    def to_screen(self, wx, wy):
        return (self.width / 2 + (wx - self.cx) * self.ppu, self.height / 2 - (wy - self.cy) * self.ppu)

    def to_world(self, sx, sy):
        return (self.cx + (sx - self.width / 2) / self.ppu, self.cy - (sy - self.height / 2) / self.ppu)


def apply(m, x, y):
    return (m[0][0] * x + m[0][1] * y + m[0][2], m[1][0] * x + m[1][1] * y + m[1][2])


def sprite_corners(item):
    w, h = item["size"]
    px, py = item["pivot"]
    return [(-px * w, -py * h), ((1 - px) * w, -py * h), ((1 - px) * w, (1 - py) * h), (-px * w, (1 - py) * h)]


def quads(sc, tick, camera):
    """{name: {"screen": 4 corners, "key": painter key}} for sprites, anims and maps."""
    mats = global_matrices(sc, tick)
    out = {}
    for item in sc.items:
        if item["kind"] in ("sprite", "anim"):
            corners = sprite_corners(item)
            key = (item["layer"], item["order"], item["slot"])
        elif item["kind"] == "map":
            cw, ch = item["cell"]
            w, h = item["size"]
            corners = [(0.0, 0.0), (w * cw, 0.0), (w * cw, h * ch), (0.0, h * ch)]
            key = (0, 0, item["slot"])
        else:
            continue
        m = mats[item["name"]]
        out[item["name"]] = {"screen": [camera.to_screen(*apply(m, x, y)) for x, y in corners], "key": key}
    return out


def inside(poly, x, y):
    sign = 0
    for i in range(4):
        (x0, y0), (x1, y1) = poly[i], poly[(i + 1) % 4]
        cross = (x1 - x0) * (y - y0) - (y1 - y0) * (x - x0)
        if abs(cross) < 1e-12:
            continue
        s = 1 if cross > 0 else -1
        if sign == 0:
            sign = s
        elif s != sign:
            return False
    return True


def bbox(poly, width, height, pad=0):
    xs, ys = [p[0] for p in poly], [p[1] for p in poly]
    x0 = max(0, math.floor(min(xs)) - pad)
    y0 = max(0, math.floor(min(ys)) - pad)
    x1 = min(width, math.ceil(max(xs)) + pad)
    y1 = min(height, math.ceil(max(ys)) + pad)
    return x0, y0, max(x0, x1), max(y0, y1)


def mask(poly, width, height, dilate=2):
    """Pixel indices whose centre lies in the quad, dilated by `dilate` pixels."""
    x0, y0, x1, y1 = bbox(poly, width, height, dilate + 1)
    core = set()
    for y in range(y0, y1):
        for x in range(x0, x1):
            if inside(poly, x + 0.5, y + 0.5):
                core.add((x, y))
    grown = set()
    for (x, y) in core:
        for dy in range(-dilate, dilate + 1):
            for dx in range(-dilate, dilate + 1):
                if 0 <= x + dx < width and 0 <= y + dy < height:
                    grown.add((x + dx, y + dy))
    return grown


# ----------------------------------------------------------------- preview --

def load_art(art_dir):
    names = ["f0", "f1", "f2", "f3", "f4", "t0", "t1", "t2", "sheet", "badge", "plates"]
    return {n: fb_png.read(art_dir / f"{n}.png") for n in names}


def preview(sc, tick, camera, art, manifest, background=(0, 0, 0)):
    """Approximate RGBA rows of the scene at `tick`. Exact only for opaque/binary-alpha,
    untinted, axis-aligned nearest simple sprites and tiles away from ties."""
    states = fs.expected_states(sc, [tick])
    mats = global_matrices(sc, tick)
    w, h = camera.width, camera.height
    canvas = [[tuple(background)] * w for _ in range(h)]
    draws = []
    for item in sc.items:
        if item["kind"] in ("sprite", "anim"):
            if item["kind"] == "anim":
                name = item["frames"][states[item["name"]][tick][0]]
                if item["texture"] == "sheet":
                    image, region = art["sheet"], manifest["sheet"]["regions"][name]
                else:
                    image, region = art[name], [0, 0, art[name].width, art[name].height]
            else:
                kind = item["region"][0]
                if kind == "plate":
                    image, region = art["plates"], manifest["plates"][item["region"][1]]
                elif kind == "badge":
                    image, region = art["badge"], [0, 0, *manifest["badge"]]
                else:
                    image = art[item["region"][1]]
                    region = [0, 0, image.width, image.height]
            draws.append(((item["layer"], item["order"], item["slot"], 0, 0, 0), item, image, region, None))
        elif item["kind"] == "map":
            cw, ch = item["cell"]
            mw = item["size"][0]
            for lay in item["layers"]:
                for index, tile in enumerate(fs.cells_bottom_up(lay["rows"])):
                    if tile is None:
                        continue
                    y, x = divmod(index, mw)
                    cell = {"size": [cw, ch], "pivot": [0.0, 0.0], "flip": [False, False], "tint": [1, 1, 1, 1],
                            "cell": (x * cw, y * ch)}
                    draws.append(((lay["layer"], lay["order"], item["slot"], lay["id"], y, x), item, art[tile],
                                  [0, 0, 8, 8], cell))
    draws.sort(key=lambda d: d[0])
    for _, item, image, region, cell in draws:
        geom = cell or item
        m = mats[item["name"]]
        if cell:
            m = mul(m, [[1, 0, cell["cell"][0]], [0, 1, cell["cell"][1]], [0, 0, 1]])
        corners = [camera.to_screen(*apply(m, x, y)) for x, y in sprite_corners(geom)]
        det = m[0][0] * m[1][1] - m[0][1] * m[1][0]
        inv = [[m[1][1] / det, -m[0][1] / det], [-m[1][0] / det, m[0][0] / det]]
        x0, y0, x1, y1 = bbox(corners, w, h)
        sw, sh = geom["size"]
        px, py = geom["pivot"]
        rx, ry, rw, rh = region
        for sy in range(y0, y1):
            for sx in range(x0, x1):
                if not inside(corners, sx + 0.5, sy + 0.5):
                    continue
                wx, wy = camera.to_world(sx + 0.5, sy + 0.5)
                lx = inv[0][0] * (wx - m[0][2]) + inv[0][1] * (wy - m[1][2])
                ly = inv[1][0] * (wx - m[0][2]) + inv[1][1] * (wy - m[1][2])
                u = (lx + px * sw) / sw
                v = 1.0 - (ly + py * sh) / sh
                if geom["flip"][0]:
                    u = 1.0 - u
                if geom["flip"][1]:
                    v = 1.0 - v
                tx = rx + min(rw - 1, max(0, int(u * rw)))
                ty = ry + min(rh - 1, max(0, int(v * rh)))
                r, g, b, a = image.px(tx, ty)
                alpha = a / 255 * geom["tint"][3]
                if alpha <= 0:
                    continue
                tint = geom["tint"]
                src = (r * tint[0], g * tint[1], b * tint[2])
                dst = canvas[sy][sx]
                canvas[sy][sx] = tuple(round(s * alpha + d * (1 - alpha)) for s, d in zip(src, dst))
    return [bytes(c for p in row for c in (*p, 255)) for row in canvas]
