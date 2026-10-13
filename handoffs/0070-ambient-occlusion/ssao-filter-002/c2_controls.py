"""Synthetic CPU controls for the 0070-ssao-filter-002 C2 analysis (analyze.py --selftest).

In-memory evidence only: no GPU, Cargo, build or ignored test. The shared
controls.py helpers (frames_for, Memory, model_planes, write_case and
ambiguous_and_empty) are reused read only, bound by c2_analysis.REUSED.

Positive: R8 stage planes from an INDEPENDENT f64 C2 model pass every C2
filtered-reference and stage-readback gate on the five quality fixtures. The
model is quality_model's single-layer test with the phase rebuilt from the
Sobol' direction numbers (as root's CPU mirror does), not c2_analysis's table.
Fidelity: C2FilterOracle given the R1/R3 phase reproduces the shared oracle's
k intervals exactly, so the override changes the phase and nothing else.
Negative: R1/R3 planes cannot pass as C2 (and C2 cannot pass the shared R1/R3
oracle); a wrong phase, base index, ray order or stratum offset, or a missing
filter pass, fails; the shared perturbation, ambiguity, missing-evidence and
report/CLI refusals still hold; tampered reused sources are refused.
Quality: the C2 model reproduces the screen on all 14 N8 rows; authored N6
fails and is reported; N6T accepts the independent smooth contact control and
rejects the shipped stepped output, banding, a hatch and a 4-code step.
"""

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

import c2_analysis as c2
import controls  # noqa: E402  (ssao-repair/filter/controls.py, read only and pinned)

af, orc, qm, qr = c2.af, c2.orc, c2.qm, c2.qr
W = af.SIZE[0]
SPECS = {name: spec for name, *spec in af.REFERENCE_GATES}
FIXTURES = {"ssao_core_p_filtered": "white_slab", "ssao_core_o_filtered": "white_slab",
            "ssao_core_edge_filtered": "edge_slab", "ssao_quality_p_stripes_filtered": "stripes",
            "ssao_quality_o_stripes_filtered": "stripes"}
SCREEN_C2_ROWS = {False: [0.22, 0.22, 0.22, 0.094, 0.105, 0.146, 0.125],
                  True: [0.075, 0.075, 0.125, 0.125, 0.105, 0.105, 0.084]}
SCREEN_R1R3_ROWS = {False: [0.376, 0.439, 0.502, 0.533, 0.356, 0.335, 0.272],
                    True: [0.176, 0.163, 0.173, 0.167, 0.167, 0.125, 0.146]}


def sobol(ray):
    """Second Sobol' coordinate in sixteenths from its direction numbers."""
    value = 0
    for bit, direction in enumerate((8, 12, 10, 15)):
        if ray >> bit & 1:
            value ^= direction
    return value


def reverse(stratum):
    return sum(((stratum >> bit) & 1) << (3 - bit) for bit in range(4))


PHASES = {
    "c2": lambda x, y, ray: ((reverse(qm.BAYER[y % 4][x % 4]) ^ sobol(ray)) + 0.5) / 16,
    "c1_shared_phase": lambda x, y, ray: (reverse(qm.BAYER[y % 4][x % 4]) + 0.5) / 16,
    "transposed_base_index": lambda x, y, ray: ((reverse(qm.BAYER[x % 4][y % 4]) ^ sobol(ray)) + 0.5) / 16,
    "reversed_ray_order": lambda x, y, ray: ((reverse(qm.BAYER[y % 4][x % 4]) ^ sobol(15 - ray)) + 0.5) / 16,
    "stratum_start_offset": lambda x, y, ray: (reverse(qm.BAYER[y % 4][x % 4]) ^ sobol(ray)) / 16,
}
_RAW = {}


