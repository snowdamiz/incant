"""Reviewer inspection, CPU only: histogram of the published activity maps (peak
consecutive max-channel change x4, saturating at 255 = 64+ steps) for the original
and corrected runs, split by the fixture's own edge mask (temporal-off reference,
threshold 24, one-pixel dilation). Reads published PNGs only.

  python3 -I -B activity_histogram.py <dir holding 0067 pngio.py> <packet dir>
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import pngio
from pathlib import Path
root = Path(sys.argv[2])
old, new = root / "temporal-visual-screenshots", root / "temporal-corrected-screenshots"

def load(path):
    w, h, rows = pngio.read(str(path))
    return w, h, b"".join(rows)

def edges(ref, w, h, threshold=24):
    step = lambda i, j: max(abs(ref[i*4+c] - ref[j*4+c]) for c in range(3))
    raw = bytearray(w * h)
    for y in range(h):
        for x in range(w):
            i = y * w + x
            if x + 1 < w and step(i, i + 1) > threshold:
                raw[i] = raw[i + 1] = 1
            if y + 1 < h and step(i, i + w) > threshold:
                raw[i] = raw[i + w] = 1
    out = bytearray(raw)
    for y in range(h):
        for x in range(w):
            if raw[y * w + x]:
                for ny in range(max(0, y - 1), min(h, y + 2)):
                    for nx in range(max(0, x - 1), min(w, x + 2)):
                        out[ny * w + nx] = 1
    return out

BINS = [(1, 2), (3, 8), (9, 16), (17, 32), (33, 63), (64, 999)]
def peak(v):
    return 64 if v == 255 else v // 4

for case, ref_name in (("stationary", "stationary-off.png"), ("excluded", "excluded-off.png")):
    w, h, ref = load(new / ref_name)
    _, _, ref_old = load(old / ref_name)
    assert ref == ref_old, "off reference changed"
    mask = edges(ref, w, h)
    print(case, "edge pixels", sum(mask), "(off references byte-identical)")
    for label, d in (("original", old), ("corrected", new)):
        _, _, act = load(d / f"{case}-activity.png")
        counts = {b: [0, 0] for b in BINS}
        for i in range(w * h):
            p = peak(act[i * 4])
            for lo, hi in BINS:
                if lo <= p <= hi:
                    counts[(lo, hi)][mask[i]] += 1
        print("  %-9s" % label, "  ".join("%s-%s: int %d edge %d" % (lo, hi if hi < 999 else "", c[0], c[1]) for (lo, hi), c in counts.items()))
