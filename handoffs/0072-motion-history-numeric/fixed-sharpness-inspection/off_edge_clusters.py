"""Reviewer inspection aid, CPU only: where the comparison's off-edge differences sit.
Same rule as visual/tools/sharpness.py off_edge_bins (max-channel step between the
temporal-on and temporal-off PNGs at pixels at least 3 px, 8-connected, from any
neighbour step above 12 in either image), restricted to steps >= a threshold and
grouped into 24x24 cells (largest cells first) with the single worst pixel.

  python3 -I -B off_edge_clusters.py <visual/tools dir> <on.png> <off.png> <min step>
"""
import sys
from collections import deque

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def main():
    on, off = s.Image(sys.argv[2]), s.Image(sys.argv[3])
    threshold = int(sys.argv[4])
    w, h = on.w, on.h
    edge = bytearray(w * h)
    for img in (on, off):
        for i, v in enumerate(s.edge_mask(img, 12, dilate=False)):
            if v:
                edge[i] = 1
    far = 1 << 30
    dist = [far] * (w * h)
    queue = deque(i for i, v in enumerate(edge) if v)
    for i in queue:
        dist[i] = 0
    while queue:
        i = queue.popleft()
        y, x = divmod(i, w)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and dist[ny * w + nx] > dist[i] + 1:
                    dist[ny * w + nx] = dist[i] + 1
                    queue.append(ny * w + nx)
    points = []
    for i in range(w * h):
        if dist[i] >= 3:
            v = s.step2(on.rgba, off.rgba, i * 4)
            if v >= threshold:
                points.append((i % w, i // w, v))
    print(f"off-edge pixels >= {threshold} steps: {len(points)}")
    cells = {}
    for x, y, v in points:
        cells.setdefault((x // 24, y // 24), []).append((x, y, v))
    for (cx, cy), items in sorted(cells.items(), key=lambda kv: -len(kv[1]))[:12]:
        xs, ys = [p[0] for p in items], [p[1] for p in items]
        print(f"  cell x{cx * 24}-{cx * 24 + 23} y{cy * 24}-{cy * 24 + 23}: {len(items)} px, "
              f"bbox ({min(xs)},{min(ys)})-({max(xs)},{max(ys)}), max {max(p[2] for p in items)}")
    if points:
        x, y, v = max(points, key=lambda p: p[2])
        i = (y * w + x) * 4
        print(f"worst ({x}, {y}): on {tuple(on.rgba[i:i + 3])} off {tuple(off.rgba[i:i + 3])}")


if __name__ == "__main__":
    main()
