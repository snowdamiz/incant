#!/usr/bin/env python3
"""CPU geometry for the quality follow-up (handoff 0072). Standard library only.

Exact scene facts the model needs, from the frozen fixture constants
(visual/review/temporal_visual_review.rs: CAMERA_AT, CAMERA_PITCH, fov 50, 960x540,
lookdev_mesh, CARD_AT, the 128x128 MASK grille, MOVER_START/STEP, the sun at
elevation 38 / azimuth 240). It only intersects rays with planes and boxes to
report which surface a sample position sees and at what view depth: no shading,
no renderer. Colours used later come from the published temporal-off PNGs.

  python3 -I -B geometry.py
"""

import math

W, H, FOV = 960.0, 540.0, math.radians(50.0)
CAMERA_AT, PITCH = (0.4, 4.2, 8.5), -0.42
CARD_AT = (2.4, 1.15, -0.6)
MOVER_START, MOVER_STEP = (-2.6, 1.0, -1.0), 0.35
SUN = (0.6824, -0.6157, 0.394)  # light travel direction (shadow casting)


def rot_x(a, v):
    c, s = math.cos(a), math.sin(a)
    return (v[0], c * v[1] - s * v[2], s * v[1] + c * v[2])


BASIS = [rot_x(PITCH, axis) for axis in ((1, 0, 0), (0, 1, 0), (0, 0, 1))]  # right, up, back


def project(p):
    d = [p[k] - CAMERA_AT[k] for k in range(3)]
    x, y, z = (sum(d[k] * a[k] for k in range(3)) for a in BASIS)
    t = math.tan(FOV / 2)
    return ((x / (-z * t * W / H)) + 1) / 2 * W, (1 - y / (-z * t)) / 2 * H, -z


def ray(sx, sy):
    """World direction through continuous pixel position (sx, sy) (pixel centres at +0.5)."""
    t = math.tan(FOV / 2)
    nx, ny = (sx / W) * 2 - 1, 1 - (sy / H) * 2
    local = (nx * t * W / H, ny * t, -1.0)
    d = [sum(local[i] * BASIS[i][k] for i in range(3)) for k in range(3)]
    n = math.sqrt(sum(v * v for v in d))
    return [v / n for v in d]


def view_depth(p):
    return project(p)[2]


def hit_box(o, d, centre, half):
    tmin, tmax = -math.inf, math.inf
    for k in range(3):
        lo, hi = centre[k] - half[k], centre[k] + half[k]
        if abs(d[k]) < 1e-12:
            if not lo <= o[k] <= hi:
                return None
            continue
        a, b = (lo - o[k]) / d[k], (hi - o[k]) / d[k]
        tmin, tmax = max(tmin, min(a, b)), min(tmax, max(a, b))
    return tmin if tmax >= tmin and tmin > 0 else None


def hit_ground(o, d):
    return -o[1] / d[1] if d[1] < 0 else None


POSTS = [((-4.5, 0.75, -4.0 - 4.0 * k), (0.12, 0.75, 0.12)) for k in range(19)]
WALL = ((-0.6, 0.6, -2.2), (1.4, 0.6, 0.01))


def first_hit(sx, sy, boxes):
    o, d = CAMERA_AT, ray(sx, sy)
    best = (hit_ground(o, d), "ground")
    for name, (c, h) in boxes:
        t = hit_box(o, d, c, h)
        if t is not None and (best[0] is None or t < best[0]):
            best = (t, name)
    if best[0] is None:
        return "sky", None
    p = [o[k] + d[k] * best[0] for k in range(3)]
    return best[1], view_depth(p)


def grille_solid(u, v):
    x, y = min(int(u * 128), 127), min(int(v * 128), 127)
    return x % 4 == 0 or y % 6 == 0 or (x + y) % 13 == 0


def card_hit(sx, sy):
    """The card is a 2x2 quad at CARD_AT facing +Z; CARD_UV maps x -> u, y -> 1 - v."""
    o, d = CAMERA_AT, ray(sx, sy)
    if abs(d[2]) < 1e-12:
        return None
    t = (CARD_AT[2] - o[2]) / d[2]
    p = [o[k] + d[k] * t for k in range(3)]
    u, v = (p[0] - (CARD_AT[0] - 1)) / 2, 1 - (p[1] - (CARD_AT[1] - 1)) / 2
    if not (0 <= u < 1 and 0 <= v < 1):
        return None
    return t, u, v, view_depth(p)


