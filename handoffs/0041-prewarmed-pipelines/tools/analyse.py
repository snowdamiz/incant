#!/usr/bin/env python3
"""CPU-only numeric evidence for handoff 0041. Standard library only.

Decodes the captured engine PNGs (never edits them) and writes
handoffs/0041-prewarmed-pipelines/evidence/numeric.json with:

  * baseline/candidate, repeat, reimport, source-free and diagnostic comparisons
    on decoded pixels (byte identity is reported separately);
  * measurements at analytically projected points, using the saved project's own
    Transform components and camera specs: culling/alpha classification of every
    material card, normal-map relief direction, shadow coverage and holes, tone
    ramp, HDR shoulder samples, reimport colors and nine-slice corner colors.

Numbers support, but do not replace, the visual verdict in result.md.

Usage (worktree root): python3 -I handoffs/0041-prewarmed-pipelines/tools/analyse.py
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
from fixtures import pngio  # noqa: E402  (handoff 0032 decoder, unmodified)

ROOT = TOOLS.parents[2]
ART = ROOT / "artifacts/0041-prewarmed-pipelines"
SHOTS = ART / "shots"
OUT = TOOLS.parent / "evidence/numeric.json"
BUILD = json.loads((ART / "build.json").read_text(encoding="utf-8"))
_cache = {}


def image(path):
    if path not in _cache:
        _cache[path] = pngio.read(path)
    return _cache[path]


def shot(state, name, role, repeat=False):
    return SHOTS / state / f"{name}-{role}{'-repeat' if repeat else ''}.png"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def compare(a, b, inside=None):
    """Decoded-pixel comparison. `inside(x, y)` marks where differences are allowed."""
    ia, ib = image(a), image(b)
    result = {"a": str(a.relative_to(ART)), "b": str(b.relative_to(ART)), "byte_identical": sha(a) == sha(b),
              "dims": [ia.width, ia.height]}
    if (ia.width, ia.height) != (ib.width, ib.height):
        result["pixel_identical"] = False
        return result
    count, peak, box, outside = 0, 0, None, 0
    for y in range(ia.height):
        ra, rb = ia.rows[y], ib.rows[y]
        if ra == rb:
            continue
        for x in range(ia.width):
            pa, pb = ra[4 * x:4 * x + 4], rb[4 * x:4 * x + 4]
            if pa != pb:
                count += 1
                peak = max(peak, max(abs(p - q) for p, q in zip(pa, pb)))
                box = [x, y, x, y] if box is None else [min(box[0], x), min(box[1], y), max(box[2], x),
                                                        max(box[3], y)]
                if inside is not None and not inside(x, y):
                    outside += 1
    result.update(pixel_identical=count == 0, differing_pixels=count, max_channel_delta=peak, bbox=box)
    if inside is not None:
        result["differences_outside_expected_region"] = outside
    return result


# ------------------------------------------------------------ geometry --

def entities(project_name):
    """Saved document entities by name (read only)."""
    info = BUILD[project_name]
    document = json.loads((ROOT / info["project"]).read_text(encoding="utf-8"))
    scene = document["scenes"][info["scene"]]
    return {e["name"]: e for e in scene["entities"].values()}


def world(transform_value, local):
    s = transform_value["scale"]
    scaled = tuple(local[i] * s[i] for i in range(3))
    r = fixtures.rotate(transform_value["rotation"], scaled)
    return tuple(r[i] + transform_value["translation"][i] for i in range(3))


def view(project_name, camera):
    spec = BUILD[project_name]["views"][camera]
    return fixtures.View(camera, spec["eye"], spec["rotation"], fov=spec["fov"],
                         ortho_half_height=spec["ortho_half_height"], size=tuple(spec["size"]))


def card_local(u, v, size):
    return ((u - 0.5) * size, (0.5 - v) * size, 0.0)


def sample(img, x, y, radius=1):
    if not (0 <= x < img.width and 0 <= y < img.height):
        raise SystemExit(f"sample point {x:.1f},{y:.1f} is outside the frame")
    xs = range(max(0, round(x) - radius), min(img.width, round(x) + radius + 1))
    ys = range(max(0, round(y) - radius), min(img.height, round(y) + radius + 1))
    px = [img.px(i, j) for j in ys for i in xs]
    return [round(sum(p[c] for p in px) / len(px), 1) for c in range(3)]


def linear(c):
    v = c / 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def luminance(rgb):
    r, g, b = (linear(c) for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


# UV landmarks on the 64 x 64 card texture (texel centres / 64).
UV = {"top": (30.5 / 64, 3.5 / 64), "paper": (30.5 / 64, 30.5 / 64), "triangle": (12.7 / 64, 51.3 / 64), "glyph": (10.5 / 64, 20.5 / 64),
      "stripe0.2": (37.5 / 64, 48.5 / 64), "stripe0.4": (44.5 / 64, 48.5 / 64),
      "stripe0.6": (51.5 / 64, 48.5 / 64), "stripe0.8": (58.5 / 64, 48.5 / 64)}


def classify(rgb):
    r, g, b = rgb
    if g > r + 40 and b > r + 30:
        return "teal"
    if r > g + 60 and r > b + 50:
        return "red"
    if r > 150 and g > 130 and b > 100:
        return "paper"
    if r < 110 and g < 120 and b < 140 and abs(r - g) < 25 and b - r < 40:
        return "backdrop"
    return "mixed"


# ---------------------------------------------------------- materials --

def materials_cards(state):
    img = image(shot(state, "materials-board", "candidate"))
    cam = view("materials", "board")
    ents = entities("materials")
    rows = {}
    for name, meta in BUILD["materials"]["meta"].items():
        if name.startswith("Normals"):
            continue
        t = ents[name]["components"]["Transform"]
        out = {}
        for key, (u, v) in UV.items():
            p = cam.project(world(t, card_local(u, v, meta["size"])))
            rgb = sample(img, p[0], p[1])
            out[key] = {"rgb": rgb, "class": classify(rgb)}
        rows[name] = out
    return rows


def expected_material(name, reimported=False):
    alpha, cell, facing = name.split(" ")
    double = cell.startswith("double")
    if reimported and alpha == "OPAQUE" and cell.startswith("single"):
        return {"present": True, "triangle": "hole", "stripes": ["hole", "kept", "kept", "kept"]}
    present = facing == "front" or double
    if alpha == "OPAQUE":
        return {"present": present, "triangle": "teal", "stripes": ["kept"] * 4}
    if alpha == "MASK":
        return {"present": present, "triangle": "hole", "stripes": ["hole", "hole", "kept", "kept"]}
    return {"present": present, "triangle": "behind opaque bar", "stripes": ["behind opaque bar"] * 4}


def judge_material(name, sample_, reimported=False):
    exp = expected_material(name, reimported)
    # The top band (texel row 3.5) is plain paper on every card and lies above the glass strip.
    present = sample_["top"]["class"] == "paper"
    ok = present == exp["present"]
    if exp["present"] and not name.startswith("BLEND"):
        tri = sample_["triangle"]["class"]
        ok &= (tri == "teal") if exp["triangle"] == "teal" else (tri == "backdrop")
        for k, want in enumerate(exp["stripes"]):
            got = sample_[f"stripe{(k + 1) * 0.2:.1f}"]["class"]
            ok &= (got == "red") if want == "kept" else (got == "backdrop")
    if exp["present"] and name.startswith("BLEND"):
        bar = (88, 135, 102)
        for k in range(4):
            rgb = sample_[f"stripe{(k + 1) * 0.2:.1f}"]["rgb"]
            ok &= all(abs(a - b) <= 12 for a, b in zip(rgb, bar))
    return {"expected": exp, "matches": bool(ok)}


def relief(project_name, camera, state, names, card_size):
    """Direction of the dome's bright side on screen (0 deg = right, 90 deg = up)."""
    img = image(shot(state, f"{project_name}-{camera}", "candidate"))
    cam = view(project_name, camera)
    ents = entities(project_name)
    centre_uv = (fixtures.DOME_CENTER[0] / 64, fixtures.DOME_CENTER[1] / 64)
    offset = 0.55 * fixtures.DOME_RADIUS / 64
    key_screen = None
    out = {}
    for name in names:
        t = ents[name]["components"]["Transform"]
        centre = cam.project(world(t, card_local(*centre_uv, card_size)))
        flat = cam.project(world(t, card_local(30.5 / 64, 40.5 / 64 if card_size > 2 else 30.5 / 64, card_size)))
        gx = gy = 0.0
        points = {}
        for label, (du, dv) in {"image-up": (0, -offset), "image-down": (0, offset), "image-left": (-offset, 0),
                                "image-right": (offset, 0)}.items():
            p = cam.project(world(t, card_local(centre_uv[0] + du, centre_uv[1] + dv, card_size)))
            lum = luminance(sample(img, p[0], p[1]))
            points[label] = {"screen": [round(p[0], 1), round(p[1], 1)], "luminance": round(lum, 4)}
            gx += lum * (p[0] - centre[0])
            gy += lum * -(p[1] - centre[1])
        angle = math.degrees(math.atan2(gy, gx)) % 360
        quadrant = ("upper-right" if angle < 90 else "upper-left" if angle < 180 else "lower-left" if angle < 270
                    else "lower-right")
        out[name] = {"bright_side_angle_deg": round(angle, 1), "bright_quadrant": quadrant,
                     "flat_luminance": round(luminance(sample(img, flat[0], flat[1])), 4), "samples": points}
        if key_screen is None:
            light = fixtures.KEY_L
            a = cam.project(world(t, (0.0, 0.0, 0.0)))
            b = cam.project(tuple(world(t, (0.0, 0.0, 0.0))[i] + light[i] for i in range(3)))
            key_screen = round(math.degrees(math.atan2(-(b[1] - a[1]), b[0] - a[0])) % 360, 1)
    return {"key_light_screen_angle_deg": key_screen, "cards": out}


