#!/usr/bin/env python3
"""Measures pixel_neartie.py output: follower offsets from their intended pixels beside each tie.

Usage (repo root):
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse_neartie.py RUN_DIR OUT_JSON [--repeat OTHER_RUN_DIR]

For every configuration, series (right+, right-, up+) and delta, reports each
follower's offset (dx, dy) in logical pixels from the rectangle a grid-aligned
follower must occupy (see analyse_follow.py), plus the snapped camera cell the
background shows. "0,0" for both followers is correct. OUT_JSON must not exist.
"""

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from analyse import read_png  # noqa: E402
from analyse_follow import expected_rect, locate, shift_mismatch, grow  # noqa: E402


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--repeat", type=Path)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit("Output exists; choose a new file.")
    scene = json.loads((args.run / "scene.json").read_text())
    helper = json.loads((args.run / "helper.json").read_text())
    plan = scene["positions"]
    report = {"run": str(args.run), "binary": scene["binary"], "binary_sha256": helper["binary_sha256_start"],
              "authored_unchanged": helper["authored_unchanged"], "configs": {}}
    for key, config in scene["configs"].items():
        w, h = config["resolution"]
        folder = args.run / f"neartie-{key}-{w}x{h}"
        f0 = read_png(folder / "frame-000000.png")
        rects = {k: expected_rect((w, h), f) for k, f in scene["followers"].items()}
        colours = {k: f0[2][(r[2] + r[3]) // 2][((r[0] + r[1]) // 2) * 4:((r[0] + r[1]) // 2) * 4 + 4]
                   for k, r in rects.items()}
        holes = [grow(r, 2) for r in rects.values()]
        rows, wrong = [], []
        for n, step in enumerate(plan, start=1):
            img = read_png(folder / f"frame-{n:06d}.png")
            offsets = {}
            for k, r in rects.items():
                box, count = locate(img, colours[k], r)
                offsets[k] = None if box is None else [box[0] - r[0], box[2] - r[2]] + (
                    [] if count == (r[1] - r[0]) * (r[3] - r[2]) else ["partial"])
            # Which whole-pixel camera cell the background shows, relative to frame 0 (cell 0, 0).
            cell = None
            for cr in (-1, 0, 1):
                for cu in (-1, 0, 1):
                    if shift_mismatch(f0, img, -cr, cu, holes)[0] == 0:
                        cell = [cr, cu]
            row = {"frame": n, "series": step["series"], "delta": step["delta"], "camera_px": [step["right"], step["up"]],
                   "background_cell": cell, "a": offsets["a"], "b": offsets["b"]}
            rows.append(row)
            if offsets["a"] != [0, 0] or offsets["b"] != [0, 0]:
                wrong.append({k: row[k] for k in ("series", "delta", "background_cell", "a", "b")})
        report["configs"][key] = {"resolution": [w, h], "roll": config["roll"], "frames": len(rows),
                                  "followers_correct": len(rows) - len(wrong), "displaced": wrong, "rows": rows}
    if args.repeat:
        other = json.loads((args.repeat / "helper.json").read_text())
        pairs = [(c, n, d) for c, v in helper["plays"].items() for n, d in v["frames"].items()]
        same = sum(other["plays"].get(c, {}).get("frames", {}).get(n) == d for c, n, d in pairs)
        report["repeat"] = {"other": str(args.repeat), "compared": len(pairs), "equal": same}
    args.out.write_text(json.dumps(report, indent=1), encoding="utf-8")
    for key, entry in report["configs"].items():
        print(key, f"followers correct {entry['followers_correct']}/{entry['frames']}")
        for wrong in entry["displaced"]:
            print("   ", wrong)
    if args.repeat:
        print("repeat", report["repeat"])


if __name__ == "__main__":
    main()
