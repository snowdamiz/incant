#!/usr/bin/env python3
"""Copy the reviewed handoff 0034 evidence into the packet folder. Standard library only.

Selects unedited engine frames, derived figures, analyses, helper records, the art and
the packet budget ledger from ignored artifacts/0034-sprite-atlas-review/ runs, writes
budget.json and screenshots.sha256. Existing identical files are kept; it refuses to
replace any file with different content.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_evidence.py              # round one
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_evidence.py --followup   # follow-up round
"""

import hashlib
import json
import shutil
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
PACKET = TOOLS.parent
ROOT = TOOLS.parents[2]
A = ROOT / "artifacts/0034-sprite-atlas-review"
EVIDENCE, SHOTS = PACKET / "evidence", PACKET / "screenshots"

ENGINE = {
    "v1-board-s-stable-frames-pp-960x540.png": "run-2/states/s0-r1/pp-S-960x540.png",
    "v1-board-t-transforms-pp-960x540.png": "run-2/states/s0-r1/pp-T-960x540.png",
    "v1-board-n-nine-slice-pp-960x540.png": "run-2/states/s0-r1/pp-N-960x540.png",
    "v1-board-o-order-alpha-pp-960x540.png": "run-2/states/s0-r1/pp-O-960x540.png",
    "v1-board-l-linear-pp-960x540.png": "run-2/states/s0-r1/pp-L-960x540.png",
    "v1-board-b-identity-pp-960x540.png": "run-2/states/s0-r1/pp-B-960x540.png",
    "v1-board-w-strips-1x-pp-1280x200.png": "run-2/states/s0-r1/pp-W-1280x200.png",
    "v1-board-v-strips-1x-pp-320x1080.png": "run-2/states/s0-r1/pp-V-320x1080.png",
    "v1-board-x-magnified-pp-1024x832.png": "run-2/states/s0-r1/pp-X-1024x832.png",
    "v1-board-y-magnified-pp-832x1024.png": "run-2/states/s0-r1/pp-Y-832x1024.png",
    "v1-board-s-ortho-2.5x-offset-640x360.png": "run-2/states/s0-r1/of-S-640x360.png",
    "v1-board-s-ortho-exact-3x-960x540.png": "run-2/states/s0-r1/o3-S-960x540.png",
    "v1-board-n-ortho-roll7-640x360.png": "run-2/states/s0-r1/rl-N-640x360.png",
    "v1-board-w-ortho-roll3-1280x288.png": "run-2/states/s0-r1/rl-W-1280x288.png",
    "v2-board-b-relocated-pp-320x180.png": "run-2/states/s1-v2/pp-B-320x180.png",
    "v2-board-b-relocated-roll7-640x360.png": "run-2/states/s1-v2/rl-B-640x360.png",
    "v3-board-b-repacked-pp-320x180.png": "run-2/states/s5-v3/pp-B-320x180.png",
    "v3-board-b-repacked-ortho-2.5x-640x360.png": "run-2/states/s5-v3/of-B-640x360.png",
    "extra-board-z-packed-x6266-pp-1280x240.png": "extra-1/states/atlas-r1/pp-Z-1280x240.png",
    "upper-span-source-x-36x-56x-pp-1920x1080.png": "upper-1/states/atlas/pp-U-1920x1080.png",
    "upper-span-source-y-36x-56x-pp-1920x1080.png": "upper-2-swapped/states/atlas/pp-UV-1920x1080.png",
}
FIGURES = ["art-contact-sheet.png", "atlas-pages-v1-v2-v3-quarter.png", "board-b-v1-v2-v3-zoom3.png",
           "decoded-upper-y-40x-cut-ties.png", "decoded-x-cuts-1024x832.png", "decoded-y-cuts-832x1024.png",
           "linear-diff-rl-L-v1-v3-x64.png"]
