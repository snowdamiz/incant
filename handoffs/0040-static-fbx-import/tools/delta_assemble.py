"""Copy selected unedited tangent-delta frames and evidence into the packet (CPU only).

Engine frames are copied byte for byte and must equal the delta ledger's output
hashes. DERIVED- figures are difference maps, not engine frames. Writes only under
screenshots/tangent-delta and evidence/tangent-delta; original evidence is untouched.
Usage: python3 -I delta_assemble.py DELTA_RUN   (e.g. d1)
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = Path(__file__).resolve().parents[1]
DELTA = ROOT / "artifacts/0040-static-fbx-import/tangent-delta"
SHOTS = PACKET / "screenshots/tangent-delta"
EVIDENCE = PACKET / "evidence/tangent-delta"
ENGINE = [
    ("textures/frames/controls_ortho.png", "candidate-controls-colour-and-normal-ortho.png"),
    ("textures/frames/controls_ortho-source-free.png", "candidate-controls-ortho-source-free.png"),
    ("textures/frames/instances_ortho.png", "candidate-fbx-instances-ortho.png"),
    ("textures/frames/instances_front_right_top.png", "candidate-fbx-instances-front-right-top.png"),
    ("textures/frames/instances_back_left_bottom.png", "candidate-fbx-instances-back-left-bottom.png"),
    ("textures-twin/frames/instances_ortho.png", "candidate-twin-maya-tangents-instances-ortho.png"),
]
DERIVED = [
    "DERIVED-diff-candidate-fbx-vs-twin-instances_front_right_top.png",
    "DERIVED-diff-candidate-fbx-vs-twin-instances_back_left_bottom.png",
    "DERIVED-diff-candidate-vs-original-fbx-instances_front_right_top.png",
    "DERIVED-diff-candidate-twin-vs-original-twin-controls_front_right_top.png",
]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main(run):
    ledger = [json.loads(line) for line in (DELTA / "ledger.jsonl").read_text().splitlines() if line.strip()]
    by_output = {r["output"]: r for r in ledger if r.get("output")}
    lines, index = [], []
    for sub, items in [("engine", [(DELTA / f"runs/{run}" / s, n) for s, n in ENGINE]),
                       ("derived", [(DELTA / "derived" / n, n) for n in DERIVED])]:
        (SHOTS / sub).mkdir(parents=True, exist_ok=True)
        for source, name in items:
            target = SHOTS / sub / name
            shutil.copyfile(source, target)
            digest = sha(target)
            entry = {"file": f"screenshots/tangent-delta/{sub}/{name}", "sha256": digest,
                     "source": str(source.relative_to(ROOT))}
            if sub == "engine":
                record = by_output[str(source.relative_to(ROOT))]
                if record["output_sha256"] != digest:
                    raise SystemExit(f"{name}: copy differs from the delta ledger output hash")
                entry.update({"ledger_seq": record["seq"], "pid": record["pid"], "camera": record["camera"],
                              "adapter": record["adapter"], "width": record["width"], "height": record["height"],
                              "binary_sha256": record["binary_sha256_after"]})
            index.append(entry)
            lines.append(f"{digest}  screenshots/tangent-delta/{sub}/{name}")
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "screenshots.sha256").write_text("\n".join(lines) + "\n")
    (EVIDENCE / "screenshots.json").write_text(json.dumps(index, indent=1) + "\n")
    shutil.copyfile(DELTA / "ledger.jsonl", EVIDENCE / "ledger.jsonl")
    shutil.copyfile(DELTA / "captures-complete.json", EVIDENCE / "captures-complete.json")
    print(f"{len(lines)} screenshots copied; ledger and captures-complete copied")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "d1")
