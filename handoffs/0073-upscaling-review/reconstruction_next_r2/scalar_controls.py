#!/usr/bin/env python3
"""0073 reconstruction_next_r2: analytic and negative controls for the selected equations.

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_next_r2/scalar_controls.py \
        --out handoffs/0073-upscaling-review/reconstruction_next_r2/scalar_controls.json

Standard library only. Pure transcriptions of the proposed resolve arithmetic and of the
production 76be9608 admission rules; no renderer, GPU, image or model sweep.
"""
import argparse
import json
import math
import sys

sys.dont_write_bytecode = True
CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def clean(c):
    if not all(math.isfinite(v) for v in c) or c[3] <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    return tuple([min(max(v, 0.0), 65504.0) for v in c[:3]] + [min(max(c[3], 0.0), 1.0)])


def stored_evidence(w, cap):
    return min(w, cap) if (w == w and 0.0 <= w <= 65504.0) else 0.0


def history(taps, cap, valid=True):
    """Selected rule: one admitted tap set gives numerator, confidence and colour.
    taps: (bilinear weight >= 0, colour, stored evidence, admitted)."""
    num, conf = [0.0] * 4, 0.0
    for b, c, w, ok in taps:
        if not ok or not b > 0:
            continue
        e = stored_evidence(w, cap) if valid else 0.0
        conf += b * e
        num = [n + b * e * v for n, v in zip(num, clean(c))]
    return num, conf


def blend(num, conf, density, cap, s, m, prior, k):
    """colour = (scale*N + S + k*E) / (W' + m + k), W' = min(conf*density, cap), scale = W'/conf."""
    w = min(conf * density, cap) if conf > 0 else 0.0
    scale = w / conf if conf > 0 else 0.0
    out = [(scale * n + sv + k * e) / (w + m + k) for n, sv, e in zip(num, s, prior)]
    return clean(out), min(w + m, cap)


def controls():
    cap = 36.0
    # 1. Root's counterexample: confidence-associated history numerator is zero.
    taps = [(0.5, (1, 1, 1, 1), 0.0, True), (0.5, (0, 0, 0, 1), 8.0, True)]
    num, conf = history(taps, cap)
    got, _ = blend(num, conf, 1.0, cap, (0, 0, 0, 0), 1.0, (0, 0, 0, 0), 0.0)
    independent = 0.5 * conf / (conf + 1.0)       # independently bilinear H = .5 with W = 4
    check("same-support history: zero-confidence bright tap contributes nothing", got[0] == 0.0 and conf == 4.0,
          {"selected": got[0], "independent_bilinear_H_would_give": independent})
    # 2. Rejected taps leave both numerator and confidence.
    taps = [(0.5, (1, 1, 1, 1), 8.0, False), (0.5, (0.25, 0.25, 0.25, 1), 8.0, True)]
    num, conf = history(taps, cap)
    check("rejected tap removes its colour and its confidence", conf == 4.0 and num[0] == 1.0)
    # 3. Non-finite or negative stored evidence is none; non-finite colour is clean() zero.
    for w in (float("nan"), float("inf"), -1.0):
        num, conf = history([(1.0, (1, 1, 1, 1), w, True)], cap)
        check(f"stored evidence {w} reads as zero", conf == 0.0 and num == [0.0] * 4)
    num, conf = history([(1.0, (float("nan"), 1, 1, 1), 8.0, True)], cap)
    check("non-finite history colour is canonical zero with its confidence", num == [0.0] * 4 and conf == 8.0)
    # 4. Invalid evidence chain (native frame, new allocation): history ignored entirely.
    num, conf = history([(1.0, (1, 1, 1, 1), 30.0, True)], cap, valid=False)
    got, w_new = blend(num, conf, 1.0, cap, (0.2, 0.2, 0.2, 1.0), 1.0, (0.3, 0.3, 0.3, 1), 8.0)
    check("evidence-invalid chain: output is current evidence plus prior only", abs(got[0] - (0.2 + 8 * 0.3) / 9) < 1e-12
          and w_new == 1.0)
    # 5. Density conversion is exact in sample units and capped at the new ratio.
    num, conf = history([(1.0, (0.5, 0.5, 0.5, 1), 36.0, True)], 36.0)
    _, w16 = blend(num, conf, (480 / 960) ** 2, 9.0, (0, 0, 0, 0), 1.0, (0, 0, 0, 0), 0.0)
    check("8/16 -> 16/16: 36 units become 9 (= new cap), then + m", w16 == 9.0)
    _, w8 = blend(*history([(1.0, (0.5, 0.5, 0.5, 1), 9.0, True)], 9.0), (960 / 480) ** 2, 36.0, (0, 0, 0, 0), 1.0,
                  (0, 0, 0, 0), 0.0)
    check("16/16 -> 8/16: 9 units become 36 (no softening pop), then capped", w8 == 36.0)
    # 6. Convex coverage: alpha never exceeds 1, premultiplied stays premultiplied.
    num, conf = history([(1.0, (0.3, 0.3, 0.3, 0.4), 10.0, True)], cap)
    got, _ = blend(num, conf, 1.0, cap, (0.0, 0.0, 0.0, 0.0), 2.0, (0.8, 0.8, 0.8, 1.0), 3.0)
    check("coverage blend is convex: 0 <= rgb <= alpha <= 1", 0 <= got[0] <= got[3] <= 1, got)
    # 7. Opaque-history guard: no persistence over an all-clear neighbourhood.
    persist = lambda surface_seen, hist_alpha: surface_seen or hist_alpha < 1.0
    check("opaque history over all-clear 3x3 never persists", not persist(False, 1.0))
    check("partial coverage over all-clear 3x3 may persist (thin wire missed phase)", persist(False, 0.4))
    # 8. Bilinear resampling attenuation: exactly 1 at integer reprojection, 0.5 at a half texel per axis.
    att = lambda f: (1 - 2 * f[0] * (1 - f[0])) * (1 - 2 * f[1] * (1 - f[1]))
    check("resampling attenuation is 1 for a static (integer) reprojection", att((0.0, 0.0)) == 1.0)
    check("resampling attenuation is 0.25 at (0.5, 0.5)", att((0.5, 0.5)) == 0.25)
    # 9. Changed secondary owner: arbitrarily small positive weight still rejects the history tap.
    tiny = 2.0 ** -20
    def admissible(record, anchor, changed):
        return record[0] == anchor and not any(changed(o) for o in record[1:])
    check("history tap with changed secondary rejected at 2^-20 weight",
          tiny > 0 and not admissible((5, 7), 5, lambda o: o == 7))
    # 10. Mean-only clamp on DRS-change frames keeps detail, bounds the mean.
    hist, hist_low, lo, hi = 0.9, 0.5, 0.45, 0.55
    kept = min(max(hist_low, lo), hi) + (hist - hist_low)
    check("DRS-frame mean clamp keeps the history's high-frequency part", abs(kept - 0.9) < 1e-12)
    hist_low2 = 0.9
    kept2 = min(max(hist_low2, lo), hi) + (hist - hist_low2)
    check("DRS-frame mean clamp still bounds a mean (lighting) change", abs(kept2 - 0.55) < 1e-12)


