"""Reviewer inspection for handoff 0076, CPU only, standard library. It reads the
run's PNGs and reports and reuses the frozen tools (pngread, shadowfit, regions,
pairs) unchanged. Text goes to stdout. Crops are viewing aids written into a new
scratch directory, never committed.

  python3 -I -B inspect.py <tools dir> totals <baseline report> <candidate report>
  python3 -I -B inspect.py <tools dir> interior <on.png> <off.png> [count]
  python3 -I -B inspect.py <tools dir> clusters <a.png> <b.png> <threshold> [count]
  python3 -I -B inspect.py <tools dir> pixels <x> <y> <radius> <png>...
  python3 -I -B inspect.py <tools dir> columns <x0> <x1> <y0> <y1> <png>...
  python3 -I -B inspect.py <tools dir> stack <new out.png> <x0,y0,x1,y1> <scale> <png>...
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import pairs  # noqa: E402
import pngread  # noqa: E402
import shadowfit  # noqa: E402

VIEWPORT = (37, 19, 885, 501)


def totals(a, b):
    """Consecutive-frame totals independent of each run's edge mask: interior plus
    edge sums of max-channel deltas, changed counts and maxima."""
    ra, rb = (json.loads(Path(p).read_text()) for p in (a, b))
    for key in ("still_consecutive", "pan_consecutive"):
        for x, y in zip(ra["observations"][key], rb["observations"][key]):
            row = []
            for d in (x["delta"], y["delta"]):
                i, e = d["interior"], d["edge"]
                row.append((round(i["mean_delta"] * i["pixels"] + e["mean_delta"] * e["pixels"]),
                            i["changed_over_threshold"] + e["changed_over_threshold"],
                            max(i["max_delta"], e["max_delta"])))
            print(f'{key} {x["from"]}->{x["to"]}: sum {row[0][0]} vs {row[1][0]} (diff {row[0][0] - row[1][0]}), '
                  f'changed>2 {row[0][1]} vs {row[1][1]}, max {row[0][2]} vs {row[1][2]}')
    fa = {f["name"]: f["rgba_sha256"] for f in ra["frames"]}
    fb = {f["name"]: f["rgba_sha256"] for f in rb["frames"]}
    print("frames differing:", [n for n in fa if fa[n] != fb.get(n)])


def interior(on_path, off_path, count=12):
    """Largest on/off max-channel differences at pixels outside the fixture's edge
    mask of the off reference (step > 24, 3x3 dilated), as report.json counts them."""
    on, off = shadowfit.Image(on_path), shadowfit.Image(off_path)
    edge = shadowfit.edge_mask(off)
    x0, y0, w, h = VIEWPORT
    found = []
    for y in range(y0, y0 + h):
        for x in range(x0, x0 + w):
            i = y * on.w + x
            if not edge[i]:
                found.append((shadowfit.step2(on.rgba, off.rgba, i * 4), x, y))
    found.sort(reverse=True)
    for v, x, y in found[:count]:
        i = (y * on.w + x) * 4
        print(f"({x}, {y}) delta {v}: on {tuple(on.rgba[i:i + 3])} off {tuple(off.rgba[i:i + 3])}")


def clusters(a_path, b_path, threshold, count=20):
    a, b = shadowfit.Image(a_path), shadowfit.Image(b_path)
    result = shadowfit.clusters(a, b, threshold, VIEWPORT, top=count)
    print(json.dumps({k: v for k, v in result.items() if k != "largest"}))
    for c in result["largest"]:
        print(c)


def pixels(x, y, radius, paths):
    images = [("/".join(Path(p).parts[-2:]), pngread.read(p)) for p in paths]
    for yy in range(y - radius, y + radius + 1):
        for label, (w, _, d) in images:
            row = " ".join(f"{d[(yy * w + xx) * 4]:3d},{d[(yy * w + xx) * 4 + 1]:3d},{d[(yy * w + xx) * 4 + 2]:3d}"
                           for xx in range(x - radius, x + radius + 1))
            print(f"y={yy} {label}: {row}")


def columns(x0, x1, y0, y1, paths):
    """Mean linear luminance per 8-pixel column band inside a box, per image."""
    for path in paths:
        img = shadowfit.Image(path)
        bands = []
        for bx in range(x0, x1, 8):
            values = [img.lum[y * img.w + x] for y in range(y0, y1) for x in range(bx, min(bx + 8, x1))]
            bands.append(f"{sum(values) / len(values):.3f}")
        print("/".join(Path(path).parts[-2:]), " ".join(bands))


def stack(out, box, scale, paths):
    """Vertical stack of magnified crops, one per image, magenta separators."""
    x0, y0, x1, y1 = (int(v) for v in box.split(","))
    rows = []
    for index, path in enumerate(paths):
        if index:
            rows.extend([pairs.SEPARATOR * ((x1 - x0) * scale)] * 4)
        rows.extend(pairs.crop(pngread.read(path), (x0, y0, x1, y1), scale))
    pairs.write_png(out, (x1 - x0) * scale, len(rows), b"".join(rows))
    print(out, (x1 - x0) * scale, "x", len(rows))


if __name__ == "__main__":
    command, args = sys.argv[2], sys.argv[3:]
    if command == "totals":
        totals(*args)
    elif command == "interior":
        interior(args[0], args[1], *(int(v) for v in args[2:]))
    elif command == "clusters":
        clusters(args[0], args[1], int(args[2]), *(int(v) for v in args[3:]))
    elif command == "pixels":
        pixels(int(args[0]), int(args[1]), int(args[2]), args[3:])
    elif command == "columns":
        columns(int(args[0]), int(args[1]), int(args[2]), int(args[3]), args[4:])
    elif command == "stack":
        stack(args[0], args[1], int(args[2]), args[3:])
    else:
        raise SystemExit(__doc__)
