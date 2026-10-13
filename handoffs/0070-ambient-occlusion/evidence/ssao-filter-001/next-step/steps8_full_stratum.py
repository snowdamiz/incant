"""Second and last bounded CPU check for the N8 perspective failure.

Candidate (not adopted, not production): 8 probes per ray with the
full-stratum step phase t = (b + 0.5)/16, keeping R1's rotation and R3's
filter. The addendum tested each half alone (8 probes with the upper-half
phase: 0.220 at the ROI but 0.376/0.408 below it and N6T 5.37; full stratum
with 4 probes: 0.471), never the pair. Rationale: halving the probe spacing
halves the gap that aliases with the 0.1-wide stripes, and the full-stratum
phase removes the upper-half bias that leaves the converged comb (15.80/255
against 4.81 for the continuous limit). Cost: the AO pass roughly doubles,
like the addendum's 8-probe variant.

Self-checks: the design and the upper-half 8-probe rows are recomputed and
compared with the addendum. Uses ../../../ssao-repair/quality_model.py and
quality_report.py read-only. Prints JSON; stdlib only.
"""

import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parents[2] / "ssao-repair")]
import quality_model as m  # noqa: E402
import quality_report as qr  # noqa: E402

VARIANTS = {"design": {"full_stratum": False, "steps": 4},
            "steps8_upper_half": {"full_stratum": False, "steps": 8},
            "steps8_full_stratum": {"full_stratum": True, "steps": 8}}
ADDENDUM_ROWS = {"design": [0.376, 0.439, 0.502, 0.533, 0.356, 0.335, 0.272],
                 "steps8_upper_half": [0.220, 0.282, 0.376, 0.408, 0.293, 0.230, 0.146]}


def output(grid, variant, pixels=None):
    source = m.support(grid, pixels) if pixels is not None else None
    return m.filtered(grid, m.raw(grid, pixels=source, **VARIANTS[variant]), pixels=pixels)


def n8(orthographic, extra):
    grid = m.Grid(qr.STRIPES, orthographic)
    row, cols = qr.n8_roi(grid)
    rows = {}
    for r in range(row, row + extra + 1):
        idx = [r * grid.w + x for x in cols]
        old = m.raw(grid, rotate=False, jitter=False, pixels=set(idx))
        shipped = max(old[i] for i in idx) - min(old[i] for i in idx)
        rows[str(r)] = {}
        for variant in VARIANTS:
            v = output(grid, variant, set(idx))
            rows[str(r)][variant] = round((max(v[i] for i in idx) - min(v[i] for i in idx)) / shipped, 3)
    return {"roi_row": row, "cols": [cols[0], cols[-1]], "rows": rows}


def n6t():
    grid = m.Grid("white_slab", False)
    at = lambda x: qr.N6_ROW * grid.w + x  # noqa: E731
    span = {at(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)}
    truth = m.raw_directions(grid, m.support(grid, span), *qr.CONTINUOUS)
    truth = m.filtered(grid, [truth.get(i) for i in range(len(grid.hits))], quantize=False, pixels=span)
    pairs = qr.n6_pairs(grid)
    step = lambda v, x: v[at(x + 1)] - v[at(x)]  # noqa: E731
    out = {}
    for variant in VARIANTS:
        v = output(grid, variant, span)
        out[variant] = round(max(abs(step(v, x) - step(truth, x)) for x in pairs) * 255, 2)
    return out


def main():
    start = time.time()
    persp = n8(False, 6)
    report = {"candidate": "8 probes per ray, full-stratum step phase t = (b + 0.5)/16, R1 rotation, R3 filter",
              "n8_persp": persp, "n8_ortho": n8(True, 0), "n6t_persp_255": n6t()}
    report["self_check_rows_match_addendum"] = {
        variant: [persp["rows"][r][variant] for r in sorted(persp["rows"], key=int)] == rows
        for variant, rows in ADDENDUM_ROWS.items()}
    n7, n6 = {}, {}
    for name, orthographic, key, bound in (("white_slab", False, "white_slab_persp", 138.2),
                                           ("white_slab", True, "white_slab_ortho", 176.7),
                                           ("edge_slab", False, "edge_slab_persp", 202.7)):
        grid = m.Grid(name, orthographic)
        values = output(grid, "steps8_full_stratum")
        n7[key] = {"count": m.n7_steps(grid, values), "bound": bound}
        if key == "white_slab_persp":
            n6["steps8_full_stratum"] = round(m.n6_row(grid, values)[0] * 255, 2)
    report["n7_candidate"] = n7
    report["n6_authored_persp_255"] = n6
    report["seconds"] = round(time.time() - start, 1)
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
