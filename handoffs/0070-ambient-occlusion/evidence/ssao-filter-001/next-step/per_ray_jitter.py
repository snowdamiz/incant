"""Bounded CPU check of one candidate repair for the N8 perspective failure.

Candidate (not adopted, not production): keep R1's rotation and R3's filter
unchanged, and replace the per-pixel upper-half step phase t = (33 + 2 b)/64
with a per-ray full-stratum phase t_i = frac((b + 0.5)/16 + i/16), i = ray.
Rationale: the eeeac1d4 phase only covers the upper half of each step stratum,
so its expectation keeps a comb at thin occluders (15.8 codes on the N8 row
against 4.81 for the continuous limit, addendum). A full-stratum phase removes
that bias, and spreading the phase across the 16 rays of every pixel (instead
of one shared phase) keeps each pixel's estimate stratified over step
positions, which bounds the extra variance that a shared full-stratum phase
showed (addendum: 0.471). Cost: 16 adds and fracts per pixel, no probe.

Uses ../../../ssao-repair/quality_model.py read-only (f64 model, exact f32
uniform inputs). Prints JSON; stdlib only.
"""

import json
import math
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPAIR = HERE.parents[2] / "ssao-repair"
sys.path[:0] = [str(REPAIR)]
import quality_model as m  # noqa: E402
import quality_report as qr  # noqa: E402


def raw_per_ray(grid, pixels=None):
    """quality_model.raw with t_i = frac((b + 0.5)/16 + i/16) per ray."""
    w, project, hits, r2 = grid.w, grid.camera.project, grid.hits, m.RADIUS * m.RADIUS
    out = [None] * len(hits)
    for index, hit in enumerate(hits):
        if pixels is not None and index not in pixels:
            continue
        y, x = divmod(index, w)
        if hit is None:
            out[index] = 1.0
            continue
        px, py, pz = hit[0], hit[1], hit[2]
        cos, sin = m.ROTATIONS[m.BAYER[y % 4][x % 4]]
        base = (m.BAYER[x % 4][y % 4] + 0.5) / 16
        count = 0
        for ray, (kx, ky, kz) in enumerate(m.DIRECTIONS):
            t = (base + ray / 16) % 1.0
            dx, dy = kx * cos - ky * sin, kx * sin + ky * cos
            for j in range(4):
                s = m.RADIUS * (j + t) / 4
                qx, qy, qz = px + dx * s, py + dy * s, pz + kz * s
                at = project(qx, qy, qz)
                if at is None:
                    continue
                sx, sy = int(math.floor(at[0])), int(math.floor(at[1]))
                if not (0 <= sx < w and 0 <= sy < grid.h):
                    continue
                surface = hits[sy * w + sx]
                if surface is None:
                    continue
                ex, ey, ez = surface[0] - px, surface[1] - py, surface[2] - pz
                if surface[2] >= qz + m.BIAS and ez > m.BIAS and ex * ex + ey * ey + ez * ez <= r2:
                    count += 1
                    break
        out[index] = 1.0 - count / 16
    return out


def n8(orthographic, extra):
    grid = m.Grid(qr.STRIPES, orthographic)
    row, cols = qr.n8_roi(grid)
    rows = {}
    for r in range(row, row + extra + 1):
        idx = [r * grid.w + x for x in cols]
        old = m.raw(grid, rotate=False, jitter=False, pixels=set(idx))
        shipped = max(old[i] for i in idx) - min(old[i] for i in idx)
        cand = m.filtered(grid, raw_per_ray(grid, m.support(grid, set(idx))), pixels=set(idx))
        design = m.filtered(grid, m.raw(grid, pixels=m.support(grid, set(idx))), pixels=set(idx))
        rows[str(r)] = {"candidate_ratio": round((max(cand[i] for i in idx) - min(cand[i] for i in idx)) / shipped, 3),
                        "design_ratio": round((max(design[i] for i in idx) - min(design[i] for i in idx)) / shipped, 3)}
    return {"roi_row": row, "rows": rows}


def main():
    start = time.time()
    report = {"candidate": "per-ray full-stratum step phase t_i = frac((b + 0.5)/16 + i/16)",
              "n8_persp": n8(False, 6), "n8_ortho": n8(True, 0)}
    n7 = {}
    for name, orthographic, key, bound in (("white_slab", False, "white_slab_persp", 138.2),
                                           ("white_slab", True, "white_slab_ortho", 176.7),
                                           ("edge_slab", False, "edge_slab_persp", 202.7)):
        grid = m.Grid(name, orthographic)
        values = m.filtered(grid, raw_per_ray(grid))
        n7[key] = {"count": m.n7_steps(grid, values), "bound": bound}
        if key == "white_slab_persp":
            report["n6_authored_255"] = round(m.n6_row(grid, values)[0] * 255, 2)
    report["n7"] = n7
    report["seconds"] = round(time.time() - start, 1)
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