def tone_map(c):
    """Python port of crates/incant_render/src/tone_map.wgsl (prediction only)."""
    peak = max(c)
    if peak < 0.8:
        return list(c)
    shoulder = 0.2
    new_peak = 1 - shoulder * shoulder / (peak + shoulder - 0.8)
    compressed = [v * new_peak / peak for v in c]
    g = 1 - 1 / (0.15 * (peak - new_peak) + 1)
    return [a + (new_peak - a) * g for a in compressed]


def srgb(v):
    v = min(max(v, 0.0), 1.0)
    return 255 * (12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055)


def tone_ramp():
    img = image(shot("initial", "materials-board", "candidate"))
    cam = view("materials", "board")
    out = []
    for k, (label, emissive) in enumerate(fixtures.M_SWATCHES):
        p = cam.project(((k - 3.5) * fixtures.M_SWATCH_PITCH, fixtures.M_SWATCH_Y, 0.0))
        measured = sample(img, p[0], p[1], radius=3)
        predicted = [round(srgb(v), 1) for v in tone_map(emissive)]
        out.append({"swatch": label, "emissive_linear": list(emissive), "measured_srgb": measured,
                    "emissive_only_prediction_srgb": predicted,
                    "measured_minus_prediction": [round(m - q, 1) for m, q in zip(measured, predicted)]})
    return out


