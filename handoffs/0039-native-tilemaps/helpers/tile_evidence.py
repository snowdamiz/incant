#!/usr/bin/env python3
"""Assemble committed evidence for handoff 0039 from one run. No engine calls.

Copies the run's small JSON records, the final analysis, the engine call log and
capture ledger, a curated set of unedited engine frames and the derived figures
into handoffs/0039-native-tilemaps/evidence/, then writes shasum-compatible image
hashes and an artifact manifest that lists every ignored artifact by size and
SHA-256. Large regenerable records (the control entity lists) are committed by
hash only, after checking that tile_scene regenerates them byte for byte.
An existing file is never replaced with different content.

Usage (repo root):
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_evidence.py RUN ANALYSIS.json FIGURES_DIR
"""

import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
PACKET = HELPERS.parent
ROOT = HELPERS.parents[2]
EVIDENCE = PACKET / "evidence"
sys.path.insert(0, str(HELPERS))
import tile_scene as ts  # noqa: E402

FRAMES = {"s0--tile": ["pp1-A", "pp1-B", "pp1-C", "pp3-A", "pp3-B", "pp3-C", "of25-A", "of25-B", "of25-C", "o3-C"],
          "s1--tile": ["pp1-A", "pp1-B", "pp3-A", "pp3-B", "of25-A", "of25-B"]}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def place(source_bytes, target):
    target = Path(target)
    if target.exists():
        if target.read_bytes() != source_bytes:
            raise SystemExit(f"refusing to replace {target} with different content")
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(source_bytes)


def copy(source, target):
    place(Path(source).read_bytes(), target)


def regenerate_controls(run):
    build = json.loads((run / "build.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    out = {}
    for state in ("s0", "s1"):
        path = run / f"entities-control-{state}.json"
        entities = ts.control_entities(ts.build(state), ts.Assets("control", build[f"control-{state}"]["ids"],
                                                                   manifest))
        regenerated = (json.dumps(entities, indent=1) + "\n").encode()
        out[path.name] = {"sha256": sha(path), "bytes": path.stat().st_size, "entities": len(entities),
                          "regenerated_identical": regenerated == path.read_bytes()}
    return out


def inventory(directory, base):
    return {str(p.relative_to(base)): {"bytes": p.stat().st_size, "sha256": sha(p)}
            for p in sorted(Path(directory).rglob("*")) if p.is_file()}


def main():
    run, analysis, figures = (Path(a).resolve() for a in sys.argv[1:4])
    if analysis.name.endswith(".summary.json"):
        raise SystemExit("pass the full analysis, not its summary")
    copy(analysis, EVIDENCE / "analysis.json")
    for name in ("build.json", "scene-s0.json", "scene-s1.json", "entities-tile-s0.json"):
        copy(run / name, EVIDENCE / "run" / name)
    for path in sorted((run / "steps").glob("*.json")):
        copy(path, EVIDENCE / "run/steps" / path.name)
    for path in sorted((run / "states").glob("*/record.json")):
        copy(path, EVIDENCE / "run/states" / path.parent.name / "record.json")
    copy(run.parent / "engine-calls.jsonl", EVIDENCE / "engine-calls.jsonl")
    copy(run.parent / "ledger.jsonl", EVIDENCE / "ledger.jsonl")
    controls = regenerate_controls(run)
    if not all(c["regenerated_identical"] for c in controls.values()):
        raise SystemExit(f"control entity lists do not regenerate: {controls}")
    place((json.dumps(controls, indent=1) + "\n").encode(), EVIDENCE / "run/control-entities.json")

    lines = []
    for folder, views in FRAMES.items():
        for view in views:
            w, h = ts.VIEWS[view][1]
            source = run / "states" / folder / f"{view}-{w}x{h}.png"
            target = EVIDENCE / "frames" / f"{folder.split('--')[0]}-{view}-{w}x{h}.png"
            copy(source, target)
            lines.append(f"{sha(target)}  {target.relative_to(EVIDENCE)}")
    for path in sorted(figures.glob("*.png")):
        target = EVIDENCE / "figures" / path.name
        copy(path, target)
        lines.append(f"{sha(target)}  {target.relative_to(EVIDENCE)}")
    place(("\n".join(lines) + "\n").encode(), EVIDENCE / "images.sha256")

    binary = json.loads((ROOT / "artifacts/tools/binary.json").read_text(encoding="utf-8"))
    manifest = {
        "binary": {"path": "artifacts/tools/incant_headless", "sha256": sha(ROOT / "artifacts/tools/incant_headless"),
                   "bytes": (ROOT / "artifacts/tools/incant_headless").stat().st_size, "binary_json": binary},
        "ignored_artifacts_root": "artifacts/0039-native-tilemaps",
        "ignored_artifacts": inventory(ROOT / "artifacts/0039-native-tilemaps", ROOT / "artifacts/0039-native-tilemaps"),
        "committed": {**inventory(PACKET / "art", PACKET), **inventory(PACKET / "helpers", PACKET),
                      **inventory(EVIDENCE, PACKET)},
    }
    manifest["committed"].pop("evidence/artifact-manifest.json", None)
    place((json.dumps(manifest, indent=1) + "\n").encode(), EVIDENCE / "artifact-manifest.json")
    print(json.dumps({"frames": sum(len(v) for v in FRAMES.values()), "figures": len(list(figures.glob('*.png'))),
                      "controls": controls, "ignored_files": len(manifest["ignored_artifacts"]),
                      "committed_files": len(manifest["committed"])}))


if __name__ == "__main__":
    main()
