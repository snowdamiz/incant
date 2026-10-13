"""End-to-end CPU check: the 002 C2 analysis applied, in process and read only,
to the retained 0070-ssao-filter-001 device evidence (R1/R3 output, Apple M5
Pro / Metal). R1/R3 device frames must not pass as C2 wherever the two
samplings differ, and the report must carry authored N6, the revised N6T and
all 14 N8 rows. Nothing is written into the evidence directory; the summary
goes to stdout.

  /opt/homebrew/bin/python3 -I -B c2_on_001.py > c2-on-001-device.json
"""

import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent / "ssao-filter-002")]
import c2_analysis as c2  # noqa: E402

EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001")


def main():
    start = time.time()
    before = sorted(p.name for p in EVIDENCE.iterdir())
    report = c2.analyse(EVIDENCE)
    assert sorted(p.name for p in EVIDENCE.iterdir()) == before, "the evidence directory changed"
    numeric = report["numeric_gates"]
    rows = lambda key: {r: (v["ratio"], v["pass"]) for r, v in report["diagnostics"][key]["rows"].items()}  # noqa: E731
    summary = {
        "format": "incant-0070-c2-analysis-of-001-device-v1",
        "evidence": str(EVIDENCE), "binding": report["binding"],
        "numeric_pass": report["numeric_pass"],
        "failed_numeric_gates": report["failed_numeric_gates"],
        "passed_numeric_gates": sorted(n for n, g in numeric.items() if g["pass"]),
        "reference_disagreements": {n: g.get("disagree") for n, g in numeric.items() if "disagree" in g},
        "stage_raw_outside": {n: g.get("raw_outside") for n, g in numeric.items() if "raw_outside" in g},
        "quality_pass": report["quality_pass"], "failed_quality_gates": report["failed_quality_gates"],
        "authored_n6": {k: report["authored_n6"].get(k) for k in ("max_step_255", "at", "verdict")},
        "authored_quality_pass": report["authored_quality_pass"],
        "n6t_revised_persp_255": report["quality_gates"]["quality_n6t_revised_persp"].get("step_residual_255"),
        "n6t_ortho_diagnostic_255": report["diagnostics"]["n6t_ortho"].get("step_residual_255"),
        "n7": {k: report["quality_gates"][k].get("count") for k in report["quality_gates"] if k.startswith("quality_n7")},
        "n8_rows_persp": rows("n8_rows_persp"), "n8_rows_ortho": rows("n8_rows_ortho"),
        "pass": report["pass"], "seconds": round(time.time() - start, 1),
    }
    json.dump(summary, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
