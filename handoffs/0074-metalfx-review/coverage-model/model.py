#!/usr/bin/env python3
"""CPU coverage-history model for native-coverage-repair-addendum.md (handoff 0074).

Standard library only and deterministic. It models only the engine-owned
coverage history (alpha) proposed for the native composite, never MetalFX
colour. Conventions are the ones verified in 077f7c39 source:
- temporal/jitter.rs SAMPLES is an image shift in internal px, +right/down;
- upscaling/common.wgsl source_position(P) = source.xy + (u + J/W)*W, so with
  zero offsets the unjittered output point P is found at P/r + J (source px);
- a texel t at phase k therefore samples the scene point (t + 0.5 - J)*r
  (output px); bilinear taps are taken around q = source_position - 0.5;
- motion/shader.wgsl writes previous_uv - current_uv with both raster jitters,
  and metalfx/prepare.wgsl adds J_cur/W - J_prev/W_prev, so the packed motion
  is the true current->previous displacement; here it is given in output px.

Rules compared (alpha only):
- R0: native-coverage-repair.md as committed in 9998e60c: 3x3 coverage class,
  history reprojected with the nearest-depth motion of the 3x3, clamped every
  frame to the current 3x3 coverage range.
- R1a: owner-depth admission. Candidate owner = primary tap if covered, else
  the nearest-depth covered tap of the 2x2, else of the 3x3. History is
  gathered per bilinear tap and admitted only where the stored owner depth is
  compatible with the candidate (REVIEW tolerances 0.05 relative, 0.02
  absolute). No per-frame coverage clamp. With no candidate, only clear
  history is admitted.
- R1b: R1a plus miss persistence: with no candidate, the pixel's own history
  is kept for at most MISS_LIMIT consecutive frames.

References are matched resets: the same rule on the scene without the departed
owner, with history reset in the same frame, so only stale history differs.

  python3 -I -B model.py [--write NEW_RESULTS.json]
"""

import json
import math
import sys

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
H = 0.9
REL, ABS = 0.05, 0.02
MISS_LIMIT = 5


def compatible(a, b):
    return a > 0 and b > 0 and abs(a - b) <= max(ABS, REL * b)


def footprint(P, r, k):
    """Bilinear taps (texel, weight) and the 3x3 texels around the tap nearest q."""
    jx, jy = SAMPLES[k % 8]
    qx, qy = P[0] / r + jx - 0.5, P[1] / r + jy - 0.5
    bx, by = math.floor(qx), math.floor(qy)
    fx, fy = qx - bx, qy - by
    taps = [((bx, by), (1 - fx) * (1 - fy)), ((bx + 1, by), fx * (1 - fy)),
            ((bx, by + 1), (1 - fx) * fy), ((bx + 1, by + 1), fx * fy)]
    cx, cy = math.floor(qx + 0.5), math.floor(qy + 0.5)
    return taps, [(cx + dx, cy + dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1)]


def point(t, r, k):
    jx, jy = SAMPLES[k % 8]
    return (t[0] + 0.5 - jx) * r, (t[1] + 0.5 - jy) * r


# ------------------------------------------------------------------ part A: EMA versus uniform 8-phase coverage

def ema_case(alphas, h=H):
    mean = sum(alphas) / 8
    weights = [(1 - h) * h ** m / (1 - h ** 8) for m in range(8)]
    steady = [sum(weights[m] * alphas[(j - m) % 8] for m in range(8)) for j in range(8)]
    trajectory, value = [], 0.0
    for n in range(16):  # reset at frame 1 (phase 0): history = current
        value = alphas[0] if n == 0 else (1 - h) * alphas[n % 8] + h * value
        trajectory.append(value)
    cumulative, value = [], 0.0
    for n in range(16):  # cumulative mean up to 8 frames, then weight 7/8
        value += (alphas[n % 8] - value) / min(n + 1, 8)
        cumulative.append(value)
    w8 = [(1 / 8) * (7 / 8) ** m / (1 - (7 / 8) ** 8) for m in range(8)]
    steady8 = [sum(w8[m] * alphas[(j - m) % 8] for m in range(8)) for j in range(8)]
    return {"mean": mean, "steady_dev": max(abs(s - mean) for s in steady),
            "steady_p2p": max(steady) - min(steady), "f8_err": abs(trajectory[7] - mean),
            "f16_err": abs(trajectory[15] - mean), "trajectory": trajectory,
            "cumulative_f8_err": abs(cumulative[7] - mean), "cumulative_f16_err": abs(cumulative[15] - mean),
            "h78_steady_dev": max(abs(s - mean) for s in steady8)}


