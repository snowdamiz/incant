"""CPU-model AO views for the next-repair design choice (not renders, not screenshots).

Each panel is the CPU model's final R8 accessibility plane (screen.py, the same
f64 model the 001 device matched to within one code per texel) shown as
darkening (1 - A) x 6 over the receiver's occluded region, upscaled x 3:
white is more occlusion. Fixtures: stripes perspective (D | C2 | S16) with
the N8 ROI row marked red at its left and right margins, white_slab and
edge_slab perspective (D | C2). Uses ../evidence/appearance-review/pngtool.py
read only. Writes create-new PNGs and a manifest with their hashes.

  /opt/homebrew/bin/python3 -I -B model_views.py
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "evidence" / "appearance-review")]
import pngtool  # noqa: E402
import screen  # noqa: E402

m, qr = screen.m, screen.qr
GAIN, SCALE, GAP = 6, 3, 6


def darkening(grid, values, box, mark_row=None):
    x0, y0, x1, y1 = box
    out = bytearray()
    for y in range(y0, y1):
        for _ in range(SCALE):
            for x in range(x0, x1):
                v = values[y * grid.w + x]
                g = 0 if v is None else min(255, int(round((1 - v) * 255 * GAIN)))
                px = (255, 0, 0, 255) if mark_row == y and (x - x0 < 2 or x1 - x <= 2) else (g, g, g, 255)
                out += bytes(px) * SCALE
    return ((x1 - x0) * SCALE, (y1 - y0) * SCALE), bytes(out)


def side(panels):
    """Equal-size RGBA panels left to right with white gaps."""
    (w, h), out = panels[0][0], bytearray()
    for y in range(h):
        for i, (_, data) in enumerate(panels):
            if i:
                out += bytes((255, 255, 255, 255)) * GAP
            out += data[y * w * 4:(y + 1) * w * 4]
    return (len(panels) * w + (len(panels) - 1) * GAP, h), bytes(out)


def occluded_box(grid, values_list, pad=6):
    xs, ys = [], []
    for values in values_list:
        for i, v in enumerate(values):
            if v is not None and v < 1 and grid.hits[i] is not None and grid.hits[i][3] == 0:
                y, x = divmod(i, grid.w)
                xs.append(x)
                ys.append(y)
    return (max(min(xs) - pad, 0), max(min(ys) - pad, 0), min(max(xs) + pad + 1, grid.w), min(max(ys) + pad + 1, grid.h))


def main():
    views = {}
    stripes = m.Grid(qr.STRIPES, False)
    row = qr.n8_roi(stripes)[0]
    values = {c: screen.full(stripes, c) for c in ("D", "C2", "S16")}
    box = occluded_box(stripes, values.values())
    views["stripes_persp_D_C2_S16.png"] = (
        f"stripes perspective, darkening x{GAIN}, x{SCALE}: D (R1/R3) | C2 | S16; red ticks: N8 ROI row {row}",
        side([darkening(stripes, values[c], box, row) for c in ("D", "C2", "S16")]))
    for name in ("white_slab", "edge_slab"):
        grid = m.Grid(name, False)
        values = {c: screen.full(grid, c) for c in ("D", "C2")}
        box = occluded_box(grid, values.values())
        views[f"{name}_persp_D_C2.png"] = (f"{name} perspective, darkening x{GAIN}, x{SCALE}: D (R1/R3) | C2",
                                           side([darkening(grid, values[c], box) for c in ("D", "C2")]))
    manifest = {"format": "incant-0070-next-repair-model-views-v1",
                "note": "CPU model planes (screen.py), not renders or screenshots; no acceptance evidence.",
                "tool": {"model_views.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}, "outputs": {}}
    for name, (description, (size, data)) in views.items():
        path = HERE / name
        if path.exists():
            raise SystemExit(f"{path} exists; views are never replaced")
        pngtool.encode(path, size, data)
        manifest["outputs"][name] = {"description": description, "size": list(size),
                                     "png_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    with open(HERE / "model-views-manifest.json", "x") as out:
        json.dump(manifest, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({k: v["size"] for k, v in manifest["outputs"].items()}, indent=1))


if __name__ == "__main__":
    main()
