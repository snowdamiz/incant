#!/usr/bin/env python3
"""Bounded CPU model of the fixed-mode temporal resolve for the 0072 quality
follow-up. Standard library only, deterministic, foreground. A numerical model, not
render evidence: it reproduces the resolve's decisions on exact scene geometry so
they can be compared with the retained GPU captures pixel by pixel.

Resolve rules, verbatim from crates/incant_render/src/temporal/resolve.wgsl at
296e0226 (byte-identical to a143b5c0): per-tap clean (alpha <= 0 -> 0), 3x3
per-channel RGBA min/max including the centre, clear taps push the depth range to
the far plane, nearest previous-depth motion with strict '<' and centre priority,
range admission widened by max(0.02, 0.05 * depth), old = p + m - dj, positive-tap
renormalised bilinear history with per-tap previous-depth admission, the registered
current (bilinear at p + j) on reset/refusal, reaction measured before the clamp
(0.5 x min(1, max-channel |h - c| / 4.0)), out = mix(c, clamp(h, min, max), w).
Host weight w = options.x from temporal/owner.rs at 296e0226: 0 for the reset frame,
then min(0.9, (n - 1) / n) for the n-th accepted sample ("warmup"); "a143" keeps
0.9; "baseline" is e23776df (history on the jittered grid, old = p + m, refusal and
reset return the raw sample, presentation is the bilinear un-jitter at p + j).
Jitter: temporal/jitter.rs SAMPLES, geometry shifted by +j, raster motion
j_prev - j - (screen displacement). Scenes, camera and sun come from geometry.py.

  python3 -I -B model.py             # reproduction against the retained GPU runs
  python3 -I -B model.py screen      # the three declared candidates (candidates.md)
  python3 -I -B model.py checks      # post-screening checks (added after screening ran)
  python3 -I -B model.py viewaid     # writes the two labelled CPU-model viewing aids (no overwrite)
"""

import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import geometry as g  # noqa: E402

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
WEIGHT, REL, ABS, THRESHOLD, STRENGTH = 0.9, 0.05, 0.02, 4.0, 0.5
FAR_PLANE = 120.0


def lin(v8):
    v = v8 / 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def srgb8(x):
    x = min(max(x, 0.0), 1.0)
    v = 12.92 * x if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055
    return int(round(v * 255))


def rgb(t):
    return tuple(lin(v) for v in t)


def host_weight(variant, t):
    """options.x for frame index t (0 = reset)."""
    if t == 0:
        return 0.0
    if variant == "baseline" or variant == "a143":
        return WEIGHT
    n = t + 1
    return min(WEIGHT, (n - 1) / n)


# ------------------------------------------------------------------ scenes
# sample(sx, sy, k) -> (rgb, alpha, view depth, (dx, dy) screen displacement since
# the previous frame, previous view depth of the same surface point)

POST, GROUND = rgb((82, 84, 91)), rgb((184, 183, 180))
BAR, HOLE = rgb((58, 163, 98)), rgb((185, 185, 184))
CUBE_FRONT, CUBE_TOP, CUBE_SIDE = rgb((164, 108, 60)), rgb((218, 152, 87)), rgb((96, 60, 33))
WALL_C, GROUND_C, BOX_C, SPHERE_C = rgb((96, 97, 103)), rgb((183, 182, 178)), rgb((200, 196, 188)), rgb((190, 187, 182))
LIT, SHADOW = rgb((201, 222, 250)), rgb((169, 205, 247))
STATIC = (0.0, 0.0)


def posts_sample(sx, sy, k):
    name, depth = g.first_hit(sx, sy, [(f"post{i}", b) for i, b in enumerate(g.POSTS)])
    return (POST if name.startswith("post") else GROUND), 1.0, depth, STATIC, depth


def grille_sample(sx, sy, k):
    hit = g.card_hit(sx, sy)
    if hit is not None and g.grille_solid(hit[1], hit[2]):
        return BAR, 1.0, hit[3], STATIC, hit[3]
    name, depth = g.first_hit(sx, sy, [("wall", g.WALL)])
    return HOLE, 1.0, depth, STATIC, depth


def hit_sphere(o, d, centre, radius):
    oc = [o[i] - centre[i] for i in range(3)]
    b = sum(oc[i] * d[i] for i in range(3))
    c = sum(v * v for v in oc) - radius * radius
    disc = b * b - c
    if disc < 0:
        return None
    t = -b - math.sqrt(disc)
    return t if t > 0 else None


STATIC_BOXES = [("wall", g.WALL), ("box", ((-2.2, 0.5, 0.5), (0.5, 0.5, 0.5)))] + \
               [(f"post{i}", b) for i, b in enumerate(g.POSTS)]


def cube_centre(k):
    return (g.MOVER_START[0] + g.MOVER_STEP * k, g.MOVER_START[1], g.MOVER_START[2])


def cube_sample(sx, sy, k):
    o, d = g.CAMERA_AT, g.ray(sx, sy)
    best, kind = g.hit_ground(o, d), "ground"
    for name, (c, h) in STATIC_BOXES:
        t = g.hit_box(o, d, c, h)
        if t is not None and (best is None or t < best):
            best, kind = t, name
    t = hit_sphere(o, d, (0.6, 0.8, 1.0), 0.8)
    if t is not None and (best is None or t < best):
        best, kind = t, "sphere"
    cc = cube_centre(k)
    t = g.hit_box(o, d, cc, (0.5, 0.5, 0.5))
    if t is not None and (best is None or t < best):
        best, kind = t, "cube"
    p = [o[i] + d[i] * best for i in range(3)]
    depth = g.view_depth(p)
    if kind != "cube":
        colour = {"wall": WALL_C, "ground": GROUND_C, "box": BOX_C, "sphere": SPHERE_C}.get(kind, POST)
        return colour, 1.0, depth, STATIC, depth
    local = [p[i] - cc[i] for i in range(3)]
    if abs(local[1] - 0.5) < 1e-6:
        colour = CUBE_TOP
    elif abs(local[2] - 0.5) < 1e-6:
        colour = CUBE_FRONT
    else:
        colour = CUBE_SIDE
    prev = (p[0] - g.MOVER_STEP, p[1], p[2])
    now_s, prev_s = g.project(p), g.project(prev)
    return colour, 1.0, depth, (now_s[0] - prev_s[0], now_s[1] - prev_s[1]), prev_s[2]


def shadow_polygon(k):
    cc = cube_centre(k)
    pts = []
    for sx in (-1, 1):
        for cy in (0.5, 1.5):
            for sz in (-1, 1):
                p = (cc[0] + sx * 0.5, cy, cc[2] + sz * 0.5)
                pts.append((p[0] + g.SUN[0] * p[1] / -g.SUN[1], p[2] + g.SUN[2] * p[1] / -g.SUN[1]))
    pts = sorted(set(pts))
    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def signed_distance(poly, x, z):
    inside = True
    best = math.inf
    n = len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        ex, ez = b[0] - a[0], b[1] - a[1]
        length = math.hypot(ex, ez)
        t = max(0.0, min(1.0, ((x - a[0]) * ex + (z - a[1]) * ez) / (length * length)))
        best = min(best, math.hypot(x - a[0] - t * ex, z - a[1] - t * ez))
        if ex * (z - a[1]) - ez * (x - a[0]) < 0:
            inside = False
    return -best if inside else best


PENUMBRA = 0.07  # ground units: least-squares fit (0.02..0.15 in 0.01 steps) to the GPU temporal-off
                 # reference, column 590 rows 298-311, row 304 x 576-608, column 600: RMS 2.67 steps
_POLYS = {}


def shadow_sample(sx, sy, k):
    o, d = g.CAMERA_AT, g.ray(sx, sy)
    t = g.hit_ground(o, d)
    p = [o[i] + d[i] * t for i in range(3)]
    poly = _POLYS.setdefault(k, shadow_polygon(k))
    s = signed_distance(poly, p[0], p[2])
    lit = min(1.0, max(0.0, 0.5 + s / (2 * PENUMBRA)))
    colour = tuple(SHADOW[i] + (LIT[i] - SHADOW[i]) * lit for i in range(3))
    depth = g.view_depth(p)
    return colour, 1.0, depth, STATIC, depth


