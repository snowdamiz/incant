#!/usr/bin/env python3
"""0073 repair model, second addendum delta. CPU only, Python standard library.

Imports the committed model (model.py, unchanged; its output results.json stays
the immutable record of the 7b3dc64c rules) and adds the rules and
counterexamples of quality-repair-addendum-2.md:

  * shadow invalidation by changed casters' swept light-space footprints
    ("rect", required) instead of the scalar shadow key; a per-light 8-bit
    visibility signature ("slots") is kept as the measured alternative;
  * explicit camera-state ownership, separate from screen displacement;
  * persistence only while every stored and current contributor is unchanged;
  * static drops by owner identity, not depth;
  * moving-path admission by owner identity and expected previous depth, with
    the admitted history's owners kept as provenance.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/model_v2.py \
        --out handoffs/0073-upscaling-review/repair-model/results-2.json

All 50 retained cases are re-run twice with the "corrected" algorithm replaced
by these rules ("draft" stays e1b26907): once per shadow detector. New cases
pair each accepted repair with the counterexample it fixes. The exit status is
nonzero if any outcome differs from its expectation.
"""
import argparse
import json
import math
import os
import statistics
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model as m  # noqa: E402  (the committed 7b3dc64c model, unchanged)

SLOT_LEVELS = 255          # 8-bit per-slot visibility code
SLOT_TRIGGER = 16          # codes: |delta| >= 16/255 (> 1/16) invalidates


def quantize(v):
    return int(math.floor(min(max(v, 0.0), 1.0) * SLOT_LEVELS + 0.5))


def slots_of(scene, x, y, t):
    # Per shadowed-light visibility at the stored world point (<= 4 slots).
    if hasattr(scene, "slots"):
        return tuple(scene.slots(x, y, t))
    return (scene.shade(x, y, t),)


def prev_depth_of(scene, x, y, t, z):
    return scene.prev_depth(x, y, t) if hasattr(scene, "prev_depth") else z


def rect_hit(scene, x, y, t):
    """Whether the receiver lies in a changed caster's swept light-space
    footprint this frame (previous and current bounds, plus PCF support). The
    host lists these per shadowed light; no shadow map is read."""
    if hasattr(scene, "rect_hit"):
        return scene.rect_hit(x, y, t)
    if isinstance(scene, m.Shadow):
        if scene.key == "none":             # retained "no detector" variant (F5)
            return False
        b0, b1 = scene.band(t - 1), scene.band(t)
        if b0 == b1:
            return False
        lo, hi = min(b0, b1) - 1, max(b0, b1) + 6 + 1   # 6-texel caster, 3-tap PCF support
        return lo <= math.floor(x / scene.TEXEL) < hi
    return False


class Schedule(dict):
    """Drops by frame ({t: pixels}); `resets` holds confidence resets the same
    way, so a matched reference can replay both."""

    def __init__(self):
        super().__init__()
        self.resets = {}


