#!/usr/bin/env python3
"""Round-three figures: active-build frames, before/after decoded views of the
magnification sweep, high-magnification strips and wide-region exact ties.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/round3_figures.py BEFORE_RUN AFTER_RUN OUT_DIR

BEFORE_RUN is a pre-magnification run and AFTER_RUN an active run of the same scene.
  * engine-*: engine frames copied byte for byte from AFTER_RUN;
  * decoded-*: diagnostics derived from engine pixels, NOT engine images. Each texel
    is redrawn by the parity of the source texel it names, so a texel one pixel too
    wide or narrow shows as a wider or narrower stripe. BEFORE is the upper strip,
    AFTER the lower one, 8 px apart, enlarged by nearest neighbour.
OUT_DIR must not exist. Standard library only.
"""

import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402
import sprite_lookdev  # noqa: E402

GAP = (20, 21, 25, 255)
TONES = ((236, 236, 236, 255), (64, 120, 210, 255))
DARK = (24, 22, 40, 255)

COPIES = {
    "engine-board-g-offset-sweep-active-1024x640.png": "shots/pp-G-1024x640-r1.png",
    "engine-board-h-falsification-active-1280x640.png": "shots/pp-H-1280x640-r1.png",
    "engine-board-i-wide-strip-active-1024x640.png": "shots/pp-I-1024x640-r1.png",
    "engine-board-j-region-width-map-active-1024x640.png": "shots/pp-J-1024x640-r1.png",
}


def texel_of(rgb):
    """Source texel named by a pixel of either ruler or either strip, else None."""
    r, g, b = rgb
    if b == 150 and (r - 60) % 25 == 0 and (g - 60) % 25 == 0 and 60 <= r <= 210 and 60 <= g <= 210:
        return (r - 60) // 25 + (g - 60) // 25
    if b in (90, 92, 120) and (r - 60) % 2 == 0 and (g - 60) % 2 == 0 and 60 <= r <= 186 and 60 <= g <= 186:
        if b == 120:
            return (r - 60) // 2 + (g - 60) // 2
        return (r - 60) // 2 + 64 * ((g - 60) // 2) + 4096 * ((b - 90) // 2)
    return None


def decoded(img, x0, y0, w, h, n):
    rows = []
    for y in range(h * n):
        row = []
        for x in range(w * n):
            t = texel_of(img.rgb(x0 + x // n, y0 + y // n))
            row.append(DARK if t is None else TONES[t % 2])
        rows.append(row)
    return rows


def stack(strips):
    """Strips (lists of rows) one above another, 8 px apart."""
    width = max(len(s[0]) for s in strips)
    out = []
    for k, s in enumerate(strips):
        for row in s:
            out.append(row + [GAP] * (width - len(row)))
        if k < len(strips) - 1:
            out += [[GAP] * width for _ in range(8)]
    return width, len(out), [p for row in out for p in row]


def side_by_side(blocks):
    height = max(len(b) for b in blocks)
    rows = [[] for _ in range(height)]
    for k, b in enumerate(blocks):
        for y in range(height):
            rows[y] += (b[y] if y < len(b) else [GAP] * len(b[0])) + ([GAP] * 8 if k < len(blocks) - 1 else [])
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
    shots = {run: {k: pngio.read(run / f"shots/{k}-r1.png") for k in ("pp-G-1024x640", "pp-H-1280x640",
                                                                       "pp-J-1024x640")}
             for run in (before, after)}

    # Board G: 8x unflipped and 6x flipped at offsets 0, 0.625, 1 and 3 (1/256 px).
    cells = sprite_lookdev.sweep_layout()
    for m, flipped in ((8, False), (6, True)):
        picked = [c for c in cells if c[0] == m and c[1] == flipped and c[2] in (0, 3, 4, 6)]
        strips = [side_by_side([decoded(shots[run]["pp-G-1024x640"], cx - 1, cy - 1, 7 * m + 3, 7 * m + 3, 4)
                                for _, _, _, cx, cy in picked]) for run in (before, after)]
        tag = f"{m}x-{'flipped' if flipped else 'unflipped'}"
        pngio.write_rgba(out / f"decoded-board-g-{tag}-offsets-0-0.625-1-3-before-above-after-zoom4.png",
                         *stack(strips))

    # Board H: 32x unflipped strips at offsets 0, 0.625, 1, 2 and 8 (1/256 px).
    scene = json.loads((after / "scene.json").read_text(encoding="utf-8"))
    high = sorted((meta["cell"][0], meta["cell"][1]) for key, meta in scene["meta"].items()
                  if key.startswith("H:high:32x:plain:"))
    strips = [side_by_side([decoded(shots[run]["pp-H-1280x640"], x - 2, y, 7 * 32 + 4, 32, 2) for x, y in high])
              for run in (before, after)]
    pngio.write_rgba(out / "decoded-board-h-32x-offsets-0-0.625-1-2-8-before-above-after-zoom2.png", *stack(strips))

    # Board J: the flipped strips' leading texels at width x magnification 16,384 and 32,768.
    rows = [(key, meta) for key, meta in scene["meta"].items() if meta.get("kind") == "map" and meta["flipped"]
            and (meta["width"], meta["magnification"]) in ((2048, 8), (4096, 4), (4096, 8), (8192, 4), (8192, 8))]
    rows.sort(key=lambda kv: (kv[1]["width"], kv[1]["magnification"]))
    strips = []
    for run in (before, after):
        blocks = [decoded(shots[run]["pp-J-1024x640"], 816, meta["row_y"], 48, meta["magnification"], 6)
                  for _, meta in rows]
        strips.append([row for block in blocks for row in block + [[GAP] * len(block[0])] * 6])
    pngio.write_rgba(out / "decoded-board-j-flipped-leading-texels-before-above-after-zoom6.png", *stack(strips))
    (out / "board-j-rows.json").write_text(json.dumps([[m["width"], m["magnification"]] for _, m in rows]),
                                           encoding="utf-8")
    print(f"{len(COPIES)} engine frames and decoded views written to {out}")


if __name__ == "__main__":
    main()
