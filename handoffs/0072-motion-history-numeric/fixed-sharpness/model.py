#!/usr/bin/env python3
"""Bounded 1-D CPU model of fixed-mode temporal accumulation at e23776df, for the
sharpness design. Standard library only, deterministic, foreground, about 6 s.

It reproduces, along one axis, the rules of temporal/resolve.wgsl and temporal/copy.wgsl
at e23776df: jittered point samples (jitter.rs SAMPLES, geometry shifted by +j), motion =
j_prev - j_curr - v (motion/shader.wgsl), the 3-tap current neighbourhood (a vertical edge
or line makes the 3x3 exactly this 1-D window), view-depth range admission with the
review tolerances, clear as the far plane, closest-depth motion with centre priority,
positive-tap renormalized bilinear history, canonical clean(), the per-channel clamp, the
reactive weight (history 0.9, threshold 4.0, strength 0.5) and the premultiplied
positive-alpha bilinear presentation un-jitter. One scalar colour channel plus alpha.

Variants: the current baseline; A, a Catmull-Rom history fetch used only when every
nonzero-weight tap is admitted (otherwise the unchanged bilinear path); B, a Catmull-Rom
presentation clamped to its bilinear footprint; A+B; C, history kept on the unjittered
grid (the history fetch subtracts the frame's global jitter delta, so static content is
fetched at integer positions, and the presentation copy becomes an identity load); C+A,
C with the guarded Catmull-Rom fetch for fractional object motion; C-reg, C whose
history-free pixels keep today's registered (bilinear un-jittered) current reconstruction;
and negative controls (current-only, unclamped A, an unsharp-masked baseline, A
renormalized without fallback).
Results are model predictions for axis-aligned edges/lines, not GPU or image results.

  python3 -I -B model.py
"""

import math

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
WEIGHT, REL, ABS, THRESHOLD, STRENGTH = 0.9, 0.05, 0.02, 4.0, 0.5
NEAR_PLANE, FAR_PLANE, SKY = 0.1, 100.0, 0.008  # SKY: displayed colour behind clear
N, FRAMES = 48, 96


def cr(f):
    """Catmull-Rom (Keys a = -0.5) weights for taps base-1 .. base+2 at fraction f."""
    f2, f3 = f * f, f * f * f
    return (-0.5 * f3 + f2 - 0.5 * f, 1.5 * f3 - 2.5 * f2 + 1.0,
            -1.5 * f3 + 2.0 * f2 + 0.5 * f, 0.5 * f3 - 0.5 * f2)


def clean(v):
    rgb, a = v
    if not (math.isfinite(rgb) and math.isfinite(a)) or a <= 0.0:
        return (0.0, 0.0)
    return (min(max(rgb, 0.0), 65504.0), min(max(a, 0.0), 1.0))


class Scene:
    """Foreground intervals over a background; fg/bg are (value, alpha, view depth)."""

    def __init__(self, intervals, fg, bg, velocity=0.0):
        self.intervals, self.fg, self.bg, self.velocity = intervals, fg, bg, velocity

    def surface(self, x, t):
        shift = self.velocity * t
        inside = any(a + shift <= x < b + shift for a, b in self.intervals)
        return (self.fg, self.velocity) if inside else (self.bg, 0.0)

    def displayed(self, surface):
        value, alpha, _ = surface
        return value * alpha + (1.0 - alpha) * SKY

    def ideal(self, i, t):
        """Box (pixel-area) average of the displayed scene over [i, i+1)."""
        shift = self.velocity * t
        cover = sum(max(0.0, min(i + 1.0, b + shift) - max(float(i), a + shift)) for a, b in self.intervals)
        cover = min(cover, 1.0)
        return cover * self.displayed(self.fg) + (1.0 - cover) * self.displayed(self.bg)


