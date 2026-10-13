#!/usr/bin/env python3
"""Bounded 2-D CPU derivation for the C-reg comparison's remaining failures. Standard
library only, deterministic, foreground, about 15 s. A derivation, not a measurement.

It reuses fixed-sharpness/model2d.py's shader rules unchanged (temporal/resolve.wgsl
and copy.wgsl at e23776df and a143b5c0: jitter.rs SAMPLES, raster motion j_prev -
j_curr - v, 3x3 neighbourhood, view-depth range admission 0.05/0.02, closest-depth
motion with centre priority, positive-tap renormalised bilinear history, clamp,
reactive weight 0.9/4.0/0.5; baseline history on the jittered grid plus a bilinear
presentation un-jitter; C-reg history on the unjittered grid, refusal and reset use
the registered current). Candidate changes to C-reg, combinable with '+':

  ramp      history weight min(0.9, n / (n + 1)) for the n-th accumulation after a
            reset (a per-frame uniform value; shader unchanged)
  regblend  the registered current (the bilinear estimate the refusal path already
            uses) replaces the raw jittered centre sample in the blend and reaction
  depth2x2  a history tap is admitted when any of the four previous raster depth
            samples nearest its unjittered centre is in range (the 2x2 support the
            baseline's fractional reprojection gets implicitly)
  oracle    model-only attribution, not a candidate: history depth paired on the
            colour grid (unjittered previous depth)

Path counts (accumulated, clamp active, refused) are recorded for watched pixels.

  python3 -I -B repair_model.py
"""

import math

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
WEIGHT, REL, ABS, THRESHOLD, STRENGTH = 0.9, 0.05, 0.02, 4.0, 0.5
N = 40
FG, BG = (0.05, 5.0), (0.5, 10.0)  # (scalar colour, view depth); FG is near and dark
CONTRAST = abs(FG[0] - BG[0])


class Band:
    """Foreground where lo + s*t <= n.(x, y) < hi + s*t (static unless speed)."""

    def __init__(self, normal, lo, hi, speed=0.0):
        length = math.hypot(*normal)
        self.n = (normal[0] / length, normal[1] / length)
        self.lo, self.hi, self.speed = lo, hi, speed

    def inside(self, x, y, t):
        d = self.n[0] * x + self.n[1] * y - self.speed * t
        return self.lo <= d < self.hi

    def velocity(self):
        return (self.n[0] * self.speed, self.n[1] * self.speed)


class Posts:
    """Static vertical dark posts with background gaps: x in [a, b) for each span."""

    def __init__(self, spans):
        self.spans = spans

    def inside(self, x, y, t):
        return 6 <= y < 34 and any(a <= x < b for a, b in self.spans)

    def velocity(self):
        return (0.0, 0.0)


class Box:
    """A foreground rectangle moving along +x at `speed` px/frame."""

    def __init__(self, x0, width, y0, y1, speed):
        self.x0, self.width, self.y0, self.y1, self.speed = x0, width, y0, y1, speed

    def left(self, t):
        return self.x0 + self.speed * t

    def inside(self, x, y, t):
        return self.left(t) <= x < self.left(t) + self.width and self.y0 <= y < self.y1

    def velocity(self):
        return (self.speed, 0.0)


def surface(scene, x, y, t):
    return FG if scene.inside(x, y, t) else BG


def cover(scene, x0, y0, t, steps=8):
    hits = sum(scene.inside(x0 + (a + 0.5) / steps, y0 + (b + 0.5) / steps, t)
               for a in range(steps) for b in range(steps))
    return hits / (steps * steps)