def weight_table(P=(0.5, 0.5)):
    """Max steady-state deviation from the uniform 8-phase mean and max frame-16
    error after a reset, for several history weights, over every 16/16 edge."""
    rows = {}
    for h in (0.875, 0.9, 0.95, 0.98):
        dev = f16 = 0.0
        for step in range(24):
            theta = math.radians(7.5 * step)
            n = (math.cos(theta), math.sin(theta))
            centre = n[0] * P[0] + n[1] * P[1]
            for i in range(-80, 81):
                c = centre + i / 32
                alphas = []
                for k in range(8):
                    taps, _ = footprint(P, 1.0, k)
                    alphas.append(sum(w for t, w in taps if n[0] * point(t, 1.0, k)[0]
                                      + n[1] * point(t, 1.0, k)[1] < c))
                if max(alphas) - min(alphas) < 1e-12:
                    continue
                case = ema_case(alphas, h)
                dev, f16 = max(dev, case["steady_dev"]), max(f16, case["f16_err"])
        rows[str(h)] = {"max_steady_dev": round(dev, 4), "max_f16_err": round(f16, 4)}
    return rows


def part_a():
    P, out = (0.5, 0.5), {}
    for name, r in (("16/16", 1.0), ("12/16", 4 / 3), ("8/16", 2.0)):
        worst, cases = None, 0
        keys = ("steady_dev", "steady_p2p", "f8_err", "f16_err", "cumulative_f8_err",
                "cumulative_f16_err", "h78_steady_dev")
        agg = dict.fromkeys(keys, 0.0)
        for step in range(24):  # edge normals every 7.5 degrees
            theta = math.radians(7.5 * step)
            n = (math.cos(theta), math.sin(theta))
            centre = n[0] * P[0] + n[1] * P[1]
            for i in range(-80, 81):  # edge offsets every 1/32 output px over +-2.5 r
                c = centre + i / 32 * r
                alphas = []
                for k in range(8):
                    taps, _ = footprint(P, r, k)
                    alphas.append(sum(w for t, w in taps if n[0] * point(t, r, k)[0]
                                      + n[1] * point(t, r, k)[1] < c))
                if max(alphas) - min(alphas) < 1e-12:
                    continue
                cases += 1
                case = ema_case(alphas)
                for key in keys:
                    agg[key] = max(agg[key], case[key])
                if worst is None or case["f16_err"] > worst["f16_err"]:
                    worst = {**case, "theta_deg": 7.5 * step, "offset_px": i / 32 * r, "alphas": alphas}
        out[name] = {"edge_cases": cases, **{f"max_{k}": round(v, 4) for k, v in agg.items()},
                     "worst_f16_case": {
                         "edge_normal_deg": worst["theta_deg"], "offset_px": round(worst["offset_px"], 4),
                         "per_phase_alpha": [round(a, 4) for a in worst["alphas"]],
                         "uniform_8_phase_mean": round(worst["mean"], 4),
                         "ema_0p9_trajectory_f1_to_f16": [round(a, 4) for a in worst["trajectory"]]}}
    return out


