"""Reviewer inspection, CPU only: context for detail ratios. The detail the temporal-off
reference retains after a discrete separable Gaussian blur of sigma px (linear light);
the sigma whose retention matches a run's detail ratio is that run's effective softness.

  python3 -I -B blur_equivalent.py <off.png> '<name>=x0,y0,x1,y1;...' <dir holding 0067 pngio.py> off=<dir>
"""
import math, sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[3:])
off_name = sys.argv[1]
regions = dict(item.split("=") for item in sys.argv[2].split(";"))
LUT = [(v / 255 / 12.92) if v / 255 <= 0.04045 else ((v / 255 + 0.055) / 1.055) ** 2.4 for v in range(256)]
w, h, rows = pngio.read(str(find(runs["off"], off_name))); d = b"".join(rows)
L = [0.2126 * LUT[d[i*4]] + 0.7152 * LUT[d[i*4+1]] + 0.0722 * LUT[d[i*4+2]] for i in range(w * h)]
def blur(img, box, sigma):
    x0, y0, x1, y1 = box; r = max(1, int(math.ceil(3 * sigma)))
    k = [math.exp(-(t * t) / (2 * sigma * sigma)) for t in range(-r, r + 1)]; s = sum(k); k = [v / s for v in k]
    pad = r + 2; X0, Y0, X1, Y1 = x0 - pad, y0 - pad, x1 + pad, y1 + pad
    tmp = {}
    for y in range(Y0, Y1):
        for x in range(X0, X1):
            tmp[(x, y)] = sum(k[t + r] * img[y * w + min(max(x + t, 0), w - 1)] for t in range(-r, r + 1))
    out = {}
    for y in range(Y0 + r, Y1 - r):
        for x in range(X0, X1):
            out[(x, y)] = sum(k[t + r] * tmp[(x, y + t)] for t in range(-r, r + 1))
    return out
def detail(get, box):
    x0, y0, x1, y1 = box; total = 0.0; n = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            m = sum(get(x + dx, y + dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
            total += abs(get(x, y) - m); n += 1
    return total / n
for name, spec in regions.items():
    box = tuple(map(int, spec.split(",")))
    base = detail(lambda x, y: L[y * w + x], box)
    ratios = []
    for sigma in (0.29, 0.5, 0.7, 0.9, 1.1, 1.4):
        b = blur(L, box, sigma)
        ratios.append("s%.2f:%3.0f%%" % (sigma, 100 * detail(lambda x, y: b[(x, y)], box) / base))
    print("%-14s" % name, " ".join(ratios))