def unjitter(cur, i, j):
    """Today's copy.wgsl reconstruction applied to the current frame: bilinear, premultiplied,
    positive-alpha taps, clamped indices; the registered value of a history-free pixel."""
    q = i + j
    base = math.floor(q)
    f = q - base
    acc = [0.0, 0.0]
    for k, w in ((base, 1.0 - f), (base + 1, f)):
        v = cur[min(max(k, 0), N - 1)]
        if w > 0 and v[1] > 0:
            acc = [acc[0] + v[0] * w, acc[1] + v[1] * w]
    return (acc[0], min(max(acc[1], 0.0), 1.0))


def run(scene, axis, variant):
    """Returns displayed frames (list of lists) for frames 0..FRAMES-1."""
    history = depth_prev = None
    shown = []
    for t in range(FRAMES):
        j = SAMPLES[t % 8][axis]
        jp = SAMPLES[(t - 1) % 8][axis]
        cur, dep, mot = [], [], []
        for i in range(N):
            (value, alpha, depth), v = scene.surface(i + 0.5 - j, t)
            cur.append(clean((value * alpha, alpha)))
            dep.append(depth if alpha > 0 else FAR_PLANE)
            mot.append(jp - j - v if alpha > 0 else None)  # clear has no motion record
        if history is None or variant.get("weight", WEIGHT) == 0.0:
            out = [unjitter(cur, i, j) for i in range(N)] if variant.get("registered") else list(cur)
        else:
            out = [resolve(i, cur, dep, mot, history, depth_prev, variant, jp - j, j) for i in range(N)]
        history, depth_prev = out, dep
        shown.append(present(out, j, variant))
    return shown


def resolve(i, cur, dep, mot, history, depth_prev, variant, jitter_delta, j):
    c = cur[i]
    alone = unjitter(cur, i, j) if variant.get("registered") else c  # history-free output
    if c[1] > 0 and mot[i] is None:
        return alone
    lo_c, hi_c = list(c), list(c)
    near, far = FAR_PLANE, NEAR_PLANE
    static = jitter_delta if variant.get("unjittered") else 0.0  # clear centre: static motion
    m, closest = (mot[i], dep[i]) if c[1] > 0 else (static, math.inf)
    for k in (i - 1, i, i + 1):
        k = min(max(k, 0), N - 1)
        n = cur[k]
        lo_c = [min(lo_c[0], n[0]), min(lo_c[1], n[1])]
        hi_c = [max(hi_c[0], n[0]), max(hi_c[1], n[1])]
        if n[1] == 0.0:
            far = FAR_PLANE
            continue
        if mot[k] is None:
            continue
        near, far = min(near, dep[k]), max(far, dep[k])
        if dep[k] < closest:
            m, closest = mot[k], dep[k]
    if far < near:
        return alone
    near -= max(ABS, REL * near)
    far += max(ABS, REL * far)
    old = i + 0.5 + m - (jitter_delta if variant.get("unjittered") else 0.0)
    if old < 0 or old >= N:
        return alone
    q = old - 0.5
    base = math.floor(q)
    f = q - base

    def admitted(k):
        return 0 <= k <= N - 1 and near <= depth_prev[k] <= far

    h = None
    if variant.get("history") == "cr":
        taps = [(base - 1 + n, w) for n, w in enumerate(cr(f)) if w != 0.0]
        if variant.get("cr_renormalize"):  # negative control: no fallback
            ok = [(k, w) for k, w in taps if admitted(k)]
            mass = sum(w for _, w in ok)
            if ok and mass > 1e-6:
                h = clean((sum(history[k][0] * w for k, w in ok) / mass,
                           sum(history[k][1] * w for k, w in ok) / mass))
        elif all(admitted(k) for k, _ in taps):
            h = clean((sum(history[k][0] * w for k, w in taps), sum(history[k][1] * w for k, w in taps)))
    if h is None:  # unchanged accepted bilinear positive-tap path
        acc, mass = [0.0, 0.0], 0.0
        for k, w in ((base, 1.0 - f), (base + 1, f)):
            if w > 0.0 and admitted(k):
                acc = [acc[0] + history[k][0] * w, acc[1] + history[k][1] * w]
                mass += w
        if not mass > 0.0:
            return alone
        h = clean((acc[0] / mass, acc[1] / mass))
    reaction = min(max(abs(h[0] - c[0]) / THRESHOLD, 0.0), 1.0)
    weight = variant.get("weight", WEIGHT) * (1.0 - STRENGTH * reaction)
    if not variant.get("no_clamp"):
        h = (min(max(h[0], lo_c[0]), hi_c[0]), min(max(h[1], lo_c[1]), hi_c[1]))
    return clean((c[0] + (h[0] - c[0]) * weight, c[1] + (h[1] - c[1]) * weight))