class RunV2(m.Run):
    """Addendum-2 rules on top of the 7b3dc64c corrected rules."""

    def __init__(self, scene, algo, key_mode="rect", admit_depth="previous", identity=True, provenance=True,
                 **kw):
        super().__init__(scene, algo, **kw)
        self.key_mode, self.admit_depth = key_mode, admit_depth
        self.identity, self.provenance = identity, provenance   # False: retained alternatives
        self.dropped = Schedule()
        self.stats.update({"camera_blocked": 0, "identity_rejected": 0, "rect_tests": 0,
                           "rect_invalidations": 0, "owner_resets": 0})
        self.reset_now = ()

    def frame(self, t, n):
        if self.algo == "draft":
            return super().frame(t, n)
        sc = self.scene
        ox, oy, ow, oh = self.rect
        r = 16.0 / n
        sx, sy, sw, sh = (v * n / 16.0 for v in self.rect)
        jx, jy = m.JITTER[self.phase]
        rho = m.rho_of(r)
        norm = r * r / (2 * math.pi * rho * rho)
        near = max(r, 4.0 * rho)
        wmax = m.wmax_of(r, self.flat_cap)
        lod = m.lod_of(r, self.bias)
        slx, shx = m.lo_hi(sx, sw)
        sly, shy = m.lo_hi(sy, sh)
        kx, ky = ow / sw, oh / sh
        cur = {}
        for k in range(sly, shy + 1):
            for i in range(slx, shx + 1):
                X = ox + (i + 0.5 - sx - jx) * kx
                Y = oy + (k + 0.5 - sy - jy) * ky
                s = sc.sample(X, Y, t, lod)
                cur[(i, k)] = (X, Y) + s + (prev_depth_of(sc, X, Y, t, s[2]),)
        self.drop_now = self.drops.get(t, ()) if self.drops is not None else ()
        self.reset_now = getattr(self.drops, "resets", {}).get(t, ()) if self.drops is not None else ()
        ctx = {"changed": sc.changed(t), "cam_static": sc.camera_static(t), "suspended": sc.suspended(t),
               "rescale": 1.0 if (self.prev_n is None or n <= self.prev_n or not self.use_rescale)
               else (self.prev_n / n) ** 2,
               "wmax": wmax, "near2": near * near, "t": t, "rho": rho}
        width, height = self.size
        out = [[0.0] * width for _ in range(height)]
        new = {}
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
                        if tap is None:
                            continue
                        X, Y, val, a, z, owner, dx, dy, zp = tap
                        d2 = (X - P[0]) ** 2 + (Y - P[1]) ** 2
                        taps.append((norm * math.exp(-d2 / (2 * rho * rho)), val * a, a, z, owner, dx, dy, d2, zp))
                self.last_mass[(px, py)] = sum(tp[0] for tp in taps)
                new[(px, py)], out[py][px] = self.pixel_corrected(P, taps, ctx, (px, py))
        self.hist = new
        self.prev_n = n
        self.phase = (self.phase + 1) % 8
        return out

    @staticmethod
    def left(howners, hfront, hz, near, changed):
        """A changed stored owner whose samples are gone from the near taps: its
        contribution is stale and cannot be separated, so the history drops.
        Identity, not depth: coplanar owners tie in depth. With an overflowed
        owner set only the front owner is known (depth fallback)."""
        present = {tp[4] for tp in near if tp[3] > 0.0}
        if howners is None:
            return hfront in changed and not any(m.compatible(tp[3], hz) for tp in near if tp[3] > 0.0)
        return any(o in changed and o not in present for o in howners)

    def key_changed(self, old, new):
        if old is None or len(old) != len(new):
            return False
        return any(abs(a - b) >= SLOT_TRIGGER for a, b in zip(old, new))

    def pixel_corrected(self, P, taps, ctx, index):
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
        screen_static = ((abs(src[5]) < m.STATIC_EPS and abs(src[6]) < m.STATIC_EPS) if src is not None
                         else True)
        # Camera-state ownership: unclipped persistence needs an unchanged camera,
        # whatever the pixel's screen displacement.
        static = screen_static and ctx["cam_static"]
        box = self.box_of(near)
        owners = contrib if len(contrib) <= 2 else None
        st = None if (self.hist is None or index in self.drop_now) else self.hist.get(index)
        if st is not None and index in self.reset_now:
            st = st[:3] + (min(st[3], self.w_reset),) + st[4:]
        used, key_store = None, None
        if self.hist is None:
            self.stats["fresh"] += 1
        elif static and st is not None:
            hc, ha, hz, hw, hkey, howners, hfront = (st[0], st[1], st[2], min(st[3], ctx["wmax"]) * ctx["rescale"],
                                                       st[4], st[5], st[8])
            if (howners is not None and not (howners & ctx["changed"]) and not (contrib & ctx["changed"])
                    and not ctx["suspended"]):
                invalid = False
                if self.key_mode == "rect":
                    # Conservative: any receiver inside a changed caster's swept
                    # light-space footprint is invalidated. No shadow map is read.
                    self.stats["rect_tests"] += 1
                    invalid = rect_hit(self.scene, P[0], P[1], ctx["t"])
                    if invalid:
                        self.stats["rect_invalidations"] += 1
                elif hkey is not None and not self.scene.gate(P[0], P[1], ctx["t"]):
                    key_store = hkey
                else:
                    key_store = tuple(quantize(v) for v in slots_of(self.scene, P[0], P[1], ctx["t"]))
                    self.stats["key_evaluations"] += 1
                    invalid = self.key_changed(hkey, key_store)
                if invalid:
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
            elif self.left(howners, hfront, hz, near, ctx["changed"]):
                self.stats["dropped"] += 1
                self.dropped.setdefault(ctx["t"], set()).add(index)
            else:
                hc, ha = self.clip(hc, ha, box)
                changed_here = bool(contrib & ctx["changed"]) or (
                    (hfront in ctx["changed"]) if howners is None else bool(howners & ctx["changed"]))
                if ctx["suspended"] or changed_here:
                    # A changed owner covers part of the window now: its coverage at
                    # P may have grown, so old background must not keep its weight.
                    hw = min(hw, self.w_reset)
                    self.stats["owner_resets"] += 1
                    self.dropped.resets.setdefault(ctx["t"], set()).add(index)
                used = (hc, ha, hw)
                self.stats["clip_in_place"] += 1
        elif src is not None and not static and self.hist is not None:
            if screen_static and not ctx["cam_static"]:
                self.stats["camera_blocked"] += 1
            # Same-motion near surfaces: their owners and expected previous depths.
            same = [tp for tp in near if tp[3] > 0.0
                    and abs(tp[5] - src[5]) < m.STATIC_EPS and abs(tp[6] - src[6]) < m.STATIC_EPS]
            expect = {}
            for tp in same:
                expect.setdefault(tp[4], []).append(tp[8] if self.admit_depth == "previous" else tp[3])
            admitted_owners = set()

            def admit(h):
                hz, hfront = h[2], h[8]
                if not self.identity:
                    ok = any(m.compatible(hz, e) for es in expect.values() for e in es)
                else:
                    ok = hfront in expect and any(m.compatible(hz, e) for e in expect[hfront])
                if not ok and hfront not in expect and any(m.compatible(hz, e) for es in expect.values() for e in es):
                    self.stats["identity_rejected"] += 1
                if ok:
                    if h[5] is None:
                        admitted_owners.add(None)            # provenance overflowed
                    else:
                        admitted_owners.update(h[5])
                return ok
            h = self.reproject_v2(P, src, ctx, admit)
            if h is not None:
                used = self.clip(h[0], h[1], box) + (h[2],)
                merged = contrib | frozenset(admitted_owners) if self.provenance else contrib
                owners = merged if (None not in merged and len(merged) <= 2) else None
            self.stats["moving"] += 1
        if owners is None:
            self.stats["overflow"] += 1
        c, a, w_new = self.finish(taps, used, ctx)
        if key_store is None and static and self.key_mode == "slots":
            key_store = tuple(quantize(v) for v in slots_of(self.scene, P[0], P[1], ctx["t"]))
        return (c, a, z_front, w_new, key_store, owners, static, 0, front), c

    def reproject_v2(self, P, src, ctx, admit):
        q = (P[0] - src[5], P[1] - src[6])
        ox, oy, ow, oh = self.rect
        if not (ox <= q[0] < ox + ow and oy <= q[1] < oy + oh):
            return None
        bx, by = math.floor(q[0] - 0.5), math.floor(q[1] - 0.5)
        fx, fy = q[0] - 0.5 - bx, q[1] - 0.5 - by
        lx, hx = m.lo_hi(ox, ow)
        ly, hy = m.lo_hi(oy, oh)
        acc = [0.0, 0.0, 0.0]
        mass = 0.0
        for ex, ey, w in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
            cx, cy = bx + ex, by + ey
            if w <= 0.0 or not (lx <= cx <= hx and ly <= cy <= hy):
                continue
            h = self.hist.get((cx, cy))
            if h is None or not admit(h):
                continue
            acc[0] += w * h[0]
            acc[1] += w * h[1]
            acc[2] += w * min(h[3], ctx["wmax"]) * ctx["rescale"]
            mass += w
        return None if mass <= 0.0 else (acc[0] / mass, acc[1] / mass, acc[2] / mass)


