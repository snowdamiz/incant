#!/usr/bin/env python3
"""Follow-up figures: corrected-build frames, before/after crops and decoded views.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/followup_figures.py BEFORE_RUN AFTER_RUN OUT_DIR

BEFORE_RUN is a pre-ties run and AFTER_RUN a corrected run of the same scene. Three
kinds of PNG are written, distinguished by name:
  * engine-*: engine frames copied byte for byte;
  * *-before-after-zoomN: nearest-neighbour enlargements of engine pixels, BEFORE on
    the left and AFTER on the right of an 8 px gap;
  * decoded-*: diagnostics derived from engine pixels, NOT engine images. Each ruler
    pixel is redrawn as a two-tone checker by the source texel it names, so a
    repeated or dropped texel shows as a broken checker. Other pixels are dark.
OUT_DIR must not exist. Standard library only.
"""

import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402
import sprite_lookdev  # noqa: E402

GAP = (20, 21, 25, 255)
CHECKER = ((236, 236, 236, 255), (64, 120, 210, 255))
DARK = (24, 22, 40, 255)

COPIES = {
    "engine-board-f-orientations-after-960x540.png": "shots/pp-F-960x540-r1.png",
    "engine-board-g-offset-sweep-after-1024x640.png": "shots/pp-G-1024x640-r1.png",
}

# name: (frame, crop x, y, w, h, zoom); BEFORE and AFTER side by side.
PAIRS = {
    "ruler-station-board-a-before-after-zoom10.png": ("shots/pp-A-320x180-r1.png", 220, 52, 76, 28, 10),
    "ruler-station-80px-sheet-before-after-zoom10.png": ("reimport/7-sheet-tall-pp-A-320x180.png", 220, 52, 76, 28, 10),
    "board-f-ties-1x-before-after-zoom8.png": ("shots/pp-F-320x180-r1.png", 8, 18, 172, 13, 8),
    "board-f-ties-3x-before-after-zoom3.png": ("shots/pp-F-320x180-r1.png", 6, 86, 300, 28, 3),
    "nine-slice-stretch-and-density-before-after-zoom4.png": ("shots/pp-B-320x180-r1.png", 92, 10, 112, 162, 4),
    "sky-strip-nearest-before-after-zoom6.png": ("shots/pp-D-320x180-r1.png", 164, 134, 26, 44, 6),
}


def zoom(img, x0, y0, w, h, n):
    return [[img.px(x0 + x // n, y0 + y // n) for x in range(w * n)] for y in range(h * n)]


def side_by_side(blocks):
    height = max(len(b) for b in blocks)
    width = sum(len(b[0]) for b in blocks) + 8 * (len(blocks) - 1)
    canvas = [[GAP] * width for _ in range(height)]
    ox = 0
    for b in blocks:
        for y, row in enumerate(b):
            canvas[y][ox:ox + len(row)] = row
        ox += len(b[0]) + 8
    return width, height, [p for row in canvas for p in row]


def decoded(img, x0, y0, w, h, n):
    """Checker by named texel parity, enlarged n times."""
    rows = []
    for y in range(h * n):
        row = []
        for x in range(w * n):
            r, g, b = img.rgb(x0 + x // n, y0 + y // n)
            if b == 150 and (r - 60) % 25 == 0 and (g - 60) % 25 == 0 and 60 <= r <= 210 and 60 <= g <= 210:
                i, j = (r - 60) // 25, (g - 60) // 25
                row.append(CHECKER[(i + j) % 2])
            else:
                row.append(DARK)
        rows.append(row)
    return rows


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    before, after, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    if out.exists():
        raise SystemExit("OUT_DIR exists; choose a new directory.")
    out.mkdir(parents=True)
    for name, rel in COPIES.items():
        shutil.copyfile(after / rel, out / name)
    for name, (rel, x, y, w, h, n) in PAIRS.items():
        blocks = [zoom(pngio.read(run / rel), x, y, w, h, n) for run in (before, after)]
        pngio.write_rgba(out / name, *side_by_side(blocks))
    # Decoded board F: tie rows at 1x and 3x, BEFORE above AFTER.
    for label, (rel, x, y, w, h, n) in {
            "decoded-board-f-ties-1x-before-after-zoom8.png": ("shots/pp-F-320x180-r1.png", 8, 18, 172, 13, 8),
            "decoded-board-f-ties-3x-before-after-zoom3.png": ("shots/pp-F-320x180-r1.png", 6, 86, 300, 28, 3)}.items():
        blocks = [decoded(pngio.read(run / rel), x, y, w, h, n) for run in (before, after)]
        pngio.write_rgba(out / label, *side_by_side(blocks))
    # Decoded board G: the 4x, 5x, 6x and 8x rows, offset columns 0 to 6 (0 to 3/256 px).
    cells = sprite_lookdev.sweep_layout()
    rows = sorted({cy for m, _, _, _, cy in cells if m in (4, 5, 6, 8)})
    top, bottom = rows[0] - 2, max(cy + 7 * m + 2 for m, _, _, _, cy in cells if m == 8)
    for run_name, run in (("before", before), ("after", after)):
        img = pngio.read(run / "shots/pp-G-1024x640-r1.png")
        block = decoded(img, 40, top, 62 * 7, bottom - top, 2)
        w, h = len(block[0]), len(block)
        pngio.write_rgba(out / f"decoded-board-g-4x-to-8x-{run_name}-zoom2.png", w, h, [p for r in block for p in r])
        crop = zoom(img, 40, top, 62 * 7, bottom - top, 2)
        pngio.write_rgba(out / f"board-g-4x-to-8x-{run_name}-zoom2.png", w, h, [p for r in crop for p in r])
    # Targeted decoded cells: offsets 0, 0.625, 1 and 3 (in 1/256 px) for 8x unflipped
    # and 6x flipped. BEFORE is the upper strip, AFTER the lower one, 8 px apart.
    for m, flipped in ((8, False), (6, True)):
        picked = [cell for cell in cells if cell[0] == m and cell[1] == flipped and cell[2] in (0, 3, 4, 6)]
        strips = []
        for run in (before, after):
            img = pngio.read(run / "shots/pp-G-1024x640-r1.png")
            blocks = [decoded(img, cx - 1, cy - 1, 7 * m + 3, 7 * m + 3, 4) for _, _, _, cx, cy in picked]
            strips.append(side_by_side(blocks))
        width = max(w for w, _, _ in strips)
        pixels = []
        for k, (w, h, px) in enumerate(strips):
            for y in range(h):
                pixels += px[y * w:(y + 1) * w] + [GAP] * (width - w)
            if k == 0:
                pixels += [GAP] * (width * 8)
        height = len(pixels) // width
        tag = f"{m}x-{'flipped' if flipped else 'unflipped'}"
        pngio.write_rgba(out / f"decoded-board-g-{tag}-offsets-0-0.625-1-3-before-above-after-zoom4.png",
                         width, height, pixels)
    print(f"{len(COPIES)} engine frames, {len(PAIRS)} before/after crops, decoded views written to {out}")


if __name__ == "__main__":
    main()
