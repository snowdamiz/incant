#!/usr/bin/env python3
"""Round-five figures: candidate frames of the short-span boards and decoded views.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/round5_figures.py BEFORE_RUN AFTER_RUN \
      BEFORE_PROBE AFTER_PROBE GRID_PRE_WIDE GRID_PRE_SHORT OUT_DIR

BEFORE_RUN and BEFORE_PROBE are round-four runs of the 512-texel span build (0494b40);
AFTER_RUN and AFTER_PROBE are runs of the 64-texel span candidate (1f72a99);
GRID_PRE_WIDE (f9aa1da) and GRID_PRE_SHORT (0494b40) render the two-axis views only.
  * engine-*: engine frames copied byte for byte from AFTER_RUN;
  * decoded-*: diagnostics derived from engine pixels, NOT engine images. A pixel is
    redrawn by the parity of the texel it names (light or blue; for the two-axis
    grid, the parity of x + y), red where that texel differs from the exact nearest
    rule, dark where no texel is named. Before/after views put the BEFORE window above
    the AFTER window, 2 px apart; cases are 8 px apart.
OUT_DIR must not exist. Standard library only.
"""

import json
import shutil
import sys
from fractions import Fraction
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyse  # noqa: E402
import pngio  # noqa: E402
from round3_figures import DARK, GAP, TONES, stack  # noqa: E402
from round4_figures import RED, window  # noqa: E402

COPIES = {
    "engine-board-r1-short-spans-48x-128x-along-x-1024x1024.png": "shots/pp-R1-1024x1024-r1.png",
    "engine-board-r2-short-spans-160x-512x-along-x-1024x1024.png": "shots/pp-R2-1024x1024-r1.png",
    "engine-board-s2-short-spans-160x-512x-along-y-1920x1080.png": "shots/pp-S2-hd-1920x1080-r1.png",
}


