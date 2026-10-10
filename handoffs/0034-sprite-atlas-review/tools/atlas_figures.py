#!/usr/bin/env python3
"""Derived review figures for handoff 0034 (not engine frames). Standard library only.

  decoded-x-cuts / decoded-y-cuts  every strip pixel of boards X and Y redrawn by the
      parity of the texel its colour names: a duplicated or dropped texel breaks the
      checker. Red ticks in the gaps mark span cuts (and the 32x leading/trailing edges).
  atlas-pages-v1-v2-v3             cooked level-0 pages decoded from KTX2 at 1/4 scale, with
      the card 7, 700-texel and 1100-texel frames outlined.
  board-b-v1-v2-v3-zoom3           board B from the v1, v2 and v3 pixel-perfect frames.
  linear-diff-rl-L-v1-v3-x64       rolled linear board, |v3 - v1| x 64 over a dimmed frame.
  art-contact-sheet                the new source art enlarged over a checker.
  knife-edges-control-*            (--followup-round1) rolled control views of the follow-up
      build, dimmed, with every pixel that differs from round one's build ringed in red.
  decoded-upper-y-40x-cut-ties     (--upper) vertical 40x columns of the upper-span probe around
      the 512 cut, 4x wide; exact ties showing the wrong texel are red.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_figures.py RUN ANALYSIS.json OUT_DIR
"""

import argparse
import json
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import atlas_analyse as an  # noqa: E402
import atlas_scene as sc  # noqa: E402
import pngio  # noqa: E402

BG = (24, 22, 40, 255)
EVEN, ODD = (226, 226, 226, 255), (44, 44, 56, 255)
RED = (255, 64, 64, 255)


def write(path, w, h, px):
    pngio.write_rgba(path, w, h, px)


def decoded(run, scene, phase, board, png, out):
    img = an.image(png)
    w, h = img.width, img.height
    px = [BG] * (w * h)
    for case in an.strip_cases(scene):
        if case["board"] != board:
            continue
        along = w if case["axis"] == 0 else h
        for u in range(along):
            for c in range(case["m"]):
                v = case["cross"] + c
                x, y = (u, v) if case["axis"] == 0 else (v, u)
                if not (0 <= x < w and 0 <= y < h):
                    continue
                texel = an.strip_texel(img.rgb(x, y), phase)
                if texel is not None and texel[1] in ((case["base"],) if isinstance(case["base"], int)
                                                      else case["base"]):
                    px[y * w + x] = EVEN if texel[0] % 2 == 0 else ODD
        marks = [c for c in an.cut_positions(case)] + ([case["start"]] if "lead" in case["name"] else []) + (
            [case["start"] + case["length"] * case["m"]] if "trail" in case["name"] else [])
        for mark in marks:
            u = int(mark)
            for d in range(1, 4):
                v = case["cross"] - d
                x, y = (u, v) if case["axis"] == 0 else (v, u)
                if 0 <= x < w and 0 <= y < h:
                    px[y * w + x] = RED
    write(out, w, h, px)


ROUND1 = ("s0-r1", "s1-v2", "s5-v3")
FOLLOWUP = ("f0-v1-r1", "f1-v2", "f4-v3")


