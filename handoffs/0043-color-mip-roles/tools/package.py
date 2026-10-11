#!/usr/bin/env python3
"""CPU-only: copy unedited engine frames and sanitized evidence into the packet.

Frames are copied byte for byte; a frame is only listed as standing for another
state after their bytes are checked equal. Launches no engine process.
Usage (worktree root): python3 -I -B handoffs/0043-color-mip-roles/tools/package.py
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
PACKET = TOOLS.parent
ROOT = TOOLS.parents[2]
ART = ROOT / "artifacts/0043-color-mip-roles"
EVIDENCE, SHOTS = PACKET / "evidence", PACKET / "screenshots"
ENGINE = {  # packet name: (state, frame, other states whose frame must be byte-identical)
    "old-materials-board.png": ("old", "materials-board", ["retained"]),
    "recooked-materials-board.png": ("recooked", "materials-board", ["sourcefree"]),
    "old-materials-oblique.png": ("old", "materials-oblique", ["retained"]),
    "recooked-materials-oblique.png": ("recooked", "materials-oblique", []),
    "old-environment-view.png": ("old", "environment-view", ["retained"]),
    "recooked-environment-view.png": ("recooked", "environment-view", ["sourcefree"]),
    "envcontrol-view.png": ("old", "envcontrol-view", ["recooked"]),
}
TEXT = {  # evidence name: source
    "ledger.jsonl": ART / "ledger.jsonl",
    "captures-complete.json": ART / "captures-complete.json",
    "timeline.jsonl": ART / "timeline.jsonl",
    "fixtures.json": ART / "build.json",
    "observations.jsonl": ART / "observations.jsonl",
    "astra-coordination.txt": ROOT / "artifacts/tools/coordination.txt",
    "acp-verification.json": ROOT / "artifacts/tools/acp-verification.json",
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    (SHOTS / "engine").mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    lines = []
    for name, (state, frame, same) in ENGINE.items():
        source = ART / "frames" / state / f"{frame}.png"
        data = source.read_bytes()
        equal = []
        for other in same:
            twin = ART / "frames" / other / f"{frame}.png"
            if twin.read_bytes() != data:
                raise SystemExit(f"{name}: {other} differs from {state}; keep both")
            equal.append(str(twin.relative_to(ROOT)))
        (SHOTS / "engine" / name).write_bytes(data)
        lines.append(f"{sha(data)}  screenshots/engine/{name}  (= {' = '.join([str(source.relative_to(ROOT))] + equal)})")
    for path in sorted((SHOTS / "derived").glob("DERIVED-*.png")):
        lines.append(f"{sha(path.read_bytes())}  screenshots/derived/{path.name}  (derived by tools/analyse.py)")
    (EVIDENCE / "screenshots.sha256").write_text("\n".join(lines) + "\n", encoding="utf-8")
    prefix = str(ROOT) + "/"
    for name, source in TEXT.items():
        text = source.read_text(encoding="utf-8").replace(prefix, "")
        if "/Users/" in text:
            raise SystemExit(f"{name} still contains an absolute user path")
        (EVIDENCE / name).write_text(text, encoding="utf-8")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