# ------------------------------------------------------------------ scenes
RESPONSES = (0.30876058959827696, 0.0)   # audit: shade_light, front-facing and back-facing


class TwoLights(m.Scene):
    """Static textured receiver lit by two equal-luminance directional lights;
    unseen casters swap their visibility (1,0) -> (0,1) at `at`. Light 2 is
    back-facing (no contribution). Camera, lights and receiver are fixed."""

    def __init__(self, at):
        self.at = at

    def vis(self, t):
        return (1.0, 0.0) if t < self.at else (0.0, 1.0)

    def sample(self, x, y, t, lod):
        v = self.vis(t)
        lit = sum(r * s for r, s in zip(RESPONSES, v)) / RESPONSES[0]
        return (m.tex("checker4", x, y, lod) * (0.2 + 0.8 * lit), 1.0, 2.0, 1, 0.0, 0.0)

    def shade(self, x, y, t):
        return sum(self.vis(t)) / 2.0              # the 7b3dc64c scalar key (equal luminance)

    def slots(self, x, y, t):
        return self.vis(t)

    def rect_hit(self, x, y, t):
        # Both casters change at `at`; their swept footprints cover the receiver.
        return t == self.at


class CameraChange(m.Scene):
    """Zero-screen-motion receiver while the camera changes between `start` and
    `stop`: an orbit about the receiver (view-dependent factor falls) or an
    axial dolly (view depth 5 -> 4, mild view dependence)."""

    def __init__(self, kind, start, stop):
        self.kind, self.start, self.stop = kind, start, stop

    def progress(self, t):
        return min(max((t - self.start + 1) / (self.stop - self.start), 0.0), 1.0)

    def factor(self, t):
        p = self.progress(t)
        if self.kind == "orbit":
            # 20.371832715762604 -> 0.009689776492202838 (audit), normalised to 1.
            return 1.0 + (0.009689776492202838 / 20.371832715762604 - 1.0) * p
        return 1.0 - 0.3 * p

    def depth(self, t):
        return 5.0 - (self.progress(t) if self.kind == "dolly" else 0.0)

    def sample(self, x, y, t, lod):
        return (m.tex("checker4", x, y, lod) * self.factor(t), 1.0, self.depth(t), 1, 0.0, 0.0)

    def prev_depth(self, x, y, t):
        return self.depth(t - 1)

    def camera_static(self, t):
        return not (self.start <= t < self.stop)


