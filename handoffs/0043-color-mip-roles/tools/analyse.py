#!/usr/bin/env python3
"""CPU-only numeric evidence for handoff 0043. Launches no engine process.

Decodes the unedited engine frames (never edits them) and writes
handoffs/0043-color-mip-roles/evidence/analysis.json plus DERIVED images:

* byte identity: baseline vs candidate on the retained old cache, recooked vs
  fresh source-free replay, and the all-alpha-one environment control;
* the ortho board, cell by cell (pixel-aligned rectangles from fixtures): which
  cells change between the retained and recooked caches on the same candidate
  binary, exact comparisons of every alpha variant against its all-alpha-one
  control at the same LOD, colour means and the teal-triangle interior;
* the oblique view: where retained/recooked differences fall (projected cells);
* the environment: sphere and sprite regions, the all-alpha-one oracle and
  confinement of every change to the spheres.

Numbers support, but do not replace, the visual review in result.md.
Usage (worktree root): python3 -I -B handoffs/0043-color-mip-roles/tools/analyse.py
"""

import hashlib
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import fixtures  # noqa: E402

REF, PNG = fixtures.REF, fixtures.PNG
ROOT = TOOLS.parents[2]
ART = ROOT / "artifacts/0043-color-mip-roles"
FRAMES = ART / "frames"
PACKET = TOOLS.parent
EVIDENCE = PACKET / "evidence"
DERIVED = PACKET / "screenshots/derived"
BUILD = json.loads((ART / "build.json").read_text(encoding="utf-8"))
CONTROLS = {"O-mixed": "O-one", "O-zero": "O-one", "O-rgb": "O-one", "E-O-mixed": "E-O-one",
            "E-O-zero": "E-O-one", "E-B-mixed": "E-B-one", "E-B-zero": "E-B-one"}
_images = {}


def frame(state, view):
    return FRAMES / state / f"{view}.png"


def image(path):
    if path not in _images:
        _images[path] = PNG.read(path)
    return _images[path]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def rel(path):
    return str(Path(path).relative_to(ROOT))


def rgb(img, x, y):
    return img.rows[y][4 * x:4 * x + 3]


def mean(img, pixels):
    pixels = list(pixels)
    if not pixels:
        return None
    total = [0, 0, 0]
    for x, y in pixels:
        for i, c in enumerate(rgb(img, x, y)):
            total[i] += c
    return [round(t / len(pixels), 1) for t in total]


def differing(a, b, pixels=None):
    ia, ib = image(a), image(b)
    if pixels is None:
        pixels = ((x, y) for y in range(ia.height) for x in range(ia.width) if ia.rows[y] != ib.rows[y])
    out = []
    for x, y in pixels:
        if ia.rows[y][4 * x:4 * x + 4] != ib.rows[y][4 * x:4 * x + 4]:
            out.append((x, y))
    return out


def rows_differ(a, b):
    ia, ib = image(a), image(b)
    return [(x, y) for y in range(ia.height) if ia.rows[y] != ib.rows[y]
            for x in range(ia.width) if ia.rows[y][4 * x:4 * x + 4] != ib.rows[y][4 * x:4 * x + 4]]


def identity(a, b):
    same = sha(a) == sha(b)
    return {"a": rel(a), "b": rel(b), "byte_identical": same,
            "differing_pixels": 0 if same else len(rows_differ(a, b))}


# ------------------------------------------------------------- the board --

CELLS = fixtures.cells()
RECTS = {name: fixtures.cell_rect(cell) for name, cell in CELLS.items()}


def deep_triangle():
    """64x64 texel mask: teal triangle texels at least two texels from its edge."""
    inside = {(x, y) for y in range(64) for x in range(64) if REF.triangle_hole(x, y)}
    return {(x, y) for x, y in inside
            if all((x + i, y + j) in inside for i in range(-2, 3) for j in range(-2, 3))}


TRIANGLE = deep_triangle()


def cell_pixels(name, inset=1):
    left, top, right, bottom = RECTS[name]
    return [(x, y) for y in range(top + inset, bottom - inset) for x in range(left + inset, right - inset)]


def triangle_pixels(name):
    left, top, _, _ = RECTS[name]
    k = CELLS[name]["k"]
    out = []
    for x, y in cell_pixels(name):
        u, v = ((x + 0.5 - left) / 80 * k) % 1, ((y + 0.5 - top) / 80 * k) % 1
        if (int(u * 64), int(v * 64)) in TRIANGLE:
            out.append((x, y))
    return out


