"""Evidence for root's N6T adoption decision (next-repair-preparation.md).

CPU only; standard library only. N6 authored stays failed and unchanged: this
script only measures how the authored statistic and the proposed N6T treat
fields whose answer is known.

  N6   = max over pairs |A(x+1) - A(x)|                      (row 97, white_slab perspective)
  N6T  = max over pairs |(A(x+1) - A(x)) - (T(x+1) - T(x))|  (same pairs, same 1/64 bound)

T is the continuous limit of the same single-layer test (24 x 64 cosine
directions, 32 steps), passed through the candidate's own H/V taps without R8
rounding, or unfiltered for an unfiltered output: N6T measures the error of the
local gradient, not the gradient. Pairs are the authored 70 (x 100..169),
both texels Chebyshev-2 interior. Orthographic is recorded as a diagnostic.

Fields:
  build003   the shipped stepped/hatch output, inferred per texel from the
             retained build003 device frames (tools/analyze.py infer_k, read
             only) and cross-checked against the shipped model;
  control    an independent smooth legitimate contact gradient: the finer
             continuous limit (32 x 96 directions, 48 steps; not T's grid)
             through R3's filter with its three R8 writes, and unfiltered
             with one R8 write;
  banded     the control requantized to the shipped 1/16 levels;
  hatch      the control with an alternating +-1/32 dither along the row;
  injected   the control with one step of s codes added at x 135;
  R1/R3      the measured 001 device V plane and the CPU model;
  C2         the next-repair candidate (screen.py), CPU model.

  /opt/homebrew/bin/python3 -I -B n6t_controls.py > n6t-controls-001.json
"""

import json
import math
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
sys.path[:0] = [str(HERE), str(PACKET / "ssao-repair"), str(PACKET / "tools")]
import analyze as build003  # noqa: E402  (tools/analyze.py: released build004 analyzer, read only)
import quality_model as m  # noqa: E402
import quality_report as qr  # noqa: E402
import screen  # noqa: E402

B003 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-003/outputs/case-ssao_core")
E001 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001/outputs/case-ssao_core")
BOUND = 1 / 64


class Row:
    def __init__(self, orthographic):
        self.grid = g = m.Grid("white_slab", orthographic)
        self.at = lambda x: qr.N6_ROW * g.w + x  # noqa: E731
        self.span = {self.at(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)}
        self.pairs = qr.n6_pairs(g)
        support = m.support(g, self.span)
        base = m.raw_directions(g, support, *qr.CONTINUOUS)
        fine = m.raw_directions(g, support, *qr.CONTINUOUS_FINE)
        as_list = lambda d: [d.get(k) for k in range(len(g.hits))]  # noqa: E731
        self.truth_unfiltered = as_list(base)
        self.truth_filtered = m.filtered(g, as_list(base), quantize=False, pixels=self.span)
        self.fine_unfiltered = as_list(fine)
        self.fine_filtered_exact = m.filtered(g, as_list(fine), quantize=False, pixels=self.span)
        self.fine_filtered_r8 = m.filtered(g, as_list(fine), pixels=self.span)

    def score(self, values, filtered=True):
        truth = self.truth_filtered if filtered else self.truth_unfiltered
        at = self.at
        steps = {x: values[at(x + 1)] - values[at(x)] for x in self.pairs}
        n6x = max(steps, key=lambda x: abs(steps[x]))
        resid = {x: steps[x] - (truth[at(x + 1)] - truth[at(x)]) for x in self.pairs}
        n6tx = max(resid, key=lambda x: abs(resid[x]))
        return {"n6_255": round(abs(steps[n6x]) * 255, 2), "n6_at": n6x, "n6_pass": abs(steps[n6x]) <= BOUND,
                "n6t_255": round(abs(resid[n6tx]) * 255, 2), "n6t_at": n6tx, "n6t_pass": abs(resid[n6tx]) <= BOUND}


