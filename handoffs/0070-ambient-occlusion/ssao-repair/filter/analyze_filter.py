#!/usr/bin/env python3
"""Offline analysis for a future R1/R3 filter run (ssao-filter-preparation.md).

CPU only, standard library only. Reuses tools/analyze.py and tools/ssao_reference.py
read-only (both pinned by build004) and the filtered interval oracle in oracle.py.

Numeric gates (no quality judgement):
* filtered reference gates: at every viewport-interior pixel, the accessibility
  interval inferred from the AO-on / white / black output frames (the unchanged
  +/-0.75-code tone inversion, plus only the f16 radiance term; the R8 term is
  now an explicit oracle stage) must meet the oracle's final R8 code interval.
  Full coverage, no invalid inference, at least OCCLUSION_FLOOR certainly
  occluded pixels (final codes <= 254), exact 255 for absence controls, and the
  oracle's own width bound wherever no decision in a pixel's support is
  ambiguous;
* stage gates (when the run retains the proposed R8 stage planes): every
  viewport texel's AO-pass code lies in the oracle's raw interval; every H code
  lies in the H interval computed from the READ-BACK raw plane; every V code in
  the V interval from the READ-BACK H plane; every V code in the oracle's final
  interval; the output frames' inferred interval contains the V code; texels
  outside the viewport keep the white clear (255) in all three planes;
* identity, mixed BLEND pane and DRS gates exactly as in tools/analyze.py, plus
  the isolated 24 x 48 sphere identity (preconditions: the sphere's largest
  facet rise is below the plane bias, and the sphere covers the projected area).

Quality gates (no waiver): N6 as authored, N7 exact, N8 exact ROI, computed on
the read-back final codes, so they need the stage planes. N6T is reported as an
additional diagnostic and does not gate.

Fail-closed: every declared case, frame, plane and gate must be present; the
report is written once and never replaced; exit 0 only when every gate passes.

  python3 -I -B analyze_filter.py <evidence-dir>   # writes <evidence-dir>/filter-analysis.json
  python3 -I -B analyze_filter.py --selftest
"""

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPAIR = HERE.parent
TOOLS = REPAIR.parent / "tools"
sys.path[:0] = [str(HERE), str(REPAIR), str(TOOLS)]
import analyze as base  # noqa: E402  (tools/analyze.py, read-only)
import oracle as orc  # noqa: E402
import quality_model as qm  # noqa: E402
import quality_report as qr  # noqa: E402
import sphere_rise_bound as srb  # noqa: E402

CASES = json.loads((HERE / "cases.json").read_text())
INTERIOR, RECT, SIZE = base.INTERIOR, base.RECT, base.SIZE
F16_SLACK = 0.002  # forward f16 radiance only (tools/analyze AO_SLACK minus its R8 term)
OCCLUSION_FLOOR = base.OCCLUSION_FLOOR
REJECTIONS = base.REJECTIONS
CORE, MATERIALS, GEOMETRY, LIFECYCLE = base.CORE, base.MATERIALS, base.GEOMETRY, base.LIFECYCLE
QUALITY = "case-ssao_quality"
N7_SHIPPED = {"white_slab_persp": 1382, "white_slab_ortho": 1767, "edge_slab_persp": 2027}
SPHERE_AREA_TOLERANCE = 0.1


def declared_cases():
    return [f"case-{c['name']}" for c in CASES["numeric_cases"]]


def stage_frames():
    return {(f"case-{c['name']}", label) for c in CASES["numeric_cases"] for label in c["stage_frames"]}


class Missing(Exception):
    """A declared case, frame or stage plane is absent or invalid."""


def load_stages(directory):
    """R8 stage planes of one output directory, hash- and size-verified."""
    index = json.loads((directory / "index.json").read_text())
    planes = {}
    for entry in index["frames"]:
        if not entry["file"].endswith(".r8"):
            continue
        data = (directory / entry["file"]).read_bytes()
        width, height = entry["size"]
        if hashlib.sha256(data).hexdigest() != entry["r8_sha256"]:
            raise ValueError(f"{entry['file']}: content does not match its recorded hash")
        if len(data) != width * height:
            raise ValueError(f"{entry['file']}: byte length does not match its recorded size")
        meta = entry["meta"]
        if meta.get("stage") not in ("raw", "h", "v"):
            raise ValueError(f"{entry['file']}: unknown stage {meta.get('stage')!r}")
        planes.setdefault(meta["frame"], {})[meta["stage"]] = ((width, height), data)
    return planes


