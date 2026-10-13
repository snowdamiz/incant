#!/usr/bin/env python3
"""Bounded 2-D CPU derivation for the C-reg comparison: diagonal and thin features,
moving diagonal edges with depth, and how often C-reg pairs an unjittered history
texel with a raster depth sample from the other surface. Standard library only,
deterministic, foreground, about 40 s. A derivation, not a measurement.

Rules follow temporal/resolve.wgsl and copy.wgsl at e23776df (baseline) and
0c80df4e (C-reg): jitter.rs SAMPLES (geometry shifted by +j), raster motion
j_prev - j_curr - v, 3x3 current neighbourhood, view-depth range admission (review
tolerances 0.05/0.02), closest-depth motion with centre priority, positive-tap
renormalized bilinear history, per-channel clamp, reactive weight (0.9, 4.0, 0.5).
Baseline: history on the jittered raster grid and a bilinear presentation
un-jitter. C-reg: reprojection subtracts the jitter delta, history-free pixels use
the registered current reconstruction, presentation is an identity copy. Opaque
scalar colour; the 1-D model.py covers clear coverage and alpha.

  python3 -I -B model2d.py
"""

import math

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
WEIGHT, REL, ABS, THRESHOLD, STRENGTH = 0.9, 0.05, 0.02, 4.0, 0.5
N, FRAMES = 40, 64
FG, BG = (0.05, 5.0), (0.5, 10.0)  # (scalar colour, view depth)


