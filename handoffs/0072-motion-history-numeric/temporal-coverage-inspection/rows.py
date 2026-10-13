"""Reviewer inspection, CPU only: raw sRGB pixel rows for a short x range, per run.

  python3 -I -B rows.py <on.png>,<off.png> <x0>:<x1>:<y,y,...> <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[3:])
name_on, name_off = sys.argv[1].split(",")
x0, x1, ys = sys.argv[2].split(":")[0], sys.argv[2].split(":")[1], sys.argv[2].split(":")[2]
x0, x1 = int(x0), int(x1)
for y in map(int, ys.split(",")):
    print("row", y, "x", x0, "..", x1 - 1, "(sRGB R,G,B)")
    for label, d in runs.items():
        w, h, rows = pngio.read(str(find(d, name_off if label == "off" else name_on)))
        r = rows[y]
        print("  %-9s" % label, " ".join("%3d,%3d,%3d" % (r[x*4], r[x*4+1], r[x*4+2]) for x in range(x0, x1)))