# ------------------------------------------------------------- shadows --

def shadows_top():
    img = image(shot("initial", "shadows-top", "candidate"))
    cam = view("shadows", "top")
    ents = entities("shadows")
    light = fixtures.SUN_L
    open_floor = cam.project((-9.0, 0.0, 6.5))
    lit = luminance(sample(img, open_floor[0], open_floor[1], radius=2))
    out = {"open_floor_luminance": round(lit, 4), "cards": {}}
    for name, meta in BUILD["shadows"]["meta"].items():
        t = ents[name]["components"]["Transform"]
        card = {}
        for key, (u, v) in UV.items():
            if key in ("glyph", "top"):
                continue
            p = world(t, card_local(u, v, fixtures.S_CARD))
            floor = tuple(p[i] - p[1] / light[1] * light[i] for i in range(3))
            q = cam.project(floor)
            lum = luminance(sample(img, q[0], q[1]))
            card[key] = {"floor_luminance": round(lum, 4), "shadowed": lum < 0.6 * lit}
        if not meta["casts"]:
            expect = {k: False for k in card}
        elif meta["facing"] == "down" and not meta["double_sided"]:
            expect = {k: False for k in card}
        elif meta["alpha"] == "OPAQUE":
            expect = {k: True for k in card}
        else:
            expect = {"paper": True, "triangle": False, "stripe0.2": False, "stripe0.4": False, "stripe0.6": True,
                      "stripe0.8": True}
        card["expected_shadowed"] = expect
        card["matches"] = all(card[k]["shadowed"] == expect[k] for k in expect)
        out["cards"][name] = card
    return out


