"""C2 analysis for 0070-ssao-filter-002 (after release only; CPU, standard library).

Root selected C2 on 2026-10-13: the Bayer rotation stratum r = B[y % 4][x % 4]
is unchanged; each ray's step phase is the stratum's bit reversal digitally
shifted by the ray's second Sobol' coordinate,

    t_ray = ((rev4(r) ^ SOBOL[ray]) + 0.5) / 16        (exact in f32; at most 64 probes)

This is the minimal local adaptation of the shared analyzer; nothing shared is
edited:

* every reused module is imported read only, and only if its bytes equal the
  pins in REUSED (the bytes 0070-ssao-filter-001 was analysed with);
* C2FilterOracle overrides FilterOracle.k_interval alone, moving the phase into
  the ray loop. R8 code intervals, tie handling, plane weights, tolerance bands,
  ambiguity decisions and the decisive-width bound are inherited unchanged;
* install() points analyze_filter's oracle factory at C2 and its plan check at
  this namespace's plan. Every numeric gate (filtered reference, R8 stage
  readback, identity, DRS, mixed pane) is otherwise the shared one;
* quality, as root adopted it (REVISED_MEASURE): N6T on the authored 70 pairs of
  white_slab perspective, bound 1/64, R3 taps, replaces authored N6 as the
  scoped measure. Authored N6 is still computed in every report with its actual
  value and verdict, next to 0070-ssao-filter-001's retained failure.
  Orthographic N6T is a named diagnostic. N8 keeps its 0.25 bound on the ROI
  rows, and all 14 screened rows (perspective 124-130, orthographic 110-116)
  are reported with their verdicts.

  python3 -I -B analyze.py <evidence-dir>    # writes <evidence-dir>/filter-analysis.json
  python3 -I -B analyze.py --selftest        # c2_controls.py
"""

import hashlib
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
sys.path[:0] = [str(HERE), str(PACKET / "ssao-repair" / "filter"), str(PACKET / "ssao-repair"),
                str(PACKET / "tools")]

# Reused read only; bytes unchanged since the 001 analysis (relative to the packet directory).
REUSED = {
    "ssao-repair/filter/analyze_filter.py": "d04732cdf32e27097c11da4f633e7663b1afd6c398c5ce4872e11d5c5b2a0207",
    "ssao-repair/filter/oracle.py": "bdc81363181da907eae789c06b6f00807c71d3db08bf0b5e62cdecba7bb5b8d4",
    "ssao-repair/filter/controls.py": "2b4794afc8d92856434c897f8af8bcb4a04410a0bd8a06780653f47967a3c1e0",
    "ssao-repair/filter/cases.json": "fa6d1076a08e54c47843f26367b51998a4035fd33a004495c480b4f31c5716d7",
    "ssao-repair/quality_model.py": "e7d083e55cf4d8b0c7c1527954a655f7d4230c6434ce143c5f30f44f2fa5c190",
    "ssao-repair/quality_report.py": "882953de7e9cc7fbdfde8c4440086b8b2a879b84d0cca1424ab08cea07646277",
    "ssao-repair/sphere_rise_bound.py": "e1bba7f144e5aa309dd88369bc5ab5e60146335b5539c5bcc966c358aa4fc93d",
    "tools/analyze.py": "91245f64046f7d1ddeeb4b5e1c5a524da4813a8ccfa0a58d2b35f84791831889",
    "tools/ssao_reference.py": "8b0f50eb7061d5f09ad53d4cd304569cfab06ace74745a8f2cb4e324971fbe0a",
}


class SourceBindingError(Exception):
    """A reused module's bytes differ from its pin."""


def verify_sources(root=PACKET, pins=None):
    pins = REUSED if pins is None else pins
    actual = {rel: hashlib.sha256((root / rel).read_bytes()).hexdigest() if (root / rel).is_file() else None
              for rel in pins}
    wrong = sorted(rel for rel, sha in pins.items() if actual[rel] != sha)
    if wrong:
        raise SourceBindingError(f"reused sources differ from their pins: {wrong}")
    return actual


verify_sources()
import analyze_filter as af  # noqa: E402  (ssao-repair/filter, read only)
import oracle as orc  # noqa: E402

qm, qr, ref = af.qm, af.qr, orc.ref