def ideal(scene, t):
    out = []
    for i in range(N * N):
        c = cover(scene, i % N, i // N, t)
        out.append(c * FG[0] + (1 - c) * BG[0])
    return out


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


def run(scene, variant, frames, watch=()):
    """Returns the shown frames and, for watched pixels, path counts over the run."""
    creg = variant != "baseline"
    flags = set(variant.split("+")[1:])
    vel = scene.velocity()
    history = depth_prev = depth_prev_grid = None
    shown = []
    paths = {i: {"accumulated": 0, "clamped": 0, "refused": 0} for i in watch}
    for t in range(frames):
        j, jp = SAMPLES[t % 8], SAMPLES[(t - 1) % 8]
        cur, dep, dep_grid, mot = [], [], [], []
        for i in range(N * N):
            x, y = i % N, i // N
            col, depth = surface(scene, x + 0.5 - j[0], y + 0.5 - j[1], t)
            moving = depth == FG[1]
            cur.append(col)
            dep.append(depth)
            dep_grid.append(surface(scene, x + 0.5, y + 0.5, t)[1])
            mot.append((jp[0] - j[0] - (vel[0] if moving else 0.0), jp[1] - j[1] - (vel[1] if moving else 0.0)))
        if history is None:
            out = [bilinear(cur, i % N + j[0], i // N + j[1]) for i in range(N * N)] if creg else list(cur)
        else:
            weight_base = min(WEIGHT, t / (t + 1)) if "ramp" in flags else WEIGHT
            out = []
            dj = (jp[0] - j[0], jp[1] - j[1])
            pairing = depth_prev_grid if "oracle" in flags else depth_prev
            sx, sy = (1 if jp[0] > 0 else -1), (1 if jp[1] > 0 else -1)
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
                        if "depth2x2" in flags:
                            support = [min(max(ry, 0), N - 1) * N + min(max(rx, 0), N - 1)
                                       for ry in (ty, ty + sy) for rx in (tx, tx + sx)]
                        else:
                            support = [ty * N + tx]
                        if any(near <= pairing[r] <= far for r in support):
                            acc += history[ty * N + tx] * w
                            mass += w
                if not mass > 0:
                    if i in paths:
                        paths[i]["refused"] += 1
                    out.append(alone)
                    continue
                h = acc / mass
                sample = alone if "regblend" in flags else c
                reaction = min(max(abs(h - sample) / THRESHOLD, 0.0), 1.0)
                weight = weight_base * (1 - STRENGTH * reaction)
                clipped = min(max(h, lo), hi)
                if i in paths:
                    paths[i]["accumulated"] += 1
                    paths[i]["clamped"] += int(clipped != h)
                out.append(sample + (clipped - sample) * weight)
        history, depth_prev, depth_prev_grid = out, dep, dep_grid
        shown.append(list(out) if creg else [bilinear(out, i % N + j[0], i // N + j[1]) for i in range(N * N)])
    return shown, paths


def interior(i, border=3):
    x, y = i % N, i // N
    return border <= x < N - border and border <= y < N - border


def flicker(scene, shown, last):
    worst = 0.0
    for t in last:
        a, b = ideal(scene, t), ideal(scene, t - 1)
        for i in range(N * N):
            if interior(i):
                worst = max(worst, abs((shown[t][i] - shown[t - 1][i]) - (a[i] - b[i])))
    return worst / CONTRAST


def detail_ratio(scene, shown, last):
    mean = [sum(shown[t][i] for t in last) / len(last) for i in range(N * N)]
    ref = ideal(scene, last[-1])

    def detail(values):
        total = count = 0
        for i in range(N * N):
            if interior(i):
                x, y = i % N, i // N
                m = sum(values[(y + dy) * N + x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
                total += abs(values[i] - m)
                count += 1
        return total / count
    return detail(mean) / detail(ref)


VARIANTS = ("baseline", "creg", "creg+ramp", "creg+regblend", "creg+depth2x2", "creg+regblend+depth2x2")


def static_scenes():
    frames = 32
    last = list(range(frames - 8, frames))
    print("== A/B. static diagonal step (slope 1/2) and thin 45-degree line: last 8 of 32 frames;"
          " flicker = largest per-frame change error, fraction of contrast; detail = ratio to the box ideal")
    for name, scene in (("diagonal step", Band((1, -2), 0.0, 99.0)), ("thin 45-degree line", Band((1, -1), 0.1, 1.1))):
        for variant in VARIANTS:
            shown, _ = run(scene, variant, frames)
            print(f"  {name:20s} {variant:23s} flicker {flicker(scene, shown, last):.3f} | "
                  f"detail {detail_ratio(scene, shown, last):.2f}")


def gap_scene():
    frames = 32
    last = list(range(frames - 8, frames))
    # Dark posts with a 0.7 px and a 0.5 px background gap (the far posts' sub-pixel gaps).
    scene = Posts([(10.0, 19.35), (20.05, 24.3), (24.8, 30.0)])
    rows = range(10, 30)
    watch = [y * N + x for y in rows for x in (19, 24)]
    print("== C. static dark posts with a 0.7 px gap (column 19) and a 0.5 px gap (column 24) over a far"
          " background: last 8 of 32 frames; largest per-frame change at the gap columns (fraction of"
          " contrast) and the resolve path at those pixels over frames 2-32")
    for variant in VARIANTS:
        shown, paths = run(scene, variant, frames, watch)
        for column in (19, 24):
            ids = [y * N + column for y in rows]
            change = max(abs(shown[t][i] - shown[t - 1][i]) for t in last for i in ids) / CONTRAST
            mean = sum(shown[t][i] for t in last for i in ids) / (len(last) * len(ids))
            coverage = 1 - (mean - FG[0]) / (BG[0] - FG[0])
            totals = {k: sum(paths[i][k] for i in ids) for k in ("accumulated", "clamped", "refused")}
            visits = totals["accumulated"] + totals["refused"]
            print(f"  gap column {column} {variant:23s} max change {change:.3f} | mean dark coverage "
                  f"{coverage:.2f} (ideal {cover(scene, column, 20, 0):.2f}) | refused "
                  f"{100 * totals['refused'] / visits:.0f}% | clamp active "
                  f"{100 * totals['clamped'] / max(totals['accumulated'], 1):.0f}% of accumulations")


def reset_scene():
    # Pixel-aligned dark half-plane x < 20 (an orthographic texture edge) after a reset.
    scene = Band((-1, 0), -20.0, 99.0)
    print("== D. pixel-aligned vertical edge after a history reset: edge position bias (px, + = dark"
          " side extends right) after 1, 6 and 16 frames")
    for variant in VARIANTS:
        shown, _ = run(scene, variant, 16)
        biases = []
        for t in (0, 5, 15):
            total = 0.0
            for y in range(5, 35):
                total += sum((BG[0] - shown[t][y * N + x]) / CONTRAST for x in range(5, 35)) - 15
            biases.append(total / 30)
        print(f"  {variant:23s} f1 {biases[0]:+.3f} | f6 {biases[1]:+.3f} | f16 {biases[2]:+.3f}")


def moving_scenes():
    print("== E. dark box moving right over the far background: darkness beyond the box at the last"
          " frame, shown minus box ideal as a fraction of contrast; trailing = mean over rows 14-25"
          " left of the box, top control = mean over the box's columns above it")
    for speed, frames, x0 in ((2.3, 16, -10.0), (6.3, 6, -7.0)):
        scene = Box(x0, 10.0, 12.4, 28.4, speed)
        t = frames - 1
        left = scene.left(t)
        ref = ideal(scene, t)
        for variant in ("baseline", "creg", "creg+oracle") + VARIANTS[3:]:
            shown, _ = run(scene, variant, frames)

            def band_trailing(k):
                x = math.floor(left) - k
                return sum((ref[y * N + x] - shown[t][y * N + x]) / CONTRAST for y in range(14, 26)) / 12

            def band_top(k):
                y = 12 - k
                xs = range(math.ceil(left) + 1, math.floor(left + scene.width) - 1)
                return sum((ref[y * N + x] - shown[t][y * N + x]) / CONTRAST for x in xs) / len(xs)
            print(f"  {speed} px/frame {variant:23s} trailing 1 px {band_trailing(1):+.3f}, 2 px "
                  f"{band_trailing(2):+.3f}, 3 px {band_trailing(3):+.3f} | top 1 px {band_top(1):+.3f}, "
                  f"2 px {band_top(2):+.3f}")
    # model2d's disocclusion case: residue wholly 2 px or more behind a moving diagonal bar.
    scene = Band((2, 1), -6.0, 0.0, speed=0.5)
    frames = 64
    print("== F. diagonal bar moving 0.5 px/frame along its normal (model2d's disocclusion case): largest"
          " residue wholly 2 px or more behind the bar over the last 8 of 64 frames, fraction of contrast")
    for variant in VARIANTS:
        shown, _ = run(scene, variant, frames)
        behind = 0.0
        for t in range(frames - 8, frames):
            ref = ideal(scene, t)
            for i in range(N * N):
                if interior(i):
                    x, y = i % N + 0.5, i // N + 0.5
                    if scene.n[0] * x + scene.n[1] * y - scene.speed * t < scene.lo - 2.0 - 0.75:
                        behind = max(behind, abs(shown[t][i] - ref[i]))
        print(f"  {variant:23s} residue {behind / CONTRAST:.3f}")


def main():
    print(f"{N}x{N} grid; 3 px border excluded where noted; contrast {CONTRAST}; colours/depths FG {FG}, BG {BG}.")
    static_scenes()
    gap_scene()
    reset_scene()
    moving_scenes()


if __name__ == "__main__":
    main()