# -------------------------------------------------------------- lights --

def lights_samples():
    img = image(shot("initial", "lights-board", "candidate"))
    cam = view("lights", "board")
    points = {"white spot pool (floor, 10 deg off axis)": (3.6, 0.0, 1.4), "red cube top face": (3.6, 1.1, 0.3),
              "magenta wall pool centre": (-4.5, 3.2, -4.0), "dim floor outside every light range": (6.5, 0.0, -3.0)}
    out = {}
    for label, p in points.items():
        q = cam.project(p)
        rgb = sample(img, q[0], q[1], radius=2)
        out[label] = {"screen": [round(q[0], 1), round(q[1], 1)], "srgb": rgb}
    return out


def diagnostic_region():
    cam = view("lights", "board")
    ents = entities("lights")
    t = ents["Diagnostic cube"]["components"]["Transform"]
    xs, ys = [], []
    for sx in (-0.5, 0.5):
        for sy in (-0.5, 0.5):
            for sz in (-0.5, 0.5):
                q = cam.project(world(t, (sx, sy, sz)))
                xs.append(q[0])
                ys.append(q[1])
    box = [min(xs) - 2, min(ys) - 2, max(xs) + 2, max(ys) + 2]
    return box, lambda x, y: box[0] <= x <= box[2] and box[1] <= y <= box[3]


def reimport_region():
    cam = view("materials", "board")
    ents = entities("materials")
    boxes = []
    for name in ents:
        if name.startswith("OPAQUE single"):
            t = ents[name]["components"]["Transform"]
            pts = [cam.project(world(t, (sx * 0.75, sy * 0.75, 0.0))) for sx in (-1, 1) for sy in (-1, 1)]
            boxes.append([min(p[0] for p in pts) - 2, min(p[1] for p in pts) - 2, max(p[0] for p in pts) + 2,
                          max(p[1] for p in pts) + 2])
    return boxes, lambda x, y: any(b[0] <= x <= b[2] and b[1] <= y <= b[3] for b in boxes)


# ------------------------------------------------------------- sprites --

PANEL_A = {"TL": "red", "TR": "green", "BL": "blue", "BR": "yellow"}
PANEL_B = {"TL": "magenta", "TR": "cyan", "BL": "lime", "BR": "white"}
NAMED = {"red": (220, 60, 60), "green": (60, 190, 90), "blue": (70, 110, 230), "yellow": (235, 200, 60),
         "magenta": (215, 70, 200), "cyan": (60, 210, 220), "lime": (150, 220, 60), "white": (245, 245, 245)}


def nearest_name(rgb):
    return min(NAMED, key=lambda n: sum((a - b) ** 2 for a, b in zip(rgb, NAMED[n])))


def nine_slices():
    img = image(shot("initial", "sprites-pixel", "candidate"))
    panels = {"C ordinary nine-slice": ((8, 86, 64, 40), PANEL_A, 2.5),
              "C atlas nine-slice": ((80, 86, 64, 40), PANEL_B, 2.5),
              "C reflected nine-slice": ((152, 86, 64, 40), {"TL": "green", "TR": "red", "BL": "yellow",
                                                             "BR": "blue"}, 2.5),
              "C atlas nine-slice flip-xy": ((224, 86, 64, 40), {"TL": "white", "TR": "lime", "BL": "cyan",
                                                                 "BR": "magenta"}, 2.5),
              "C atlas nine-slice compressed": ((300, 86, 8, 40), PANEL_B, 2.0)}
    out = {}
    for name, ((x, y, w, h), expect, inset) in panels.items():
        corners = {"TL": (x + inset, y + 2.5), "TR": (x + w - inset, y + 2.5), "BL": (x + inset, y + h - 2.5),
                   "BR": (x + w - inset, y + h - 2.5)}
        got = {}
        for corner, (lx, ly) in corners.items():
            rgb = sample(img, 3 * lx, 3 * ly, radius=0)
            got[corner] = {"srgb": rgb, "nearest": nearest_name(rgb)}
        out[name] = {"corners": got, "expected": expect,
                     "matches": all(got[c]["nearest"] == expect[c] for c in expect)}
    return out