def build003_values(row):
    """Shipped accessibility 1 - k/16 per texel from the retained build003 frames."""
    frames = build003.load_output(B003)
    on, off, spec = frames["p_ao_white"], frames["p_off_white"], frames["p_off_black"]
    values, ambiguous = [None] * len(row.grid.hits), []
    for i in row.span:
        y, x = divmod(i, row.grid.w)
        interval, reason = build003.infer_k(on.px(x, y), off.px(x, y), spec.px(x, y), 1.0)
        ks = [] if interval is None else [k for k in range(17) if interval[0] <= k <= interval[1]]
        if len(ks) != 1:
            ambiguous.append({"x": x, "interval": interval, "reason": reason})
            continue
        values[i] = 1 - ks[0] / 16
    return values, ambiguous


def r8(v):
    return None if v is None else m.r8(v)


def main():
    start = time.time()
    persp, ortho = Row(False), Row(True)
    g = persp.grid
    shipped_model = m.raw(g, rotate=False, jitter=False, pixels=persp.span)
    device003, ambiguous = build003_values(persp)
    compared = [i for i in persp.span if device003[i] is not None]
    fields = {}
    fields["build003_device"] = {**persp.score(device003, filtered=False),
                                 "inferred_texels": len(compared), "ambiguous": ambiguous,
                                 "equals_shipped_model": all(device003[i] == shipped_model[i] for i in compared)}
    fields["build003_shipped_model"] = persp.score(shipped_model, filtered=False)
    control = persp.fine_filtered_r8
    fields["control_fine_limit_filtered_r8"] = persp.score(control)
    fields["control_fine_limit_filtered_exact"] = persp.score(persp.fine_filtered_exact)
    fields["control_fine_limit_unfiltered_r8"] = persp.score([r8(v) for v in persp.fine_unfiltered], filtered=False)
    fields["truth_grid_unfiltered_r8"] = persp.score([r8(v) for v in persp.truth_unfiltered], filtered=False)
    fields["banded_control_1_16"] = persp.score([None if v is None else round(v * 16) / 16 for v in control])
    hatch = list(control)
    for i in persp.span:
        hatch[i] = min(1.0, max(0.0, hatch[i] + (1 / 32 if (i % g.w) % 2 else -1 / 32)))
    fields["hatch_control_pm_1_32"] = persp.score(hatch)
    injected = {}
    for s in (2, 3, 4, 5, 6, 8, 16):
        bumped = list(control)
        for i in persp.span:
            if i % g.w >= 135:
                bumped[i] = bumped[i] + s / 255
        injected[str(s)] = persp.score(bumped)
    fields["injected_step_codes"] = injected
    device001 = [c / 255 for c in (E001 / "p_ao_white.v.r8").read_bytes()]
    fields["r1r3_device_001"] = persp.score(device001)
    fields["r1r3_model"] = persp.score(screen.output(g, "D", persp.span))
    fields["c2_model"] = persp.score(screen.output(g, "C2", persp.span))
    og = ortho.grid
    diagnostic = {
        "control_fine_limit_filtered_r8": ortho.score(ortho.fine_filtered_r8),
        "control_fine_limit_filtered_exact": ortho.score(ortho.fine_filtered_exact),
        "r1r3_device_001": ortho.score([c / 255 for c in (E001 / "o_ao_white.v.r8").read_bytes()]),
        "c2_model": ortho.score(screen.output(og, "C2", ortho.span)),
        "shipped_model": ortho.score(m.raw(og, rotate=False, jitter=False, pixels=ortho.span), filtered=False),
    }
    report = {"format": "incant-0070-n6t-controls-v1", "bound_255": round(BOUND * 255, 3),
              "pairs": [persp.pairs[0], persp.pairs[-1], len(persp.pairs)],
              "truth": "24 x 64 directions, 32 steps; control: 32 x 96 directions, 48 steps",
              "truth_max_step_255": {
                  "unfiltered": round(max(abs(persp.truth_unfiltered[persp.at(x + 1)] - persp.truth_unfiltered[persp.at(x)])
                                          for x in persp.pairs) * 255, 2),
                  "filtered": round(max(abs(persp.truth_filtered[persp.at(x + 1)] - persp.truth_filtered[persp.at(x)])
                                        for x in persp.pairs) * 255, 2)},
              "perspective": fields, "orthographic_diagnostic": diagnostic,
              "seconds": round(time.time() - start, 1)}
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
