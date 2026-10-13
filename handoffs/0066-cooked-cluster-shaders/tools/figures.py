#!/usr/bin/env python3
"""Derived review figures for handoff 0066: tile-aligned nearest-neighbour crops of captures.

Adapted from handoff 0064's committed tools/figures.py (read-only reuse).

  python3 -I -B figures.py <evidence-dir> <new-output-dir>

Figures are CPU crops/zooms of existing candidate PNG captures (each byte-identical
to its baseline file when the analysis says so); they are never captures. Every
crop starts on the 64-pixel cluster tile lattice. A 10-pixel dark margin is added
around the zoomed crop and white ticks in that margin mark each tile edge; the
crop's own pixels are not altered. Standard library only.
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

TILE, MARGIN = 64, 10
# name -> (capture, x, y, w, h, zoom); rectangles in capture pixels, x/y on the tile lattice.
CROPS = {
    "n33-corner-light": ("C-persp-n33.png", 384, 192, 128, 128, 4),
    "n33-near-row": ("C-persp-n33.png", 384, 384, 192, 128, 3),
    "n33-horizontal-edge": ("C-persp-n33.png", 512, 256, 256, 128, 3),
    "n4096-t000-centre": ("C-persp-n4096-t000.png", 320, 320, 256, 128, 3),
    "n4096-t030-centre": ("C-persp-n4096-t030.png", 320, 320, 256, 128, 3),
    "ortho-t030-centre": ("C-ortho-n4096-t030.png", 384, 192, 192, 128, 3),
}


def crop(rows, x, y, w, h):
    return [bytes(rows[yy][4 * x:4 * (x + w)]) for yy in range(y, y + h)]


def zoom(rows, factor):
    out = []
    for row in rows:
        line = b"".join(row[4 * x:4 * x + 4] * factor for x in range(len(row) // 4))
        out.extend([line] * factor)
    return out


def framed(rows, x0, y0, w, h, factor):
    """Zoomed rows inside a dark margin with white tile-edge ticks in the margin only."""
    width = w * factor + 2 * MARGIN
    dark, white = bytes([40, 40, 40, 255]), bytes([255, 255, 255, 255])
    xs = [(x - x0) * factor + MARGIN for x in range(x0, x0 + w + 1) if x % TILE == 0]
    ys = [(y - y0) * factor + MARGIN for y in range(y0, y0 + h + 1) if y % TILE == 0]

    def margin_row(ticks):
        row = bytearray(dark * width)
        for t in ticks:
            for c in (t - 1, t):
                if 0 <= c < width:
                    row[4 * c:4 * c + 4] = white
        return bytes(row)

    out = [margin_row(xs if MARGIN - 6 <= i else []) for i in range(MARGIN)]
    for i, line in enumerate(rows):
        y = i + MARGIN
        side = white if any(y in (t - 1, t) for t in ys) else dark
        out.append(dark * (MARGIN - 6) + side * 6 + line + side * 6 + dark * (MARGIN - 6))
    out += [margin_row(xs if i < 6 else []) for i in range(MARGIN)]
    return width, len(out), out, xs, ys


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    record = {"kind": "derived figures (CPU crops/zooms of candidate captures; not renders)",
              "margin": MARGIN, "tile": TILE, "figures": {}}
    for name, (capture, x, y, w, h, factor) in CROPS.items():
        if x % TILE or y % TILE:
            raise SystemExit(f"{name} is not tile aligned")
        path = evidence / "captures/candidate/main" / capture
        width, height, rows = pngio.read(path)
        w, h = min(w, width - x), min(h, height - y)
        fw, fh, part, xs, ys = framed(zoom(crop(rows, x, y, w, h), factor), x, y, w, h, factor)
        data = pngio.write(out / f"{name}.png", fw, fh, part)
        record["figures"][name] = {"from": capture, "from_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                                   "rect": [x, y, w, h], "zoom": factor,
                                   "tile_edges_x": [x + (t - MARGIN) // factor for t in xs],
                                   "tile_edges_y": [y + (t - MARGIN) // factor for t in ys],
                                   "sha256": hashlib.sha256(data).hexdigest()}
    (out / "figures.json").write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: (v["rect"], v["tile_edges_x"], v["tile_edges_y"]) for k, v in record["figures"].items()}))


if __name__ == "__main__":
    main()
