"""Reviewer inspection, CPU only: per region, linear-light mean (coverage preservation)
and local detail contrast (mean |L - 3x3 box mean|), each run against the temporal-off
reference.

  python3 -I -B detail.py <on.png>,<off.png> '<name>=x0,y0,x1,y1;...' <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[3:])
on_name, off_name = sys.argv[1].split(",")
regions = dict(item.split("=") for item in sys.argv[2].split(";"))
LUT = [(v / 255 / 12.92) if v / 255 <= 0.04045 else ((v / 255 + 0.055) / 1.055) ** 2.4 for v in range(256)]
def lum(path):
    w, h, rows = pngio.read(str(path)); d = b"".join(rows)
    return w, h, [0.2126 * LUT[d[i*4]] + 0.7152 * LUT[d[i*4+1]] + 0.0722 * LUT[d[i*4+2]] for i in range(w * h)]
images = {}
for label, d in runs.items():
    images[label] = lum(find(d, off_name if label == "off" else on_name))
for name, box in regions.items():
    x0, y0, x1, y1 = map(int, box.split(","))
    stats = {}
    for label, (w, h, L) in images.items():
        total = detail = 0.0; n = 0
        for y in range(y0, y1):
            for x in range(x0, x1):
                i = y * w + x
                box3 = sum(L[(y + dy) * w + x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
                total += L[i]; detail += abs(L[i] - box3); n += 1
        stats[label] = (total / n, detail / n)
    off_mean, off_detail = stats["off"]
    print("%-14s" % name, " | ".join("%s mean %.4f (%+.1f%%) detail %.4f (%3.0f%%)" % (
        label, m, 100 * (m - off_mean) / off_mean, dd, 100 * dd / off_detail if off_detail else float("nan"))
        for label, (m, dd) in stats.items()))