def model_raw(name, orthographic, phase):
    """quality_model.raw's single-layer test with a per-ray phase, every texel."""
    key = (name, orthographic, phase)
    if key in _RAW:
        return _RAW[key]
    g = c2.grid(controls.GRID_NAMES[name], orthographic)
    w, project, hits, r2, phase_of = g.w, g.camera.project, g.hits, qm.RADIUS * qm.RADIUS, PHASES[phase]
    out = []
    for index, hit in enumerate(hits):
        if hit is None:
            out.append(1.0)
            continue
        y, x = divmod(index, w)
        px, py, pz = hit[0], hit[1], hit[2]
        cos, sin = qm.ROTATIONS[qm.BAYER[y % 4][x % 4]]
        count = 0
        for ray, (kx, ky, kz) in enumerate(qm.DIRECTIONS):
            dx, dy, t = kx * cos - ky * sin, kx * sin + ky * cos, phase_of(x, y, ray)
            for j in range(4):
                s = qm.RADIUS * (j + t) / 4
                qx, qy, qz = px + dx * s, py + dy * s, pz + kz * s
                at = project(qx, qy, qz)
                if at is None:
                    continue
                sx, sy = int(math.floor(at[0])), int(math.floor(at[1]))
                if not (0 <= sx < w and 0 <= sy < g.h):
                    continue
                surface = hits[sy * w + sx]
                if surface is None:
                    continue
                ex, ey, ez = surface[0] - px, surface[1] - py, surface[2] - pz
                if surface[2] >= qz + qm.BIAS and ez > qm.BIAS and ex * ex + ey * ey + ez * ez <= r2:
                    count += 1
                    break
        out.append(1.0 - count / 16)
    _RAW[key] = out
    return out


def planes(name, orthographic, phase="c2", passes=2):
    """raw, H and V R8 planes; passes 2 is R3, 1 drops the V pass, 0 drops both."""
    g = c2.grid(controls.GRID_NAMES[name], orthographic)
    raw = [qm.r8(v) for v in model_raw(name, orthographic, phase)]
    h = qm.filter_pass(g, raw, True) if passes >= 1 else raw
    v = qm.filter_pass(g, h, False) if passes >= 2 else h
    return {stage: bytes(round(value * 255) for value in plane) for stage, plane in (("raw", raw), ("h", h), ("v", v))}


def memory_for(make, fixtures=tuple(FIXTURES)):
    """In-memory evidence for the given fixtures; make(scene, orthographic) -> planes."""
    memory = controls.Memory({}, {})
    for gate in fixtures:
        scene = FIXTURES[gate]
        case, on, off, spec, _, orthographic, _, _ = SPECS[gate]
        stage_planes = make(scene, orthographic)
        for label, frame in zip((on, off, spec), controls.frames_for(stage_planes["v"], orthographic)):
            memory.frames[(case, label)] = frame
        memory.planes[(case, on)] = stage_planes
    return memory


def gates(memory, gate):
    case, on, off, spec, scene, orthographic, strength, occludes = SPECS[gate]
    filtered = af.run_gate(af.reference_gate(case, on, off, spec, scene, orthographic, strength, occludes), memory)
    staged = af.run_gate(af.stage_gate(case, on, off, spec, scene, orthographic, strength), memory)
    return filtered, staged


def phase_controls():
    by_number = tuple(sobol(ray) for ray in range(16))
    assert by_number == c2.SOBOL, (by_number, c2.SOBOL)
    for x in range(4):
        for y in range(4):
            phases = [c2.c2_phase(x, y, ray) for ray in range(16)]
            assert phases == [PHASES["c2"](x, y, ray) for ray in range(16)]
            assert sorted(phases) == [(k + 0.5) / 16 for k in range(16)], (x, y)
    pairs = [(qm.BAYER[y][x], c2.rev4(qm.BAYER[y][x])) for y in range(4) for x in range(4)]
    for a in range(5):
        assert len({(r >> (4 - a), p >> a) for r, p in pairs}) == 16, a
    design = [(qm.BAYER[y][x], qm.BAYER[x][y]) for y in range(4) for x in range(4)]
    assert len({(r >> 2, p >> 2) for r, p in design}) == 4
    return {"sobol_table_equals_direction_numbers": True, "independent_phase_equals_c2_phase": True,
            "per_texel_phases_fill_16_strata": True, "tile_pairing_0_4_2_net": True,
            "design_pairing_coarse_cells": 4}


