"""Grille-shadow and card regions for handoff 0076, derived on the CPU from the
frozen fixture constants. Standard library only; a derivation, not a measurement.

The projection and sun math are 0072's committed diagnosis
(fixed-sharpness-inspection/cascade_texels.py at 48a688c5), reused read-only:
the grille card's four corners are carried along the sun's travel direction to
the ground (y = 0) and projected through the review camera into output pixels.
`cascade_lines` reproduces cascade_texels.txt's card-shadow lines exactly, so
the region is the one the prior diagnosis located. Constants are checked
against the fixture source by selfcheck.py.

  python3 -I -B regions.py    # prints every pose's polygons as JSON
"""

import json
import math

FOV = math.radians(50.0)
CAMERA_AT, PITCH = (0.4, 4.2, 8.5), -0.42
CARD_AT, CARD_HALF = (2.4, 1.15, -0.6), 1.0  # 2x2 quad facing +Z, scale 1
SPHERE_AT, SPHERE_RADIUS = (0.6, 0.8, 1.0), 0.8  # lookdev_mesh's sphere
SUN_ELEVATION, SUN_AZIMUTH = 38.0, 240.0
OUTPUT = (960.0, 540.0)
VIEWPORT = (37.0, 19.0, 885.0, 501.0)
FULL = (0.0, 0.0) + OUTPUT
# The poses whose PNGs exist: the run's own pan f24 pair, and the retained
# 0072 full-frame pairs used only to calibrate thresholds.
POSES = {"viewport-pan-f24": (VIEWPORT, 8), "full-still": (FULL, 0), "full-pan-f6": (FULL, 6),
         "viewport-still": (VIEWPORT, 0)}


def rot_x(a, v):
    c, s = math.cos(a), math.sin(a)
    return (v[0], c * v[1] - s * v[2], s * v[1] + c * v[2])


def rot_y(a, v):
    c, s = math.cos(a), math.sin(a)
    return (c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2])


def camera(step):
    """The fixture's camera_world(step): Ry(-0.012 k) * Rx(pitch), moved 0.12 k along x."""
    position = (CAMERA_AT[0] + 0.12 * step, CAMERA_AT[1], CAMERA_AT[2])
    basis = [rot_y(-0.012 * step, rot_x(PITCH, axis)) for axis in ((1, 0, 0), (0, 1, 0), (0, 0, 1))]
    return position, basis


def project(point, step, rect):
    """(view depth, (x, y) output pixel) of a world point; rect is the viewport."""
    position, (right, up, back) = camera(step)
    d = [point[k] - position[k] for k in range(3)]
    x, y, z = (sum(d[k] * axis[k] for k in range(3)) for axis in (right, up, back))
    depth = -z
    aspect = rect[2] / rect[3]
    ndc = (x / (depth * math.tan(FOV / 2) * aspect), y / (depth * math.tan(FOV / 2)))
    return depth, (rect[0] + (ndc[0] + 1) / 2 * rect[2], rect[1] + (1 - ndc[1]) / 2 * rect[3])


def sun():
    """Travel direction: Ry(azimuth) * Rx(-elevation) applied to the light's -Z."""
    return rot_y(math.radians(SUN_AZIMUTH), rot_x(-math.radians(SUN_ELEVATION), (0.0, 0.0, -1.0)))


def card_corners():
    """(-1,-1), (-1,1), (1,-1), (1,1) in card units, as cascade_texels.py orders them."""
    return [(CARD_AT[0] + sx * CARD_HALF, CARD_AT[1] + sy * CARD_HALF, CARD_AT[2])
            for sx in (-1, 1) for sy in (-1, 1)]


def ground_corners():
    s = sun()
    return [(p[0] - p[1] / s[1] * s[0], 0.0, p[2] - p[1] / s[1] * s[2]) for p in card_corners()]


def ring(points):
    """cascade_texels order (-1,-1), (-1,1), (1,-1), (1,1) -> a simple polygon ring."""
    return [points[0], points[2], points[3], points[1]]


def pose(name):
    rect, step = POSES[name]
    shadow = [project(p, step, rect)[1] for p in ground_corners()]
    card = [project(p, step, rect)[1] for p in card_corners()]
    return {"rect": list(rect), "pan_step": step,
            "grille_shadow": [[round(x, 3), round(y, 3)] for x, y in ring(shadow)],
            "card": [[round(x, 3), round(y, 3)] for x, y in ring(card)]}


def grille_sampling(texels=(("viewport unguarded", 0.031495), ("viewport guarded", 0.031618),
                            ("960x540", 0.031740))):
    """How cascade 1 samples the one-texel grille (0072's cascade-1 texel sizes):
    a grille texel's projected length along the card's x and y axes on the sun's
    image plane, bar widths and bar spacings (every 4th column, every 6th row) in
    shadow texels. Spacing below 2 texels aliases; bars thinner than one texel
    are hit only where a texel centre happens to land on one."""
    s = sun()
    texel = 2 * CARD_HALF / 128
    along = {"x": math.sqrt(1 - s[0] ** 2) * texel, "y": math.sqrt(1 - s[1] ** 2) * texel}
    return {label: {"bar_width_texels": {k: round(v / size, 3) for k, v in along.items()},
                    "vertical_bar_spacing_texels": round(4 * along["x"] / size, 3),
                    "horizontal_bar_spacing_texels": round(6 * along["y"] / size, 3)}
            for label, size in texels}


