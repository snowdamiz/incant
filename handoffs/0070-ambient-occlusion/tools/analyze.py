#!/usr/bin/env python3
"""Offline analysis of retained 0070 readbacks. CPU only, standard library only.

Reads the helper outputs under <evidence>/outputs/<label>/ (index.json + raw
RGBA), verifies every file against its recorded hash, and derives:

* SSAO: per receiver pixel, an interval for the blocked-ray count k from the
  AO-on frame, the AO-off frame and the specular-only (diffuse factor 0) frame:
  on = D*AO + S, off = D + S, S from the black frame, with +/-0.75 sRGB code,
  R8 accessibility and f16 radiance quantization folded into the interval.
  It must meet the independent reference interval (ssao_reference.py) and lie
  inside [0, 16]. The off/specular frames are only valid bases for scenes whose
  visible surfaces all face +Z with the same material; the furnace precondition
  (uniform off frame) is checked and reported.
* DRS: reduced-scale frames equal the nearest expansion of internal-size
  references away from viewport edges (0067 texel rule).
* Bloom: zero-alpha and below-threshold emitters change nothing; fractional
  coverage scales halo energy; a constant field matches the tone-curve
  prediction at 1/4/6 levels; DRS halo centroids stay on the physical emitter.
* Determinism: identical scene/renderer/route frames match across children.

No golden image or aesthetic threshold is used.

  python3 -I -B analyze.py <evidence-dir>      # writes <evidence-dir>/analysis.json
  python3 -I -B analyze.py --selftest
"""

import hashlib
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ssao_reference as ref  # noqa: E402

RECT = (13.25, 17.5, 280.5, 160.5)
SIZE = (321, 193)
QUANTIZATION_CODES = 0.75
AO_SLACK = 0.5 / 255 + 0.002  # R8 accessibility + f16 radiance, generous


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
    """Frames of one helper output directory, hash-verified against index.json."""
    index = json.loads((directory / "index.json").read_text())
    frames = {}
    for entry in index["frames"]:
        path = directory / entry["file"]
        if entry["file"].endswith(".rgba"):
            data = path.read_bytes()
        else:
            continue  # PNGs are reviewed visually; their RGBA hash is in the index
        if hashlib.sha256(data).hexdigest() != entry["rgba_sha256"]:
            raise SystemExit(f"{path}: content does not match its recorded hash")
        if len(data) != entry["size"][0] * entry["size"][1] * 4:
            raise SystemExit(f"{path}: size mismatch")
        frames[entry["label"]] = Frame(tuple(entry["size"]), data, entry.get("meta", {}))
    return index, frames


def interior(rect, size, margin=1.5):
    x0, y0 = math.ceil(rect[0] + margin), math.ceil(rect[1] + margin)
    x1, y1 = math.floor(rect[0] + rect[2] - margin), math.floor(rect[1] + rect[3] - margin)
    for y in range(max(y0, 0), min(y1, size[1])):
        for x in range(max(x0, 0), min(x1, size[0])):
            yield x, y


def infer_k(on, off, spec, strength):
    """Blocked-ray interval from three RGBA pixels (intersection over RGB)."""
    lo_k, hi_k = -math.inf, math.inf
    for c in range(3):
        on_lo, on_hi = code_interval(on[c])
        off_lo, off_hi = code_interval(off[c])
        s_lo, s_hi = code_interval(spec[c])
        d_lo, d_hi = off_lo - s_hi, off_hi - s_lo
        if d_lo <= 0.02:
            return None  # too little diffuse signal to infer accessibility
        ao_lo = (on_lo - s_hi) / d_hi - AO_SLACK
        ao_hi = (on_hi - s_lo) / d_lo + AO_SLACK
        k_lo, k_hi = 16 * (1 - ao_hi) / strength, 16 * (1 - ao_lo) / strength
        lo_k, hi_k = max(lo_k, k_lo), min(hi_k, k_hi)
    return (lo_k, hi_k)


def furnace(frame, rect):
    values = {tuple(frame.px(x, y)[:3]) for x, y in interior(rect, frame.size)}
    return {"distinct_rgb_values": len(values), "uniform": len(values) == 1,
            "values": sorted(values)[:4]}