def fidelity_control():
    """C2FilterOracle with the R1/R3 phase equals the shared oracle on every interior texel."""
    class Through(c2.C2FilterOracle):
        @staticmethod
        def phase(x, y, ray):
            return orc.jitter(x, y)

    scene = orc.scene("stripes", False)
    shared, through = orc.FilterOracle(scene), Through(scene)
    compared = 0
    for x, y in af.INTERIOR:
        for surface in scene.candidates((x, y)):
            if surface is not None:
                assert through.k_interval(x, y, surface) == shared.k_interval(x, y, surface), (x, y)
                compared += 1
    return {"fixture": "stripes perspective", "k_intervals_equal": compared}


def positive_controls(memory):
    out = {}
    for gate in FIXTURES:
        filtered, staged = gates(memory, gate)
        assert filtered["pass"] and staged["pass"], (gate, filtered.get("problems"), staged.get("problems"))
        rounding = staged["raw_rounding_measured"]
        assert rounding["exact_raw_texels"] > 0 and rounding["ties"]["down"] == 0, rounding
        assert all(-0.5 <= float(k) <= 0.5 for k in rounding["residual_histogram"]), rounding
        out[gate] = {"filtered": "pass", "stages": "pass", "certainly_occluded": filtered["certainly_occluded"],
                     "oracle_final_widths": filtered["oracle_decisions"]["final_width_histogram"],
                     "raw_rounding_measured": {"exact_raw_texels": rounding["exact_raw_texels"],
                                               "ties": rounding["ties"]}}
    oracle, finals, _ = af.oracle_for("white_slab", False, 1.0)
    stage_planes = dict(memory.planes[(af.CORE, "p_ao_white")])
    frames = [memory.frame(af.CORE, label) for label in ("p_ao_white", "p_off_white", "p_off_black")]
    texel = next((x, y) for x, y in af.INTERIOR if oracle.raw(x, y) == (191, 191))
    raw = bytearray(stage_planes["raw"])
    raw[texel[1] * W + texel[0]] += 1
    perturbed = af.stage_agreement({**stage_planes, "raw": bytes(raw)}, oracle, finals, frames)
    assert not perturbed["pass"] and perturbed["raw_outside"] == 1, perturbed["problems"]
    v_interval = orc.r8_codes(*oracle.pass_value(*texel, False, orc.plane_source(stage_planes["h"])))
    v = bytearray(stage_planes["v"])
    v[texel[1] * W + texel[0]] = min(v_interval[0], finals[texel][0]) - 1
    shifted = af.stage_agreement({**stage_planes, "v": bytes(v)}, oracle, finals, frames)
    assert not shifted["pass"] and shifted["v_outside"] == 1 and shifted["v_outside_final"] == 1, shifted["problems"]
    out["perturbations"] = {"raw_code_plus_1": perturbed["problems"], "final_code_below_interval": shifted["problems"]}
    return out


def masquerade_controls(c2_memory):
    """R1/R3 output cannot pass as C2 wherever the two differ, and the reverse."""
    r1r3 = memory_for(controls.model_planes)
    out = {}
    for gate in FIXTURES:
        case, on = SPECS[gate][0], SPECS[gate][1]
        a, b = r1r3.planes[(case, on)]["raw"], c2_memory.planes[(case, on)]["raw"]
        differ = sum(a[y * W + x] != b[y * W + x] for x, y in af.INTERIOR)
        filtered, staged = gates(r1r3, gate)
        assert differ > 0 and not staged["pass"] and staged["raw_outside"] > 0, (gate, differ)
        out[gate] = {"raw_texels_differing": differ, "c2_stage_pass": staged["pass"],
                     "c2_stage_raw_outside": staged["raw_outside"], "c2_frames_only_pass": filtered["pass"],
                     "c2_frames_only_disagree": filtered["disagree"]}
    for gate in ("ssao_core_p_filtered", "ssao_quality_p_stripes_filtered"):
        case, on, off, spec, scene, orthographic, strength, _ = SPECS[gate]
        oracle, finals, _ = c2.R1R3_ORACLE_FOR(scene, orthographic, strength)
        frames = [c2_memory.frame(case, label) for label in (on, off, spec)]
        reverse_check = af.stage_agreement(c2_memory.planes[(case, on)], oracle, finals, frames)
        assert not reverse_check["pass"] and reverse_check["raw_outside"] > 0, gate
        out[f"{gate}: c2 planes under the shared R1/R3 oracle"] = {"raw_outside": reverse_check["raw_outside"]}
    return r1r3, out


