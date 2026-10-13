"""Reproducible CPU evidence for ssao-quality-repair-addendum.md (stdlib only).

From the worktree root:
  /opt/homebrew/bin/python3 -I -B \
    handoffs/0070-ambient-occlusion/ssao-repair/quality_report.py \
    > handoffs/0070-ambient-occlusion/ssao-repair/cpu-evidence-001.json

Foreground and bounded: one raw grid per configuration and fixture for N7, and
row-window supports only for the converged references and the continuous
limit. Imports tools/analyze.py read-only (with -B) solely to re-derive the
previous N7 definition. Exit 1 if the model does not reproduce Astra's logged
preliminary values.
"""

import json
import os
import sys
import time
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import quality_model as m  # noqa: E402

# Astra's preliminary log (ssao-quality-oracle-001/tests-001.log), for the model check.
ASTRA = {
    ("white_slab", False): {"old_N7": 1382, "design_N7": 0, "design_no_rotation_N7": 0,
                            "old_N6": 0.125, "design_N6": 0.027450980392156876,
                            "design_no_jitter_N6": 0.027450980392156876},
    ("white_slab", True): {"old_N7": 1767, "design_N7": 0, "design_no_rotation_N7": 51,
                           "old_N6": 0.1875, "design_N6": 0.04313725490196074,
                           "design_no_jitter_N6": 0.03921568627450989},
    ("edge_slab", False): {"old_N7": 2027, "design_N7": 0, "design_no_rotation_N7": 0,
                           "old_N6": 0.0, "design_N6": 0.0, "design_no_jitter_N6": 0.0},
}
FIELDS = ("rotate", "jitter", "full_stratum", "steps", "tile", "filter", "family")
# Unfiltered outputs stay unquantized 1 - k/16, as in the recorded baselines
# (R8 rounding would turn some exact 1/16 steps into 15/255).
CONFIGS = {name: dict(zip(FIELDS, v)) for name, v in {
    "old": (False, False, False, 4, 4, False, None),
    "design": (True, True, False, 4, 4, True, "design"),
    "design_no_rotation": (False, True, False, 4, 4, True, "design"),
    "design_no_jitter": (True, False, False, 4, 4, True, "design"),
    "design_filter_only": (False, False, False, 4, 4, True, "design"),
    "design_no_filter": (True, True, False, 4, 4, False, "design"),
    "steps8": (True, True, False, 8, 4, True, "steps8"),
    "steps8_no_rotation": (False, True, False, 8, 4, True, "steps8"),
    "steps8_no_jitter": (True, False, False, 8, 4, True, "steps8"),
    "steps8_filter_only": (False, False, False, 8, 4, True, "steps8"),
    "steps8_no_filter": (True, True, False, 8, 4, False, "steps8"),
    "full_stratum": (True, True, True, 4, 4, True, "full_stratum"),
    "tile8_full_stratum": (True, True, True, 4, 8, True, "full_stratum"),
}.items()}
FAMILIES = {"design": (False, 4), "steps8": (False, 8), "full_stratum": (True, 4)}  # (full_stratum, steps)
N6_ROW, N6_X = 97, (100, 171)
STRIPES = "stripes:-0.5,-0.5,1.5"
STRATA, CHECK_STRATA = (32, 16), (64, 32)
CONTINUOUS = (m.continuous_directions(24, 64), [(k + 0.5) / 32 for k in range(32)])
CONTINUOUS_FINE = (m.continuous_directions(32, 96), [(k + 0.5) / 48 for k in range(48)])