def ssao_agreement(on, off, spec, scene, strength=1.0, radius=2.0, limit=None):
    """Compare inferred intervals with the reference for every interior pixel."""
    stats = {"pixels": 0, "agree": 0, "disagree": 0, "reference_ambiguous": 0, "uninferable": 0,
             "out_of_range": 0, "occluded_reference": 0, "max_reference_k": 0, "first_disagreements": []}
    for x, y in interior(RECT, on.size):
        if limit is not None and not limit(x, y):
            continue
        interval = ref.pixel_interval(scene, (x, y), radius=radius)
        if interval is None:
            continue
        stats["pixels"] += 1
        inferred = infer_k(on.px(x, y), off.px(x, y), spec.px(x, y), strength)
        if inferred is None:
            stats["uninferable"] += 1
            continue
        if inferred[1] < -0.5 or inferred[0] > 16.5:
            stats["out_of_range"] += 1
        if interval[0] != interval[1]:
            stats["reference_ambiguous"] += 1
        stats["occluded_reference"] += interval[0] > 0
        stats["max_reference_k"] = max(stats["max_reference_k"], interval[1])
        if inferred[1] >= interval[0] - 1e-9 and inferred[0] <= interval[1] + 1e-9:
            stats["agree"] += 1
        else:
            stats["disagree"] += 1
            if len(stats["first_disagreements"]) < 12:
                stats["first_disagreements"].append({"pixel": [x, y], "reference": interval,
                                                     "inferred": [round(inferred[0], 3), round(inferred[1], 3)]})
    stats["pass"] = stats["disagree"] == 0 and stats["out_of_range"] == 0 and stats["pixels"] > 0
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


# ------------------------------------------------------------------ report

