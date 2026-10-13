#!/usr/bin/env python3
"""Derived inspection figures for handoff 0062 (CPU-only crops and nearest zooms).

These are not captures: every figure is cut from an existing candidate capture
(byte-identical to its baseline) and enlarged with pixel replication only.

  python3 -I -B figures.py <evidence-dir> <new-output-dir>
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from pngtool import Raster, column_of, row_of  # noqa: E402

FIGURES = {
    # name: [(case, png, x, y, w, h, zoom), ...] laid out in one row
    "ties-A2-A6-A7": [("A-pp", 0, 66, 30, 60, 60, 5), ("A-pp", 0, 366, 30, 60, 60, 5),
                      ("A-pp", 0, 426, 30, 60, 60, 5)],
    "atlas-edges-ring": [("A-pp", 0, 12, 132, 96, 96, 4), ("A-pp", 0, 120, 132, 96, 96, 4)],
    "plain-sheet-neighbours": [("A-pp", 0, 225, 129, 102, 102, 3), ("A-pp", 0, 339, 129, 102, 102, 3),
                               ("A-pp", 0, 447, 153, 54, 54, 3)],
    "premultiplied-alpha": [("A-pp", 0, 534, 150, 108, 60, 4), ("A-pp", 0, 654, 150, 108, 60, 4)],
    "tint-and-half-alpha": [("A-pp", 0, 768, 150, 60, 60, 4), ("A-pp", 0, 840, 132, 96, 96, 3)],
    "reflection-rotation": [("A-pp", 0, 0, 240, 480, 110, 2), ("A-pp", 0, 480, 240, 480, 110, 2)],
    "nine-slice-order": [("A-pp", 0, 6, 370, 300, 96, 3)],
    "order-ties-depth": [("A-pp", 0, 310, 350, 180, 120, 3), ("A-pp", 0, 530, 350, 236, 120, 3)],
    "sampling-row": [("A-pp", 0, 0, 0, 480, 120, 2), ("A-pp", 0, 480, 0, 480, 120, 2)],
    "ortho-fractional": [("A-ortho", 0, 0, 0, 480, 270, 2), ("A-ortho", 0, 0, 180, 480, 200, 2)],
    "perspective-depth": [("A-persp", 0, 540, 330, 240, 120, 4)],
    "perspective-rows": [("A-persp", 0, 150, 40, 400, 200, 2), ("A-persp", 0, 540, 40, 400, 200, 2)],
    "wide-ends": [("W-pp", 0, 270, 70, 200, 160, 3), ("W-pp", 0, 490, 400, 180, 70, 3)],
    "wide-vertical-ends": [("W-pp", 0, 600, 200, 90, 60, 4), ("W-pp", 0, 280, 280, 70, 60, 4)],
    "wide-tie": [("W-pp", 0, 640, 310, 190, 70, 4)],
    "tilemap-vs-cells": [("B-pp", 0, 24, 108, 204, 108, 3), ("B-pp", 0, 24, 258, 204, 108, 3)],
    "tilemap-rotated-mirrored": [("B-pp", 0, 270, 0, 156, 300, 2), ("B-ortho", 0, 460, 0, 330, 300, 1)],
    "flipbooks-tick0": [("B-pp", 0, 490, 40, 470, 280, 2)],
    "flipbooks-tick48": [("B-pp", 48, 490, 40, 470, 280, 2)],
    "flipbooks-rotated": [("B-pp", 0, 480, 340, 480, 200, 2), ("B-pp", 48, 480, 340, 480, 200, 2)],
}


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True)
    cache = {}
    manifest = {}
    for name, parts in FIGURES.items():
        tiles = []
        for case, tick, x, y, w, h, zoom in parts:
            key = (case, tick)
            if key not in cache:
                cache[key] = Raster.load(evidence / "captures/candidate" / case / f"tick-{tick:05d}.png")
            frame = cache[key]
            w, h = min(w, frame.width - x), min(h, frame.height - y)
            tiles.append(frame.crop(x, y, w, h).zoom(zoom))
        figure = row_of(tiles, gap=6, color=(255, 255, 255, 255)) if len(tiles) > 1 else tiles[0]
        if figure.width > 2400:
            figure = column_of(tiles, gap=6, color=(255, 255, 255, 255))
        figure.save(out / f"{name}.png")
        manifest[name] = {"parts": [list(p) for p in parts], "size": [figure.width, figure.height]}
    (out / "figures.json").write_text(json.dumps({"derived_from": "candidate captures (baseline identical)",
                                                  "figures": manifest}, indent=1) + "\n")
    print(json.dumps({k: v["size"] for k, v in manifest.items()}))


if __name__ == "__main__":
    main()
