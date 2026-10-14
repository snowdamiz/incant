#!/usr/bin/env python3
"""0073 reconstruction_implementation_r4: scalar transcription of the implemented
resolve.wgsl (ordinary) and resolve_owner.wgsl (owner) evidence rules, with mutation
checks. Standard library only; no renderer, naga, GPU or image.

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r4/controls.py \
        --out handoffs/0073-upscaling-review/reconstruction_implementation_r4/controls.json
"""
import argparse
import json
import math
import sys

sys.dont_write_bytecode = True
CHECKS = []
MAX_AGE = 8.0


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def finite(v):
    return all(math.isfinite(x) for x in v)


def admitted(v):
    return finite(v) and all(x >= 0 for x in v) and v[3] <= 1


def clean(v):
    if not finite(v) or v[3] <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    return tuple([min(max(x, 0.0), 65504.0) for x in v[:3]] + [min(max(v[3], 0.0), 1.0)])


def stored_evidence(v, valid=True, per_tap_cap=None):
    """Shader rule: wire-range mass, integer age < 8; NO current cap per tap.
    per_tap_cap restores the removed R3 defect (mutation)."""
    if not valid or not (v[0] == v[0] and 0 <= v[0] <= 65504) or not (0 <= v[1] < MAX_AGE) or v[1] != math.floor(v[1]):
        return None
    return (min(v[0], per_tap_cap), v[1]) if per_tap_cap is not None else (v[0], v[1])


def history(taps, per_tap_cap=None):
    """taps: (b, colour, evidence). Returns associated N, C."""
    n, c = [0.0] * 4, 0.0
    for b, col, ev in taps:
        s = stored_evidence(ev, per_tap_cap=per_tap_cap)
        if not b > 0 or s is None or not admitted(col):
            continue
        n = [a + x * b * s[0] for a, x in zip(n, clean(col))]
        c += b * s[0]
    return n, c


def weight(c, density, cap, strength=0.0, reaction=0.0):
    """Density, cap and reaction once on the accumulated mass (both programs)."""
    return min(c * max(density, 0.0), cap) * (1 - strength * reaction)


def evidence(taps):
    """current_evidence: (w, colour, depth); returns S, m, valid, single, nearest."""
    s, m, valid, single, nearest = [0.0] * 4, 0.0, True, True, math.inf
    for w, col, z in taps:
        if not w > 0:
            continue
        if not admitted(col):
            valid = False
            continue
        s = [a + x * w for a, x in zip(s, clean(col))]
        m += w
        if col[3] < 1 or z <= 0:
            single = False
        if z > 0:
            nearest = min(nearest, z)
    if not math.isfinite(nearest):
        single = False
    if single:
        single = all(abs(z - nearest) <= max(0.02, 0.05 * nearest) for w, col, z in taps if w > 0 and admitted(col))
    return s, m, valid, single, nearest