def against_control(img_path, name):
    """Exact comparison of a cell with its all-alpha-one control at the same LOD."""
    control = f"{CONTROLS[name.split(' ')[0]]} {name.split(' ')[1]}"
    img = image(img_path)
    (l1, t1, _, _), (l2, t2, _, _) = RECTS[name], RECTS[control]
    count, worst = 0, 0
    for y in range(80):
        a, b = img.rows[t1 + y], img.rows[t2 + y]
        for x in range(80):
            pa, pb = a[4 * (l1 + x):4 * (l1 + x) + 3], b[4 * (l2 + x):4 * (l2 + x) + 3]
            if pa != pb:
                count += 1
                worst = max(worst, max(abs(p - q) for p, q in zip(pa, pb)))
    return {"control": control, "differing_pixels": count, "of": 6400, "max_channel_delta": worst}


def board():
    old, retained, new = (frame(s, "materials-board") for s in ("old", "retained", "recooked"))
    per_cell, outside = {}, 0
    for x, y in rows_differ(retained, new):
        hit = [n for n, (l, t, r, b) in RECTS.items() if l <= x < r and t <= y < b]
        if hit:
            per_cell[hit[0]] = per_cell.get(hit[0], 0) + 1
        else:
            outside += 1
    oracle = {state: {name: against_control(frame(state, "materials-board"), name)
                      for name in CELLS if name.split(" ")[0] in CONTROLS} for state in ("old", "recooked")}
    colours = {}
    for name in CELLS:
        entry = {}
        for state, path in (("old", old), ("recooked", new)):
            entry[state] = {"cell_mean_srgb": mean(image(path), cell_pixels(name))}
            if name.split(" ")[0] in ("O-mixed", "O-zero", "O-one", "O-rgb", "E-O-mixed", "E-O-zero", "E-O-one") \
                    and CELLS[name]["k"] in (1, 2):
                entry[state]["triangle_interior_srgb"] = mean(image(path), triangle_pixels(name))
        colours[name] = entry
    unchanged = sorted(n for n in CELLS if n not in per_cell)
    return {
        "lod": {f"k{k}": round(fixtures.lod(k), 3) for k in fixtures.TILES},
        "retained_vs_recooked_same_candidate_binary": {
            "changed_cells": dict(sorted(per_cell.items())), "changed_pixels_outside_cells": outside,
            "unchanged_cells": unchanged},
        "alpha_variant_vs_all_alpha_one_control": oracle,
        "colours": colours,
    }


# ----------------------------------------------------------- the oblique --

def view_of(project, camera):
    v = BUILD[project]["views"][camera]
    return REF.View(camera, v["eye"], v["rotation"], fov=v["fov"], ortho_half_height=v["ortho_half_height"],
                    size=tuple(v["size"]), far=60.0)


def inside_polygon(px, py, polygon):
    sign = 0
    for (x0, y0), (x1, y1) in zip(polygon, polygon[1:] + polygon[:1]):
        cross = (x1 - x0) * (py - y0) - (y1 - y0) * (px - x0)
        if cross != 0:
            if sign == 0:
                sign = 1 if cross > 0 else -1
            elif (cross > 0) != (sign > 0):
                return False
    return True


def grow(polygon, pixels):
    cx = sum(p[0] for p in polygon) / len(polygon)
    cy = sum(p[1] for p in polygon) / len(polygon)
    out = []
    for x, y in polygon:
        d = math.hypot(x - cx, y - cy)
        out.append((x + (x - cx) / d * pixels, y + (y - cy) / d * pixels))
    return out


def oblique():
    view = view_of("materials", "oblique")
    polygons = {}
    for name, cell in CELLS.items():
        x, y, z = cell["centre"]
        h = fixtures.CELL / 2
        corners = [(x - h, y + h, z), (x + h, y + h, z), (x + h, y - h, z), (x - h, y - h, z)]
        polygons[name] = grow([view.project(c)[:2] for c in corners], 1.5)
    per_cell, outside = {}, 0
    for x, y in rows_differ(frame("retained", "materials-oblique"), frame("recooked", "materials-oblique")):
        hit = [n for n, poly in polygons.items() if inside_polygon(x + 0.5, y + 0.5, poly)]
        if hit:
            per_cell[hit[0]] = per_cell.get(hit[0], 0) + 1
        else:
            outside += 1
    return {"retained_vs_recooked_same_candidate_binary": {
        "changed_cells": dict(sorted(per_cell.items())), "changed_pixels_outside_cells": outside,
        "cell_polygons_grown_px": 1.5}}


# ------------------------------------------------------- the environment --