class Evidence:
    def __init__(self, root):
        self.root = Path(root)
        self.cases, self.planes, self.status = {}, {}, {}
        for case in declared_cases():
            directory = self.root / "outputs" / case
            if not (directory / "index.json").is_file():
                self.status[case] = {"status": "missing", "reason": f"{directory / 'index.json'} absent"}
                continue
            try:
                self.cases[case] = base.load_output(directory)
                self.planes[case] = load_stages(directory)
            except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
                self.status[case] = {"status": "invalid", "reason": f"{type(error).__name__}: {error}"}
                self.cases.pop(case, None)
                continue
            if not self.cases[case]:
                self.status[case] = {"status": "invalid", "reason": "index.json lists no RGBA frame"}
                del self.cases[case]
                continue
            self.status[case] = {"status": "loaded", "frames": len(self.cases[case]),
                                 "stage_frames": len(self.planes[case])}

    def frame(self, case, label):
        if case not in self.cases:
            state = self.status.get(case, {"status": "undeclared", "reason": "not a declared numeric case"})
            raise Missing(f"{case} {state['status']}: {state['reason']}")
        if label not in self.cases[case]:
            raise Missing(f"{case}: frame {label} absent from index.json")
        return self.cases[case][label]

    def stages(self, case, label):
        self.frame(case, label)
        found = self.planes[case].get(label, {})
        absent = [s for s in ("raw", "h", "v") if s not in found]
        if absent:
            raise Missing(f"{case}: R8 stage planes {absent} of {label} absent")
        for stage, (size, _) in found.items():
            if tuple(size) != SIZE:
                raise Missing(f"{case}: {label} {stage} plane is {size}, expected {SIZE}")
        return {stage: data for stage, (_, data) in found.items()}


# ------------------------------------------------------------------ oracles (cached per geometry)

_ORACLES = {}


def oracle_for(scene_name, orthographic, strength):
    scene = orc.scene(scene_name, orthographic)
    key = (orthographic, tuple((r.x, r.y, r.z, r.normal) for r in scene.rects), scene.edges, strength)
    if key not in _ORACLES:
        oracle = orc.FilterOracle(scene, strength)
        finals = {p: oracle.v(*p) for p in INTERIOR}
        _ORACLES[key] = (oracle, finals, oracle.summary())
    return _ORACLES[key]


def infer_accessibility(on, off, spec):
    """tools/analyze.infer_k's radiometry, returning accessibility, without the R8 term."""
    try:
        on_b, off_b, spec_b = map(base.scene_intervals, (on, off, spec))
    except ValueError:
        return None, "radiometry"
    lo, hi = -math.inf, math.inf
    for c in range(3):
        d_lo, d_hi = off_b[c][0] - spec_b[c][1], off_b[c][1] - spec_b[c][0]
        if not d_lo > base.LOW_SIGNAL:
            return None, "low_signal"
        a_lo = (on_b[c][0] - spec_b[c][1]) / d_hi - F16_SLACK
        a_hi = (on_b[c][1] - spec_b[c][0]) / d_lo + F16_SLACK
        if not (math.isfinite(a_lo) and math.isfinite(a_hi)):
            return None, "nonfinite"
        if a_lo > a_hi:
            return None, "inverted"
        lo, hi = max(lo, a_lo), min(hi, a_hi)
    if not lo <= hi:
        return None, "empty"
    return (lo, hi), None


