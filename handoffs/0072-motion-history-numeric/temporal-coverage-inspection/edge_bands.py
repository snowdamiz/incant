"""Reviewer inspection, CPU only: spread of the moving cube's edges in motion-object f8.
Per row (or column) the cube edge is found in the temporal-off frame (orange: R-B > 60).
Reports on-minus-off orangeness (R-B, sRGB steps) at 1, 2 and 3 px outside the trailing
(left; the cube moves right), leading (right, rows clear of the sphere) and top (perpendicular
to the motion, a no-motion control) edges, plus the edge pixel itself. Needs labels off,
original, corrected, coverage.

  python3 -I -B edge_bands.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find

pngio, runs = setup(sys.argv[1:])


def image(directory, name):
    w, _, rows = pngio.read(str(find(directory, name)))
    return w, b"".join(rows)


w, off = image(runs["off"], "motion-object-off-f8.png")


def rb(data, x, y):
    return data[(y * w + x) * 4] - data[(y * w + x) * 4 + 2]


left, right, top = [], [], []
for y in range(190, 240):
    xs = [x for x in range(400, 540) if rb(off, x, y) > 60]
    if xs and xs[-1] - xs[0] > 30:
        left.append((xs[0], y))
        if xs[-1] >= 495:
            right.append((xs[-1], y))
for x in range(445, 490):
    ys = [y for y in range(150, 230) if rb(off, x, y) > 60]
    if ys:
        top.append((x, ys[0]))
cube = sum(rb(off, x + 3, y) for x, y in left) / len(left)
wall = sum(rb(off, x - 6, y) for x, y in left) / len(left)
print("samples: trailing %d, leading %d, top %d; reference R-B cube %.1f, wall %.1f (contrast %.1f)"
      % (len(left), len(right), len(top), cube, wall, cube - wall))
for label, directory in runs.items():
    if label == "off":
        continue
    _, on = image(directory, "motion-object-on-f8.png")

    def band(points, dx, dy):
        means = []
        for k in (1, 2, 3):
            values = [rb(on, x + dx * k, y + dy * k) - rb(off, x + dx * k, y + dy * k) for x, y in points]
            means.append("%+5.1f" % (sum(values) / len(values)))
        edge = sum(rb(on, x, y) - rb(off, x, y) for x, y in points) / len(points)
        return "/".join(means) + " (edge px %+5.1f)" % edge

    print("%-9s trailing 1/2/3 px %s | leading %s | top %s"
          % (label, band(left, -1, 0), band(right, 1, 0), band(top, 0, -1)))