def profile(label, y, x0, x1, step, sampler):
    print(f"== {label}: row y={y} (continuous), x {x0}..{x1}, step {step}")
    runs, prev, start = [], None, x0
    x = x0
    while x <= x1 + 1e-9:
        cur = sampler(x, y)
        key = cur[0]
        if prev is not None and key != prev[0]:
            runs.append((start, x, prev))
            start = x
        prev = cur
        x += step
    runs.append((start, x1, prev))
    for a, b, (name, depth) in runs:
        print(f"   x [{a:8.3f}, {b:8.3f})  width {b - a:6.3f} px  {name:8s} depth {depth if depth is None else round(depth, 3)}")


def main():
    t = math.tan(FOV / 2)
    print(f"camera {CAMERA_AT}, pitch {PITCH}, fov 50, {int(W)}x{int(H)}")
    # 1. Far-post gaps where the saturated pixels sit (columns 424-436, rows 48-57).
    boxes = [(f"post{k}", b) for k, b in enumerate(POSTS)]
    for y in (50.5, 53.5, 56.5):
        profile("far posts", y, 423.0, 437.0, 1 / 64, lambda x, yy: first_hit(x, yy, boxes))
    # 2. The MASK card around the two new saturated pixels (556, 242) and (542, 243).
    def card_sampler(x, y):
        hit = card_hit(x, y)
        if hit is not None and grille_solid(hit[1], hit[2]):
            return "bar", hit[3]
        if hit is not None:
            behind = first_hit(x, y, [("wall", WALL)])
            return ("hole/" + behind[0]), behind[1]
        behind = first_hit(x, y, [("wall", WALL)])
        return ("off/" + behind[0]), behind[1]
    for y in (240.5, 242.5, 243.5):
        profile("grille card", y, 538.0, 560.0, 1 / 64, card_sampler)
    texel = []
    for u in (0.0, 1.0):
        texel.append(project((CARD_AT[0] - 1 + 2 * u, CARD_AT[1], CARD_AT[2])))
    print(f"card width {texel[1][0] - texel[0][0]:.2f} px for 128 texels -> {(texel[1][0] - texel[0][0]) / 128:.3f} px/texel; card depth {texel[0][2]:.3f}")
    # 3. The moving cube's trailing edge and its shadow at frames 7 and 8.
    for k in (7, 8):
        cx = MOVER_START[0] + MOVER_STEP * k
        near_left = [project((cx - 0.5, yy, MOVER_START[2] + 0.5)) for yy in (0.6, 1.0, 1.4)]
        wall = first_hit(near_left[1][0] - 1.0, near_left[1][1], [("wall", WALL)])
        print(f"cube f{k}: near-left edge x/y/depth {[tuple(round(v, 3) for v in p) for p in near_left]}; "
              f"1 px left sees {wall[0]} at depth {round(wall[1], 3)}")
        corners = [(cx + sx * 0.5, cy, MOVER_START[2] + sz * 0.5) for sx in (-1, 1) for cy in (0.5, 1.5) for sz in (-1, 1)]
        ground = [(p[0] + SUN[0] * p[1] / -SUN[1], 0.0, p[2] + SUN[2] * p[1] / -SUN[1]) for p in corners]
        screen = [project(g) for g in ground]
        xs, ys = [s[0] for s in screen], [s[1] for s in screen]
        print(f"cube f{k} shadow (8 projected corners) screen bbox x [{min(xs):.1f}, {max(xs):.1f}] y [{min(ys):.1f}, {max(ys):.1f}]; "
              f"ground x [{min(g[0] for g in ground):.2f}, {max(g[0] for g in ground):.2f}] z [{min(g[2] for g in ground):.2f}, {max(g[2] for g in ground):.2f}]")
    for sx, sy in ((590.5, 304.5), (436.5, 215.5)):
        o, d = CAMERA_AT, ray(sx, sy)
        tg = hit_ground(o, d)
        p = [o[i] + d[i] * tg for i in range(3)]
        print(f"pixel ({sx}, {sy}) -> ground ({p[0]:.3f}, 0, {p[2]:.3f}), depth {view_depth(p):.3f}")


if __name__ == "__main__":
    main()
