#!/usr/bin/env python3
"""0073 repair model: CPU only, Python standard library, deterministic.

A grey, scene-linear 2D model of the portable output reconstruction in
quality-repair-brief.md (draft e1b26907) and its correction in
quality-repair-addendum.md. It checks design rules, not the renderer: no GPU,
no shaders, no screenshots. Values are linear; sRGB codes are 8-bit.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/model.py \
        --out handoffs/0073-upscaling-review/repair-model/results.json

Every case states its expected outcome. Retained failures are expected to
fail. The exit status is nonzero if any outcome differs from its expectation.
"""
import argparse
import json
import math
import statistics
import sys

sys.dont_write_bytecode = True

# Engine constants (temporal/jitter.rs at 535003d7; review policy of 0073).
JITTER = ((-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
          (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125))
HISTORY_WEIGHT = 0.9
RELATIVE_DEPTH, ABSOLUTE_DEPTH = 0.05, 0.02
# Design constants (brief e1b26907 and addendum).
KEY_DELTA = 1.0 / 16.0
W_RESET = 0.125
DRAFT_W_RESET = 0.25   # e1b26907 value, used only by the draft rules
STATIC_EPS = 1.0 / 256.0   # output px
DRAFT_MISS_LIMIT = 8
WIDEN_FRAMES = 4.0   # only used by the retained widening alternative
CLEAR = (0.0, 0.0, 0.0, 0, 0.0, 0.0)   # value, coverage, depth, owner, dx, dy


def rho_of(r):
    return 0.35 if r <= 2.0 else 0.35 + 0.325 * (r - 2.0)


def wmax_of(r, flat=False):
    return HISTORY_WEIGHT / (1.0 - HISTORY_WEIGHT) * (1.0 if flat else min(r * r, 4.0))


def lod_of(r, bias=True):
    # Internal LOD of a texture at one texel per output pixel, after the bias.
    b = -min(math.log2(r), 1.0) if bias else 0.0
    return max(0.0, math.log2(r) + b)


def compatible(a, b):
    return a > 0.0 and b > 0.0 and abs(a - b) <= max(ABSOLUTE_DEPTH, RELATIVE_DEPTH * b)


def srgb_code(v):
    v = min(max(v, 0.0), 1.0)
    e = 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1.0 / 2.4) - 0.055
    return 255.0 * e


def lo_hi(origin, extent):
    # Pixel or texel indices whose centres lie in [origin, origin + extent).
    return math.ceil(origin - 0.5), math.ceil(origin + extent - 0.5) - 1