def output(grid, name, pixels=None):
    c = CONFIGS[name]
    source = m.support(grid, pixels, c["tile"] // 2) if (c["filter"] and pixels is not None) else pixels
    values = m.raw(grid, rotate=c["rotate"], jitter=c["jitter"], full_stratum=c["full_stratum"],
                   steps=c["steps"], tile=c["tile"], pixels=source)
    return m.filtered(grid, values, taps=m.taps(c["tile"]), pixels=pixels) if c["filter"] else values


class References:
    """Converged family expectations and the continuous limit, raw and filtered."""

    def __init__(self, grid, pixels):
        self.grid, self.pixels, self.cache, self.precision = grid, pixels, {}, {}

    def _raw(self, key, half):
        support = m.support(self.grid, self.pixels, half)
        if key == "truth":
            values = m.raw_directions(self.grid, support, *CONTINUOUS)
            return [values.get(i) for i in range(len(self.grid.hits))]
        full, steps = FAMILIES[key]
        values = m.converged(self.grid, support, STRATA, full, steps)
        check = m.converged(self.grid, set(self.pixels), CHECK_STRATA, full, steps)  # finer strata
        self.precision[key] = max(self.precision.get(key, 0.0), max(abs(values[i] - check[i]) for i in self.pixels))
        return values

    def get(self, key, config):
        c = CONFIGS[config]
        tile = c["tile"] if c["filter"] else 0
        if (key, tile) not in self.cache:
            raw_key = (key, "raw", max(tile // 2, 2))
            if raw_key not in self.cache:
                self.cache[raw_key] = self._raw(key, max(tile // 2, 2))
            base = self.cache[raw_key]
            self.cache[(key, tile)] = (m.filtered(self.grid, base, taps=m.taps(tile), quantize=False, pixels=self.pixels)
                                       if tile else base)
        return self.cache[(key, tile)]


def n6_pairs(grid):
    return [x for x in range(*N6_X) if grid.interior(x, N6_ROW) and grid.interior(x + 1, N6_ROW)]


def n6_section(orthographic):
    g = m.Grid("white_slab", orthographic)
    pairs, i = n6_pairs(g), (lambda x: N6_ROW * g.w + x)
    span = [i(x) for x in range(N6_X[0], N6_X[1] + 1)]
    refs = References(g, set(span))
    step = lambda v, x: v[i(x + 1)] - v[i(x)]  # noqa: E731
    section = {"pairs": [pairs[0], pairs[-1], len(pairs)]}
    for name, c in CONFIGS.items():
        v = output(g, name, set(span))
        steps = {x: abs(step(v, x)) for x in pairs}
        top = max(steps, key=steps.get)
        entry = {"N6_max_step": steps[top], "N6_at": top}
        truth = refs.get("truth", name)
        entry["N6T_vs_continuous"] = max(abs(step(v, x) - step(truth, x)) for x in pairs)
        if c["family"]:
            ref = refs.get(c["family"], name)
            resid = {x: abs(step(v, x) - step(ref, x)) for x in pairs}
            worst = max(resid, key=resid.get)
            entry.update({"N6R_vs_family": resid[worst], "N6R_at": worst,
                          "family_reference_max_step": max(abs(step(ref, x)) for x in pairs)})
        section[name] = entry
    section["continuous_max_step_raw"] = max(abs(step(refs.get("truth", "old"), x)) for x in pairs)
    section["continuous_max_step_filtered"] = max(abs(step(refs.get("truth", "design"), x)) for x in pairs)
    section["reference_precision"] = refs.precision
    return section


def n8_roi(g):
    """Topmost receiver row below the stripes whose window texels are all
    Chebyshev-2 receiver-interior; window = texels whose centre ray meets the
    receiver plane z = -4 at view x in [-0.15, 0.15)."""
    def view_x(x):
        o, d = g.camera.ray(x + 0.5, 100.5)
        return o[0] + d[0] * ((-4.0 - o[2]) / d[2])
    cols = [x for x in range(g.w) if -0.15 <= view_x(x) < 0.15]
    last = max(y for y in range(g.h) if any(g.at(x, y) and g.at(x, y)[3] > 0 for x in range(g.w)))
    row = next(y for y in range(last + 1, g.h) if all(g.interior(x, y) and g.at(x, y)[3] == 0 for x in cols))
    return row, cols


def n8_section(orthographic):
    g = m.Grid(STRIPES, orthographic)
    row, cols = n8_roi(g)
    idx = [row * g.w + x for x in cols]
    refs = References(g, set(idx))
    p2p = lambda vals: max(vals) - min(vals)  # noqa: E731
    old = output(g, "old", set(idx))
    old_p2p = p2p([old[i] for i in idx])
    old_vs_truth = p2p([old[i] - refs.get("truth", "old")[i] for i in idx])
    section = {"row": row, "cols": [cols[0], cols[-1], len(cols)],
               "continuous_p2p_raw": p2p([refs.get("truth", "old")[i] for i in idx])}
    for name, c in CONFIGS.items():
        v = output(g, name, set(idx)) if name != "old" else old
        truth = refs.get("truth", name)
        entry = {"N8_p2p": p2p([v[i] for i in idx]), "profile_255": [round(v[i] * 255, 2) for i in idx]}
        entry["N8_ratio_to_old"] = entry["N8_p2p"] / old_p2p
        entry["N8T_ratio_to_old"] = p2p([v[i] - truth[i] for i in idx]) / old_vs_truth
        if c["family"]:
            ref = refs.get(c["family"], name)
            old_vs_ref = p2p([old[i] - refs.get(c["family"], "old")[i] for i in idx])
            entry["family_reference_p2p"] = p2p([ref[i] for i in idx])
            entry["N8R_ratio_to_old"] = p2p([v[i] - ref[i] for i in idx]) / old_vs_ref
        section[name] = entry
    section["reference_precision"] = refs.precision
    return section


def continuous_convergence():
    """Base (24 x 64 directions, 32 steps) versus fine (32 x 96, 48) continuous
    limits: largest change of any filtered N6 step and of the raw N8 p2p."""
    out = {}
    for orthographic in (False, True):
        tag = "ortho" if orthographic else "persp"
        g = m.Grid("white_slab", orthographic)
        pairs, i = n6_pairs(g), (lambda x: N6_ROW * g.w + x)
        span = {i(x) for x in range(N6_X[0], N6_X[1] + 1)}
        support = m.support(g, span, 2)
        steps = []
        for grid_spec in (CONTINUOUS, CONTINUOUS_FINE):
            raw = m.raw_directions(g, support, *grid_spec)
            f = m.filtered(g, [raw.get(k) for k in range(len(g.hits))], quantize=False, pixels=span)
            steps.append({x: f[i(x + 1)] - f[i(x)] for x in pairs})
        out[f"N6_{tag}_filtered_step_change"] = max(abs(steps[0][x] - steps[1][x]) for x in pairs)
        out[f"N6_{tag}_fine_filtered_max_step"] = max(abs(v) for v in steps[1].values())
        s = m.Grid(STRIPES, orthographic)
        row, cols = n8_roi(s)
        idx = [row * s.w + x for x in cols]
        p2p = []
        for grid_spec in (CONTINUOUS, CONTINUOUS_FINE):
            raw = m.raw_directions(s, idx, *grid_spec)
            p2p.append(max(raw[k] for k in idx) - min(raw[k] for k in idx))
        out[f"N8_{tag}_p2p_base_fine"] = p2p
    return out


def n8_rows(orthographic, extra=6):
    """Diagnostic only (not the N8 ROI): the same window on the N8 row and the
    next `extra` rows, for the shipped, eeeac1d4 and 8-step outputs."""
    g = m.Grid(STRIPES, orthographic)
    row, cols = n8_roi(g)
    p2p = lambda vals: max(vals) - min(vals)  # noqa: E731
    out = {}
    for r in range(row, row + extra + 1):
        idx = [r * g.w + x for x in cols]
        old = output(g, "old", set(idx))
        base = p2p([old[i] for i in idx])
        truth = m.raw_directions(g, idx, *CONTINUOUS)
        entry = {"old_p2p_255": round(base * 255, 2), "continuous_ratio": p2p([truth[i] for i in idx]) / base}
        for name in ("design", "steps8"):
            v = output(g, name, set(idx))
            entry[f"{name}_ratio"] = p2p([v[i] for i in idx]) / base
        out[str(r)] = entry
    return out


def n7_section():
    out, reproduced = {}, True
    for (name, ortho), logged in ASTRA.items():
        g = m.Grid(name, ortho)
        values = {c: output(g, c) for c in CONFIGS}
        counts = {c: m.n7_steps(g, v) for c, v in values.items()}
        n6 = {c: m.n6_row(g, values[c])[0] for c in ("old", "design", "design_no_jitter")}
        mine = {"old_N7": counts["old"], "design_N7": counts["design"],
                "design_no_rotation_N7": counts["design_no_rotation"],
                "old_N6": n6["old"], "design_N6": n6["design"], "design_no_jitter_N6": n6["design_no_jitter"]}
        reproduced &= all(abs(mine[k] - logged[k]) < 1e-12 for k in logged)
        out[f"{name}_{'ortho' if ortho else 'persp'}"] = counts
    out["astra_log_reproduced"] = reproduced
    return out


def n7_attribution():
    """Previous (2026-10-13) definition on the interval oracle versus the exact one."""
    sys.path.insert(1, os.path.join(HERE, "..", "tools"))
    import analyze as a  # read-only; -B keeps tools/ unchanged
    result = {}
    for name, ortho in (("white_slab", False), ("white_slab", True), ("edge_slab", False)):
        g = m.Grid(name, ortho)
        old, w = output(g, "old"), g.w
        inside = {(x, y): g.interior(x, y) for y in range(g.h) for x in range(g.w)}
        exact = {(x, y) for y in range(g.h - 1) for x in range(g.w - 1)
                 if inside[(x, y)] and inside[(x + 1, y)] and inside[(x, y + 1)]
                 and max(abs(old[y * w + x] - old[y * w + x + 1]), abs(old[y * w + x] - old[(y + 1) * w + x])) >= 1 / 16}
        scene = a.ref.numeric_scene(name, ortho)
        iv = a.reference_intervals(scene)
        depth = {}
        for p in a.INTERIOR:
            c = scene.candidates(p)
            depth[p] = None if not c or c[0] is None else round(c[0][0][2], 6)
        near = lambda x, y: any(depth.get((x + dx, y + dy), depth[(x, y)]) != depth[(x, y)]  # noqa: E731
                                for dx in range(-2, 3) for dy in range(-2, 3))
        A = {p: 1 - v[0] / 16 for p, v in iv.items() if v is not None and v[0] == v[1]}
        previous = {(x, y) for (x, y), val in A.items()
                    if any(q is not None and abs(q - val) >= 1 / 16 - 1e-12 for q in (A.get((x + 1, y)), A.get((x, y + 1))))
                    and not near(x, y)}
        only_prev, only_exact = previous - exact, exact - previous
        result[f"{name}_{'ortho' if ortho else 'persp'}"] = {
            "previous_definition": len(previous), "exact_definition": len(exact),
            "only_previous_forward_neighbour_in_band": dict(Counter(
                "right" if not inside[(x + 1, y)] else "lower" if not inside[(x, y + 1)] else "other"
                for x, y in only_prev)),
            "only_exact_interval_inexact": sum(1 for x, y in only_exact
                                               if (x, y) not in A or (x + 1, y) not in A or (x, y + 1) not in A),
            "only_exact_other": sum(1 for x, y in only_exact
                                    if (x, y) in A and (x + 1, y) in A and (x, y + 1) in A),
            "interval_inexact_pixels": sum(1 for v in iv.values() if v is not None and v[0] != v[1]),
            "model_disagreements_on_exact_pixels": sum(1 for (x, y), v in A.items() if abs(old[y * w + x] - v) > 1e-12),
        }
    return result


def main():
    start = time.time()
    report = {
        "format": "incant-0070-ssao-quality-addendum-cpu-v3",
        "model": "ssao-repair/quality_model.py (f64, exact f32 uniform inputs; not a GPU oracle)",
        "configs": CONFIGS,
        "families": {k: {"full_stratum": v[0], "steps": v[1]} for k, v in FAMILIES.items()},
        "strata": {"reference": STRATA, "precision_check": CHECK_STRATA},
        "continuous_limit": {"directions": "24 x 64 cosine-weighted midpoint grid", "steps": 32,
                             "convergence_check": "32 x 96 directions, 48 steps"},
        "N7": n7_section(),
        "N7_attribution": n7_attribution(),
        "N6": {"persp": n6_section(False), "ortho": n6_section(True)},
        "N8": {"fixture": STRIPES, "persp": n8_section(False), "ortho": n8_section(True)},
        "N8_adjacent_rows_diagnostic": {"persp": n8_rows(False), "ortho": n8_rows(True)},
        "continuous_convergence": continuous_convergence(),
    }
    report["seconds"] = round(time.time() - start, 1)
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")
    return 0 if report["N7"]["astra_log_reproduced"] else 1


if __name__ == "__main__":
    sys.exit(main())