def filtered_agreement(on, off, spec, oracle, finals, expects_occlusion, preconditions):
    stats = {"interior_pixels": len(INTERIOR), "compared": 0, "agree": 0, "disagree": 0, "uninferable": 0,
             "rejected": dict.fromkeys(REJECTIONS, 0), "out_of_range": 0, "certainly_occluded": 0,
             "occluded_compared": 0, "not_exactly_white": 0, "undecisive_pixels": 0, "decisive_too_wide": 0,
             "first_disagreements": [], "first_rejections": []}
    for x, y in INTERIOR:
        c_lo, c_hi = finals[(x, y)]
        stats["certainly_occluded"] += c_hi <= 254
        stats["not_exactly_white"] += (c_lo, c_hi) != (255, 255)
        if oracle.decisive(x, y):
            stats["decisive_too_wide"] += c_hi - c_lo > orc.DECISIVE_WIDTH
        else:
            stats["undecisive_pixels"] += 1
        inferred, reason = infer_accessibility(on.px(x, y), off.px(x, y), spec.px(x, y))
        if inferred is None:
            stats["uninferable"] += 1
            stats["rejected"][reason] += 1
            if len(stats["first_rejections"]) < 12:
                stats["first_rejections"].append({"pixel": [x, y], "reason": reason})
            continue
        stats["compared"] += 1
        stats["occluded_compared"] += c_hi <= 254
        if inferred[1] < -0.01 or inferred[0] > 1.01:
            stats["out_of_range"] += 1
        if inferred[1] >= c_lo / 255 - 1e-9 and inferred[0] <= c_hi / 255 + 1e-9:
            stats["agree"] += 1
        else:
            stats["disagree"] += 1
            if len(stats["first_disagreements"]) < 12:
                stats["first_disagreements"].append({"pixel": [x, y], "oracle_codes": [c_lo, c_hi],
                                                     "inferred_codes": [round(inferred[0] * 255, 2),
                                                                        round(inferred[1] * 255, 2)]})
    stats["preconditions"] = dict(preconditions)
    problems = [f"precondition failed: {name}" for name, ok in preconditions.items() if ok is not True]
    if stats["compared"] < len(INTERIOR):
        problems.append(f"coverage {stats['compared']} of {len(INTERIOR)} required interior pixels")
    invalid = {k: v for k, v in stats["rejected"].items() if v and k != "low_signal"}
    if invalid:
        problems.append(f"invalid inferences {invalid}")
    if stats["disagree"]:
        problems.append(f"{stats['disagree']} pixels disagree with the filtered oracle")
    if stats["out_of_range"]:
        problems.append(f"{stats['out_of_range']} inferred intervals outside [0, 1]")
    if stats["decisive_too_wide"]:
        problems.append(f"{stats['decisive_too_wide']} decisive pixels wider than {orc.DECISIVE_WIDTH} codes")
    if expects_occlusion:
        if stats["certainly_occluded"] < OCCLUSION_FLOOR:
            problems.append(f"only {stats['certainly_occluded']} certainly occluded pixels (< {OCCLUSION_FLOOR})")
        elif stats["occluded_compared"] < stats["certainly_occluded"]:
            problems.append(f"{stats['occluded_compared']} of {stats['certainly_occluded']} occluded pixels compared")
    elif stats["not_exactly_white"]:
        problems.append(f"absence control, but {stats['not_exactly_white']} pixels are not exactly 255")
    stats["problems"] = problems
    stats["pass"] = not problems
    return stats