# Root's frozen C2 engine source (uncommitted; based on 5dfc522e). The pinned
# engine commit must carry exactly this shader (make_plan.py verify_engine).
AO_WGSL = "crates/incant_render/src/ambient_occlusion/ao.wgsl"
ENGINE_SOURCE = {
    "freeze": "~/.codex/incant-evidence/ssao-c2-sampling-001/source-001/freeze.json",
    "base": "5dfc522ec164f2bc50d760f12db57eae233f33d8",
    "prescription": "766dc101 next-repair-preparation.md section 4",
    AO_WGSL: "4aaa0cc0ed375555fb0069a974c79d7360337f3eebdebee43a87de58be94c6b6",
    "crates/incant_render/src/ambient_occlusion/quality_oracle.rs":
        "be5dc86c02027a1a530cb7df2df3be300b859a1a0b79c5f415d6e2490bf94f9b",
    "crates/incant_render/src/ambient_occlusion/quality_tests.rs":
        "0ae711accc64189bfdd40b4a54ff07f8fcd62d0f6cd11242a5d2224608aae2e7",
}
SOBOL = (0, 8, 12, 4, 10, 2, 6, 14, 15, 7, 3, 11, 5, 13, 9, 1)
SHADER_LINES = ("let stratum=BAYER[phase.y*4u+phase.x];", "let rotation=ao.rotations[stratum].xy;",
                "let base=reverseBits(stratum)>>28u;", "let jitter=(f32(base^SOBOL[ray])+0.5)/16.;")


def shader_problems(text):
    """What keeps an ao.wgsl source from being the C2 shader this oracle models."""
    problems = []
    table = re.findall(r"const SOBOL=array<u32,16>\(([^)]*)\);", text)
    if len(table) != 1:
        problems.append(f"{len(table)} SOBOL tables")
    elif tuple(int(v.strip().rstrip("u")) for v in table[0].split(",")) != SOBOL:
        problems.append(f"SOBOL table {table[0]} differs from {SOBOL}")
    problems += [f"missing `{line}`" for line in SHADER_LINES if text.count(line) != 1]
    if "33u+2u*BAYER" in text:
        problems.append("the R1/R3 step phase is still present")
    return problems


def rev4(value):
    return int(f"{value:04b}"[::-1], 2)


def c2_phase(x, y, ray):
    """ao.wgsl's jitter: ((reverseBits(stratum) >> 28) ^ SOBOL[ray] + 0.5) / 16."""
    return ((rev4(orc.BAYER[y % 4][x % 4]) ^ SOBOL[ray]) + 0.5) / 16


class C2FilterOracle(orc.FilterOracle):
    """The shared FilterOracle with a per-ray phase; only k_interval differs."""

    @staticmethod
    def phase(x, y, ray):
        return c2_phase(x, y, ray)

    def k_interval(self, x, y, surface):
        key = (x, y, surface)
        if key in self._k:
            return self._k[key]
        p, n = surface
        axis = (0.0, 1.0, 0.0) if abs(n[2]) > 0.9 else (0.0, 0.0, 1.0)
        tangent = ref.normalize(ref.cross(axis, n))
        bitangent = ref.cross(n, tangent)
        c, s = orc.rotation(x, y)
        rt = tuple(tangent[i] * c + bitangent[i] * s for i in range(3))
        rb = tuple(bitangent[i] * c - tangent[i] * s for i in range(3))
        k_lo = k_hi = 0
        for ray, k in enumerate(ref.DIRECTIONS):
            t = self.phase(x, y, ray)
            d = tuple(rt[i] * k[0] + rb[i] * k[1] + n[i] * k[2] for i in range(3))
            blocked = maybe = False
            for step in range(1, ref.STEPS + 1):
                q = tuple(p[i] + d[i] * (self.radius * (step - 1 + t) / ref.STEPS) for i in range(3))
                outcome = ref._step(self.scene, p, n, q, self.radius, self.bias)
                if outcome is True:
                    blocked = True
                    break
                if outcome == "?":
                    maybe = True
            k_lo += blocked
            k_hi += blocked or maybe
        self._k[key] = (k_lo, k_hi)
        return self._k[key]


_ORACLES = {}
R1R3_ORACLE_FOR = af.oracle_for  # the shared factory, kept for the masquerade controls


def oracle_for(scene_name, orthographic, strength):
    """analyze_filter.oracle_for with C2FilterOracle and its own cache."""
    scene = orc.scene(scene_name, orthographic)
    key = (orthographic, tuple((r.x, r.y, r.z, r.normal) for r in scene.rects), scene.edges, strength)
    if key not in _ORACLES:
        oracle = C2FilterOracle(scene, strength)
        finals = {p: oracle.v(*p) for p in af.INTERIOR}
        _ORACLES[key] = (oracle, finals, oracle.summary())
    return _ORACLES[key]


