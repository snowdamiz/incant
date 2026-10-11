"""Write tangent-delta/captures-complete.json once every candidate child has exited.

Reads only the separate delta ledger. Requires every recorded child to have an exit
status (timed-out children are killed and reaped by engine.call) and no running
process whose command line contains the candidate executable path. Launches no
engine process. Usage: python3 -I delta_release.py
"""
import datetime
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CANDIDATE = json.loads((ROOT / "artifacts/tools/tangent-candidate.json").read_text())
BINARY = ROOT / "artifacts/tools/incant_headless-tangent"
DELTA = ROOT / "artifacts/0040-static-fbx-import/tangent-delta"


def running_candidates(binary=BINARY):
    """PIDs whose full command line contains the candidate path (pgrep exits 1 when none)."""
    found = subprocess.run(["pgrep", "-f", str(binary)], capture_output=True, text=True)
    if found.returncode not in (0, 1):
        raise RuntimeError(f"pgrep failed: {found.stderr.strip()}")
    return [int(p) for p in found.stdout.split()]


def summary(records, running):
    children = [{"seq": r["seq"], "run": r["run"], "label": r["label"], "pid": r["pid"], "exit": r["exit"],
                 "timed_out": r["timed_out"], "kind": r.get("kind"), "output_sha256": r.get("output_sha256")}
                for r in records]
    shots = [r for r in records if r.get("kind") == "screenshot" and r.get("output_bytes")]
    problems = [c for c in children if c["exit"] is None]
    if problems or running:
        raise SystemExit(f"not complete: unexited={problems} running={running}")
    return {"children": children, "captures": len(shots), "raw_bytes": sum(r["output_bytes"] for r in shots),
            "adapters": sorted({r.get("adapter") for r in shots}),
            "candidate_hash_checked_every_child": all(
                r["binary_sha256_before"] == CANDIDATE["sha256"] == r["binary_sha256_after"] for r in records)}


def main():
    records = [json.loads(line) for line in (DELTA / "ledger.jsonl").read_text().splitlines() if line.strip()]
    result = summary(records, running_candidates())
    result.update({
        "written_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "candidate": {"source_commit": CANDIDATE["source_commit"], "sha256": CANDIDATE["sha256"],
                      "binary": str(BINARY.relative_to(ROOT))},
        "process_inventory": f"no running process command line contains {BINARY.relative_to(ROOT)}",
        "release": "All candidate renderer and CLI children have exited and were reaped. "
                   "The graphics slot can be released; Claude will launch no further renderer for this delta.",
    })
    out = DELTA / "captures-complete.json"
    out.write_text(json.dumps(result, indent=1) + "\n")
    print(out.relative_to(ROOT), result["captures"], "captures,", len(result["children"]), "children")


if __name__ == "__main__":
    main()