def stage_agreement(planes, oracle, finals, frames):
    """Each GPU stage against the oracle on that stage's own read-back inputs."""
    raw, h, v = planes["raw"], planes["h"], planes["v"]
    width = SIZE[0]
    counts = {"viewport_texels": 0, "raw_outside": 0, "h_outside": 0, "v_outside": 0, "v_outside_final": 0,
              "clear_not_white": 0, "forward_outside": 0, "forward_uninferable": 0}
    first = []
    raw_source, h_source = orc.plane_source(raw), orc.plane_source(h)
    for y in range(SIZE[1]):
        for x in range(SIZE[0]):
            i = y * width + x
            if not oracle.inside(x, y):
                counts["clear_not_white"] += (raw[i], h[i], v[i]) != (255, 255, 255)
                continue
            counts["viewport_texels"] += 1
            checks = (("raw_outside", raw[i], oracle.raw(x, y)),
                      ("h_outside", h[i], orc.r8_codes(*oracle.pass_value(x, y, True, raw_source))),
                      ("v_outside", v[i], orc.r8_codes(*oracle.pass_value(x, y, False, h_source))))
            for name, code, (lo, hi) in checks:
                if not lo <= code <= hi:
                    counts[name] += 1
                    if len(first) < 12:
                        first.append({"texel": [x, y], "stage": name, "code": code, "interval": [lo, hi]})
    on, off, spec = frames
    for x, y in INTERIOR:
        code = v[y * width + x]
        lo, hi = finals[(x, y)]
        counts["v_outside_final"] += not lo <= code <= hi
        inferred, _ = infer_accessibility(on.px(x, y), off.px(x, y), spec.px(x, y))
        if inferred is None:
            counts["forward_uninferable"] += 1
        elif not inferred[0] - 1e-9 <= code / 255 <= inferred[1] + 1e-9:
            counts["forward_outside"] += 1
    problems = [f"{k} {n}" for k, n in counts.items() if n and k != "viewport_texels"]
    return {**counts, "first_failures": first, "problems": problems, "pass": not problems}


# ------------------------------------------------------------------ gates

REFERENCE_GATES = [
    ("ssao_core_p_filtered", CORE, "p_ao_white", "p_off_white", "p_off_black", "white_slab", False, 1.0, True),
    ("ssao_core_o_filtered", CORE, "o_ao_white", "o_off_white", "o_off_black", "white_slab", True, 1.0, True),
    ("ssao_core_strength_0_5_filtered", CORE, "p_ao_strength_0_5", "p_off_white", "p_off_black", "white_slab",
     False, 0.5, True),
    ("ssao_core_edge_filtered", CORE, "p_ao_edge", "p_off_edge", "p_off_black", "edge_slab", False, 1.0, True),
    *[(f"ssao_geometry_{label}_filtered", GEOMETRY, label, "g_off_white", "g_off_black", scene, False, 1.0, occludes)
      for label, scene, occludes in (
          ("g_ao_white", "white_slab", True), ("g_ao_mask_equal", "mask_equal", True),
          ("g_ao_mask_cutout", "mask_cutout", True), ("g_ao_backface", "double_sided_back_receiver", True),
          ("g_ao_mirrored", "mirrored_receiver", True), ("g_ao_skin_t0", "baked_slab_far", False),
          ("g_ao_skin_t1", "baked_slab_near", True), ("g_ao_motion_far", "far_slab", False),
          ("g_ao_motion_near", "white_slab", True))],
    ("ssao_quality_p_stripes_filtered", QUALITY, "p_ao_stripes", "p_off_stripes", "p_off_stripes_black",
     "stripes", False, 1.0, True),
    ("ssao_quality_o_stripes_filtered", QUALITY, "o_ao_stripes", "o_off_stripes", "o_off_stripes_black",
     "stripes", True, 1.0, True),
]
IDENTITY_GATES = [*base.IDENTITY_GATES,
                  ("ssao_quality_p_sphere_identity", QUALITY, "p_ao_sphere", QUALITY, "p_off_sphere"),
                  ("ssao_quality_o_sphere_identity", QUALITY, "o_ao_sphere", QUALITY, "o_off_sphere")]


def reference_gate(case, on, off, spec, scene_name, orthographic, strength, occludes):
    def gate(ev):
        frames = [ev.frame(case, label) for label in (on, off, spec)]
        oracle, finals, summary = oracle_for(scene_name, orthographic, strength)
        result = filtered_agreement(*frames, oracle, finals, occludes,
                                    base.basis_contract(frames, oracle.scene))
        result["oracle_decisions"] = summary
        return result
    return gate


def stage_gate(case, on, off, spec, scene_name, orthographic, strength):
    def gate(ev):
        planes = ev.stages(case, on)
        frames = [ev.frame(case, label) for label in (on, off, spec)]
        oracle, finals, _ = oracle_for(scene_name, orthographic, strength)
        return stage_agreement(planes, oracle, finals, frames)
    return gate


