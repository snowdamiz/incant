#!/usr/bin/env python3
"""0073 owner_quality_reconstruction: scalar CPU controls, standard library only.

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_reconstruction/controls/reconstruction_model.py \
        --out handoffs/0073-upscaling-review/owner_quality_reconstruction/controls/reconstruction_model.json

An f64 scalar mirror of the proposed resolve (../src/.../upscaling/*.wgsl) and of the
ce6171957 baseline, run on small synthetic inputs. Colour is one premultiplied grey
channel plus alpha; depth is linear view depth (0 = clear). It models reconstruction
arithmetic only: no shading, tonemap, MASK, GPU rounding, shadows or rasterisation
beyond point samples. Its numbers are model observations, never rendered results or
gates. Every check is an assertion; the script exits nonzero if one fails.
"""
import argparse
import json
import math
import sys

sys.dont_write_bytecode = True

# ------------------------------------------------------------------ sequences

# ce6171957 temporal/jitter.rs SAMPLES (internal pixels, +x right, +y down).
LATTICE = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
# Proposed upscaling-only sequence (jitter.rs UPSCALE_SAMPLES): the lattice with frames
# 0-3 moved by (-1/16, -1/16) and frames 4-7 by (+1/16, +1/16).
SPLIT = [(x - 1 / 16, y - 1 / 16) if k < 4 else (x + 1 / 16, y + 1 / 16)
         for k, (x, y) in enumerate(LATTICE)]

# --------------------------------------------------------- common.wgsl mirror

FRESH = 0.5        # kernel width at zero admitted evidence, internal pixels
FAR = 100.0
CFG = {"h": 0.9, "rel": 0.05, "abs": 0.02, "threshold": 4.0, "strength": 0.5}


def rho_min(r):
    return 0.35 + 0.325 * max(r - 2.0, 0.0)


def kernel_rho(r, w, fresh=FRESH):
    wide = max(rho_min(r), fresh * r)
    t = min(max(w, 0.0), 1.0)
    return wide + (rho_min(r) - wide) * t


def kernel_norm(r, rho):
    return r * r / (2.0 * math.pi * rho * rho)


def evidence_cap(h, r):
    h = min(max(h, 0.0), 0.99)
    return h / (1.0 - h) * min(r * r, 4.0)


def catmull_rom(d):
    x = abs(d)
    if x < 1:
        return (1.5 * x - 2.5) * x * x + 1
    if x < 2:
        return ((-0.5 * x + 2.5) * x - 4) * x + 2
    return 0.0


def clean(c):
    v, a = c
    if not (math.isfinite(v) and math.isfinite(a)) or a <= 0:
        return (0.0, 0.0)
    return (min(max(v, 0.0), 65504.0), min(a, 1.0))


def admitted(c):
    v, a = c
    return math.isfinite(v) and math.isfinite(a) and v >= 0 and 0 <= a <= 1


class Frame:
    """A resolve uniform: rects in output pixels, jitter in internal pixels."""

    def __init__(self, rect, n, jitter, cfg=CFG):
        s = n / 16
        self.cfg = cfg
        self.output = list(rect)
        self.source = [v * s for v in rect]
        self.ratio = rect[2] / self.source[2]
        self.ju = (jitter[0] / self.source[2], jitter[1] / self.source[3])

    def compatible(self, a, b):
        return (a > 0 and b > 0 and a <= FAR and b <= FAR
                and abs(a - b) <= max(self.cfg["abs"], self.cfg["rel"] * b))

    def source_position(self, p):
        return tuple(self.source[i] + ((p[i] - self.output[i]) / self.output[i + 2] + self.ju[i])
                     * self.source[i + 2] for i in range(2))

    def sample_position(self, t):
        return tuple(self.output[i] + ((t[i] + 0.5 - self.source[i]) / self.source[i + 2] - self.ju[i])
                     * self.output[i + 2] for i in range(2))

    def lo(self, r=None):
        r = r or self.source
        return tuple(math.ceil(r[i] - 0.5) for i in range(2))

    def hi(self, r=None):
        r = r or self.source
        return tuple(math.ceil(r[i] + r[i + 2] - 0.5) - 1 for i in range(2))

    def in_source(self, t):
        lo, hi = self.lo(), self.hi()
        return all(lo[i] <= t[i] <= hi[i] for i in range(2))

    def clamp(self, t):
        lo, hi = self.lo(), self.hi()
        return tuple(min(max(t[i], lo[i]), hi[i]) for i in range(2))

    def inside(self, p):
        o = self.output
        return o[0] <= p[0] < o[0] + o[2] and o[1] <= p[1] < o[1] + o[3]

    def weight(self, p, t, rho):
        pos = self.sample_position(t)
        d2 = (p[0] - pos[0]) ** 2 + (p[1] - pos[1]) ** 2
        return kernel_norm(self.ratio, rho) * math.exp(-d2 / (2 * rho * rho))