JSON = {
    "analysis-run-2.json": ("analysis/final-run-2c.json", True),
    "helper-run-2.json": ("run-2/helper.json", True),
    "helper-run-3.json": ("run-3/helper.json", True),
    "scene-atlas-run-2.json": ("run-2/scene-atlas.json", False),
    "extra/helper.json": ("extra-1/helper.json", True),
    "extra/analysis.json": ("analysis/extra-1.json", True),
    "extra/scene.json": ("extra-1/scene-extra.json", False),
    "upper/summary.json": ("analysis/upper-span-stats.json", True),
    "controls-run-2.json": ("analysis/controls-run-2.json", True),
    "upper/decode-upper-1.json": ("upper-1/decode.json", False),
    "upper/decode-upper-2-swapped.json": ("upper-2-swapped/decode.json", False),
    "packing/packing-probe.json": ("packing-probe-1/packing-probe.json", True),
    "packing/packing-repro.json": ("packing-repro-1/packing-repro.json", True),
}


FU = "followup"
FU_ENGINE = {f"v1-board-{b.lower()}-pp-320x180.png": f"followup/fu-1/states/f0-v1-r1/pp-{b}-320x180.png"
             for b in "STNOLB"}
FU_ENGINE.update({
    "v1-board-w-strips-1x-pp-1280x200.png": "followup/fu-1/states/f0-v1-r1/pp-W-1280x200.png",
    "v1-board-x-magnified-pp-1024x832.png": "followup/fu-1/states/f0-v1-r1/pp-X-1024x832.png",
    "v1-board-y-magnified-pp-832x1024.png": "followup/fu-1/states/f0-v1-r1/pp-Y-832x1024.png",
    "v1-board-w-ortho-roll3-1280x288.png": "followup/fu-1/states/f0-v1-r1/rl-W-1280x288.png",
    "v1-board-v-ortho-roll3-360x1080.png": "followup/fu-1/states/f0-v1-r1/rl-V-360x1080.png",
    "v2-board-b-relocated-pp-320x180.png": "followup/fu-1/states/f1-v2/pp-B-320x180.png",
    "v2-board-b-relocated-roll7-640x360.png": "followup/fu-1/states/f1-v2/rl-B-640x360.png",
    "v3-board-b-repacked-pp-320x180.png": "followup/fu-1/states/f4-v3/pp-B-320x180.png",
    "v3-board-b-repacked-ortho-2.5x-640x360.png": "followup/fu-1/states/f4-v3/of-B-640x360.png",
    "watch-mixed-board-o-orange-badge-pp-320x180.png": "followup/fu-1/states/f7-watch-mixed/pp-O-320x180.png",
    "extra-board-z-packed-x6266-pp-1280x240.png": "followup/extra-1/states/atlas-r1/pp-Z-1280x240.png",
    "upper-span-source-x-36x-56x-pp-1920x1080.png": "followup/upper-1/states/atlas/pp-U-1920x1080.png",
    "upper-span-source-y-36x-56x-pp-1920x1080.png": "followup/upper-2-swapped/states/atlas/pp-UV-1920x1080.png",
})
FU_FIGURES = ["atlas-pages-v1-v2-v3-quarter.png", "board-b-v1-v2-v3-zoom3.png", "decoded-upper-y-40x-cut-ties.png",
              "decoded-x-cuts-1024x832.png", "decoded-y-cuts-832x1024.png", "knife-edges-control-rl-V-360x1080.png",
              "knife-edges-control-rl-W-1280x288.png", "linear-diff-rl-L-v1-v3-x64.png"]