def variant_controls():
    """Wrong phase, base index, ray order or stratum offset, and missing filter passes, all fail."""
    out, chosen = {}, ("ssao_core_p_filtered", "ssao_quality_p_stripes_filtered")
    variants = {name: (name, 2) for name in PHASES if name != "c2"}
    variants.update({"missing_vertical_pass": ("c2", 1), "missing_filter": ("c2", 0)})
    for variant, (phase, passes) in variants.items():
        memory = memory_for(lambda scene, orthographic: planes(scene, orthographic, phase, passes), chosen)
        out[variant] = {}
        for gate in chosen:
            filtered, staged = gates(memory, gate)
            assert not staged["pass"], (variant, gate)
            out[variant][gate] = {"stage_problems": staged["problems"], "frames_only_pass": filtered["pass"]}
    return out


def quality_controls(c2_memory, r1r3_memory):
    out = {}
    for name, memory, rows in (("c2", c2_memory, SCREEN_C2_ROWS), ("r1r3", r1r3_memory, SCREEN_R1R3_ROWS)):
        q = c2.quality(memory)
        for orthographic in (False, True):
            got = [v["ratio"] for _, v in sorted(q["diagnostics"][f"n8_rows_{'ortho' if orthographic else 'persp'}"]
                                                 ["rows"].items(), key=lambda kv: int(kv[0]))]
            assert got == rows[orthographic], (name, orthographic, got)
        assert q["authored_n6"]["verdict"] == "failed" and q["authored_n6"]["max_step_255"] == 7.0, q["authored_n6"]
        out[name] = {"revised_gates": {k: g["pass"] for k, g in q["quality_gates"].items()},
                     "n6t_persp_255": q["quality_gates"]["quality_n6t_revised_persp"]["step_residual_255"],
                     "authored_n6": {"max_step_255": q["authored_n6"]["max_step_255"], "at": q["authored_n6"]["at"],
                                     "verdict": q["authored_n6"]["verdict"]},
                     "n6t_ortho_diagnostic_255": q["diagnostics"]["n6t_ortho"]["step_residual_255"],
                     "n7": [q["quality_gates"][k]["count"] for k in ("quality_n7_white_slab_persp",
                                                                     "quality_n7_white_slab_ortho",
                                                                     "quality_n7_edge_slab_persp")],
                     "n8_rows_persp": q["diagnostics"]["n8_rows_persp"]["rows"],
                     "n8_rows_ortho": q["diagnostics"]["n8_rows_ortho"]["rows"]}
    assert all(out["c2"]["revised_gates"].values()), out["c2"]["revised_gates"]
    assert not out["r1r3"]["revised_gates"]["quality_n8_persp"]
    assert out["c2"]["n6t_persp_255"] == 3.19 and out["r1r3"]["n6t_persp_255"] == 3.37
    return out


