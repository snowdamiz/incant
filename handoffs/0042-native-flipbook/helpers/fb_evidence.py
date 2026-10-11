#!/usr/bin/env python3
"""Assemble committed evidence for handoff 0042 from one run. No engine calls.

Copies the run's JSON records (build, scene, requests, capture records, sidecar
reports, source-free record, animated entity list), the final analysis, the call
log and capture ledger, a curated set of unedited native_frames PNGs and the
derived figures into handoffs/0042-native-flipbook/evidence/. Control entity lists
are committed by hash after checking that fb_scene regenerates them byte for
byte. Writes shasum-compatible image hashes and an artifact manifest listing every
committed file and every ignored artifact with size and SHA-256. An existing file
is never replaced with different content.

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/fb_evidence.py RUN ANALYSIS.json FIGURES_DIR
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
PACKET = HELPERS.parent
ROOT = HELPERS.parents[2]
EVIDENCE = PACKET / "evidence"
sys.path.insert(0, str(HELPERS))
import fb_scene as fs  # noqa: E402

FRAMES = {"main": [0, 9, 17, 25, 26, 34, 42, 69, 2100], "ortho": [17]}


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


def regenerate_entities(run):
    build = json.loads((run / "build.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    sc = fs.build()
    out = {}
    for name, record in build.items():
        if not isinstance(record, dict) or "cooked" not in record:
            continue
        assets = fs.Assets(record["ids"], manifest)
        entities = fs.animated_entities(sc, assets) if name == "anim" else \
            fs.control_entities(sc, assets, int(name[len("ctl-t"):]))
        path = run / "entities" / f"{name}.json"
        regenerated = (json.dumps(entities, indent=1) + "\n").encode()
        out[f"{name}.json"] = {"sha256": sha(path), "bytes": path.stat().st_size, "entities": len(entities),
                               "regenerated_identical": regenerated == path.read_bytes()}
    return out


def optional_json(path):
    """Coordinator files may be withdrawn after release; record what exists now."""
    path = Path(path)
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def inventory(directory, base):
    return {str(p.relative_to(base)): {"bytes": p.stat().st_size, "sha256": sha(p)}
            for p in sorted(Path(directory).rglob("*")) if p.is_file()}


def main():
    run, analysis, figures = (Path(a).resolve() for a in sys.argv[1:4])
    copy(analysis, EVIDENCE / "analysis.json")
    for name in ("build.json", "scene.json", "source-free.json"):
        copy(run / name, EVIDENCE / "run" / name)
    copy(run / "entities/anim.json", EVIDENCE / "run/entities-anim.json")
    for path in sorted((run / "requests").glob("*.json")):
        copy(path, EVIDENCE / "run/requests" / path.name)
    for path in sorted((run / "records").glob("*.json")):
        copy(path, EVIDENCE / "run/records" / path.name)
    for folder in sorted((run / "captures").iterdir()):
        copy(folder / "report.json", EVIDENCE / "run/reports" / f"{folder.name}.json")
    copy(run.parent / "engine-calls.jsonl", EVIDENCE / "engine-calls.jsonl")
    copy(run.parent / "ledger.jsonl", EVIDENCE / "ledger.jsonl")
    entities = regenerate_entities(run)
    if not all(e["regenerated_identical"] for e in entities.values()):
        raise SystemExit(f"entity lists do not regenerate: {entities}")
    place((json.dumps(entities, indent=1) + "\n").encode(), EVIDENCE / "run/entity-lists.json")

    lines = []
    for capture, ticks in FRAMES.items():
        for tick in ticks:
            source = run / "captures" / capture / f"tick-{tick:05d}.png"
            target = EVIDENCE / "frames" / f"{capture}-t{tick:05d}-640x360.png"
            copy(source, target)
            lines.append(f"{sha(target)}  {target.relative_to(EVIDENCE)}")
    for path in sorted(figures.glob("*.png")):
        target = EVIDENCE / "figures" / path.name
        copy(path, target)
        lines.append(f"{sha(target)}  {target.relative_to(EVIDENCE)}")
    place(("\n".join(lines) + "\n").encode(), EVIDENCE / "images.sha256")

    tools = ROOT / "artifacts/tools"
    ignored = ROOT / "artifacts/0042-native-flipbook"
    manifest = {
        "binary_json": json.loads((tools / "binary.json").read_text(encoding="utf-8")),
        "tools": {name: {"sha256": sha(tools / name), "bytes": (tools / name).stat().st_size}
                  for name in ("incant_headless", "cook_scene", "native_frames")},
        "gpu_review_ready_at_assembly": optional_json(tools / "gpu-review-ready"),
        "gpu_review_release_at_assembly": optional_json(tools / "gpu-review-release.json"),
        "capture_py_sha256": sha(PACKET / "capture.py"),
        "capture_budget_file": json.loads((ROOT / "artifacts/flipbook-capture-budget.json").read_text()),
        "ignored_artifacts_root": "artifacts/0042-native-flipbook",
        "ignored_artifacts": inventory(ignored, ignored),
        "committed": {**inventory(PACKET / "art", PACKET), **inventory(PACKET / "helpers", PACKET),
                      **inventory(EVIDENCE, PACKET)},
    }
    manifest["committed"].pop("evidence/artifact-manifest.json", None)
    place((json.dumps(manifest, indent=1) + "\n").encode(), EVIDENCE / "artifact-manifest.json")
    print(json.dumps({"frames": sum(len(v) for v in FRAMES.values()), "figures": len(list(figures.glob("*.png"))),
                      "entity_lists": len(entities), "ignored_files": len(manifest["ignored_artifacts"]),
                      "committed_files": len(manifest["committed"])}))


if __name__ == "__main__":
    main()
