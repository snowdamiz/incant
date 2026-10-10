#!/usr/bin/env python3
"""Measures sprite_lookdev.py output from PNG bytes. Standard library only.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/analyse.py RUN_DIR OUT_JSON [OTHER_RUN_DIR]

OUT_JSON must not exist. Every engine frame (static, reimport, source-free and all
live frames) is compared pixel by pixel with reference.py's prediction. Relational
checks (3x replication, odd-size phase, snapping, flips, nine-slice mirrors) use
engine pixels only. With OTHER_RUN_DIR, every PNG is also compared byte for byte
with a second from-scratch run.

The nearest-sampling expectation follows the run's binary, recorded by the helper:
the corrected build uses the documented tie-band rule, the pre-ties build (and
round-one runs, which predate the field) the hardware rule with ambiguity flags.
Rulers on the follow-up boards are also decoded from engine pixels alone.
"""

import hashlib
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402
import reference  # noqa: E402

BG = (24, 22, 40)
TOL_NEAREST, TOL_LINEAR = 1, 2
# Nearest rule per tested build source. Round-one runs predate the field (pre-ties).
RULES = {"39e5c38107451ba44a02ef7b80f27bef11d4141d": "hardware",
         "c2c7758c14c37d36aa13e322aa2c87e63ce489ad": "tie-band",
         "f9aa1da0818313c5c6cac580e54d7107646378bc": "pixel-band"}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Run:
    def __init__(self, path):
        self.path = Path(path)
        self.scene = json.loads((self.path / "scene.json").read_text(encoding="utf-8"))
        self.helper = json.loads((self.path / "helper.json").read_text(encoding="utf-8"))
        art = self.path / "art"
        t = self.scene["textures"]
        self.tex = {name: reference.Texture(art / f) for name, f in (
            ("sheet", "sheet.png"), ("teal", "sheet-teal.png"), ("tall", "sheet-tall.png"),
            ("labels", "labels.png"), ("badge", "badge.png"), ("orange", "badge-orange.png"),
            ("tone16", "tone16.png"), ("ruler64", "ruler64.png"), ("strip", "strip4096.png"),
            ("strip8192", "strip8192.png"))
            if (art / f).exists()}
        self.ids = self.scene["ids"]
        commit = self.helper.get("binary_source_commit")
        if commit is None:
            self.rule = "hardware"
        elif commit in RULES:
            self.rule = RULES[commit]
        else:
            raise SystemExit(f"no documented nearest rule for build {commit}; add it to RULES")
        occ = self.scene["occluder"]
        self.meshes = [(self.ids["C:occluder"], occ["positions"], occ["emissive"])]
        self.asset = t

    def textures(self, sheet="sheet", badge="badge"):
        t = self.asset
        out = {t["sheet"]: self.tex[sheet], t["labels"]: self.tex["labels"], t["badge"]: self.tex[badge],
               t["tone16"]: self.tex["tone16"]}
        for name in ("ruler64", "strip", "strip8192"):
            if name in t:
                out[t[name]] = self.tex[name]
        return out

    def render(self, camera, w, h, sheet="sheet", badge="badge", overrides=None):
        return reference.render(self.scene["entities"], self.textures(sheet, badge), self.scene["cameras"][camera]["id"]
                                if camera in self.scene["cameras"] else camera, w, h, self.meshes, overrides)


def compare(png, predicted):
    """Engine PNG vs reference prediction. Returns stats and the decoded image."""
    rows, frame, view = predicted
    img = pngio.read(png)
    assert (img.width, img.height) == (len(rows[0]), len(rows)), (png, img.width, img.height)
    s = 1 if view.output is None else view.output[2]
    ox = 0 if view.output is None else view.output[3]
    oy = 0 if view.output is None else view.output[4]
    hist = Counter()
    stats = {"pixels": img.width * img.height, "ambiguous": 0, "ambiguous_mismatch": 0, "unexplained": 0,
             "linear_pixels": 0, "examples": []}
    for y in range(img.height):
        ly = (y - oy) // s
        row = img.rows[y]
        for x in range(img.width):
            e = (row[4 * x], row[4 * x + 1], row[4 * x + 2])
            r = rows[y][x]
            d = max(abs(e[0] - r[0]), abs(e[1] - r[1]), abs(e[2] - r[2]))
            hist[d] += 1
            lx = (x - ox) // s
            i = ly * frame.w + lx if 0 <= lx < frame.w and 0 <= ly < frame.h else None
            amb = i is not None and frame.ambiguous[i]
            lin = i is not None and frame.linear[i]
            stats["ambiguous"] += amb
            stats["linear_pixels"] += lin
            if d > (TOL_LINEAR if lin else TOL_NEAREST):
                if amb:
                    stats["ambiguous_mismatch"] += 1
                else:
                    stats["unexplained"] += 1
                    if len(stats["examples"]) < 8:
                        stats["examples"].append({"x": x, "y": y, "engine": e, "reference": r,
                                                  "owner": frame.owner[i] if i is not None else None})
    stats["exact"] = hist[0]
    stats["max_diff"] = max(hist)
    stats["diff_histogram"] = {str(k): v for k, v in sorted(hist.items())}
    return stats, img


def ambiguous_owners(predicted, img):
    rows, frame, view = predicted
    s = 1 if view.output is None else view.output[2]
    ox = 0 if view.output is None else view.output[3]
    oy = 0 if view.output is None else view.output[4]
    owners, mism = Counter(), Counter()
    for i in range(frame.w * frame.h):
        if not frame.ambiguous[i]:
            continue
        lx, ly = i % frame.w, i // frame.w
        x, y = ox + lx * s, oy + ly * s
        owners[frame.owner[i]] += 1
        e, r = img.rgb(x, y), rows[y][x]
        if max(abs(a - b) for a, b in zip(e, r)) > (TOL_LINEAR if frame.linear[i] else TOL_NEAREST):
            mism[frame.owner[i]] += 1
    return {"pixels_by_owner": dict(owners.most_common()), "mismatches_by_owner": dict(mism.most_common())}