def present(out, j, variant):
    shown = []
    if variant.get("unjittered"):  # history already on the unjittered grid: identity copy
        return [v[0] + (1.0 - v[1]) * SKY for v in out]
    for i in range(N):
        q = i + j  # copy.wgsl: p.xy + jitter - 0.5 with p at the pixel centre
        base = math.floor(q)
        f = q - base
        tap = lambda k: out[min(max(k, 0), N - 1)]
        if variant.get("present") == "cr":
            acc = [0.0, 0.0]
            for n, w in enumerate(cr(f)):
                v = tap(base - 1 + n)
                if v[1] > 0:
                    acc = [acc[0] + v[0] * w, acc[1] + v[1] * w]
            a, b = tap(base), tap(base + 1)
            lo = (min(a[0], b[0]), min(a[1], b[1]))
            hi = (max(a[0], b[0]), max(a[1], b[1]))
            v = clean((min(max(acc[0], lo[0]), hi[0]), min(max(acc[1], lo[1]), hi[1])))
        else:
            acc = [0.0, 0.0]
            for k, w in ((base, 1.0 - f), (base + 1, f)):
                v = tap(k)
                if w > 0 and v[1] > 0:
                    acc = [acc[0] + v[0] * w, acc[1] + v[1] * w]
            v = (acc[0], min(max(acc[1], 0.0), 1.0))
        shown.append(v[0] + (1.0 - v[1]) * SKY)
    if variant.get("unsharp"):
        k = variant["unsharp"]
        shown = [s + k * (s - (shown[max(i - 1, 0)] + s + shown[min(i + 1, N - 1)]) / 3.0)
                 for i, s in enumerate(shown)]
    return shown


def metrics(scene, frames, window):
    """Steady-state metrics over the last full jitter cycle (frames FRAMES-8 .. FRAMES-1)."""
    fg, bg = scene.displayed(scene.fg), scene.displayed(scene.bg)
    contrast, lo, hi = abs(fg - bg), min(fg, bg), max(fg, bg)
    last = range(FRAMES - 8, FRAMES)
    err = flick = over = 0.0
    sq = cnt = 0
    for t in last:
        ideal = [scene.ideal(i, t) for i in range(N)]
        prev_ideal = [scene.ideal(i, t - 1) for i in range(N)]
        for i in window:
            e = frames[t][i] - ideal[i]
            sq += e * e
            cnt += 1
            moved = (frames[t][i] - frames[t - 1][i]) - (ideal[i] - prev_ideal[i])
            flick = max(flick, abs(moved))
            over = max(over, frames[t][i] - hi, lo - frames[t][i])
    mean = [sum(frames[t][i] for t in last) / 8.0 for i in range(N)]
    return {"rms": math.sqrt(sq / cnt) / contrast, "flicker": flick / contrast,
            "overshoot": max(over, 0.0) / contrast, "mean": mean}


def width_and_bias(scene, mean, window):
    fg, bg = scene.displayed(scene.fg), scene.displayed(scene.bg)
    norm = [(mean[i] - bg) / (fg - bg) for i in range(N)]
    ideal = [(scene.ideal(i, 0) - bg) / (fg - bg) for i in range(N)]
    bias = sum(norm[i] - ideal[i] for i in window)

    def crossing(level):
        for i in window:
            a, b = norm[i], norm[i + 1]
            if (a - level) * (b - level) <= 0 and a != b:
                return i + 0.5 + (level - a) / (b - a)
        return float("nan")
    lo, hi = crossing(0.1), crossing(0.9)
    return abs(hi - lo), bias


