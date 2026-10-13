#!/usr/bin/env python3
"""Derived inspection figures for handoff 0063 (CPU-only crops and nearest zooms).

These are not captures: every figure is cut from an existing candidate capture
(byte-identical to its baseline) and enlarged by pixel replication only.

  python3 -I -B figures.py <evidence-dir> <new-output-dir>
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from pngio import Raster, column_of, row_of  # noqa: E402

# name: (layout, [(frame, x, y, w, h, zoom), ...]); frames are <case>/<capture>.
FIGURES = {
    "w-mirror-controls": ("column", [("W-ortho/t1", 13, 178, 304, 38, 2), ("W-ortho/t1", 333, 178, 304, 38, 2),
                                     ("W-ortho/t1", 653, 178, 304, 38, 2)]),
    "w-flip-parent-control": ("column", [("W-ortho/t1", 333, 296, 304, 36, 2),
                                         ("W-ortho/t1", 653, 263, 304, 36, 2)]),
    "w-scripts": ("column", [("W-ortho/t1", 13, 8, 304, 38, 2), ("W-ortho/t1", 430, 8, 215, 40, 2)]),
    "w-coverage-over-bright": ("column", [("W-ortho/t1", 652, 8, 260, 40, 3), ("W-ortho/t1", 652, 93, 260, 40, 3)]),
    "w-magnified-rotated": ("column", [("W-ortho/t1", 13, 263, 230, 60, 3), ("W-ortho/t1", 340, 65, 180, 110, 2)]),
    "w-minification-ladder": ("column", [("W-ortho/t1", 12, 346, 240, 62, 4)]),
    "w-overlap": ("column", [("W-ortho/t1", 333, 440, 215, 40, 3)]),
    "w-persp-yaw-depth": ("column", [("W-persp/t1", 330, 355, 150, 95, 3), ("W-persp/t1", 640, 335, 300, 100, 2)]),
    "w-persp-motion": ("column", [("W-persp/t1", 650, 440, 260, 42, 2), ("W-persp/t61", 650, 440, 260, 42, 2)]),
    "u-rounded-corners-1x": ("row", [("U-1x/idle", 28, 134, 102, 46, 6)]),
    "u-disabled-and-arabic-buttons": ("row", [("U-1x/idle", 232, 134, 206, 46, 3)]),
    "u-clip-strip": ("row", [("U-1x/idle", 186, 186, 102, 46, 4)]),
    "u-overflow-clips": ("column", [("U-1x/idle", 28, 330, 368, 44, 2), ("U-1x/idle", 28, 400, 368, 44, 2)]),
    "u-selection-focus": ("column", [("U-1x/select", 28, 260, 368, 44, 2)]),
    "u-focus-2x": ("column", [("U-2x/focus", 56, 268, 200, 92, 2)]),
    "u-preedit-15x": ("column", [("U-15x-name/preedit", 44, 392, 260, 62, 3)]),
    "u-label-over-world-2x": ("column", [("U-2x/focus", 600, 140, 240, 110, 2)]),
    "u-srgb-vs-unorm": ("row", [("U-1x/idle", 28, 134, 102, 46, 4), ("U-1x-unorm/idle", 28, 134, 102, 46, 4)]),
    "u-srgb-vs-unorm-text": ("column", [("U-1x/idle", 32, 78, 250, 24, 3), ("U-1x-unorm/idle", 32, 78, 250, 24, 3)]),
}


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True)
    cache, manifest = {}, {}
    for name, (layout, parts) in FIGURES.items():
        tiles = []
        for frame, x, y, w, h, zoom in parts:
            if frame not in cache:
                case, capture = frame.split("/")
                cache[frame] = Raster.load(evidence / "captures/candidate" / case / f"{capture}.png")
            tiles.append(cache[frame].crop(x, y, w, h).zoom(zoom))
        figure = tiles[0] if len(tiles) == 1 else (row_of(tiles) if layout == "row" else column_of(tiles))
        if figure.width > 960 or figure.height > 540:
            raise SystemExit(f"{name} is {figure.width}x{figure.height}; figures stay within 960x540")
        figure.save(out / f"{name}.png")
        manifest[name] = {"parts": [list(p) for p in parts], "layout": layout, "size": [figure.width, figure.height]}
    (out / "figures.json").write_text(json.dumps({"derived_from": "candidate captures (each byte-identical to its "
                                                                  "baseline); crops with nearest zoom only",
                                                  "figures": manifest}, indent=1) + "\n")
    print(json.dumps({k: v["size"] for k, v in manifest.items()}))


if __name__ == "__main__":
    main()