def sphere_pixels(project, label, scale=1.0, half=None):
    """Pixels whose centre ray meets the sphere (radius x scale): exact perspective
    silhouette, independent of the 32x64 tessellation inscribed in it."""
    view = view_of(project, "view")
    meta = BUILD[project]["meta"][label]
    centre, radius = meta["centre"], meta["radius"] * scale
    w, h = view.size
    t = math.tan(math.radians(view.fov) / 2)
    cx, cy, _ = view.project(centre)
    reach = 1.3 * meta["radius"] * (h / 2) / t / math.dist(view.eye, centre)
    out = []
    for y in range(max(0, int(cy - reach)), min(h, int(cy + reach) + 1)):
        if half == "upper" and y + 0.5 >= cy or half == "lower" and y + 0.5 < cy:
            continue
        for x in range(max(0, int(cx - reach)), min(w, int(cx + reach) + 1)):
            d = REF.norm(REF.rotate(view.rotation, ((2 * (x + 0.5) / w - 1) * t * w / h,
                                                     (1 - 2 * (y + 0.5) / h) * t, -1.0)))
            oc = REF.sub(view.eye, centre)
            b = REF.dot(oc, d)
            if b * b - (REF.dot(oc, oc) - radius * radius) >= 0:
                out.append((x, y))
    return out


def sprite_rects(project):
    view = view_of(project, "view")
    out = {}
    for name, meta in BUILD[project]["meta"].items():
        if "size" not in meta:
            continue
        (x, y, z), (w, h) = meta["centre"], meta["size"]
        corners = [view.project((x + sx * w / 2, y + sy * h / 2, z))[:2] for sx in (-1, 1) for sy in (-1, 1)]
        xs, ys = [c[0] for c in corners], [c[1] for c in corners]
        out[name] = [min(xs), min(ys), max(xs), max(ys)]
    return out


def inside_rect(p, rect, grow=1):
    x0, y0, x1, y1 = rect
    return x0 - grow <= p[0] + 0.5 <= x1 + grow and y0 - grow <= p[1] + 0.5 <= y1 + grow


def rect_pixels(rect, inset=2):
    x0, y0, x1, y1 = rect
    return [(x, y) for y in range(math.ceil(y0) + inset, math.floor(y1) - inset)
            for x in range(math.ceil(x0) + inset, math.floor(x1) - inset)]


def environment():
    sprites = sprite_rects("environment")
    old, new = frame("old", "environment-view"), frame("recooked", "environment-view")
    control = frame("recooked", "envcontrol-view")
    changed = rows_differ(old, new)
    silhouettes = {label: set(sphere_pixels("environment", label)) for label in fixtures.SPHERES}
    covered = set().union(*silhouettes.values())
    outside = [p for p in changed if p not in covered]
    means = {}
    for label in fixtures.SPHERES:
        for half in ("upper", "lower"):
            pixels = sphere_pixels("environment", label, scale=0.85, half=half)
            means[f"{label} {half}"] = {
                "old": mean(image(old), pixels), "recooked": mean(image(new), pixels),
                "control_recooked": mean(image(control), pixels),
                "control_old": mean(image(frame("old", "envcontrol-view")), pixels)}
    sphere_pixels_all = sorted(covered)
    sprite_checks = {}
    for name in ("Sky sprite", "Atlas flag", "Atlas panel"):
        pixels = rect_pixels(sprites[name])
        sprite_checks[name] = {
            "pixels": len(pixels),
            "old_vs_recooked_differing": len(differing(old, new, pixels)),
            "recooked_vs_alpha_one_control_differing": len(differing(new, control, pixels)),
            "mean_old": mean(image(old), pixels), "mean_recooked": mean(image(new), pixels)}
    stripe_pixels = [p for n, r in sprites.items() if n.startswith("Stripe") for p in rect_pixels(r, 1)]
    return {
        "sphere_silhouette_pixels": {k: len(v) for k, v in silhouettes.items()},
        "old_vs_recooked": {"differing_pixels": len(changed),
                            "differing_outside_exact_sphere_silhouettes": len(outside)},
        "recooked_vs_alpha_one_control_inside_spheres": {
            "pixels": len(sphere_pixels_all), "differing": len(differing(new, control, sphere_pixels_all))},
        "old_vs_alpha_one_control_inside_spheres": {
            "pixels": len(sphere_pixels_all),
            "differing": len(differing(old, frame("old", "envcontrol-view"), sphere_pixels_all))},
        "recooked_vs_alpha_one_control_outside_spheres_and_sky_sprite": len(
            [p for p in rows_differ(new, control) if p not in covered
             and not inside_rect(p, sprites["Sky sprite"])]),
        "stripes_old_vs_recooked_differing": len(differing(old, new, stripe_pixels)),
        "sphere_half_means_srgb": means,
        "sprites": sprite_checks,
    }


