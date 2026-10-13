#!/usr/bin/env python3
"""0073 addendum-3 support witnesses. CPU only, Python standard library.

Tiny deterministic checks for quality-repair-addendum-3.md: one static output
pixel with the real 8-phase jitter and the brief 2.1-2.3 kernel, plus an exact
check of the 3x3 support extent. No image simulation; model.py, model_v2.py and
their results are not imported or changed.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/witness_v3.py \
        --out handoffs/0073-upscaling-review/repair-model/results-3.json

Rules compared on the same pixel:
  a2       addendum 2: owners and changed checks over near taps (d <= R) only;
  a2_full  addendum 2 with owners over every positive-weight tap, but its
           overflow rule and owners <- contributors when clipping in place;
  a3       addendum 3: full support, union provenance in every kept branch,
           overflow class mask with a drop when authenticity is unprovable.
The exit status is nonzero if any outcome differs from its expectation.
"""
import argparse
import json
import math
import sys

sys.dont_write_bytecode = True

# temporal/jitter.rs SAMPLES (internal px), as in model.py and root's witness.
JITTER = ((-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
          (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125))
W_RESET = 0.125
CLASSES = 31                     # overflow class mask: bit (id mod 31)
EXTENT_MARGIN = 1.0 / 64.0       # source texels, covers f32 rounding of floor(s)


def kernel(r):
    rho = 0.35 + 0.325 * max(r - 2.0, 0.0)
    # W_max = h / (1 - h) * min(r^2, 4) with h = 0.9, taken as exactly 9 as in
    # root's witness (f64 0.9 / (1 - 0.9) evaluates to 9.000000000000002).
    return {"r": r, "rho": rho, "norm": r * r / (2.0 * math.pi * rho * rho),
            "near": max(r, 4.0 * rho), "wmax": 9.0 * min(r * r, 4.0)}


K1 = kernel(1.0)                 # 16/16: rho 0.35, R 1.4, W_max 9


def offsets(phase):
    # Pixel centre P, integer origins, 16/16: tap t - P = d - J for d in {-1,0,1}^2.
    jx, jy = JITTER[phase]
    return [(dx - jx, dy - jy) for dy in (-1, 0, 1) for dx in (-1, 0, 1)]


def classes(owners):
    mask = 0
    for o in owners:
        mask |= 1 << (o % CLASSES)
    return mask


def compatible(a, b):
    return a > 0.0 and b > 0.0 and abs(a - b) <= max(0.02, 0.05 * b)


class Pixel:
    """One static pixel under one rule. `drops`/`resets` replay another run's
    schedule (matched reference); `dropped`/`reset` record this run's."""

    def __init__(self, scene, rule, drops=(), resets=()):
        self.scene, self.rule = scene, rule
        self.drops, self.resets = set(drops), set(resets)
        self.dropped, self.reset = [], []
        self.st = None

    def step(self, t):
        k = K1
        taps = []
        for x, y in offsets(t % 8):
            v, o, z = self.scene.sample(x, y, t)
            d2 = x * x + y * y
            w = k["norm"] * math.exp(-d2 / (2.0 * k["rho"] * k["rho"]))
            taps.append((w, v, o, z, d2 <= k["near"] * k["near"]))
        S = Wc = 0.0
        for w, v, _, _, _ in taps:          # plain left-to-right sums (sum() compensates on 3.12+)
            S += w * v
            Wc += w
        near = [tp for tp in taps if tp[4]]
        box = (min(tp[1] for tp in near), max(tp[1] for tp in near))
        near_owners = frozenset(tp[2] for tp in near if tp[3] > 0.0)
        support = frozenset(tp[2] for tp in taps if tp[3] > 0.0)
        contrib = near_owners if self.rule == "a2" else support
        geo = [tp for tp in near if tp[3] > 0.0]
        front = min(geo, key=lambda tp: tp[3]) if geo else None
        front = (front[2], front[3]) if front else (0, 0.0)
        changed = self.scene.changed(t)
        st = None if (self.st is None or t in self.drops) else dict(self.st)
        if st is not None and t in self.resets:
            st["W"] = min(st["W"], W_RESET)
        used, owners, mask = None, contrib, None
        if st is not None:
            hw = min(st["W"], k["wmax"])
            if st["owners"] is not None:
                sc, cc = st["owners"] & changed, contrib & changed
                if not sc and not cc:
                    used, owners = (st["c"], hw), st["owners"] | contrib          # persist
                elif any(o not in near_owners for o in sc):
                    self.dropped.append(t)                                       # a changed owner left
                else:
                    c = min(max(st["c"], box[0]), box[1])
                    if sc or cc:
                        hw = min(hw, W_RESET)
                        self.reset.append(t)
                    used = (c, hw)
                    owners = (st["owners"] | contrib) if self.rule == "a3" else contrib
            elif self.rule == "a3":
                m = st["mask"] | classes(contrib)
                if m & classes(changed):
                    self.dropped.append(t)                                       # authenticity unprovable
                else:
                    used, owners, mask = (st["c"], hw), None, m                  # provably unchanged
            else:
                hfront, hz = st["front"]
                if hfront in changed and not any(compatible(tp[3], hz) for tp in near if tp[3] > 0.0):
                    self.dropped.append(t)
                else:
                    c = min(max(st["c"], box[0]), box[1])
                    if (contrib & changed) or hfront in changed:
                        hw = min(hw, W_RESET)
                        self.reset.append(t)
                    used, owners = (c, hw), contrib
        if used is None:
            c, W = S / Wc, min(Wc, k["wmax"])
        else:
            c, W = (used[0] * used[1] + S) / (used[1] + Wc), min(used[1] + Wc, k["wmax"])
        if owners is not None and len(owners) > 2:
            # Kept branches already hold stored | current owners; a drop holds current only.
            owners, mask = None, (classes(owners) if self.rule == "a3" else None)
        self.st = {"c": c, "W": W, "owners": owners, "mask": mask, "front": front}
        return c


def run(scene, rule, frames, **kw):
    px = Pixel(scene, rule, **kw)
    return px, [px.step(t) for t in range(frames)]


# ------------------------------------------------------------------ scenes
A, B, Y, E = 1, 2, 3, 7
EMITTER = (1.3, -0.95, 1.45, -0.8)        # root's patch, relative to P


class Scene:
    def __init__(self, emitter_until=504, emitter_moves=505, strip=False, enter=None, extra=None):
        self.emitter_until, self.emitter_moves = emitter_until, emitter_moves
        self.strip, self.enter, self.extra = strip, enter, extra or {}

    def sample(self, x, y, t):
        if (self.emitter_until is not None and t <= self.emitter_until
                and EMITTER[0] <= x <= EMITTER[2] and EMITTER[1] <= y <= EMITTER[3]):
            return 65504.0, E, 1.0                                   # emissive, not a caster
        if self.enter is not None and t >= self.enter and 0.5 <= x <= 1.45 and -0.5 <= y <= 0.5:
            return 1.0, Y, 1.5                                       # arrives at `enter`, then stays
        if self.strip and x <= -0.6:
            return 1.0, B, 2.0                                       # static white strip
        return 0.0, A, 2.0                                           # black background

    def changed(self, t):
        moved = set()
        if self.emitter_until is not None and t == self.emitter_moves:
            moved.add(E)
        if self.enter is not None and t == self.enter:
            moved.add(Y)
        moved |= self.extra.get(t, set())
        return frozenset(moved)


def case(res, name, expect, ok, **values):
    res.append({"case": name, "expect": expect, "observed": "pass" if ok else "fail",
                "as_expected": (expect == "pass") == bool(ok), "values": values})


def matched(scene, reference, rule, frames, window):
    """Residual of `scene` against `reference` replaying the same drops/resets."""
    px, a = run(scene, rule, frames)
    _, b = run(reference, rule, frames, drops=px.dropped, resets=px.reset)
    return [abs(a[t] - b[t]) for t in window], px


def cases():
    res = []
    # E1 support extent for actual raster sample centres, scales and viewports.
    worst = {}
    for n in range(4, 17):
        kx = 16.0 / n
        axis = euclid = 0.0
        for ox, oy in ((0.0, 0.0), (0.3, 0.45), (0.7, 0.15), (1.5, 2.25)):
            ow, oh = 37.0, 29.0
            sx, sy, sw, sh = ox * n / 16.0, oy * n / 16.0, ow * n / 16.0, oh * n / 16.0
            for phase in range(8):
                ju, jv = JITTER[phase][0] / sw, JITTER[phase][1] / sh
                for py in range(math.ceil(oy - 0.5), math.ceil(oy + oh - 0.5)):
                    for qx in range(math.ceil(ox - 0.5), math.ceil(ox + ow - 0.5)):
                        P = (qx + 0.5, py + 0.5)
                        s = (sx + ((P[0] - ox) / ow + ju) * sw, sy + ((P[1] - oy) / oh + jv) * sh)
                        base = (math.floor(s[0]), math.floor(s[1]))
                        for dk in (-1, 0, 1):
                            for di in (-1, 0, 1):
                                tx, ty = base[0] + di, base[1] + dk
                                if not (sx <= tx + 0.5 < sx + sw and sy <= ty + 0.5 < sy + sh):
                                    continue
                                X = ox + ((tx + 0.5 - sx) / sw - ju) * ow
                                Yp = oy + ((ty + 0.5 - sy) / sh - jv) * oh
                                ex, ey = abs(X - P[0]) / kx, abs(Yp - P[1]) / kx
                                axis = max(axis, ex, ey)
                                euclid = max(euclid, math.hypot(ex, ey))
        k = kernel(kx)
        worst[n] = {"max_axis_texels": round(axis, 9), "max_radius_texels": round(euclid, 9),
                    "support_radius_px": round(math.sqrt(2.0) * (1.5 + EXTENT_MARGIN) * kx, 4),
                    "near_radius_px": round(k["near"], 4)}
    case(res, "E1 every positive-weight tap lies within 1.5 source texels of P per axis", "pass",
         all(v["max_axis_texels"] <= 1.5 + 1e-9 for v in worst.values()), per_numerator=worst)

    window = list(range(505, 520))
    # S1 root's witness: an emitter seen only by a far 3x3 tap, then removed.
    for rule, expect in (("a2", "fail"), ("a3", "pass")):
        px, out = run(Scene(), rule, 520)
        tail = [out[t] for t in window]
        case(res, f"S1 ({rule}): emitter only in far taps, removed at 505", expect, max(tail) <= 1e-12,
             radiance_504=out[504], radiance_505=out[505], radiance_512=out[512], radiance_519=out[519],
             drops_at_removal=[t for t in px.dropped if t >= 505])
    _, a2 = run(Scene(), "a2", 520)
    case(res, "S1 replay equals root's retained values bit for bit", "pass",
         (a2[504], a2[512], a2[519]) == (0.2930896432402765, 0.1263291343794111, 0.060404993732075994),
         root=[0.2930896432402765, 0.1263291343794111, 0.060404993732075994],
         replay=[a2[504], a2[512], a2[519]])

    # O1 overflow: background, a white strip and the far emitter are three owners;
    # the emitter (unrecorded id) leaves. Matched against the emitter-free scene.
    for rule, expect in (("a2_full", "fail"), ("a3", "pass")):
        resid, px = matched(Scene(strip=True), Scene(emitter_until=None, strip=True), rule, 520, window)
        case(res, f"O1 ({rule}): an overflowed pixel's unrecorded third owner leaves", expect,
             max(resid) <= 1e-12, residual_505=resid[0], residual_512=resid[7], residual_519=resid[14],
             drops_at_removal=[t for t in px.dropped if t >= 505])
    # O1 false positives: a non-contributor changes at 300 with a colliding or a free class.
    rows = {}
    for label, other in (("colliding_class", 32), ("free_class", 40)):
        px, _ = run(Scene(strip=True, extra={300: {other}}), "a3", 400)
        rows[f"{label}_owner"] = other
        rows[f"{label}_drops"] = [t for t in px.dropped if 290 <= t <= 310]
    case(res, "O1 cost: a non-contributor drops only on a class collision (reported)", "pass",
         rows["colliding_class_drops"] == [300] and rows["free_class_drops"] == [], **rows)

    # U1 union: an owner arrives in a phase without emitter samples and stays;
    # the emitter leaves two frames later.
    for rule, expect in (("a2_full", "fail"), ("a3", "pass")):
        resid, px = matched(Scene(emitter_until=506, emitter_moves=507, enter=505),
                            Scene(emitter_until=None, enter=505), rule, 520, list(range(507, 520)))
        case(res, f"U1 ({rule}): provenance kept through a clip-in-place frame", expect, max(resid) <= 1e-12,
             residual_507=resid[0], residual_512=resid[5], residual_519=resid[12],
             resets=[t for t in px.reset if t >= 500], drops=[t for t in px.dropped if t >= 500])

    # B1 unbounded support (algebra): a pan left a fraction f of surface G in P's
    # history; after the stop G lies 5 px from P's ray. A caster rectangle covers G.
    f, lit, shadow, texel = 0.2, 1.0, 0.25, 0.5
    rect = (4.5, 5.5)
    grow = {"a2_near": K1["near"] + 2.25 * texel,
            "a3_bounded": math.sqrt(2.0) * (1.5 + EXTENT_MARGIN) * 1.0 + 2.25 * texel}
    decay = K1["wmax"] / (K1["wmax"] + 1.0)
    for label, half, flagged, expect in (("a2_near", grow["a2_near"], False, "fail"),
                                         ("a3_bounds_without_flag", grow["a3_bounded"], False, "fail"),
                                         ("a3_unbounded_flag", grow["a3_bounded"], True, "pass")):
        # The flag makes any rectangle a hit; a hit on an unbounded history drops it.
        hit = flagged or not (half < rect[0] or -half > rect[1])
        excess = [0.0 if hit else f * (lit - shadow) * decay ** t for t in (0, 7, 14)]
        case(res, f"B1 ({label}): moving-path history meets a caster rectangle 5 px away", expect,
             max(excess) <= 1e-12, grown_half_width_px=round(half, 4), hit=hit,
             excess_first=excess[0], excess_eighth=excess[1], excess_fifteenth=excess[2])
    return res


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    res = cases()
    unexpected = [r["case"] for r in res if not r["as_expected"]]
    report = {"model": "0073 addendum-3 support witnesses (single pixel, analytic)",
              "python": sys.version.split()[0], "cases": res, "unexpected": unexpected}
    text = json.dumps(report, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 1 if unexpected else 0


if __name__ == "__main__":
    sys.exit(main())
