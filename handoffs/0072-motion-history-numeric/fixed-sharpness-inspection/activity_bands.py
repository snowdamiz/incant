"""Reviewer inspection aid, CPU only: the full activity distribution behind the
comparison's stability counts (visual/tools/sharpness.py activity_bins with the
fixture's dilated edge mask of the temporal-off reference): pixels per peak band
(frames 9-16 consecutive change) as [interior, edge], per run.

  python3 -I -B activity_bands.py <visual/tools dir> <off.png> label=<activity.png> ...
"""
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import sharpness as s  # noqa: E402


def main():
    mask = s.edge_mask(s.Image(sys.argv[2]))
    for spec in sys.argv[3:]:
        label, path = spec.split("=", 1)
        bins = s.activity_bins(s.Image(path), mask)
        bands = "  ".join(f"{k}: {v[0]}/{v[1]}" for k, v in bins["edge_interior"].items())
        print(f"{label:9s} [interior/edge] {bands} | >=33 {bins['ge33']} | saturated {bins['saturated']}")


if __name__ == "__main__":
    main()