# ------------------------------------------------------------------- scenes

class Sample:
    __slots__ = ("c", "z", "owner", "disp")

    def __init__(self, c, z, owner=0, disp=(0.0, 0.0)):
        self.c, self.z, self.owner, self.disp = c, z, owner, disp


def mip_pyramid(base):
    levels = [base]
    while len(levels[-1]) > 1:
        b = levels[-1]
        n = len(b) // 2
        levels.append([[(b[2 * y][2 * x] + b[2 * y][2 * x + 1] + b[2 * y + 1][2 * x] + b[2 * y + 1][2 * x + 1]) / 4
                        for x in range(n)] for y in range(n)])
    return levels


def bilinear_level(level, u, v):
    n = len(level)
    uu, vv = u - 0.5, v - 0.5
    i0, j0 = math.floor(uu), math.floor(vv)
    fu, fv = uu - i0, vv - j0
    out = 0.0
    for dj, wy in ((0, 1 - fv), (1, fv)):
        for di, wx in ((0, 1 - fu), (1, fu)):
            out += level[(j0 + dj) % n][(i0 + di) % n] * wx * wy
    return out


def trilinear(pyr, u, v, lod):
    lod = min(max(lod, 0.0), len(pyr) - 1.0)
    l0 = int(math.floor(lod))
    t = lod - l0
    a = bilinear_level(pyr[l0], u / 2 ** l0, v / 2 ** l0)
    if t == 0 or l0 + 1 >= len(pyr):
        return a
    return a * (1 - t) + bilinear_level(pyr[l0 + 1], u / 2 ** (l0 + 1), v / 2 ** (l0 + 1)) * t