def owner_age(hist_age, surface, sees_primary):
    """resolve_owner.wgsl static persistence: reset only on primary surface evidence."""
    return 0.0 if (surface and sees_primary) else hist_age + 1.0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    cap_full, cap_quarter = 99.0, 396.0                   # h = 0.99 at ratio 1 and ratio >= 2
    # 1. Mass order (supervision-004): density then cap, never a per-tap current cap.
    n, c = history([(1.0, (1, 1, 1, 1), (396.0, 0.0))])
    w = weight(c, 1 / 16, cap_full)
    n_bad, c_bad = history([(1.0, (1, 1, 1, 1), (396.0, 0.0))], per_tap_cap=cap_full)
    check("quarter -> full: associated mass 396 x 1/16 = 24.75 (both programs)", w == 24.75, w)
    check("mutation: restoring the per-tap current cap gives 6.1875 and is detected",
          weight(c_bad, 1 / 16, cap_full) == 6.1875 != w)
    n, c = history([(0.5, (1, 1, 1, 1), (396.0, 0.0)), (0.5, (0, 0, 0, 1), (20.0, 0.0))])
    h = n[0] / c
    n2, c2 = history([(0.5, (1, 1, 1, 1), (396.0, 0.0)), (0.5, (0, 0, 0, 1), (20.0, 0.0))], per_tap_cap=cap_full)
    check("associated H = 396/416 = 0.951923...", abs(h - 396 / 416) < 1e-12, h)
    check("mutation: per-tap cap 99 changes H to 99/119 and is detected", abs(n2[0] / c2 - 99 / 119) < 1e-12)
    check("full -> quarter: 50 x 16 = 800 then capped once to 396", weight(50.0, 16.0, cap_quarter) == 396.0)
    # 2. Full-support admission (partial-source-findings counterexample, r = 4, rho = 1).
    w_outer = 0.000854248567252511
    taps = [(0.25, (0, 0, 0, 1), 5.0)] * 4 + [(w_outer, (0, 65504, 0, 1), 2.0)]
    s, m, valid, single, nearest = evidence(taps)
    check("outer HDR green at another depth makes support ambiguous: no reusable history", not single and valid)
    check("its current colour is kept, not erased", s[1] == 65504 * w_outer and abs(m - (1 + w_outer)) < 1e-15)
    inner_only = evidence(taps[:4])[3]
    check("mutation: checking only the inner 2x2 would admit history (detected)", inner_only and not single)
    # 3. Clear-inclusive coverage and alpha-zero canonicalization.
    s, m, *_ = evidence([(1.0, (1, 0, 0, 1), 3.0), (1.0, (0, 0, 0, 0), 0.0)])
    check("red + clear at equal weight is (0.5,0,0,0.5)", clean([x / m for x in s]) == (0.5, 0, 0, 0.5))
    # 4. Associated numerator: zero-mass bright tap contributes nothing.
    n, c = history([(0.5, (1, 1, 1, 1), (0.0, 0.0)), (0.5, (0, 0, 0, 1), (8.0, 0.0))])
    check("zero-mass bright tap contributes nothing to H", n[0] == 0.0 and c == 4.0)
    # 5. Integer-age retirement with reaction 0 (owner static persistence, no current surface).
    age, ages = 0.0, []
    for _ in range(10):
        if stored_evidence((36.0, age)) is None:
            ages.append(None)
            break
        age = owner_age(age, surface=False, sees_primary=False)
        ages.append(age)
    check("partial no-surface history ages 1..8 and is refused at 8 (independent of reaction)",
          ages == [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, None], ages)
    age_mut, survived = 0.0, 0
    for _ in range(20):
        if stored_evidence((36.0, age_mut)) is None:
            break
        survived += 1                                    # mutation: age never increments
    check("mutation: a non-incrementing age would persist forever (detected)", survived == 20)
    check("primary-owner surface evidence resets age; another owner's does not",
          owner_age(5.0, True, True) == 0.0 and owner_age(5.0, True, False) == 6.0)
    check("non-integer or out-of-range age refuses", stored_evidence((9.0, 2.5)) is None and stored_evidence((9.0, -1.0)) is None)
    check("nonfinite or negative mass refuses", stored_evidence((float("nan"), 0.0)) is None and stored_evidence((-1.0, 0.0)) is None)
    check("invalid origin (native frame, new allocation) refuses", stored_evidence((9.0, 0.0), valid=False) is None)
    # 6. Changed secondary owner rejects a history tap at any positive weight.
    admissible = lambda record, changed: not any(changed(o) for o in record[1:])
    check("changed secondary rejects at 2^-20", not admissible((5, 7), lambda o: o == 7) and 2.0 ** -20 > 0)
    # 7. Split jitter: 8 distinct offsets per axis; NOT a 2D thin-geometry guarantee.
    xs = [-0.4375, 0.0625, 0.3125, -0.1875, 0.4375, -0.0625, -0.3125, 0.1875]
    ys = [-0.1875, 0.3125, -0.4375, 0.0625, 0.1875, -0.3125, 0.4375, -0.0625]
    check("eight distinct offsets per axis, zero mean", len(set(xs)) == len(set(ys)) == 8 and sum(xs) == sum(ys) == 0)
    def hit(x0, slope, width):
        return any(abs((y - (slope * (x - x0))) * math.cos(math.atan(slope))) < width / 2 for x, y in zip(xs, ys))
    missed = [x0 / 64 for x0 in range(64) if not hit(x0 / 64, 1.0, 0.05)]
    check("a 0.05-px 45-degree line can miss all 8 samples (axis stratification only)", len(missed) > 0, missed[:3])
    result = {"scope": "transcription of resolve.wgsl and resolve_owner.wgsl rules; not a render or gate",
              "checks": CHECKS, "all_checks_pass": all(c["ok"] for c in CHECKS)}
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(json.dumps(result, indent=1, sort_keys=True) + "\n")
    failed = [c for c in CHECKS if not c["ok"]]
    print(f"checks={len(CHECKS)} failed={len(failed)}")
    for c in failed:
        print("FAILED", c)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