FU_JSON = {
    "analysis-fu-1.json": ("followup/analysis/fu-1-final.json", True),
    "helper-fu-1.json": ("followup/fu-1/helper.json", False),
    "scene-atlas-fu-1.json": ("followup/fu-1/scene-atlas.json", False),
    "import-diagnostics.json": ("followup/import-diag-1/import-diagnostics.json", True),
    "rejection-messages.json": ("followup/rejection-messages.json", True),
    "packing/packing-residual.json": ("followup/packing-residual-1/packing-residual.json", True),
    "packing/packing-repro.json": ("followup/packing-repro-1/packing-repro.json", True),
    "packing/packing-probe.json": ("followup/packing-probe-1/packing-probe.json", True),
    "packing/packing-sweep.json": ("followup/packing-sweep-1/packing-sweep.json", True),
    "packing/feasible-1217x1104.json": ("followup/feasible-1217x1104-tool.json", True),
    "upper/summary.json": ("followup/analysis/upper-span-stats.json", True),
    "upper/helper-upper-1.json": ("followup/upper-1/helper.json", False),
    "upper/helper-upper-2-swapped.json": ("followup/upper-2-swapped/helper.json", False),
    "upper/decode-upper-1.json": ("followup/upper-1/decode.json", False),
    "upper/decode-upper-2-swapped.json": ("followup/upper-2-swapped/decode.json", False),
    "extra/helper.json": ("followup/extra-1/helper.json", True),
    "extra/analysis.json": ("followup/analysis/extra-1.json", True),
}


def ledger_totals(path):
    runs = {}
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        e = json.loads(line)
        r = runs.setdefault(e["run"], {"frames": 0, "raw_bytes": 0})
        r["frames"] += e["frames"]
        r["raw_bytes"] += e["raw"]
    total = {"frames": sum(r["frames"] for r in runs.values()), "raw_bytes": sum(r["raw_bytes"] for r in runs.values())}
    return runs, total


def followup():
    ev, shots = EVIDENCE / FU, SHOTS / FU
    for name, rel in FU_ENGINE.items():
        put(shots / "engine" / name, src=A / rel)
    for name in FU_FIGURES:
        put(shots / "figures" / name, src=A / "followup/figures-1" / name)
    for name, (rel, indent) in FU_JSON.items():
        value = json.loads((A / rel).read_text(encoding="utf-8"))
        put(ev / name, json.dumps(value, indent=1 if indent else None, separators=None if indent else (",", ":"))
            + "\n")
    put(ev / "art-followup/drop-unused.atlas.json", src=A / "followup/fu-1/art-followup/drop-unused.atlas.json")
    runs, total = ledger_totals(A / "ledger-followup.jsonl")
    r1_runs, r1_total = ledger_totals(A / "ledger.jsonl")
    ledger = [json.loads(line) for line in (A / "ledger-followup.jsonl").read_text(encoding="utf-8").splitlines()]
    put(ev / "budget.json", json.dumps({
        "followup_limits": {"per_invocation": {"frames": 128, "raw_mib": 256}, "round": {"frames": 600, "raw_mib": 512}},
        "followup_runs": runs, "followup_total": total, "followup_total_raw_mib": round(total["raw_bytes"] / 2 ** 20, 1),
        "largest_frame_raw_bytes": max(e["raw"] for e in ledger), "frames_per_invocation": 1,
        "round1_total": r1_total, "round1_total_raw_mib": round(r1_total["raw_bytes"] / 2 ** 20, 1),
        "lifetime_total": {"frames": total["frames"] + r1_total["frames"],
                           "raw_bytes": total["raw_bytes"] + r1_total["raw_bytes"]},
        "lifetime_total_raw_mib": round((total["raw_bytes"] + r1_total["raw_bytes"]) / 2 ** 20, 1),
        "overwrites": 0}, indent=1) + "\n")
    put(ev / "budget-ledger-followup.jsonl", src=A / "ledger-followup.jsonl")
    lines = [f"{sha(p)}  {p.relative_to(PACKET)}" for p in sorted(shots.rglob("*.png"))]
    put(ev / "screenshots.sha256", "\n".join(lines) + "\n")
    print(json.dumps({"engine_frames": len(FU_ENGINE), "figures": len(FU_FIGURES), "json": len(FU_JSON),
                      "followup_budget": total, "round1_budget": r1_total}))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def put(dst, data=None, src=None):
    """Write once. An existing identical file is kept; any different content is refused."""
    dst.parent.mkdir(parents=True, exist_ok=True)
    content = Path(src).read_bytes() if src is not None else data.encode("utf-8")
    if dst.exists():
        if dst.read_bytes() != content:
            raise SystemExit(f"refusing to overwrite {dst} with different content")
        return
    dst.write_bytes(content)


