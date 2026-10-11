#!/usr/bin/env python3
"""Derived contact sheets for handoff 0042. Standard library only; no engine calls.

Every figure is cut from unedited native_frames PNGs (or the decoded cooked atlas
page) by cropping and integer nearest upscaling, with 3x5 labels in the margins.
None is an engine image and no engine pixel is altered.

  board-a-frames-zoom2.png        board A at one tick per frame (0, 9, 17, 26, 34) and the wrap (42)
  boundaries-row1-zoom3.png       board A row 1 on both sides of every frame boundary
  controls-timeline-zoom2.png     board C entities (rows) across ticks with reported state
  board-d-order-zoom2.png         board D painter order at ticks 0, 25, 50 and 69
  magnified-d7-d9-zoom2.png       4x nearest, 4x linear and linear nine-slice at each frame
  board-b-motion-zoom2.png        board B transforms and motion at ticks 0, 25, 50, 69 and 420
  ortho-t0-t17-t42.png            2.5x orthographic captures at ticks 0, 17 and 42
  details-zoom6.png               flips (A1, A3-A5), mirrored nine-slice (A11, A13), sheet linear
                                  (A16), D5 between ground and overlay tiles, moving D3 under a badge
  atlas-page-zoom8.png            decoded cooked level-0 atlas page with packed frames outlined

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/fb_figures.py RUN ANALYSIS.json OUT_DIR
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
sys.path.insert(0, str(HELPERS))
import fb_analyse as fa  # noqa: E402
import fb_model as fm  # noqa: E402
import fb_png  # noqa: E402
import fb_scene as fs  # noqa: E402

BACK = (12, 12, 16)
INK = (220, 220, 228)
MUTED = (130, 130, 150)
WHITE = (240, 240, 240)
LABEL_H = 9


class Figure:
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.rows = [bytearray(bytes((*BACK, 255)) * width) for _ in range(height)]

    def put(self, x, y, rgb):
        if 0 <= x < self.width and 0 <= y < self.height:
            self.rows[y][4 * x:4 * x + 4] = bytes((*rgb[:3], 255))

    def paste(self, img, sx, sy, w, h, dx, dy, scale):
        crop = img.crop(sx, sy, w, h)
        for y in range(h):
            row = crop.rows[y]
            line = bytearray()
            for x in range(w):
                line += bytes(row[4 * x:4 * x + 3]) + b"\xff"
            wide = bytearray()
            for x in range(w):
                wide += bytes(line[4 * x:4 * x + 4]) * scale
            for k in range(scale):
                ty = dy + y * scale + k
                if 0 <= ty < self.height:
                    lo = max(0, dx)
                    hi = min(self.width, dx + w * scale)
                    if hi > lo:
                        self.rows[ty][4 * lo:4 * hi] = wide[4 * (lo - dx):4 * (hi - dx)]

    def text(self, x, y, value, rgb=INK, scale=1):
        for ch in str(value).upper():
            shape = fb_png.FONT.get(ch, fb_png.FONT[" "])
            for r, line in enumerate(shape):
                for c, mark in enumerate(line):
                    if mark == "#":
                        for dy in range(scale):
                            for dx in range(scale):
                                self.put(x + c * scale + dx, y + r * scale + dy, rgb)
            x += 4 * scale

    def ring(self, x0, y0, w, h, rgb):
        for x in range(x0 - 1, x0 + w + 1):
            self.put(x, y0 - 1, rgb)
            self.put(x, y0 + h, rgb)
        for y in range(y0 - 1, y0 + h + 1):
            self.put(x0 - 1, y, rgb)
            self.put(x0 + w, y, rgb)

    def write(self, path):
        fb_png.write_rgba(path, self.width, self.height, [bytes(r) for r in self.rows])


def world_box(camera, x0, y0, x1, y1):
    """Screen crop (x, y, w, h) of a world rectangle."""
    sx0, sy0 = camera.to_screen(x0, y1)
    sx1, sy1 = camera.to_screen(x1, y0)
    return int(sx0), int(sy0), int(sx1 - sx0), int(sy1 - sy0)


def grid(run, capture, ticks, box, scale, columns, labels=None):
    x, y, w, h = box
    cell_w, cell_h = w * scale, h * scale + LABEL_H
    rows = -(-len(ticks) // columns)
    fig = Figure(columns * (cell_w + 8) - 8, rows * (cell_h + 6) - 6)
    for i, tick in enumerate(ticks):
        dx, dy = (i % columns) * (cell_w + 8), (i // columns) * (cell_h + 6)
        fig.text(dx, dy + 1, labels[i] if labels else f"T{tick}")
        fig.paste(fa.image(run, capture, tick), x, y, w, h, dx, dy + LABEL_H, scale)
    return fig


def controls_timeline(run, sc, analysis, camera):
    names = [i["name"] for i in sc.items if i["name"].startswith("C")]
    ticks = [0, 9, 10, 12, 16, 17, 20, 24, 25, 26, 33, 34, 41, 42, 50, 51, 59, 60, 68, 69]
    scale, cell, gap, head = 2, 28, 4, 40
    fig = Figure(head + len(ticks) * (cell * scale + gap), LABEL_H + len(names) * (cell * scale + LABEL_H + gap))
    for j, tick in enumerate(ticks):
        fig.text(head + j * (cell * scale + gap), 1, f"T{tick}")
    q = fm.quads(sc, 0, camera)
    for i, name in enumerate(names):
        top = LABEL_H + i * (cell * scale + LABEL_H + gap)
        fig.text(0, top + 20, name.split("-")[0])
        x0, y0, _, _ = fm.bbox(q[name]["screen"], *fs.VIEW)
        for j, tick in enumerate(ticks):
            dx = head + j * (cell * scale + gap)
            fig.paste(fa.image(run, "main", tick), x0 - 2, y0 - 2, cell, cell, dx, top, scale)
            frame, playing, completed = analysis["states"][name][str(tick)]
            state = "P" if playing else ("C" if completed else "-")
            fig.text(dx + 2, top + cell * scale + 1, f"{frame[1]} {state}", MUTED if state == "-" else INK)
    return fig


def details(run, sc, camera):
    def box(name, tick, pad):
        x0, y0, x1, y1 = fm.bbox(fm.quads(sc, tick, camera)[name]["screen"], *fs.VIEW, pad=pad)
        return x0, y0, x1 - x0, y1 - y0

    cells = [("A1 T9", 9, box("A1-nearest", 0, 3)), ("A3 T9", 9, box("A3-flip-x", 0, 3)),
             ("A4 T9", 9, box("A4-flip-y", 0, 3)), ("A5 T9", 9, box("A5-flip-xy", 0, 3)),
             ("A11 T26", 26, box("A11-nine-nearest", 0, 3)), ("A13 T26", 26, box("A13-nine-flip-x", 0, 3)),
             ("A16 T17", 17, box("A16-sheet-linear", 0, 3)), ("D5 T0", 0, box("D5-above-ground", 0, 10)),
             ("D3 T69", 69, box("D3-moving-paused", 69, 12))]
    s, gap = 6, 10
    fig = Figure(sum(c[2][2] * s + gap for c in cells) - gap, max(c[2][3] for c in cells) * s + LABEL_H + 2)
    dx = 0
    for label, tick, (x, y, w, h) in cells:
        fig.text(dx, 1, label)
        fig.paste(fa.image(run, "main", tick), x, y, w, h, dx, LABEL_H + 2, s)
        dx += w * s + gap
    return fig


def atlas_page(run, analysis):
    cache = run / "anim/.incant/cache/textures"
    meta = analysis["atlas"]
    ktx = fa.read_ktx2(cache / f"{meta['ktx2_sha256']}.ktx2")
    w, h = meta["size"]
    s = 8
    fig = Figure(w * s, h * s + LABEL_H * 2)
    page = ktx["level0"]
    for y in range(h * s):
        for x in range(w * s):
            i = (y // s) * w + x // s
            r, g, b, a = page[4 * i:4 * i + 4]
            check = (70, 70, 76) if ((x // 8 + y // 8) % 2) else (96, 96, 104)
            fig.put(x, y + LABEL_H * 2, (r, g, b) if a else check)
    for name, frame in meta["frames"].items():
        rx, ry, rw, rh = frame["region"]
        fig.ring(rx * s, ry * s + LABEL_H * 2, rw * s, rh * s, WHITE)
    fig.text(0, 1, f"COOKED ATLAS PAGE {w}X{h} PADDING {meta['padding']}")
    return fig


def main():
    run, analysis_path, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), Path(sys.argv[3])
    if out.exists():
        raise SystemExit("output directory already exists")
    out.mkdir(parents=True)
    analysis = json.loads(analysis_path.read_text(encoding="utf-8"))
    sc = fs.build()
    camera = fm.Camera(sc, "camera-pp", *fs.VIEW)
    board_a = world_box(camera, -19.75, 2.875, -0.25, 11.125)
    grid(run, "main", [0, 9, 17, 26, 34, 42], board_a, 2, 2).write(out / "board-a-frames-zoom2.png")
    row1 = world_box(camera, -19.0, 8.625, -2.0, 10.375)
    pairs = [8, 9, 16, 17, 25, 26, 33, 34, 41, 42, 50, 51]
    grid(run, "main", pairs, row1, 3, 2).write(out / "boundaries-row1-zoom3.png")
    controls_timeline(run, sc, analysis, camera).write(out / "controls-timeline-zoom2.png")
    board_d = world_box(camera, 0.25, -11.0, 19.75, 2.5)
    grid(run, "main", [0, 25, 50, 69], board_d, 2, 2).write(out / "board-d-order-zoom2.png")
    big = world_box(camera, 2.75, -2.25, 17.25, 1.25)
    grid(run, "main", [0, 9, 17, 26, 34], big, 2, 1).write(out / "magnified-d7-d9-zoom2.png")
    board_b = world_box(camera, 0.25, 2.875, 19.75, 11.125)
    grid(run, "main", [0, 25, 50, 69, 420], board_b, 2, 1).write(out / "board-b-motion-zoom2.png")
    grid(run, "ortho", [0, 17, 42], (0, 0, 640, 360), 1, 3).write(out / "ortho-t0-t17-t42.png")
    details(run, sc, camera).write(out / "details-zoom6.png")
    atlas_page(run, analysis).write(out / "atlas-page-zoom8.png")
    print(json.dumps({"figures": sorted(p.name for p in out.iterdir())}))


if __name__ == "__main__":
    main()
