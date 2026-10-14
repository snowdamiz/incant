#!/usr/bin/env python3
"""Compare two capture sets of handoff 0077 (PNG bytes and measured fields of shared states).

Usage (worktree root): /opt/homebrew/bin/python3 -I handoffs/0077-project-file-ui/tools/compare_sets.py r4 r5 OUT.json
"""
import hashlib, json, sys
from pathlib import Path

base = Path("artifacts/0077-project-file-ui")
a, b, out = base / sys.argv[1], base / sys.argv[2], Path(sys.argv[3])
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
pngs = {p.name: {"a": sha(p), "b": sha(b / p.name) if (b / p.name).exists() else None} for p in sorted(a.glob("*.png"))}
ra = {c["file"]: c for c in json.loads((a / "report.json").read_text())["captures"]}
rb = {c["file"]: c for c in json.loads((b / "report.json").read_text())["captures"]}
keys = ["dialog", "notice", "status", "menu", "projectButtonLabel", "projectName", "active", "titlebar", "projectButton", "panels", "window"]
changed = [[f, k] for f, c in ra.items() for k in keys if c["measured"].get(k) != rb[f]["measured"].get(k)]
changed += [[f, "calls"] for f, c in ra.items() if c["calls"] != rb[f]["calls"]]
report = {
    "a": sys.argv[1], "b": sys.argv[2],
    "png_identical": sum(1 for v in pngs.values() if v["a"] == v["b"]),
    "png_different": [name for name, v in pngs.items() if v["a"] != v["b"]],
    "measured_differences": changed,
    "new_in_b": sorted(set(rb) - set(ra)),
    "pngs": pngs,
}
out.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({k: report[k] for k in ("png_identical", "png_different", "measured_differences", "new_in_b")}))