# --------------------------------------------------------------- derived --

def diff_map(a, b, out):
    """DERIVED: b dimmed to grey, red where a and b differ."""
    ia, ib = image(a), image(b)
    pixels = []
    for y in range(ib.height):
        ra, rb = ia.rows[y], ib.rows[y]
        for x in range(ib.width):
            p = rb[4 * x:4 * x + 3]
            if ra[4 * x:4 * x + 4] != rb[4 * x:4 * x + 4]:
                pixels.append((255, 0, 0, 255))
            else:
                g = round(0.4 * (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]))
                pixels.append((g, g, g, 255))
    PNG.write_rgba(out, ib.width, ib.height, pixels, overwrite=True)
    return out


CROP_CELLS = ("O-mixed", "O-zero", "O-one", "M-mixed", "B-mixed", "E-O-zero", "E-B-zero")


def crops(out, scale=3, gap=8):
    """DERIVED: columns follow CROP_CELLS; rows are k2 old, k2 recooked, k6 old, k6 recooked."""
    old, new = image(frame("old", "materials-board")), image(frame("recooked", "materials-board"))
    tile = 80 * scale
    width = len(CROP_CELLS) * (tile + gap) + gap
    rows = [(old, 2), (new, 2), (old, 6), (new, 6)]
    height = len(rows) * (tile + gap) + gap
    canvas = [[(24, 24, 28, 255)] * width for _ in range(height)]
    for r, (img, k) in enumerate(rows):
        for c, name in enumerate(CROP_CELLS):
            left, top, _, _ = RECTS[f"{name} k{k}"]
            ox, oy = gap + c * (tile + gap), gap + r * (tile + gap)
            for y in range(tile):
                row = img.rows[top + y // scale]
                for x in range(tile):
                    i = 4 * (left + x // scale)
                    canvas[oy + y][ox + x] = (row[i], row[i + 1], row[i + 2], 255)
    PNG.write_rgba(out, width, height, [p for row in canvas for p in row], overwrite=True)
    return out


def main():
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    DERIVED.mkdir(parents=True, exist_ok=True)
    pairs = {
        "baseline_vs_candidate_retained_old_cache": [identity(frame("old", v), frame("retained", v))
                                                     for v in ("materials-board", "materials-oblique",
                                                               "environment-view")],
        "recooked_vs_fresh_sourcefree_replay": [identity(frame("recooked", v), frame("sourcefree", v))
                                                for v in ("materials-board", "environment-view")],
        "alpha_one_environment_control_baseline_vs_candidate_recooked": [
            identity(frame("old", "envcontrol-view"), frame("recooked", "envcontrol-view"))],
        "retained_vs_recooked": [identity(frame("retained", v), frame("recooked", v))
                                 for v in ("materials-board", "materials-oblique", "environment-view")],
    }
    frames = {f"{p.parent.name}/{p.name}": sha(p) for p in sorted(FRAMES.rglob("*.png"))}
    result = {"frames": frames, "identity": pairs, "board": board(), "oblique": oblique(),
              "environment": environment()}
    derived = [
        diff_map(frame("retained", "materials-board"), frame("recooked", "materials-board"),
                 DERIVED / "DERIVED-board-retained-vs-recooked-diff.png"),
        diff_map(frame("retained", "materials-oblique"), frame("recooked", "materials-oblique"),
                 DERIVED / "DERIVED-oblique-retained-vs-recooked-diff.png"),
        diff_map(frame("old", "environment-view"), frame("recooked", "environment-view"),
                 DERIVED / "DERIVED-environment-old-vs-recooked-diff.png"),
        crops(DERIVED / "DERIVED-board-minified-cells-x3.png"),
    ]
    result["derived"] = {rel(p): sha(p) for p in derived}
    (EVIDENCE / "analysis.json").write_text(json.dumps(result, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"identity": {k: [(e["byte_identical"], e["differing_pixels"]) for e in v]
                                   for k, v in pairs.items()},
                      "board_changed": result["board"]["retained_vs_recooked_same_candidate_binary"],
                      "oblique": result["oblique"], "environment": {k: v for k, v in result["environment"].items()
                                                                   if k != "sphere_half_means_srgb"}}, indent=1))


if __name__ == "__main__":
    main()