def install():
    """Bind the shared analyzer to C2 and to this namespace's plan."""
    af.oracle_for = oracle_for
    af.NAMESPACE_PLAN = HERE / "plan.json"


# ------------------------------------------------------------------ quality

REVISED_MEASURE = {
    "adopted_by": "root, 2026-10-13",
    "review": {"path": "~/.codex/incant-reviews/ssao-n6t-metric-root-20261013.json",
               "sha256": "22ea6c42ab9e7763912277f0c0380a616856236a3602b5b5207134036e652bc9"},
    "control_replay_sha256": "ce63d89dfe4f9be12dfb32b2e0e5921cf053a5d2d4e32468c92092d1aaf8c2ef",
    "scope": "N6T on the authored 70 pairs of white_slab perspective row 97, bound 1/64, R3 taps",
    "definition": "max |(A(x+1) - A(x)) - (T(x+1) - T(x))|, T the continuous limit (24 x 64 directions, "
                  "32 steps) through the R3 taps without R8 rounding",
    "authored_n6": "computed and reported in every report with its actual value and verdict; never waived",
    "unchanged": "N7 and N8 definitions and bounds (N8 0.25 on the ROI rows)",
}
RETAINED_001 = {"quality_n6_authored": {"max_step_255": 7.0, "at": 161, "bound": 1 / 64, "pass": False},
                "source": "0070-ssao-filter-001 filter-analysis.json, sha256 "
                          "3eb3c8781c00d16c509f99a528331ad37a851501dd78e45f251711632ae69b79"}
_TRUTH, _GRIDS = {}, {}


def grid(name, orthographic):
    if (name, orthographic) not in _GRIDS:
        _GRIDS[(name, orthographic)] = qm.Grid(name, orthographic)
    return _GRIDS[(name, orthographic)]


def n6t(values, orthographic=False, filtered=True):
    """Step residual against the continuous limit on the authored N6 pairs.
    filtered=False compares an unfiltered output with the unfiltered limit."""
    g = grid("white_slab", orthographic)
    at = lambda x: qr.N6_ROW * g.w + x  # noqa: E731
    key = (orthographic, filtered)
    if key not in _TRUTH:
        span = {at(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)}
        raw = qm.raw_directions(g, qm.support(g, span), *qr.CONTINUOUS)
        raw = [raw.get(k) for k in range(len(g.hits))]
        _TRUTH[key] = (qm.filtered(g, raw, quantize=False, pixels=span) if filtered else raw, qr.n6_pairs(g))
    truth, pairs = _TRUTH[key]
    residual = {x: (values[at(x + 1)] - values[at(x)]) - (truth[at(x + 1)] - truth[at(x)]) for x in pairs}
    worst = max(residual, key=lambda x: abs(residual[x]))
    value = abs(residual[worst])
    return {"step_residual": value, "step_residual_255": round(value * 255, 2), "at": worst, "pairs": len(pairs),
            "bound": 1 / 64, "pass": value <= 1 / 64}


def n8_rows(values, orthographic):
    """The N8 window on the ROI row and the next six rows, each against the
    shipped algorithm's own peak-to-peak on that row; bound 0.25 per row."""
    g = grid(qr.STRIPES, orthographic)
    row, cols = qr.n8_roi(g)
    rows = {}
    for r in range(row, row + 7):
        window = [r * g.w + x for x in cols]
        old = qm.raw(g, rotate=False, jitter=False, pixels=set(window))
        shipped = max(old[i] for i in window) - min(old[i] for i in window)
        p2p = max(values[i] for i in window) - min(values[i] for i in window)
        rows[str(r)] = {"p2p_255": round(p2p * 255, 2), "shipped_255": round(shipped * 255, 2),
                        "ratio": round(p2p / shipped, 3), "pass": p2p <= 0.25 * shipped}
    return {"roi_row": row, "cols": [cols[0], cols[-1]], "rows": rows,
            "failing_rows": [r for r, v in rows.items() if not v["pass"]],
            "pass": all(v["pass"] for v in rows.values())}


