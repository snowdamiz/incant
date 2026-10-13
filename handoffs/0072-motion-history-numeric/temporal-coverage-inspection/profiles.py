"""Reviewer inspection, CPU only: linear-light vertical profiles across a horizontal edge
and the integrated coverage (sum of (L - L_top)/(L_bottom - L_top) over the window) per
column. The label off reads the temporal-off file.

  python3 -I -B profiles.py <on.png> <x0,x1,y0,y1> <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[3:])
name = sys.argv[1]
x0, x1, y0, y1 = map(int, sys.argv[2].split(","))
def lin(v):
    v /= 255.0
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
def column(data, w, x):
    return [lin(0.2126 * data[(y*w+x)*4] + 0.7152 * data[(y*w+x)*4+1] + 0.0722 * data[(y*w+x)*4+2]) for y in range(y0, y1)]
images = {}
for label, d in runs.items():
    w, h, rows = pngio.read(str(find(d, name if label != "off" else sys.argv[1].replace("on-f16", "off").replace("on-f8", "off-f8"))))
    images[label] = (w, b"".join(rows))
for x in range(x0, x1, max(1, (x1 - x0) // 4)):
    print("column", x)
    for label, (w, data) in images.items():
        col = column(data, w, x)
        top, bottom = col[0], col[-1]
        cov = sum((v - top) / (bottom - top) for v in col) if abs(bottom - top) > 1e-4 else float("nan")
        print("  %-9s" % label, " ".join("%5.3f" % v for v in col), "| coverage sum %.2f" % cov)