def replicate_mismatches(big, small, scale):
    bad = 0
    for y in range(big.height):
        for x in range(big.width):
            if big.px(x, y) != small.px(x // scale, y // scale):
                bad += 1
    return bad


def block(img, x0, y0, w, h):
    return [[img.px(x0 + i, y0 + j) for i in range(w)] for j in range(h)]


def mirror_h(b):
    return [list(reversed(r)) for r in b]


def mirror_v(b):
    return list(reversed(b))


def rot_ccw(b):
    """Counter-clockwise quarter turn on screen: out[row][col] = b[col][n - 1 - row]."""
    n = len(b)
    return [[b[c][n - 1 - r] for c in range(n)] for r in range(n)]


def ruler_texels(img, x0, y0, x1, y1):
    """Decode the 7 x 7 ruler: pixel colour (60 + 25 i, 60 + 25 j, 150) names texel (i, j)."""
    found = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            r, g, b = img.rgb(x, y)
            if b == 150 and (r - 60) % 25 == 0 and (g - 60) % 25 == 0 and 60 <= r <= 210 and 60 <= g <= 210:
                found[(x, y)] = ((r - 60) // 25, (g - 60) // 25)
    if not found:
        return None
    xs = sorted({x for x, _ in found})
    ys = sorted({y for _, y in found})
    mid_y, mid_x = ys[len(ys) // 2], xs[len(xs) // 2]
    columns = [found.get((x, mid_y), (None,))[0] for x in xs]
    rows = [found.get((mid_x, y), (None, None))[1] for y in ys]
    return {"pixels": len(found), "x": [xs[0], xs[-1]], "y": [ys[0], ys[-1]],
            "columns_left_to_right": columns, "rows_top_to_bottom": rows,
            "each_texel_once": sorted(columns) == list(range(7)) and sorted(rows) == list(range(7))}


# Screen block (u, v) -> source texel (i, j) for each board F orientation.
D4_MAP = {
    "none": lambda u, v: (u, v), "flip-x": lambda u, v: (6 - u, v), "flip-y": lambda u, v: (u, 6 - v),
    "both": lambda u, v: (6 - u, 6 - v), "rot90": lambda u, v: (6 - v, u), "rot270": lambda u, v: (v, 6 - u),
    "rot90-flip-x": lambda u, v: (v, u), "rot90-flip-y": lambda u, v: (6 - v, 6 - u),
    "rot180": lambda u, v: (6 - u, 6 - v), "mirror": lambda u, v: (6 - u, v),
}


def decode_window(img, x0, y0, x1, y1):
    """Ruler pixels in a window: (x, y) -> source texel (i, j)."""
    found = {}
    for y in range(max(0, y0), min(img.height, y1)):
        for x in range(max(0, x0), min(img.width, x1)):
            r, g, b = img.rgb(x, y)
            if b == 150 and (r - 60) % 25 == 0 and (g - 60) % 25 == 0 and 60 <= r <= 210 and 60 <= g <= 210:
                found[(x, y)] = ((r - 60) // 25, (g - 60) // 25)
    return found


def runs(values):
    out = []
    for v in values:
        if out and out[-1][0] == v:
            out[-1][1] += 1
        else:
            out.append([v, 1])
    return out


def board_f(img, scene):
    """Every orientation on tie and aligned placements: each pixel must name the texel
    the orientation maps to, and every texel must cover exactly M x M pixels."""
    out = {}
    rows = [("TIES 1X", 1, 0.0, 24, 16), ("ALIGNED 1X", 1, 0.5, 54, 16),
            ("TIES 3X", 3, 0.0, 100, 30), ("ALIGNED 3X", 3, 0.5, 150, 30)]
    for caption, m, phase, centre_y, pitch in rows:
        for k, name in enumerate(scene["d4"]):
            cx, cy = 20 + pitch * k + phase, centre_y + phase
            half = 3.5 * m
            found = decode_window(img, int(cx - half) - 2, int(cy - half) - 2, int(cx + half) + 3, int(cy + half) + 3)
            xs = [x for x, _ in found]
            ys = [y for _, y in found]
            bx, by = min(xs), min(ys)
            size = (max(xs) - bx + 1, max(ys) - by + 1)
            wrong = sum(1 for (x, y), texel in found.items()
                        if texel != D4_MAP[name]((x - bx) // m, (y - by) // m))
            counts = Counter(found.values())
            out[f"{caption} {k} {name}"] = {
                "box": [bx, by, size[0], size[1]], "pixels": len(found), "wrong_texel_pixels": wrong,
                "each_texel_m_squared": len(counts) == 49 and set(counts.values()) == {m * m},
                "pass": size == (7 * m, 7 * m) and len(found) == 49 * m * m and wrong == 0}
    return out


def board_g(img, scene):
    """Offset sweep: per cell, texel runs along the middle row and column."""
    out = {}
    deltas = scene["sweep"]["deltas_px"]
    for key, meta in scene["meta"].items():
        if not key.startswith("G:") or "cell" not in meta:
            continue
        m, flipped, (x, y) = meta["magnification"], meta["flipped"], meta["cell"]
        found = decode_window(img, x - 2, y - 2, x + 7 * m + 3, y + 7 * m + 3)
        xs = sorted({px for px, _ in found})
        ys = sorted({py for _, py in found})
        mid_y, mid_x = ys[len(ys) // 2], xs[len(xs) // 2]
        columns = runs([found[(px, mid_y)][0] for px in xs if (px, mid_y) in found])
        rows_ = runs([found[(mid_x, py)][1] for py in ys if (mid_x, py) in found])
        order = list(range(6, -1, -1)) if flipped else list(range(7))
        uniform = ([t for t, _ in columns] == order and [t for t, _ in rows_] == order
                   and all(n == m for _, n in columns + rows_))
        out[key] = {"magnification": m, "flipped": flipped, "delta_px": meta["delta_px"],
                    "delta_steps_of_1_256": round(meta["delta_px"] * 256, 4),
                    "column_widths": [n for _, n in columns], "row_heights": [n for _, n in rows_],
                    "column_texels": [t for t, _ in columns], "row_texels": [t for t, _ in rows_],
                    "uniform": uniform, "first_x": xs[0] - x, "first_y": ys[0] - y}
    return out


def decode_generic(img, x0, y0, x1, y1, blue):
    """Self-naming texels with the given blue channel: (R, G) = (60 + 2 i, 60 + 2 j)."""
    found = {}
    for y in range(max(0, y0), min(img.height, y1)):
        for x in range(max(0, x0), min(img.width, x1)):
            r, g, b = img.rgb(x, y)
            if b == blue and (r - 60) % 2 == 0 and (g - 60) % 2 == 0 and 60 <= r <= 186 and 60 <= g <= 186:
                found[(x, y)] = ((r - 60) // 2, (g - 60) // 2)
    return found


def tuple_runs(found):
    """Runs of decoded texels along the middle row and middle column."""
    xs = sorted({x for x, _ in found})
    ys = sorted({y for _, y in found})
    mid_y, mid_x = ys[len(ys) // 2], xs[len(xs) // 2]
    return (runs([found[(x, mid_y)] for x in xs if (x, mid_y) in found]),
            runs([found[(mid_x, y)] for y in ys if (mid_x, y) in found]))


def board_h(img, scene):
    """Round-three falsification cases on board H, decoded from engine pixels."""
    out = {}
    for key, meta in scene["meta"].items():
        if not key.startswith("H:") or "kind" not in meta:
            continue
        kind, (x, y) = meta["kind"], meta["cell"]
        entry = {"kind": kind, "delta_px": meta["delta_px"], "delta_steps_of_1_256": round(meta["delta_px"] * 256, 4)}
        if kind == "high":
            m = meta["magnification"]
            found = decode_window(img, x - 2, y, x + 7 * m + 3, y + m)
            cols, _ = tuple_runs(found)
            order = list(range(6, -1, -1)) if meta["flipped"] else list(range(7))
            widths = [n for _, n in cols]
            entry.update(magnification=m, flipped=meta["flipped"], widths=widths,
                         uniform=[t[0] for t, _ in cols] == order and widths == [m] * 7)
        elif kind in ("turn", "aniso"):
            if kind == "turn":
                mx = my = meta["magnification"]
                found = decode_window(img, x - 1, y - 1, x + 59, y + 59)
                entry.update(magnification=mx, angle=meta["angle"])
            else:
                mx, my = meta["magnification"]
                found = decode_window(img, x - 1, y - 1, x + 7 * mx + 3, y + 7 * my + 3)
                entry.update(magnification=[mx, my], flipped=meta["flipped"])
            cols, rows_ = tuple_runs(found)
            # A quarter turn exchanges screen axes, so each axis must hold seven
            # distinct texels of equal width (the axis magnification).
            col_w, row_w = [n for _, n in cols], [n for _, n in rows_]
            entry.update(widths=col_w, heights=row_w,
                         uniform=(len({t for t, _ in cols}) == 7 == len(cols) and len({t for t, _ in rows_}) == 7
                                  == len(rows_) and col_w == [mx] * 7 and row_w == [my] * 7))
        elif kind == "minify":
            k, size = meta["texels_per_pixel"], meta["size_px"]
            found = decode_generic(img, x - 2, y - 2, x + size + 3, y + size + 3, 120)
            xs = sorted({px for px, _ in found})
            ys = sorted({py for _, py in found})
            mid_y, mid_x = ys[len(ys) // 2], xs[len(xs) // 2]
            row = [found[(px, mid_y)][0] for px in xs if (px, mid_y) in found]
            col = [found[(mid_x, py)][1] for py in ys if (mid_x, py) in found]
            step = -k if meta["flipped"] else k
            entry.update(texels_per_pixel=k, flipped=meta["flipped"], placement=meta["placement"],
                         columns=row, rows=col, covered=[len(xs), len(ys)],
                         uniform=(len(xs) == size and len(ys) == size
                                  and all(b - a == step for a, b in zip(row, row[1:]))
                                  and all(b - a == step for a, b in zip(col, col[1:]))))
        out[key] = entry
    # Mirror consistency for minification: is a flipped sprite the mirror image of
    # the unflipped one at the same placement? Ties pick one side, so it may not be.
    for key, entry in out.items():
        if entry["kind"] == "minify" and ":plain:" in key:
            twin = out.get(key.replace(":plain:", ":flip:"))
            if twin:
                entry["flipped_twin_is_mirror"] = (twin["columns"] == entry["columns"][::-1]
                                                   and twin["rows"] == entry["rows"][::-1])
    return out


def board_i(img, scene):
    """4096-texel strips near t = 4096: every complete texel run must span M pixels."""
    out = {}
    for key, meta in scene["meta"].items():
        if not key.startswith("I:") or meta.get("kind") != "wide":
            continue
        m, y, flipped = meta["magnification"], meta["row_y"], meta["flipped"]
        found = decode_generic(img, 0, y + m // 2, img.width, y + m // 2 + 1, 90)
        xs = sorted(px for px, _ in found)
        seq = [found[(px, y + m // 2)][0] + 64 * found[(px, y + m // 2)][1] for px in xs]
        rr = runs(seq)
        # Drop the run cut by the frame edge: the left one for plain strips, the
        # right one for flipped strips. The strip's own end is a complete texel.
        partial = rr[0] if not flipped else rr[-1]
        complete = rr[1:] if not flipped else rr[:-1]
        steps = [b[0] - a[0] for a, b in zip(complete, complete[1:])]
        expected_step = -1 if flipped else 1
        bad = [(t, n) for t, n in complete if n != m]
        out[key] = {"magnification": m, "flipped": flipped, "delta_px": meta["delta_px"],
                    "delta_steps_of_1_256": round(meta["delta_px"] * 256, 4),
                    "complete_texels": len(complete), "texel_range": [min(seq), max(seq)],
                    "partial_edge_run": partial, "width_counts": dict(Counter(n for _, n in complete)),
                    "consecutive": all(st == expected_step for st in steps), "off_width_texels": bad[:12],
                    "uniform": not bad and all(st == expected_step for st in steps)}
    return out


def decode_strip_row(img, y, x0, x1):
    """Strip texels on one row: i = R + 64 G + 4096 B in the strip encodings."""
    out = {}
    for x in range(max(0, x0), min(img.width, x1)):
        r, g, b = img.rgb(x, y)
        if b in (90, 92) and (r - 60) % 2 == 0 and (g - 60) % 2 == 0 and 60 <= r <= 186 and 60 <= g <= 186:
            out[x] = (r - 60) // 2 + 64 * ((g - 60) // 2) + 4096 * ((b - 90) // 2)
    return out


def board_j(img, scene):
    """Exact ties across region widths: complete texel runs must span M pixels."""
    out = {}
    for key, meta in scene["meta"].items():
        if not key.startswith("J:") or meta.get("kind") != "map":
            continue
        m, y, flipped = meta["magnification"], meta["row_y"], meta["flipped"]
        x0, x1 = (820, img.width) if flipped else (0, 781)
        found = decode_strip_row(img, y + m // 2, x0, x1)
        xs = sorted(found)
        rr = runs([found[x] for x in xs])
        if rr and not flipped and xs[0] == 0:
            rr = rr[1:]
        if rr and flipped and xs[-1] == img.width - 1:
            rr = rr[:-1]
        steps = [b[0] - a[0] for a, b in zip(rr, rr[1:])]
        bad = [(t, n) for t, n in rr if n != m]
        ok = bool(rr) and not bad and all(st == (-1 if flipped else 1) for st in steps)
        out[key] = {"width": meta["width"], "magnification": m, "flipped": flipped, "complete_texels": len(rr),
                    "texel_range": [min(found.values()), max(found.values())] if found else None,
                    "width_counts": dict(Counter(n for _, n in rr)), "off_width_texels": bad[:12], "uniform": ok}
    return out


def odd_station(img):
    out = {}
    for k, name in enumerate(("normal", "flip_x", "flip_y", "both")):
        cx = 228 + 12 * k
        out[f"grid-point pivot {name}"] = ruler_texels(img, cx - 5, 54, cx + 5, 66)
        out[f"pixel-centre pivot {name}"] = ruler_texels(img, cx - 5, 68, cx + 6, 79)
    out["grid-point pivot rotation 180"] = ruler_texels(img, 271, 54, 282, 66)
    out["grid-point pivot parent scale -1"] = ruler_texels(img, 283, 54, 294, 66)
    return out


def eq(a, b):
    return sum(1 for ra, rb in zip(a, b) for pa, pb in zip(ra, rb) if pa != pb)


def srgb8_over(src, a, dst):
    return tuple(round(s * a + d * (1 - a)) for s, d in zip(src, dst))


def main():
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    run = Run(sys.argv[1])
    out_json = Path(sys.argv[2])
    if out_json.exists():
        raise SystemExit("OUT_JSON exists; choose a new path.")
    other = Path(sys.argv[3]) if len(sys.argv) == 4 else None
    t0 = time.time()
    reference.NEAREST_RULE = run.rule
    result = {"run": run.path.name, "nearest_rule": run.rule,
              "binary": {k: run.helper.get(k) for k in ("binary", "binary_file", "binary_source_commit")},
              "helper_summary": {
        k: run.helper[k] for k in ("binary_sha256_start", "binary_sha256_end", "binary_unchanged_every_call",
                                   "engine_calls", "authored_unchanged_by_rendering", "authored_unchanged_by_play")}}
    shots_dir = run.path / "shots"
    helper_shots = run.helper["shots"]
    first = next(iter(helper_shots.values()))
    result["adapter"] = first.get("adapter")

    # 1. Every static frame against the reference.
    static, images, predictions = {}, {}, {}
    for key, entry in helper_shots.items():
        name, size = key.rsplit("-", 1)
        w, h = map(int, size.split("x"))
        pred = run.render(name, w, h)
        stats, img = compare(shots_dir / f"{key}-r1.png", pred)
        stats["repeat_equal"] = entry["equal"]
        stats["geometry"] = entry.get("geometry")
        if stats["ambiguous_mismatch"]:
            stats["ambiguous_detail"] = ambiguous_owners(pred, img)
        static[key] = stats
        images[key] = img
        predictions[key] = pred
    result["static"] = static

    # 2. Pixel-perfect 960x540 is an exact 3x replication of the 320x180 frame.
    result["pp_3x_replication_mismatches"] = {
        b: replicate_mismatches(images[f"pp-{b}-960x540"], images[f"pp-{b}-320x180"], 3) for b in "ABCDE"}

    # 3. Odd logical size and sub-pixel camera offset, from engine pixels only.
    base, odd, sub = images["pp-A-320x180"], images["pp-A-odd-319x179"], images["pp-A-sub-320x180"]

    def shifted(img, w, h, dx, dy):
        diffs = [max(abs(a - b) for a, b in zip(img.rgb(x, y), base.rgb(x + dx, y + dy)))
                 for y in range(h) for x in range(w)]
        return {"compared": len(diffs), "identical": diffs.count(0), "max_level_difference": max(diffs),
                "beyond_one_level": sum(1 for d in diffs if d > 1)}
    result["odd_319x179_vs_base_shifted_by_1_1"] = shifted(odd, 319, 179, 1, 1)
    result["camera_plus_0.6_0.2_px_vs_base_shifted_left_1"] = shifted(sub, 319, 180, 1, 0)

    # 4. Ordinary 3x orthographic vs pixel-perfect 3x: where they differ, and why.
    ortho_vs_pp = {}
    for b in "ABCDE":
        a, p = images[f"ortho3-{b}-960x540"], images[f"pp-{b}-960x540"]
        _, frame, _ = predictions[f"ortho3-{b}-960x540"]
        owners = Counter()
        for y in range(540):
            for x in range(960):
                if a.px(x, y) != p.px(x, y):
                    owners[frame.owner[y * 960 + x]] += 1
        ortho_vs_pp[b] = {"differing_pixels": sum(owners.values()), "by_top_sprite": dict(owners.most_common(12))}
    result["ortho3_vs_pixel_perfect_3x"] = ortho_vs_pp

    # 5. Orientation: flips, mirror parent and rotations on board A (2x wisp, 32 px).
    A = images["pp-A-320x180"]
    boxes = {n: block(A, 8 + 40 * k, 100, 32, 32) for k, n in
             enumerate(["normal", "flip-x", "flip-y", "both", "scale-1", "rot90", "rot180"])}
    n = boxes["normal"]
    result["orientation_mismatched_pixels"] = {
        "flip_x == mirror": eq(boxes["flip-x"], mirror_h(n)),
        "flip_y == vertical mirror": eq(boxes["flip-y"], mirror_v(n)),
        "both == 180 degrees": eq(boxes["both"], mirror_v(mirror_h(n))),
        "scale -1 parent == flip_x": eq(boxes["scale-1"], boxes["flip-x"]),
        "rotation 180 == both flips": eq(boxes["rot180"], boxes["both"]),
        "rotation 90 == counter-clockwise turn": eq(boxes["rot90"], rot_ccw(n)),
        "normal != flip_x (asymmetric art)": eq(n, boxes["flip-x"]) > 0,
    }
    result["ruler_station"] = odd_station(A)
    if "pp-F-320x180" in images:
        result["board_f"] = board_f(images["pp-F-320x180"], run.scene)
    if "pp-G-1024x640" in images:
        result["board_g"] = board_g(images["pp-G-1024x640"], run.scene)
    if "pp-H-1280x640" in images:
        result["board_h"] = board_h(images["pp-H-1280x640"], run.scene)
    if "pp-I-1024x640" in images:
        result["board_i"] = board_i(images["pp-I-1024x640"], run.scene)
    if "pp-J-1024x640" in images:
        result["board_j"] = board_j(images["pp-J-1024x640"], run.scene)
    if run.rule == "tie-band" and "pp-G-1024x640" in images and "pp-F-320x180" in images:
        # Model-only counterfactual for the recommendation, not engine evidence: the
        # corrected rule with the band scaled by the signed derivative (1/1024 px).
        reference.NEAREST_RULE = "pixel-band"
        modelled = {}
        for key, decode_board in (("pp-F-320x180", board_f), ("pp-G-1024x640", board_g)):
            name, size = key.rsplit("-", 1)
            w, h = map(int, size.split("x"))
            rows = run.render(name, w, h)[0]
            img = pngio.Image(w, h, [bytes(c for p in row for c in (*p, 255)) for row in rows])
            modelled[key] = decode_board(img, run.scene)
        reference.NEAREST_RULE = run.rule
        result["counterfactual_pixel_band_model_only"] = {
            "board_f_pass": sum(v["pass"] for v in modelled["pp-F-320x180"].values()),
            "board_f_cases": len(modelled["pp-F-320x180"]),
            "board_g_nonuniform": sorted([v["magnification"], v["flipped"], v["delta_steps_of_1_256"]]
                                         for v in modelled["pp-G-1024x640"].values() if not v["uniform"]),
            "board_g_cases": len(modelled["pp-G-1024x640"])}
    B = images["pp-B-320x180"]
    panels = [block(B, 8 + 74 * k, 74, 64, 40) for k in range(4)]
    result["nine_slice_flip_mismatched_pixels"] = {
        "flip_x == mirror": eq(panels[1], mirror_h(panels[0])),
        "flip_y == vertical mirror": eq(panels[2], mirror_v(panels[0])),
        "both == 180 degrees": eq(panels[3], mirror_v(mirror_h(panels[0]))),
        "nine-slice 24x24 == simple 24x24": eq(block(B, 290, 14, 24, 24), block(B, 8, 14, 24, 24)),
    }

    # 6. Painter-order probes (board C logical pixels): top sprite by the validated reference.
    _, cframe, _ = predictions["pp-C-320x180"]
    C = images["pp-C-320x180"]
    probes = {
        "layer: all three overlap -> red (layer 2)": ((28, 34), "C:LAYER:red"),
        "layer: green+blue -> green (layer 1)": ((36, 42), "C:LAYER:green"),
        "order: all three -> red (order 5)": ((84, 34), "C:ORDER:red"),
        "order: green+blue -> blue (order 0 > -3)": ((92, 42), "C:ORDER:blue"),
        "id tie: all three -> blue (largest ID)": ((140, 34), "C:ID TIE:blue"),
        "id tie: red+green -> green": ((132, 26), "C:ID TIE:green"),
        "z ignored: far red (layer 1) over near green": ((190, 30), "C:Z:red-far"),
        "batch: badge (order 1) over key (order 0)": ((247, 29), "C:batch1"),
        "batch: key (order 2) over badge (order 1)": ((254, 36), "C:batch2"),
        "depth: opaque mesh hides sprite at z 0": ((260, 100), "C:occluder"),
        "depth: sprite at z 2 drawn over mesh": ((290, 110), "C:depth:front"),
        "depth: layer 100 sprite behind mesh is hidden": ((295, 90), "C:occluder"),
        "depth: layer 100 sprite visible above mesh": ((295, 83), "C:depth:layer100-behind"),
    }
    result["painter_probes"] = {k: {"pixel": list(p), "expected_top": want, "reference_top": cframe.owner[p[1] * 320 + p[0]],
                                    "engine_rgb": C.rgb(*p), "pass": cframe.owner[p[1] * 320 + p[0]] == want}
                                for k, (p, want) in probes.items()}

    # 7. Alpha: measured vs the linear-light contract vs paint-program (sRGB-space) blending.
    rgb = {"red": (214, 64, 72), "green": (72, 186, 112), "blue": (76, 120, 222)}
    alpha_rows = {}
    for label, (ox, oy), under in (("over board background", (8, 80), BG),
                                   ("over light plate", (76, 80), (200, 200, 200))):
        pts = {"red": (ox + 4, oy + 4), "red+green": (ox + 20, oy + 4), "red+green+blue": (ox + 22, oy + 20),
               "green+blue": (ox + 36, oy + 28), "blue": (ox + 22, oy + 40)}
        stack = {"red": ["red"], "red+green": ["red", "green"], "red+green+blue": ["red", "green", "blue"],
                 "green+blue": ["green", "blue"], "blue": ["blue"]}
        rows = {}
        for k, p in pts.items():
            paint = under
            for colour in stack[k]:
                paint = srgb8_over(rgb[colour], 0.5, paint)
            rows[k] = {"engine": C.rgb(*p), "contract": predictions["pp-C-320x180"][0][p[1]][p[0]],
                       "paint_program_srgb_blend": paint}
        alpha_rows[label] = rows
    pts = {"red": (180, 148), "red+green": (194, 148), "red+green+blue": (190, 160), "blue": (190, 172)}
    stack = {"red": ["red"], "red+green": ["red", "green"], "red+green+blue": ["red", "green", "blue"],
             "blue": ["blue"]}
    rows = {}
    for k, p in pts.items():
        paint = reference.BACKDROP_SRGB
        for colour in stack[k]:
            paint = srgb8_over(rgb[colour], 0.5, paint)
        rows[k] = {"engine": C.rgb(*p), "contract": predictions["pp-C-320x180"][0][p[1]][p[0]],
                   "paint_program_srgb_blend": paint}
    alpha_rows["over display backdrop (no opaque scene behind)"] = rows
    strips = {}
    for label, y in (("white over 128 gray", 84), ("black over 128 gray", 96), ("white over backdrop", 152),
                     ("black over backdrop", 166)):
        strips[label] = [C.rgb(152 + 8 * k + 4, y) if y < 140 else C.rgb(8 + 8 * k + 4, y) for k in range(9)]
    alpha_rows["alpha strips 0,32,...,255"] = strips
    result["alpha"] = alpha_rows

    # 8. Tone: authored sRGB -> displayed (8-bit source, 16-bit source, 0.5 linear tint).
    D = images["pp-D-320x180"]
    swatches = json.loads((run.path / "art/regions.json").read_text(encoding="utf-8"))["tone_swatches"]
    tone = []
    for k, authored in enumerate(swatches):
        x = 12 + 8 * k
        shown8, shown16, tinted = D.rgb(x, 142), D.rgb(x, 152), D.rgb(x, 162)
        peak = max(reference.decode_srgb(c / 255) for c in authored)
        tone.append({"authored": authored, "shown_8bit": shown8, "shown_16bit": shown16,
                     "shown_tint_0.5": tinted, "identity": tuple(authored) == shown8,
                     "peak_linear": round(peak, 4), "max_channel_shift": max(abs(a - b) for a, b in zip(authored, shown8))})
    result["tone"] = tone
    result["tone_8bit_equals_16bit"] = all(t["shown_8bit"] == t["shown_16bit"] for t in tone)
    # Full gray table from the 16 x 16 ramp at 2x on board C (texel (i, j) = 16 j + i).
    shown = {}
    for v in range(256):
        i, j = v % 16, v // 16
        px = C.rgb(232 + 2 * i, 146 + 2 * j)
        assert px[0] == px[1] == px[2], (v, px)
        shown[v] = px[0]
    changed = [v for v in range(256) if shown[v] != v]
    result["gray_ramp"] = {
        "shown": shown, "first_changed": changed[0] if changed else None,
        "identical_values": 256 - len(changed),
        "distinct_outputs_from_first_changed": len({shown[v] for v in changed}),
        "inputs_shown_as_243": [v for v in range(256) if shown[v] == 243]}

    # 9. Filtering fringe: a 4x linear orb whose zero-alpha texels hide magenta RGB.
    def lum(c):
        lin = [reference.decode_srgb(v / 255) for v in c]
        return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]

    plate, core = D.rgb(154, 12), D.rgb(188, 46)
    floor_lum = min(lum(plate), lum(core))
    dark = sum(1 for y in range(14, 78) for x in range(156, 220) if lum(D.rgb(x, y)) < floor_lum * 0.98)
    # Counterfactual: the same frame if straight (unpremultiplied) texels were filtered,
    # letting the hidden magenta under zero alpha into the edge samples.
    reference.STRAIGHT_ALPHA_FILTERING = True
    naive_rows, _, _ = run.render("pp-D", 320, 180)
    reference.STRAIGHT_ALPHA_FILTERING = False
    contract_rows = predictions["pp-D-320x180"][0]
    footprints = [(80, 14, 144, 78), (156, 14, 220, 78)]  # the two 4x linear orbs

    def shift(rows_a, get_b):
        tinted, worst = 0, 0
        for x0, y0, x1, y1 in footprints:
            for y in range(y0, y1):
                for x in range(x0, x1):
                    a, b = rows_a[y][x], get_b(x, y)
                    d = (a[2] - b[2]) - (a[1] - b[1])  # magenta direction: blue up, green down
                    worst = max(worst, d)
                    tinted += d >= 3
        return tinted, worst
    naive_tinted, naive_worst = shift(naive_rows, lambda x, y: contract_rows[y][x])
    engine_tinted, engine_worst = shift([[D.rgb(x, y) for x in range(320)] for y in range(180)],
                                        lambda x, y: contract_rows[y][x])
    result["linear_fringe"] = {
        "plate": plate, "orb_core": core,
        "engine_pixels_darker_than_plate_and_core": dark,
        "engine_pixels_shifted_toward_magenta_by_3_or_more": engine_tinted,
        "engine_worst_magenta_shift_levels": engine_worst,
        "counterfactual_straight_alpha_pixels_shifted_toward_magenta_by_3_or_more": naive_tinted,
        "counterfactual_worst_magenta_shift_levels": naive_worst}

    # 10. Bleed / hidden-RGB scan over every frame written by the run.
    def magenta_like(img):
        count = 0
        for row in img.rows:
            for i in range(0, len(row), 4):
                r, g, b = row[i], row[i + 1], row[i + 2]
                if min(r, b) - g > 110:
                    count += 1
        return count

    # 11. Cracks: background colour enclosed by tile or nine-slice pixels (reference ownership).
    def cracks(key):
        img = images[key]
        _, frame, view = predictions[key]
        if (img.width, img.height) != (frame.w, frame.h):
            return None
        w, h = frame.w, frame.h
        owner = frame.owner
        count = 0
        for y in range(1, h - 1):
            for x in range(1, w - 1):
                if img.rgb(x, y) != BG:
                    continue
                for a, b in (((x - 1, y), (x + 1, y)), ((x, y - 1), (x, y + 1))):
                    oa, ob = owner[a[1] * w + a[0]], owner[b[1] * w + b[0]]
                    if oa and ob and any(t in oa for t in TOKENS) and any(t in ob for t in TOKENS):
                        count += 1
                        break
        return count
    TOKENS = ("tiles", "nine", "flip", "density", "rot12", "linear8")
    result["cracks_enclosed_background_pixels"] = {
        k: cracks(k) for k in ("pp-B-320x180", "pp-D-320x180", "ortho3-B-960x540", "ortho3-D-960x540",
                               "orthof-B-960x540", "orthof-D-960x540", "roll7-B-960x540", "roll7-D-960x540")}

    # 12. Reimport sequence.
    re_dir = run.path / "reimport"
    steps = run.helper["reimport"]
    initial = {k: v["asset"] for k, v in run.scene["imports"].items()}
    re = {"steps": {}}
    for label, entry in steps.items():
        if label.endswith(":shots"):
            continue
        item = {"exit": entry["exit"], "authored_unchanged": entry["authored_unchanged"], "stderr": entry["stderr"]}
        if entry["exit"] == 0:
            tex_name = "sheet" if "sheet" in label else "badge"
            item.update(same_asset_id=entry["asset"]["id"] == initial[tex_name]["id"], sha256=entry["asset"]["sha256"],
                        changed=entry["changed"], width=entry["details"]["width"], height=entry["details"]["height"],
                        initial_sha256=initial[tex_name]["sha256"])
        re["steps"][label] = item
    sheet_variant = {"1-sheet-teal": ("teal", "badge"), "2-badge-orange": ("teal", "orange"),
                     "4-badge-restore": ("teal", "badge"), "6-sheet-restore": ("sheet", "badge"),
                     "7-sheet-tall": ("tall", "badge"), "8-sheet-restore": ("sheet", "badge")}
    shots_cmp = {}
    for label, entry in steps.items():
        if not label.endswith(":shots"):
            continue
        step = label[:-6]
        for key, shot in entry.items():
            item = {"exit": shot["exit"], "image_written": shot["image_written"], "stderr": shot["stderr"][-300:]}
            if shot["exit"] == 0:
                name, size = key.rsplit("-", 1)
                w, h = map(int, size.split("x"))
                sheet, badge = sheet_variant[step]
                stats, _ = compare(re_dir / f"{step}-{key}.png", run.render(name, w, h, sheet, badge))
                item["reference"] = {k: stats[k] for k in ("unexplained", "ambiguous_mismatch", "max_diff")}
                original = helper_shots.get(key, {}).get("r1")
                item["equals_original_static"] = shot["sha256"] == original
            shots_cmp[f"{step}:{key}"] = item
    re["shots"] = shots_cmp
    teal_diff = {}
    for b in "ABCDE":
        key = f"pp-{b}-320x180"
        img = pngio.read(re_dir / f"1-sheet-teal-{key}.png")
        _, frame, _ = predictions[key]
        owners = Counter()
        for y in range(180):
            for x in range(320):
                if img.px(x, y) != images[key].px(x, y):
                    owners[frame.owner[y * 320 + x]] += 1
        teal_diff[b] = dict(owners.most_common())
    re["teal_changed_pixels_by_top_sprite"] = teal_diff
    tall_diff = {}
    for key in steps.get("7-sheet-tall:shots", {}):
        if key not in images or not steps["7-sheet-tall:shots"][key]["image_written"]:
            continue
        img = pngio.read(re_dir / f"7-sheet-tall-{key}.png")
        _, frame, view = predictions[key]
        if view.output is not None and view.output[2] != 1:
            continue
        owners, unflagged, nearest, linear = Counter(), 0, 0, 0
        for y in range(img.height):
            for x in range(img.width):
                if img.px(x, y) != images[key].px(x, y):
                    i = y * frame.w + x
                    owners[frame.owner[i]] += 1
                    unflagged += not frame.ambiguous[i] and not frame.linear[i]
                    linear += frame.linear[i]
                    nearest += not frame.linear[i]
        tall_diff[key] = {"changed_pixels": sum(owners.values()), "changed_nearest_or_uncovered": nearest,
                          "changed_linear": linear, "by_top_sprite": dict(owners.most_common()),
                          "changed_outside_ambiguous_or_linear_pixels": unflagged}
    re["tall_sheet_changed_pixels"] = tall_diff
    re["ruler_station_after_tall_sheet"] = odd_station(pngio.read(re_dir / "7-sheet-tall-pp-A-320x180.png"))
    c1 = steps["1-sheet-teal:shots"]["pp-C-320x180"]["sha256"]
    re["badge_restore_equals_before_orange"] = steps["4-badge-restore:shots"]["pp-C-320x180"]["sha256"] == c1
    result["reimport"] = re

    # 13. Source-free frames equal the frames rendered with sources present.
    free = run.helper["source_free"]
    result["source_free"] = {"removed": free["removed"],
                             "equal_to_original": {k: v == helper_shots[k]["r1"] for k, v in free["shots"].items()}}

    # 14. Live flipbook + follow: every frame against the reference; find the command latency.
    live = run.scene["live"]
    steps_live = {}
    lat_found = None
    for key, (ux, uy) in live["directions"].items():
        frames_dir = run.path / f"live-{key}-r1"
        names = sorted(p.name for p in frames_dir.glob("frame-*.png"))
        regions = {k: run.scene["regions"][f"sheet/wisp{k}"] for k in range(4)}
        cam0 = next(e for e in run.scene["entities"] if e["id"] == live["id"])["components"]["Transform"]
        wisp = next(e for e in run.scene["entities"] if e["id"] == live["wisp"])
        w0 = wisp["components"]["Transform"]
        sprite0 = wisp["components"]["SpriteRenderer"]

        def overrides(applied):
            """State after `applied` scripted steps (0 = authored)."""
            if applied == 0:
                return None
            k = applied * live["step"]
            frame_index = (applied // 4) % 4
            cam = dict(cam0, translation=[cam0["translation"][0] + ux * k, uy * k, cam0["translation"][2]])
            wt = dict(w0, translation=[w0["translation"][0] + ux * k, w0["translation"][1] + uy * k,
                                       w0["translation"][2]])
            return {live["id"]: {"Transform": cam},
                    live["wisp"]: {"Transform": wt, "SpriteRenderer": dict(sprite0, region=regions[frame_index])}}

        if lat_found is None:
            for latency in (0, 1, 2):
                ok = True
                for f in (5, 13):
                    applied = max(0, f - latency)
                    stats, _ = compare(frames_dir / names[f], run.render(live["id"], 320, 180,
                                                                         overrides=overrides(applied)))
                    ok &= stats["unexplained"] == 0
                if ok:
                    lat_found = latency
                    break
        latency = lat_found if lat_found is not None else 1
        per = {"frames": len(names), "unexplained": 0, "ambiguous_mismatch": 0, "max_diff": 0, "wisp_boxes": Counter()}
        for f, name in enumerate(names):
            applied = max(0, min(f - latency, live["ticks"]))
            stats, img = compare(frames_dir / name, run.render(live["id"], 320, 180, overrides=overrides(applied)))
            per["unexplained"] += stats["unexplained"]
            per["ambiguous_mismatch"] += stats["ambiguous_mismatch"]
            per["max_diff"] = max(per["max_diff"], stats["max_diff"])
            xs, ys = [], []
            for y in range(90, 140):
                for x in range(80, 140):
                    p = img.rgb(x, y)
                    if p in ((54, 32, 132), (243, 243, 243)):  # wisp outline and eye whites
                        xs.append(x)
                        ys.append(y)
            per["wisp_boxes"][(min(xs), min(ys), max(xs), max(ys)) if xs else None] += 1
        per["repeat_equal"] = run.helper["plays"][key]["repeat_equal"]
        per["wisp_boxes"] = {str(k): v for k, v in per["wisp_boxes"].items()}
        steps_live[key] = per
    result["live"] = {"command_latency_frames": lat_found, "directions": steps_live}

    # 15. Bleed scan of every PNG in the run.
    all_pngs = sorted(p for p in run.path.rglob("*.png") if "art" not in p.parts and "project" not in p.parts)
    bleed = {}
    for p in all_pngs:
        c = magenta_like(pngio.read(p))
        if c:
            bleed[str(p.relative_to(run.path))] = c
    result["magenta_like_pixels"] = {"frames_scanned": len(all_pngs), "frames_with_hits": bleed}

    # 16. Cross-run byte equality.
    if other:
        mine = {str(p.relative_to(run.path)): sha(p) for p in all_pngs}
        theirs = {str(p.relative_to(other)): sha(p) for p in sorted(other.rglob("*.png"))
                  if "art" not in p.parts and "project" not in p.parts}
        result["cross_run"] = {"other": other.name, "compared": len(mine), "missing": sorted(set(mine) ^ set(theirs)),
                               "different": sorted(k for k in mine if k in theirs and mine[k] != theirs[k])}
        art_a = {p.name: sha(p) for p in sorted((run.path / "art").glob("*.png"))}
        art_b = {p.name: sha(p) for p in sorted((other / "art").glob("*.png"))}
        result["cross_run"]["art_identical"] = art_a == art_b

    totals = {"frames_vs_reference": len(static) + sum(1 for v in shots_cmp.values() if "reference" in v)
              + sum(v["frames"] for v in steps_live.values()),
              "unexplained": sum(v["unexplained"] for v in static.values())
              + sum(v["reference"]["unexplained"] for v in shots_cmp.values() if "reference" in v)
              + sum(v["unexplained"] for v in steps_live.values()),
              "ambiguous_mismatch": sum(v["ambiguous_mismatch"] for v in static.values())
              + sum(v["reference"]["ambiguous_mismatch"] for v in shots_cmp.values() if "reference" in v)
              + sum(v["ambiguous_mismatch"] for v in steps_live.values())}
    result["totals"] = totals
    result["seconds"] = round(time.time() - t0, 1)
    out_json.write_text(json.dumps(result, indent=1, default=list), encoding="utf-8")
    print(json.dumps({"totals": totals, "seconds": result["seconds"],
                      "replication": result["pp_3x_replication_mismatches"],
                      "odd": result["odd_319x179_vs_base_shifted_by_1_1"],
                      "sub": result["camera_plus_0.6_0.2_px_vs_base_shifted_left_1"],
                      "ruler": {k: (v or {}).get("each_texel_once") for k, v in result["ruler_station"].items()},
                      "ruler_tall": {k: (v or {}).get("each_texel_once")
                                     for k, v in result["reimport"]["ruler_station_after_tall_sheet"].items()},
                      "tall_changed": {k: (v["changed_nearest_or_uncovered"], v["changed_linear"])
                                       for k, v in result["reimport"]["tall_sheet_changed_pixels"].items()},
                      "board_f_failures": [k for k, v in result.get("board_f", {}).items() if not v["pass"]],
                      "board_g_nonuniform": sorted({(v["magnification"], v["flipped"], v["delta_steps_of_1_256"])
                                                    for v in result.get("board_g", {}).values() if not v["uniform"]}),
                      "board_h_nonuniform": sorted(k for k, v in result.get("board_h", {}).items() if not v["uniform"]),
                      "board_h_mirror_mismatch": sorted(k for k, v in result.get("board_h", {}).items()
                                                        if v.get("flipped_twin_is_mirror") is False),
                      "board_i_nonuniform": sorted(k for k, v in result.get("board_i", {}).items() if not v["uniform"]),
                      "board_j_nonuniform": sorted(k for k, v in result.get("board_j", {}).items() if not v["uniform"]),
                      "orientation": result["orientation_mismatched_pixels"],
                      "nine_slice_flips": result["nine_slice_flip_mismatched_pixels"],
                      "painter_all_pass": all(v["pass"] for v in result["painter_probes"].values()),
                      "tone_8_equals_16": result["tone_8bit_equals_16bit"],
                      "gray_ramp": {k: v for k, v in result["gray_ramp"].items() if k != "shown"},
                      "fringe": result["linear_fringe"],
                      "cracks": result["cracks_enclosed_background_pixels"],
                      "source_free_equal": all(result["source_free"]["equal_to_original"].values()),
                      "live_latency": lat_found,
                      "live": {k: (v["unexplained"], v["ambiguous_mismatch"], len(v["wisp_boxes"]), v["repeat_equal"])
                               for k, v in steps_live.items()},
                      "magenta_hits": result["magenta_like_pixels"]["frames_with_hits"],
                      "cross_run": {k: (v if not isinstance(v, list) else len(v)) for k, v in result.get("cross_run", {}).items()}},
                     default=list))


if __name__ == "__main__":
    main()