def miss_runs():
    """Longest run of consecutive phases whose x-samples all miss a static vertical
    feature [c, c + w), worst over offsets c (output px)."""
    out = {}
    for name, r in (("16/16", 1.0), ("12/16", 4 / 3), ("8/16", 2.0)):
        row = {}
        for w in (0.25, 0.4, 0.5, 0.8, 1.0, 1.6):
            worst = 0
            for i in range(0, 256):
                c = i / 128 * r
                hit = [any(c <= (t + 0.5 - SAMPLES[k][0]) * r < c + w for t in range(-4, 8))
                       for k in range(8)]
                if not any(hit):
                    worst = 8
                    break
                run = best = 0
                for k in range(16):
                    run = 0 if hit[k % 8] else run + 1
                    best = max(best, run)
                worst = max(worst, best)
            row[f"{w}px"] = worst
        out[name] = row
    return out


# ------------------------------------------------------------------ part B: dynamic strip scenes

class Bar:
    """A vertical bar [x0, x1) shifted by velocity * frames moved (output px)."""

    def __init__(self, x0, x1, depth, velocity=0.0, start=0, stop=10 ** 9):
        self.x0, self.x1, self.depth, self.velocity = x0, x1, depth, velocity
        self.start, self.stop = start, stop

    def offset(self, frame):
        return self.velocity * (min(max(frame, self.start), self.stop) - self.start)

    def motion(self, frame):  # current -> previous, output px
        return -self.velocity if self.start < frame <= self.stop else 0.0

    def covers(self, x, frame):
        o = self.offset(frame)
        return self.x0 + o <= x < self.x1 + o


def sample(bars, t, r, k, frame):
    """(coverage, depth, motion) of texel t: the nearest covering bar, or clear."""
    x, _ = point(t, r, k)
    hits = [b for b in bars if b.covers(x, frame)]
    if not hits:
        return 0.0, 0.0, None
    front = min(hits, key=lambda b: b.depth)
    return 1.0, front.depth, front.motion(frame)


def gather(history, owners, x, admit):
    """Bilinear history at output position x over one row, admitted taps only."""
    q = x - 0.5
    base = math.floor(q)
    total = value = 0.0
    for i, w in ((base, 1 - (q - base)), (base + 1, q - base)):
        if 0 <= i < len(history) and w > 0 and admit(owners[i]):
            total += w
            value += w * history[i]
    return value / total if total > 0 else None


def run(bars, rule, r, width, frames, reset_at=None):
    history = owners = misses = None
    trace = []
    for frame in range(frames):
        if frame == reset_at:
            history = owners = misses = None
        k = frame % 8
        alpha, owner, missed = [], [], []
        for px in range(width):
            P = (px + 0.5, 0.5)
            taps, near = footprint(P, r, k)
            cur = [(sample(bars, t, r, k, frame), w) for t, w in taps]
            a_cur = sum(s[0] * w for s, w in cur)
            hood = [sample(bars, t, r, k, frame) for t in near]
            lo, hi = min(s[0] for s in hood), max(s[0] for s in hood)
            covered = [s for s in hood if s[0] > 0]
            front = min(covered, key=lambda s: s[1]) if covered else None
            miss = 0
            if rule == "R0":
                prev = None
                if history is not None and front is not None and front[2] is not None:
                    prev = gather(history, owners, P[0] + front[2], lambda o: True)
                if lo == hi:
                    a = lo
                elif prev is None:
                    a = a_cur
                else:
                    a = min(max((1 - H) * a_cur + H * min(max(prev, lo), hi), lo), hi)
                own = front[1] if front else 0.0
            else:  # R1a, R1b
                primary = max(cur, key=lambda c: c[1])[0]
                in_f = [s for s, _ in cur if s[0] > 0]
                cand = primary if primary[0] > 0 else min(in_f, key=lambda s: s[1]) if in_f else front
                prev = None
                if history is not None and cand is not None:
                    prev = gather(history, owners, P[0] + (cand[2] or 0.0), lambda o: compatible(o, cand[1]))
                elif history is not None:
                    prev = gather(history, owners, P[0], lambda o: o == 0.0)
                    if rule == "R1b" and owners[px] > 0 and misses[px] < MISS_LIMIT:
                        prev, miss = history[px], misses[px] + 1
                if lo == 1:
                    a = 1.0
                else:
                    a = a_cur if prev is None else (1 - H) * a_cur + H * prev
                own = cand[1] if cand is not None else (owners[px] if miss else 0.0)
            alpha.append(a)
            owner.append(own)
            missed.append(miss)
        history, owners, misses = alpha, owner, missed
        trace.append(alpha)
    return trace