# --------------------------------------------------------------------- exact admission

P, INST = 4 * 1024 * 1024, 64 * 1024 * 1024
CEILING = 128 * P + 2 * INST + 16384


def source_extent(out, n):
    return tuple(math.ceil(v * n / 16) for v in out)       # Plan::new: ceil only on attachments


def grid(out):
    c = math.ceil(out[0] / 64) * math.ceil(out[1] / 64) * 24
    return c * 516 if c <= 131072 else None


def ordinary(stride, old, new):
    """plan.rs preflight (old + candidate, no reuse discount) + State output grid (old + candidate)."""
    (oo, os_), (no, ns) = old, new
    o = lambda e: e[0] * e[1]
    total = stride * o(oo) + 8 * o(oo) + 24 * o(os_) + (stride + 8) * o(no) + 24 * o(ns) + 2 * INST + 16384
    return total + grid(oo) + grid(no)


def owner(stride, old, new):
    """motion/attachment.rs admit_records (exact reuse discount) + State grid (old + candidate)."""
    (oo, os_), (no, ns) = old, new
    o = lambda e: e[0] * e[1]
    comp = lambda a, b, s: s * o(a) + (0 if a == b else s * o(b))
    total = 2 * INST + 16384 + comp(os_, ns, 24) + comp(os_, ns, 4) + comp(oo, no, stride) + comp(oo, no, 16) \
        + comp(oo, no, 8)
    return total + grid(oo) + grid(no)


def admission():
    rows = {}
    cases = {"2048x2048 steady 8/16": ((2048, 2048), 8, 8), "2048x2048 16/16 -> 8/16": ((2048, 2048), 16, 8),
             "2015x2015 16/16 -> 8/16": ((2015, 2015), 16, 8), "2014x2014 16/16 -> 8/16": ((2014, 2014), 16, 8),
             "2560x1440 steady 16/16": ((2560, 1440), 16, 16), "2532x1170 16/16 -> 8/16": ((2532, 1170), 16, 8),
             "1923x1923 steady 16/16": ((1923, 1923), 16, 16), "1890x1890 steady 16/16": ((1890, 1890), 16, 16)}
    for name, (out, a, b) in cases.items():
        old, new = (out, source_extent(out, a)), (out, source_extent(out, b))
        row = {}
        for label, stride, fn in (("ordinary_36", 36, ordinary), ("ordinary_40", 40, ordinary),
                                  ("owner_36", 36, owner), ("owner_40", 40, owner)):
            t = fn(stride, old, new)
            row[label] = {"bytes": t, "fits": t <= CEILING}
        rows[name] = row
    n = 2015
    exact = ordinary(40, ((n, n), source_extent((n, n), 16)), ((n, n), source_extent((n, n), 8)))
    floor_proto = exact - 24 * (source_extent((n, n), 8)[0] ** 2) + 24 * ((n * n) // 4)
    check("odd 2015: exact ceil source differs from the n*n//4 prototype by 24192 B", exact - floor_proto == 24192,
          exact - floor_proto)
    check("ordinary 40 B stride: 2014 fits and 2015 is refused under exact extents (16 -> 8)",
          rows["2014x2014 16/16 -> 8/16"]["ordinary_40"]["fits"] and not rows["2015x2015 16/16 -> 8/16"]["ordinary_40"]["fits"])
    check("production 36 B stride admits the 2048^2 16 -> 8 transition; 40 B refuses it",
          rows["2048x2048 16/16 -> 8/16"]["ordinary_36"]["fits"] and not rows["2048x2048 16/16 -> 8/16"]["ordinary_40"]["fits"])
    check("ceiling unchanged", CEILING == 671_105_024)
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    controls()
    result = {"scope": "analytic transcription; not renders, not historical gates", "admission": admission(),
              "checks": CHECKS}
    result["all_checks_pass"] = all(c["ok"] for c in CHECKS)
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