class CoplanarSlide(m.Scene):
    """A panel (owner 5) slides 3 px/frame across a coplanar static textured
    wall (owner 1, same depth); the panel wins visibility where it lies."""

    def __init__(self, start, fixed=None):
        self.start, self.fixed = start, fixed

    def edge(self, t):
        if self.fixed is not None:
            return self.fixed
        return 2.0 + 3.0 * max(0, t - self.start)

    def sample(self, x, y, t, lod):
        e = self.edge(t)
        if e - 12.0 <= x < e and 8.0 <= y < 24.0:
            return (0.9, 1.0, 2.0, 5, e - self.edge(t - 1), 0.0)
        return (m.tex("checker4", x, y, lod), 1.0, 2.0, 1, 0.0, 0.0)

    def changed(self, t):
        return frozenset({5}) if self.edge(t) != self.edge(t - 1) else frozenset()


class EnterStop(m.Scene):
    """A bar (owner 3) moves in 2 px/frame and stops at `stop`, partly covering
    pixels whose stored owners never changed. It is nearer than the floor
    (bar_z 1) or at the floor's depth (bar_z 2)."""

    def __init__(self, stop, start=6, bar_z=1.0):
        self.start, self.stop, self.bar_z = start, stop, bar_z

    def edge(self, t):
        return -4.0 + 2.0 * max(0, min(t, self.stop) - self.start)

    def sample(self, x, y, t, lod):
        e = self.edge(t)
        if e - 10.0 <= x < e and 8.0 <= y < 24.0:
            return (0.9, 1.0, self.bar_z, 3, e - self.edge(t - 1), 0.0)
        return (m.tex("checker4", x, y, lod), 1.0, 2.0, 1, 0.0, 0.0)

    def changed(self, t):
        return frozenset({3}) if self.edge(t) != self.edge(t - 1) else frozenset()


