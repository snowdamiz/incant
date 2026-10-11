"""Pixel analysis of unedited handoff 0040 engine frames (standard library only).

compare(a, b): byte identity, differing pixel count, max channel delta, and the
bounding box of differences. region_mean(): mean RGB over a pixel rectangle.
Derived figures are written only by explicit functions and are labelled as derived.
"""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import pngio  # noqa: E402


def load(path):
    w, h, c, px = pngio.read(path)
    return w, h, c, px


def compare(a, b, threshold=0):
    da, db = Path(a).read_bytes(), Path(b).read_bytes()
    wa, ha, ca, pa = pngio.read(da)
    wb, hb, cb, pb = pngio.read(db)
    if (wa, ha) != (wb, hb):
        raise ValueError("size mismatch")
    diff, max_delta, box = 0, 0, None
    for i in range(wa * ha):
        d = max(abs(pa[i * ca + k] - pb[i * cb + k]) for k in range(3))
        if d > threshold:
            diff += 1
            x, y = i % wa, i // wa
            box = [x, y, x, y] if box is None else [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
        max_delta = max(max_delta, d)
    return {"bytes_identical": hashlib.sha256(da).digest() == hashlib.sha256(db).digest(), "pixels": wa * ha,
            "differing_pixels": diff, "max_channel_delta": max_delta, "bbox": box}


def region_mean(path_or_img, x0, y0, x1, y1):
    w, h, c, px = path_or_img if isinstance(path_or_img, tuple) else load(path_or_img)
    total = [0, 0, 0]
    n = 0
    for y in range(max(0, y0), min(h, y1)):
        for x in range(max(0, x0), min(w, x1)):
            for k in range(3):
                total[k] += px[(y * w + x) * c + k]
            n += 1
    return [round(t / n, 2) for t in total] if n else None


def luminance(rgb):
    return round(0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2], 2)


def crop_compare(path, box_a, box_b, threshold=0):
    """Compare two equal-size rectangles inside one frame (for exact pixel offsets)."""
    w, h, c, px = load(path)
    (ax, ay, bw, bh), (bx, by) = box_a, box_b
    diff, max_delta = 0, 0
    for y in range(bh):
        for x in range(bw):
            i, j = ((ay + y) * w + ax + x) * c, ((by + y) * w + bx + x) * c
            d = max(abs(px[i + k] - px[j + k]) for k in range(3))
            max_delta = max(max_delta, d)
            if d > threshold:
                diff += 1
    return {"pixels": bw * bh, "differing_pixels": diff, "max_channel_delta": max_delta}


def diff_figure(a, b, out, gain=8):
    """DERIVED figure: |a-b| amplified, red where any channel differs. Not an engine frame."""
    wa, ha, ca, pa = load(a)
    _, _, cb, pb = load(b)
    pixels = bytearray()
    for i in range(wa * ha):
        d = max(abs(pa[i * ca + k] - pb[i * cb + k]) for k in range(3))
        base = int(0.25 * (pa[i * ca] + pa[i * ca + 1] + pa[i * ca + 2]) / 3)
        pixels += bytes((min(255, base + d * gain), base if d == 0 else 0, base if d == 0 else 0)) if d else bytes((base, base, base))
    pngio.write(out, wa, ha, pixels, 3)


if __name__ == "__main__":
    print(json.dumps(compare(sys.argv[1], sys.argv[2]), indent=1))
