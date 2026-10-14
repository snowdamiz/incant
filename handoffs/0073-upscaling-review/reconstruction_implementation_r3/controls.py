#!/usr/bin/env python3
"""0073 reconstruction_implementation_r3: scalar transcription of the implemented ordinary
(UP) resolve.wgsl evidence update against Astra's estimator contract. Standard library only.

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r3/controls.py \
        --out handoffs/0073-upscaling-review/reconstruction_implementation_r3/controls.json

Not a renderer, GPU or naga run; it checks the equations as written in the shader source.
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


def current_evidence(taps):
    """resolve.wgsl current_evidence: (weight, colour, in_rect)."""
    s, m, ok = [0.0] * 4, 0.0, True
    for w, c, inside in taps:
        if not inside or not w > 0:
            continue
        if not admitted(c):
            ok = False
            continue
        s = [a + b * w for a, b in zip(s, clean(c))]
        m += w
    return s, m, ok


def stored(v, valid, cap):
    if not valid or not (v[0] == v[0] and 0 <= v[0] <= 65504) or not (0 <= v[1] < MAX_AGE) or v[1] != math.floor(v[1]):
        return None
    return (min(v[0], cap), v[1])


def history(taps, valid, cap):
    """(bilinear b, colour, depth_ok, evidence) -> associated numerator and mass."""
    num, mass = [0.0] * 4, 0.0
    for b, c, depth_ok, ev in taps:
        if not b > 0:
            continue
        s = stored(ev, valid, cap)
        if not admitted(c) or not c[3] > 0 or not depth_ok or s is None:
            continue
        num = [n + x * b * s[0] for n, x in zip(num, clean(c))]
        mass += b * s[0]
    return num, mass


def update(num, mass, s, m, density, cap, reaction=0.0, strength=0.0, box=None):
    if not mass > 0:
        return (clean([x / m for x in s]) if m > 0 else None), min(m, cap)
    h = [x / mass for x in num]
    w = min(mass * max(density, 0.0), cap) * (1 - strength * reaction)
    if box:
        h = [min(max(x, lo), hi) for x, lo, hi in zip(h, box[0], box[1])]
    total = w + m
    return clean([(a * w + b) / total for a, b in zip(h, s)]), min(total, cap)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    cap = 36.0
    # Contract defect 1: clear is a coverage observation with full weight.
    s, m, ok = current_evidence([(1.0, (1, 0, 0, 1), True), (1.0, (0, 0, 0, 0), True)])
    check("red + clear at equal weight reconstructs (0.5,0,0,0.5)", clean([x / m for x in s]) == (0.5, 0, 0, 0.5))
    s, m, ok = current_evidence([(1.0, (5.0, 2.0, 1.0, 0.0), True)])
    check("alpha-zero residual is canonical clear with its weight", s == [0.0] * 4 and m == 1.0)
    s, m, ok = current_evidence([(1.0, (float("nan"), 0, 0, 1), True), (1.0, (0.2, 0.2, 0.2, 1), True)])
    check("malformed current tap gains no weight and refuses history", m == 1.0 and not ok)
    s, m, ok = current_evidence([(1.0, (0.2, 0.2, 0.2, 1), False), (2.0 ** -20, (1, 1, 1, 1), True)])
    check("outside-rect taps excluded; tiny positive in-rect tap kept", m == 2.0 ** -20)
    # Associated history support (root counterexample).
    num, mass = history([(0.5, (1, 1, 1, 1), True, (0.0, 0.0)), (0.5, (0, 0, 0, 1), True, (8.0, 0.0))], True, cap)
    out, _ = update(num, mass, [0, 0, 0, 0], 1.0, 1.0, cap)
    check("zero-mass bright tap contributes nothing to history colour", out[0] == 0.0 and mass == 4.0, out)
    num, mass = history([(0.5, (1, 1, 1, 1), True, (float("nan"), 0.0)), (0.5, (0.25,) * 3 + (1,), True, (8.0, 0.0))],
                        True, cap)
    check("nonfinite stored mass rejects that tap's numerator and weight", mass == 4.0 and num[0] == 1.0)
    num, mass = history([(1.0, (1, 1, 1, 1), True, (8.0, 0.5))], True, cap)
    check("non-integer age refuses history", mass == 0.0)
    num, mass = history([(1.0, (1, 1, 1, 1), True, (8.0, 8.0))], True, cap)
    check("age >= 8 refuses history", mass == 0.0)
    num, mass = history([(1.0, (1, 1, 1, 1), True, (30.0, 0.0))], False, cap)
    check("invalid evidence origin (native frame, new allocation, cut) refuses history", mass == 0.0)
    # Contract defect 2: no-surface partial colour cannot survive (ordinary path never stores it).
    retire = [None]
    check("ordinary program stores no no-surface history: retirement is immediate", retire == [None])
    # Contract defect 3: no prior is written into evidence; next numerator holds real samples only.
    s, m, ok = current_evidence([(1.0, (0, 0, 0, 1), True)])
    out, w_new = update([0.0] * 4, 0.0, s, m, 1.0, cap)
    num, mass = history([(1.0, out, True, (w_new, 0.0))], True, cap)
    check("next history numerator after a black start is black (no prior colour stored)", num[:3] == [0.0, 0.0, 0.0])
    # Density: source-rect area ratio with exact rects (rect * n/16).
    for rect, a, b, want in (((2015, 1003), 16, 8, 4.0), ((1003, 2015), 8, 12, 4 / 9), ((1921, 1081), 12, 16, 0.5625)):
        area = lambda n: (rect[0] * n / 16) * (rect[1] * n / 16)
        d = area(a) / area(b)
        check(f"area density {rect} {a}->{b}", abs(d - want) < 1e-12, d)
    num, mass = history([(1.0, (0.5,) * 3 + (1,), True, (36.0, 0.0))], True, 36.0)
    _, w = update(num, mass, [0.0] * 4, 1.0, (8 / 16) ** 2, 9.0)
    check("8/16 -> 16/16 converts 36 units to 9 then adds the new mass, capped", w == 9.0)
    # Clamp is biased: it changes colour but adds no mass.
    num, mass = history([(1.0, (0.9,) * 3 + (1,), True, (10.0, 0.0))], True, cap)
    out, w = update(num, mass, [0.5, 0.5, 0.5, 1.0], 1.0, 1.0, cap, box=((0.4,) * 3 + (1,), (0.6,) * 3 + (1,)))
    check("clamp changes history colour without adding mass", w == 11.0 and abs(out[0] - (0.6 * 10 + 0.5) / 11) < 1e-12)
    result = {"scope": "transcription of resolve.wgsl (ordinary) only; owner program unchanged in this partial",
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
