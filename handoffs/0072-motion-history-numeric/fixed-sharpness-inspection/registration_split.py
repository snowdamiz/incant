"""Reviewer inspection aid, CPU only: where the skinned case's registration shift comes
from. Uses the comparison's own point rule (visual/tools/sharpness.py
registration_linear: linear-light luminance x255, central-gradient squared > 100 in the
temporal-off reference) and the same 6-iteration Lucas-Kanade solve, but on subsets:
all points, points inside a box, and points outside it; then a per-cluster solve for
each 9x9 neighbourhood of points. A positive dx means the temporal-on content sits to
the right of the reference.

  python3 -I -B registration_split.py <visual/tools dir> <on.png> <off.png> x0,y0,x1,y1
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def solve(a, b, w, h, pts):
    def sample(x, y):
        x = min(max(x, 0.0), w - 1.001)
        y = min(max(y, 0.0), h - 1.001)
        x0, y0 = int(x), int(y)
        fx, fy = x - x0, y - y0
        i = y0 * w + x0
        return b[i] * (1 - fx) * (1 - fy) + b[i + 1] * fx * (1 - fy) + b[i + w] * (1 - fx) * fy + b[i + w + 1] * fx * fy
    v = [0.0, 0.0]
    for _ in range(6):
        g11 = g12 = g22 = e1 = e2 = 0.0
        for x, y in pts:
            r = sample(x - v[0], y - v[1])
            gx = (sample(x + 1 - v[0], y - v[1]) - sample(x - 1 - v[0], y - v[1])) / 2
            gy = (sample(x - v[0], y + 1 - v[1]) - sample(x - v[0], y - 1 - v[1])) / 2
            t = a[y * w + x] - r
            g11 += gx * gx
            g12 += gx * gy
            g22 += gy * gy
            e1 += gx * t
            e2 += gy * t
        det = g11 * g22 - g12 * g12
        if abs(det) < 1e-9:
            return None
        dx = -(g22 * e1 - g12 * e2) / det
        dy = -(g11 * e2 - g12 * e1) / det
        v[0] += dx
        v[1] += dy
        if abs(dx) + abs(dy) < 1e-4:
            break
    return round(v[0], 3), round(v[1], 3)


def main():
    on, off = s.Image(sys.argv[2]), s.Image(sys.argv[3])
    x0, y0, x1, y1 = map(int, sys.argv[4].split(","))
    w, h = off.w, off.h
    a = [v * 255 for v in on.lum]
    b = [v * 255 for v in off.lum]
    pts = [(x, y) for y in range(2, h - 2) for x in range(2, w - 2)
           if ((b[y * w + x + 1] - b[y * w + x - 1]) / 2) ** 2 + ((b[(y + 1) * w + x] - b[(y - 1) * w + x]) / 2) ** 2 > 100.0]
    inside = [p for p in pts if x0 <= p[0] < x1 and y0 <= p[1] < y1]
    outside = [p for p in pts if not (x0 <= p[0] < x1 and y0 <= p[1] < y1)]
    print(f"all {len(pts)} points: {solve(a, b, w, h, pts)}")
    print(f"inside box {sys.argv[4]} ({len(inside)} points): {solve(a, b, w, h, inside) if inside else None}")
    print(f"outside box ({len(outside)} points): {solve(a, b, w, h, outside) if outside else None}")
    clusters = {}
    for x, y in pts:
        clusters.setdefault((x // 9, y // 9), []).append((x, y))
    print("per 9x9 cluster (x0,y0: points, dx, dy, max on-off step there):")
    for (cx, cy), group in sorted(clusters.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        worst = max(s.step2(on.rgba, off.rgba, (y * w + x) * 4) for x, y in group)
        print(f"  {cx * 9},{cy * 9}: {len(group)}, {solve(a, b, w, h, group)}, {worst}")


if __name__ == "__main__":
    main()
