"""Reviewer inspection, CPU only: the same per-region Lucas-Kanade translation measured
on linear-light luminance (x255), which a linear-light anti-aliased edge does not bias
towards the dark side the way sRGB luma does.

  python3 -I -B registration_regions_linear.py <on.png>,<off.png> <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[2:])
on_name, off_name = sys.argv[1].split(",")
LUT = [(v / 255 / 12.92) if v / 255 <= 0.04045 else ((v / 255 + 0.055) / 1.055) ** 2.4 for v in range(256)]
def luma(path):
    w, h, rows = pngio.read(str(path)); data = b"".join(rows)
    return w, h, [255 * (0.2126 * LUT[data[i*4]] + 0.7152 * LUT[data[i*4+1]] + 0.0722 * LUT[data[i*4+2]]) for i in range(w * h)]
def sample(img, w, h, x, y):
    x = min(max(x, 0.0), w - 1.001); y = min(max(y, 0.0), h - 1.001)
    x0, y0 = int(x), int(y); fx, fy = x - x0, y - y0; i = y0 * w + x0
    return img[i] * (1 - fx) * (1 - fy) + img[i + 1] * fx * (1 - fy) + img[i + w] * (1 - fx) * fy + img[i + w + 1] * fx * fy
def register(on, off, w, h, box):
    x0, y0, x1, y1 = box
    pts = [(x, y) for y in range(max(2, y0), min(h - 2, y1)) for x in range(max(2, x0), min(w - 2, x1))
           if ((off[y*w+x+1] - off[y*w+x-1]) / 2) ** 2 + ((off[(y+1)*w+x] - off[(y-1)*w+x]) / 2) ** 2 > 100.0]
    if len(pts) < 200: return None, len(pts)
    s = [0.0, 0.0]
    for _ in range(6):
        a = b = c = d = e = 0.0
        for x, y in pts:
            r = sample(off, w, h, x - s[0], y - s[1])
            gx = (sample(off, w, h, x + 1 - s[0], y - s[1]) - sample(off, w, h, x - 1 - s[0], y - s[1])) / 2
            gy = (sample(off, w, h, x - s[0], y + 1 - s[1]) - sample(off, w, h, x - s[0], y - 1 - s[1])) / 2
            t = on[y * w + x] - r
            a += gx * gx; b += gx * gy; c += gy * gy; d += gx * t; e += gy * t
        det = a * c - b * b
        if abs(det) < 1e-9: return None, len(pts)
        dx = -(c * d - b * e) / det; dy = -(a * e - b * d) / det
        s[0] += dx; s[1] += dy
        if abs(dx) + abs(dy) < 1e-4: break
    return s, len(pts)
regions = {"horizon band y20-60": (0, 20, 960, 60), "posts x230-460 y30-230": (230, 30, 460, 230),
           "cube+wall x250-510 y170-330": (250, 170, 510, 330), "sphere x430-560 y205-335": (430, 205, 560, 335),
           "grille x535-680 y145-275": (535, 145, 680, 275), "shadows/ground y300-540": (0, 300, 960, 540)}
for label, d in runs.items():
    w, h, on = luma(find(d, on_name)); _, _, off = luma(find(d, off_name))
    out = []
    for name, box in regions.items():
        s, n = register(on, off, w, h, box)
        out.append("%s: %s (n=%d)" % (name, "(%+.2f, %+.2f)" % tuple(s) if s else "n/a", n))
    print("%-9s" % label, " | ".join(out))