def stripes6(x, lod):
    """Vertical 3 px black / 3 px white stripes, trilinear box mips: a 3 px
    shift inverts them."""
    def level(n):
        s = 1 << n
        u = x / s - 0.5
        a = math.floor(u)
        f = u - a

        def texel(i):
            return sum(1.0 if ((i * s + p) // 3) % 2 == 0 else 0.0 for p in range(s)) / s
        return (1 - f) * texel(a) + f * texel(a + 1)
    l0 = math.floor(lod)
    fr = lod - l0
    base = level(l0)
    return base if fr == 0 else (1 - fr) * base + fr * level(l0 + 1)


class StripeSlide(m.Scene):
    """A grey panel (owner 5, depth 2) slides 3 px/frame over a static striped
    wall (owner 1) whose depth is tolerance-close (2.06) or well separated (4)."""

    def __init__(self, start, wall_z):
        self.start, self.wall_z = start, wall_z

    def edge(self, t):
        return 2.0 + 3.0 * max(0, t - self.start)

    def sample(self, x, y, t, lod):
        e = self.edge(t)
        if e - 12.0 <= x < e and 8.0 <= y < 24.0:
            return (0.5, 1.0, 2.0, 5, e - self.edge(t - 1), 0.0)
        return (stripes6(x, lod), 1.0, self.wall_z, 1, 0.0, 0.0)

    def changed(self, t):
        return frozenset({5}) if self.edge(t) != self.edge(t - 1) else frozenset()


class Crossing(m.Scene):
    """A checker panel A (owner 2, depth 1) moves right 2 px/frame from `start`
    and stops at `stop`; a grey bar X (owner 7, depth 2, behind A) keeps moving
    left 3 px/frame. Background: clear or a grey wall (owner 1, depth 3)."""

    def __init__(self, start, stop, background, show_x=True):
        self.start, self.stop, self.background, self.show_x = start, stop, background, show_x

    def a(self, t):
        return 4.0 + 2.0 * max(0, min(t, self.stop) - self.start)

    def xb(self, t):
        return 30.0 - 3.0 * max(0, t - self.start)

    def sample(self, x, y, t, lod):
        a = self.a(t)
        if a <= x < a + 10.0 and 8.0 <= y < 24.0:
            return (m.tex("checker2", x - a, y, lod), 1.0, 1.0, 2, a - self.a(t - 1), 0.0)
        b = self.xb(t)
        if self.show_x and b <= x < b + 4.0 and 4.0 <= y < 28.0:
            return (0.5, 1.0, 2.0, 7, b - self.xb(t - 1), 0.0)
        return m.CLEAR if self.background == "clear" else (0.2, 1.0, 3.0, 1, 0.0, 0.0)

    def changed(self, t):
        moved = set()
        if self.a(t) != self.a(t - 1):
            moved.add(2)
        if self.show_x and self.xb(t) != self.xb(t - 1):
            moved.add(7)
        return frozenset(moved)


# ------------------------------------------------------------------- cases
def new_cases():
    res = []
    size = (32, 32)
    old = lambda s, **kw: m.Run(s, "corrected", size=size, **kw)          # 7b3dc64c rules
    v2 = lambda s, **kw: RunV2(s, "corrected", size=size, **kw)

    # N1 two-light visibility swap: the scalar key stays 0.5.
    T = 16
    interior = [(x, y) for y in range(4, 28) for x in range(4, 28)]
    swapped_ref = TwoLights(-10 ** 6)          # always (0,1): converged answer
    for label, runner, expect in (("7b3dc64c scalar key", old, "fail"),
                                  ("addendum-2 caster footprints", v2, "pass"),
                                  ("alternative per-light signature", lambda s: v2(s, key_mode="slots"), "pass")):
        a = m.run_frames(runner(TwoLights(T)), T + 4, 8)
        b = m.run_frames(runner(swapped_ref), T + 4, 8)
        lags = [statistics.mean(a[t][y][x] - b[t][y][x] for x, y in interior) for t in (T, T + 1, T + 3)]
        m.case(res, f"N1 ({label}): two-light visibility swap at a fixed receiver", expect,
               max(lags) <= 0.03, lag_first=lags[0], lag_second=lags[1], lag_fourth=lags[2])

    # N2/N3 zero-motion camera orbit and dolly.
    for kind, start, stop in (("orbit", 16, 22), ("dolly", 16, 19)):
        base_scene = CameraChange(kind, 10 ** 6, 10 ** 6 + 1)
        for label, runner, expect in (("7b3dc64c screen-static", old, "fail"),
                                      ("addendum-2 camera ownership", v2, "pass")):
            run = runner(CameraChange(kind, start, stop))
            outs = []
            for t in range(stop + 2):
                if t == start:
                    before = dict(run.stats)
                outs.append(run.frame(t, 8))
                if t == stop - 1:
                    during = {k: run.stats.get(k, 0) - before.get(k, 0) for k in run.stats}
            checks = (start, start + 1, stop - 1)
            truths = {t: m.full(CameraChange(kind, start, stop), t, size) for t in checks}
            err = [statistics.mean(abs(outs[t][y][x] - truths[t][y][x]) for x, y in interior) for t in checks]
            base_out = m.run_frames(runner(base_scene), 24, 8)[-1]
            base_full = m.full(base_scene, 0, size)
            base = statistics.mean(abs(base_out[y][x] - base_full[y][x]) for x, y in interior)
            tests = during.get("key_evaluations", 0) + during.get("rect_tests", 0)
            m.case(res, f"N{2 if kind == 'orbit' else 3} ({label}): zero-motion camera {kind}", expect,
                   during.get("persist", 0) == 0 and tests == 0
                   and max(err) <= base + 0.03, error_first=err[0], error_second=err[1], error_last=err[2],
                   static_camera_reference_error=base, persisted_while_camera_changed=during.get("persist", 0),
                   shading_tests_while_camera_changed=tests,
                   camera_blocked=during.get("camera_blocked", 0))
        if kind == "dolly":
            truth = m.full(CameraChange(kind, start, stop), stop - 1, size)
            errs = {}
            for mode in ("current", "previous"):
                run = RunV2(CameraChange(kind, start, stop), "corrected", size=size, admit_depth=mode)
                outs = m.run_frames(run, stop + 2, 8)
                errs[mode] = statistics.mean(abs(outs[stop - 1][y][x] - truth[y][x]) for x, y in interior)
            m.case(res, "F10 dolly admission by current depth rejects valid history", "fail",
                   errs["current"] <= errs["previous"] + 0.01, error_current_depth_admission=errs["current"],
                   error_previous_depth_admission=errs["previous"])

    # N4 coplanar slide: depth equality is not identity. Wall pixels the panel
    # vacated, against a wall the panel never crossed (signed bias) and against
    # that wall reset at the same pixels and frames (matched reset).
    T = 16
    for label, runner, expect in (("7b3dc64c front-owner drop", old, "fail"),
                                  ("addendum-2 any-owner drop", v2, "pass")):
        scene = CoplanarSlide(8)
        run = runner(scene)
        a = m.run_frames(run, T + 1, 8)[-1]
        wall = CoplanarSlide(8, fixed=scene.edge(T))
        b = m.run_frames(runner(wall), T + 1, 8)[-1]
        trailing = scene.edge(T) - 12.0
        cols = [x for x in range(0, 32) if x + 1.0 <= trailing - 2.0]
        bias = [statistics.mean(a[y][x] - b[y][x] for y in range(10, 22)) for x in cols]
        matched = m.run_frames(runner(wall, drops=run.dropped), T + 1, 8)[-1]
        worst = max(abs(a[y][x] - matched[y][x]) for y in range(10, 22) for x in cols)
        m.case(res, f"N4 ({label}): coplanar panel leaves a textured wall", expect, worst <= 0.02,
               worst_matched_reset_residual=worst, worst_signed_bias=max(bias, key=abs), columns=len(cols),
               dropped=run.stats.get("dropped", 0))

    # N5 an owner, nearer than the floor or at its depth, enters and stops. It
    # reaches the pixel in the same frame it first contributes, so no STORED
    # owner changed; next frame it is static. The 7b3dc64c rule keeps the old
    # background at full confidence.
    for bar_z, kind in ((1.0, "nearer"), (2.0, "same-depth")):
        for label, runner, expect in (("7b3dc64c stored owners only", old, "fail"),
                                      ("addendum-2 every involved owner", v2, "pass")):
            scene = EnterStop(12, bar_z=bar_z)
            never = EnterStop(-10 ** 6, bar_z=bar_z)
            never.start = -10 ** 6
            never_edge = scene.edge(30)
            never.edge = lambda t, e=never_edge: e
            a = m.run_frames(runner(scene), 31, 8)[-1]
            b = m.run_frames(runner(never), 31, 8)[-1]
            cells = [(x, y) for y in range(6, 26) for x in range(0, 16)]
            worst = max(abs(a[y][x] - b[y][x]) for x, y in cells)
            m.case(res, f"N5 ({label}): {kind} owner enters and stops; no stale background", expect,
                   worst <= 0.06, worst_residual=worst)

    # N6 a tolerance-close depth is not identity. A static striped wall 0.06
    # behind a sliding panel must give exactly the output of a wall 2.0 behind
    # it; depth-only admission fetches wall history along the panel's motion.
    for label, runner, expect in (("7b3dc64c depth admission", old, "fail"),
                                  ("depth-only admission", lambda s: v2(s, identity=False), "fail"),
                                  ("addendum-2 identity admission", v2, "pass")):
        outs = {z: m.run_frames(runner(StripeSlide(8, z)), 24, 8) for z in (2.06, 4.0)}
        diffs = [abs(p - q) for fa, fb in zip(outs[2.06], outs[4.0]) for ra, rb in zip(fa, fb)
                 for p, q in zip(ra, rb)]
        m.case(res, f"N6 ({label}): a tolerance-close static wall behaves like a separated one", expect,
               max(diffs) <= 1e-9, worst_difference=max(diffs),
               differing_pixel_frames=sum(1 for d in diffs if d > 1e-9))

    # P1 provenance (reported, no criterion). A checker panel stops while a bar
    # that passed behind it keeps moving. Residual against the bar-free scene
    # replaying the run's own drops and resets, away from the bar, for stops 9-13.
    rows = {}
    for background in ("clear", "grey"):
        for n in (8, 16):
            worst = {}
            for provenance in (True, False):
                per_stop = []
                for stop in range(9, 14):
                    scene = Crossing(6, stop, background)
                    run = RunV2(scene, "corrected", size=size, provenance=provenance)
                    a = m.run_frames(run, stop + 6, n)
                    b = m.run_frames(RunV2(Crossing(6, stop, background, show_x=False), "corrected", size=size,
                                           provenance=provenance, drops=run.dropped), stop + 6, n)
                    w = 0.0
                    for t in range(stop, stop + 6):
                        cells = [(x, y) for y in range(8, 24) for x in range(32)
                                 if abs(x + 0.5 - (scene.xb(t) + 2.0)) > 6.0]
                        w = max(w, max(abs(a[t][y][x] - b[t][y][x]) for x, y in cells))
                    per_stop.append(w)
                worst[provenance] = per_stop
            key = f"{background}_{n}of16"
            rows[f"{key}_worst_with"] = max(worst[True])
            rows[f"{key}_worst_without"] = max(worst[False])
            rows[f"{key}_stops_lower_with"] = sum(p < q - 1e-6 for p, q in zip(worst[True], worst[False]))
            rows[f"{key}_stops_lower_without"] = sum(q < p - 1e-6 for p, q in zip(worst[True], worst[False]))
    # The same switch on the other moving scenes: any bitwise output change.
    others = (lambda: m.Square(12, 3.0, "black"), lambda: m.Square(12, 3.0, "clear"),
              lambda: m.Square(12, 3.0, "checker4", stop=16), lambda: m.Occluder(4, 4.0),
              lambda: m.Pan(8, 16, 2.0), lambda: m.Triple(4, 3.0), lambda: m.Triple(4, 1.0),
              lambda: CoplanarSlide(8), lambda: EnterStop(12), lambda: StripeSlide(8, 2.0),
              lambda: StripeSlide(8, 2.06))
    changed_runs = 0
    for make in others:
        for n in (4, 8, 16):
            a = m.run_frames(RunV2(make(), "corrected", size=size), 26, n)
            b = m.run_frames(RunV2(make(), "corrected", size=size, provenance=False), 26, n)
            changed_runs += any(p != q for fa, fb in zip(a, b) for ra, rb in zip(fa, fb) for p, q in zip(ra, rb))
    rows["other_scene_runs"] = 3 * len(others)
    rows["other_scene_runs_changed"] = changed_runs
    m.case(res, "P1 provenance on crossing owners, with and without (reported)", "pass", True, **rows)

    # K3 moving shadow: caster-footprint invalidation (no shadow map read) and
    # the alternative per-light signature (PCF at the stored point).
    T = 25
    rows = {}
    for mode in ("slots", "rect"):
        scene = m.Shadow(12, 1, "pcf3", gated=True)
        run = RunV2(scene, "corrected", size=size, key_mode=mode)
        a = m.run_frames(run, T + 1, 8)[-1]
        c0 = -6 + (T - 12)
        b = m.run_frames(RunV2(m.Shadow(None, 0, "pcf3", fixed=c0), "corrected", size=size), T + 1, 8)[-1]
        brighter = [(x, y) for y in range(4, 28) for x in range(2, 30) if scene.pcf(x + 0.5, T) > scene.pcf(x + 0.5, T - 3) + 1e-9]
        darker = [(x, y) for y in range(4, 28) for x in range(2, 30) if scene.pcf(x + 0.5, T) < scene.pcf(x + 0.5, T - 3) - 1e-9]
        inside = [(x, y) for y in range(4, 28) for x in range(2, 30) if scene.pcf(x + 0.5, T) == 0.0
                  and scene.pcf(x + 0.5, T - 3) == 0.0]
        lag = statistics.mean([b[y][x] - a[y][x] for x, y in brighter] + [a[y][x] - b[y][x] for x, y in darker])
        detail = statistics.mean(abs(a[y][x] - b[y][x]) for x, y in inside) if inside else 0.0
        rows[mode] = (lag, detail, run.stats["key_evaluations"], run.stats["rect_invalidations"])
    m.case(res, "K3 moving shadow: caster-footprint lag <= 0.03 (signature alternative reported)", "pass",
           rows["rect"][0] <= 0.03, rect_lag=rows["rect"][0], rect_shadow_interior_error=rows["rect"][1],
           rect_invalidations=rows["rect"][3], signature_lag=rows["slots"][0],
           signature_shadow_interior_error=rows["slots"][1], signature_evaluations=rows["slots"][2])

    # Q1 8-bit code of the alternative signature. For any values, |dv| < 15/255
    # never triggers and |dv| >= 17/255 always does; between, rounding decides.
    grid = [i / 1020.0 for i in range(1021)]
    codes = [quantize(v) for v in grid]
    early = late = band = band_hits = 0
    for a_, qa in zip(grid, codes):
        for b_, qb in zip(grid, codes):
            d, hit = abs(a_ - b_), abs(qa - qb) >= SLOT_TRIGGER
            if d < 15.0 / 255.0:
                early += hit
            elif d >= 17.0 / 255.0:
                late += not hit
            else:
                band += 1
                band_hits += hit
    levels = sorted({round(i / 9.0, 12) for i in range(10)} | {round(i / 64.0, 12) for i in range(65)})
    extra = missed = 0
    for a_ in levels:
        for b_ in levels:
            exact = abs(a_ - b_) > 1.0 / 16.0
            coded = abs(quantize(a_) - quantize(b_)) >= SLOT_TRIGGER
            extra += coded and not exact
            missed += exact and not coded
    m.case(res, "Q1 signature code: never below 15/255, always from 17/255", "pass", early == 0 and late == 0,
           grid_pairs=len(grid) ** 2, triggers_below=early, misses_from=late, band_pairs=band,
           band_triggered=band_hits, pcf_level_pairs=len(levels) ** 2, pcf_level_extra_vs_one_sixteenth=extra,
           pcf_level_missed_vs_one_sixteenth=missed)
    return res


def retained_under_v2(key_mode):
    original_run = m.Run

    def factory(scene, algo, **kw):
        if algo == "corrected":
            return RunV2(scene, algo, key_mode=key_mode, **kw)
        return original_run(scene, algo, **kw)
    m.Run = factory
    try:
        return m.cases()
    finally:
        m.Run = original_run


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    retained = {mode: retained_under_v2(mode) for mode in ("rect", "slots")}
    added = new_cases()
    # The brief's original C9 stays a recorded failure under either reference;
    # retained C9 checks only the addendum-1 replacement (0.65/0.85).
    c9 = next(r for r in retained["rect"] if r["case"] == "C9 corrected: 4/16 -> 16/16 recovery")["values"]
    m.case(added, "C9 as written (e1b26907): >= 0.75 by the third 16/16 frame and >= 0.90 by the fourth", "fail",
           (c9["third_frame"] >= 0.75 and c9["fourth_frame"] >= 0.90)
           or (c9["third_vs_frame_39"] >= 0.75 and c9["fourth_vs_frame_39"] >= 0.90), **c9)
    unexpected = ([f"{mode}: {r['case']}" for mode in retained for r in retained[mode] if not r["as_expected"]]
                  + [r["case"] for r in added if not r["as_expected"]])
    report = {"model": "0073 repair model, addendum-2 delta (imports unchanged model.py)",
              "python": sys.version.split()[0],
              "detectors": {
                  "rect": "required: changed casters' swept light-space footprints; in retained cases key "
                          "'none' disables it and 'pcf3'/'gather2' both select it",
                  "slots": "alternative: per-light 8-bit visibility at the stored point; for one light it "
                           "equals the 7b3dc64c key"},
              "retained_cases_rect": retained["rect"],
              "retained_cases_slots": retained["slots"],
              "new_cases": added,
              "unexpected": unexpected}
    text = json.dumps(report, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 1 if unexpected else 0


if __name__ == "__main__":
    sys.exit(main())
