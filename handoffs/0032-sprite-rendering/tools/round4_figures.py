#!/usr/bin/env python3
"""Round-four figures: candidate frames of the span boards and before/after decoded
views of the windows that decide the wide-region correction.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/round4_figures.py BEFORE_RUN AFTER_RUN OUT_DIR

BEFORE_RUN is a pre-wide run and AFTER_RUN an active (0494b40) run of the same scene.
  * engine-*: engine frames copied byte for byte from AFTER_RUN;
  * decoded-*: diagnostics derived from engine pixels, NOT engine images. Each pixel is
    redrawn by the parity of the source texel it names (light or blue); a pixel whose
    texel differs from the exact nearest rule is red, and an unnamed pixel dark. For
    each case the BEFORE window is drawn above the AFTER window, 2 px apart; cases are
    8 px apart. Windows are centred on the row's tie at 512.5 and enlarged by nearest
    neighbour.
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

RED = (220, 48, 48, 255)
COPIES = {
    "engine-board-k-spans-along-x-active-1024x1024.png": "shots/pp-K-1024x1024-r1.png",
    "engine-board-l-spans-along-y-active-1024x1024.png": "shots/pp-L-1024x1024-r1.png",
    "engine-board-m-span-variants-active-1024x1024.png": "shots/pp-M-1024x1024-r1.png",
    "engine-board-n-isotropic-active-1920x1080.png": "shots/pp-N-hd-1920x1080-r1.png",
}


def window(img, meta, half, zoom, offset=(0, 0)):
    """Decoded rows of one span row's band over [512 - half, 512 + half), along the row."""
    ox, oy = offset
    thick = meta["magnification"] if meta.get("family") == "isotropic" else 3
    first = meta["line"] - thick // 2
    axis = 0 if meta["texture_axis"] == "x" else 1
    origin, size = meta["region"][axis], meta["region"][2 + axis]
    segments = [(analyse.snap(Fraction(a)), analyse.snap(Fraction(b)), Fraction(ta), Fraction(tb))
                for a, b, ta, tb in meta["segments"]]
    rows = []
    for a in range(min(thick, 8)):  # isotropic bands are uniform across; 8 rows suffice
        row = []
        for s in range(512 - half, 512 + half):
            index, _, _ = analyse.span_expected(segments, Fraction(2 * s + 1, 2), size)
            want = None if index is None else origin + index
            x, y = (s + ox, first + a + oy) if meta["along"] == "x" else (first + a + ox, s + oy)
            got = analyse.strip_index(img.rgb(x, y))
            colour = DARK if got is None else RED if got != want else TONES[got % 2]
            row += [colour] * zoom
        rows += [row] * zoom
    return rows


def figure(out, name, cases, shots, half, zoom):
    """Writes the view; returns each case's red (wrong-texel) pixel count per decoded
    line inside its window, before and after."""
    strips, listed = [], {}
    for meta, frame, offset in cases:
        pair = [window(shots[run][frame], meta, half, zoom, offset) for run in ("before", "after")]
        strips.append(pair[0] + [[GAP] * len(pair[0][0])] * 2 + pair[1])
        listed[meta["key"]] = {run: sum(c == RED for c in rows[0]) // zoom for run, rows in zip(("before", "after"), pair)}
    pngio.write_rgba(out / name, *stack(strips))
    return {"file": name, "red_pixels_on_first_line": listed}


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    before, after, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    if out.exists():
        raise SystemExit("OUT_DIR exists; choose a new directory.")
    out.mkdir(parents=True)
    for name, rel in COPIES.items():
        shutil.copyfile(after / rel, out / name)
    frames = ("pp-J-1024x640", "pp-K-1024x1024", "pp-L-1024x1024", "pp-M-1024x1024", "pp-N-hd-1920x1080")
    shots = {label: {k: pngio.read(run / f"shots/{k}-r1.png") for k in frames}
             for label, run in (("before", before), ("after", after))}
    scene = json.loads((after / "scene.json").read_text(encoding="utf-8"))
    meta = {k: dict(v, key=k) for k, v in scene["meta"].items()}
    picked = {}
    # Boards K and L: the upper half of a span (texels 448 on), exact ties, 32x to 64x.
    for b, frame in (("K", "pp-K-1024x1024"), ("L", "pp-L-1024x1024")):
        cases = [(meta[f"{b}:{m}x:{o}:0:upper"], frame, (0, 0)) for m in (32, 48, 64) for o in ("plain", "flip")]
        picked[b] = figure(out, f"decoded-board-{b.lower()}-upper-span-ties-32x-48x-64x-before-above-after-zoom2.png",
                           cases, shots, 128, 2)
    # Board K: every window at 8x and 16x, where both directions must pass.
    cases = [(meta[f"K:{m}x:{o}:0:{w}"], "pp-K-1024x1024", (0, 0)) for m in (8, 16) for o in ("plain", "flip")
             for w in ("cut", "upper")]
    picked["K low"] = figure(out, "decoded-board-k-8x-16x-ties-before-above-after-zoom2.png", cases, shots, 128, 2)
    # Board M: wide nine-slice windows at 32x borders, and the variants' short end spans at 48x.
    cases = [(meta[k], "pp-M-1024x1024", (0, 0)) for k in sorted(meta) if k.startswith("M:nine:32x12:")]
    cases += [(meta[f"M:{r}:48x:{o}:0:end"], "pp-M-1024x1024", (0, 0)) for r in ("r4097", "r8191", "r1000")
              for o in ("plain", "flip")]
    picked["M"] = figure(out, "decoded-board-m-nine-slice-and-short-spans-before-above-after-zoom2.png", cases,
                         shots, 128, 2)
    # Board N at 1920 x 1080: isotropic 48x and 64x rows, every window, both directions.
    cases = [(meta[f"N:{m}x:{o}:{w}"], "pp-N-hd-1920x1080", (448, 28)) for m in (48, 64)
             for o in ("plain", "flip") for w in ("cut", "mid", "upper")]
    picked["N hd"] = figure(out, "decoded-board-n-hd-48x-64x-before-above-after-zoom2.png", cases, shots, 160, 2)
    (out / "figure-rows.json").write_text(json.dumps(picked, indent=1), encoding="utf-8")
    print(f"{len(COPIES)} engine frames and {len(picked)} decoded views written to {out}")


if __name__ == "__main__":
    main()
