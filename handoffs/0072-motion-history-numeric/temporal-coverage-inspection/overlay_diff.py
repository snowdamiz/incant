"""Reviewer inspection, CPU only: excluded case f8, coverage against corrected per overlay
region (max-channel sRGB difference), and each against the temporal-off frame. Overlays
draw after history in both runs. Needs labels off, corrected and coverage.

  python3 -I -B overlay_diff.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[1:])
def img(d, name):
    w, h, rows = pngio.read(str(find(d, name))); return w, b"".join(rows)
w, off = img(runs["off"], "excluded-off.png")
_, cor = img(runs["corrected"], "excluded-on-f8.png")
_, cov = img(runs["coverage"], "excluded-on-f8.png")
regions = {"world text": (290, 170, 452, 200), "glass card": (541, 176, 662, 297), "sprites": (305, 265, 565, 366),
           "ui button": (24, 24, 264, 80), "checker only": (560, 330, 640, 430)}
d = lambda a, b, i: max(abs(a[i*4+c] - b[i*4+c]) for c in range(3))
for name, (x0, y0, x1, y1) in regions.items():
    out = []
    for label, (a, b) in (("coverage-corrected", (cov, cor)), ("coverage-off", (cov, off)), ("corrected-off", (cor, off))):
        vals = [d(a, b, y * w + x) for y in range(y0, y1) for x in range(x0, x1)]
        out.append("%s mean %.2f max %3d >8: %4d" % (label, sum(vals) / len(vals), max(vals), sum(v > 8 for v in vals)))
    print("%-13s" % name, " | ".join(out))
