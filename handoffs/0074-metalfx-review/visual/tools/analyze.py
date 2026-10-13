#!/usr/bin/env python3
"""CPU-only evidence check and publication for handoff 0074 stage B.
Standard library only; never starts a renderer or reads the GPU. Adapted from
handoff 0072's committed visual/tools/analyze.py (visual tree ac8a48de).

  analyze.py check     after the review cases, also when stopped: re-verify every
                       ledger row's logs, reports and PNG hashes and summarize the
                       contract checks and observations into a new analysis file
  analyze.py publish   after the GPU and target releases: copy the retained PNGs
                       (<= 20, hash-verified, never overwritten) into the worktree

Personal inspection of every retained PNG is still required; this file only
proves which pixels were produced and that nothing was lost or replaced.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402


def rows():
    if not c.LEDGER.exists():
        return []
    return [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]


def check():
    _, grant_sha = c.grant()
    problems, cases, suites = [], {}, {}
    for row in rows():
        for key in ("stdout", "stderr"):
            if c.sha(row[key]) != row[key + "_sha256"]:
                problems.append(f"{row['label']}: {key} changed")
        if row["kind"] == "suite":
            suites[row["item"]] = {"failed": row["failed"], "libtest": row["libtest"]}
        if row["kind"] != "review":
            continue
        output = Path(row["output"])
        report = output / "report.json"
        if row.get("report_sha256") and c.sha(report) != row["report_sha256"]:
            problems.append(f"{row['label']}: report.json changed")
        for name, digest in row.get("pngs", {}).items():
            if c.sha(output / name) != digest:
                problems.append(f"{row['label']}: {name} changed")
        body = c.read_json(report) if report.exists() else {}
        cases[row["item"]] = {
            "failed": row["failed"], "outcome": body.get("outcome"), "checks": body.get("checks"),
            "frames_rendered": body.get("frames_rendered"),
            "ui_submissions": body.get("ui_submissions"), "pngs": row.get("pngs", {}),
            "adapter": body.get("renderer", {}).get("adapter"),
            "observations": body.get("observations")}
    summary = {"recorded": c.now(), "grant_sha256": grant_sha,
               "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
               "budget": c.budget_used(), "debits": len(c.budget_rows()),
               "stopped": c.read_json(c.STOP) if c.STOP.exists() else None,
               "gpu_released": c.LATCH.exists(), "problems": problems,
               "suites": suites, "cases": cases,
               "pngs_total": sum(len(case["pngs"]) for case in cases.values())}
    path = c.EVIDENCE / "analysis" / ("analysis-" + summary["recorded"].replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({k: summary[k] for k in ("budget", "stopped", "problems", "pngs_total")},
                     indent=1))
    print("wrote", path)
    if problems:
        raise SystemExit("evidence problems found; nothing is published")


def publish():
    data, _ = c.grant()
    if not c.LATCH.exists():
        c.refuse("release the GPU (gpu.py release) before publishing")
    lease = c.read_json(c.LEASE) if c.LEASE.exists() else {}
    if not lease.get("released_at"):
        c.refuse("release the target lease before publishing")
    retained = [(Path(row["output"]), name, digest) for row in rows() if row["kind"] == "review"
                for name, digest in row.get("pngs", {}).items()]
    if len(retained) > c.plan()["budgets"]["pngs_max"]:
        c.refuse(f"{len(retained)} PNGs exceed the retained budget")
    if c.PUBLISHED.exists():
        c.refuse(f"{c.PUBLISHED} exists; publication is never repeated")
    c.PUBLISHED.mkdir(parents=True)
    manifest = []
    for output, name, digest in retained:
        source = output / name
        if c.sha(source) != digest:
            c.refuse(f"{source} changed since its run")
        target = c.PUBLISHED / name
        with open(target, "xb") as out:
            out.write(source.read_bytes())
        if c.sha(target) != digest:
            c.refuse(f"copy mismatch for {target}")
        manifest.append({"name": name, "sha256": digest, "source": str(source)})
    c.write_new_json(c.PUBLISHED / "manifest.json", {"grant_sha256": c.sha(c.GRANT),
                                                    "visual_tree": data["visual_tree"],
                                                    "pngs": manifest, "recorded": c.now()})
    print(json.dumps(manifest, indent=1))


if __name__ == "__main__":
    steps = {"check": check, "publish": publish}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
