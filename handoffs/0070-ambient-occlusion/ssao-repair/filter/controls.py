"""Focused synthetic controls for analyze_filter.py (run with --selftest).

Positive: R8 stage planes from the INDEPENDENT f64 model (../quality_model.py)
must satisfy every stage check against the interval oracle, and the analyzer's
quality gates must reproduce the addendum's CPU-domain verdicts (N6 fails, N7
passes, N8 fails in perspective and passes orthographic).
Negative: missing, empty and ambiguous evidence, perturbed codes, a fully
ambiguous oracle and an inflated oracle must all fail, never pass vacuously;
the CLI must write a failing report, exit 1, and refuse to replace it (exit 2).
"""

import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

import analyze_filter as af
import oracle as orc

base, qm = af.base, af.qm
W, H = af.SIZE
DIFFUSE, SPECULAR = base.LIGHT * (1 - base.REFLECTANCE), base.LIGHT * base.REFLECTANCE
GRID_NAMES = {"white_slab": "white_slab", "edge_slab": "edge_slab", "stripes": af.qr.STRIPES}


def model_planes(name, orthographic):
    """raw, H and V R8 planes of the independent f64 model (round-half-up writes)."""
    grid = qm.Grid(GRID_NAMES[name], orthographic)
    raw = [qm.r8(v) for v in qm.raw(grid)]
    h = qm.filter_pass(grid, raw, True)
    v = qm.filter_pass(grid, h, False)
    return {stage: bytes(round(value * 255) for value in plane) for stage, plane in (("raw", raw), ("h", h), ("v", v))}


def frames_for(final, orthographic):
    """On / white / black output frames whose accessibility is the final plane."""
    meta = {"scene": {"key_light": 0.0, "environment": {"linear_byte": 179, "intensity": 1.0},
                      "projection": "orthographic" if orthographic else "perspective"},
            "viewport": {"rect": list(af.RECT)}}
    on = base._filled(af.SIZE, base.OFF_BYTE)
    for x, y in af.INTERIOR:
        base._put(on, af.SIZE, x, y, base._byte(DIFFUSE * final[y * W + x] / 255 + SPECULAR))
    return [base.Frame(af.SIZE, bytes(on), meta), base.Frame(af.SIZE, base._gray(base.OFF_BYTE).data, meta),
            base.Frame(af.SIZE, base._gray(base.SPEC_BYTE).data, meta)]


class Memory:
    """In-memory evidence with the analyzer's frame/stages interface."""

    def __init__(self, frames, planes):
        self.frames, self.planes = frames, planes

    def frame(self, case, label):
        if (case, label) not in self.frames:
            raise af.Missing(f"{case}: frame {label} absent")
        return self.frames[(case, label)]

    def stages(self, case, label):
        found = self.planes.get((case, label), {})
        absent = [s for s in ("raw", "h", "v") if s not in found]
        if absent:
            raise af.Missing(f"{case}: R8 stage planes {absent} of {label} absent")
        return found


