"""Reviewer inspection aid, CPU only: where the stationary stability and halo failures
sit, per run. Uses the comparison's own definitions (visual/tools/sharpness.py):
activity map red channel = peak consecutive change x4 over frames 9-16 (255 = 64+
steps), the fixture's dilated reference edge mask (threshold 24), and halo = any
channel above the temporal-off 3x3 maximum by more than 2 steps. Lists every
saturated activity pixel and the 24x24 cells holding the most activity >= 33 and
halo pixels.

  python3 -I -B localize.py <visual/tools dir> <off.png> label=<on.png>,<activity.png> ...
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def cells(points, top=8):
    grid = {}
    for x, y, v in points:
        grid.setdefault((x // 24, y // 24), []).append((x, y, v))
    rows = []
    for (cx, cy), items in sorted(grid.items(), key=lambda kv: -len(kv[1]))[:top]:
        xs, ys = [p[0] for p in items], [p[1] for p in items]
        rows.append(f"    cell {cx * 24},{cy * 24}: {len(items)} px, bbox ({min(xs)},{min(ys)})-({max(xs)},{max(ys)}), "
                    f"max {max(p[2] for p in items)}")
    return rows


def main():
    off = s.Image(sys.argv[2])
    w, h = off.w, off.h
    mask = s.edge_mask(off)
    hi = [s.channel_extreme_3x3(off, c, max) for c in range(3)]
    for spec in sys.argv[3:]:
        label, files = spec.split("=", 1)
        on_path, activity_path = files.split(",")
        on, act = s.Image(on_path), s.Image(activity_path)
        saturated, strong, halo = [], [], []
        for i in range(w * h):
            v = act.rgba[i * 4]
            peak = 64 if v == 255 else v // 4
            x, y = i % w, i // w
            if peak >= 64:
                saturated.append((x, y, "edge" if mask[i] else "interior"))
            if peak >= 33:
                strong.append((x, y, peak))
            excess = max(on.rgba[i * 4 + c] - hi[c][i] for c in range(3))
            if excess > 2:
                halo.append((x, y, excess))
        print(f"== {label}")
        print(f"  saturated (64+ steps) {len(saturated)}: {saturated}")
        print(f"  activity >= 33: {len(strong)}")
        print("\n".join(cells(strong)))
        print(f"  halo above local max by > 2 steps: {len(halo)}")
        print("\n".join(cells(halo)))


if __name__ == "__main__":
    main()
