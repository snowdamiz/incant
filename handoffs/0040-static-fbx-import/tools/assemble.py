"""Copy the selected unedited frames and evidence into the handoff 0040 packet.

Engine frames are copied byte for byte (their hashes must equal the ledger's output
hashes). Derived figures carry a DERIVED- prefix; UI captures come from the headless
browser fixture and are not engine pixels. Writes evidence/screenshots.sha256.
Usage: python3 -I handoffs/0040-static-fbx-import/tools/assemble.py
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import engine  # noqa: E402

PACKET = Path(__file__).resolve().parents[1]
SHOTS = PACKET / "screenshots"
RUNS = engine.ART / "runs"
ENGINE_FRAMES = [
    ("r1/static-fbx/frames/overview.png", "static-overview-true-scale.png"),
    ("r1/static-fbx/frames/suzanne_front.png", "static-suzanne-front.png"),
    ("r1/static-fbx/frames/suzanne_profile.png", "static-suzanne-profile.png"),
    ("r1/static-fbx/frames/max_3q.png", "static-max-zup-inch-instances-3q.png"),
    ("r1/static-fbx/frames/max_top.png", "static-max-zup-inch-instances-top.png"),
    ("r1/static-fbx/frames/shear_mirror_3q.png", "static-maya-mirrored-nonuniform-parent-sheared-child.png"),
    ("r1/static-fbx/frames/units.png", "static-maya-1cm-cube-on-1cm-ruler.png"),
    ("r1/textures/frames/controls_ortho.png", "textures-controls-colour-and-normal-ortho.png"),
    ("r1/textures/frames/instances_ortho.png", "textures-fbx-instances-ortho.png"),
    ("r1/textures-twin/frames/instances_ortho.png", "textures-twin-maya-tangents-instances-ortho.png"),
    ("r1/textures/frames/instances_front_right_top.png", "textures-fbx-instances-front-right-top.png"),
    ("r2/reimport/frames/s0-r3q.png", "reimport-s0-v1.png"),
    ("r2/reimport/frames/s3-r3q.png", "reimport-s3-v2-and-dependency-v2.png"),
    ("r2/reimport/frames/s10-rfront.png", "reimport-s10-source-free-fresh-process.png"),
    ("r5/materials/frames/materials_ortho.png", "materials-lambert-cases-vs-expected-gltf.png"),
    ("r5/materials/frames/materials_3q.png", "materials-lambert-cases-3q.png"),
]
DERIVED = [("derived/DERIVED-diff-fbx-vs-twin-instances_front_right_top.png", "DERIVED-diff-fbx-vs-twin-instances-front-right-top.png"),
           ("derived/DERIVED-diff-fbx-vs-twin-instances_back_left_bottom.png", "DERIVED-diff-fbx-vs-twin-instances-back-left-bottom.png")]
UI = [("ui/shots/ui-import-form-fbx-after-stale-usage.png", "ui-import-form-fbx-after-stale-usage.png"),
      ("ui/shots/ui-import-row-fbx-upper.png", "ui-import-row-fbx-upper.png")]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    ledger = {r["output"]: r for r in engine.ledger() if r.get("output")}
    lines, index = [], []
    for sub, items, base in [("engine", ENGINE_FRAMES, RUNS), ("derived", DERIVED, engine.ART), ("ui", UI, engine.ART)]:
        (SHOTS / sub).mkdir(parents=True, exist_ok=True)
        for src, name in items:
            source = base / src
            target = SHOTS / sub / name
            shutil.copyfile(source, target)
            digest = sha(target)
            entry = {"file": f"screenshots/{sub}/{name}", "sha256": digest, "source": str(source.relative_to(engine.ROOT))}
            if sub == "engine":
                record = ledger[str(source.relative_to(engine.ROOT))]
                if record["output_sha256"] != digest:
                    raise SystemExit(f"{name}: copy differs from the ledger's output hash")
                entry.update({"ledger_seq": record["seq"], "camera": record["camera"], "adapter": record["adapter"],
                              "width": record["width"], "height": record["height"], "project_state": record["project_state"]})
            index.append(entry)
            lines.append(f"{digest}  screenshots/{sub}/{name}")
    (PACKET / "evidence/screenshots.sha256").write_text("\n".join(lines) + "\n")
    (PACKET / "evidence/screenshots.json").write_text(json.dumps(index, indent=1) + "\n")
    shutil.copyfile(engine.LEDGER, PACKET / "evidence/ledger.jsonl")
    shutil.copyfile(engine.ART / "sources/manifest.json", PACKET / "evidence/source-manifest.json")
    # Refusal records from the CPU-only probe imports (state after = next call's state before).
    records = engine.ledger()
    refusals = []
    for i, r in enumerate(records):
        if r.get("run") == "probe" and r.get("kind") == "import":
            following = next((n for n in records[i + 1:] if n.get("run") == "probe" and n.get("state_before")), None)
            after = following["state_before"] if following else None
            refusals.append({"source": r["source"], "exit": r["exit"], "stderr": r["stderr"].strip(),
                             "project_and_journal_unchanged": None if after is None or r["exit"] == 0 else
                             (after["project_sha256"] == r["state_before"]["project_sha256"]
                              and after["journal_sha256"] == r["state_before"]["journal_sha256"])})
    (PACKET / "evidence/probe-imports.json").write_text(json.dumps(refusals, indent=1) + "\n")
    print(f"{len(lines)} screenshots, {len(refusals)} probe imports")


if __name__ == "__main__":
    main()
