"""Reviewer inspection aid, CPU only: inside one box, how two temporal-on runs differ
from the temporal-off reference and from each other (max-channel steps): the count
of pixels at 3-8, 9-16 and >16 steps from the reference per run, and the largest
run-to-run step. Not edge-masked; use with boxes chosen from off_edge_bins clusters.

  python3 -I -B region_compare.py <visual/tools dir> <off.png> <a.png> <b.png> x0,y0,x1,y1
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def main():
    off, a, b = (s.Image(p) for p in sys.argv[2:5])
    x0, y0, x1, y1 = map(int, sys.argv[5].split(","))
    w = off.w
    bins = {"a": [0, 0, 0], "b": [0, 0, 0]}
    between = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            i = (y * w + x) * 4
            for key, img in (("a", a), ("b", b)):
                v = s.step2(img.rgba, off.rgba, i)
                if 3 <= v <= 8:
                    bins[key][0] += 1
                elif 9 <= v <= 16:
                    bins[key][1] += 1
                elif v > 16:
                    bins[key][2] += 1
            between = max(between, s.step2(a.rgba, b.rgba, i))
    print(f"box {sys.argv[5]}: a vs off 3-8/9-16/>16 {bins['a']}; b vs off {bins['b']}; max a-b step {between}")


if __name__ == "__main__":
    main()