# ---------------------------------------------------------------- textures
def texel(kind, a, b):
    if kind == "lines1":
        return 1.0 if a % 2 == 0 else 0.0
    if kind == "checker2":
        return 1.0 if (a // 2 + b // 2) % 2 == 0 else 0.0
    if kind == "checker4":
        return 1.0 if (a // 4 + b // 4) % 2 == 0 else 0.0
    if kind == "sine":
        return 0.5 + 0.5 * math.sin(2 * math.pi * (a + 0.5) / 7.3) * math.sin(2 * math.pi * (b + 0.5) / 9.1)
    raise ValueError(kind)


def tex(kind, u, v, lod):
    # Trilinear: box mips, bilinear inside a level, linear between levels.
    def level(n):
        s = 1 << n
        x, y = u / s - 0.5, v / s - 0.5
        a, b = math.floor(x), math.floor(y)
        fx, fy = x - a, y - b

        def m(i, k):
            return sum(texel(kind, i * s + p, k * s + q) for q in range(s) for p in range(s)) / (s * s)
        return ((1 - fx) * (1 - fy) * m(a, b) + fx * (1 - fy) * m(a + 1, b)
                + (1 - fx) * fy * m(a, b + 1) + fx * fy * m(a + 1, b + 1))
    l0 = math.floor(lod)
    f = lod - l0
    base = level(l0)
    return base if f == 0 else (1 - f) * base + f * level(l0 + 1)


# ------------------------------------------------------------------ scenes
class Scene:
    """sample(x, y, t, lod) at unjittered output coordinates returns
    (value, coverage, depth, owner, dx, dy); dx/dy is the surface point's
    screen displacement from frame t-1 to t. Owner 0 is clear."""

    def changed(self, t):
        return frozenset()

    def camera_static(self, t):
        return True

    def shade(self, x, y, t):
        return 1.0

    def suspended(self, t):
        return False

    def gate(self, x, y, t):
        # Whether a changed caster's swept light-space bounds can reach this receiver.
        return True


class Chart(Scene):
    def __init__(self, kind):
        self.kind = kind

    def sample(self, x, y, t, lod):
        return (tex(self.kind, x, y, lod), 1.0, 2.0, 1, 0.0, 0.0)


class Wires(Scene):
    def __init__(self, wires, floor=None):
        self.wires, self.floor = wires, floor

    def sample(self, x, y, t, lod):
        for i, (y0, width) in enumerate(self.wires):
            if y0 <= y < y0 + width:
                return (0.8, 1.0, 1.0, 10 + i, 0.0, 0.0)
        return CLEAR if self.floor is None else (self.floor, 1.0, 2.0, 1, 0.0, 0.0)


class Square(Scene):
    """Root's case: a white square (owner 2), stationary until `start`, then
    moving `speed` px/frame (optionally stopping) over a background."""

    def __init__(self, start, speed, background, stop=None, x0=6.0, y0=10.0, size=10.0):
        self.start, self.speed, self.background, self.stop = start, speed, background, stop
        self.x0, self.y0, self.size = x0, y0, size

    def offset(self, t):
        end = t if self.stop is None else min(t, self.stop)
        return self.x0 + self.speed * max(0, end - self.start)

    def ground(self, x, y, lod):
        if self.background == "clear":
            return CLEAR
        if self.background == "black":
            return (0.0, 1.0, 2.0, 1, 0.0, 0.0)
        return (tex(self.background, x, y, lod), 1.0, 2.0, 1, 0.0, 0.0)

    def sample(self, x, y, t, lod):
        s = self.offset(t)
        if s <= x < s + self.size and self.y0 <= y < self.y0 + self.size:
            return (1.0, 1.0, 1.0, 2, s - self.offset(t - 1), 0.0)
        return self.ground(x, y, lod)

    def changed(self, t):
        return frozenset({2}) if self.offset(t) != self.offset(t - 1) else frozenset()


class Occluder(Scene):
    """Static checker floor (y >= 16) and a static 0.8 px wire over clear
    (y = 8.3); a grey bar (owner 3, nearer) sweeps across both from `start`.
    start=None never shows the bar (converged reference)."""

    def __init__(self, start, speed):
        self.start, self.speed = start, speed

    def bar(self, t):
        if self.start is None:
            return -1000.0
        return -8.0 + self.speed * max(0, t - self.start)

    def sample(self, x, y, t, lod):
        b = self.bar(t)
        if b <= x < b + 6.0 and 4.0 <= y < 28.0:
            return (0.6, 1.0, 0.5, 3, b - self.bar(t - 1), 0.0)
        if 8.3 <= y < 9.1:
            return (0.8, 1.0, 1.0, 10, 0.0, 0.0)
        if y >= 16.0:
            return (tex("checker4", x, y, lod), 1.0, 2.0, 1, 0.0, 0.0)
        return CLEAR

    def changed(self, t):
        return frozenset({3}) if self.bar(t) != self.bar(t - 1) else frozenset()


class Pan(Scene):
    """Static world (checker floor, 0.8 px wire over clear); the camera pans
    `speed` px/frame from `start` to `stop`, then holds."""

    def __init__(self, start, stop, speed, offset=0.0):
        self.start, self.stop, self.speed, self.offset = start, stop, speed, offset

    def cam(self, t):
        return self.offset + self.speed * (min(max(t, self.start), self.stop) - self.start)

    def sample(self, x, y, t, lod):
        wx = x + self.cam(t)
        d = -(self.cam(t) - self.cam(t - 1))
        if 8.3 <= y < 9.1:
            return (0.8, 1.0, 1.0, 10, d, 0.0)
        if y >= 16.0:
            return (tex("checker4", wx, y, lod), 1.0, 2.0, 1, d, 0.0)
        return CLEAR

    def camera_static(self, t):
        return self.cam(t) == self.cam(t - 1)


class Triple(Scene):
    """Three static owners meet at (16, 16); a bar (owner 3) crosses near the
    junction and leaves."""

    def __init__(self, start, speed):
        self.start, self.speed = start, speed

    def bar(self, t):
        if self.start is None:
            return -1000.0
        return -8.0 + self.speed * max(0, t - self.start)

    def sample(self, x, y, t, lod):
        b = self.bar(t)
        if b <= x < b + 5.0 and 12.0 <= y < 20.0:
            return (1.0, 1.0, 0.5, 3, b - self.bar(t - 1), 0.0)
        if x < 16.0:
            return (0.2, 1.0, 2.0, 4, 0.0, 0.0)
        if y < 16.0:
            return (0.5, 1.0, 3.0, 5, 0.0, 0.0)
        return (0.8, 1.0, 4.0, 6, 0.0, 0.0)

    def changed(self, t):
        return frozenset({3}) if self.bar(t) != self.bar(t - 1) else frozenset()


class Relit(Scene):
    """Static checker floor whose light halves at `at`; `suspend` reports the
    host's frame-global light-change flag for that frame."""

    def __init__(self, at, suspend):
        self.at, self.suspend = at, suspend

    def sample(self, x, y, t, lod):
        return (tex("checker4", x, y, lod) * (0.5 if t >= self.at else 1.0), 1.0, 2.0, 1, 0.0, 0.0)

    def suspended(self, t):
        return self.suspend and t == self.at


class Shadow(Scene):
    """Static checker floor lit by one directional light with ambient 0.25. A
    caster (not visible on screen) moves its shadow band in shadow texels of
    1.5 output px; shading uses 3x3 PCF (1D here, the band is vertical). The
    change key is evaluated at each static receiver point."""
    TEXEL = 1.5

    def __init__(self, start, speed, key, fixed=None, gated=False):
        self.start, self.speed, self.key, self.fixed, self.gated = start, speed, key, fixed, gated

    def band(self, t):
        return self.fixed if self.fixed is not None else -6 + self.speed * max(0, t - self.start)

    def gate(self, x, y, t):
        if not self.gated:
            return True
        lo = min(self.band(t), self.band(t - 1)) - 1
        hi = max(self.band(t), self.band(t - 1)) + 6 + 1
        return lo <= math.floor(x / self.TEXEL) < hi

    def occluded(self, column, t):
        c0 = self.band(t)
        return c0 <= column < c0 + 6

    def pcf(self, x, t):
        ix = math.floor(x / self.TEXEL)
        return 1.0 - sum(self.occluded(ix + d, t) for d in (-1, 0, 1)) / 3.0

    def sample(self, x, y, t, lod):
        return (tex("checker4", x, y, lod) * (0.25 + 0.75 * self.pcf(x, t)), 1.0, 2.0, 1, 0.0, 0.0)

    def shade(self, x, y, t):
        if self.key == "none":
            return 1.0
        if self.key == "pcf3":
            return self.pcf(x, t)
        # gather2: the two texels whose centres bracket x (one gather in 2D)
        g = math.floor(x / self.TEXEL - 0.5)
        return 1.0 - sum(self.occluded(g + d, t) for d in (0, 1)) / 2.0


# --------------------------------------------------------------- resolver
class Run:
    """Runs one algorithm on one scene. algo: 'draft' (e1b26907 rules) or
    'corrected' (addendum rules). Options select retained failure variants."""

    def __init__(self, scene, algo, size=(32, 32), rect=None, flat_cap=False,
                 clip_static=False, bias=True, rescale=True, widen=0.0, drops=None, w_reset=W_RESET):
        self.scene, self.algo, self.size = scene, algo, size
        self.rect = rect or (0.0, 0.0, float(size[0]), float(size[1]))
        self.flat_cap, self.clip_static, self.bias, self.use_rescale = flat_cap, clip_static, bias, rescale
        self.widen, self.drops, self.w_reset = widen, drops, w_reset   # drops: {frame: pixels to reset}
        self.dropped = {}                       # corrected runs record their own drops here
        self.drop_now = ()
        self.hist = None
        self.phase = 0
        self.prev_n = None
        self.stats = {"persist": 0, "clip_in_place": 0, "dropped": 0, "moving": 0, "fresh": 0, "key_resets": 0,
                      "key_evaluations": 0, "overflow": 0}
        self.last_mass = {}

    def cut(self):
        self.hist, self.phase, self.prev_n = None, 0, None

    def frame(self, t, n):
        sc = self.scene
        ox, oy, ow, oh = self.rect
        r = 16.0 / n
        sx, sy, sw, sh = (v * n / 16.0 for v in self.rect)
        jx, jy = JITTER[self.phase]
        rho = rho_of(r)
        norm = r * r / (2 * math.pi * rho * rho)
        near = max(r, 4.0 * rho)          # contributor and rectification radius, output px
        wmax = wmax_of(r, self.flat_cap)
        lod = lod_of(r, self.bias)
        slx, shx = lo_hi(sx, sw)
        sly, shy = lo_hi(sy, sh)
        kx, ky = ow / sw, oh / sh
        cur = {}
        for k in range(sly, shy + 1):
            for i in range(slx, shx + 1):
                X = ox + (i + 0.5 - sx - jx) * kx
                Y = oy + (k + 0.5 - sy - jy) * ky
                cur[(i, k)] = (X, Y) + sc.sample(X, Y, t, lod)
        self.drop_now = self.drops.get(t, ()) if self.drops is not None else ()
        ctx = {"changed": sc.changed(t), "cam_static": sc.camera_static(t), "suspended": sc.suspended(t),
               "rescale": 1.0 if (self.prev_n is None or n <= self.prev_n or not self.use_rescale)
               else (self.prev_n / n) ** 2,
               "wmax": wmax, "near2": near * near, "t": t, "rho": rho}
        width, height = self.size
        out = [[0.0] * width for _ in range(height)]
        new = {}
        resolve = self.pixel_draft if self.algo == "draft" else self.pixel_corrected
        for py in range(height):
            for px in range(width):
                P = (px + 0.5, py + 0.5)
                if not (ox <= P[0] < ox + ow and oy <= P[1] < oy + oh):
                    continue
                spx = sx + (P[0] - ox) / kx + jx
                spy = sy + (P[1] - oy) / ky + jy
                ni, nk = math.floor(spx), math.floor(spy)
                taps = []
                for dk in (-1, 0, 1):
                    for di in (-1, 0, 1):
                        tap = cur.get((ni + di, nk + dk))
                        if tap is None:      # outside the source viewport: skipped, never clamped
                            continue
                        X, Y, val, a, z, owner, dx, dy = tap
                        d2 = (X - P[0]) ** 2 + (Y - P[1]) ** 2
                        taps.append((norm * math.exp(-d2 / (2 * rho * rho)), val * a, a, z, owner, dx, dy, d2))
                self.last_mass[(px, py)] = sum(tp[0] for tp in taps)
                new[(px, py)], out[py][px] = resolve(P, taps, ctx, (px, py))
        self.hist = new
        self.prev_n = n
        self.phase = (self.phase + 1) % 8
        return out

    @staticmethod
    def box_of(taps):
        return ((min(tp[1] for tp in taps), max(tp[1] for tp in taps)),
                (min(tp[2] for tp in taps), max(tp[2] for tp in taps))) if taps else ((0.0, 0.0), (0.0, 0.0))

    @staticmethod
    def clip(hc, ha, box):
        return min(max(hc, box[0][0]), box[0][1]), min(max(ha, box[1][0]), box[1][1])

    def reproject(self, P, src, ctx, admit):
        """Bilinear history at P - motion; taps outside the output viewport or
        failing `admit` are skipped; returns (c, a, W) or None."""
        q = (P[0] - src[5], P[1] - src[6])
        ox, oy, ow, oh = self.rect
        if not (ox <= q[0] < ox + ow and oy <= q[1] < oy + oh):
            return None
        bx, by = math.floor(q[0] - 0.5), math.floor(q[1] - 0.5)
        fx, fy = q[0] - 0.5 - bx, q[1] - 0.5 - by
        lx, hx = lo_hi(ox, ow)
        ly, hy = lo_hi(oy, oh)
        acc = [0.0, 0.0, 0.0]
        mass = 0.0
        for ex, ey, w in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
            cx, cy = bx + ex, by + ey
            if w <= 0.0 or not (lx <= cx <= hx and ly <= cy <= hy):
                continue
            h = self.hist.get((cx, cy))
            if h is None or not admit(h[2]):
                continue
            acc[0] += w * h[0]
            acc[1] += w * h[1]
            acc[2] += w * min(h[3], ctx["wmax"]) * ctx["rescale"]
            mass += w
        return None if mass <= 0.0 else (acc[0] / mass, acc[1] / mass, acc[2] / mass)

    def finish(self, taps, used, ctx, age=255):
        Wc = sum(tp[0] for tp in taps)
        S = sum(tp[0] * tp[1] for tp in taps)
        A = sum(tp[0] * tp[2] for tp in taps)
        grow = 1.0 + self.widen * max(0.0, 1.0 - age / WIDEN_FRAMES)
        if grow > 1.0 and taps:
            # Retained alternative: a wider kernel for freshly started history.
            rho = ctx["rho"] * grow
            ws = [math.exp(-tp[7] / (2 * rho * rho)) for tp in taps]
            total = sum(ws)
            S = Wc * sum(w * tp[1] for w, tp in zip(ws, taps)) / total
            A = Wc * sum(w * tp[2] for w, tp in zip(ws, taps)) / total
        if used is None:
            return (S / Wc if Wc > 0.0 else 0.0), (A / Wc if Wc > 0.0 else 0.0), min(Wc, ctx["wmax"])
        hc, ha, hw = used
        total = hw + Wc
        if total <= 0.0:
            return 0.0, 0.0, 0.0
        return (hc * hw + S) / total, (ha * hw + A) / total, min(total, ctx["wmax"])

    def pixel_draft(self, P, taps, ctx, index):
        """e1b26907 rules: static and history-static pixels keep history with no
        depth test or clip (shading key aside); a miss counter drops it after
        more than 8 frames; moving pixels reproject along the motion source with
        admission against its depth and clip to all 9 taps."""
        src = None
        if taps:
            primary = max(taps, key=lambda tp: tp[0])
            geo = [tp for tp in taps if tp[3] > 0.0]
            src = primary if primary[3] > 0.0 else (min(geo, key=lambda tp: tp[3]) if geo else None)
        static = (abs(src[5]) < STATIC_EPS and abs(src[6]) < STATIC_EPS) if src is not None else ctx["cam_static"]
        box = self.box_of(taps)
        st = self.hist.get(index) if self.hist is not None else None
        used, depth, key_store, misses = None, (src[3] if src is not None else 0.0), -1.0, 0
        if self.hist is None:
            self.stats["fresh"] += 1
        elif static and st is not None and st[6]:
            hc, ha, hz, hw, hkey = st[0], st[1], st[2], min(st[3], ctx["wmax"]) * ctx["rescale"], st[4]
            key_store = self.scene.shade(P[0], P[1], ctx["t"])
            if hkey >= 0.0 and abs(key_store - hkey) > KEY_DELTA:
                hc, ha = self.clip(hc, ha, box)
                hw = min(hw, DRAFT_W_RESET)
                self.stats["key_resets"] += 1
            supported = hz <= 0.0 or any(compatible(tp[3], hz) for tp in taps if tp[3] > 0.0)
            misses = 0 if supported else st[7] + 1
            if misses > DRAFT_MISS_LIMIT:
                used = None
            else:
                used, depth = (hc, ha, hw), (hz if hz > 0.0 else depth)
            self.stats["persist"] += 1
        else:
            if src is not None:
                h = self.reproject(P, src, ctx, lambda z: compatible(z, src[3]))
                if h is not None:
                    used = self.clip(h[0], h[1], box) + (h[2],)
            self.stats["moving"] += 1
        c, a, w_new = self.finish(taps, used, ctx)
        if key_store < 0.0 and static:
            key_store = self.scene.shade(P[0], P[1], ctx["t"])
        return (c, a, depth, w_new, key_store, None, static, misses, 0), c

    def pixel_corrected(self, P, taps, ctx, index):
        """Addendum rules. Near taps lie within max(r, 4 rho) of P; their
        geometry owners are this frame's contributors and their values bound
        rectification. History stores up to two contributing owners (or an
        overflow mark), the front-most contributor's owner and its depth.
        A static pixel keeps unclipped history only while every stored owner is
        unchanged. Otherwise: if the front owner changed and no near sample lies
        at its depth, the history is dropped; else it is clipped in place.
        A moving pixel reprojects along its primary surface and admits history
        whose front depth matches a near surface moving with it."""
        src, contrib, z_front, front, near = None, frozenset(), 0.0, 0, []
        if taps:
            primary = max(taps, key=lambda tp: tp[0])
            near = [tp for tp in taps if tp[7] <= ctx["near2"]] or [primary]
            geo = [tp for tp in near if tp[3] > 0.0]
            src = primary if primary[3] > 0.0 else (min(geo, key=lambda tp: tp[3]) if geo else None)
            contrib = frozenset(tp[4] for tp in geo if tp[2] > 0.0)
            if geo:
                nearest = min(geo, key=lambda tp: tp[3])
                z_front, front = nearest[3], nearest[4]
        static = (abs(src[5]) < STATIC_EPS and abs(src[6]) < STATIC_EPS) if src is not None else ctx["cam_static"]
        box = self.box_of(near)
        owners = contrib if len(contrib) <= 2 else None
        st = None if (self.hist is None or index in self.drop_now) else self.hist.get(index)
        used, key_store = None, -1.0
        if self.hist is None:
            self.stats["fresh"] += 1
        elif static and st is not None:
            hc, ha, hz, hw, hkey, howners, hfront = (st[0], st[1], st[2], min(st[3], ctx["wmax"]) * ctx["rescale"],
                                                       st[4], st[5], st[8])
            if howners is not None and not (howners & ctx["changed"]) and not ctx["suspended"]:
                if hkey >= 0.0 and not self.scene.gate(P[0], P[1], ctx["t"]):
                    key_store = hkey
                else:
                    key_store = self.scene.shade(P[0], P[1], ctx["t"])
                    self.stats["key_evaluations"] += 1
                if hkey >= 0.0 and abs(key_store - hkey) > KEY_DELTA:
                    hc, ha = self.clip(hc, ha, box)
                    hw = min(hw, self.w_reset)
                    self.stats["key_resets"] += 1
                if self.clip_static:
                    hc, ha = self.clip(hc, ha, box)
                merged = howners | contrib
                owners = merged if len(merged) <= 2 else None
                if hz > 0.0 and (z_front <= 0.0 or hz <= z_front):
                    z_front, front = hz, hfront
                used = (hc, ha, hw)
                self.stats["persist"] += 1
            elif hfront in ctx["changed"] and not any(compatible(tp[3], hz) for tp in near if tp[3] > 0.0):
                self.stats["dropped"] += 1        # the front contributor left: its history is stale
                self.dropped.setdefault(ctx["t"], set()).add(index)
            else:
                hc, ha = self.clip(hc, ha, box)
                if ctx["suspended"]:
                    hw = min(hw, self.w_reset)
                used = (hc, ha, hw)
                self.stats["clip_in_place"] += 1
        elif src is not None and not static:
            same = [tp[3] for tp in near if tp[3] > 0.0
                    and abs(tp[5] - src[5]) < STATIC_EPS and abs(tp[6] - src[6]) < STATIC_EPS]
            h = self.reproject(P, src, ctx, lambda z: any(compatible(z, e) for e in same))
            if h is not None:
                used = self.clip(h[0], h[1], box) + (h[2],)
            self.stats["moving"] += 1
        if owners is None:
            self.stats["overflow"] += 1
        age = 0 if used is None else min((st[7] if (st is not None and static) else 0) + 1, 255)
        c, a, w_new = self.finish(taps, used, ctx, age)
        if key_store < 0.0 and static:
            key_store = self.scene.shade(P[0], P[1], ctx["t"])
        return (c, a, z_front, w_new, key_store, owners, static, age, front), c


def full(scene, t, size):
    return [[(lambda s: s[0] * s[1])(scene.sample(px + 0.5, py + 0.5, t, 0.0))
             for px in range(size[0])] for py in range(size[1])]


def baseline_535003d7_chart(kind, r, frames=24, size=32, h=HISTORY_WEIGHT):
    """535003d7 resolve on an opaque single surface: bilinear current at the
    de-jittered position, fixed 0.9 blend, history clamped to the current 3x3."""
    n = int(size / r)
    lod = math.log2(r)
    H = None
    outs = []
    for t in range(frames):
        jx, jy = JITTER[t % 8]
        c = [[tex(kind, (i + 0.5 - jx) * r, (k + 0.5 - jy) * r, lod) for i in range(n)] for k in range(n)]

        def cl(i, k):
            return c[min(max(k, 0), n - 1)][min(max(i, 0), n - 1)]
        new = [[0.0] * size for _ in range(size)]
        for py in range(size):
            for px in range(size):
                qx = (px + 0.5) / r + jx - 0.5
                qy = (py + 0.5) / r + jy - 0.5
                bx, by = math.floor(qx), math.floor(qy)
                fx, fy = qx - bx, qy - by
                cur = ((1 - fx) * (1 - fy) * cl(bx, by) + fx * (1 - fy) * cl(bx + 1, by)
                       + (1 - fx) * fy * cl(bx, by + 1) + fx * fy * cl(bx + 1, by + 1))
                if H is None:
                    new[py][px] = cur
                    continue
                ax = bx if (1 - fx) > 0 else bx + 1
                ay = by if (1 - fy) > 0 else by + 1
                box = [cl(ax + i, ay + k) for k in (-1, 0, 1) for i in (-1, 0, 1)] + [cur]
                hist = min(max(H[py][px], min(box)), max(box))
                new[py][px] = cur + h * (hist - cur)
        H = new
        outs.append(new)
    return outs


# ----------------------------------------------------------------- metrics
def region(frame, x0, y0, x1, y1):
    return [frame[y][x] for y in range(y0, y1) for x in range(x0, x1)]


def contrast_fraction(frame, ref, box):
    return statistics.pstdev(region(frame, *box)) / statistics.pstdev(region(ref, *box))


def activity_codes(frames, box):
    x0, y0, x1, y1 = box
    return statistics.mean(max(srgb_code(f[y][x]) for f in frames) - min(srgb_code(f[y][x]) for f in frames)
                           for y in range(y0, y1) for x in range(x0, x1))


def max_residual(frame, ref, cells):
    return max(abs(frame[y][x] - ref[y][x]) for x, y in cells)


# ------------------------------------------------------------------- cases
def case(results, name, expect, ok, **values):
    results.append({"case": name, "expect": expect, "observed": "pass" if ok else "fail",
                    "as_expected": (expect == "pass") == bool(ok),
                    "values": {k: (round(v, 4) if isinstance(v, float) else v) for k, v in values.items()}})


def run_frames(run, frames, n_of):
    outs = []
    for t in range(frames):
        outs.append(run.frame(t, n_of(t) if callable(n_of) else n_of))
    return outs


def cases():
    res = []
    size = (32, 32)

    # F1: root's scalar recurrence (static path keeps history, one unit of black per frame, cap 9).
    h, w, rows = 1.0, 9.0, []
    for frame in range(1, 10):
        if frame > DRAFT_MISS_LIMIT:
            w = 0.0
        h = h * w / (w + 1.0)
        w = min(w + 1.0, 9.0)
        rows.append(h)
    case(res, "F1 root recurrence: draft static path retains stale white", "fail",
         rows[2] <= 0.06 and rows[7] <= 0.06, after_1=rows[0], after_3=rows[2], after_8=rows[7], after_9=rows[8])

    # C0: the 535003d7 model against the measured a5ae3179 chart fractions at 8/16.
    measured = {"lines1": 0.032, "checker2": 0.061, "checker4": 0.434}
    calib = {}
    for kind in measured:
        out = baseline_535003d7_chart(kind, 2.0)
        calib[kind] = contrast_fraction(out[-1], full(Chart(kind), 0, size), (4, 4, 28, 28))
    case(res, "C0 calibration: 535003d7 model vs measured a5ae3179 chart", "pass",
         all(abs(calib[k] - measured[k]) <= 0.05 for k in measured),
         **{f"model_{k}": v for k, v in calib.items()}, **{f"measured_{k}": v for k, v in measured.items()})
    base_act = {kind: activity_codes(baseline_535003d7_chart(kind, 2.0, frames=48)[-8:], (4, 4, 28, 28))
                for kind in ("checker2", "checker4")}

    # F2/C12: root's counterexample. Pixels the stationary square covered, now at
    # least 2 px behind its trailing edge, against the true background.
    for background in ("black", "clear"):
        for n in (8, 16):
            scene = Square(12, 3.0, background)
            for algo, expect in (("draft", "pass" if (background, n) == ("clear", 8) else "fail"),
                                 ("corrected", "pass")):
                outs = run_frames(Run(scene, algo, size=size), 22, n)
                found = {}
                for k in (1, 3, 8):
                    t = scene.start + k
                    s_edge = scene.offset(t)
                    cells = [(x, y) for y in range(10, 20) for x in range(6, 16) if x + 1.0 <= s_edge - 2.0]
                    found[f"max_residual_after_{k}"] = max_residual(outs[t], full(scene, t, size), cells)
                tag = "F2 draft" if algo == "draft" else "C12 corrected"
                case(res, f"{tag}: stationary square starts moving over {background} at {n}/16", expect,
                     all(v <= 0.06 for v in found.values()), **found)

    # C7 as written in e1b26907, then controls that separate ghosting from
    # unavoidable post-disocclusion convergence. Columns beyond two frames of
    # motion behind the trailing edge; textured floor; 8/16.
    T = 17
    moving = Square(12, 3.0, "checker4")
    never = Square(10 ** 6, 0.0, "checker4", x0=moving.offset(T))
    vac = [x for x in range(6, 16) if x + 1.0 <= moving.offset(T) - 6.0]
    b_never = run_frames(Run(never, "corrected", size=size), T + 1, 8)[-1]
    for algo in ("draft", "corrected"):
        run = Run(moving, algo, size=size)
        a_mov = run_frames(run, T + 1, 8)[-1]
        cols = [statistics.mean(abs(a_mov[y][x] - b_never[y][x]) for y in range(11, 19)) for x in vac]
        bias = [statistics.mean(a_mov[y][x] - b_never[y][x] for y in range(11, 19)) for x in vac]
        case(res, f"C7 as written ({algo}): |moving - never moved| per column <= 0.06", "fail",
             max(cols) <= 0.06, worst_column=max(cols), mean_column=statistics.mean(cols))
        case(res, f"C7a ({algo}): signed bias toward the white occluder per column <= 0.01",
             "pass" if algo == "corrected" else "fail", max(abs(v) for v in bias) <= 0.01,
             worst_signed_bias=max(bias, key=abs))
        if algo == "corrected":
            matched = run_frames(Run(never, "corrected", size=size, drops=run.dropped), T + 1, 8)[-1]
            worst = max(abs(a_mov[y][x] - matched[y][x]) for y in range(11, 19) for x in vac)
            case(res, "C7b (corrected): equals a never-moved floor reset at the same pixels and frames",
                 "pass", worst <= 0.02, worst_pixel=worst, dropped_pixel_frames=sum(map(len, run.dropped.values())))
            fresh = run_frames(Run(never, "corrected", size=size, drops={t: {(x, y) for y in range(32) for x in range(32)}
                                                                         for t in range(T + 1)}), T + 1, 8)[-1]
            floor_fresh = statistics.mean(abs(fresh[y][x] - b_never[y][x]) for y in range(11, 19) for x in vac)
            case(res, "C7c reference: a current-frame-only floor vs the converged floor", "pass", True,
                 mean_difference=floor_fresh)
    moving_long = Square(12, 3.0, "checker4")
    T2 = 21
    never2 = Square(10 ** 6, 0.0, "checker4", x0=moving_long.offset(T2))
    a_long = run_frames(Run(moving_long, "corrected", size=size), T2 + 1, 8)[-1]
    b_long = run_frames(Run(never2, "corrected", size=size), T2 + 1, 8)[-1]
    by_age = {}
    for x in range(6, 16):
        vacated_at = next(t for t in range(13, T2 + 1) if x + 1.0 <= moving_long.offset(t))
        by_age.setdefault(T2 - vacated_at, []).append(
            statistics.mean(abs(a_long[y][x] - b_long[y][x]) for y in range(11, 19)))
    curve = {f"frames_since_vacated_{k}": max(v) for k, v in sorted(by_age.items())}
    old = [max(v) for k, v in by_age.items() if k >= 6]
    case(res, "C7d (corrected): disocclusion residue converges below 0.06 within 6 frames", "pass",
         bool(old) and max(old) <= 0.06, **curve)
    widened = Run(moving, "corrected", size=size, widen=1.0)
    a_w = run_frames(widened, T + 1, 8)[-1]
    cols_w = [statistics.mean(abs(a_w[y][x] - b_never[y][x]) for y in range(11, 19)) for x in vac]
    case(res, "F7 wider kernel for fresh history: C7 as written still fails", "fail", max(cols_w) <= 0.06,
         worst_column=max(cols_w), mean_column=statistics.mean(cols_w))

    # C14: start, move, stop over texture; compare with a square that never moved.
    stop = Square(12, 3.0, "checker4", stop=16)
    never = Square(10 ** 6, 0.0, "checker4", x0=stop.offset(33))
    a = run_frames(Run(stop, "corrected", size=size), 34, 8)[-1]
    b = run_frames(Run(never, "corrected", size=size), 34, 8)[-1]
    cols = [statistics.mean(abs(a[y][x] - b[y][x]) for y in range(10, 20)) for x in range(2, 17)]
    case(res, "C14 corrected: start-move-stop over texture", "pass", max(cols) <= 0.06, worst_column=max(cols))

    # C15: a nearer bar sweeps over a static floor and a static wire over clear.
    occ, calm = Occluder(4, 4.0), Occluder(None, 4.0)
    a_runs = run_frames(Run(occ, "corrected", size=size), 26, 8)
    b_runs = run_frames(Run(calm, "corrected", size=size), 26, 8)
    pres = [statistics.mean(max(f[y][x] for y in range(5, 13)) for x in range(2, 30)) for f in a_runs[-8:]]
    pres_ref = [statistics.mean(max(f[y][x] for y in range(5, 13)) for x in range(2, 30)) for f in b_runs[-8:]]
    floor_cols = [statistics.mean(abs(a_runs[-1][y][x] - b_runs[-1][y][x]) for y in range(17, 30)) for x in range(2, 30)]
    case(res, "C15 corrected: occluder passes over floor and wire", "pass",
         min(pres) > 0.15 and max(floor_cols) <= 0.06, wire_presence_max=max(pres),
         wire_presence_min=min(pres), wire_presence_reference_min=min(pres_ref), floor_worst_column=max(floor_cols))

    # C16: camera pans 16 px over 8 frames, then holds; compare with a camera that
    # was always at the final position.
    pan, held = Pan(8, 16, 2.0), Pan(0, 0, 0.0, offset=16.0)
    run = Run(pan, "corrected", size=size)
    a_runs = run_frames(run, 40, 8)
    b_runs = run_frames(Run(held, "corrected", size=size), 40, 8)
    floor_cols = [statistics.mean(abs(a_runs[-1][y][x] - b_runs[-1][y][x]) for y in range(18, 30)) for x in range(2, 30)]
    case(res, "C16 corrected: camera pan then hold", "pass",
         max(floor_cols) <= 0.06 and activity_codes(a_runs[-8:], (2, 18, 30, 30)) <= 4.0,
         floor_worst_column=max(floor_cols), floor_activity_codes=activity_codes(a_runs[-8:], (2, 18, 30, 30)),
         persisted=run.stats["persist"], moving=run.stats["moving"])

    # C17: partial coverage. Every vacated pixel, including the square's AA band.
    for background in ("black", "clear"):
        scene = Square(12, 3.0, background)
        t = scene.start + 2
        cells = [(x, y) for y in range(9, 21) for x in range(5, 17) if x + 1.0 <= scene.offset(t) - 2.0]
        for algo, expect in (("draft", "fail"), ("corrected", "pass")):
            outs = run_frames(Run(scene, algo, size=size), t + 1, 8)
            case(res, f"C17 ({algo}): partial-coverage band over {background}", expect,
                 max_residual(outs[t], full(scene, t, size), cells) <= 0.02,
                 max_residual=max_residual(outs[t], full(scene, t, size), cells))

    # C19: three static owners meet; a bar passes near the junction and leaves.
    tri, calm = Triple(4, 3.0), Triple(None, 3.0)
    run = Run(tri, "corrected", size=size)
    a = run_frames(run, 26, 8)[-1]
    b = run_frames(Run(calm, "corrected", size=size), 26, 8)[-1]
    cells = [(x, y) for y in range(10, 22) for x in range(8, 24)]
    case(res, "C19 corrected: triple junction after a passing bar", "pass",
         max_residual(a, b, cells) <= 0.06, max_residual=max_residual(a, b, cells),
         overflow_pixel_frames=run.stats["overflow"])

    # C5: static wires over clear and over a static floor must persist in every phase.
    for floor, label in ((None, "clear"), (0.1, "floor")):
        for algo in ("draft", "corrected"):
            scene = Wires([(8.3, 1.6), (16.3, 0.8)], floor)
            outs = run_frames(Run(scene, algo, size=size), 48, 8)
            p16 = [statistics.mean(max(f[y][x] for y in range(5, 13)) for x in range(2, 30)) for f in outs[-8:]]
            p08 = [statistics.mean(max(f[y][x] for y in range(13, 21)) for x in range(2, 30)) for f in outs[-8:]]
            bg = 0.0 if floor is None else floor
            case(res, f"C5 ({algo}): static wires over {label} present in every phase", "pass",
                 min(p08) - bg > 0.15 and (max(p08) - min(p08)) <= 0.2 * (statistics.mean(p08) - bg) + 0.02,
                 wire_1_6_min=min(p16), wire_0_8_min=min(p08), wire_0_8_max=max(p08))
    # V5: a 0.4 px wire between the 8/16 sample rows is never sampled.
    scene = Wires([(20.3, 0.4)])
    rows = sorted({round(((k + 0.5 - jy) * 2.0) % 2.0, 4) for k in range(16) for _, jy in JITTER})
    outs = run_frames(Run(scene, "corrected", size=size), 24, 8)
    pres = max(max(f[y][x] for y in range(17, 24) for x in range(2, 30)) for f in outs[-8:])
    case(res, "V5 limit: 0.4 px wire between 8/16 sub-rows gets no sample", "pass", pres == 0.0,
         sub_row_offsets=str(rows), max_presence=pres)

    # C3/F3/F4: static chart detail and static activity (sRGB codes).
    for kind, floor_fraction in (("lines1", 0.30), ("checker2", 0.40), ("checker4", 0.65)):
        scene = Chart(kind)
        outs = run_frames(Run(scene, "corrected", size=size), 48, 8)
        frac = contrast_fraction(outs[-1], full(scene, 0, size), (4, 4, 28, 28))
        act = activity_codes(outs[-8:], (4, 4, 28, 28))
        case(res, f"C3 corrected: static {kind} at 8/16", "pass",
             frac >= floor_fraction and act <= max(base_act.get(kind, 0.0), 3.0),
             contrast_fraction=frac, activity_codes=act, activity_535003d7_codes=base_act.get(kind, 0.0))
    outs = run_frames(Run(Chart("checker2"), "corrected", size=size, flat_cap=True), 48, 8)
    act = activity_codes(outs[-8:], (4, 4, 28, 28))
    case(res, "F3 flat W cap 9: static checker2 shimmer at 8/16", "fail", act <= 3.0,
         activity_codes=act, activity_535003d7_codes=base_act["checker2"])
    outs = run_frames(Run(Chart("lines1"), "corrected", size=size, clip_static=True), 32, 8)
    flicker = statistics.mean(max(f[y][x] for f in outs[-8:]) - min(f[y][x] for f in outs[-8:])
                              for y in range(4, 28) for x in range(4, 28))
    case(res, "F4 bias with per-frame clamp (D1): 1-texel lines flicker", "fail", flicker <= 0.05,
         linear_activity=flicker)

    # F8: the rejected colour-ratio shading test. On static, converged texture the
    # ratio of the current kernel estimate to the history at the same pixel must
    # stay near 1 to separate shading changes from aliasing; its tails reach a
    # 0.2 shadow ratio, so it cannot.
    run = Run(Chart("checker2"), "corrected", size=size)
    ratios = []
    original = run.pixel_corrected

    def probe(P, taps, ctx, index):
        st = run.hist.get(index) if run.hist is not None else None
        wc = sum(tp[0] for tp in taps)
        if ctx["t"] >= 32 and st is not None and st[0] > 1e-3 and wc > 0.0 \
                and 4 <= index[0] < 28 and 4 <= index[1] < 28:
            ratios.append(sum(tp[0] * tp[1] for tp in taps) / wc / st[0])
        return original(P, taps, ctx, index)
    run.pixel_corrected = probe
    run_frames(run, 40, 8)
    ratios.sort()
    p05, p95 = ratios[int(0.05 * len(ratios))], ratios[int(0.95 * len(ratios))]
    case(res, "F8 current/history colour ratio cannot separate a 0.2 shadow from aliasing", "fail",
         p05 > 0.4 and p95 < 2.5, p05=p05, p95=p95, samples=len(ratios))

    # C8/F5/K1: a caster's shadow band moves over a static textured receiver.
    # Lag = signed error against a converged run with the band fixed where it
    # is now: pixels that brightened in the last 3 frames should not be darker,
    # pixels that darkened should not be brighter.
    T = 25
    lags = {}
    for key in ("pcf3", "gather2", "none"):
        scene = Shadow(12, 1, key)
        run = Run(scene, "corrected", size=size)
        a = run_frames(run, T + 1, 8)[-1]
        c0 = -6 + (T - 12)
        b = run_frames(Run(Shadow(None, 0, key, fixed=c0), "corrected", size=size), T + 1, 8)[-1]
        brighter = [(x, y) for y in range(4, 28) for x in range(2, 30)
                    if scene.pcf(x + 0.5, T) > scene.pcf(x + 0.5, T - 3) + 1e-9]
        darker = [(x, y) for y in range(4, 28) for x in range(2, 30)
                  if scene.pcf(x + 0.5, T) < scene.pcf(x + 0.5, T - 3) - 1e-9]
        lag = statistics.mean([b[y][x] - a[y][x] for x, y in brighter] + [a[y][x] - b[y][x] for x, y in darker])
        lags[key] = (lag, run.stats["key_resets"], len(brighter), len(darker))
    case(res, "C8 corrected, exact PCF key: moving shadow lag", "pass", lags["pcf3"][0] <= 0.03,
         signed_lag=lags["pcf3"][0], key_resets=lags["pcf3"][1])
    case(res, "F5 no shading key: moving shadow lags", "fail", lags["none"][0] <= 0.03, signed_lag=lags["none"][0])
    case(res, "K1 gather 2x2 key", "pass" if lags["gather2"][0] <= 0.03 else "fail",
         lags["gather2"][0] <= 0.03, signed_lag=lags["gather2"][0], key_resets=lags["gather2"][1])
    full_key, gated_key = Run(Shadow(12, 1, "pcf3"), "corrected", size=size), Run(Shadow(12, 1, "pcf3", gated=True), "corrected", size=size)
    a = run_frames(full_key, T + 1, 8)
    b = run_frames(gated_key, T + 1, 8)
    worst = max(abs(fa[y][x] - fb[y][x]) for fa, fb in zip(a, b) for y in range(32) for x in range(32))
    case(res, "K2 gated exact key equals the ungated key", "pass", worst == 0.0, worst_difference=worst,
         ungated_evaluations=full_key.stats["key_evaluations"], gated_evaluations=gated_key.stats["key_evaluations"])

    # R1: reset value after a key change (reported): signed lag and mean |difference|.
    reset_rows = {}
    for w_reset in (0.25, 0.125, 0.0):
        scene = Shadow(12, 1, "pcf3")
        a = run_frames(Run(scene, "corrected", size=size, w_reset=w_reset), T + 1, 8)[-1]
        b = run_frames(Run(Shadow(None, 0, "pcf3", fixed=-6 + (T - 12)), "corrected", size=size), T + 1, 8)[-1]
        brighter = [(x, y) for y in range(4, 28) for x in range(2, 30)
                    if scene.pcf(x + 0.5, T) > scene.pcf(x + 0.5, T - 3) + 1e-9]
        darker = [(x, y) for y in range(4, 28) for x in range(2, 30)
                  if scene.pcf(x + 0.5, T) < scene.pcf(x + 0.5, T - 3) - 1e-9]
        cells = [(x, y, 1.0) for x, y in brighter] + [(x, y, -1.0) for x, y in darker]
        reset_rows[f"lag_at_{w_reset}"] = statistics.mean(sgn * (b[y][x] - a[y][x]) for x, y, sgn in cells)
        reset_rows[f"abs_at_{w_reset}"] = statistics.mean(abs(b[y][x] - a[y][x]) for x, y, _ in cells)
    case(res, "R1 reset value after a key change (reported)", "pass", True, **reset_rows)

    # C6/F9: a light change between frames. With the host's frame-global flag the
    # history is clipped and its confidence reset; without it the old lighting lags.
    lag = {}
    for suspend in (True, False):
        relit = Run(Relit(24, suspend), "corrected", size=size)
        outs = run_frames(relit, 28, 8)
        converged = run_frames(Run(Relit(0, False), "corrected", size=size), 28, 8)[-1]
        lag[suspend] = statistics.mean(outs[25][y][x] - converged[y][x] for y in range(4, 28) for x in range(4, 28))
    case(res, "C6 corrected: light change with frame-global suspension", "pass", abs(lag[True]) <= 0.03,
         signed_lag_second_frame=lag[True])
    case(res, "F9 light change without the frame-global flag lags", "fail", abs(lag[False]) <= 0.03,
         signed_lag_second_frame=lag[False])

    # C9: scale changes with the confidence rescale, relative to the converged
    # 16/16 steady state (the e1b26907 figures used an under-converged reference).
    ref = full(Chart("checker2"), 0, size)
    steady = contrast_fraction(run_frames(Run(Chart("checker2"), "corrected", size=size), 48, 16)[-1], ref, (4, 4, 28, 28))
    for rescale, expect in ((True, "pass"), (False, "fail")):
        up = run_frames(Run(Chart("checker2"), "corrected", size=size, rescale=rescale), 40,
                        lambda t: 4 if t < 32 else 16)
        third = contrast_fraction(up[34], ref, (4, 4, 28, 28)) / steady
        fourth = contrast_fraction(up[35], ref, (4, 4, 28, 28)) / steady
        late = contrast_fraction(up[39], ref, (4, 4, 28, 28))
        name = "C9 corrected: 4/16 -> 16/16 recovery" if rescale else "F6 no confidence rescale: slow 4/16 -> 16/16"
        case(res, name, expect, third >= 0.65 and fourth >= 0.85, third_frame=third, fourth_frame=fourth,
             third_vs_frame_39=contrast_fraction(up[34], ref, (4, 4, 28, 28)) / late,
             fourth_vs_frame_39=contrast_fraction(up[35], ref, (4, 4, 28, 28)) / late)
    ref4 = full(Chart("checker4"), 0, size)
    down = run_frames(Run(Chart("checker4"), "corrected", size=size), 40, lambda t: 12 if t < 32 else 4)
    kept = contrast_fraction(down[32], ref4, (4, 4, 28, 28)) / contrast_fraction(down[31], ref4, (4, 4, 28, 28))
    case(res, "C9 corrected: 12/16 -> 4/16 retention", "pass", kept >= 0.9, first_frame_kept=kept)

    # C10: the frame after a cut repeats the first frame exactly.
    run = Run(Chart("sine"), "corrected", size=size)
    first = run.frame(0, 8)
    for t in range(1, 11):
        run.frame(t, 8)
    run.cut()
    case(res, "C10 corrected: cut restarts bit-identically", "pass", run.frame(11, 8) == first)

    # C2: interior cycle mass for n = 4..16 (full 3x3 windows only).
    lo, hi = math.inf, -math.inf
    for n in range(4, 17):
        run = Run(Chart("sine"), "corrected", size=(48, 48))
        masses = {}
        for t in range(8):
            run.frame(t, n)
            for key, m in run.last_mass.items():
                masses[key] = masses.get(key, 0.0) + m
        inner = [m for (x, y), m in masses.items() if 18 <= x < 30 and 18 <= y < 30]
        lo, hi = min(lo, min(inner)), max(hi, max(inner))
    case(res, "C2 interior cycle mass within [7.9, 8.1] for n = 4..16", "pass", 7.9 <= lo and hi <= 8.1,
         minimum=lo, maximum=hi)

    # C21: fractional, offset viewport at 8/16.
    rect = (3.25, 5.5, 41.0, 37.0)
    run = Run(Chart("sine"), "corrected", size=(48, 48), rect=rect)
    masses = {}
    outs = []
    for t in range(16):
        outs.append(run.frame(t, 8))
        if t >= 8:
            for key, m in run.last_mass.items():
                masses[key] = masses.get(key, 0.0) + m
    inside = [m for (x, y), m in masses.items() if 10 <= x < 36 and 12 <= y < 36]
    edge = [m for (x, y), m in masses.items() if not (10 <= x < 36 and 12 <= y < 36)]
    outside_zero = all(outs[-1][y][x] == 0.0 for y in range(48) for x in range(48)
                       if not (rect[0] <= x + 0.5 < rect[0] + rect[2] and rect[1] <= y + 0.5 < rect[1] + rect[3]))
    finite = all(math.isfinite(v) for row in outs[-1] for v in row)
    case(res, "C21 fractional viewport: outside zero, finite, interior mass in bound", "pass",
         outside_zero and finite and 7.9 <= min(inside) and max(inside) <= 8.1,
         interior_min=min(inside), interior_max=max(inside), edge_min=min(edge), edge_max=max(edge))
    return res


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    results = cases()
    unexpected = [r["case"] for r in results if not r["as_expected"]]
    report = {"model": "0073 portable reconstruction repair model",
              "python": sys.version.split()[0],
              "cases": results,
              "unexpected": unexpected}
    text = json.dumps(report, indent=1, sort_keys=False)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 1 if unexpected else 0


if __name__ == "__main__":
    sys.exit(main())