def compare(bars, reference, r, width, frames, pixels, window, reset_at=None):
    """Per frame in window: max (alpha - reference) and max |alpha - reference|."""
    out = {}
    for rule in ("R0", "R1a", "R1b"):
        a = run(bars, rule, r, width, frames)
        b = run(reference, rule, r, width, frames, reset_at)
        out[rule] = {"frames": list(window),
                     "max_excess_opacity": [round(max(a[f][p] - b[f][p] for p in pixels), 4) for f in window],
                     "max_abs_difference": [round(max(abs(a[f][p] - b[f][p]) for p in pixels), 4) for f in window]}
    return out


def part_b():
    res = {}
    for name, r in (("16/16", 1.0), ("8/16", 2.0)):
        # B1 root's case for coverage: nearer bar A (depth 3) covers [12, 22), static
        # for 24 frames, then leaves left at 6 px/frame. Static bar B (depth 5,
        # x >= 20) stays: its edge is a NEW nearby edge for the vacated pixels.
        res[f"B1_vacated_with_new_nearby_edge_{name}"] = compare(
            [Bar(12, 22, 3.0, -6.0, start=24, stop=40), Bar(20, 60, 5.0)], [Bar(20, 60, 5.0)],
            r, 40, 40, range(14, 22), range(26, 40, 2), reset_at=25)
        # B1c the same without B: vacated pixels become fully clear.
        res[f"B1c_vacated_fully_clear_{name}"] = compare(
            [Bar(12, 22, 3.0, -6.0, start=24, stop=40)], [], r, 40, 40, range(14, 22),
            range(26, 40, 2), reset_at=25)
        # B2 dilated motion from another owner: static C (depth 5, x >= 20); nearer D
        # (depth 3) approaches at 0.5 px/frame and stops at 18.5. After the stop both
        # scenes are identical, so the difference at C's edge pixels 19-21 against D
        # parked at 18.5 from the start is history contaminated during the approach.
        res[f"B2_dilation_picks_other_owner_{name}"] = compare(
            [Bar(20, 60, 5.0), Bar(8, 14.5, 3.0, 0.5, start=24, stop=32)],
            [Bar(20, 60, 5.0), Bar(12, 18.5, 3.0)], r, 40, 48, range(19, 22), range(33, 48, 2))
    # B3 0073's V3 repeated: a static 0.8 px wire over clear at 8/16. Alpha at the
    # wire's pixel over the last full cycle.
    for rule in ("R0", "R1a", "R1b"):
        trace = run([Bar(20.1, 20.9, 5.0)], rule, 2.0, 40, 32)
        res[f"B3_static_wire_0p8px_8/16_{rule}"] = [round(trace[f][20], 4) for f in range(24, 32)]
    # B4 residual risk: the departing surface is within the depth tolerance of B.
    res["B4_departing_owner_within_tolerance_16/16"] = compare(
        [Bar(12, 22, 5.1, -6.0, start=24, stop=40), Bar(20, 60, 5.0)], [Bar(20, 60, 5.0)],
        1.0, 40, 40, range(14, 22), range(26, 40, 2), reset_at=25)
    return res


def main():
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--write"):
        raise SystemExit(__doc__)
    result = {"model": "native coverage history, alpha only", "history_weight": H,
              "tolerances": {"relative": REL, "absolute": ABS}, "miss_limit_R1b": MISS_LIMIT,
              "part_a_ema_vs_uniform_8_phase": part_a(),
              "part_a_history_weight_tradeoff_16_16": weight_table(),
              "consecutive_miss_runs_static_vertical_feature": miss_runs(),
              "part_b_dynamic": part_b()}
    text = json.dumps(result, indent=1) + "\n"
    if args:
        with open(args[1], "x") as out:
            out.write(text)
    print(text, end="")


if __name__ == "__main__":
    main()
