"""Reviewer inspection, CPU only: activity-map histograms (peak consecutive max-channel
change x4; 255 = 64+ steps), split by the fixture's edge mask (temporal-off reference,
threshold 24, one-pixel dilation). The first run label supplies the reference.

  python3 -I -B activity_histogram.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, load
pngio, runs = setup(sys.argv[1:])

def edges(ref, w, h, threshold=24):
    step = lambda i, j: max(abs(ref[i*4+c] - ref[j*4+c]) for c in range(3))
    raw = bytearray(w * h)
    for y in range(h):
        for x in range(w):
            i = y * w + x
            if x + 1 < w and step(i, i + 1) > threshold: raw[i] = raw[i + 1] = 1
            if y + 1 < h and step(i, i + w) > threshold: raw[i] = raw[i + w] = 1
    out = bytearray(raw)
    for y in range(h):
        for x in range(w):
            if raw[y * w + x]:
                for ny in range(max(0, y - 1), min(h, y + 2)):
                    for nx in range(max(0, x - 1), min(w, x + 2)):
                        out[ny * w + nx] = 1
    return out

BINS = [(1, 2), (3, 8), (9, 16), (17, 32), (33, 63), (64, 999)]
peak = lambda v: 64 if v == 255 else v // 4
first = next(iter(runs.values()))
for case in ("stationary", "excluded"):
    w, h, ref = load(pngio, first, f"{case}-off.png")
    mask = edges(ref, w, h)
    print(case, "edge pixels", sum(mask))
    for label, d in runs.items():
        _, _, act = load(pngio, d, f"{case}-activity.png")
        counts = {b: [0, 0] for b in BINS}
        for i in range(w * h):
            p = peak(act[i * 4])
            for lo, hi in BINS:
                if lo <= p <= hi:
                    counts[(lo, hi)][mask[i]] += 1
        big = sum(c[0] + c[1] for (lo, _), c in counts.items() if lo >= 33)
        print("  %-9s" % label, "  ".join("%s-%s: e%d/i%d" % (lo, hi if hi < 999 else "", c[1], c[0]) for (lo, hi), c in counts.items()),
              "| >=33 total", big, "| saturated", sum(counts[(64, 999)]))
