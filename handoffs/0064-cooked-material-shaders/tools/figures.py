#!/usr/bin/env python3
"""Derived review figures for handoff 0064: nearest-neighbour crops of captures.

  python3 -I -B figures.py <evidence-dir> <new-output-dir>

Figures are CPU crops/zooms of existing candidate PNG captures (each byte-identical
to its baseline file when the analysis says so). They are never captures and are
labelled as derived in figures.json. Standard library only.
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

# name -> (capture, x, y, w, h, zoom[, plan]); rectangles in capture pixels; plan defaults to main.
CROPS = {
    "M-close-normal-occlusion": ("M-close.png", 0, 120, 480, 300, 2),
    "M-close-emissive-mask": ("M-close.png", 440, 100, 520, 330, 2),
    "M-close-spheres": ("M-close.png", 0, 300, 960, 240, 1),
    "M-wide-culling-row": ("M-wide.png", 520, 190, 440, 150, 2),
    "M-wide-spheres": ("M-wide.png", 150, 250, 660, 150, 2),
    "M-wide-minification": ("M-wide.png", 0, 140, 960, 120, 1),
    "L-persp-t090-centre": ("L-persp-t090.png", 280, 160, 400, 225, 2),
    "R-v1-panel": ("R-v1.png", 60, 30, 300, 210, 2),
    "R-v2-panel": ("R-v2.png", 60, 30, 300, 210, 2),
    "S-shadows-near": ("S-shadows.png", 230, 230, 500, 270, 2, "shadow"),
    "S-shadows-far": ("S-shadows.png", 330, 170, 320, 120, 3, "shadow"),
}


def crop(rows, x, y, w, h):
    return [bytes(rows[yy][4 * x:4 * (x + w)]) for yy in range(y, y + h)]


def zoom(rows, factor):
    out = []
    for row in rows:
        line = b"".join(row[4 * x:4 * x + 4] * factor for x in range(len(row) // 4))
        out.extend([line] * factor)
    return out


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    record = {"kind": "derived figures (CPU crops/zooms of candidate captures; not renders)", "figures": {}}
    for name, (capture, x, y, w, h, factor, *plan) in CROPS.items():
        path = evidence / "captures/candidate" / (plan[0] if plan else "main") / capture
        width, height, rows = pngio.read(path)
        w, h = min(w, width - x), min(h, height - y)
        part = zoom(crop(rows, x, y, w, h), factor)
        data = pngio.write(out / f"{name}.png", w * factor, h * factor, part)
        record["figures"][name] = {"from": capture, "from_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                                   "rect": [x, y, w, h], "zoom": factor,
                                   "sha256": hashlib.sha256(data).hexdigest()}
    (out / "figures.json").write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: v["rect"] for k, v in record["figures"].items()}))


if __name__ == "__main__":
    main()