def positive_and_perturbed():
    out = {}
    specs = {name: spec for name, *spec in af.REFERENCE_GATES}
    chosen = {"ssao_core_p_filtered": "white_slab", "ssao_core_o_filtered": "white_slab",
              "ssao_core_edge_filtered": "edge_slab", "ssao_quality_p_stripes_filtered": "stripes",
              "ssao_quality_o_stripes_filtered": "stripes"}
    memory = Memory({}, {})
    for gate, scene in chosen.items():
        case, on, off, spec, scene_name, orthographic, strength, occludes = specs[gate]
        planes = model_planes(scene, orthographic)
        frames = frames_for(planes["v"], orthographic)
        for label, frame in zip((on, off, spec), frames):
            memory.frames[(case, label)] = frame
        memory.planes[(case, on)] = planes
        filtered = af.run_gate(af.reference_gate(case, on, off, spec, scene_name, orthographic, strength, occludes),
                               memory)
        staged = af.run_gate(af.stage_gate(case, on, off, spec, scene_name, orthographic, strength), memory)
        assert filtered["pass"] and staged["pass"], (gate, filtered.get("problems"), staged.get("problems"))
        out[gate] = {"filtered": "pass", "stages": "pass", "certainly_occluded": filtered["certainly_occluded"],
                     "oracle_final_widths": filtered["oracle_decisions"]["final_width_histogram"]}
    # One raw code off by one where the oracle is exact: only the raw stage can see it.
    oracle, _, _ = af.oracle_for("white_slab", False, 1.0)
    planes = dict(memory.planes[(af.CORE, "p_ao_white")])
    texel = next((x, y) for x, y in af.INTERIOR if oracle.raw(x, y) == (191, 191))
    raw = bytearray(planes["raw"])
    raw[texel[1] * W + texel[0]] += 1
    perturbed = af.stage_agreement({**planes, "raw": bytes(raw)}, oracle,
                                   af.oracle_for("white_slab", False, 1.0)[1],
                                   [memory.frame(af.CORE, label) for label in ("p_ao_white", "p_off_white", "p_off_black")])
    assert not perturbed["pass"] and perturbed["raw_outside"] == 1, perturbed["problems"]
    # A final code one below both its V-stage interval and the oracle's final interval.
    finals = af.oracle_for("white_slab", False, 1.0)[1]
    v_interval = orc.r8_codes(*oracle.pass_value(*texel, False, orc.plane_source(planes["h"])))
    v = bytearray(planes["v"])
    v[texel[1] * W + texel[0]] = min(v_interval[0], finals[texel][0]) - 1
    shifted = af.stage_agreement({**planes, "v": bytes(v)}, oracle, finals,
                                 [memory.frame(af.CORE, label) for label in ("p_ao_white", "p_off_white", "p_off_black")])
    assert not shifted["pass"] and shifted["v_outside"] == 1 and shifted["v_outside_final"] == 1, shifted["problems"]
    out["perturbations"] = {"raw_code_plus_1": perturbed["problems"], "final_code_below_interval": shifted["problems"]}
    # Quality gates on the model's read-back-equivalent planes reproduce the addendum.
    quality = {name: af.run_gate(gate, memory) for name, gate in af.QUALITY_GATES.items()}
    expected = {"quality_n6_authored": False, "quality_n7_white_slab_persp": True,
                "quality_n7_white_slab_ortho": True, "quality_n7_edge_slab_persp": True,
                "quality_n8_persp": False, "quality_n8_ortho": True}
    assert {k: g["pass"] for k, g in quality.items()} == expected, {k: g for k, g in quality.items()}
    diagnostic = af.run_gate(af.n6t_diagnostic, memory)
    out["quality_reproduces_addendum"] = {
        "n6_max_step_255": quality["quality_n6_authored"]["max_step_255"],
        "n7_counts": [quality[k]["count"] for k in ("quality_n7_white_slab_persp", "quality_n7_white_slab_ortho",
                                                    "quality_n7_edge_slab_persp")],
        "n8_ratios": [round(quality[k]["ratio"], 3) for k in ("quality_n8_persp", "quality_n8_ortho")],
        "n6t_step_residual_255": diagnostic["step_residual_255"]}
    return memory, out


def ambiguous_and_empty(memory):
    out = {}
    case, on, off, spec, scene, orthographic, strength, occludes = next(
        s for n, *s in af.REFERENCE_GATES if n == "ssao_core_p_filtered")
    oracle, finals, _ = af.oracle_for(scene, orthographic, strength)
    frames = [memory.frame(case, label) for label in (on, off, spec)]
    contract = base.basis_contract(frames, oracle.scene)
    # Conflicting RGB channels: an empty inference is invalid, not skipped.
    conflicted = bytearray(frames[0].data)
    for x in range(40, 60):
        for y in range(40, 60):
            i = (y * W + x) * 4
            conflicted[i + 1] = base.OFF_BYTE if conflicted[i] != base.OFF_BYTE else base.SPEC_BYTE + 40
    result = af.filtered_agreement(base.Frame(af.SIZE, bytes(conflicted), frames[0].meta), frames[1], frames[2],
                                   oracle, finals, True, contract)
    assert not result["pass"] and result["rejected"]["empty"] > 0, result["rejected"]
    # No diffuse signal anywhere: every pixel uninferable fails coverage.
    blind = af.filtered_agreement(frames[0], frames[2], frames[2], oracle, finals, True, contract)
    assert not blind["pass"] and blind["compared"] == 0, blind["problems"]
    # A fully ambiguous oracle (every ray "?") cannot certify occlusion.
    vague = orc.FilterOracle(oracle.scene)
    vague.k_interval = lambda x, y, surface: (0, 16)
    vague_finals = {p: vague.v(*p) for p in af.INTERIOR}
    vacuous = af.filtered_agreement(*frames, vague, vague_finals, True, contract)
    assert not vacuous["pass"] and any("certainly occluded" in p for p in vacuous["problems"]), vacuous["problems"]
    # Inflated slack: widening decisive intervals beyond the derived bound is refused.
    inflated = {p: (max(0, lo - 2), min(255, hi + 2)) for p, (lo, hi) in finals.items()}
    wide = af.filtered_agreement(*frames, oracle, inflated, True, contract)
    assert not wide["pass"] and wide["decisive_too_wide"] > 0, wide["problems"]
    # An absence control may not contain occlusion.
    absent = af.filtered_agreement(*frames, oracle, finals, False, contract)
    assert not absent["pass"] and any("absence control" in p for p in absent["problems"]), absent["problems"]
    out["ambiguous"] = ["conflicting RGB (empty inference)", "no diffuse signal (coverage)",
                        "every ray ambiguous (no certain occlusion)", "inflated decisive widths", "absence mismatch"]
    # Missing planes: stage and quality gates fail with the reason; frames alone do not pass them.
    partial = Memory(dict(memory.frames), {k: {s: p for s, p in v.items() if s != "h"}
                                           for k, v in memory.planes.items()})
    gate = af.run_gate(af.stage_gate(case, on, off, spec, scene, orthographic, strength), partial)
    assert not gate["pass"] and "['h']" in gate["missing"], gate
    assert all(not af.run_gate(g, Memory(dict(memory.frames), {}))["pass"] for g in af.QUALITY_GATES.values())
    missing_frame = Memory({k: v for k, v in memory.frames.items() if k != (case, off)}, memory.planes)
    gate = af.run_gate(af.reference_gate(case, on, off, spec, scene, orthographic, strength, occludes), missing_frame)
    assert not gate["pass"] and "p_off_white absent" in gate["missing"], gate
    out["missing"] = ["stage plane", "all planes (quality gates)", "frame"]
    return out


