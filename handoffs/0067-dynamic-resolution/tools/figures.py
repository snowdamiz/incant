#!/usr/bin/env python3
"""Derived review figures for handoff 0067: nearest-neighbour zoomed crops of the
unedited appearance captures, panels stacked with a dark 8 px gap. Figures are
CPU crops of existing PNGs; they are never captures and never replace them.
Standard library only.

  python3 -I -B figures.py <appearance-capture-dir> <new-output-dir>

Panel order within every figure is the order of its `frames` list.
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

GAP = 8
DARK = bytes([40, 40, 40, 255])
# name -> (frames, (x, y, w, h), zoom, layout)
FIGURES = {
    "world-text-100-75-50": (["b1-full-ui-100", "b2-full-ui-75", "b3-full-ui-50"], (584, 92, 300, 84), 2, "column"),
    "crate-floor-100-75-50": (["b1-full-ui-100", "b2-full-ui-75", "b3-full-ui-50"], (612, 252, 168, 150), 2, "row"),
    "floor-edge-left-100-75-50": (["b1-full-ui-100", "b2-full-ui-75", "b3-full-ui-50"], (0, 262, 192, 44), 3, "column"),
    "ui-over-pillar-100-75-50": (["b1-full-ui-100", "b2-full-ui-75", "b3-full-ui-50"], (240, 112, 160, 220), 2, "row"),
    "chrome-corner-disabled-75-50": (["a1-offset-ui-disabled", "a3-offset-ui-75", "a4-offset-ui-50"],
                                     (24, 8, 96, 72), 4, "row"),
    "ortho-crate-text-100-75-50": (["c1-ortho-ui-100", "c2-ortho-ui-75", "c3-ortho-ui-50"],
                                   (584, 92, 300, 290), 1, "row"),
    "pixel-bypass-disabled-50": (["d1-pixel-offset-disabled", "d2-pixel-offset-fixed-50"], (588, 84, 252, 84), 2,
                                 "column"),
}


def crop(rows, x, y, w, h):
    return [bytes(rows[yy][4 * x:4 * (x + w)]) for yy in range(y, y + h)]


def zoom(rows, factor):
    out = []
    for row in rows:
        line = b"".join(row[4 * x:4 * x + 4] * factor for x in range(len(row) // 4))
        out.extend([line] * factor)
    return out


def stack(panels, layout):
    w, h = len(panels[0][0]) // 4, len(panels[0])
    if layout == "row":
        width = w * len(panels) + GAP * (len(panels) - 1)
        gap = DARK * GAP
        return width, h, [gap.join(p[y] for p in panels) for y in range(h)]
    height = h * len(panels) + GAP * (len(panels) - 1)
    rows = []
    for i, p in enumerate(panels):
        if i:
            rows.extend([DARK * w] * GAP)
        rows.extend(p)
    return w, height, rows


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    source, output = Path(sys.argv[1]), Path(sys.argv[2])
    output.mkdir(parents=True, exist_ok=False)
    record = {}
    for name, (frames, (x, y, w, h), factor, layout) in FIGURES.items():
        panels = []
        inputs = {}
        for frame in frames:
            path = source / f"{frame}.png"
            inputs[frame] = hashlib.sha256(path.read_bytes()).hexdigest()
            _, _, rows = pngio.read(path)
            panels.append(zoom(crop(rows, x, y, w, h), factor))
        width, height, rows = stack(panels, layout)
        data = pngio.write(output / f"{name}.png", width, height, rows)
        record[name] = {"frames": frames, "crop": [x, y, w, h], "zoom": factor, "layout": layout,
                        "inputs_png_sha256": inputs, "size": [width, height],
                        "sha256": hashlib.sha256(data).hexdigest()}
    (output / "figures.json").write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: v["size"] for k, v in record.items()}, indent=1))


if __name__ == "__main__":
    main()