def main():
    for name, rel in ENGINE.items():
        put(SHOTS / "engine" / name, src=A / rel)
    for name in FIGURES:
        put(SHOTS / "figures" / name, src=A / "figures-3" / name)
    for name, (rel, indent) in JSON.items():
        value = json.loads((A / rel).read_text(encoding="utf-8"))
        put(EVIDENCE / name, json.dumps(value, indent=1 if indent else None, separators=None if indent else (",", ":"))
            + "\n")
    for p in sorted((A / "run-2/art").iterdir()):
        put(EVIDENCE / "art" / p.name, src=p)
    for p in sorted((A / "packing-repro-1/art").iterdir()):
        put(EVIDENCE / "packing/repro-art" / p.name, src=p)
    # Upper-span provenance: upper-1 kept its full helper; the swapped run's summary was
    # written under a row name by a since-fixed naming slip, without end-of-run fields.
    upper1 = json.loads((A / "upper-1/helper.json").read_text(encoding="utf-8"))
    swapped = json.loads((A / "upper-2-swapped/UV:56x:506").read_text(encoding="utf-8"))
    ledger = [json.loads(line) for line in (A / "ledger.jsonl").read_text(encoding="utf-8").splitlines()]
    put(EVIDENCE / "upper/provenance.json", json.dumps({
        "upper-1": {k: upper1[k] for k in ("binary_sha256_start", "binary_sha256_end", "binary_unchanged_every_call",
                                           "engine_calls", "planned")},
        "upper-2-swapped": {"binary_sha256_start": swapped["binary_sha256_start"], "planned": swapped["planned"],
                            "board_centres": swapped["board_centres"],
                            "frames": [e for e in ledger if e["run"] == "upper-2-swapped"],
                            "note": "Every engine call verified the binary hash before and after and aborts on a "
                                    "mismatch; the run completed. Its summary file was misnamed 'UV:56x:506' by a "
                                    "since-fixed variable-shadowing slip and lacks end-of-run fields."}}, indent=1) + "\n")
    runs = {}
    for e in ledger:
        r = runs.setdefault(e["run"], {"frames": 0, "raw_bytes": 0})
        r["frames"] += e["frames"]
        r["raw_bytes"] += e["raw"]
    total = {"frames": sum(r["frames"] for r in runs.values()), "raw_bytes": sum(r["raw_bytes"] for r in runs.values())}
    put(EVIDENCE / "budget.json", json.dumps({
        "limits": {"per_invocation": {"frames": 128, "raw_mib": 256}, "packet": {"frames": 2048, "raw_mib": 1024}},
        "largest_frame_raw_bytes": max(e["raw"] for e in ledger), "frames_per_invocation": 1,
        "runs": runs, "total": total, "total_raw_mib": round(total["raw_bytes"] / 2 ** 20, 1),
        "overwrites": 0}, indent=1) + "\n")
    put(EVIDENCE / "budget-ledger.jsonl", src=A / "ledger.jsonl")
    lines = [f"{sha(p)}  {p.relative_to(PACKET)}" for p in sorted(SHOTS.rglob("*.png"))]
    put(EVIDENCE / "screenshots.sha256", "\n".join(lines) + "\n")
    print(json.dumps({"engine_frames": len(ENGINE), "figures": len(FIGURES), "json": len(JSON),
                      "budget": total}))


if __name__ == "__main__":
    import sys
    followup() if "--followup" in sys.argv[1:] else main()
