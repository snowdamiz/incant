#!/usr/bin/env python3
"""Copy the concise engine frame set and make labelled zoom crops. Standard library only.

Engine PNGs are copied byte for byte. Zoom crops are nearest-neighbour enlargements
of engine pixels (each source pixel becomes an N x N block, nothing else changes)
and carry "-zoomN" in their names. Strips place two or more crops side by side on
the renderer's own backdrop colour with an 8 px gap.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/crops.py RUN_DIR OUT_DIR
OUT_DIR must not exist.
"""

import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

GAP = (20, 21, 25, 255)

COPIES = {
    "board-a-regions-pivots-flips-960x540.png": "shots/pp-A-960x540-r1.png",
    "board-b-nine-slice-960x540.png": "shots/pp-B-960x540-r1.png",
    "board-c-order-alpha-depth-960x540.png": "shots/pp-C-960x540-r1.png",
    "board-d-sampling-seams-tone-960x540.png": "shots/pp-D-960x540-r1.png",
    "board-e-showcase-pixel-perfect-960x540.png": "shots/pp-E-960x540-r1.png",
    "board-e-showcase-ortho-3.75x-960x540.png": "shots/orthof-E-960x540-r1.png",
    "board-b-nine-slice-ortho-roll7-960x540.png": "shots/roll7-B-960x540-r1.png",
    "board-d-ortho-3.75x-960x540.png": "shots/orthof-D-960x540-r1.png",
    "reimport-teal-sheet-board-e-320x180.png": "reimport/1-sheet-teal-pp-E-320x180.png",
    "reimport-orange-badge-board-e-320x180.png": "reimport/2-badge-orange-pp-E-320x180.png",
}

# name: [(source, x, y, w, h)], zoom
CROPS = {
    "ruler-ties-64px-sheet-vs-80px-sheet-zoom12.png": (
        [("shots/pp-A-320x180-r1.png", 220, 52, 76, 28), ("reimport/7-sheet-tall-pp-A-320x180.png", 220, 52, 76, 28)],
        12),
    "nine-slice-corners-and-flips-zoom4.png": ([("shots/pp-B-320x180-r1.png", 4, 70, 300, 48)], 4),
    "nine-slice-small-targets-zoom8.png": ([("shots/pp-B-320x180-r1.png", 208, 10, 108, 38)], 8),
    "linear-orb-fringe-and-sampling-zoom4.png": ([("shots/pp-D-320x180-r1.png", 4, 10, 312, 72)], 4),
    "tone-swatches-8bit-16bit-tint-zoom6.png": ([("shots/pp-D-320x180-r1.png", 4, 134, 164, 34)], 6),
    "alpha-venn-and-strips-zoom4.png": ([("shots/pp-C-320x180-r1.png", 4, 70, 230, 106)], 4),
    "gray-ramp-0-255-zoom6.png": ([("shots/pp-C-320x180-r1.png", 228, 138, 47, 42)], 6),
    "depth-occluder-zoom6.png": ([("shots/pp-C-320x180-r1.png", 230, 76, 82, 46)], 6),
    "follow-ne-ticks-3-4-5-zoom6.png": (
        [(f"live-ne-r1/frame-{t:06d}.png", 84, 96, 48, 40) for t in (3, 4, 5)], 6),
    "flipbook-e-frames-0-4-8-12-zoom6.png": (
        [(f"live-e-r1/frame-{t:06d}.png", 88, 104, 32, 32) for t in (0, 4, 8, 12)], 6),
    "seams-rolled-tiles-ortho-crop.png": ([("shots/roll7-D-960x540-r1.png", 0, 250, 960, 290)], 1),
}


def zoom(img, x0, y0, w, h, n):
    rows = []
    for y in range(h * n):
        row = []
        for x in range(w * n):
            row.append(img.px(x0 + x // n, y0 + y // n))
        rows.append(row)
    return rows


def main():
    run, out = Path(sys.argv[1]), Path(sys.argv[2])
    if out.exists():
        raise SystemExit("OUT_DIR exists; choose a new directory.")
    out.mkdir(parents=True)
    for name, rel in COPIES.items():
        shutil.copyfile(run / rel, out / name)
    for name, (parts, n) in CROPS.items():
        blocks = [zoom(pngio.read(run / src), x, y, w, h, n) for src, x, y, w, h in parts]
        height = max(len(b) for b in blocks)
        width = sum(len(b[0]) for b in blocks) + 8 * (len(blocks) - 1)
        canvas = [[GAP] * width for _ in range(height)]
        ox = 0
        for b in blocks:
            for y, row in enumerate(b):
                canvas[y][ox:ox + len(row)] = row
            ox += len(b[0]) + 8
        pngio.write_rgba(out / name, width, height, [p for row in canvas for p in row])
    print(f"{len(COPIES)} engine frames copied, {len(CROPS)} zoom crops written")


if __name__ == "__main__":
    main()