def pattern(kind, n=64, low=0.1, high=0.9):
    if kind == "lines_1_texel":
        return mip_pyramid([[high if x % 2 == 0 else low for x in range(n)] for _ in range(n)])
    cell = {"checker_2_texel": 2, "checker_4_texel": 4}[kind]
    return mip_pyramid([[high if (x // cell + y // cell) % 2 == 0 else low for x in range(n)] for y in range(n)])


TEXEL = 1.05  # output pixels per texel, as on the review sign (about 134 px for 128 texels)


def lod(ratio, bias=0.0):
    return math.log2(ratio / TEXEL) + bias


class Scene:
    """Point-sampled scene. sample(x, y, step, ratio) -> Sample; changed(owner, step)."""

    def __init__(self, sample, changed):
        self.sample, self.changed = sample, changed


def textured(kind, bias=0.0):
    pyr = pattern(kind)
    return Scene(lambda x, y, step, r: Sample((trilinear(pyr, x / TEXEL, y / TEXEL, lod(r, bias)), 1.0), 10.0, 1),
                 lambda owner, step: step == 0)


def wire_scene(thickness, centres):
    """Horizontal wires (owners 3..) against clear, static, depth 20."""
    def sample(x, y, step, r):
        for k, (t, c) in enumerate(zip(thickness, centres)):
            if abs(y - c) < t / 2:
                return Sample((0.8, 1.0), 20.0, 3 + k)
        return Sample((0.0, 0.0), 0.0)
    return Scene(sample, lambda owner, step: step == 0)


def c7_scene(moving, start=8, speed=3.0, x0=10.0, half=6.0):
    """Static 4-texel checker (owner 1, depth 10); a bright square (owner 2, depth 4)
    waits `start` steps then moves `speed` output px per step. Reference: no square."""
    pyr = pattern("checker_4_texel")

    def sample(x, y, step, r):
        cx = x0 + max(step - start, 0) * speed
        if moving and abs(x - cx) < half and abs(y - 16) < half:
            return Sample((0.92, 1.0), 4.0, 2, (speed, 0.0) if step > start else (0.0, 0.0))
        return Sample((trilinear(pyr, x / TEXEL, y / TEXEL, lod(r)), 1.0), 10.0, 1)
    return Scene(sample, lambda owner, step: step == 0 or (owner == 2 and step > start))


def render(frame, scene, step):
    cache = {}

    def at(t):
        if t not in cache:
            x, y = frame.sample_position(t)
            cache[t] = scene.sample(x, y, step, frame.ratio)
        return cache[t]
    return at


# ------------------------------------------------------- shared 2x2 support

def support_2x2(frame, cur, p):
    s = frame.source_position(p)
    q = (s[0] - 0.5, s[1] - 0.5)
    base = (math.floor(q[0]), math.floor(q[1]))
    f = (q[0] - base[0], q[1] - base[1])
    v = a = 0.0
    nearest, anchor, same = FAR + 1, frame.clamp(base), True
    taps = []
    for y in (0, 1):
        for x in (0, 1):
            w = (f[0] if x else 1 - f[0]) * (f[1] if y else 1 - f[1])
            if w <= 0:
                continue
            t = frame.clamp((base[0] + x, base[1] + y))
            smp = cur(t)
            cc = clean(smp.c)
            v += cc[0] * w
            a += cc[1] * w
            if not admitted(smp.c) or smp.c[1] < 1 or smp.z <= 0:
                same = False
            if 0 < smp.z < nearest:
                nearest, anchor = smp.z, t
            taps.append(smp)
    if nearest <= FAR:
        same = same and all(frame.compatible(smp.z, nearest) for smp in taps)
    else:
        same = False
    return (v, a), nearest, anchor, same, taps, q


def clamp_box(frame, cur, anchor, nearest, value):
    lo = hi = value
    owners = set()
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            smp = cur(frame.clamp((anchor[0] + dx, anchor[1] + dy)))
            if admitted(smp.c) and smp.c[1] >= 1 and frame.compatible(smp.z, nearest):
                lo, hi = min(lo, clean(smp.c)[0]), max(hi, clean(smp.c)[0])
                owners.add(smp.owner)
    return lo, hi, owners


def history_taps(frame, prev, old):
    oq = (old[0] - 0.5, old[1] - 0.5)
    b = (math.floor(oq[0]), math.floor(oq[1]))
    f = (oq[0] - b[0], oq[1] - b[1])
    lo, hi = frame.lo(frame.output), frame.hi(frame.output)
    out = []
    for y in (0, 1):
        for x in (0, 1):
            w = (f[0] if x else 1 - f[0]) * (f[1] if y else 1 - f[1])
            if not w > 0:
                continue
            t = (b[0] + x, b[1] + y)
            if lo[0] <= t[0] <= hi[0] and lo[1] <= t[1] <= hi[1]:
                out.append((t, w, prev.get(t)))
    return out


# --------------------------------------------------------- ce6171957 baseline

def sharp_current(frame, cur, q, nearest, bilinear, outer_keep=None):
    """ce6171957 sharp_current: Catmull-Rom 4x4 on the anchor surface, clamped to the
    central 2x2 range; any failing tap keeps the bilinear estimate."""
    base = (math.floor(q[0]), math.floor(q[1]))
    f = (q[0] - base[0], q[1] - base[1])
    total, low, high = 0.0, 1e9, -1e9
    for y in range(-1, 3):
        for x in range(-1, 3):
            w = catmull_rom(x - f[0]) * catmull_rom(y - f[1])
            if w == 0:
                continue
            smp = cur(frame.clamp((base[0] + x, base[1] + y)))
            if not admitted(smp.c) or smp.c[1] < 1 or not frame.compatible(smp.z, nearest):
                return bilinear
            if outer_keep and not (0 <= x <= 1 and 0 <= y <= 1) and not outer_keep(smp.owner):
                return bilinear
            total += clean(smp.c)[0] * w
            if 0 <= x <= 1 and 0 <= y <= 1:
                low, high = min(low, clean(smp.c)[0]), max(high, clean(smp.c)[0])
    return min(max(total, low), high)


def baseline(frame, cur, prev, p, mass, owner, still, changed, ebc=False):
    """ce6171957 resolve; ebc=True gives the earlier ebc7e690 behaviour (bilinear current,
    bilinear history, fixed reactive-reduced fraction)."""
    bil, nearest, anchor, same, taps, q = support_2x2(frame, cur, p)
    if ebc:
        mass = 1e9
    if not same:
        return {"c": bil, "z": 0.0, "rec": frozenset(), "w": 0.0}
    current = (clean((bil[0], 1.0))[0], 1.0)
    if not ebc:
        current = (sharp_current(frame, cur, q, nearest, current[0], (lambda o: not changed(o)) if owner else None), 1.0)
    rec = frozenset(smp.owner for smp in taps if smp.z > 0)
    if prev is None:
        return {"c": current, "z": nearest, "rec": rec, "w": 0.0}
    a = cur(anchor)
    old = (p[0] - a.disp[0], p[1] - a.disp[1])
    if not frame.inside(old):
        return {"c": current, "z": nearest, "rec": rec, "w": 0.0}
    h = frame.cfg["h"]
    if owner and still and not any(changed(smp.owner) for smp in taps if smp.z > 0) \
            and abs(old[0] - p[0]) < 1 / 256 and abs(old[1] - p[1]) < 1 / 256:
        own = prev.get((math.floor(p[0]), math.floor(p[1])))
        if own and own["rec"] and own["c"][1] >= 1 and frame.compatible(own["z"], a.z) \
                and not any(changed(o) for o in own["rec"]):
            frac = min(h, mass / (mass + 1))
            return {"c": (current[0] + (own["c"][0] - current[0]) * frac, 1.0), "z": nearest,
                    "rec": rec | own["rec"], "w": 0.0}
    acc = wsum = 0.0
    joined = set(rec)
    for t, w, hh in history_taps(frame, prev, old):
        if hh is None or hh["c"][1] < 1 or not frame.compatible(hh["z"], a.z):
            continue
        if owner and (not hh["rec"] or a.owner not in hh["rec"]
                      or any(changed(o) and o != a.owner for o in hh["rec"])):
            continue
        joined |= hh["rec"]
        acc += hh["c"][0] * w
        wsum += w
    if not wsum > 0:
        return {"c": current, "z": nearest, "rec": rec, "w": 0.0}
    hist = acc / wsum
    lo, hi, box = clamp_box(frame, cur, anchor, nearest, current[0])
    reaction = min(max(abs(hist - current[0]) / frame.cfg["threshold"], 0.0), 1.0)
    frac = min(h * (1 - frame.cfg["strength"] * reaction), mass / (mass + 1))
    hist = min(max(hist, lo), hi)
    return {"c": (current[0] + (hist - current[0]) * frac, 1.0), "z": nearest,
            "rec": frozenset(joined | box), "w": 0.0}


# ------------------------------------------------------------------ proposed

def surface_evidence(frame, cur, p, nearest, rho, owner_filter):
    s = frame.source_position(p)
    c = (math.floor(s[0]), math.floor(s[1]))
    v = m = 0.0
    owners = set()
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            t = (c[0] + dx, c[1] + dy)
            if not frame.in_source(t):
                continue
            smp = cur(t)
            if not admitted(smp.c) or smp.c[1] < 1 or not frame.compatible(smp.z, nearest):
                continue
            if owner_filter and not owner_filter(smp.owner):
                continue
            w = frame.weight(p, t, rho)
            v += clean(smp.c)[0] * w
            m += w
            owners.add(smp.owner)
    return v, m, owners


def coverage_taps(frame, cur, p, changed):
    """Every 3x3 sample: clear ones always, surfaces only with an unchanged owner."""
    s = frame.source_position(p)
    c = (math.floor(s[0]), math.floor(s[1]))
    out = []
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            t = (c[0] + dx, c[1] + dy)
            if not frame.in_source(t):
                continue
            smp = cur(t)
            if smp.z > 0 and changed(smp.owner):
                continue
            out.append((t, smp))
    return out


PARAMS = {"fresh": FRESH, "prior_ref": 0.0, "prior": "cr", "beta": None, "symmetric": False}


def proposed(frame, cur, prev, p, density, owner, still, changed, fresh=None):
    fresh = PARAMS["fresh"] if fresh is None else fresh
    prior_ref = PARAMS["prior_ref"]
    if PARAMS["beta"] is not None:
        prior_ref = PARAMS["beta"] * evidence_cap(frame.cfg["h"], frame.ratio)
    if PARAMS.get("cycle"):
        prior_ref = min(PARAMS["cycle"], 0.5 * evidence_cap(frame.cfg["h"], frame.ratio))
    bil, nearest, anchor, same, taps, q = support_2x2(frame, cur, p)
    r = frame.ratio
    cap = evidence_cap(frame.cfg["h"], r)
    support = frozenset(smp.owner for smp in taps if smp.z > 0)
    if owner and still and prev is not None and not any(changed(o) for o in support):
        static = nearest > FAR or (abs(cur(anchor).disp[0]) < 1 / 256 and abs(cur(anchor).disp[1]) < 1 / 256)
        if static:
            own = prev.get((math.floor(p[0]), math.floor(p[1])))
            cov = coverage_taps(frame, cur, p, changed)
            seen = {smp.owner for _, smp in cov if smp.z > 0}
            persist = bool(own and own["rec"] and admitted(own["c"])
                           and not any(changed(o) for o in own["rec"])
                           and (seen or own["c"][1] < 1))
            if persist or not same:
                w = min(own["w"], cap) if persist else 0.0
                rho = kernel_rho(r, w, fresh)
                v = a = m = 0.0
                for t, smp in cov:
                    wt = frame.weight(p, t, rho)
                    cc = clean(smp.c)
                    v += cc[0] * wt
                    a += cc[1] * wt
                    m += wt
                rec = frozenset(seen | support) | (own["rec"] if persist else frozenset())
                if m > 0:
                    h = own["c"] if persist else (0.0, 0.0)
                    k = max(prior_ref - w, 0.0)
                    est = bil
                    if k > 0 and same and PARAMS["prior"] == "cr":
                        est = (sharp_current(frame, cur, q, nearest, clean((bil[0], 1.0))[0],
                                             lambda o: not changed(o)), 1.0)
                    color = clean(((h[0] * w + v + k * est[0]) / (w + m + k), (h[1] * w + a + k * est[1]) / (w + m + k)))
                    if not rec:
                        return {"c": color, "z": 0.0, "rec": frozenset(), "w": 0.0}
                    z = nearest if nearest <= FAR else (own["z"] if persist else 0.0)
                    return {"c": color, "z": z, "rec": rec, "w": min(w + m, cap)}
    if not same:
        return {"c": bil, "z": 0.0, "rec": frozenset(), "w": 0.0}
    bilinear = clean((bil[0], 1.0))[0]
    rec = set(support)
    hist, w = bilinear, 0.0
    a_smp = cur(anchor)
    if prev is not None:
        old = (p[0] - a_smp.disp[0], p[1] - a_smp.disp[1])
        if frame.inside(old):
            acc = wsum = ev = 0.0
            joined = set()
            for t, wt, hh in history_taps(frame, prev, old):
                if hh is None or hh["c"][1] < 1 or not frame.compatible(hh["z"], a_smp.z):
                    continue
                if owner and (not hh["rec"] or a_smp.owner not in hh["rec"]
                              or any(changed(o) and o != a_smp.owner for o in hh["rec"])):
                    continue
                joined |= hh["rec"]
                acc += hh["c"][0] * wt
                wsum += wt
                ev += min(hh["w"], cap) * wt
            if wsum > 0:
                hist = acc / wsum
                w = min(ev, cap) * density
                lo, hi, box = clamp_box(frame, cur, anchor, nearest, bilinear)
                reaction = min(max(abs(hist - bilinear) / frame.cfg["threshold"], 0.0), 1.0)
                w *= 1 - frame.cfg["strength"] * reaction
                hist = min(max(hist, lo), hi)
                rec |= joined | box
    keep = (lambda o: o == a_smp.owner or not changed(o)) if owner else None
    v, m, owners = surface_evidence(frame, cur, p, nearest, kernel_rho(r, w, fresh), keep)
    rec |= owners
    if not m > 0:
        return {"c": (bilinear, 1.0), "z": nearest, "rec": frozenset(rec), "w": 0.0}
    k = max(prior_ref - w, 0.0)
    estimate = 0.0
    if k > 0:
        estimate = bilinear if PARAMS["prior"] == "bilinear" else sharp_current(frame, cur, q, nearest, bilinear, keep)
    return {"c": ((hist * w + v + k * estimate) / (w + m + k), 1.0), "z": nearest, "rec": frozenset(rec),
            "w": min(w + m, cap)}


# --------------------------------------------------------------- simulation

def simulate(scene, schedule, rect, pixels, variant, seq, cfg=CFG, fresh=FRESH):
    """schedule: list of (sixteenths, cut_before). Returns per-frame {pixel: (v, a)}."""
    prev, out = None, []
    mass = old_width = 0.0
    phase = 0
    for step, (n, cut) in enumerate(schedule):
        has_prev = prev is not None and not cut
        if not has_prev:
            phase = 0
        frame = Frame(rect, n, seq[phase % 8], cfg)
        width = frame.source[2]
        still = has_prev and width == old_width
        cur = render(frame, scene, step)

        def changed(o, step=step):
            return scene.changed(o, step)
        cap = cfg["h"] / (1 - cfg["h"])
        hm = 0.0
        if has_prev:
            hm = min(max(mass, 0.0), cap)
            if 0 < old_width < width:
                hm *= (old_width / width) ** 2
        density = (old_width / width) ** 2 if has_prev and 0 < old_width < width else 1.0
        if PARAMS["symmetric"] and has_prev and old_width > 0:
            density = (old_width / width) ** 2
        result = {}
        for px in pixels:
            p = (px[0] + 0.5, px[1] + 0.5)
            if not frame.inside(p):
                continue
            if variant in ("ce617_up", "ce617_own", "ebc_up"):
                res = baseline(frame, cur, prev if has_prev else None, p, hm, variant == "ce617_own", still, changed,
                               variant == "ebc_up")
            else:
                res = proposed(frame, cur, prev if has_prev else None, p, density, variant == "new_own", still,
                               changed, fresh)
            result[px] = res
        prev = result
        out.append({px: v["c"] for px, v in result.items()})
        mass = min(hm + 1, cap)
        old_width = width
        phase += 1
    return out


def full_reference(scene, rect, pixels, step=0, bias=0.0):
    """FULL: one unjittered output-resolution point sample per pixel (ratio 1)."""
    out = {}
    for x, y in pixels:
        smp = scene.sample(x + 0.5, y + 0.5, step, 1.0)
        out[(x, y)] = clean(smp.c)
    return out


def drs_reference(scene, rect, pixels, n, step=0):
    """DRS: the unjittered internal frame, nearest texel per output pixel."""
    frame = Frame(rect, n, (0.0, 0.0))
    cur = render(frame, scene, step)
    out = {}
    for x, y in pixels:
        s = frame.source_position((x + 0.5, y + 0.5))
        out[(x, y)] = clean(cur(frame.clamp((math.floor(s[0]), math.floor(s[1])))).c)
    return out


def sd(values):
    m = sum(values) / len(values)
    return math.sqrt(sum((v - m) ** 2 for v in values) / len(values))


def mean_abs(a, b, keys):
    return sum(abs(a[k][0] - b[k][0]) for k in keys) / len(keys)


# ------------------------------------------------------------- experiments

CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def window(x0, x1, y0, y1):
    return [(x, y) for y in range(y0, y1) for x in range(x0, x1)]


def canonical(frames):
    for f in frames:
        for v, a in f.values():
            if not (math.isfinite(v) and math.isfinite(a) and 0 <= a <= 1 and 0 <= v <= a + 1e-12):
                return False
            if a == 0 and v != 0:
                return False
    return True


def static_detail():
    rect = [0.0, 0.0, 64.0, 32.0]
    win = window(8, 56, 8, 24)
    out = {}
    for kind in ("lines_1_texel", "checker_2_texel", "checker_4_texel"):
        scene = textured(kind)
        full = full_reference(scene, rect, win)
        drs = drs_reference(scene, rect, win, 8)
        row = {"full_sd": sd([full[k][0] for k in win]), "drs_sd": sd([drs[k][0] for k in win]),
               "drs_error": mean_abs(drs, full, win)}
        runs = {"ce617_up": ("ce617_up", LATTICE), "ce617_own": ("ce617_own", LATTICE),
                "new_up": ("new_up", SPLIT), "new_own": ("new_own", SPLIT),
                "new_up_lattice": ("new_up", LATTICE)}
        for name, (variant, seq) in runs.items():
            frames = simulate(scene, [(8, False)] * 40, rect, win, variant, seq)
            last = frames[-1]
            activity = max(max(f[k][0] for f in frames[-8:]) - min(f[k][0] for f in frames[-8:]) for k in win)
            row[name] = {"sd": sd([last[k][0] for k in win]), "error": mean_abs(last, full, win),
                         "activity": activity, "canonical": canonical(frames)}
        # Dependency estimate only (not in this patch): a -1 texture LOD bias at 2x.
        biased = textured(kind, -1.0)
        frames = simulate(biased, [(8, False)] * 40, rect, win, "new_own", SPLIT)
        row["dependency_lod_bias_new_own"] = {"sd": sd([frames[-1][k][0] for k in win]),
                                              "error": mean_abs(frames[-1], full, win)}
        out[kind] = row
    c2 = out["checker_2_texel"]
    check("2-texel checker: proposed UP and OWN contrast at least nearest DRS, ce617 below it",
          c2["new_up"]["sd"] >= c2["drs_sd"] * 0.95 and c2["new_own"]["sd"] >= c2["drs_sd"] * 0.95
          and c2["ce617_up"]["sd"] < c2["drs_sd"], {k: round(c2[k]["sd"], 4) for k in ("ce617_up", "new_up", "new_own")})
    check("2-texel checker: proposed error against FULL below ce617 and DRS",
          c2["new_own"]["error"] < c2["ce617_up"]["error"] and c2["new_own"]["error"] < c2["drs_error"])
    for kind, row in out.items():
        check(f"{kind}: proposed outputs canonical", row["new_up"]["canonical"] and row["new_own"]["canonical"])
        check(f"{kind}: proposed last-cycle activity at most 0.02", max(row["new_up"]["activity"],
                                                                       row["new_own"]["activity"]) <= 0.02,
              {"new_up": round(row["new_up"]["activity"], 5), "new_own": round(row["new_own"]["activity"], 5)})
    return out


def presence(frame, ys, xs):
    """Mean over columns of the strongest coverage within rows ys (the review's presence)."""
    return sum(max(frame[(x, y)][1] for y in ys) for x in xs) / len(xs)


def thin_wires():
    rect = [0.0, 0.0, 48.0, 40.0]
    thick = (1.6, 0.8, 0.4)
    centres = (10.37, 20.21, 30.29)
    scene = wire_scene(thick, centres)
    win = window(8, 40, 4, 37)
    xs = range(10, 38)
    rows = [range(int(c) - 2, int(c) + 3) for c in centres]
    full = full_reference(scene, rect, win)
    out = {"full": [presence(full, r, xs) for r in rows]}
    for name, (variant, seq) in {"ce617_up": ("ce617_up", LATTICE), "ce617_own": ("ce617_own", LATTICE),
                                 "new_up": ("new_up", SPLIT), "new_own": ("new_own", SPLIT),
                                 "new_own_lattice": ("new_own", LATTICE)}.items():
        frames = simulate(scene, [(8, False)] * 40, rect, win, variant, seq)
        per = [[presence(f, r, xs) for f in frames[-16:]] for r in rows]
        out[name] = {"min": [min(v) for v in per], "max": [max(v) for v in per],
                     "mean": [sum(v) / len(v) for v in per], "canonical": canonical(frames)}
    n, b = out["new_own"], out["ce617_up"]
    check("thin wires: ce617 blinks (some wire falls to zero presence in a phase)", min(b["min"]) == 0.0, b["min"])
    check("thin wires: proposed OWN keeps every wire present in every phase", min(n["min"]) > 0.05, n["min"])
    check("thin wires: proposed OWN phase range at most a quarter of its mean",
          all(mx - mn <= 0.25 * me for mn, mx, me in zip(n["min"], n["max"], n["mean"])),
          [round(mx - mn, 4) for mn, mx in zip(n["min"], n["max"])])
    check("thin wires: the lattice never samples the 0.4 px wire; the split sequence does",
          out["new_own_lattice"]["max"][2] == 0.0 and n["min"][2] > 0.0,
          {"lattice": out["new_own_lattice"]["max"][2], "split": n["min"][2]})
    check("thin wires: proposed outputs canonical", n["canonical"] and out["new_up"]["canonical"])
    return out


def disocclusion(fresh_values=(FRESH,)):
    """C7 mirror: columns vacated more than 2 frames (6 px) before the step, rows 2 px
    inside the square, per-column mean |moving - same-variant never-occluded reference|."""
    rect = [0.0, 0.0, 72.0, 32.0]
    win = window(0, 72, 6, 27)
    moving, still = c7_scene(True), c7_scene(False)
    out = {}
    runs = {"ebc_up": ("ebc_up", LATTICE, FRESH), "ce617_up": ("ce617_up", LATTICE, FRESH),
            "ce617_own": ("ce617_own", LATTICE, FRESH)}
    for f in fresh_values:
        runs[f"new_up_fresh{f}"] = ("new_up", SPLIT, f)
        runs[f"new_own_fresh{f}"] = ("new_own", SPLIT, f)
    start_left = 10.0 - 6.0
    for name, (variant, seq, fresh) in runs.items():
        a = simulate(moving, [(8, False)] * 21, rect, win, variant, seq, fresh=fresh)
        b = simulate(still, [(8, False)] * 21, rect, win, variant, seq, fresh=fresh)
        worst = {}
        for step in (12, 16, 20):
            left = 10.0 + (step - 8) * 3.0 - 6.0
            cols = range(math.ceil(start_left), max(math.floor(left - 6.0), math.ceil(start_left)))
            per = [sum(abs(a[step][(x, y)][0] - b[step][(x, y)][0]) for y in range(12, 20)) / 8 for x in cols]
            worst[step] = max(per) if per else 0.0
        out[name] = {"worst_trailing_column": worst, "canonical": canonical(a)}
    return out


SCHEDULE = [(8, False)] * 10 + [(12, False)] * 3 + [(4, False)] * 3 + [(16, False)] * 4 + [(8, False)] * 3 \
    + [(8, True)] + [(8, False)] * 4


def transitions():
    rect = [3.25, 5.5, 64.0, 32.0]
    win = window(12, 60, 10, 30)
    out = {}
    for kind in ("checker_2_texel", "checker_4_texel"):
        scene = textured(kind)
        row = {}
        for name, (variant, seq) in {"ce617_up": ("ce617_up", LATTICE), "new_up": ("new_up", SPLIT),
                                     "new_own": ("new_own", SPLIT)}.items():
            frames = simulate(scene, SCHEDULE, rect, win, variant, seq)
            fixed = simulate(scene, [(16, False)] * 24, rect, win, variant, seq)
            ref = sd([fixed[-1][k][0] for k in win])
            c = [sd([f[k][0] for k in win]) for f in frames]

            def pop(i):
                return sum(abs(frames[i][k][0] - frames[i - 1][k][0]) for k in win) / len(win)
            row[name] = {"third_16": c[18] / ref, "fourth_16": c[19] / ref, "first_4_over_last_12": c[13] / c[12],
                         "pop_4_to_16": pop(16), "pop_16_to_8": pop(20), "canonical": canonical(frames)}
        out[kind] = row
        check(f"transitions {kind}: proposed outputs canonical", row["new_up"]["canonical"] and row["new_own"]["canonical"])
    return out


def odd_rect():
    rect = [3.25, 5.5, 65.0, 33.0]
    win = window(4, 68, 6, 38)
    scene = textured("checker_4_texel")
    full = full_reference(scene, rect, win)
    out = {}
    for n in (5, 7, 9, 11, 13, 15):
        row = {}
        for name, (variant, seq) in {"ce617_up": ("ce617_up", LATTICE), "new_up": ("new_up", SPLIT),
                                     "new_own": ("new_own", SPLIT)}.items():
            frames = simulate(scene, [(n, False)] * 24, rect, win, variant, seq)
            inner = [k for k in win if k in frames[-1]]
            row[name] = {"error": mean_abs(frames[-1], full, inner), "pixels": len(inner),
                         "canonical": canonical(frames)}
        out[n] = row
        check(f"odd DRS n={n} fractional rectangle: every output pixel resolved and canonical",
              row["new_up"]["canonical"] and row["new_own"]["canonical"]
              and row["new_up"]["pixels"] == row["ce617_up"]["pixels"] > 0)
        check(f"odd DRS n={n}: proposed error against FULL not above ce617",
              row["new_up"]["error"] <= row["ce617_up"]["error"] + 1e-9,
              {k: round(v["error"], 5) for k, v in row.items()})
    return out
