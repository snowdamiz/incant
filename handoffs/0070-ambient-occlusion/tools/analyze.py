#!/usr/bin/env python3
"""Offline analysis of retained 0070 readbacks. CPU only, standard library only.

Reads the helper outputs under <evidence>/outputs/<label>/ (index.json + raw
RGBA), verifies every file against its recorded hash, and derives:

* SSAO: per receiver pixel, an interval for the blocked-ray count k from the
  AO-on frame, the AO-off frame and the specular-only (diffuse factor 0) frame:
  on = D*AO + S, off = D + S, S from the black frame, with +/-0.75 sRGB code,
  R8 accessibility and f16 radiance quantization folded into the interval.
  Inference is rejected (never compared) when the diffuse signal is too small,
  a channel bound is not finite, a channel interval is inverted, or the RGB
  intervals do not intersect. The result must meet the independent reference
  interval (ssao_reference.py) and lie inside [0, 16].
* DRS: reduced-scale frames equal the nearest expansion of internal-size
  references away from viewport edges (0067 texel rule).
* Bloom: zero-alpha and below-threshold emitters change nothing; fractional
  coverage scales halo energy; a constant field matches the tone-curve
  prediction at 1/4/6 levels; DRS halo centroids stay on the physical emitter.
* Determinism: identical scene/renderer/route frames match across children.

Fail-closed: every numeric case declared in plan.json and every gate in GATES
must be present and pass. A missing case, missing frame or invalid index fails
the gates that need it, with the reason retained in the report. The report is
always written (never replaced); the exit status is 0 only when it passes.

Minimum SSAO coverage is fixed by the frozen fixture geometry, not by results:
the 16 x 12 receiver at z = -4 covers the whole viewport interior in both
projections (visible half-extents at z = -4: 3.82 x 2.19 perspective, 6.99 x
4.00 orthographic); the reference defines every interior pixel of every
declared scene (it finds no background, silhouette or cut-edge ambiguity at
any interior texel centre); and the white material's diffuse term (about 0.67
linear) is some 30 times the 0.02 inference floor. A faithful renderer leaves
no pixel uninferable, so every interior pixel must be compared. Occlusion gates
must also compare at least OCCLUSION_FLOOR pixels the reference marks occluded
(a guard against a vacuous reference; the slab edge 1 unit in front of the
receiver occludes a band along its whole projected length), and absence
controls must have none.

Every SSAO reference gate reuses off/specular bases: the specular basis is
always a separately rendered black scene and the geometry gates share one white
off frame. Those bases are only valid when they are uniform (the white-furnace
precondition), so uniformity of both bases is an enforced precondition.

Numerical invariants only. The appearance PNGs are not judged here; Claude's
appearance review is a separate, later judgment.

No golden image or aesthetic threshold is used.

  python3 -I -B analyze.py <evidence-dir>      # writes <evidence-dir>/analysis.json
  python3 -I -B analyze.py --selftest
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
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import ssao_reference as ref  # noqa: E402

RECT = (13.25, 17.5, 280.5, 160.5)
SIZE = (321, 193)
QUANTIZATION_CODES = 0.75
AO_SLACK = 0.5 / 255 + 0.002  # R8 accessibility + f16 radiance, generous
LOW_SIGNAL = 0.02             # linear diffuse signal below which AO cannot be inferred
MIN_COVERAGE_FRACTION = 1.0   # of the viewport interior; justified in the module docstring
OCCLUSION_FLOOR = 1000        # reference-occluded pixels an occlusion gate must compare
REJECTIONS = ("low_signal", "nonfinite", "inverted", "empty")


def srgb_to_linear(s):
    s = min(max(s, 0.0), 1.0)
    return s / 12.92 if s <= 0.04045 else ((s + 0.055) / 1.055) ** 2.4


def linear_to_srgb(v):
    v = min(max(v, 0.0), 1.0)
    return 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055


LINEAR = [srgb_to_linear(b / 255) for b in range(256)]


def code_interval(byte):
    return (srgb_to_linear((byte - QUANTIZATION_CODES) / 255), srgb_to_linear((byte + QUANTIZATION_CODES) / 255))


def tone(r):
    """The documented peak-preserving display curve (bloom reference.rs)."""
    peak = max(r)
    if peak < 0.8:
        return list(r)
    new_peak = 1 - 0.04 / (peak - 0.6)
    ratio = new_peak / peak
    scaled = [v * ratio for v in r]
    g = 1 - 1 / (0.15 * (peak - new_peak) + 1)
    return [s * (1 - g) + new_peak * g for s in scaled]


class Frame:
    def __init__(self, size, data, meta):
        self.size, self.data, self.meta = size, data, meta

    def px(self, x, y):
        i = (y * self.size[0] + x) * 4
        return self.data[i:i + 4]


def load_output(directory):
    """Raw frames of one helper output directory, hash-verified against index.json."""
    index = json.loads((directory / "index.json").read_text())
    frames = {}
    for entry in index["frames"]:
        if not entry["file"].endswith(".rgba"):
            continue  # PNGs belong to the appearance review
        path = directory / entry["file"]
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != entry["rgba_sha256"]:
            raise ValueError(f"{path}: content does not match its recorded hash")
        if len(data) != entry["size"][0] * entry["size"][1] * 4:
            raise ValueError(f"{path}: byte length does not match its recorded size")
        frames[entry["label"]] = Frame(tuple(entry["size"]), data, entry.get("meta", {}))
    return frames


def interior(rect, size, margin=1.5):
    x0, y0 = math.ceil(rect[0] + margin), math.ceil(rect[1] + margin)
    x1, y1 = math.floor(rect[0] + rect[2] - margin), math.floor(rect[1] + rect[3] - margin)
    for y in range(max(y0, 0), min(y1, size[1])):
        for x in range(max(x0, 0), min(x1, size[0])):
            yield x, y


INTERIOR = list(interior(RECT, SIZE))
_REFERENCE = {}


def reference_intervals(scene, radius=2.0, unlimited=False):
    """Reference interval per interior pixel, computed once per geometry."""
    key = (scene.camera.orthographic, scene.camera.rect, scene.edges, radius, unlimited,
           tuple((r.x, r.y, r.z, r.normal) for r in scene.rects))
    if key not in _REFERENCE:
        _REFERENCE[key] = {p: ref.pixel_interval(scene, p, radius=radius, unlimited=unlimited) for p in INTERIOR}
    return _REFERENCE[key]


def infer_k(on, off, spec, strength):
    """Blocked-ray interval from three RGBA pixels, intersected over RGB.

    Returns (interval, None) or (None, reason) with reason in REJECTIONS:
    too little diffuse signal, a non-finite channel bound, an inverted channel
    interval, or channel intervals that do not intersect.
    """
    if not (math.isfinite(strength) and strength > 0):
        return None, "nonfinite"
    lo_k, hi_k = -math.inf, math.inf
    for c in range(3):
        on_lo, on_hi = code_interval(on[c])
        off_lo, off_hi = code_interval(off[c])
        s_lo, s_hi = code_interval(spec[c])
        d_lo, d_hi = off_lo - s_hi, off_hi - s_lo
        if not d_lo > LOW_SIGNAL:
            return None, "low_signal"
        ao_lo = (on_lo - s_hi) / d_hi - AO_SLACK
        ao_hi = (on_hi - s_lo) / d_lo + AO_SLACK
        k_lo, k_hi = 16 * (1 - ao_hi) / strength, 16 * (1 - ao_lo) / strength
        if not (math.isfinite(k_lo) and math.isfinite(k_hi)):
            return None, "nonfinite"
        if k_lo > k_hi:
            return None, "inverted"
        lo_k, hi_k = max(lo_k, k_lo), min(hi_k, k_hi)
    if not lo_k <= hi_k:
        return None, "empty"
    return (lo_k, hi_k), None


def furnace(frame, rect=RECT):
    values = {tuple(frame.px(x, y)[:3]) for x, y in interior(rect, frame.size)}
    return {"distinct_rgb_values": len(values), "uniform": len(values) == 1, "values": sorted(values)[:4]}


def ssao_agreement(on, off, spec, scene, strength=1.0, radius=2.0, expects_occlusion=True, preconditions=None):
    """Compare inferred intervals with the reference over the viewport interior.

    Passes only with every precondition met, full coverage, no rejected or
    out-of-range inference, no disagreement, and the occlusion the fixture
    design declares (at least OCCLUSION_FLOOR occluded comparisons, or none).
    """
    reference = reference_intervals(scene, radius)
    stats = {"interior_pixels": len(INTERIOR), "pixels": 0, "compared": 0, "agree": 0, "disagree": 0,
             "uninferable": 0, "rejected": dict.fromkeys(REJECTIONS, 0), "out_of_range": 0,
             "reference_ambiguous": 0, "occluded_reference": 0, "occluded_compared": 0, "max_reference_k": 0,
             "first_disagreements": [], "first_rejections": []}
    for x, y in INTERIOR:
        interval = reference[(x, y)]
        if interval is None:
            continue
        stats["pixels"] += 1
        stats["reference_ambiguous"] += interval[0] != interval[1]
        stats["occluded_reference"] += interval[0] > 0
        stats["max_reference_k"] = max(stats["max_reference_k"], interval[1])
        inferred, reason = infer_k(on.px(x, y), off.px(x, y), spec.px(x, y), strength)
        if inferred is None:
            stats["uninferable"] += 1
            stats["rejected"][reason] += 1
            if len(stats["first_rejections"]) < 12:
                stats["first_rejections"].append({"pixel": [x, y], "reason": reason, "on": list(on.px(x, y)[:3]),
                                                  "off": list(off.px(x, y)[:3]), "spec": list(spec.px(x, y)[:3])})
            continue
        stats["compared"] += 1
        stats["occluded_compared"] += interval[0] > 0
        if inferred[1] < -0.5 or inferred[0] > 16.5:
            stats["out_of_range"] += 1
        if inferred[1] >= interval[0] - 1e-9 and inferred[0] <= interval[1] + 1e-9:
            stats["agree"] += 1
        else:
            stats["disagree"] += 1
            if len(stats["first_disagreements"]) < 12:
                stats["first_disagreements"].append({"pixel": [x, y], "reference": interval,
                                                     "inferred": [round(inferred[0], 3), round(inferred[1], 3)]})
    required = math.ceil(MIN_COVERAGE_FRACTION * len(INTERIOR))
    stats["coverage_required"] = required
    stats["preconditions"] = dict(preconditions or {})
    problems = [f"precondition failed: {name}" for name, ok in stats["preconditions"].items() if ok is not True]
    if stats["pixels"] < required:
        problems.append(f"the reference defines {stats['pixels']} of {required} required interior pixels")
    if stats["compared"] < required:
        problems.append(f"coverage {stats['compared']} of {required} required interior pixels")
    invalid = {k: v for k, v in stats["rejected"].items() if v and k != "low_signal"}
    if invalid:
        problems.append(f"invalid inferences {invalid}")
    if stats["disagree"]:
        problems.append(f"{stats['disagree']} pixels disagree with the reference")
    if stats["out_of_range"]:
        problems.append(f"{stats['out_of_range']} inferred intervals outside [0, 16]")
    if expects_occlusion:
        if stats["occluded_reference"] < OCCLUSION_FLOOR:
            problems.append(f"only {stats['occluded_reference']} reference-occluded pixels "
                            f"(< {OCCLUSION_FLOOR}): the comparison would not exercise occlusion")
        elif stats["occluded_compared"] < stats["occluded_reference"]:
            problems.append(f"{stats['occluded_compared']} of {stats['occluded_reference']} "
                            "reference-occluded pixels compared")
    elif stats["occluded_reference"]:
        problems.append(f"absence control, but the reference marks {stats['occluded_reference']} pixels occluded")
    stats["problems"] = problems
    stats["pass"] = not problems
    return stats


def identical(a, b):
    return a.size == b.size and a.data == b.data


def differing(a, b):
    return sum(1 for i in range(0, len(a.data), 4) if a.data[i:i + 4] != b.data[i:i + 4])


def nearest_expansion(frame, reference, n, rect):
    """0067 rule: output pixel x samples texel floor((2x+1) n / 32)."""
    scaled = [v * n / 16 for v in rect]
    compared = mismatched = 0
    for x, y in interior(rect, frame.size, margin=1.5):
        tx, ty = ((2 * x + 1) * n) // 32, ((2 * y + 1) * n) // 32
        cx, cy = tx + 0.5, ty + 0.5
        if not (scaled[0] + 1 <= cx < scaled[0] + scaled[2] - 1 and scaled[1] + 1 <= cy < scaled[1] + scaled[3] - 1):
            continue
        compared += 1
        mismatched += frame.px(x, y) != reference.px(tx, ty)
    return {"scale16": n, "compared": compared, "mismatched": mismatched, "pass": compared > 0 and mismatched == 0}


def project(point, rect=RECT):
    x, y, w, h = rect
    f = 1 / math.tan(0.5)
    ndc = (point[0] / -point[2] * f / (w / h), point[1] / -point[2] * f)
    return (x + (ndc[0] * 0.5 + 0.5) * w, y + (0.5 - ndc[1] * 0.5) * h)


def box(centre, half, size):
    return [(x, y) for y in range(max(0, int(centre[1] - half)), min(size[1], int(centre[1] + half) + 1))
            for x in range(max(0, int(centre[0] - half)), min(size[0], int(centre[0] + half) + 1))]


def halo_energy(on, off, pixels):
    """Linear halo outside emitter footprints (pixels where `off` shows the backdrop)."""
    backdrop = off.px(int(RECT[0] + 3), int(RECT[1] + 3))[:3]
    total = 0.0
    for x, y in pixels:
        o = off.px(x, y)
        if o[:3] != backdrop:
            continue
        p = on.px(x, y)
        total += sum(LINEAR[p[c]] - LINEAR[o[c]] for c in range(3))
    return total


def max_change(a, b, pixels):
    return max((abs(a.px(x, y)[c] - b.px(x, y)[c]) for x, y in pixels for c in range(3)), default=0)


def centroid(on, off, rect):
    """Halo centroid over non-emitter pixels inside the viewport interior."""
    backdrop = off.px(int(rect[0] + 3), int(rect[1] + 3))[:3]
    weight = sx = sy = 0.0
    for x, y in interior(rect, on.size, margin=2):
        o = off.px(x, y)
        if o[:3] != backdrop:
            continue
        p = on.px(x, y)
        w = sum(LINEAR[p[c]] - LINEAR[o[c]] for c in range(3))
        if w > 0:
            weight += w
            sx += w * (x + 0.5)
            sy += w * (y + 0.5)
    return None if weight == 0 else (sx / weight, sy / weight, weight)


# ------------------------------------------------------------------ evidence and gates

CORE, MATERIALS, GEOMETRY, LIFECYCLE, BLOOM = ("case-ssao_core", "case-ssao_materials", "case-ssao_geometry",
                                               "case-lifecycle_drs", "case-bloom_numeric")


def declared_cases():
    plan = json.loads((TOOLS / "plan.json").read_text())
    return [f"case-{case['name']}" for case in plan["numeric_cases"]]


class Missing(Exception):
    """A declared case or frame is absent or invalid; its gates fail with this reason."""


class Evidence:
    def __init__(self, root):
        self.root = Path(root)
        self.cases, self.status = {}, {}
        for case in declared_cases():
            directory = self.root / "outputs" / case
            if not (directory / "index.json").is_file():
                self.status[case] = {"status": "missing", "reason": f"{directory / 'index.json'} absent"}
                continue
            try:
                self.cases[case] = load_output(directory)
            except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
                self.status[case] = {"status": "invalid", "reason": f"{type(error).__name__}: {error}"}
                continue
            self.status[case] = {"status": "loaded", "frames": len(self.cases[case])}

    def frame(self, case, label):
        if case not in self.cases:
            state = self.status.get(case, {"status": "undeclared", "reason": "not a declared numeric case"})
            raise Missing(f"{case} {state['status']}: {state['reason']}")
        if label not in self.cases[case]:
            raise Missing(f"{case}: frame {label} absent from index.json")
        return self.cases[case][label]


# (gate, case, on, off basis, specular basis, reference scene, orthographic, strength, occlusion expected)
SSAO_REFERENCE_GATES = [
    ("ssao_core_p_reference", CORE, "p_ao_white", "p_off_white", "p_off_black", "white_slab", False, 1.0, True),
    ("ssao_core_o_reference", CORE, "o_ao_white", "o_off_white", "o_off_black", "white_slab", True, 1.0, True),
    ("ssao_core_strength_0_5_reference", CORE, "p_ao_strength_0_5", "p_off_white", "p_off_black", "white_slab",
     False, 0.5, True),
    ("ssao_core_edge_reference", CORE, "p_ao_edge", "p_off_edge", "p_off_black", "edge_slab", False, 1.0, True),
    *[(f"ssao_geometry_{label}_reference", GEOMETRY, label, "g_off_white", "g_off_black", scene, False, 1.0, occludes)
      for label, scene, occludes in (
          ("g_ao_white", "white_slab", True), ("g_ao_mask_equal", "mask_equal", True),
          ("g_ao_mask_cutout", "mask_cutout", True), ("g_ao_backface", "double_sided_back_receiver", True),
          ("g_ao_mirrored", "mirrored_receiver", True), ("g_ao_skin_t0", "baked_slab_far", False),
          ("g_ao_skin_t1", "baked_slab_near", True), ("g_ao_motion_far", "far_slab", False),
          ("g_ao_motion_near", "white_slab", True))],
]

# (gate, case, frame, case, frame) that must be byte-identical
IDENTITY_GATES = [
    ("ssao_core_p_ao_flat_identity", CORE, "p_ao_flat", CORE, "p_off_flat"),
    ("ssao_core_p_ao_reversed_identity", CORE, "p_ao_reversed", CORE, "p_off_reversed"),
    ("ssao_core_p_ao_far_identity", CORE, "p_ao_far", CORE, "p_off_far"),
    ("ssao_core_p_ao_radius_0_5_identity", CORE, "p_ao_radius_0_5", CORE, "p_off_white"),
    ("determinism_g_off_white_equals_p_off_white", GEOMETRY, "g_off_white", CORE, "p_off_white"),
    ("determinism_g_ao_white_equals_p_ao_white", GEOMETRY, "g_ao_white", CORE, "p_ao_white"),
    ("determinism_retained_drs_disabled_equals_ordinary", LIFECYCLE, "d_ao_disabled", CORE, "p_ao_white"),
]


def reference_gate(case, on, off, spec, scene_name, orthographic, strength, occludes):
    def gate(ev):
        frames = [ev.frame(case, label) for label in (on, off, spec)]
        bases = {f"{off}_uniform": furnace(frames[1]), f"{spec}_uniform": furnace(frames[2])}
        result = ssao_agreement(*frames, ref.numeric_scene(scene_name, orthographic), strength=strength,
                                expects_occlusion=occludes,
                                preconditions={name: value["uniform"] for name, value in bases.items()})
        result["bases"] = bases
        return result
    return gate


def identity_gate(case_a, a, case_b, b):
    def gate(ev):
        first, second = ev.frame(case_a, a), ev.frame(case_b, b)
        return {"pass": identical(first, second), "differing_pixels": differing(first, second)
                if first.size == second.size else None, "sizes": [first.size, second.size]}
    return gate


def mixed_pane_gate(ev):
    core_off, core_ao = ev.frame(CORE, "p_off_white"), ev.frame(CORE, "p_ao_white")
    mixed_off, mixed_ao = ev.frame(MATERIALS, "m_off_mixed"), ev.frame(MATERIALS, "m_ao_mixed")
    pane = {(x, y) for x, y in INTERIOR if mixed_off.px(x, y) != core_off.px(x, y)}
    outside_equal = all(mixed_ao.px(x, y) == core_ao.px(x, y) for x, y in INTERIOR if (x, y) not in pane)
    ratios = []
    for x, y in sorted(pane):
        core_delta = LINEAR[core_off.px(x, y)[1]] - LINEAR[core_ao.px(x, y)[1]]
        mixed_delta = LINEAR[mixed_off.px(x, y)[1]] - LINEAR[mixed_ao.px(x, y)[1]]
        if core_delta > 0.03:
            ratios.append(mixed_delta / core_delta)
    ratios.sort()
    median = ratios[len(ratios) // 2] if ratios else None
    return {"pane_pixels": len(pane), "occluded_under_pane": len(ratios), "median_ratio": median,
            "outside_pane_identical_to_core": outside_equal,
            "pass": outside_equal and median is not None and 0.4 <= median <= 0.6}


def expansion_gate(n):
    def gate(ev):
        return nearest_expansion(ev.frame(LIFECYCLE, f"d_ao_{n}"), ev.frame(LIFECYCLE, f"d_ao_reference_{n}"), n, RECT)
    return gate


def drs_centroid_gate(ev):
    frames = {n: (ev.frame(LIFECYCLE, f"e_bloom_{n}"), ev.frame(LIFECYCLE, f"e_off_{n}")) for n in (16, 12, 8)}
    expected = project((0.9, 0.4, -4.0))
    centres = {n: centroid(on, off, RECT) for n, (on, off) in frames.items()}
    tolerance = {n: max(1.5, 1.5 * 16 / n) for n in (16, 12, 8)}
    ok = all(c is not None and math.hypot(c[0] - expected[0], c[1] - expected[1]) <= tolerance[n]
             for n, c in centres.items())
    return {"expected": expected, "centroids": {n: c and [round(c[0], 2), round(c[1], 2)] for n, c in centres.items()},
            "tolerance_px": tolerance, "pass": ok}


def bloom_boxes():
    return [box(project((x, 0.0, -4.0)), 40, SIZE) for x in (-2.4, 0.0, 2.4)]


def bloom_coverage_gate(ev):
    on, off = ev.frame(BLOOM, "c_bloom"), ev.frame(BLOOM, "c_off")
    boxes = bloom_boxes()
    energy = [halo_energy(on, off, b) for b in boxes]
    ratio = energy[1] / energy[2] if energy[2] > 0 else None
    unchanged = max_change(on, off, boxes[0]) == 0
    return {"energy_alpha_0_0.5_1": energy, "ratio_half_to_full": ratio, "alpha_0_unchanged": unchanged,
            "pass": unchanged and ratio is not None and 0.4 <= ratio <= 0.6}


def bloom_threshold_gate(ev):
    on, off = ev.frame(BLOOM, "t_bloom"), ev.frame(BLOOM, "t_off")
    changes = [max_change(on, off, b) for b in bloom_boxes()]
    return {"max_change_below_at_above": changes, "pass": changes[0] == 0 and changes[1] <= 1 and changes[2] > 0}


def bloom_field_gate(ev):
    frames = {label: ev.frame(BLOOM, label) for label in ("f_off", "f_bloom_l1", "f_bloom_l4", "f_bloom_l6")}
    predicted_on = round(linear_to_srgb(tone([1.25] * 3)[0]) * 255)
    predicted_off = round(linear_to_srgb(tone([1.0] * 3)[0]) * 255)
    core_box = box((RECT[0] + RECT[2] / 2, RECT[1] + RECT[3] / 2), 40, SIZE)
    field = {label: sorted({frame.px(x, y)[c] for x, y in core_box for c in range(3)})
             for label, frame in frames.items()}
    on_ok = all(all(abs(v - predicted_on) <= 1 for v in field[label])
                for label in ("f_bloom_l1", "f_bloom_l4", "f_bloom_l6"))
    off_ok = all(abs(v - predicted_off) <= 1 for v in field["f_off"])
    return {"predicted_on": predicted_on, "predicted_off": predicted_off, "interior_values": field,
            "pass": on_ok and off_ok}


GATES = {
    **{name: reference_gate(*spec) for name, *spec in SSAO_REFERENCE_GATES},
    **{name: identity_gate(*spec) for name, *spec in IDENTITY_GATES},
    "mixed_blend_pane_scales_receiver_change": mixed_pane_gate,
    "drs_ssao_nearest_expansion_12": expansion_gate(12),
    "drs_ssao_nearest_expansion_8": expansion_gate(8),
    "drs_bloom_halo_centroid_on_physical_emitter": drs_centroid_gate,
    "bloom_coverage": bloom_coverage_gate,
    "bloom_threshold": bloom_threshold_gate,
    "bloom_constant_field_levels": bloom_field_gate,
}


def run_gate(function, ev):
    try:
        result = function(ev)
    except Missing as missing:
        return {"pass": False, "missing": str(missing)}
    except Exception as error:  # a malformed frame fails its gate; the report still records it
        return {"pass": False, "error": f"{type(error).__name__}: {error}"}
    result["pass"] = result.get("pass") is True
    return result


def analyse(evidence):
    ev = Evidence(evidence)
    gates = {name: run_gate(function, ev) for name, function in GATES.items()}
    observations = {}
    if CORE in ev.cases:
        edge = ref.numeric_scene("edge_slab", False)
        single, true = reference_intervals(edge), reference_intervals(edge, unlimited=True)
        both = [(single[p], true[p]) for p in INTERIOR if single[p] and true[p]]
        observations["edge_single_layer_limitation"] = {
            "pixels_with_true_occlusion": sum(1 for _, t in both if t[0] > 0),
            "pixels_where_screen_space_sees_fewer_blocked_rays": sum(1 for s, t in both if s[1] < t[0]),
            "note": "off-screen geometry cannot occlude in the documented first-layer approximation"}
    outputs = Path(evidence) / "outputs"
    if outputs.is_dir():
        observations["retry_outputs_not_analysed"] = sorted(p.name for p in outputs.iterdir()
                                                            if p.is_dir() and p.name.startswith("retry-"))
    missing_cases = sorted(case for case, state in ev.status.items() if state["status"] != "loaded")
    failed = sorted(name for name, gate in gates.items() if gate["pass"] is not True)
    return {"format": "incant-0070-analysis-v2", "evidence": str(evidence), "declared_cases": declared_cases(),
            "cases": ev.status, "declared_gates": sorted(GATES), "gates": gates, "observations": observations,
            "missing_or_invalid_cases": missing_cases, "failed_gates": failed,
            "appearance": {"judged_here": False,
                           "note": "Numerical invariants only. The appearance PNGs await Claude's separate "
                                   "appearance review; their in-run invariants are the helper's own verdict."},
            "pass": not missing_cases and not failed and set(gates) == set(GATES)}


# ------------------------------------------------------------------ selftest

LIGHT, REFLECTANCE = 179 / 255, 0.05


def _byte(value):
    return round(linear_to_srgb(value) * 255)


OFF_BYTE, SPEC_BYTE = _byte(LIGHT), _byte(LIGHT * REFLECTANCE)


def _filled(size, byte):
    return bytearray(bytes((byte, byte, byte, 255)) * (size[0] * size[1]))


def _put(data, size, x, y, byte):
    i = (y * size[0] + x) * 4
    data[i:i + 3] = bytes((byte, byte, byte))


def _ao_frame(scene_name, orthographic, strength=1.0, perturb=None):
    """Simulated AO-on readback: L(1-R)AO + LR per interior pixel, the off value elsewhere."""
    data = _filled(SIZE, OFF_BYTE)
    for (x, y), interval in reference_intervals(ref.numeric_scene(scene_name, orthographic)).items():
        k = 0 if interval is None else interval[0]
        if perturb and perturb(x, y):
            k = min(16, k + 2)
        ao = round(255 * max(0.0, 1 - strength * k / 16)) / 255
        _put(data, SIZE, x, y, _byte(LIGHT * (1 - REFLECTANCE) * ao + LIGHT * REFLECTANCE))
    return Frame(SIZE, bytes(data), {})


def _gray(byte, size=SIZE):
    return Frame(size, bytes(_filled(size, byte)), {})


def _halo(off, centre, inner, outer, add, size=SIZE):
    data = bytearray(off.data)
    for y in range(size[1]):
        for x in range(size[0]):
            if inner <= math.hypot(x + 0.5 - centre[0], y + 0.5 - centre[1]) <= outer:
                i = (y * size[0] + x) * 4
                data[i:i + 3] = bytes(min(255, v + add) for v in data[i:i + 3])
    return Frame(size, bytes(data), {})


def _emitters(backdrop, centres, byte=250, half=3):
    data = _filled(SIZE, backdrop)
    for cx, cy in centres:
        for y in range(int(cy) - half, int(cy) + half + 1):
            for x in range(int(cx) - half, int(cx) + half + 1):
                _put(data, SIZE, x, y, byte)
    return Frame(SIZE, bytes(data), {})


def _expanded(reference, n):
    data = _filled(SIZE, 0)
    for y in range(SIZE[1]):
        for x in range(SIZE[0]):
            i = (y * SIZE[0] + x) * 4
            data[i:i + 4] = reference.px(((2 * x + 1) * n) // 32, ((2 * y + 1) * n) // 32)
    return Frame(SIZE, bytes(data), {})


def _pattern(size):
    data = bytearray()
    for y in range(size[1]):
        for x in range(size[0]):
            data += bytes(((x * 7 + y * 13) % 256, (x * 3) % 256, (y * 5) % 256, 255))
    return Frame(size, bytes(data), {})


def synthetic_evidence():
    """A complete, internally consistent evidence set for every declared gate."""
    off, spec = _gray(OFF_BYTE), _gray(SPEC_BYTE)
    p_ao = _ao_frame("white_slab", False)
    core = {"p_off_white": off, "p_ao_white": p_ao, "p_off_black": spec, "p_ao_black": spec,
            "o_off_white": off, "o_ao_white": _ao_frame("white_slab", True), "o_off_black": spec, "o_ao_black": spec,
            "p_off_flat": off, "p_ao_flat": off, "o_off_flat": off, "o_ao_flat": off,
            "p_off_reversed": off, "p_ao_reversed": off, "p_off_far": off, "p_ao_far": off,
            "p_ao_radius_0_5": off, "p_ao_strength_0_5": _ao_frame("white_slab", False, strength=0.5),
            "p_off_edge": off, "p_ao_edge": _ao_frame("edge_slab", False)}
    geometry = {"g_off_white": off, "g_ao_white": p_ao, "g_off_black": spec, "g_ao_mask_equal": p_ao,
                "g_off_mask_below": off, "g_ao_mask_below": off, "g_ao_mask_cutout": _ao_frame("mask_cutout", False),
                "g_ao_backface": p_ao, "g_ao_mirrored": p_ao, "g_off_tilted": off, "g_ao_tilted": off,
                "g_ao_skin_t0": _ao_frame("baked_slab_far", False), "g_ao_skin_t1": p_ao,
                "g_ao_motion_far": _ao_frame("far_slab", False), "g_ao_motion_near": p_ao}
    occluded = reference_intervals(ref.numeric_scene("white_slab", False))
    pane = {p for p, interval in occluded.items() if interval and interval[0] >= 2 and p[1] < 100}
    mixed_off, mixed_ao = bytearray(off.data), bytearray(p_ao.data)
    for x, y in pane:
        for frame, data in ((off, mixed_off), (p_ao, mixed_ao)):
            _put(data, SIZE, x, y, _byte(0.5 * 0.2 + 0.5 * LINEAR[frame.px(x, y)[1]]))
    materials = {"m_off_mixed": Frame(SIZE, bytes(mixed_off), {}), "m_ao_mixed": Frame(SIZE, bytes(mixed_ao), {})}
    lifecycle = {"d_ao_disabled": p_ao}
    for n in (12, 8):
        internal = (math.ceil(SIZE[0] * n / 16), math.ceil(SIZE[1] * n / 16))
        lifecycle[f"d_ao_reference_{n}"] = _pattern(internal)
        lifecycle[f"d_ao_{n}"] = _expanded(lifecycle[f"d_ao_reference_{n}"], n)
    emitter = project((0.9, 0.4, -4.0))
    for n in (16, 12, 8):
        lifecycle[f"e_off_{n}"] = _emitters(40, [emitter])
        lifecycle[f"e_bloom_{n}"] = _halo(lifecycle[f"e_off_{n}"], emitter, 6, 12, 6)
    centres = [project((x, 0.0, -4.0)) for x in (-2.4, 0.0, 2.4)]
    c_off = _emitters(40, centres)
    c_bloom = _halo(_halo(c_off, centres[1], 6, 12, 3), centres[2], 6, 12, 6)
    t_off = _emitters(40, centres)
    predicted_on = round(linear_to_srgb(tone([1.25] * 3)[0]) * 255)
    predicted_off = round(linear_to_srgb(tone([1.0] * 3)[0]) * 255)
    bloom = {"c_off": c_off, "c_bloom": c_bloom, "t_off": t_off, "t_bloom": _halo(t_off, centres[2], 6, 12, 6),
             "f_off": _gray(predicted_off), **{f"f_bloom_l{n}": _gray(predicted_on) for n in (1, 4, 6)}}
    return {CORE: core, MATERIALS: materials, GEOMETRY: geometry, LIFECYCLE: lifecycle, BLOOM: bloom}


def write_case(directory, frames):
    directory.mkdir(parents=True)
    entries = []
    for label, frame in frames.items():
        (directory / f"{label}.rgba").write_bytes(frame.data)
        entries.append({"label": label, "file": f"{label}.rgba", "size": list(frame.size),
                        "rgba_sha256": hashlib.sha256(frame.data).hexdigest(), "meta": {}})
    (directory / "index.json").write_text(json.dumps({"frames": entries, "summary": {}}))


def write_evidence(root, cases):
    for case, frames in cases.items():
        write_case(root / "outputs" / case, frames)
    return root


def _rejection_controls():
    off, spec = (OFF_BYTE,) * 3 + (255,), (SPEC_BYTE,) * 3 + (255,)
    on = (_byte(LIGHT * (1 - REFLECTANCE) * 0.75 + LIGHT * REFLECTANCE),) * 3 + (255,)
    interval, reason = infer_k(on, off, spec, 1.0)
    assert reason is None and interval[0] <= 4 <= interval[1], (interval, reason)
    assert infer_k(on, spec, spec, 1.0) == (None, "low_signal")       # no diffuse signal: empty inference
    conflict = (on[0], OFF_BYTE, on[2], 255)                          # G says AO = 1, R and B say 0.75
    assert infer_k(conflict, off, spec, 1.0) == (None, "empty")
    assert infer_k(on, off, spec, math.nan) == (None, "nonfinite")
    assert infer_k(on, off, spec, 0.0) == (None, "nonfinite")
    black = (0, 0, 0, 255)  # darker than the specular floor
    inverted = [b for b in range(SPEC_BYTE + 1, 256) if infer_k(black, (b,) * 3 + (255,), spec, 1.0)[1] == "inverted"]
    assert inverted, "no off level produced an inverted channel interval"
    below = infer_k(black, off, spec, 1.0)[0]
    assert below is not None and below[0] > 16.5, below  # kept, then counted out of range, never agreement
    return {"infer_k": "valid, low_signal, empty (conflicting RGB), nonfinite (NaN and zero strength) and "
                       f"inverted (off bytes {inverted[0]}..{inverted[-1]}) checked; below-floor is out of range"}


def selftest():
    results = {"rejections": _rejection_controls()}
    off, spec = _gray(OFF_BYTE), _gray(SPEC_BYTE)
    uniform = {"off_uniform": True, "spec_uniform": True}
    for orthographic in (False, True):
        scene = ref.numeric_scene("white_slab", orthographic)
        on = _ao_frame("white_slab", orthographic)
        good = ssao_agreement(on, off, spec, scene, preconditions=uniform)
        assert good["pass"] and good["compared"] == len(INTERIOR) and good["occluded_compared"] > OCCLUSION_FLOOR, good
        bad = ssao_agreement(_ao_frame("white_slab", orthographic, perturb=lambda x, y: (x + y) % 97 == 0),
                             off, spec, scene, preconditions=uniform)
        assert not bad["pass"] and bad["disagree"] > 0, bad
        results["orthographic" if orthographic else "perspective"] = {
            "matching_pixels": good["agree"], "occluded": good["occluded_compared"],
            "perturbed_disagreements": bad["disagree"]}
    scene, on = ref.numeric_scene("white_slab", False), _ao_frame("white_slab", False)
    # Empty inference: every pixel uninferable must fail, never pass vacuously.
    empty = ssao_agreement(on, spec, spec, scene, preconditions=uniform)
    assert not empty["pass"] and empty["compared"] == 0 and empty["rejected"]["low_signal"] == len(INTERIOR), empty
    # Conflicting RGB intervals in a block are invalid inferences.
    conflicted = bytearray(on.data)
    for x in range(40, 60):
        for y in range(40, 60):
            i = (y * SIZE[0] + x) * 4
            conflicted[i + 1] = OFF_BYTE if conflicted[i] != OFF_BYTE else SPEC_BYTE + 40
    result = ssao_agreement(Frame(SIZE, bytes(conflicted), {}), off, spec, scene, preconditions=uniform)
    assert not result["pass"] and result["rejected"]["empty"] > 0, result["rejected"]
    # Inadequate coverage: a low-signal block, no disagreement anywhere else.
    weak_off = bytearray(off.data)
    for x in range(200, 230):
        for y in range(120, 140):
            _put(weak_off, SIZE, x, y, SPEC_BYTE + 1)
    weak = ssao_agreement(on, Frame(SIZE, bytes(weak_off), {}), spec, scene, preconditions=uniform)
    assert not weak["pass"] and weak["disagree"] == 0 and weak["rejected"]["low_signal"] == 600, weak
    assert any("coverage" in p for p in weak["problems"]), weak["problems"]
    # Occlusion must be real: a slab-free scene cannot stand in for an occlusion gate.
    vacuous = ssao_agreement(off, off, spec, ref.numeric_scene("far_slab", False), preconditions=uniform)
    assert not vacuous["pass"] and any("reference-occluded" in p for p in vacuous["problems"]), vacuous
    absent = ssao_agreement(on, off, spec, scene, expects_occlusion=False, preconditions=uniform)
    assert not absent["pass"] and any("absence control" in p for p in absent["problems"]), absent
    # Failed furnace precondition: a non-uniform shared basis fails the gate.
    lumpy = bytearray(off.data)
    _put(lumpy, SIZE, 100, 100, OFF_BYTE - 1)
    bases = {"off_uniform": furnace(Frame(SIZE, bytes(lumpy), {}))["uniform"], "spec_uniform": True}
    furnace_failed = ssao_agreement(on, Frame(SIZE, bytes(lumpy), {}), spec, scene, preconditions=bases)
    assert not furnace_failed["pass"] and "precondition failed: off_uniform" in furnace_failed["problems"], furnace_failed
    results["negative_controls"] = ["empty inference", "conflicting RGB", "inadequate coverage", "vacuous occlusion",
                                    "absence mismatch", "furnace precondition"]
    # Whole-report controls on synthetic evidence inside the packet directory.
    scratch = TOOLS.parent / f".analyze-selftest-{os.getpid()}"
    scratch.mkdir()
    try:
        cases = synthetic_evidence()
        full = write_evidence(scratch / "full", cases)
        report = analyse(full)
        assert report["pass"] and set(report["gates"]) == set(GATES) and not report["failed_gates"], report["failed_gates"]
        assert main([str(full)]) == 0 and json.loads((full / "analysis.json").read_text())["pass"] is True
        results["complete_synthetic_evidence"] = {"gates": len(report["gates"]), "pass": report["pass"]}
        # A declared frame missing from its index fails that gate with a retained reason.
        partial_core = dict(cases[CORE])
        del partial_core["p_ao_edge"]
        frame_missing = analyse(write_evidence(scratch / "frame-missing", {**cases, CORE: partial_core}))
        gate = frame_missing["gates"]["ssao_core_edge_reference"]
        assert not frame_missing["pass"] and "p_ao_edge absent" in gate["missing"], gate
        assert frame_missing["failed_gates"] == ["ssao_core_edge_reference"], frame_missing["failed_gates"]
        # Missing declared cases fail every gate that needs them; the rest still run.
        only_bloom = write_evidence(scratch / "only-bloom", {BLOOM: cases[BLOOM]})
        sparse = analyse(only_bloom)
        assert not sparse["pass"] and set(sparse["gates"]) == set(GATES), sparse["failed_gates"]
        assert sparse["missing_or_invalid_cases"] == sorted(set(declared_cases()) - {BLOOM})
        assert all(sparse["gates"][g]["pass"] for g in ("bloom_coverage", "bloom_threshold", "bloom_constant_field_levels"))
        assert "missing" in sparse["gates"]["ssao_core_p_reference"]
        # A tampered file makes its case invalid instead of crashing the report.
        tampered = write_evidence(scratch / "tampered", {BLOOM: cases[BLOOM]})
        (tampered / "outputs" / BLOOM / "c_off.rgba").write_bytes(b"\0" * len(cases[BLOOM]["c_off"].data))
        assert Evidence(tampered).status[BLOOM]["status"] == "invalid"
        # The CLI writes the failing report and exits nonzero; it never replaces a report.
        cli = [sys.executable, "-I", "-B", str(Path(__file__).resolve()), str(only_bloom)]
        first = subprocess.run(cli, capture_output=True, text=True, timeout=600)
        saved = (only_bloom / "analysis.json").read_bytes()
        assert first.returncode == 1 and json.loads(saved)["pass"] is False, (first.returncode, first.stderr)
        second = subprocess.run(cli, capture_output=True, text=True, timeout=600)
        assert second.returncode == 2 and (only_bloom / "analysis.json").read_bytes() == saved, second.returncode
        results["report_controls"] = ["complete evidence passes (CLI exit 0)", "missing frame", "missing cases",
                                      "tampered file", "CLI exit 1 with the report retained", "never replaced (exit 2)"]
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    assert round(linear_to_srgb(tone([1.0] * 3)[0]) * 255) == 243  # engine: tone_map(1.0) = 0.9
    frame = Frame((4, 1), bytes([1, 2, 3, 255] * 4), {})
    assert nearest_expansion(frame, frame, 16, (0, 0, 4, 1))["mismatched"] == 0
    return results


def main(argv=None):
    """Exit status: 0 the analysis passed, 1 it failed (report written), 2 usage."""
    argv = sys.argv[1:] if argv is None else argv
    if argv == ["--selftest"]:
        print(json.dumps(selftest(), indent=1))
        return 0
    if len(argv) != 1 or not Path(argv[0]).is_dir():
        print(__doc__, file=sys.stderr)
        return 2
    evidence = Path(argv[0])
    out = evidence / "analysis.json"
    if out.exists():
        print(f"{out} exists; reports are never replaced", file=sys.stderr)
        return 2
    report = analyse(evidence)
    with open(out, "x") as handle:
        json.dump(report, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({"pass": report["pass"], "missing_or_invalid_cases": report["missing_or_invalid_cases"],
                      "failed_gates": report["failed_gates"]}, indent=1))
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
