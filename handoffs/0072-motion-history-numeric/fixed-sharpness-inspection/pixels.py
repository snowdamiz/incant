"""Reviewer inspection aid, CPU only: raw 8-bit sRGB values from PNGs, either a square
window of one channel (window) or one column of RGB triples (column), side by side
per file, to read edge profiles exactly.

  python3 -I -B pixels.py <visual/tools dir> window <x0> <y0> <size> <channel 0-2> <png> ...
  python3 -I -B pixels.py <visual/tools dir> column <x> <y0> <y1> <png> ...
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def name(path):
    parts = path.split("/")
    return "/".join(parts[-2:])


def main():
    mode = sys.argv[2]
    if mode == "window":
        x0, y0, size, channel = map(int, sys.argv[3:7])
        for path in sys.argv[7:]:
            im = s.Image(path)
            print(f"== {name(path)} (channel {'RGB'[channel]})")
            print("      " + " ".join(f"{x:4d}" for x in range(x0, x0 + size)))
            for y in range(y0, y0 + size):
                print(f"{y:4d}: " + " ".join(f"{im.rgba[(y * im.w + x) * 4 + channel]:4d}"
                                             for x in range(x0, x0 + size)))
    elif mode == "column":
        x, y0, y1 = map(int, sys.argv[3:6])
        images = [(name(p), s.Image(p)) for p in sys.argv[6:]]
        print(f"x = {x}")
        print("   y " + " | ".join(f"{n:>36s}" for n, _ in images))
        for y in range(y0, y1):
            print(f"{y:4d} " + " | ".join(
                f"{str(tuple(im.rgba[(y * im.w + x) * 4:(y * im.w + x) * 4 + 3])):>36s}" for _, im in images))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