def pairs(out, name, cases, half, zoom):
    """cases: (meta, before image or None, after image, offset). Returns red counts."""
    strips, listed = [], {}
    for meta, before, after, offset in cases:
        views = [window(img, meta, half, zoom, offset) for img in (before, after) if img is not None]
        strip = views[0]
        for v in views[1:]:
            strip = strip + [[GAP] * len(strip[0])] * 2 + v
        strips.append(strip)
        counts = [sum(c == RED for c in v[0]) // zoom for v in views]
        listed[meta["key"]] = dict(zip(("before", "after") if before is not None else ("after",), counts))
    pngio.write_rgba(out / name, *stack(strips))
    return {"file": name, "red_pixels_on_first_line": listed}


def grid_view(img, meta, offset, zoom):
    """Two-axis parity view of one grid case, red where (x, y) differs from the rule."""
    ox, oy = offset
    axes = {}
    for name, axis_index in (("u", 0), ("v", 1)):
        spec = meta[name]
        segments = [(analyse.snap(Fraction(a)), analyse.snap(Fraction(b)), Fraction(ta), Fraction(tb))
                    for a, b, ta, tb in spec["segments"]]
        axes[name] = (spec["axis"], segments, meta["region"][axis_index], meta["region"][2 + axis_index])
    rows, red = [], 0
    for y in range(img.height):
        row = []
        for x in range(img.width):
            c = {"x": Fraction(2 * (x - ox) + 1, 2), "y": Fraction(2 * (y - oy) + 1, 2)}
            want = []
            for name in ("u", "v"):
                axis, segments, origin, size = axes[name]
                index, _, _ = analyse.span_expected(segments, c[axis], size)
                want.append(None if index is None else origin + index)
            expected = None if None in want else tuple(want)
            got = analyse.grid_texel(img.rgb(x, y))
            if got != expected:
                colour, red = RED, red + 1
            else:
                colour = DARK if got is None else TONES[(got[0] + got[1]) % 2]
            row += [colour] * zoom
        rows += [row] * zoom
    return rows, red


def main():
    if len(sys.argv) != 8:
        raise SystemExit(__doc__)
    before, after, before_probe, after_probe, grid_wide, grid_short, out = (Path(a) for a in sys.argv[1:])
    if out.exists():
        raise SystemExit("OUT_DIR exists; choose a new directory.")
    out.mkdir(parents=True)
    for name, rel in COPIES.items():
        shutil.copyfile(after / rel, out / name)
    scene = json.loads((after / "scene.json").read_text(encoding="utf-8"))
    probe_scene = json.loads((after_probe / "scene.json").read_text(encoding="utf-8"))
    meta = {k: dict(v, key=k) for k, v in scene["meta"].items()}
    probe_meta = {k: dict(v, key=k) for k, v in probe_scene["meta"].items()}

    def img(run, key):
        path = run / f"shots/{key}-r1.png"
        return pngio.read(path) if path.exists() else None

    picked = {}
    # Rows that failed on the 512-texel build in round four, before above after.
    failed4 = [("K:48x:plain:0:cut", "pp-K-1024x1024", (0, 0)), ("K:48x:plain:0:upper", "pp-K-1024x1024", (0, 0)),
               ("L:32x:plain:0:upper", "pp-L-1024x1024", (0, 0)), ("L:48x:plain:0:mid", "pp-L-1024x1024", (0, 0)),
               ("N:48x:plain:upper", "pp-N-hd-1920x1080", (448, 28)), ("N:64x:plain:upper", "pp-N-hd-1920x1080", (448, 28)),
               ("N:48x:plain:mid", "pp-N-hd-1920x1080", (448, 28))]
    cases = [(meta[k], img(before, f), img(after, f), o) for k, f, o in failed4]
    picked["round-four failures"] = pairs(out, "decoded-round4-failing-rows-before-above-after-zoom2.png", cases, 128, 2)
    # Probe rows from the upper half of a 512-texel span, 36x to 128x, before above after.
    keys = [f"{b}:{m}x:plain:0:480" for b in ("O", "P") for m in (36, 48, 80, 128)]
    keys += [f"O:{m}x:flip:1:288" for m in (56, 96)]
    frames = {"O": "pp-O-hd-1920x1080", "P": "pp-P-hd-1920x1080"}
    cases = [(probe_meta[k], img(before_probe, frames[k[0]]), img(after_probe, frames[k[0]]), (448, 28)) for k in keys]
    picked["probe"] = pairs(out, "decoded-probe-hd-36x-128x-before-above-after-zoom2.png", cases, 160, 2)
    # Short spans at 1920 x 1080: passing rows at 128x, then the measured failures at
    # 160x-512x (exact ties, and near ties at 512x), candidate only.
    rows = ["R1:128x:plain:0:60", "S1:128x:plain:0:60", "R2:160x:plain:0:52", "R2:256x:plain:0:36",
            "R2:320x:plain:0:52", "R2:384x:plain:0:44", "R2:512x:plain:0:36", "S2:320x:plain:0:44",
            "S2:384x:flip:0:44", "S2:512x:flip:1:52"]
    cases = [(meta[k], None, img(after, f"pp-{k.split(':')[0]}-hd-1920x1080"), (448, 28)) for k in rows]
    picked["short spans"] = pairs(out, "decoded-short-spans-hd-128x-512x-candidate-zoom2.png", cases, 160, 2)
    # Two-axis nine-slices that fail on some build: pre-wide, 512-texel spans and the
    # candidate side by side (left to right), one case per row.
    names = [("A:none:corner cut:0", "pp"), ("A:rot180:corner cut:0", "pp"), ("A:rot-90:corner cut:0", "pp"),
             ("B:flip-y:corner cut:0", "pp"), ("A:mirror:junction:0", "pp"), ("A:rot+90:junction:0", "pp"),
             ("C:none:junction:0", "pp96"), ("C:mirror:junction:0", "pp96")]
    rows, listed = [], {}
    for case_name, view_name in names:
        key = next(k for k, m in meta.items() if m.get("kind") == "span2d" and k.endswith(":" + case_name))
        size = 128 if view_name == "pp" else 96
        frame = f"{view_name}-{meta[key]['board']}-{size}x{size}"
        offset = (size // 2 - 64, size // 2 - 64)
        blocks, reds = [], []
        for run in (grid_wide, grid_short, after):
            block, red = grid_view(img(run, frame), meta[key], offset, 2)
            blocks.append(block)
            reds.append(red)
        listed[f"{frame} {case_name}"] = dict(zip(("pre-wide", "512-texel spans", "candidate"), reds))
        for y in range(2 * size):
            row = []
            for k, b in enumerate(blocks):
                row += b[y] + ([GAP] * 8 if k < 2 else [])
            rows.append(row)
        rows += [[GAP] * len(rows[-1]) for _ in range(8)]
    width = max(len(r) for r in rows)
    rows = [r + [GAP] * (width - len(r)) for r in rows]
    pngio.write_rgba(out / "decoded-grid-failing-views-pre-wide-512-span-candidate-zoom2.png", width, len(rows),
                     [p for row in rows for p in row])
    picked["grid"] = {"file": "decoded-grid-failing-views-pre-wide-512-span-candidate-zoom2.png",
                      "red_pixels": listed}
    (out / "figure-rows.json").write_text(json.dumps(picked, indent=1), encoding="utf-8")
    print(f"{len(COPIES)} engine frames and {len(picked)} decoded views written to {out}")


if __name__ == "__main__":
    main()