REVISED_GATES = {
    "quality_n6t_revised_persp": lambda ev: n6t(af.final_values(ev, af.CORE, "p_ao_white")),
    **{name: af.QUALITY_GATES[name] for name in ("quality_n7_white_slab_persp", "quality_n7_white_slab_ortho",
                                                 "quality_n7_edge_slab_persp", "quality_n8_persp",
                                                 "quality_n8_ortho")},
}
AUTHORED_GATES = {"quality_n6_authored": af.QUALITY_GATES["quality_n6_authored"]}
DIAGNOSTICS = {
    "n6t_ortho": lambda ev: n6t(af.final_values(ev, af.CORE, "o_ao_white"), orthographic=True),
    "n8_rows_persp": lambda ev: n8_rows(af.final_values(ev, af.QUALITY, "p_ao_stripes"), False),
    "n8_rows_ortho": lambda ev: n8_rows(af.final_values(ev, af.QUALITY, "o_ao_stripes"), True),
}


def quality(ev):
    """Revised gates, the retained authored N6 and the named diagnostics."""
    revised = {name: af.run_gate(gate, ev) for name, gate in REVISED_GATES.items()}
    authored = af.run_gate(AUTHORED_GATES["quality_n6_authored"], ev)
    diagnostics = {name: af.run_gate(gate, ev) for name, gate in DIAGNOSTICS.items()}
    others = [g["pass"] for name, g in revised.items() if name != "quality_n6t_revised_persp"]
    return {"quality_gates": revised, "failed_quality_gates": sorted(n for n, g in revised.items() if not g["pass"]),
            "authored_n6": {**authored, "verdict": "passed" if authored["pass"] else
                            "unavailable" if "missing" in authored or "error" in authored else "failed",
                            "role": "authored metric, reported with its actual value; not part of the revised "
                                    "measure and never waived", "retained_001": RETAINED_001},
            "authored_quality_pass": authored["pass"] and all(others),
            "diagnostics": diagnostics}


def sampling():
    return {"candidate": "C2", "rotation": "unchanged: B[y % 4][x % 4] strata",
            "phase": "t_ray = ((rev4(B[y % 4][x % 4]) ^ SOBOL[ray]) + 0.5) / 16", "sobol": list(SOBOL),
            "probes_max": 64, "oracle": "C2FilterOracle (k_interval only)", "engine_source": ENGINE_SOURCE,
            "reused_sources": REUSED}


def analyse(evidence):
    install()
    ev = af.Evidence(evidence)
    numeric = {name: af.run_gate(gate, ev) for name, gate in af.NUMERIC_GATES.items()}
    q = quality(ev)
    binding = af.run_binding(evidence)
    missing = sorted(c for c, s in ev.status.items() if s["status"] != "loaded")
    failed_numeric = sorted(n for n, g in numeric.items() if not g["pass"])
    numeric_pass = binding["plan_matches_build"] is not False and not missing and not failed_numeric
    quality_pass = not missing and not q["failed_quality_gates"]
    return {"format": "incant-0070-ssao-filter-002-analysis-v1", "evidence": str(evidence), "sampling": sampling(),
            "binding": binding, "declared_cases": af.declared_cases(), "cases": ev.status,
            "missing_or_invalid_cases": missing, "numeric_gates": numeric, "failed_numeric_gates": failed_numeric,
            "numeric_pass": numeric_pass, "revised_measure": REVISED_MEASURE, **q, "quality_pass": quality_pass,
            "authored_quality_pass": not missing and q["authored_quality_pass"],
            "appearance": {"judged_here": False, "note": "Claude's rendered-pair review is separate."},
            "acceptance": "none: these are measurements for review; GPU quality and appearance stay open",
            "pass": numeric_pass and quality_pass}


def main(argv=None):
    """Exit status: 0 numeric and revised quality pass, 1 otherwise (report written), 2 usage."""
    argv = sys.argv[1:] if argv is None else argv
    if argv == ["--selftest"]:
        import c2_controls
        print(json.dumps(c2_controls.run(), indent=1))
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
    rows = {k: report["diagnostics"][k].get("failing_rows") for k in ("n8_rows_persp", "n8_rows_ortho")}
    print(json.dumps({**{key: report[key] for key in ("pass", "numeric_pass", "quality_pass", "authored_quality_pass",
                                                      "missing_or_invalid_cases", "failed_numeric_gates",
                                                      "failed_quality_gates")},
                      "authored_n6": {k: report["authored_n6"].get(k) for k in ("max_step_255", "verdict")},
                      "n8_failing_rows": rows}, indent=1))
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
