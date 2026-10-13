"""Reviewer inspection, CPU only: iterative Lucas-Kanade global translation of an
'on' frame against its unjittered temporal-off reference, on sRGB luma, over
reference pixels with a strong gradient. Positive = content in the 'on' frame sits
right/down of the reference. Reads published PNGs only.

  python3 -I -B registration.py <dir holding 0067 pngio.py> <packet dir>
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import pngio
from pathlib import Path
root = Path(sys.argv[2])
dirs = {"original": root / "temporal-visual-screenshots", "corrected": root / "temporal-corrected-screenshots"}

def luma(path):
    w, h, rows = pngio.read(str(path))
    data = b"".join(rows)
    return w, h, [0.2126 * data[i*4] + 0.7152 * data[i*4+1] + 0.0722 * data[i*4+2] for i in range(w * h)]

def sample(img, w, h, x, y):
    x = min(max(x, 0.0), w - 1.001); y = min(max(y, 0.0), h - 1.001)
    x0, y0 = int(x), int(y); fx, fy = x - x0, y - y0
    i = y0 * w + x0
    return (img[i] * (1 - fx) * (1 - fy) + img[i + 1] * fx * (1 - fy)
            + img[i + w] * (1 - fx) * fy + img[i + w + 1] * fx * fy)

def register(on, off, w, h, region=None):
    pts = []
    for y in range(2, h - 2):
        for x in range(2, w - 2):
            if region and not (region[0] <= x < region[2] and region[1] <= y < region[3]):
                continue
            i = y * w + x
            gx = (off[i + 1] - off[i - 1]) / 2; gy = (off[i + w] - off[i - w]) / 2
            if gx * gx + gy * gy > 100.0:  # |grad| > 10 luma steps per pixel
                pts.append((x, y))
    s = [0.0, 0.0]
    for _ in range(6):
        a = b = c = d = e = 0.0
        for x, y in pts:
            # reference shifted by s: R(x) = off(x - s)
            r = sample(off, w, h, x - s[0], y - s[1])
            gx = (sample(off, w, h, x + 1 - s[0], y - s[1]) - sample(off, w, h, x - 1 - s[0], y - s[1])) / 2
            gy = (sample(off, w, h, x - s[0], y + 1 - s[1]) - sample(off, w, h, x - s[0], y - 1 - s[1])) / 2
            t = on[y * w + x] - r
            a += gx * gx; b += gx * gy; c += gy * gy; d += gx * t; e += gy * t
        det = a * c - b * b
        # on - R ~ -ds . grad  =>  ds = -(G^-1) [d, e]
        dx = -(c * d - b * e) / det; dy = -(a * e - b * d) / det
        s[0] += dx; s[1] += dy
        if abs(dx) + abs(dy) < 1e-4:
            break
    return s, len(pts)

cases = [("stationary-on-f16.png", "stationary-off.png", 7, None),
         ("motion-object-on-f8.png", "motion-object-off-f8.png", 7, None),
         ("skinned-on-f6.png", "skinned-off-f6.png", 5, None),
         ("excluded-on-f8.png", "excluded-off.png", 7, None)]
J = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125), (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
for on_name, off_name, phase, region in cases:
    for label, d in dirs.items():
        w, h, on = luma(d / on_name)
        _, _, off = luma(d / off_name)
        s, n = register(on, off, w, h, region)
        print("%-24s %-9s phase %d jitter %s  shift (%+.3f, %+.3f) px over %d gradient pixels" % (on_name, label, phase, J[phase], s[0], s[1], n))
