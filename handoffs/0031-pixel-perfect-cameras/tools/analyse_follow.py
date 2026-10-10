#!/usr/bin/env python3
"""Measures pixel_follow.py output from PNG bytes. Pure standard library.

Usage (repo root):
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse_follow.py RUN_DIR OUT_JSON \
      --rule {top-left,away-from-zero} [--repeat OTHER_RUN_DIR]

The rule names how the binary under test documents half-pixel camera ties: the
corrected 72ede59 build rounds camera-right ties down and camera-up ties up
("top-left"); the foundation 04a39ac build rounds both away from zero. The rule
only groups frames into snapping cells and predicts the background shift. The
follower check never depends on it: a grid-aligned follower must occupy exactly
its expected logical pixels in every frame.

Expected follower rectangle for a W x H logical frame, follower corner (r, u) and
size (w, h) on the camera-plane grid: x from floor(W/2) + r, y from floor(H/2) - u - h.
OUT_JSON must not exist.
"""

import argparse
import json
import math
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from analyse import bands, read_png, replication, sha  # noqa: E402
from pixel_follow import path  # noqa: E402


def cell(rule, p):
    """Snapped camera-plane cell (right, up) for a position p in logical pixels."""
    r, u = p
    if rule == "top-left":
        return math.ceil(r - 0.5), math.floor(u + 0.5)
    away = lambda v: int(math.copysign(math.floor(abs(v) + 0.5), v))  # noqa: E731
    return away(r), away(u)


def expected_rect(resolution, follower):
    (fr, fu), (fw, fh) = follower["corner"], follower["size"]
    x0 = resolution[0] // 2 + fr
    y0 = resolution[1] // 2 - fu - fh
    return [x0, x0 + fw, y0, y0 + fh]


def locate(img, colour, rect, pad=8):
    """Bounding box and pixel count of `colour` within `rect` grown by `pad`."""
    w, h, rows = img
    xs, ys = [], []
    for y in range(max(0, rect[2] - pad), min(h, rect[3] + pad)):
        row = rows[y]
        for x in range(max(0, rect[0] - pad), min(w, rect[1] + pad)):
            if row[x * 4:x * 4 + 4] == colour:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None, 0
    return [min(xs), max(xs) + 1, min(ys), max(ys) + 1], len(xs)


def count_colour(img, colour):
    return sum(row[i:i + 4] == colour for row in img[2] for i in range(0, len(row), 4))


def shift_mismatch(a, b, dx, dy, holes, margin=4):
    """Pixels where b[x][y] != a[x - dx][y - dy], outside `holes` (rects in either frame's coordinates)."""
    w, h, ar = a
    _, _, br = b
    bad = compared = 0
    for y in range(margin, h - margin):
        cuts = sorted([(r[0], r[1]) for r in holes if r[2] <= y < r[3]] +
                      [(r[0] + dx, r[1] + dx) for r in holes if r[2] <= y - dy < r[3]])
        segments, start = [], margin
        for c0, c1 in cuts:
            if c0 > start:
                segments.append((start, min(c0, w - margin)))
            start = max(start, c1)
        if start < w - margin:
            segments.append((start, w - margin))
        for x0, x1 in segments:
            compared += x1 - x0
            if br[y][x0 * 4:x1 * 4] != ar[y - dy][(x0 - dx) * 4:(x1 - dx) * 4]:
                bad += sum(br[y][x * 4:x * 4 + 4] != ar[y - dy][(x - dx) * 4:(x - dx) * 4 + 4] for x in range(x0, x1))
    return bad, compared