def n6t_controls():
    """N6T on fields whose answer is known, with the authored statistic beside it."""
    g = c2.grid("white_slab", False)
    at = lambda x: qr.N6_ROW * g.w + x  # noqa: E731
    span = {at(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)}
    fine = qm.raw_directions(g, qm.support(g, span), *qr.CONTINUOUS_FINE)
    smooth = qm.filtered(g, [fine.get(k) for k in range(len(g.hits))], pixels=span)
    shipped = qm.raw(g, rotate=False, jitter=False, pixels=span)
    banded = [None if v is None else round(v * 16) / 16 for v in smooth]
    hatch = [None if v is None else min(1.0, max(0.0, v + (1 / 32 if (i % g.w) % 2 else -1 / 32)))
             for i, v in enumerate(smooth)]
    step = lambda codes: [v if v is None or i % g.w < 135 else v + codes / 255  # noqa: E731
                          for i, v in enumerate(smooth)]
    fields = {"smooth_fine_limit_r3_r8": (smooth, True, True), "shipped_stepped": (shipped, False, False),
              "shipped_stepped_against_r3_truth": (shipped, True, False), "banded_1_16": (banded, True, False),
              "hatch_pm_1_32": (hatch, True, False), "step_4_codes": (step(4), True, False),
              "step_3_codes": (step(3), True, True)}
    out = {}
    for name, (values, filtered, accepted) in fields.items():
        result = c2.n6t(values, filtered=filtered)
        authored, _ = qm.n6_row(g, values)
        assert result["pass"] is accepted, (name, result)
        out[name] = {"n6t_255": result["step_residual_255"], "n6t_pass": result["pass"],
                     "authored_n6_255": round(authored * 255, 2), "authored_n6_pass": authored <= 1 / 64}
    assert not out["smooth_fine_limit_r3_r8"]["authored_n6_pass"], "the smooth control fails authored N6 after R8"
    return out


def binding_controls(scratch):
    """Tampered reused bytes are refused; a non-C2 shader is refused; the C2 edit is accepted."""
    copy = scratch / "pins"
    for rel in c2.REUSED:
        (copy / rel).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c2.PACKET / rel, copy / rel)
    assert c2.verify_sources(root=copy) == c2.REUSED
    oracle_copy = copy / "ssao-repair/filter/oracle.py"
    oracle_copy.write_bytes(oracle_copy.read_bytes().replace(b"DECISIVE_WIDTH = 3", b"DECISIVE_WIDTH = 4"))
    try:
        c2.verify_sources(root=copy)
        raise AssertionError("a tampered oracle was accepted")
    except c2.SourceBindingError as refusal:
        assert "oracle.py" in str(refusal)
    old = subprocess.run(["git", "show", f"{c2.ENGINE_SOURCE['base']}:{c2.AO_WGSL}"], cwd=c2.PACKET,
                         capture_output=True, text=True, check=True).stdout
    refused = c2.shader_problems(old)
    assert any("R1/R3" in p for p in refused) and any("SOBOL" in p for p in refused), refused
    edited = old.replace("const BAYER=", "const SOBOL=array<u32,16>(0u,8u,12u,4u,10u,2u,6u,14u,15u,7u,3u,11u,5u,13u,"
                                         "9u,1u);\nconst BAYER=")
    edited = edited.replace("let rotation=ao.rotations[BAYER[phase.y*4u+phase.x]].xy;",
                            "let stratum=BAYER[phase.y*4u+phase.x];\n    let rotation=ao.rotations[stratum].xy;")
    edited = edited.replace("let jitter=f32(33u+2u*BAYER[phase.x*4u+phase.y])/64.;",
                            "let base=reverseBits(stratum)>>28u;")
    edited = edited.replace("let k=ao.directions[ray].xyz;",
                            "let jitter=(f32(base^SOBOL[ray])+0.5)/16.;\n        let k=ao.directions[ray].xyz;")
    assert c2.shader_problems(edited) == [], c2.shader_problems(edited)
    frozen = Path(c2.ENGINE_SOURCE["freeze"]).expanduser().parent / "files" / c2.AO_WGSL
    record = {"tampered_reuse_refused": True, "r1r3_shader_refused": refused, "c2_edit_accepted": True}
    if frozen.is_file():
        text = frozen.read_bytes()
        assert hashlib.sha256(text).hexdigest() == c2.ENGINE_SOURCE[c2.AO_WGSL]
        assert c2.shader_problems(text.decode()) == []
        record["root_frozen_shader"] = "present: hash and C2 phase lines match"
    else:
        record["root_frozen_shader"] = "absent on this host (checked at make_plan --engine-commit instead)"
    return record


