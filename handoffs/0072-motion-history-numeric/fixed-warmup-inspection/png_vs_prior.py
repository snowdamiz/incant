"""Reviewer inspection aid, CPU only: for each published PNG of one run, the
max-channel step against the same-named PNG of another run, as a histogram
(identical, 1-2, 3-8, >8 steps) and the largest step.

  python3 -I -B png_vs_prior.py <visual/tools dir> <run screenshots> <other screenshots>
"""
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def main():
    run, other = Path(sys.argv[2]), Path(sys.argv[3])
    for path in sorted(run.glob("*.png")):
        a, b = s.Image(path), s.Image(other / path.name)
        hist, worst = [0, 0, 0, 0], 0
        for i in range(0, len(a.rgba), 4):
            v = s.step2(a.rgba, b.rgba, i)
            worst = max(worst, v)
            hist[0 if v == 0 else 1 if v <= 2 else 2 if v <= 8 else 3] += 1
        print(f"{path.name:28s} identical {hist[0]:7d} | 1-2 {hist[1]:6d} | 3-8 {hist[2]:6d} | >8 {hist[3]:6d} | max {worst}")


if __name__ == "__main__":
    main()
