"""Itemized decisions of the filtered oracle on every reference fixture, and the
quality verdicts any GPU result agreeing with it must reach (interval-sound).

  /opt/homebrew/bin/python3 -I -B oracle_summary.py > oracle-summary-001.json
"""

import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze_filter as af  # noqa: E402

qm, qr = af.qm, af.qr


def bounds_n6(finals):
    """Smallest and largest possible authored N6 (codes) over the oracle intervals."""
    grid = qm.Grid("white_slab", False)
    low = high = 0
    for x in qr.n6_pairs(grid):
        (a_lo, a_hi), (b_lo, b_hi) = finals[(x, qr.N6_ROW)], finals[(x + 1, qr.N6_ROW)]
        low = max(low, b_lo - a_hi, a_lo - b_hi, 0)
        high = max(high, b_hi - a_lo, a_hi - b_lo)
    return low, high


def bounds_n8(finals, orthographic):
    grid = qm.Grid(qr.STRIPES, orthographic)
    row, cols = qr.n8_roi(grid)
    window = [finals[(x, row)] for x in cols]
    old = qm.raw(grid, rotate=False, jitter=False, pixels={row * grid.w + x for x in cols})
    shipped = (max(old[row * grid.w + x] for x in cols) - min(old[row * grid.w + x] for x in cols)) * 255
    low = max(0, max(lo for lo, _ in window) - min(hi for _, hi in window))
    high = max(hi for _, hi in window) - min(lo for lo, _ in window)
    return {"row": row, "cols": [cols[0], cols[-1]], "shipped_codes": round(shipped, 2),
            "ratio_range": [round(low / shipped, 3), round(high / shipped, 3)], "bound": 0.25,
            "verdict": "fails for every agreeing result" if low > 0.25 * shipped
            else "passes for every agreeing result" if high <= 0.25 * shipped else "undetermined"}


def bounds_n7(finals, name, orthographic, key):
    """Largest possible exact-definition N7 count from the oracle intervals."""
    grid = qm.Grid(name, orthographic)
    inside = [[grid.interior(x, y) for x in range(grid.w)] for y in range(grid.h)]
    worst = 0
    for x, y in af.INTERIOR:
        if inside[y][x] and inside[y][x + 1] and inside[y + 1][x]:
            a = finals[(x, y)]
            step = max(max(abs(b[1] - a[0]), abs(a[1] - b[0])) for b in (finals.get((x + 1, y)), finals.get((x, y + 1)))
                       if b is not None)
            worst += step >= 255 / 16
    bound = af.N7_SHIPPED[key] / 10
    return {"max_possible": worst, "bound": bound,
            "verdict": "passes for every agreeing result" if worst <= bound else "undetermined"}


def main():
    start, fixtures = time.time(), {}
    for name, case, on, off, spec, scene, orthographic, strength, occludes in af.REFERENCE_GATES:
        oracle, finals, summary = af.oracle_for(scene, orthographic, strength)
        fixtures[name] = {**summary, "expects_occlusion": occludes,
                          "certainly_occluded": sum(c[1] <= 254 for c in finals.values()),
                          "exactly_white": sum(c == (255, 255) for c in finals.values()),
                          "decisive_pixels": sum(oracle.decisive(*p) for p in af.INTERIOR)}
    white_p = af.oracle_for("white_slab", False, 1.0)[1]
    n6_low, n6_high = bounds_n6(white_p)
    predictions = {
        "n6_authored_codes": {"range": [n6_low, n6_high], "bound": round(255 / 64, 3),
                              "verdict": "fails for every agreeing result" if n6_low > 255 / 64 else "undetermined"},
        "n7": {key: bounds_n7(af.oracle_for(n, o, 1.0)[1], n, o, key) for n, o, key in (
            ("white_slab", False, "white_slab_persp"), ("white_slab", True, "white_slab_ortho"),
            ("edge_slab", False, "edge_slab_persp"))},
        "n8_persp": bounds_n8(af.oracle_for("stripes", False, 1.0)[1], False),
        "n8_ortho": bounds_n8(af.oracle_for("stripes", True, 1.0)[1], True),
    }
    report = {"format": "incant-0070-ssao-filter-oracle-summary-v1", "fixtures": fixtures,
              "predictions": predictions, "interior_pixels": len(af.INTERIOR),
              "seconds": round(time.time() - start, 1)}
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
