#!/usr/bin/env python3
"""Derived review figures for handoff 0039. Standard library only; no engine calls.

Every figure is built from engine PNGs or decoded cooked pages by nearest
upscaling, cropping and marking; none is an engine image.

  board-b-order-s0-s1-zoom6.png    board B's maps and sprites, s0 left, s1 right
  s1-changes-pp1-A-B-zoom2.png     s0 frames dimmed, pixels changed by the s1 edit in red
  atlas-pages-v1-v2-zoom6.png      decoded level-0 pages v1 and v2 over a checker, frames outlined
  ties-off-rule-zoom12.png         every tie the engine resolved against the documented rule, ringed

Usage (repo root):
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_figures.py RUN ANALYSIS.json OUT_DIR
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
ROOT = HELPERS.parents[2]
sys.path.insert(0, str(HELPERS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import pngio  # noqa: E402
import tile_analyse as ta  # noqa: E402

BACK = (12, 12, 16)
RED = (255, 48, 48)
WHITE = (240, 240, 240)


class Figure:
    def __init__(self, width, height, fill=BACK):
        self.width, self.height = width, height
        self.data = [(*fill, 255)] * (width * height)

    def put(self, x, y, rgb):
        if 0 <= x < self.width and 0 <= y < self.height:
            self.data[y * self.width + x] = (*rgb[:3], 255)

    def paste(self, img, sx, sy, w, h, dx, dy, scale, colour=lambda c, x, y: c):
        for y in range(h * scale):
            for x in range(w * scale):
                px, py = sx + x // scale, sy + y // scale
                self.put(dx + x, dy + y, colour(img.px(px, py)[:3], px, py))

    def ring(self, x0, y0, w, h, rgb, thickness=1):
        for t in range(thickness):
            for x in range(x0 - t, x0 + w + t):
                self.put(x, y0 - 1 - t, rgb)
                self.put(x, y0 + h + t, rgb)
            for y in range(y0 - 1 - t, y0 + h + t + 1):
                self.put(x0 - 1 - t, y, rgb)
                self.put(x0 + w + t, y, rgb)

    def write(self, path):
        pngio.write_rgba(path, self.width, self.height, self.data)


def frame(run, folder, view):
    return ta.image(ta.shots(run, folder)[view][0])


def board_b(run, out):
    # World x 32.5..40.5, y -3..2 in the 320x180 pixel-perfect view centred on (40, 0).
    x0, y0, w, h, s, gap = 40, 58, 128, 80, 6, 12
    fig = Figure(2 * w * s + gap, h * s)
    for i, folder in enumerate(("s0--tile", "s1--tile")):
        fig.paste(frame(run, folder, "pp1-B"), x0, y0, w, h, i * (w * s + gap), 0, s)
    fig.write(out / "board-b-order-s0-s1-zoom6.png")


def changes(run, out):
    s, gap = 2, 8
    fig = Figure(2 * 320 * s + gap, 180 * s)
    for i, board in enumerate("AB"):
        before, after = frame(run, "s0--tile", f"pp1-{board}"), frame(run, "s1--tile", f"pp1-{board}")
        fig.paste(before, 0, 0, 320, 180, i * (320 * s + gap), 0, s,
                  lambda c, x, y, a=after, b=before: RED if a.px(x, y) != b.px(x, y) else tuple(v // 3 for v in c))
    fig.write(out / "s1-changes-pp1-A-B-zoom2.png")


def pages(run, analysis, out):
    cache = run / "tile/.incant/cache/textures"
    order = [analysis["workflow"]["v1_fingerprint"], analysis["relocation_v1_v2"]["to"]]
    s, gap = 6, 24
    decoded = [(analysis["pages"][f], ta.read_ktx2(cache / f"{analysis['pages'][f]['content_sha256']}.ktx2"))
               for f in order]
    width = sum(p["size"][0] * s for p, _ in decoded) + gap * (len(decoded) - 1)
    height = max(p["size"][1] * s for p, _ in decoded)
    fig = Figure(width, height)
    dx = 0
    for meta, ktx in decoded:
        w, h = meta["size"]
        page = ktx["level0"]
        for y in range(h * s):
            for x in range(w * s):
                i = (y // s) * w + x // s
                r, g, b, a = page[4 * i:4 * i + 4]
                check = (70, 70, 76) if ((x // 6 + y // 6) % 2) else (96, 96, 104)
                fig.put(dx + x, y, (r, g, b) if a == 255 else check)
        for name, (rx, ry, rw, rh) in meta["regions"].items():
            fig.ring(dx + rx * s, ry * s, rw * s, rh * s, WHITE)
        dx += w * s + gap
    fig.write(out / "atlas-pages-v1-v2-zoom6.png")


def ties(run, analysis, out):
    crops = []
    for entry in analysis["model"]:
        if not entry["other_side"] or not entry["state"].startswith("s0"):
            continue
        img = frame(run, entry["state"], entry["view"])
        pixels = [tuple(o["pixel"]) for o in entry["other_side"]]
        groups = []
        for p in pixels:
            for g in groups:
                if abs(g[0][0] - p[0]) <= 6 and abs(g[-1][1] - p[1]) <= 12:
                    g.append(p)
                    break
            else:
                groups.append([p])
        for g in groups:
            cx = sum(p[0] for p in g) // len(g)
            cy = sum(p[1] for p in g) // len(g)
            x0 = min(max(cx - 8, 0), img.width - 16)
            y0 = min(max(cy - 8, 0), img.height - 16)
            crops.append((entry["view"], img, x0, y0, g))
    s, gap = 12, 12
    fig = Figure(len(crops) * (16 * s + gap) - gap, 16 * s)
    for k, (_, img, x0, y0, marked) in enumerate(crops):
        dx = k * (16 * s + gap)
        fig.paste(img, x0, y0, 16, 16, dx, 0, s)
        for px, py in marked:
            if x0 <= px < x0 + 16 and y0 <= py < y0 + 16:
                fig.ring(dx + (px - x0) * s, (py - y0) * s, s, s, RED, 2)
    fig.write(out / "ties-off-rule-zoom12.png")
    return [{"view": view, "crop_origin": [x0, y0], "pixels": marked} for view, _, x0, y0, marked in crops]


def main():
    run, analysis_path, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), Path(sys.argv[3])
    if out.exists():
        raise SystemExit("output directory already exists")
    out.mkdir(parents=True)
    analysis = json.loads(analysis_path.read_text(encoding="utf-8"))
    board_b(run, out)
    changes(run, out)
    pages(run, analysis, out)
    crops = ties(run, analysis, out)
    print(json.dumps({"figures": sorted(p.name for p in out.iterdir()), "tie_crops": crops}))


if __name__ == "__main__":
    main()