# ---- controls (synthetic geometry on the same rules; screen coordinates of a 40x40 patch)
FG_DEPTH, BG_DEPTH = 20.0, 30.0


def diag_sample(sx, sy, k):
    fg = (sx - 20) - 0.5 * (sy - 20) < 0  # static step, slope 1/2
    return (POST if fg else GROUND), 1.0, (FG_DEPTH if fg else BG_DEPTH), STATIC, (FG_DEPTH if fg else BG_DEPTH)


def thin_sample(sx, sy, k):
    d = (sx - sy) * 0.70710678
    fg = 0.1 <= d < 0.1 + 0.70710678  # static 1-px-wide 45-degree line
    return (POST if fg else GROUND), 1.0, (FG_DEPTH if fg else BG_DEPTH), STATIC, (FG_DEPTH if fg else BG_DEPTH)


def aligned_sample(sx, sy, k):
    fg = sx < 20.0  # pixel-aligned static edge (orthographic texture edge)
    return (POST if fg else GROUND), 1.0, (FG_DEPTH if fg else BG_DEPTH), STATIC, (FG_DEPTH if fg else BG_DEPTH)


OCCLUDER_DEPTH, OCCLUDER_SPEED = 40.0, 4.0


def occluder_sample(sx, sy, k):
    """The far posts plus a 6-px-wide occluder in front, moving right 4 px/frame;
    it crosses the patch during frames 3-9 and has left it from frame 11."""
    left = 404.0 + OCCLUDER_SPEED * k
    if left <= sx < left + 6.0:
        return CUBE_FRONT, 1.0, OCCLUDER_DEPTH, (OCCLUDER_SPEED, 0.0), OCCLUDER_DEPTH
    return posts_sample(sx, sy, k)


def sweep_sample(sx, sy, k):
    """The far posts (static geometry) under a shadow edge sweeping right 2.5 px/frame:
    a pure shading change at static aliasing sites (no motion vectors)."""
    colour, alpha, depth, motion, prev = posts_sample(sx, sy, k)
    edge = 410.0 + 2.5 * k
    lit = min(1.0, max(0.0, (sx - edge) / 2.0 + 0.5))
    factor = 0.45 + 0.55 * (1 - lit)
    return tuple(v * factor for v in colour), alpha, depth, motion, prev


DESPAWN_BOX = (425.3, 433.7, 47.4, 56.6)


def despawn_sample(sx, sy, k):
    """The far posts behind a static box (depth 40) that is removed, without motion, at frame 8."""
    if k < 8 and DESPAWN_BOX[0] <= sx < DESPAWN_BOX[1] and DESPAWN_BOX[2] <= sy < DESPAWN_BOX[3]:
        return CUBE_FRONT, 1.0, OCCLUDER_DEPTH, STATIC, OCCLUDER_DEPTH
    return posts_sample(sx, sy, k)


def checker(light, dark):
    def sample(sx, sy, k):
        """A flat single-depth checker of 1.5-px texels (nearest-sampled texel edges)."""
        odd = (math.floor(sx / 1.5) + math.floor(sy / 1.5)) % 2
        return (light if odd else dark), 1.0, FG_DEPTH, STATIC, FG_DEPTH
    return sample


SCENES = {
    # name: (sampler, x0, y0, width, height, frame k for each frame t = 0..)
    "posts": (posts_sample, 418, 44, 24, 18, [0] * 16),
    "grille": (grille_sample, 532, 230, 32, 22, [0] * 16),
    "cube": (cube_sample, 270, 168, 250, 82, list(range(1, 9))),
    "shadow": (shadow_sample, 560, 290, 52, 26, list(range(1, 9))),
    "diag": (diag_sample, 0, 0, 40, 40, [0] * 16),
    "thin": (thin_sample, 0, 0, 40, 40, [0] * 16),
    "aligned": (aligned_sample, 0, 0, 40, 40, [0] * 16),
    "occluder": (occluder_sample, 418, 44, 24, 18, list(range(16))),
    "sweep": (sweep_sample, 418, 44, 24, 18, list(range(16))),
    "penumbra": (shadow_sample, 560, 290, 52, 26, [8] * 16),
    "despawn": (despawn_sample, 418, 44, 24, 18, list(range(16))),
    "checker": (checker(rgb((200, 200, 200)), rgb((60, 60, 60))), 0, 0, 40, 40, [0] * 16),
}


# ------------------------------------------------------------------ raster

class Patch:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.n = w * h


def raster(sampler, x0, y0, w, h, k, jitter):
    """The jittered current frame: per pixel colour, alpha, depth and motion."""
    out = []
    for y in range(h):
        for x in range(w):
            sx, sy = x0 + x + 0.5 - jitter[0], y0 + y + 0.5 - jitter[1]
            out.append(sampler(sx, sy, k))
    return out


def reference(sampler, x0, y0, w, h, k):
    """Temporal-off reference: the same scene sampled at pixel centres."""
    return [tuple(f[0]) for f in raster(sampler, x0, y0, w, h, k, (0.0, 0.0))]


# ------------------------------------------------------------------ resolve

def clean(v):
    return v if v[3] > 0 else (0.0, 0.0, 0.0, 0.0)


def bilinear(values, w, h, qx, qy):
    bx, by = math.floor(qx), math.floor(qy)
    fx, fy = qx - bx, qy - by
    acc = [0.0, 0.0, 0.0, 0.0]
    for dy, wy in ((0, 1 - fy), (1, fy)):
        for dx, wx in ((0, 1 - fx), (1, fx)):
            wt = wx * wy
            if wt > 0:
                v = clean(values[min(max(by + dy, 0), h - 1) * w + min(max(bx + dx, 0), w - 1)])
                for c in range(4):
                    acc[c] += v[c] * wt
    return clean(tuple(acc))


class State:
    def __init__(self):
        self.history = None  # output (unjittered grid; jittered grid for the baseline)
        self.prev_depth = None
        self.memory = None  # C2/C3 per-pixel glimpse memory (R8Uint in the source plan)


# Candidate constants, fixed before screening (candidates.md).
GLIMPSE_REL, GLIMPSE_ABS, MEMORY_FRAMES, STILL_EPS = 0.01, 0.002, 8, 1 / 64


def history_fetch(state, w, h, taps, near, far):
    """Per-tap previous-depth admission exactly as the shader, plus the unconditioned
    ("loose") sums a held pixel uses."""
    acc, loose = [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]
    mass = loose_mass = 0.0
    depths = []
    for tx, ty, wt in taps:
        if wt == 0 or not (0 <= tx < w and 0 <= ty < h):
            continue
        kk = ty * w + tx
        d = state.prev_depth[kk]
        colour = clean(state.history[kk])
        depths.append(d)
        for ch in range(4):
            loose[ch] += colour[ch] * wt
        loose_mass += wt
        if near <= d <= far:
            for ch in range(4):
                acc[ch] += colour[ch] * wt
            mass += wt
    return acc, mass, loose, loose_mass, depths