def cascade_lines():
    """The 0072 cascade_texels.txt card-shadow lines for the viewport poses."""
    lines = []
    for step, label in ((0, "still pose"), (8, "last pan pose (f24)")):
        lines.append(f"== shadow of the card on the ground (y = 0), {label}")
        for p in ground_corners():
            depth, (sx, sy) = project(p, step, VIEWPORT)
            lines.append(f"  ground ({p[0]:+.3f}, {p[2]:+.3f}): view depth {depth:.3f} (cascade 1 or blend), "
                         f"viewport pixel ({sx:.1f}, {sy:.1f})")
    return lines


# ------------------------------------------------------------------ pixel masks

def inside(polygon, x, y):
    """Even-odd point-in-polygon test at a pixel centre (x + 0.5, y + 0.5)."""
    px, py = x + 0.5, y + 0.5
    hit = False
    for (x0, y0), (x1, y1) in zip(polygon, polygon[1:] + polygon[:1]):
        if (y0 > py) != (y1 > py) and px < x0 + (py - y0) * (x1 - x0) / (y1 - y0):
            hit = not hit
    return hit


def distance_to_polygon(polygon, x, y):
    px, py = x + 0.5, y + 0.5
    best = math.inf
    for (x0, y0), (x1, y1) in zip(polygon, polygon[1:] + polygon[:1]):
        dx, dy = x1 - x0, y1 - y0
        t = max(0.0, min(1.0, ((px - x0) * dx + (py - y0) * dy) / (dx * dx + dy * dy)))
        best = min(best, math.hypot(px - (x0 + t * dx), py - (y0 + t * dy)))
    return best


def mask(polygon, grow, rect, size=OUTPUT):
    """Pixel indices inside the polygon or within `grow` px of it, clipped to rect."""
    w, h = int(size[0]), int(size[1])
    xs = [p[0] for p in polygon]
    ys = [p[1] for p in polygon]
    x0 = max(int(rect[0]), int(math.floor(min(xs) - grow)))
    x1 = min(int(rect[0] + rect[2]), int(math.ceil(max(xs) + grow)) + 1)
    y0 = max(int(rect[1]), int(math.floor(min(ys) - grow)))
    y1 = min(int(rect[1] + rect[3]), int(math.ceil(max(ys) + grow)) + 1)
    out = []
    for y in range(max(0, y0), min(h, y1)):
        for x in range(max(0, x0), min(w, x1)):
            if inside(polygon, x, y) or distance_to_polygon(polygon, x, y) <= grow:
                out.append(y * w + x)
    return out


def bbox(indices, width=int(OUTPUT[0])):
    xs = [i % width for i in indices]
    ys = [i // width for i in indices]
    return [min(xs), min(ys), max(xs) + 1, max(ys) + 1]


def sphere_hit(x, y, step, rect):
    """Whether the camera ray through output pixel centre (x + 0.5, y + 0.5) meets
    the look-dev sphere (tessellated on the GPU; this is the analytic sphere)."""
    position, (right, up, back) = camera(step)
    ndc_x = (x + 0.5 - rect[0]) / rect[2] * 2 - 1
    ndc_y = 1 - (y + 0.5 - rect[1]) / rect[3] * 2
    t = math.tan(FOV / 2)
    view = (ndc_x * t * rect[2] / rect[3], ndc_y * t, -1.0)
    d = [view[0] * right[k] + view[1] * up[k] + view[2] * back[k] for k in range(3)]
    o = [position[k] - SPHERE_AT[k] for k in range(3)]
    a = sum(v * v for v in d)
    b = 2 * sum(o[k] * d[k] for k in range(3))
    c = sum(v * v for v in o) - SPHERE_RADIUS ** 2
    return b * b - 4 * a * c >= 0


def grown(indices, grow, size=OUTPUT):
    """Chebyshev dilation of a pixel set by `grow` pixels."""
    w, h = int(size[0]), int(size[1])
    out = set()
    for i in indices:
        y, x = divmod(i, w)
        for ny in range(max(0, y - grow), min(h, y + grow + 1)):
            for nx in range(max(0, x - grow), min(w, x + grow + 1)):
                out.add(ny * w + nx)
    return out


def region(name, spec):
    """The plan's measurement region for one pose: the grille-shadow polygon grown
    by spec["shadow_grow_px"], minus the card quad and the sphere, each grown by
    spec["exclude_grow_px"], clipped to the viewport. Sorted pixel indices."""
    p = pose(name)
    rect, step = p["rect"], p["pan_step"]
    shadow = mask([tuple(v) for v in p["grille_shadow"]], spec["shadow_grow_px"], rect)
    w = int(OUTPUT[0])
    near = [i for i in mask([tuple(v) for v in p["grille_shadow"]], spec["shadow_grow_px"] + 8, rect)]
    sphere = [i for i in near if sphere_hit(i % w, i // w, step, rect)]
    card = mask([tuple(v) for v in p["card"]], 0, rect)
    excluded = grown(sphere, spec["exclude_grow_px"]) | grown(card, spec["exclude_grow_px"])
    return sorted(set(shadow) - excluded)


if __name__ == "__main__":
    print(json.dumps({"sun_travel": [round(v, 4) for v in sun()],
                      "ground_corners": [[round(p[0], 3), round(p[2], 3)] for p in ground_corners()],
                      "poses": {name: pose(name) for name in POSES},
                      "grille_sampling_cascade_1": grille_sampling(),
                      "cascade_texels_lines": cascade_lines()}, indent=1))