def write_case(directory, frames, planes):
    directory.mkdir(parents=True)
    entries = []
    for label, frame in frames.items():
        (directory / f"{label}.rgba").write_bytes(frame.data)
        entries.append({"label": label, "file": f"{label}.rgba", "size": list(frame.size),
                        "rgba_sha256": hashlib.sha256(frame.data).hexdigest(), "meta": frame.meta})
    for label, stages in planes.items():
        for stage, data in stages.items():
            name = f"{label}.{stage}.r8"
            (directory / name).write_bytes(data)
            entries.append({"label": f"{label}.{stage}", "file": name, "size": list(af.SIZE),
                            "r8_sha256": hashlib.sha256(data).hexdigest(), "meta": {"frame": label, "stage": stage}})
    (directory / "index.json").write_text(json.dumps({"frames": entries, "summary": {}}))


def report_controls(memory):
    out = []
    scratch = af.HERE / f".filter-selftest-{os.getpid()}"
    scratch.mkdir()
    try:
        # Empty evidence: every declared case missing; every gate fails; the report survives.
        empty = scratch / "empty"
        (empty / "outputs").mkdir(parents=True)
        report = af.analyse(empty)
        assert not report["pass"] and report["missing_or_invalid_cases"] == sorted(af.declared_cases())
        assert all(not g["pass"] and "missing" in g for g in report["numeric_gates"].values())
        out.append("empty evidence: every case missing, every gate fails")
        # Empty index and a zero-length plane make their cases invalid, not partially passing.
        hollow = scratch / "hollow"
        (hollow / "outputs" / af.CORE).mkdir(parents=True)
        (hollow / "outputs" / af.CORE / "index.json").write_text(json.dumps({"frames": [], "summary": {}}))
        frames = {label: memory.frame(af.QUALITY, label) for label in ("p_ao_stripes", "p_off_stripes",
                                                                        "p_off_stripes_black")}
        planes = {"p_ao_stripes": dict(memory.stages(af.QUALITY, "p_ao_stripes"))}
        write_case(hollow / "outputs" / af.QUALITY, frames, planes)
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
        # The CLI writes a failing report, exits 1, and never replaces it (exit 2).
        cli = [sys.executable, "-I", "-B", str(af.HERE / "analyze_filter.py"), str(empty)]
        first = subprocess.run(cli, capture_output=True, text=True, timeout=900)
        saved = (empty / "filter-analysis.json").read_bytes()
        assert first.returncode == 1 and json.loads(saved)["pass"] is False, (first.returncode, first.stderr[-400:])
        second = subprocess.run(cli, capture_output=True, text=True, timeout=900)
        assert second.returncode == 2 and (empty / "filter-analysis.json").read_bytes() == saved
        out.append("CLI: failing report written (exit 1), never replaced (exit 2)")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    return out


def run():
    memory, positive = positive_and_perturbed()
    return {"positive_independent_model": positive, **ambiguous_and_empty(memory),
            "report": report_controls(memory)}