def sphere_area(orthographic):
    """Projected texel area of the radius-0.6 sphere at view z = -3 (silhouette disc)."""
    (cx, cy, cz), scale = orc.SPHERE_PLACE
    r = 0.5 * scale
    if orthographic:
        pixels_per_unit = RECT[3] / 8
        radius_px = r * pixels_per_unit
    else:
        d = math.sqrt(cx * cx + cy * cy + cz * cz)
        radius_px = math.tan(math.asin(r / d)) * RECT[3] / 2 / math.tan(0.5)
    return math.pi * radius_px ** 2


def identity_gate(case_a, a, case_b, b):
    plain = base.identity_gate(case_a, a, case_b, b)
    if not a.endswith("_sphere"):
        return plain

    def gate(ev):
        result = plain(ev)
        off = ev.frame(case_b, b)
        corner = off.px(int(RECT[0] + 3), int(RECT[1] + 3))
        covered = sum(1 for x, y in INTERIOR if off.px(x, y) != corner)
        expected = sphere_area(a.startswith("o_"))
        rise = srb.column_bound(*srb.mesh(), 16)[0]
        preconditions = {"facet_rise_below_bias": rise < orc.BIAS,
                         "sphere_covers_projected_area": abs(covered - expected) <= SPHERE_AREA_TOLERANCE * expected}
        result.update({"sphere_texels": covered, "expected_area": round(expected, 1), "facet_rise": rise,
                       "preconditions": preconditions})
        result["pass"] = result["pass"] and all(preconditions.values())
        return result
    return gate


def final_values(ev, case, label):
    """Read-back final codes as accessibility values for the quality metrics."""
    return [c / 255 for c in ev.stages(case, label)["v"]]


def n6_gate(ev):
    grid = qm.Grid("white_slab", False)
    value, at = qm.n6_row(grid, final_values(ev, CORE, "p_ao_white"))
    return {"max_step": value, "max_step_255": round(value * 255, 2), "at": at, "bound": 1 / 64,
            "pass": value <= 1 / 64}


_TRUTH = {}


def n6t_diagnostic(ev):
    grid = qm.Grid("white_slab", False)
    i = lambda x: qr.N6_ROW * grid.w + x  # noqa: E731
    span = {i(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)}
    if "row" not in _TRUTH:
        raw = qm.raw_directions(grid, qm.support(grid, span), *qr.CONTINUOUS)
        _TRUTH["row"] = qm.filtered(grid, [raw.get(k) for k in range(len(grid.hits))], quantize=False, pixels=span)
    truth, values = _TRUTH["row"], final_values(ev, CORE, "p_ao_white")
    residual = max(abs((values[i(x + 1)] - values[i(x)]) - (truth[i(x + 1)] - truth[i(x)]))
                   for x in qr.n6_pairs(grid))
    return {"diagnostic_only": True, "step_residual": residual, "step_residual_255": round(residual * 255, 2),
            "bound": 1 / 64, "within_bound": residual <= 1 / 64}


def n7_gate(name, orthographic, case, label, key):
    def gate(ev):
        grid = qm.Grid(name, orthographic)
        count = qm.n7_steps(grid, final_values(ev, case, label))
        shipped = qm.n7_steps(grid, qm.raw(grid, rotate=False, jitter=False))
        if shipped != N7_SHIPPED[key]:
            return {"pass": False, "error": f"shipped baseline {shipped} != adopted {N7_SHIPPED[key]}"}
        return {"count": count, "shipped": shipped, "bound": shipped / 10, "pass": count <= shipped / 10}
    return gate


def n8_gate(orthographic, label):
    def gate(ev):
        grid = qm.Grid(qr.STRIPES, orthographic)
        row, cols = qr.n8_roi(grid)
        window = [row * grid.w + x for x in cols]
        old = qm.raw(grid, rotate=False, jitter=False, pixels=set(window))
        values = final_values(ev, QUALITY, label)
        shipped = max(old[i] for i in window) - min(old[i] for i in window)
        p2p = max(values[i] for i in window) - min(values[i] for i in window)
        return {"row": row, "cols": [cols[0], cols[-1]], "p2p_255": round(p2p * 255, 2),
                "shipped_255": round(shipped * 255, 2), "ratio": p2p / shipped, "bound": 0.25,
                "pass": p2p <= 0.25 * shipped}
    return gate