class Scene:
    """Foreground where lo + s*t <= n.x*x + n.y*y < hi + s*t (n normalised)."""

    def __init__(self, normal, lo, hi, speed=0.0):
        length = math.hypot(*normal)
        self.n = (normal[0] / length, normal[1] / length)
        self.lo, self.hi, self.speed = lo, hi, speed

    def inside(self, x, y, t):
        d = self.n[0] * x + self.n[1] * y - self.speed * t
        return self.lo <= d < self.hi

    def surface(self, x, y, t):
        return FG if self.inside(x, y, t) else BG

    def velocity(self):
        return (self.n[0] * self.speed, self.n[1] * self.speed)

    def cover(self, x0, y0, t, steps=8):
        """Foreground fraction of the unit square at (x0, y0) by steps x steps samples."""
        hits = sum(self.inside(x0 + (a + 0.5) / steps, y0 + (b + 0.5) / steps, t)
                   for a in range(steps) for b in range(steps))
        return hits / (steps * steps)

    def ideal(self, t):
        return [self.cover(i % N, i // N, t) * FG[0] + (1 - self.cover(i % N, i // N, t)) * BG[0]
                for i in range(N * N)]


def bilinear(values, qx, qy):
    bx, by = math.floor(qx), math.floor(qy)
    fx, fy = qx - bx, qy - by
    acc = 0.0
    for dy, wy in ((0, 1 - fy), (1, fy)):
        for dx, wx in ((0, 1 - fx), (1, fx)):
            w = wx * wy
            if w > 0:
                acc += values[min(max(by + dy, 0), N - 1) * N + min(max(bx + dx, 0), N - 1)] * w
    return acc


def run(scene, creg):
    vel = scene.velocity()
    history = depth_prev = None
    shown, mismatch, admitted_total = [], 0, 0
    for t in range(FRAMES):
        j, jp = SAMPLES[t % 8], SAMPLES[(t - 1) % 8]
        cur, dep, mot = [], [], []
        for i in range(N * N):
            x, y = i % N, i // N
            col, depth = scene.surface(x + 0.5 - j[0], y + 0.5 - j[1], t)
            moving = depth == FG[1]
            cur.append(col)
            dep.append(depth)
            mot.append((jp[0] - j[0] - (vel[0] if moving else 0.0), jp[1] - j[1] - (vel[1] if moving else 0.0)))
        if history is None:
            out = [bilinear(cur, i % N + j[0], i // N + j[1]) for i in range(N * N)] if creg else list(cur)
        else:
            out = []
            dj = (jp[0] - j[0], jp[1] - j[1])
            for i in range(N * N):
                x, y = i % N, i // N
                c = cur[i]
                lo = hi = c
                near, far = math.inf, -math.inf
                m, closest = mot[i], dep[i]
                for ny in (y - 1, y, y + 1):
                    for nx in (x - 1, x, x + 1):
                        k = min(max(ny, 0), N - 1) * N + min(max(nx, 0), N - 1)
                        lo, hi = min(lo, cur[k]), max(hi, cur[k])
                        near, far = min(near, dep[k]), max(far, dep[k])
                        if dep[k] < closest:
                            m, closest = mot[k], dep[k]
                near -= max(ABS, REL * near)
                far += max(ABS, REL * far)
                ox = x + 0.5 + m[0] - (dj[0] if creg else 0.0)
                oy = y + 0.5 + m[1] - (dj[1] if creg else 0.0)
                alone = bilinear(cur, x + j[0], y + j[1]) if creg else c
                if not (0 <= ox < N and 0 <= oy < N):
                    out.append(alone)
                    continue
                qx, qy = ox - 0.5, oy - 0.5
                bx, by = math.floor(qx), math.floor(qy)
                fx, fy = qx - bx, qy - by
                acc = mass = 0.0
                for dy, wy in ((0, 1 - fy), (1, fy)):
                    for dx, wx in ((0, 1 - fx), (1, fx)):
                        w = wx * wy
                        tx, ty = bx + dx, by + dy
                        if not (w > 0 and 0 <= tx < N and 0 <= ty < N):
                            continue
                        k = ty * N + tx
                        if near <= depth_prev[k] <= far:
                            acc += history[k] * w
                            mass += w
                            admitted_total += 1
                            # Pairing indicator: does the raster depth sample's surface match
                            # the surface covering most of the colour texel's footprint?
                            sx, sy = (tx, ty) if creg else (tx - jp[0], ty - jp[1])
                            majority_fg = scene.cover(sx, sy, t - 1, steps=4) > 0.5
                            if majority_fg != (depth_prev[k] == FG[1]):
                                mismatch += 1
                if not mass > 0:
                    out.append(alone)
                    continue
                h = acc / mass
                reaction = min(max(abs(h - c) / THRESHOLD, 0.0), 1.0)
                weight = WEIGHT * (1 - STRENGTH * reaction)
                out.append(c + (min(max(h, lo), hi) - c) * weight)
        history, depth_prev = out, dep
        shown.append(list(out) if creg else [bilinear(out, i % N + j[0], i // N + j[1]) for i in range(N * N)])
    return shown, mismatch, admitted_total


def interior(i):
    x, y = i % N, i // N
    return 3 <= x < N - 3 and 3 <= y < N - 3


def evaluate(name, scene):
    contrast = abs(FG[0] - BG[0])
    rows = []
    for label, creg in (("baseline", False), ("C-reg", True)):
        shown, mismatch, admitted = run(scene, creg)
        last = range(FRAMES - 8, FRAMES)
        sq = cnt = flick = 0.0
        behind = 0.0
        for t in last:
            ideal = scene.ideal(t)
            prev_ideal = scene.ideal(t - 1)
            for i in range(N * N):
                if not interior(i):
                    continue
                e = shown[t][i] - ideal[i]
                sq += e * e
                cnt += 1
                flick = max(flick, abs((shown[t][i] - shown[t - 1][i]) - (ideal[i] - prev_ideal[i])))
                if scene.speed:
                    x, y = i % N + 0.5, i // N + 0.5
                    d = scene.n[0] * x + scene.n[1] * y - scene.speed * t
                    if d < scene.lo - 2.0 - 0.75:  # wholly two or more pixels behind the trailing edge
                        behind = max(behind, abs(e))
        mean = [sum(shown[t][i] for t in last) / 8 for i in range(N * N)]
        ideal0 = scene.ideal(FRAMES - 1)

        def detail(values):
            total = count = 0
            for i in range(N * N):
                if interior(i):
                    x, y = i % N, i // N
                    m = sum(values[(y + dy) * N + x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
                    total += abs(values[i] - m)
                    count += 1
            return total / count
        rows.append((label, math.sqrt(sq / cnt) / contrast, flick / contrast,
                     detail(mean) / detail(ideal0) if not scene.speed else None,
                     behind / contrast if scene.speed else None, mismatch / max(admitted, 1)))
    print("== " + name)
    for label, rms, flick, det, behind, mism in rows:
        parts = [f"rms {rms:.3f}", f"flicker max {flick:.3f}"]
        if det is not None:
            parts.append(f"detail {det:.2f}")
        if behind is not None:
            parts.append(f"residue >=2 px behind {behind:.3f}")
        parts.append(f"admitted taps whose raster depth disagrees with the texel's majority surface {100 * mism:.2f}%")
        print("  %-9s %s" % (label, " | ".join(parts)))


def main():
    print("Last 8 of %d frames on a %dx%d grid (3 px border excluded); fractions of contrast; detail is a "
          "ratio to the box ideal (8x8 supersampled)." % (FRAMES, N, N))
    evaluate("static diagonal step, slope 1/2 (26.6 deg)", Scene((1, -2), 0.0, 99.0))
    evaluate("static diagonal step, 45 deg", Scene((1, -1), -0.3, 99.0))
    evaluate("thin diagonal line, 1 px wide, slope 1/2", Scene((1, -2), -0.2, 0.8))
    evaluate("thin diagonal line, 1 px wide, 45 deg", Scene((1, -1), 0.1, 1.1))
    # Bars start off the grid and cross its centre during the last cycle.
    evaluate("diagonal bar moving 0.5 px/frame along its normal (disocclusion behind)",
             Scene((2, 1), -6.0, 0.0, speed=0.5))
    evaluate("diagonal bar moving 0.3 px/frame along its normal", Scene((1, 1), 6.0, 14.0, speed=0.3))


if __name__ == "__main__":
    main()