# ---------------------------------------------------------------- main --

def main():
    pairs = {}
    for state in ("initial", "reimport", "sourcefree", "diagnostic"):
        for path in sorted((SHOTS / state).glob("*-baseline.png")):
            name = path.name[:-len("-baseline.png")]
            pairs[f"{state}/{name}"] = compare(path, shot(state, name, "candidate"))
    repeats = {role: compare(shot("initial", "materials-board", role),
                             shot("initial", "materials-board", role, repeat=True))
               for role in ("baseline", "candidate")}
    boxes, inside = reimport_region()
    reimport = compare(shot("initial", "materials-board", "candidate"), shot("reimport", "materials-board",
                                                                              "candidate"), inside)
    reimport["expected_region_boxes"] = [[round(v, 1) for v in b] for b in boxes]
    box, inside_cube = diagnostic_region()
    diagnostic = compare(shot("initial", "lights-board", "candidate"), shot("diagnostic", "lights-board", "candidate"),
                         inside_cube)
    diagnostic["expected_region_box"] = [round(v, 1) for v in box]
    sourcefree = {"materials-board vs reimport": compare(shot("reimport", "materials-board", "candidate"),
                                                         shot("sourcefree", "materials-board", "candidate")),
                  "sprites-pixel vs initial": compare(shot("initial", "sprites-pixel", "candidate"),
                                                      shot("sourcefree", "sprites-pixel", "candidate"))}
    cards = {}
    for state, reimported in (("initial", False), ("reimport", True)):
        measured = materials_cards(state)
        cards[state] = {name: {"samples": s, **judge_material(name, s, reimported)} for name, s in measured.items()}
    normals_names = [f"Normals {slot}" for slot in fixtures.N_SLOTS]
    front_names = [n for n in BUILD["materials"]["meta"] if not n.startswith("Normals")
                   and not n.startswith("BLEND") and (n.endswith("front") or " double" in n)]
    evidence = {
        "baseline_vs_candidate": pairs,
        "fresh_process_repeats": repeats,
        "reimport_change_localisation": reimport,
        "source_free": sourcefree,
        "diagnostic_cube_localisation": diagnostic,
        "material_cards": cards,
        "normal_relief_normals_view": relief("materials", "normals", "initial", normals_names, fixtures.N_CARD),
        "normal_relief_board": relief("materials", "board", "initial", front_names, fixtures.M_CARD),
        "tone_ramp": tone_ramp(),
        "shadows_top": shadows_top(),
        "lights": lights_samples(),
        "nine_slices": nine_slices(),
    }
    evidence["summary"] = {
        "pairs": len(pairs), "pairs_byte_identical": sum(p["byte_identical"] for p in pairs.values()),
        "pairs_pixel_identical": sum(p["pixel_identical"] for p in pairs.values()),
        "repeats_identical": all(r["pixel_identical"] for r in repeats.values()),
        "material_cards_matching": {s: f"{sum(c['matches'] for c in v.values())}/{len(v)}" for s, v in cards.items()},
        "shadow_cards_matching": f"{sum(c['matches'] for c in evidence['shadows_top']['cards'].values())}/"
                                 f"{len(evidence['shadows_top']['cards'])}",
        "nine_slices_matching": f"{sum(c['matches'] for c in evidence['nine_slices'].values())}/"
                                f"{len(evidence['nine_slices'])}",
        "reimport_differences_outside_cells": reimport["differences_outside_expected_region"],
        "diagnostic_differences_outside_cube": diagnostic["differences_outside_expected_region"],
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(evidence, indent=1, sort_keys=True), encoding="utf-8")
    print(json.dumps(evidence["summary"], indent=1))


if __name__ == "__main__":
    main()