STAGED = stage_frames()
NUMERIC_GATES = {
    **{name: reference_gate(*spec) for name, *spec in REFERENCE_GATES},
    **{f"{name}_stages": stage_gate(*spec[:7]) for name, *spec in REFERENCE_GATES if (spec[0], spec[1]) in STAGED},
    **{name: identity_gate(*spec) for name, *spec in IDENTITY_GATES},
    "mixed_blend_pane_scales_receiver_change": base.mixed_pane_gate,
    "drs_ssao_nearest_expansion_12": base.expansion_gate(12),
    "drs_ssao_nearest_expansion_8": base.expansion_gate(8),
    "drs_bloom_halo_centroid_on_physical_emitter": base.drs_centroid_gate,
}
QUALITY_GATES = {
    "quality_n6_authored": n6_gate,
    "quality_n7_white_slab_persp": n7_gate("white_slab", False, CORE, "p_ao_white", "white_slab_persp"),
    "quality_n7_white_slab_ortho": n7_gate("white_slab", True, CORE, "o_ao_white", "white_slab_ortho"),
    "quality_n7_edge_slab_persp": n7_gate("edge_slab", False, CORE, "p_ao_edge", "edge_slab_persp"),
    "quality_n8_persp": n8_gate(False, "p_ao_stripes"),
    "quality_n8_ortho": n8_gate(True, "o_ao_stripes"),
}


def run_gate(function, ev):
    try:
        result = function(ev)
    except Missing as missing:
        return {"pass": False, "missing": str(missing)}
    except Exception as error:  # a malformed input fails its gate; the report still records it
        return {"pass": False, "error": f"{type(error).__name__}: {error}"}
    result["pass"] = result.get("pass") is True
    return result


def analyse(evidence):
    ev = Evidence(evidence)
    numeric = {name: run_gate(f, ev) for name, f in NUMERIC_GATES.items()}
    quality = {name: run_gate(f, ev) for name, f in QUALITY_GATES.items()}
    diagnostics = {"n6t": run_gate(n6t_diagnostic, ev)}
    missing = sorted(c for c, s in ev.status.items() if s["status"] != "loaded")
    failed_numeric = sorted(n for n, g in numeric.items() if not g["pass"])
    failed_quality = sorted(n for n, g in quality.items() if not g["pass"])
    return {"format": "incant-0070-ssao-filter-analysis-v1", "evidence": str(evidence),
            "declared_cases": declared_cases(), "cases": ev.status, "missing_or_invalid_cases": missing,
            "numeric_gates": numeric, "quality_gates": quality, "diagnostics": diagnostics,
            "failed_numeric_gates": failed_numeric, "failed_quality_gates": failed_quality,
            "numeric_pass": not missing and not failed_numeric,
            "quality_pass": not missing and not failed_quality,
            "appearance": {"judged_here": False, "note": "Claude's rendered-pair review is separate."},
            "pass": not missing and not failed_numeric and not failed_quality}


def main(argv=None):
    """Exit status: 0 every gate passed, 1 a gate failed (report written), 2 usage."""
    argv = sys.argv[1:] if argv is None else argv
    if argv == ["--selftest"]:
        import controls
        print(json.dumps(controls.run(), indent=1))
        return 0
    if len(argv) != 1 or not Path(argv[0]).is_dir():
        print(__doc__, file=sys.stderr)
        return 2
    out = Path(argv[0]) / "filter-analysis.json"
    if out.exists():
        print(f"{out} exists; reports are never replaced", file=sys.stderr)
        return 2
    report = analyse(Path(argv[0]))
    with open(out, "x") as handle:
        json.dump(report, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({key: report[key] for key in ("pass", "numeric_pass", "quality_pass", "missing_or_invalid_cases",
                                                   "failed_numeric_gates", "failed_quality_gates")}, indent=1))
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