def detail(values, window):
    return sum(abs(values[i] - (values[i - 1] + values[i] + values[i + 1]) / 3.0) for i in window) / len(window)


VARIANTS = [
    ("baseline (e23776df)", {}),
    ("A: Catmull-Rom history", {"history": "cr"}),
    ("B: Catmull-Rom present", {"present": "cr"}),
    ("A+B", {"history": "cr", "present": "cr"}),
    ("C: unjittered history, identity copy", {"unjittered": True}),
    ("C+A: C with Catmull-Rom history", {"unjittered": True, "history": "cr"}),
    ("C-reg: C, registered history-free", {"unjittered": True, "registered": True}),
    ("N1: current-only (no history)", {"weight": 0.0}),
    ("N2: A without 3x3 clamp", {"history": "cr", "no_clamp": True}),
    ("N3: baseline + unsharp 0.8", {"unsharp": 0.8}),
    ("N4: A renormalized, no fallback", {"history": "cr", "cr_renormalize": True}),
]
IDEAL = "box ideal (reference)"
OPAQUE_DARK, OPAQUE_BRIGHT, CLEAR = (0.05, 1.0, 5.0), (0.5, 1.0, 10.0), (0.0, 0.0, FAR_PLANE)


def scenarios():
    for offset in (0.0, 0.2, 0.45, 0.7):
        e = 24 + offset
        yield "static depth step", offset, Scene([(e, 99.0)], OPAQUE_DARK, OPAQUE_BRIGHT), range(18, 30)
        yield "static clear step", offset, Scene([(e, 99.0)], OPAQUE_BRIGHT, CLEAR), range(18, 30)
        lines = [(8 + offset + 4 * k, 9 + offset + 4 * k) for k in range(8)]
        yield "1-px MASK grille, period 4", offset, Scene(lines, OPAQUE_DARK, OPAQUE_BRIGHT), range(16, 32)
        lines2 = [(8 + offset + 2 * k, 9 + offset + 2 * k) for k in range(16)]
        yield "1-px MASK grille, period 2", offset, Scene(lines2, OPAQUE_DARK, OPAQUE_BRIGHT), range(16, 32)
    for v in (0.25, 0.5, 0.75):
        bar = [(6.0 - FRAMES * v + 16, 12.0 - FRAMES * v + 16)]  # bar centred near pixel 25 at the end
        yield f"bar moving {v} px/frame", v, Scene(bar, OPAQUE_DARK, OPAQUE_BRIGHT, velocity=v), range(10, 40)


