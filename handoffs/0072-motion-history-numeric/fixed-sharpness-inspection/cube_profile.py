"""Reviewer inspection aid, CPU only: the moving cube's orangeness (R-B, on minus
temporal-off) in successive 1 px bands outside its trailing (left) edge and its top
edge (the no-motion control), using the comparison's own edge sampling
(visual/tools/sharpness.py cube_bands), plus the mean inside the first interior band.
The cube moves right at about 21 px/frame; the left edge trails.

  python3 -I -B cube_profile.py <visual/tools dir> <off.png> label=<on.png> ...
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def main():
    off = s.Image(sys.argv[2])
    w = off.w

    def rb(d, x, y):
        return d[(y * w + x) * 4] - d[(y * w + x) * 4 + 2]
    left, top = [], []
    for y in range(190, 240):
        xs = [x for x in range(400, 540) if rb(off.rgba, x, y) > 60]
        if xs and xs[-1] - xs[0] > 30:
            left.append((xs[0], y))
    for x in range(445, 490):
        ys = [y for y in range(150, 230) if rb(off.rgba, x, y) > 60]
        if ys:
            top.append((x, ys[0]))
    print(f"trailing-edge rows {len(left)} (x of first orange pixel: {sorted(set(p[0] for p in left))}); "
          f"top-edge columns {len(top)}")
    for spec in sys.argv[3:]:
        label, path = spec.split("=", 1)
        on = s.Image(path)

        def band(points, dx, dy):
            return sum(rb(on.rgba, x + dx, y + dy) - rb(off.rgba, x + dx, y + dy) for x, y in points) / len(points)
        trail = " ".join(f"{band(left, -k, 0):7.2f}" for k in (3, 2, 1))
        inside_t = band(left, 0, 0)
        topb = " ".join(f"{band(top, 0, -k):7.2f}" for k in (3, 2, 1))
        inside_p = band(top, 0, 0)
        print(f"  {label:9s} trailing outside -3,-2,-1: {trail} | first inside {inside_t:7.2f} || "
              f"top outside -3,-2,-1: {topb} | first inside {inside_p:7.2f}")


if __name__ == "__main__":
    main()
