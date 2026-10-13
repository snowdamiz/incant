"""Reviewer inspection, CPU only: iterative Lucas-Kanade global translation of each 'on'
frame against its unjittered temporal-off reference (sRGB luma, strong-gradient
reference pixels). Positive = the 'on' content sits right/down of the reference.

  python3 -I -B registration.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[1:])
def luma(path):
    w, h, rows = pngio.read(str(path)); data = b"".join(rows)
    return w, h, [0.2126 * data[i*4] + 0.7152 * data[i*4+1] + 0.0722 * data[i*4+2] for i in range(w * h)]
def sample(img, w, h, x, y):
    x = min(max(x, 0.0), w - 1.001); y = min(max(y, 0.0), h - 1.001)
    x0, y0 = int(x), int(y); fx, fy = x - x0, y - y0; i = y0 * w + x0
    return img[i] * (1 - fx) * (1 - fy) + img[i + 1] * fx * (1 - fy) + img[i + w] * (1 - fx) * fy + img[i + w + 1] * fx * fy
def register(on, off, w, h):
    pts = [(x, y) for y in range(2, h - 2) for x in range(2, w - 2)
           if ((off[y*w+x+1] - off[y*w+x-1]) / 2) ** 2 + ((off[(y+1)*w+x] - off[(y-1)*w+x]) / 2) ** 2 > 100.0]
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
        dx = -(c * d - b * e) / det; dy = -(a * e - b * d) / det
        s[0] += dx; s[1] += dy
        if abs(dx) + abs(dy) < 1e-4: break
    return s, len(pts)
cases = [("stationary-on-f16.png", "stationary-off.png", 7), ("motion-object-on-f8.png", "motion-object-off-f8.png", 7),
         ("skinned-on-f6.png", "skinned-off-f6.png", 5), ("excluded-on-f8.png", "excluded-off.png", 7)]
for on_name, off_name, phase in cases:
    for label, d in runs.items():
        w, h, on = luma(find(d, on_name)); _, _, off = luma(find(d, off_name))
        s, n = register(on, off, w, h)
        print("%-24s %-9s phase %d shift (%+.3f, %+.3f) px over %d gradient pixels" % (on_name, label, phase, s[0], s[1], n))
