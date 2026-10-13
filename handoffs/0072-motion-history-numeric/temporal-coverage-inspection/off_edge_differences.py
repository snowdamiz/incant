"""Reviewer inspection, CPU only: ghost/lag check. On-minus-off differences at pixels at
least 3 px (8-connected) from any edge (neighbour step > 12) of either image. Set
PAIR=0..3 to evaluate one image pair.

  python3 -I -B off_edge_differences.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
from collections import deque
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, load
pngio, runs = setup(sys.argv[1:])
delta = lambda a, b, i: max(abs(a[i*4+c] - b[i*4+c]) for c in range(3))
def edge_union(images, w, h, threshold=12):
    e = bytearray(w * h)
    for img in images:
        for y in range(h):
            for x in range(w):
                i = y * w + x
                if x + 1 < w and max(abs(img[i*4+c] - img[(i+1)*4+c]) for c in range(3)) > threshold: e[i] = e[i + 1] = 1
                if y + 1 < h and max(abs(img[i*4+c] - img[(i+w)*4+c]) for c in range(3)) > threshold: e[i] = e[i + w] = 1
    return e
def distance(e, w, h):
    d = [1 << 30] * (w * h); q = deque()
    for i, v in enumerate(e):
        if v: d[i] = 0; q.append(i)
    while q:
        i = q.popleft(); y, x = divmod(i, w)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and d[ny * w + nx] > d[i] + 1:
                    d[ny * w + nx] = d[i] + 1; q.append(ny * w + nx)
    return d
pairs = [("stationary-on-f16.png", "stationary-off.png"), ("motion-object-on-f8.png", "motion-object-off-f8.png"),
         ("motion-pan-on-f6.png", "motion-pan-off-f6.png"), ("skinned-on-f6.png", "skinned-off-f6.png")]
for on_name, off_name in [pairs[int(__import__("os").environ["PAIR"])]] if "PAIR" in __import__("os").environ else pairs:
    for label, d in runs.items():
        w, h, on = load(pngio, d, on_name); _, _, off = load(pngio, d, off_name)
        dist = distance(edge_union((on, off), w, h), w, h)
        bins = [0, 0, 0]; box = None; worst = 0
        for i in range(w * h):
            if dist[i] >= 3:
                v = delta(on, off, i); worst = max(worst, v)
                if 3 <= v <= 8: bins[0] += 1
                elif 9 <= v <= 16:
                    bins[1] += 1; y, x = divmod(i, w)
                    box = (min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)) if box else (x, y, x, y)
                elif v > 16: bins[2] += 1
        print("%-24s %-9s >=3px from edges: 3-8 %5d  9-16 %4d  >16 %3d  max %3d  9-16 bbox %s" % (on_name, label, *bins, worst, box))