def main():
    table = {}
    for name, param, scene, window in scenarios():
        if "step" in name:  # the box ideal measured with the same width/bias function
            ideal_width, _ = width_and_bias(scene, [scene.ideal(i, 0) for i in range(N)], window)
            table.setdefault((name, IDEAL), {"width": []})["width"].append(ideal_width)
        for axis in (0, 1):
            for label, variant in VARIANTS:
                frames = run(scene, axis, variant)
                m = metrics(scene, frames, window)
                row = table.setdefault((name, label), {"rms": [], "flicker": [], "overshoot": [], "width": [],
                                                       "bias": [], "modulation": [], "detail": []})
                row["rms"].append(m["rms"])
                row["flicker"].append(m["flicker"])
                row["overshoot"].append(m["overshoot"])
                if "step" in name:
                    w, b = width_and_bias(scene, m["mean"], window)
                    row["width"].append(w)
                    row["bias"].append(b)
                    fg, bg = scene.displayed(scene.fg), scene.displayed(scene.bg)
                    per_frame = [sum((frames[t][i] - scene.ideal(i, t)) / (fg - bg) for i in window)
                                 for t in range(FRAMES - 8, FRAMES)]
                    row.setdefault("wobble", []).append(max(per_frame) - min(per_frame))
                    # After a reset (cut, DRS restart, first frame): frames 0-3 build history.
                    early = [[(frames[t][i] - scene.ideal(i, t)) / (fg - bg) for i in window] for t in range(4)]
                    row.setdefault("reset_rms", []).append(
                        math.sqrt(sum(e * e for frame in early for e in frame) / (4 * len(window))))
                    row.setdefault("reset_offset", []).append(max(abs(sum(frame)) for frame in early))
                if "moving" in name:
                    (a, b), v = scene.intervals[0], scene.velocity
                    behind = ahead = 0.0
                    for t in range(FRAMES - 8, FRAMES):
                        for i in window:
                            e = abs(frames[t][i] - scene.ideal(i, t)) / abs(scene.displayed(scene.fg) - scene.displayed(scene.bg))
                            if i + 1 <= a + v * t - 2:
                                behind = max(behind, e)
                            if i >= b + v * t + 2:
                                ahead = max(ahead, e)
                    row.setdefault("behind", []).append(behind)
                    row.setdefault("ahead", []).append(ahead)
                if "grille" in name:
                    ideal = [scene.ideal(i, 0) for i in range(N)]
                    row["modulation"].append((max(m["mean"][i] for i in window) - min(m["mean"][i] for i in window))
                                             / (max(ideal[i] for i in window) - min(ideal[i] for i in window)))
                    row["detail"].append(detail(m["mean"], window) / detail(ideal, window))
    groups = []
    for name, *_ in scenarios():
        if name not in groups:
            groups.append(name)
    print("Steady state over one 8-phase cycle after %d frames; both jitter axes; %s" % (
        FRAMES - 8, "values are fractions of the edge contrast (rms, flicker, overshoot), px (width, bias) or ratios to the box ideal (modulation, detail)."))
    for name in groups:
        print("\n== " + name)
        if (name, IDEAL) in table:
            widths = table[(name, IDEAL)]["width"]
            print("  %-38s 10-90%% width %.2f px (%.2f-%.2f)" % (IDEAL, sum(widths) / len(widths), min(widths), max(widths)))
        for label, _ in VARIANTS:
            row = table[(name, label)]
            parts = ["rms %.3f" % (sum(row["rms"]) / len(row["rms"])),
                     "flicker max %.3f" % max(row["flicker"]),
                     "overshoot %.3f" % max(row["overshoot"])]
            if row["width"]:
                parts += ["10-90%% width %.2f px (%.2f-%.2f)" % (sum(row["width"]) / len(row["width"]), min(row["width"]), max(row["width"])),
                          "bias %+.2f px" % (sum(row["bias"]) / len(row["bias"])),
                          "phase wobble max %.2f px" % max(row["wobble"]),
                          "frames 0-3 rms %.3f, edge offset max %.2f px" % (
                              sum(row["reset_rms"]) / len(row["reset_rms"]), max(row["reset_offset"]))]
            if row["modulation"]:
                parts += ["modulation %.2f" % (sum(row["modulation"]) / len(row["modulation"])),
                          "detail %.2f" % (sum(row["detail"]) / len(row["detail"]))]
            if row.get("behind"):
                parts += ["residue >=2 px behind %.3f, ahead %.3f" % (max(row["behind"]), max(row["ahead"]))]
            print("  %-38s %s" % (label, " | ".join(parts)))
    print("\nBox ideal: pixel-area average of the scene; its step width uses the same 10-90% measure (0.80 px only"
          " for an edge on a pixel boundary). Grille modulation and detail are ratios to the box ideal (1.00)."
          "\nResidue: largest |shown - box ideal| (fraction of contrast) at pixels wholly 2+ px behind or ahead"
          " of the moving bar; a stale ghost raises 'behind'.")


if __name__ == "__main__":
    main()