def resolve_frame(frame, state, w, h, t, variant, candidate=None, trace=None):
    """One resolve pass. frame: list of (rgb, alpha, depth, (dx, dy), expected previous depth).
    candidate None runs the shader verbatim; C1/C2/C3 only decide when a still pixel
    holds its unconditioned history (candidates.md)."""
    j = SAMPLES[t % 8]
    jp = SAMPLES[(t - 1) % 8]
    creg = variant != "baseline"
    cur = [tuple(f[0]) + (f[1],) for f in frame]
    dj = (jp[0] - j[0], jp[1] - j[1])
    weight_x = host_weight(variant, t)
    out = []
    new_memory = [0] * (w * h) if candidate in ("C2", "C3") else None
    for i in range(w * h):
        x, y = i % w, i // w
        c = clean(cur[i])

        def registered(path):
            value = bilinear(cur, w, h, x + j[0], y + j[1]) if creg else c
            note(path, value)
            return value

        def note(path, value, h_val=None):
            if trace is not None and i in trace:
                trace[i].append((t, path, c, h_val, value))
        if t == 0 or state.history is None:
            out.append(registered("X"))
            continue
        lo, hi = list(c), list(c)
        near, far = FAR_PLANE, 0.1
        if c[3] > 0:
            m = (dj[0] - frame[i][3][0], dj[1] - frame[i][3][1])
            closest = frame[i][4]
        else:
            m, closest = (dj if creg else (0.0, 0.0)), math.inf
        all_still = True
        for ny in (y - 1, y, y + 1):
            for nx in (x - 1, x, x + 1):
                k = min(max(ny, 0), h - 1) * w + min(max(nx, 0), w - 1)
                n = clean(cur[k])
                for ch in range(4):
                    lo[ch] = min(lo[ch], n[ch])
                    hi[ch] = max(hi[ch], n[ch])
                if n[3] == 0:
                    far = FAR_PLANE
                    continue
                cand = (dj[0] - frame[k][3][0], dj[1] - frame[k][3][1])
                if abs(frame[k][3][0]) >= STILL_EPS or abs(frame[k][3][1]) >= STILL_EPS:
                    all_still = False
                z = frame[k][4]
                near, far = min(near, z), max(far, z)
                if z < closest:
                    m, closest = cand, z
        if far < near:
            out.append(registered("D"))
            continue
        near_raw, far_raw = near, far
        near -= max(ABS, REL * near)
        far += max(ABS, REL * far)
        ox = x + 0.5 + m[0] - (dj[0] if creg else 0.0)
        oy = y + 0.5 + m[1] - (dj[1] if creg else 0.0)
        if not (0 <= ox < w and 0 <= oy < h):
            out.append(registered("O"))
            continue
        moving = abs(m[0] - dj[0]) >= STILL_EPS or abs(m[1] - dj[1]) >= STILL_EPS
        qx, qy = ox - 0.5, oy - 0.5
        bx, by = math.floor(qx), math.floor(qy)
        fx, fy = qx - bx, qy - by
        taps = [(bx + dx, by + dy, wx * wy) for dy, wy in ((0, 1 - fy), (1, fy)) for dx, wx in ((0, 1 - fx), (1, fx))]
        acc, mass, loose, loose_mass, depths = history_fetch(state, w, h, taps, near, far)
        hold = False
        if candidate in ("C1", "C2", "C3"):
            # The pixel's own previous sample lies inside the hull of the current 3x3
            # sample positions (|dj| <= 0.75 px), so on any smooth surface its depth is
            # inside [near_raw, far_raw]: a glimpse means it saw a different surface.
            still = c[3] > 0 and all_still and not moving
            p_c = state.prev_depth[i]
            far_g = still and p_c > far_raw * (1 + GLIMPSE_REL) + GLIMPSE_ABS
            near_g = still and p_c < near_raw * (1 - GLIMPSE_REL) - GLIMPSE_ABS
            if candidate == "C1":
                hold = far_g
            else:
                previous = state.memory[i] if state.memory is not None else 0
                if not still:
                    new_memory[i] = 0
                elif far_g or near_g:
                    new_memory[i] = MEMORY_FRAMES
                else:
                    new_memory[i] = max(previous - 1, 0)
                if candidate == "C2":
                    hold = far_g or (near_g and previous > 0)
                else:
                    hold = far_g or (still and previous > 0)
        hold = hold and loose_mass > 0
        if hold:
            h_val = clean(tuple(v / loose_mass for v in loose))
            target = h_val
        else:
            if not mass > 0:
                out.append(registered("R"))
                continue
            h_val = clean(tuple(v / mass for v in acc))
            target = tuple(min(max(h_val[ch], lo[ch]), hi[ch]) for ch in range(4))
        change = max(abs(h_val[ch] - c[ch]) for ch in range(3))
        reaction = min(max(change / THRESHOLD, 0.0), 1.0)
        wgt = weight_x * (1 - STRENGTH * reaction)
        out.append(clean(tuple(c[ch] + (target[ch] - c[ch]) * wgt for ch in range(4))))
        if hold:
            path = "H"
        elif any(abs(target[ch] - h_val[ch]) > 1e-9 for ch in range(3)):
            path = "C" if mass > 0.999 else "c"
        else:
            path = "A" if mass > 0.999 else "a"
        note(path, out[-1], h_val)
    state.history = out
    state.prev_depth = [f[2] for f in frame]
    if new_memory is not None:
        state.memory = new_memory
    if creg:
        return out
    return [bilinear(out, w, h, x + j[0], y + j[1]) for y in range(h) for x in range(w)]


_FRAMES = {}


def frames(scene):
    """The jittered current frames of a scene (cached: identical for every variant)."""
    if scene not in _FRAMES:
        sampler, x0, y0, w, h, ks = SCENES[scene]
        _FRAMES[scene] = [raster(sampler, x0, y0, w, h, k, SAMPLES[t % 8]) for t, k in enumerate(ks)]
    return _FRAMES[scene]


def run(scene, variant, candidate=None, trace=None):
    sampler, x0, y0, w, h, ks = SCENES[scene]
    state = State()
    return [resolve_frame(frame, state, w, h, t, variant, candidate, trace) for t, frame in enumerate(frames(scene))]


# ------------------------------------------------------------------ metrics

def to8(px):
    return tuple(srgb8(v) for v in px[:3])


def step(a, b):
    return max(abs(x - y) for x, y in zip(a, b))


def edges(ref8, w, h, threshold=24, dilate=True):
    raw = [False] * (w * h)
    for y in range(h):
        for x in range(w):
            i = y * w + x
            if x + 1 < w and step(ref8[i], ref8[i + 1]) > threshold:
                raw[i] = raw[i + 1] = True
            if y + 1 < h and step(ref8[i], ref8[i + w]) > threshold:
                raw[i] = raw[i + w] = True
    if not dilate:
        return raw
    out = list(raw)
    for y in range(h):
        for x in range(w):
            if raw[y * w + x]:
                for ny in range(max(0, y - 1), min(h, y + 2)):
                    for nx in range(max(0, x - 1), min(w, x + 2)):
                        out[ny * w + nx] = True
    return out


def interior(i, w, h, border=3):
    x, y = i % w, i // w
    return border <= x < w - border and border <= y < h - border


def stability(shown, w, h, ref8):
    """GPU-equivalent statistics over frames 9-16 (t = 8..15)."""
    s8 = [[to8(p) for p in f] for f in shown]
    edge = edges(ref8, w, h)
    peaks_map = [0] * (w * h)     # activity map: pairs (9,10)..(15,16)
    edge_max = 0                  # frames_9_16_edge_max: pairs (8,9)..(15,16)
    for t in range(8, 16):
        for i in range(w * h):
            if not interior(i, w, h):
                continue
            d = step(s8[t][i], s8[t - 1][i])
            if t >= 9:
                peaks_map[i] = max(peaks_map[i], d)
            if edge[i]:
                edge_max = max(edge_max, d)
    ids = [i for i in range(w * h) if interior(i, w, h)]
    same = [step(s8[15][i], s8[7][i]) for i in ids if not edge[i]]
    return {"ge33": sum(1 for i in ids if peaks_map[i] >= 33), "ge64": sum(1 for i in ids if peaks_map[i] >= 64),
            "edge_max": edge_max, "peak_max": max(peaks_map[i] for i in ids),
            "same_phase_changed": sum(1 for v in same if v > 2), "same_phase_max": max(same) if same else 0}


def coverage_true(sampler, x0, y0, x, y, k, feature, steps=8):
    hits = 0
    for a in range(steps):
        for b in range(steps):
            colour = sampler(x0 + x + (a + 0.5) / steps, y0 + y + (b + 0.5) / steps, k)[0]
            hits += colour == feature
    return hits / (steps * steps)


def luminance(p):
    return 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]