def pages(run, analysis, out, names=ROUND1):
    states = [(label, analysis["fingerprints"][name]) for label, name in zip(("v1", "v2", "v3"), names)]
    tiles = []
    for label, fp in states:
        meta = json.loads((run / f"atlas/.incant/cache/textures/{fp}.json").read_text(encoding="utf-8"))
        k = an.read_ktx2(run / f"atlas/.incant/cache/textures/{meta['content_sha256']}.ktx2")
        w, h, page = k["width"], k["height"], k["level0"]
        tw, th = w // 4, h // 4
        tile = []
        for y in range(th):
            for x in range(tw):
                # Max-alpha texel of each 4 x 4 block keeps 1-texel strips visible.
                best = max((page[4 * ((4 * y + j) * w + 4 * x + i):4 * ((4 * y + j) * w + 4 * x + i) + 4]
                            for j in range(4) for i in range(4)), key=lambda p: p[3])
                chk = 52 if ((x // 4 + y // 4) % 2) else 40
                a = best[3] / 255
                tile.append(tuple(round(c * a + chk * (1 - a)) for c in best[:3]) + (255,))
        colours = {"card7": (255, 64, 64, 255), "hmid": (64, 255, 96, 255), "vmid": (64, 255, 96, 255),
                   "hstrip": (255, 220, 64, 255), "vstrip": (255, 220, 64, 255)}
        for frame in meta["atlas"]["frames"].values():
            colour = colours.get(frame["name"])
            if colour is None:
                continue
            x0, y0, fw, fh = frame["region"]
            x1, y1 = (x0 + fw - 1) // 4, (y0 + fh - 1) // 4
            x0, y0 = x0 // 4, y0 // 4
            for x in range(max(0, x0 - 1), min(tw, x1 + 2)):
                for y in (max(0, y0 - 1), min(th - 1, y1 + 1)):
                    tile[y * tw + x] = colour
            for y in range(max(0, y0 - 1), min(th, y1 + 2)):
                for x in (max(0, x0 - 1), min(tw - 1, x1 + 1)):
                    tile[y * tw + x] = colour
        tiles.append((tw, th, tile))
    gap = 12
    W = sum(t[0] for t in tiles) + gap * (len(tiles) + 1)
    H = max(t[1] for t in tiles) + 2 * gap
    px = [BG] * (W * H)
    x0 = gap
    for tw, th, tile in tiles:
        for y in range(th):
            px[(gap + y) * W + x0:(gap + y) * W + x0 + tw] = tile[y * tw:(y + 1) * tw]
        x0 += tw + gap
    write(out, W, H, px)


def board_b(run, out, names=ROUND1):
    imgs = [an.image(run / f"states/{s}/pp-B-320x180.png") for s in names]
    k, crop = 3, (0, 0, 240, 120)
    cw, ch = crop[2] * k, crop[3] * k
    W, H = 3 * cw + 4 * 8, ch + 16
    px = [(0, 0, 0, 255)] * (W * H)
    for n, img in enumerate(imgs):
        ox = 8 + n * (cw + 8)
        for y in range(ch):
            for x in range(cw):
                px[(8 + y) * W + ox + x] = img.px(crop[0] + x // k, crop[1] + y // k)
    write(out, W, H, px)


def linear_diff(run, out, names=ROUND1):
    a = an.image(run / f"states/{names[0]}/rl-L-640x360.png")
    b = an.image(run / f"states/{names[2]}/rl-L-640x360.png")
    px = []
    for y in range(a.height):
        for x in range(a.width):
            pa, pb = a.rgb(x, y), b.rgb(x, y)
            d = max(abs(p - q) for p, q in zip(pa, pb))
            base = tuple(c // 4 for c in pa)
            px.append((min(255, 64 * d + base[0]), base[1], base[2], 255) if d else base + (255,))
    write(out, a.width, a.height, px)


def upper_cut(probe, out, m=40, y0=560, y1=1080, scale=4):
    """Vertical upper-span probe at m x around the 512 cut: texel parity, failed exact ties in red."""
    import math
    import atlas_upper_probe as up
    manifest = json.loads((probe / "art/manifest.json").read_text(encoding="utf-8"))
    phase = manifest["strip"]["phase"]
    s, _ = up.scene("atlas", manifest, {"atlas": "-", "hstrip.png": "-", "vstrip.png": "-", "sheet.png": "-"})
    img = an.image(probe / "states/atlas/pp-UV-1920x1080.png")
    cols = [mt for mt in s.meta.values() if mt.get("board") == "UV" and mt.get("m") == m]
    x0, x1 = min(c["cross"] for c in cols) - 2, max(c["cross"] for c in cols) + 6
    w, h = (x1 - x0) * scale, (y1 - y0)
    px = [BG] * (w * h)
    for c in cols:
        for v in range(y0, y1):
            t = (v + 0.5 - c["start"]) / m
            want = an.expected_index(t, 1100, m, False)
            for i in range(4):
                got = an.strip_texel(img.rgb(c["cross"] + i, v), phase)
                colour = EVEN if got and got[0] % 2 == 0 else ODD
                if t == math.floor(t) and got and got[0] != want:
                    colour = RED
                for k in range(scale):
                    px[(v - y0) * w + (c["cross"] + i - x0) * scale + k] = colour
    write(out, w, h, px)


def knife_map(a_png, b_png, out):
    """Frame b dimmed, with every pixel that differs from frame a ringed in red (3 x 3)."""
    a, b = an.image(a_png), an.image(b_png)
    px = [tuple(c // 3 for c in b.rgb(x, y)) + (255,) for y in range(b.height) for x in range(b.width)]
    for y in range(b.height):
        for x in range(b.width):
            if a.rgb(x, y) != b.rgb(x, y):
                for dy in (-1, 0, 1):
                    for dx in (-1, 0, 1):
                        if 0 <= x + dx < b.width and 0 <= y + dy < b.height and (dx or dy):
                            px[(y + dy) * b.width + x + dx] = RED
                px[y * b.width + x] = b.rgb(x, y) + (255,)
    write(out, b.width, b.height, px)


def art_preview(art, out):
    """Contact sheet of the new original art over a checker (not an engine frame)."""
    items = [("cards.png", 8), ("probe.png", 12), ("halo.png", 8), ("labels34.png", 3), ("ballast-v1.png", 1),
             ("ballast-v2.png", 1)]
    imgs = [(pngio.read(art / n), s) for n, s in items]
    W = max(i.width * s for i, s in imgs) + 16
    H = sum(i.height * s + 8 for i, s in imgs) + 8
    px = [BG] * (W * H)
    y0 = 8
    for img, s in imgs:
        for y in range(img.height * s):
            for x in range(img.width * s):
                r, g, b, a = img.px(x // s, y // s)
                chk = 46 if ((x // 6 + y // 6) % 2) else 34
                al = a / 255
                px[(y0 + y) * W + 8 + x] = tuple(round(c * al + chk * (1 - al)) for c in (r, g, b)) + (255,)
        y0 += img.height * s + 8
    write(out, W, H, px)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("analysis", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--upper", type=Path, help="an atlas_upper_probe.py output directory")
    parser.add_argument("--followup-round1", type=Path, metavar="ROUND1_RUN",
                        help="RUN is an atlas_followup.py run; also map rolled-view changes against ROUND1_RUN")
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    args.out.mkdir(parents=True)
    run = args.run.resolve()
    analysis = json.loads(args.analysis.read_text(encoding="utf-8"))
    scene = json.loads((run / "scene-atlas.json").read_text(encoding="utf-8"))
    phase = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))["strip"]["phase"]
    names = FOLLOWUP if args.followup_round1 else ROUND1
    decoded(run, scene, phase, "X", run / f"states/{names[0]}/pp-X-1024x832.png",
            args.out / "decoded-x-cuts-1024x832.png")
    decoded(run, scene, phase, "Y", run / f"states/{names[0]}/pp-Y-832x1024.png",
            args.out / "decoded-y-cuts-832x1024.png")
    pages(run, analysis, args.out / "atlas-pages-v1-v2-v3-quarter.png", names)
    board_b(run, args.out / "board-b-v1-v2-v3-zoom3.png", names)
    linear_diff(run, args.out / "linear-diff-rl-L-v1-v3-x64.png", names)
    if args.followup_round1:
        round1 = args.followup_round1.resolve()
        for shot in ("rl-W-1280x288", "rl-V-360x1080"):
            knife_map(round1 / f"states/control/{shot}.png", run / f"states/control/{shot}.png",
                      args.out / f"knife-edges-control-{shot}.png")
    else:
        art_preview(run / "art", args.out / "art-contact-sheet.png")
    if args.upper:
        upper_cut(args.upper.resolve(), args.out / "decoded-upper-y-40x-cut-ties.png")
    print(json.dumps(sorted(p.name for p in args.out.iterdir())))


if __name__ == "__main__":
    main()