def report_controls(memory, scratch):
    """c2_analysis.analyse and the 002 CLI refuse incomplete or foreign evidence."""
    out = []
    empty = scratch / "empty"
    (empty / "outputs").mkdir(parents=True)
    report = c2.analyse(empty)
    assert not report["pass"] and report["missing_or_invalid_cases"] == sorted(af.declared_cases())
    assert all(not g["pass"] and "missing" in g for g in report["numeric_gates"].values())
    assert not report["quality_pass"] and report["authored_n6"]["verdict"] == "unavailable"
    out.append("empty evidence: every case missing, every gate fails, authored N6 unavailable")
    for name, record in (("foreign", {"namespace": "0070-ssao-filter-002", "plan_sha256": "0" * 64}),
                         ("ssao-filter-001", {"namespace": "0070-ssao-filter-001", "plan_sha256": hashlib.sha256(
                             (c2.PACKET / "ssao-filter-001" / "plan.json").read_bytes()).hexdigest()})):
        root = scratch / name
        (root / "outputs").mkdir(parents=True)
        (root / "build").mkdir()
        (root / "build" / "build.json").write_text(json.dumps(record))
        bound = c2.analyse(root)
        assert bound["binding"]["plan_matches_build"] is False and bound["numeric_pass"] is False, bound["binding"]
        out.append(f"build record from the {name} plan: numeric_pass false")
    hollow = scratch / "hollow"
    (hollow / "outputs" / af.CORE).mkdir(parents=True)
    (hollow / "outputs" / af.CORE / "index.json").write_text(json.dumps({"frames": [], "summary": {}}))
    frames = {label: memory.frame(af.QUALITY, label) for label in ("p_ao_stripes", "p_off_stripes",
                                                                    "p_off_stripes_black")}
    controls.write_case(hollow / "outputs" / af.QUALITY, frames,
                        {"p_ao_stripes": dict(memory.stages(af.QUALITY, "p_ao_stripes"))})
    index = json.loads((hollow / "outputs" / af.QUALITY / "index.json").read_text())
    (hollow / "outputs" / af.QUALITY / "p_ao_stripes.h.r8").write_bytes(b"")
    for entry in index["frames"]:
        if entry["file"] == "p_ao_stripes.h.r8":
            entry["r8_sha256"] = hashlib.sha256(b"").hexdigest()
    (hollow / "outputs" / af.QUALITY / "index.json").write_text(json.dumps(index))
    ev = af.Evidence(hollow)
    assert ev.status[af.CORE] == {"status": "invalid", "reason": "index.json lists no RGBA frame"}, ev.status
    assert ev.status[af.QUALITY]["status"] == "invalid" and "byte length" in ev.status[af.QUALITY]["reason"]
    out.append("empty index and zero-length plane: cases invalid")
    cli = [sys.executable, "-I", "-B", str(c2.HERE / "analyze.py"), str(empty)]
    first = subprocess.run(cli, capture_output=True, text=True, timeout=900)
    saved = (empty / "filter-analysis.json").read_bytes()
    assert first.returncode == 1 and json.loads(saved)["pass"] is False, (first.returncode, first.stderr[-400:])
    second = subprocess.run(cli, capture_output=True, text=True, timeout=900)
    assert second.returncode == 2 and (empty / "filter-analysis.json").read_bytes() == saved
    out.append("002 CLI: failing report written (exit 1), never replaced (exit 2)")
    return out


def run():
    c2.install()
    scratch = c2.HERE / f".selftest-c2-{os.getpid()}"
    scratch.mkdir()
    try:
        result = {"phase": phase_controls(), "binding": binding_controls(scratch),
                  "fidelity": fidelity_control()}
        c2_memory = memory_for(planes)
        result["positive_independent_c2_model"] = positive_controls(c2_memory)
        r1r3_memory, result["masquerade"] = masquerade_controls(c2_memory)
        result["wrong_variants"] = variant_controls()
        result.update(controls.ambiguous_and_empty(c2_memory))
        result["quality"] = quality_controls(c2_memory, r1r3_memory)
        result["n6t_known_fields"] = n6t_controls()
        result["report"] = report_controls(c2_memory, scratch)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    return result