def detail(values, w, h):
    total = count = 0
    for i in range(w * h):
        if interior(i, w, h):
            x, y = i % w, i // w
            m = sum(values[(y + dy) * w + x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
            total += abs(values[i] - m)
            count += 1
    return total / count


def halo(shown_last, ref, w, h, steps=2):
    ref8 = [to8(p) for p in ref]
    out8 = [to8(p) for p in shown_last]
    count = 0
    for i in range(w * h):
        if not interior(i, w, h):
            continue
        x, y = i % w, i // w
        for ch in range(3):
            local = max(ref8[min(max(y + dy, 0), h - 1) * w + min(max(x + dx, 0), w - 1)][ch]
                        for dy in (-1, 0, 1) for dx in (-1, 0, 1))
            if out8[i][ch] > local + steps:
                count += 1
                break
    return count


# ------------------------------------------------------------------ reproduction

def gpu_pixels(name, points):
    sys.path.insert(0, str(HERE.parent / "visual" / "tools"))
    import pngread
    out = {}
    for run_name, folder in (("baseline", "temporal-coverage-screenshots"), ("a143", "fixed-sharpness-screenshots"),
                             ("warmup", "fixed-warmup-screenshots")):
        path = HERE.parent / folder / name
        if not path.is_file():
            continue
        wd, ht, data = pngread.read(path)
        out[run_name] = {pt: tuple(data[(pt[1] * wd + pt[0]) * 4:(pt[1] * wd + pt[0]) * 4 + 3]) for pt in points}
    return out


def reproduce_static(scene, png, points, feature):
    sampler, x0, y0, w, h, ks = SCENES[scene]
    ref = reference(sampler, x0, y0, w, h, ks[-1])
    ref8 = [to8(p) for p in ref]
    print(f"== {scene}: patch x {x0}..{x0 + w - 1}, y {y0}..{y0 + h - 1}, 16 frames (reset + 15)")
    gpu = gpu_pixels(png, points)
    rows = {}
    for variant in ("baseline", "a143", "warmup"):
        shown = run(scene, variant)
        st = stability(shown, w, h, ref8)
        rows[variant] = (shown, st)
        lum_on = [luminance(p) for p in shown[15]]
        lum_ref = [luminance(p) for p in ref]
        print(f"  {variant:8s} frames 9-16: >=33 {st['ge33']:3d}, >=64 {st['ge64']:3d}, edge max {st['edge_max']:3d}, "
              f"same-phase changed {st['same_phase_changed']} (max {st['same_phase_max']}), "
              f"halo {halo(shown[15], ref, w, h)}, detail ratio {detail(lum_on, w, h) / detail(lum_ref, w, h):.3f}")
    print("  f16 pixel values (sRGB G) model vs GPU at the GPU's failing pixels; true feature coverage from 8x8 supersampling:")
    for pt in points:
        i = (pt[1] - y0) * w + (pt[0] - x0)
        cov = coverage_true(sampler, x0, y0, pt[0] - x0, pt[1] - y0, ks[-1], feature)
        model = {v: to8(rows[v][0][15][i])[1] for v in rows}
        real = {v: gpu[v][pt][1] for v in gpu}
        print(f"   {pt}: ref {ref8[i][1]:3d} | coverage of feature {cov:.2f} | model " +
              " ".join(f"{v} {model[v]:3d}" for v in model) + " | GPU " + " ".join(f"{v} {real[v]:3d}" for v in real))
    print(f"  warmup resolve path per frame 8-16 at those pixels (path, current, history, output; sRGB G). Paths: {PATHS}")
    trace_pixels(scene, points)
    print(f"  warmup frame-to-frame steps >= 33 on edge pixels, frames 9-16, by path before->after: {peak_paths(scene)}")
    return rows


PATHS = ("X reset, D no valid neighbourhood depth, O outside, R refused (no admitted history tap), "
         "A/a admitted unclamped (all/part of the tap mass), C/c clamped (all/part), H held (candidates only)")


def trace_pixels(scene, points, variant="warmup", candidate=None, first=7):
    """Per-frame resolve path at chosen pixels: path, current sample, admitted history, output (sRGB G)."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    ids = {(py - y0) * w + (px - x0): (px, py) for px, py in points}
    trace = {i: [] for i in ids}
    run(scene, variant, candidate, trace)
    g8 = lambda v: f"{srgb8(v[1]):3d}" if v is not None else "  -"
    for i, pt in ids.items():
        cells = [f"t{t + 1:<2d}{path} {g8(c)} {g8(hv)} {g8(value)}" for t, path, c, hv, value in trace[i] if t >= first]
        print(f"   {pt}: " + " | ".join(cells))


def peak_paths(scene, variant="warmup", candidate=None):
    """Frame-to-frame steps >= 33 in frames 9-16 (edge-dilated interior), by resolve path before -> after."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    ref8 = [to8(q) for q in reference(sampler, x0, y0, w, h, ks[-1])]
    edge = edges(ref8, w, h)
    trace = {i: [] for i in range(w * h) if interior(i, w, h) and edge[i]}
    shown = run(scene, variant, candidate, trace)
    counts = {}
    for i, rows in trace.items():
        paths = {t: path for t, path, *_ in rows}
        for t in range(8, 16):
            if step(to8(shown[t][i]), to8(shown[t - 1][i])) >= 33:
                key = f"{paths[t - 1]}->{paths[t]}"
                counts[key] = counts.get(key, 0) + 1
    return dict(sorted(counts.items(), key=lambda kv: -kv[1]))


def cube_bands(shown_last, ref, w, h, x0, y0):
    """sharpness.py cube_bands on the model patch (screen rows 190-240, columns 445-490)."""
    def rb(p):
        q = to8(p)
        return q[0] - q[2]
    left, top = [], []
    for y in range(190, 240):
        xs = [x for x in range(max(400, x0), min(540, x0 + w)) if rb(ref[(y - y0) * w + (x - x0)]) > 60]
        if xs and xs[-1] - xs[0] > 30:
            left.append((xs[0], y))
    for x in range(445, 490):
        ys = [y for y in range(max(150, y0), min(230, y0 + h)) if rb(ref[(y - y0) * w + (x - x0)]) > 60]
        if ys:
            top.append((x, ys[0]))

    def band(points, dx, dy):
        return sum(rb(shown_last[(y + dy - y0) * w + (x + dx - x0)]) - rb(ref[(y + dy - y0) * w + (x + dx - x0)])
                   for x, y in points) / len(points)
    return {"trailing": [round(band(left, -k, 0), 2) for k in (1, 2, 3)], "inside": round(band(left, 0, 0), 2),
            "top": [round(band(top, 0, -k), 2) for k in (1, 2)], "top_inside": round(band(top, 0, 0), 2),
            "rows": len(left), "columns": len(top)}


def off_edge(shown_last, ref, w, h, region=None):
    ref8, out8 = [to8(p) for p in ref], [to8(p) for p in shown_last]
    edge = [a or b for a, b in zip(edges(ref8, w, h, 12, False), edges(out8, w, h, 12, False))]
    from collections import deque
    dist = [1 << 30] * (w * h)
    queue = deque(i for i, v in enumerate(edge) if v)
    for i in queue:
        dist[i] = 0
    while queue:
        i = queue.popleft()
        y, x = divmod(i, w)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and dist[ny * w + nx] > dist[i] + 1:
                    dist[ny * w + nx] = dist[i] + 1
                    queue.append(ny * w + nx)
    bins = [0, 0, 0]
    worst = 0
    for i in range(w * h):
        if dist[i] >= 3 and interior(i, w, h) and (region is None or region(i % w, i // w)):
            v = step(out8[i], ref8[i])
            worst = max(worst, v)
            if 3 <= v <= 8:
                bins[0] += 1
            elif 9 <= v <= 16:
                bins[1] += 1
            elif v > 16:
                bins[2] += 1
    return {"3-8": bins[0], "9-16": bins[1], ">16": bins[2], "max": worst}


def reproduce_moving():
    sampler, x0, y0, w, h, ks = SCENES["cube"]
    ref = reference(sampler, x0, y0, w, h, ks[-1])
    print(f"== cube (moving geometry): patch x {x0}..{x0 + w - 1}, y {y0}..{y0 + h - 1}, frames k = 1..8 (reset at k = 1)")
    rows = {}
    for variant in ("baseline", "a143", "warmup"):
        shown = run("cube", variant)
        b = cube_bands(shown[-1], ref, w, h, x0, y0)
        rows[variant] = (shown, b)
        excess = sum(b["trailing"]) - b["top"][0]
        print(f"  {variant:8s} trailing -1/-2/-3 {b['trailing']} inside {b['inside']} | top -1/-2 {b['top']} inside "
              f"{b['top_inside']} | excess over top {excess:.2f} | rows {b['rows']} columns {b['columns']}")
    print("  GPU (R-B, sharpness.py cube_bands): baseline 47.73 / 6.03 / 0.09 vs top 44.29 (excess 9.6); "
          "a143 54.00 / 0.70 / 0.03 vs top 42.42 (excess 12.3); warmup 51.45 / 0.61 / 0.03 vs top 38.07 (excess 14.0)")
    sampler, x0, y0, w, h, ks = SCENES["shadow"]
    ref = reference(sampler, x0, y0, w, h, ks[-1])
    print(f"== shadow (moving shadow on static ground): patch x {x0}..{x0 + w - 1}, y {y0}..{y0 + h - 1}, k = 1..8")
    for variant in ("baseline", "a143", "warmup"):
        shown = run("shadow", variant)
        print(f"  {variant:8s} off-edge bins at f8 {off_edge(shown[-1], ref, w, h)}")
    print("  GPU off-edge 9-16 in the shadow cluster (579-600, 304-305): baseline 30, a143 39, warmup 36 (max 10-11)")
    return rows


def convexity(scene, variant, candidate=None):
    """Outputs above the current frame's 3x3 per-channel sample maximum (or below its
    minimum), counted in sRGB 8-bit steps beyond 0.5 step of float tolerance."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    state, above, below = State(), 0, 0
    for t, frame in enumerate(frames(scene)):
        shown = resolve_frame(frame, state, w, h, t, variant, candidate)
        cur8 = [to8(f[0]) for f in frame]
        for i in range(w * h):
            if not interior(i, w, h):
                continue
            x, y = i % w, i // w
            o = to8(shown[i])
            for ch in range(3):
                local = [cur8[(y + dy) * w + x + dx][ch] for dy in (-1, 0, 1) for dx in (-1, 0, 1)]
                above += o[ch] > max(local)
                below += o[ch] < min(local)
    return above, below


def same_phase_demo(scene):
    """Same-phase f8/f16 changes (> 2 steps) on non-edge interior pixels: the resolve with
    warmup weights against the plain mean of the last 8 raw samples (exactly phase-periodic)."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    ref8 = [to8(p) for p in reference(sampler, x0, y0, w, h, ks[-1])]
    edge = edges(ref8, w, h)
    ids = [i for i in range(w * h) if interior(i, w, h) and not edge[i]]
    shown = run(scene, "warmup")
    resolve = [step(to8(shown[15][i]), to8(shown[7][i])) for i in ids]
    raw = frames(scene)
    mean = lambda ts, i: tuple(sum(raw[t][i][0][c] for t in ts) / len(ts) for c in range(3))
    box = [step(to8(mean(range(0, 8), i)), to8(mean(range(8, 16), i))) for i in ids]
    return len(ids), (sum(1 for d in resolve if d > 2), max(resolve)), (sum(1 for d in box if d > 2), max(box))


def same_phase_bound(lo8, hi8):
    """The resolve recursion with warmup weights and the reaction term, for one pixel whose
    jittered samples take two values in a fixed 8-phase pattern and are never clamped (its
    3x3 always holds both values). Frame 1 is taken as the sample itself."""
    lo, hi = lin(lo8), lin(hi8)
    worst = count = 0
    for pattern in range(1, 255):
        seq = [hi if pattern >> (t % 8) & 1 else lo for t in range(16)]
        y, out = seq[0], [seq[0]]
        for t in range(1, 16):
            wgt = host_weight("warmup", t) * (1 - STRENGTH * min(abs(y - seq[t]) / THRESHOLD, 1.0))
            y = seq[t] + (y - seq[t]) * wgt
            out.append(y)
        d = abs(srgb8(out[15]) - srgb8(out[7]))
        worst, count = max(worst, d), count + (d > 2)
    return worst, count


def reproduce_halo_and_same_phase():
    print("== halo and same-phase (metric assessment)")
    for scene in ("posts", "grille", "penumbra"):
        for variant in ("baseline", "a143", "warmup"):
            above, below = convexity(scene, variant)
            print(f"  {scene:8s} {variant:8s} outputs above / below the current 3x3 sample range: {above} / {below}")
    n, resolve, box = same_phase_demo("penumbra")
    print(f"  static penumbra (single depth, soft): {n} non-edge interior pixels; same-phase changed (max): warmup "
          f"resolve {resolve[0]} ({resolve[1]}), plain 8-phase mean of the raw samples {box[0]} ({box[1]})")
    for lo8, hi8 in ((130, 150), (120, 160), (100, 180)):
        worst, count = same_phase_bound(lo8, hi8)
        print(f"  unclamped two-valued pixel, samples {lo8}/{hi8} in each of the 254 mixed 8-phase patterns: "
              f"f8/f16 same-phase step max {worst}, patterns > 2 steps {count}")
    print("  periodic EMA weight of a sample k frames old at w = 0.9: 0.1 * 0.9^k / (1 - 0.9^8) = " +
          ", ".join(f"{0.1 * 0.9 ** k / (1 - 0.9 ** 8):.3f}" for k in range(8)) + " (plain mean 0.125)")


def collapse_classes(scene, variant="warmup"):
    """Frame-to-frame steps >= 33 (frames 9-16, edge pixels) landing on a refused or clamped
    frame: whether the frame before was admitted, and the pixel's previous sample depth
    against the current 3x3 expected-depth range (+-1 % + 0.002)."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    ref8 = [to8(q) for q in reference(sampler, x0, y0, w, h, ks[-1])]
    edge = edges(ref8, w, h)
    trace = {i: [] for i in range(w * h) if interior(i, w, h) and edge[i]}
    shown = run(scene, variant, None, trace)
    raw = frames(scene)
    out = {"after admitted": 0, "after collapse": 0, "previous farther": 0, "previous nearer": 0, "previous inside": 0}
    for i, rows in trace.items():
        paths = {t: path for t, path, *_ in rows}
        x, y = i % w, i // w
        for t in range(8, 16):
            if step(to8(shown[t][i]), to8(shown[t - 1][i])) < 33 or paths[t] not in "RCc":
                continue
            out["after collapse" if paths[t - 1] in "RCc" else "after admitted"] += 1
            nb = [raw[t][min(max(y + dy, 0), h - 1) * w + min(max(x + dx, 0), w - 1)][4]
                  for dy in (-1, 0, 1) for dx in (-1, 0, 1)]
            d = raw[t - 1][i][2]
            if d > max(nb) * (1 + GLIMPSE_REL) + GLIMPSE_ABS:
                out["previous farther"] += 1
            elif d < min(nb) * (1 - GLIMPSE_REL) - GLIMPSE_ABS:
                out["previous nearer"] += 1
            else:
                out["previous inside"] += 1
    return out


def fidelity():
    """Model geometry and colour against the published temporal-off PNG (motion-object f8),
    and the top control's sensitivity to a vertical sub-pixel offset of every sample."""
    sys.path.insert(0, str(HERE.parent / "visual" / "tools"))
    import pngread
    off = pngread.read(HERE.parent / "fixed-warmup-screenshots" / "motion-object-off-f8.png")[2]
    sampler, x0, y0, w, h, ks = SCENES["cube"]
    ref = reference(sampler, x0, y0, w, h, ks[-1])
    gpu_rb = lambda x, y: off[(y * 960 + x) * 4] - off[(y * 960 + x) * 4 + 2]
    model_rb = lambda x, y: (lambda q: q[0] - q[2])(to8(ref[(y - y0) * w + (x - x0)]))
    rows = same = 0
    for y in range(190, 240):
        g = [x for x in range(400, 540) if gpu_rb(x, y) > 60]
        m = [x for x in range(400, 520) if model_rb(x, y) > 60]
        if g and g[-1] - g[0] > 30:
            rows += 1
            same += bool(m) and m[0] == g[0]
    tops = {(next(y for y in range(150, 230) if gpu_rb(x, y) > 60), next(y for y in range(168, 230) if model_rb(x, y) > 60))
            for x in range(445, 490)}
    print(f"== model fidelity (motion-object f8, temporal-off): trailing-edge first orange pixel equal in {same} of {rows} rows; "
          f"top edge first orange row (GPU, model) {sorted(tops)}")
    for pt in ((470, 220), (470, 190), (430, 220)):
        i = (pt[1] * 960 + pt[0]) * 4
        print(f"   off {pt}: GPU {tuple(off[i:i + 3])} model {to8(ref[(pt[1] - y0) * w + pt[0] - x0])}")
    band_rows = (cube_sample, 270, 170, 250, 20, list(range(1, 9)))
    for dy in (-0.2, -0.1, 0.0, 0.1, 0.2):
        def shifted(sampler, sx0, sy0, sw, sh, k, jitter):
            return [sampler(sx0 + x + 0.5 - jitter[0], sy0 + y + 0.5 - jitter[1] + dy, k) for y in range(sh) for x in range(sw)]
        levels = []
        for variant in ("baseline", "a143", "warmup"):
            state = State()
            for t, k in enumerate(band_rows[5]):
                last = resolve_frame(shifted(*band_rows[:5], k, SAMPLES[t % 8]), state, 250, 20, t, variant)
            r = shifted(*band_rows[:5], 8, (0.0, 0.0))
            rb = lambda q: (lambda v: v[0] - v[2])(to8(q))
            levels.append(sum(rb(last[8 * 250 + x - 270]) - rb(r[8 * 250 + x - 270][0]) for x in range(445, 490)) / 45)
        print(f"   top control (row 178) with every sample shifted dy {dy:+.1f} px: baseline {levels[0]:.1f}, "
              f"a143 {levels[1]:.1f}, warmup {levels[2]:.1f} (GPU 44.29, 42.42, 38.07)")


def trailing_rows():
    sys.path.insert(0, str(HERE.parent / "visual" / "tools"))
    import pngread
    runs = {"baseline": "temporal-coverage-screenshots", "a143": "fixed-sharpness-screenshots",
            "warmup": "fixed-warmup-screenshots"}
    on = {k: pngread.read(HERE.parent / v / "motion-object-on-f8.png")[2] for k, v in runs.items()}
    off = pngread.read(HERE.parent / "fixed-warmup-screenshots" / "motion-object-off-f8.png")[2]

    def rb(d, x, y):
        i = (y * 960 + x) * 4
        return d[i] - d[i + 2]
    sampler, x0, y0, w, h, ks = SCENES["cube"]
    model = {v: run("cube", v)[-1] for v in runs}
    print("== cube trailing band per row (R-B one pixel left of the off reference's first orange pixel)")
    gaps = {v: [] for v in runs}
    for y in range(190, 240):
        xs = [x for x in range(400, 540) if rb(off, x, y) > 60]
        if not xs or xs[-1] - xs[0] <= 30:
            continue
        x = xs[0]
        for v in runs:
            q = to8(model[v][(y - y0) * w + (x - 1 - x0)])
            gaps[v].append(rb(on[v], x - 1, y) - (q[0] - q[2]))
    for v in runs:
        d = gaps[v]
        print(f"  {v:8s} GPU minus model over {len(d)} rows: mean {sum(d) / len(d):+.2f}, min {min(d):+d}, max {max(d):+d}")


GAP_PIXELS = [(428, 54), (428, 56), (431, 53), (431, 50), (434, 49)]  # far-post gaps (GPU failing pixels)
BAR_PIXELS = [(556, 242), (542, 242), (543, 240), (550, 242)]  # MASK bars (GPU failing pixels)


def retention(shown_last, scene, pixels, feature, other, channel):
    """Mean fraction of the box-filtered feature contrast kept at f16 (linear channel;
    1 = the 8x8-supersampled truth, 0 = the feature erased)."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    values = []
    for px, py in pixels:
        cov = coverage_true(sampler, x0, y0, px - x0, py - y0, ks[-1], feature)
        out = shown_last[(py - y0) * w + (px - x0)][channel]
        values.append((out - other[channel]) / (feature[channel] - other[channel]) / cov)
    return sum(values) / len(values)


def ideal_retention(scene, pixels, feature, other, channel):
    """The same quantity for the plain mean of the last 8 raw jittered samples."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    raw = frames(scene)
    mean = [tuple(sum(raw[t][i][0][c] for t in range(8, 16)) / 8 for c in range(3)) for i in range(w * h)]
    return retention(mean, scene, pixels, feature, other, channel)


def registration_bias(shown, w, h, t=5):
    """Dark-coverage position bias (px, + = dark side extends right) of the aligned edge at x = 20."""
    total = 0.0
    for y in range(5, 35):
        total += sum((GROUND[1] - shown[t][y * w + x][1]) / (GROUND[1] - POST[1]) for x in range(5, 35)) - 15
    return total / 30


def rb8(p):
    q = to8(p)
    return q[0] - q[2]


def orange_residue(shown, ts, w, h, base=4):
    """Largest R-B (sRGB steps) above the most orange far-posts colour (ground, +4)."""
    return max(0, max(rb8(shown[t][i]) - base for t in ts for i in range(w * h) if interior(i, w, h)))


def disocclusion_orange(shown_last, ref, w, x0, y0):
    """Largest R-B excess over the reference in the 3-25 px static strip behind the cube."""
    worst = 0
    for y in range(190, 240):
        xs = [x for x in range(400, 540) if rb8(ref[(y - y0) * w + (x - x0)]) > 60]
        if not xs:
            continue
        for x in range(max(x0 + 3, xs[0] - 25), xs[0] - 2):
            k = (y - y0) * w + (x - x0)
            worst = max(worst, rb8(shown_last[k]) - rb8(ref[k]))
    return worst


def sweep_residue(candidate):
    """Stale shading: far-posts pixels whose 3x3 samples have been on the bright side of the
    sweeping edge for 3 frames, against the same resolve on the unshaded posts (max step)."""
    sampler, x0, y0, w, h, ks = SCENES["sweep"]
    swept, plain = run("sweep", "warmup", candidate), run("posts", "warmup", candidate)
    worst = 0
    for t in range(8, 16):
        bright_until = 410.0 + 2.5 * ks[t - 2] - 1.0
        for i in range(w * h):
            if interior(i, w, h) and x0 + i % w + 1.875 < bright_until:
                worst = max(worst, step(to8(swept[t][i]), to8(plain[t][i])))
    return worst


def overlay_check(candidate):
    """Current-only overlay: a 50 % white BLEND card over the right half of the flat checker,
    composited after history publication (system.rs draws excluded surfaces after the
    resolve). Returns resolved backdrop values differing from the verbatim shader, and
    overlay pixels whose frames 2-8 activity reaches 33 steps."""
    sampler, x0, y0, w, h, ks = SCENES["checker"]
    verbatim, shown = run("checker", "warmup"), run("checker", "warmup", candidate)
    differing = sum(1 for t in range(16) for i in range(w * h) if shown[t][i] != verbatim[t][i])
    white = rgb((255, 255, 255))
    pres = [[tuple(0.5 * white[c] + 0.5 * f[i][c] for c in range(3)) if i % w >= 20 else f[i][:3] for i in range(w * h)]
            for f in shown]
    active = sum(1 for i in range(w * h) if interior(i, w, h) and i % w >= 20
                 and max(step(to8(pres[t][i]), to8(pres[t - 1][i])) for t in range(2, 8)) >= 33)
    return differing, active


def held(scene, candidate):
    """Pixel-frames that took the hold path."""
    sampler, x0, y0, w, h, ks = SCENES[scene]
    trace = {i: [] for i in range(w * h)}
    run(scene, "warmup", candidate, trace)
    return sum(1 for rows in trace.values() for row in rows if row[1] == "H")


def measure(candidate):
    r = {}
    for scene in ("posts", "grille", "diag", "thin"):
        sampler, x0, y0, w, h, ks = SCENES[scene]
        ref = reference(sampler, x0, y0, w, h, ks[-1])
        shown = run(scene, "warmup", candidate)
        st = stability(shown, w, h, [to8(q) for q in ref])
        r[scene] = {"edge_max": st["edge_max"], "ge64": st["ge64"], "ge33": st["ge33"], "peak_max": st["peak_max"],
                    "detail": round(detail([luminance(q) for q in shown[15]], w, h) /
                                    detail([luminance(q) for q in ref], w, h), 3),
                    "same_phase": st["same_phase_changed"], "halo": halo(shown[15], ref, w, h)}
        if scene == "posts":
            r[scene]["gap_retention"] = round(retention(shown[15], "posts", GAP_PIXELS, GROUND, POST, 1), 3)
        if scene == "grille":
            r[scene]["bar_retention"] = round(retention(shown[15], "grille", BAR_PIXELS, BAR, HOLE, 0), 3)
    w, h = SCENES["aligned"][3:5]
    r["aligned_bias"] = round(registration_bias(run("aligned", "warmup", candidate), w, h), 3)
    sampler, x0, y0, w, h, ks = SCENES["cube"]
    ref = reference(sampler, x0, y0, w, h, ks[-1])
    shown = run("cube", "warmup", candidate)
    b = cube_bands(shown[-1], ref, w, h, x0, y0)
    r["cube"] = {"trailing": b["trailing"], "top": b["top"][0], "excess": round(sum(b["trailing"]) - b["top"][0], 2),
                 "disocclusion": disocclusion_orange(shown[-1], ref, w, x0, y0)}
    sampler, x0, y0, w, h, ks = SCENES["shadow"]
    r["shadow"] = off_edge(run("shadow", "warmup", candidate)[-1], reference(sampler, x0, y0, w, h, ks[-1]), w, h)
    w, h = SCENES["occluder"][3:5]
    r["occluder_ghost"] = orange_residue(run("occluder", "warmup", candidate), range(10, 16), w, h)
    w, h = SCENES["despawn"][3:5]
    r["despawn_ghost"] = orange_residue(run("despawn", "warmup", candidate), range(8, 16), w, h)
    r["sweep"] = sweep_residue(candidate)
    r["overlay"] = overlay_check(candidate)
    r["convexity_posts"] = convexity("posts", "warmup", candidate)
    r["held"] = {scene: held(scene, candidate) for scene in ("posts", "grille", "checker", "penumbra", "shadow", "cube")}
    return r


def verdict(r, base):
    flicker = all(r[s]["edge_max"] <= 50 and r[s]["ge64"] == 0 for s in ("posts", "grille"))
    detail_kept = (r["posts"]["gap_retention"] >= base["posts"]["gap_retention"]
                   and r["grille"]["bar_retention"] >= base["grille"]["bar_retention"] - 0.02
                   and r["grille"]["detail"] >= max(0.5, base["grille"]["detail"] - 0.01)
                   and all(r[s]["detail"] >= base[s]["detail"] - 0.01 for s in ("diag", "thin")))
    safety = {
        "static controls": all(r[s]["edge_max"] <= base[s]["edge_max"] + 2 for s in ("diag", "thin"))
        and abs(r["aligned_bias"]) <= abs(base["aligned_bias"]) + 0.01,
        "moving geometry": r["cube"]["trailing"][0] <= base["cube"]["trailing"][0] + 0.5
        and r["cube"]["excess"] <= base["cube"]["excess"] + 0.5
        and r["cube"]["disocclusion"] <= base["cube"]["disocclusion"] + 2,
        "depth rejection": r["occluder_ghost"] <= base["occluder_ghost"] + 2
        and r["despawn_ghost"] <= base["despawn_ghost"] + 2,
        "moving shadow": all(r["shadow"][k] <= base["shadow"][k] for k in ("3-8", "9-16", ">16", "max")),
        "shading change": r["sweep"] <= max(16, base["sweep"]),
        "overlay": r["overlay"][1] <= base["overlay"][1],
    }
    return flicker, detail_kept, safety


def screen():
    """The three candidates declared in candidates.md, against the verbatim 296e0226 shader
    with warmup weights, on the criteria declared there (fixed before this first ran)."""
    print("ideal retention (plain mean of the last 8 raw samples): gaps "
          f"{ideal_retention('posts', GAP_PIXELS, GROUND, POST, 1):.3f}, bars {ideal_retention('grille', BAR_PIXELS, BAR, HOLE, 0):.3f}")
    rows = {}
    for candidate in (None, "C1", "C2", "C3"):
        label = candidate or "verbatim"
        rows[label] = measure(candidate)
        print(f"== {label}")
        for key, value in rows[label].items():
            print(f"  {key}: {value}")
        if candidate is not None:
            print(f"  {label} path per frame 8-16 (path, current, history, output; sRGB G):")
            trace_pixels("posts", [(428, 54), (434, 49)], "warmup", candidate)
            trace_pixels("grille", [(556, 242)], "warmup", candidate)
    print("== verdicts against the declared criteria (candidates.md)")
    for label in ("C1", "C2", "C3"):
        flicker, detail_kept, safety = verdict(rows[label], rows["verbatim"])
        failed = [k for k, v in safety.items() if not v]
        print(f"  {label}: flicker {'met' if flicker else 'NOT met'}, detail {'kept' if detail_kept else 'NOT kept'}, "
              f"safety failures {failed or 'none'} -> {'SUPPORTED' if flicker and detail_kept and not failed else 'not supported'}")


def wire_sample(sx, sy, k):
    """The far posts behind a static 0.6-px wire (depth 40) removed, without motion, at frame 9."""
    if k < 8 and 429.2 <= sx < 429.8 and 46.0 <= sy < 60.0:
        return CUBE_FRONT, 1.0, OCCLUDER_DEPTH, STATIC, OCCLUDER_DEPTH
    return posts_sample(sx, sy, k)


SCENES["wire"] = (wire_sample, 418, 44, 24, 18, list(range(16)))


def residue_rows(scene, ts):
    sampler, x0, y0, w, h, ks = SCENES[scene]
    for candidate in (None, "C1", "C2", "C3"):
        shown = run(scene, "warmup", candidate)
        cells = []
        for t in ts:
            value, i = max((rb8(shown[t][i]) - 4, i) for i in range(w * h) if interior(i, w, h))
            cells.append(f"f{t + 1} {max(value, 0):3d}@{(x0 + i % w, y0 + i // w)}")
        print(f"  {candidate or 'verbatim':8s} " + " ".join(cells))


def checks():
    """Post-screening checks, added after `screen` first ran; none changes a declared verdict."""
    print("== despawn box (declared scene): largest R-B residue above +4 per frame, from the removal frame f9")
    residue_rows("despawn", range(8, 16))
    sampler, x0, y0, w, h, ks = SCENES["despawn"]
    i = (57 - y0) * w + (428 - x0)
    trace = {i: []}
    run("despawn", "warmup", None, trace)
    print("  verbatim at (428, 57), whose own samples never hit the box: path, current, history, output (sRGB RGB)")
    for t, path, c, hv, value in trace[i][5:9]:
        print(f"    f{t + 1} {path} {to8(c)} {to8(hv) if hv else '-'} -> {to8(value)}")
    print("== occluder (declared scene): largest R-B residue above +4 per frame after it left the patch")
    residue_rows("occluder", range(9, 16))
    print("== wire despawn STRESS TEST (not a declared criterion): static 0.6-px wire at depth 40, removed at f9")
    residue_rows("wire", range(6, 16))
    sampler, x0, y0, w, h, ks = SCENES["wire"]
    present = SCENES["wire"]
    for candidate in (None, "C3"):
        shown = run("wire", "warmup", candidate)
        i = (52 - y0) * w + (429 - x0)
        print(f"  {candidate or 'verbatim':8s} wire pixel (429, 52) R-B per frame f2-f16: " +
              " ".join(f"{rb8(shown[t][i]):3d}" for t in range(1, 16)))
    print("== sweep (declared scene): largest stale-shading step per frame, overall / at held pixels")
    sampler, x0, y0, w, h, ks = SCENES["sweep"]
    for candidate in (None, "C1", "C2", "C3"):
        trace = {i: [] for i in range(w * h)}
        swept, plain = run("sweep", "warmup", candidate, trace), run("posts", "warmup", candidate)
        held_at = {(i, t) for i, rows in trace.items() for t, path, *_ in rows if path == "H"}
        cells = []
        for t in range(9, 16):
            bright_until = 410.0 + 2.5 * ks[t - 2] - 1.0
            ds = [(step(to8(swept[t][i]), to8(plain[t][i])), (i, t) in held_at) for i in range(w * h)
                  if interior(i, w, h) and x0 + i % w + 1.875 < bright_until]
            cells.append(f"f{t + 1} {max(d for d, _ in ds):2d}/{max([d for d, held_px in ds if held_px] or [0]):2d}")
        print(f"  {candidate or 'verbatim':8s} " + " ".join(cells))
    sampler, x0, y0, w, h, ks = SCENES["posts"]
    ref8 = [to8(q) for q in reference(sampler, x0, y0, w, h, 0)]
    edge = edges(ref8, w, h)
    trace = {i: [] for i in range(w * h)}
    shown = run("posts", "warmup", "C3", trace)
    print("== C3 posts: remaining frame-to-frame steps >= 33 (paths f(t-3)..f(t): path(current, history, output) sRGB G)")
    for t in range(8, 16):
        for i in range(w * h):
            if interior(i, w, h) and edge[i] and step(to8(shown[t][i]), to8(shown[t - 1][i])) >= 33:
                rows = {tt: r for tt, *r in trace[i]}
                g = lambda v: srgb8(v[1]) if v is not None else "-"
                print(f"  {step(to8(shown[t][i]), to8(shown[t - 1][i]))} at {(x0 + i % w, y0 + i // w)}: " +
                      " ".join(f"f{tt + 1}{rows[tt][0]}({g(rows[tt][1])},{g(rows[tt][2])},{g(rows[tt][3])})"
                               for tt in range(t - 3, t + 1)))
    sampler, x0, y0, w, h, ks = SCENES["cube"]
    raw = frames("cube")
    trace = {i: [] for i in range(w * h)}
    c3, verbatim = run("cube", "warmup", "C3", trace), run("cube", "warmup")
    covered = [any(raw[t][i][3] != STATIC for t in range(8)) for i in range(w * h)]
    held_px = [(i, t) for i, rows in trace.items() for t, path, *_ in rows if path == "H"]
    moved = sum(1 for i, t in held_px if covered[i])
    differ = [step(to8(c3[7][i]), to8(verbatim[7][i])) for i in range(w * h)]
    print(f"== C3 cube: {len(held_px)} held pixel-frames, {moved} at pixels the cube had covered (memory set by the "
          f"disocclusion glimpse), {len(held_px) - moved} at never-covered static pixels; f8 differs from verbatim at "
          f"{sum(1 for d in differ if d)} of {w * h} pixels (max {max(differ)} steps), "
          f"{sum(1 for i in range(w * h) if differ[i] and raw[7][i][3] != STATIC)} of them moving in f8")
    print("== C3 predictions at the GPU's failing pixels: f16 sRGB RGB, verbatim model (GPU warmup matched it) -> C3")
    for scene, pts in (("posts", GAP_PIXELS + [(425, 52)]), ("grille", BAR_PIXELS + [(545, 240)])):
        sx0, sy0, sw = SCENES[scene][1], SCENES[scene][2], SCENES[scene][3]
        before, after = run(scene, "warmup")[15], run(scene, "warmup", "C3")[15]
        print(f"  {scene}: " + "; ".join(f"{pt} {to8(before[(pt[1] - sy0) * sw + pt[0] - sx0])} -> "
                                         f"{to8(after[(pt[1] - sy0) * sw + pt[0] - sx0])}" for pt in pts))
    print("== C3 posts: halo pixels (f16 above the point-sampled reference 3x3 max + 2), sRGB G")
    sampler, x0, y0, w, h, ks = SCENES["posts"]
    out8 = [to8(q) for q in shown[15]]
    raw = frames("posts")
    for i in range(w * h):
        x, y = i % w, i // w
        if not interior(i, w, h):
            continue
        local = max(ref8[(y + dy) * w + x + dx][1] for dy in (-1, 0, 1) for dx in (-1, 0, 1))
        if out8[i][1] > local + 2:
            cov = coverage_true(sampler, x0, y0, x, y, 0, GROUND)
            hits = sum(raw[t][i][0] == GROUND for t in range(8, 16))
            box = srgb8(cov * GROUND[1] + (1 - cov) * POST[1])
            print(f"  {(x0 + x, y0 + y)}: C3 {out8[i][1]} | reference 3x3 max {local} | box-filtered truth {box} "
                  f"(ground coverage {cov:.2f}) | jittered samples hitting the gap {hits}/8")


def viewaid():
    """CPU-MODEL VIEWING AIDS, not screenshots or render evidence. Per scene, x8 nearest
    magnification, panels left to right: box-filtered truth (8x8 supersampling), verbatim
    296e0226 f16, C3 f16; second row: blank, verbatim and C3 frames 9-16 activity (largest
    frame-to-frame step, grey = 4 x step)."""
    sys.path.insert(0, str(HERE.parents[1] / "0067-dynamic-resolution" / "tools"))
    import pngio
    for scene in ("posts", "grille"):
        sampler, x0, y0, w, h, ks = SCENES[scene]
        truth = []
        for y in range(h):
            for x in range(w):
                acc = [0.0, 0.0, 0.0]
                for a in range(8):
                    for b in range(8):
                        colour = sampler(x0 + x + (a + 0.5) / 8, y0 + y + (b + 0.5) / 8, ks[-1])[0]
                        acc = [acc[c] + colour[c] / 64 for c in range(3)]
                truth.append(to8(acc))
        runs = [run(scene, "warmup"), run(scene, "warmup", "C3")]
        outputs = [[to8(q) for q in r[15]] for r in runs]
        activity = []
        for r in runs:
            s8 = [[to8(q) for q in f] for f in r]
            act = [max(step(s8[t][i], s8[t - 1][i]) for t in range(9, 16)) for i in range(w * h)]
            activity.append([(min(255, 4 * v),) * 3 for v in act])
        blank = [(128, 128, 128)] * (w * h)
        grid = [[truth, outputs[0], outputs[1]], [blank, activity[0], activity[1]]]
        scale, gap = 8, 4
        width, height = 3 * w * scale + 2 * gap, 2 * h * scale + gap
        rows = []
        for py in range(height):
            line = bytearray()
            for px in range(width):
                gy, ry = divmod(py, h * scale + gap)
                gx, rx = divmod(px, w * scale + gap)
                if ry >= h * scale or rx >= w * scale:
                    line += bytes((40, 40, 40, 255))
                else:
                    r, g_, b = grid[gy][gx][(ry // scale) * w + rx // scale]
                    line += bytes((r, g_, b, 255))
            rows.append(bytes(line))
        path = HERE / f"cpu-model-{scene}-truth-verbatim-c3.png"
        pngio.write(path, width, height, rows)
        print(f"wrote {path.name} ({width}x{height})")


def main():
    args = sys.argv[1:]
    print("CPU model (not render evidence). Values are sRGB 8-bit after the identity tone map.")
    if args == ["screen"]:
        screen()
        return
    if args == ["checks"]:
        checks()
        return
    if args == ["viewaid"]:
        viewaid()
        return
    reproduce_static("posts", "stationary-on-f16.png",
                     [(428, 54), (428, 56), (431, 53), (431, 50), (434, 49), (425, 52)], POST)
    reproduce_static("grille", "stationary-on-f16.png",
                     [(556, 242), (542, 242), (543, 240), (550, 242), (545, 240)], BAR)
    for scene in ("posts", "grille"):
        print(f"== {scene}: collapses (steps >= 33 onto a refused/clamped frame) {collapse_classes(scene)}")
    reproduce_moving()
    trailing_rows()
    fidelity()
    reproduce_halo_and_same_phase()


if __name__ == "__main__":
    main()