def analyse(evidence):
    outputs = Path(evidence) / "outputs"
    cases = {}
    for name in ("case-ssao_core", "case-ssao_materials", "case-ssao_geometry", "case-lifecycle_drs",
                 "case-bloom_numeric", "appearance"):
        if (outputs / name / "index.json").exists():
            cases[name] = load_output(outputs / name)[1]
    report = {"gates": {}, "observations": {}}
    gates, notes = report["gates"], report["observations"]
    core = cases.get("case-ssao_core")
    if core:
        notes["furnace_off_perspective"] = furnace(core["p_off_white"], RECT)
        notes["furnace_off_orthographic"] = furnace(core["o_off_white"], RECT)
        for tag, orthographic in (("p", False), ("o", True)):
            scene = ref.numeric_scene("white_slab", orthographic)
            gates[f"ssao_core_{tag}_reference"] = ssao_agreement(
                core[f"{tag}_ao_white"], core[f"{tag}_off_white"], core[f"{tag}_off_black"], scene)
        perspective = ref.numeric_scene("white_slab", False)
        gates["ssao_core_strength_0_5_reference"] = ssao_agreement(
            core["p_ao_strength_0_5"], core["p_off_white"], core["p_off_black"], perspective, strength=0.5)
        edge = ref.numeric_scene("edge_slab", False)
        gates["ssao_core_edge_reference"] = ssao_agreement(core["p_ao_edge"], core["p_off_edge"],
                                                           core["p_off_black"], edge)
        limited = unlimited = 0
        for x, y in interior(RECT, SIZE):
            single = ref.pixel_interval(edge, (x, y))
            true = ref.pixel_interval(edge, (x, y), unlimited=True)
            if single and true:
                limited += single[1] < true[0]
                unlimited += true[0] > 0
        notes["edge_single_layer_limitation"] = {
            "pixels_with_true_occlusion": unlimited,
            "pixels_where_screen_space_sees_fewer_blocked_rays": limited,
            "note": "off-screen geometry cannot occlude in the documented first-layer approximation"}
        for label in ("p_ao_flat", "p_ao_reversed", "p_ao_far", "p_ao_radius_0_5"):
            base = {"p_ao_flat": "p_off_flat", "p_ao_reversed": "p_off_reversed", "p_ao_far": "p_off_far",
                    "p_ao_radius_0_5": "p_off_white"}[label]
            gates[f"ssao_core_{label}_identity"] = {"pass": identical(core[label], core[base]),
                                                    "differing_pixels": differing(core[label], core[base])}
    geometry = cases.get("case-ssao_geometry")
    if geometry:
        basis_off, basis_spec = geometry["g_off_white"], geometry["g_off_black"]
        notes["furnace_off_geometry"] = furnace(basis_off, RECT)
        for label, scene_name in (("g_ao_white", "white_slab"), ("g_ao_mask_equal", "mask_equal"),
                                  ("g_ao_mask_cutout", "mask_cutout"), ("g_ao_backface", "double_sided_back_receiver"),
                                  ("g_ao_mirrored", "mirrored_receiver"), ("g_ao_skin_t0", "baked_slab_far"),
                                  ("g_ao_skin_t1", "baked_slab_near"), ("g_ao_motion_far", "far_slab"),
                                  ("g_ao_motion_near", "white_slab")):
            scene = ref.numeric_scene(scene_name, False)
            gates[f"ssao_geometry_{label}_reference"] = ssao_agreement(geometry[label], basis_off, basis_spec, scene)
        if core:
            for a, b in (("g_off_white", "p_off_white"), ("g_ao_white", "p_ao_white")):
                gates[f"determinism_{a}_equals_{b}"] = {"pass": identical(geometry[a], core[b])}
    materials = cases.get("case-ssao_materials")
    if materials and core:
        pane = [(x, y) for x, y in interior(RECT, SIZE)
                if materials["m_off_mixed"].px(x, y) != core["p_off_white"].px(x, y)]
        outside_equal = all(materials["m_ao_mixed"].px(x, y) == core["p_ao_white"].px(x, y)
                            for x, y in interior(RECT, SIZE) if (x, y) not in set(pane))
        ratios = []
        for x, y in pane:
            core_delta = LINEAR[core["p_off_white"].px(x, y)[1]] - LINEAR[core["p_ao_white"].px(x, y)[1]]
            mixed_delta = LINEAR[materials["m_off_mixed"].px(x, y)[1]] - LINEAR[materials["m_ao_mixed"].px(x, y)[1]]
            if core_delta > 0.03:
                ratios.append(mixed_delta / core_delta)
        ratios.sort()
        median = ratios[len(ratios) // 2] if ratios else None
        gates["mixed_blend_pane_scales_receiver_change"] = {
            "pane_pixels": len(pane), "occluded_under_pane": len(ratios), "median_ratio": median,
            "outside_pane_identical_to_core": outside_equal,
            "pass": outside_equal and median is not None and 0.4 <= median <= 0.6}
    lifecycle = cases.get("case-lifecycle_drs")
    if lifecycle:
        for n in (12, 8):
            gates[f"drs_ssao_nearest_expansion_{n}"] = nearest_expansion(
                lifecycle[f"d_ao_{n}"], lifecycle[f"d_ao_reference_{n}"], n, RECT)
        if core:
            gates["determinism_retained_drs_disabled_equals_ordinary"] = {
                "pass": identical(lifecycle["d_ao_disabled"], core["p_ao_white"])}
        expected = project((0.9, 0.4, -4.0))
        centres = {}
        for n in (16, 12, 8):
            found = centroid(lifecycle[f"e_bloom_{n}"], lifecycle[f"e_off_{n}"], RECT)
            centres[n] = found
        tolerance = {n: max(1.5, 1.5 * 16 / n) for n in (16, 12, 8)}
        ok = all(c is not None and math.hypot(c[0] - expected[0], c[1] - expected[1]) <= tolerance[n]
                 for n, c in centres.items())
        gates["drs_bloom_halo_centroid_on_physical_emitter"] = {
            "expected": expected, "centroids": {n: c and [round(c[0], 2), round(c[1], 2)] for n, c in centres.items()},
            "tolerance_px": tolerance, "pass": ok}
    bloom = cases.get("case-bloom_numeric")
    if bloom:
        centres = [project((x, 0.0, -4.0)) for x in (-2.4, 0.0, 2.4)]
        boxes = [box(c, 40, SIZE) for c in centres]
        energy = [halo_energy(bloom["c_bloom"], bloom["c_off"], b) for b in boxes]
        ratio = energy[1] / energy[2] if energy[2] > 0 else None
        gates["bloom_coverage"] = {
            "energy_alpha_0_0.5_1": energy, "ratio_half_to_full": ratio,
            "alpha_0_unchanged": max_change(bloom["c_bloom"], bloom["c_off"], boxes[0]) == 0,
            "pass": max_change(bloom["c_bloom"], bloom["c_off"], boxes[0]) == 0 and ratio is not None
            and 0.4 <= ratio <= 0.6}
        changes = [max_change(bloom["t_bloom"], bloom["t_off"], b) for b in boxes]
        gates["bloom_threshold"] = {"max_change_below_at_above": changes,
                                    "pass": changes[0] == 0 and changes[1] <= 1 and changes[2] > 0}
        predicted_on = round(linear_to_srgb(tone([1.25] * 3)[0]) * 255)
        predicted_off = round(linear_to_srgb(tone([1.0] * 3)[0]) * 255)
        centre = (RECT[0] + RECT[2] / 2, RECT[1] + RECT[3] / 2)
        core_box = box(centre, 40, SIZE)
        field = {}
        for label in ("f_off", "f_bloom_l1", "f_bloom_l4", "f_bloom_l6"):
            values = {bloom[label].px(x, y)[c] for x, y in core_box for c in range(3)}
            field[label] = sorted(values)
        on_ok = all(all(abs(v - predicted_on) <= 1 for v in field[label])
                    for label in ("f_bloom_l1", "f_bloom_l4", "f_bloom_l6"))
        off_ok = all(abs(v - predicted_off) <= 1 for v in field["f_off"])
        gates["bloom_constant_field_levels"] = {"predicted_on": predicted_on, "predicted_off": predicted_off,
                                                "interior_values": field, "pass": on_ok and off_ok}
    report["pass"] = all(g.get("pass") for g in gates.values()) and bool(gates)
    return report


# ------------------------------------------------------------------ selftest

def _synthetic(scene, strength, perturb=None):
    """Simulated readbacks: L*(1-R)*AO + L*R with R = 0.05, through sRGB bytes."""
    width, height = SIZE
    light, reflectance = 179 / 255, 0.05
    on, off, spec = (bytearray(width * height * 4) for _ in range(3))
    for x, y in interior(RECT, SIZE):
        interval = ref.pixel_interval(scene, (x, y))
        k = 0 if interval is None else interval[0]
        if perturb and perturb(x, y):
            k = min(16, k + 2)
        ao = round(255 * max(0.0, 1 - strength * k / 16)) / 255
        values = (light * (1 - reflectance) * ao + light * reflectance, light, light * reflectance)
        for frame, value in zip((on, off, spec), values):
            byte = round(linear_to_srgb(value) * 255)
            i = (y * width + x) * 4
            frame[i:i + 4] = bytes((byte, byte, byte, 255))
    return [Frame(SIZE, bytes(f), {}) for f in (on, off, spec)]


def selftest():
    results = {}
    for orthographic in (False, True):
        scene = ref.numeric_scene("white_slab", orthographic)
        on, off, spec = _synthetic(scene, 1.0)
        good = ssao_agreement(on, off, spec, scene)
        assert good["pass"] and good["occluded_reference"] > 1000, good
        on_bad, _, _ = _synthetic(scene, 1.0, perturb=lambda x, y: (x + y) % 97 == 0)
        bad = ssao_agreement(on_bad, off, spec, scene)
        assert not bad["pass"] and bad["disagree"] > 0, bad
        results["orthographic" if orthographic else "perspective"] = {
            "matching_pixels": good["agree"], "occluded": good["occluded_reference"],
            "perturbed_disagreements": bad["disagree"]}
    assert round(linear_to_srgb(tone([1.0] * 3)[0]) * 255) == 243  # engine: tone_map(1.0) = 0.9
    frame = Frame((4, 1), bytes([1, 2, 3, 255] * 4), {})
    assert nearest_expansion(frame, frame, 16, (0, 0, 4, 1))["mismatched"] == 0
    return results


def main():
    if sys.argv[1:] == ["--selftest"]:
        print(json.dumps(selftest(), indent=1))
        return
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    evidence = Path(sys.argv[1])
    report = analyse(evidence)
    out = evidence / "analysis.json"
    with open(out, "x") as handle:
        json.dump(report, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({"pass": report["pass"], "gates": {k: v.get("pass") for k, v in report["gates"].items()}},
                     indent=1))


if __name__ == "__main__":
    main()