def grow(rect, n):
    return [rect[0] - n, rect[1] + n, rect[2] - n, rect[3] + n]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--rule", required=True, choices=["top-left", "away-from-zero"])
    parser.add_argument("--repeat", type=Path)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit("Output exists; choose a new file.")
    scene = json.loads((args.run / "scene.json").read_text())
    helper = json.loads((args.run / "helper.json").read_text())
    followers = scene["followers"]
    report = {"run": str(args.run), "binary": scene["binary"], "binary_sha256": helper["binary_sha256_start"],
              "rule": args.rule, "authored_unchanged": helper["authored_unchanged"], "configs": {}}
    logical_frames = {}
    for key, config in scene["configs"].items():
        w, h = config["resolution"]
        folder = args.run / f"follow-{key}-{w}x{h}"
        frames = sorted(folder.glob("frame-*.png"))
        imgs = {int(p.stem.split("-")[1]): read_png(p) for p in frames}
        logical_frames[key] = imgs
        f0 = imgs[0]
        rects = {k: expected_rect((w, h), f) for k, f in followers.items()}
        colours = {}
        for k, r in rects.items():
            colours[k] = f0[2][(r[2] + r[3]) // 2][((r[0] + r[1]) // 2) * 4:((r[0] + r[1]) // 2) * 4 + 4]
        entry = {"resolution": [w, h], "roll": config["roll"], "frames": len(imgs), "expected": rects,
                 "colour": {k: list(c) for k, c in colours.items()},
                 "colour_unique_in_frame0": {k: count_colour(f0, colours[k]) == followers[k]["size"][0] * followers[k]["size"][1]
                                             for k in followers}}
        follower_stats = {k: {"exact": 0, "offsets": Counter(), "deviations": []} for k in followers}
        cells, per_tick = {}, []
        holes = [grow(r, 2) for r in rects.values()]
        shift_exact, shift_bad = 0, []
        for tick, img in sorted(imgs.items()):
            e = path(tick)
            p = (e[0] / 8, e[1] / 8)
            c = cell(args.rule, p)
            cells.setdefault(c, []).append(tick)
            row = {"tick": tick, "camera_px": p, "cell": c}
            for k, r in rects.items():
                box, n = locate(img, colours[k], r)
                exact = box == r and n == (r[1] - r[0]) * (r[3] - r[2])
                stats = follower_stats[k]
                stats["exact"] += exact
                offset = "missing" if box is None else f"{box[0] - r[0]},{box[2] - r[2]}" + ("" if n == (box[1] - box[0]) * (box[3] - box[2]) else " partial")
                stats["offsets"][offset] += 1
                if not exact:
                    stats["deviations"].append({"tick": tick, "camera_px": p, "box": box, "count": n})
                row[k] = box
            dx, dy = -c[0], c[1]  # camera right +1 -> content left; camera up +1 -> content down
            bad, compared = shift_mismatch(f0, img, dx, dy, holes)
            row["background_mismatch"] = bad
            if bad == 0:
                shift_exact += 1
            else:
                shift_bad.append([tick, bad])
            per_tick.append(row)
        groups = {}
        for c, ticks in cells.items():
            first = imgs[ticks[0]][2]
            differing = [t for t in ticks[1:] if imgs[t][2] != first]
            groups[f"{c[0]},{c[1]}"] = {"ticks": ticks, "identical": not differing, "differing_ticks": differing}
        entry["followers"] = {k: {"exact_frames": v["exact"], "offsets": dict(v["offsets"]),
                                  "distinct_positions": len(v["offsets"]), "deviations": v["deviations"]}
                              for k, v in follower_stats.items()}
        entry["cells"] = {"count": len(groups), "identical": sum(g["identical"] for g in groups.values()),
                          "groups": groups}
        entry["background_shift"] = {"exact_frames": shift_exact, "frames": len(imgs), "mismatched": shift_bad,
                                     "max_mismatch": max((b for _, b in shift_bad), default=0)}
        entry["ticks"] = per_tick
        report["configs"][key] = entry

    # Odd-spare scaled capture: each frame must replicate the logical frame of the same tick.
    scaled = scene["scaled"]
    sw, sh = scaled["size"]
    folder = args.run / f"follow-{scaled['config']}-{sw}x{sh}"
    out = []
    for p in sorted(folder.glob("frame-*.png")):
        tick = int(p.stem.split("-")[1])
        img = read_png(p)
        r = replication(img, logical_frames[scaled["config"]][tick])
        out.append({"tick": tick, "replicates": r["mismatched_inside"] == 0 and r["mismatched_letterbox"] == 0,
                    "scale": r["scale"], "x0": r["x0"], "y0": r["y0"], "right": r["right"], "bottom": r["bottom"],
                    "bands": bands(img)})
    report["scaled"] = {"config": scaled["config"], "size": scaled["size"], "frames": len(out),
                        "replicating": sum(f["replicates"] for f in out), "detail": out}

    summary = {"follower_frames": 0, "follower_exact": 0, "cells": 0, "cells_identical": 0,
               "background_frames": 0, "background_exact": 0}
    for entry in report["configs"].values():
        for f in entry["followers"].values():
            summary["follower_frames"] += entry["frames"]
            summary["follower_exact"] += f["exact_frames"]
        summary["cells"] += entry["cells"]["count"]
        summary["cells_identical"] += entry["cells"]["identical"]
        summary["background_frames"] += entry["background_shift"]["frames"]
        summary["background_exact"] += entry["background_shift"]["exact_frames"]
    summary["scaled_replicating"] = f"{report['scaled']['replicating']}/{report['scaled']['frames']}"
    report["summary"] = summary

    if args.repeat:
        other = json.loads((args.repeat / "helper.json").read_text())
        mine = helper["plays"]
        compared = equal = 0
        different = []
        for capture, frames in mine.items():
            for name, digest in frames["frames"].items():
                compared += 1
                theirs = other["plays"].get(capture, {}).get("frames", {}).get(name)
                if theirs == digest:
                    equal += 1
                else:
                    different.append(f"{capture}/{name}")
        report["repeat"] = {"other": str(args.repeat), "compared": compared, "equal": equal, "different": different,
                            "other_binary_sha256": other["binary_sha256_start"]}
    args.out.write_text(json.dumps(report, indent=1, default=list), encoding="utf-8")
    print(json.dumps({"summary": summary, "repeat": {k: v for k, v in report.get("repeat", {}).items() if k != "different"}}))
    for key, entry in report["configs"].items():
        print(key, {k: (v["exact_frames"], v["offsets"]) for k, v in entry["followers"].items()},
              "cells", f"{entry['cells']['identical']}/{entry['cells']['count']}",
              "bg", f"{entry['background_shift']['exact_frames']}/{entry['background_shift']['frames']}",
              "max", entry["background_shift"]["max_mismatch"])


if __name__ == "__main__":
    main()
