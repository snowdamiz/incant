#!/usr/bin/env python3
"""Independent checks of a handoff 0034 run, measured from PNG and KTX2 bytes.

  * cooked pages: every packed texel, every extruded edge texel and every empty
    texel of each cooked atlas version, against the authored source images;
  * differential rendering: cooked atlas against the explicit-rectangle control,
    and every later state against the state it must reproduce;
  * decoded rendering: strip texels named by colour across the 512/1024 span cuts,
    orientation, nine-slice and painter-order predictions at exact placements,
    and hidden-RGB fringe tests for linear sampling.

Nothing here reads engine source code or calls the engine. Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_analyse.py RUN OUT.json [RUN2]
"""

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import atlas_scene as sc  # noqa: E402
import pngio  # noqa: E402

IDENTITY_MAX = 234  # handoff 0032: opaque sRGB 0..234 round-trips the display transform exactly
_CACHE = {}


def image(path):
    path = str(path)
    if path not in _CACHE:
        _CACHE[path] = pngio.read(path)
    return _CACHE[path]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


# ------------------------------------------------------------- comparisons --

def compare(a, b, masks=None, collect=None):
    """Byte comparison of two PNG frames; on difference, pixel statistics.
    masks: {name: (x, y, w, h)} rectangles used to attribute differing pixels;
    collect: a list that receives (x, y, level) for every differing pixel."""
    if sha(a) == sha(b):
        return {"identical": True}
    ia, ib = image(a), image(b)
    if (ia.width, ia.height) != (ib.width, ib.height):
        return {"identical": False, "size_mismatch": [ia.width, ia.height, ib.width, ib.height]}
    count, worst, box, hist, inside = 0, 0, None, {}, {}
    for y in range(ia.height):
        ra, rb = ia.rows[y], ib.rows[y]
        if ra == rb:
            continue
        for x in range(ia.width):
            pa, pb = ra[4 * x:4 * x + 4], rb[4 * x:4 * x + 4]
            if pa == pb:
                continue
            d = max(abs(p - q) for p, q in zip(pa, pb))
            if collect is not None:
                collect.append((x, y, d))
            count += 1
            worst = max(worst, d)
            hist[d] = hist.get(d, 0) + 1
            box = [x, y, x, y] if box is None else [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
            if masks is not None:
                owner = next((n for n, (mx, my, mw, mh) in masks.items()
                              if mx <= x < mx + mw and my <= y < my + mh), "outside")
                inside[owner] = inside.get(owner, 0) + 1
    out = {"identical": False, "pixels": count, "max_level": worst, "bbox": box,
           "histogram": {str(k): v for k, v in sorted(hist.items())}}
    if masks is not None:
        out["by_region"] = inside
    return out


# ------------------------------------------------------------------ KTX2 --

def read_ktx2(path):
    data = Path(path).read_bytes()
    if data[:12] != b"\xabKTX 20\xbb\r\n\x1a\n":
        raise ValueError(f"{path}: not KTX2")
    vk, _, w, h, _, _, _, levels, sc_ = struct.unpack("<9I", data[12:48])
    off, length, _ = struct.unpack("<3Q", data[80:104])
    if vk != 43 or sc_ != 0 or length != w * h * 4:
        raise ValueError(f"{path}: expected uncompressed R8G8B8A8_SRGB level 0, got vk={vk} sc={sc_}")
    return {"width": w, "height": h, "levels": levels, "level0": data[off:off + length]}


def descriptor_for(run, meta):
    """The authored descriptor this page was cooked from, found by its dependency hash."""
    want = meta["dependency"]["sha256"]
    for path in sorted((run / "art").glob("*.atlas.json")):
        if sha(path) == want:
            return path.name, json.loads(path.read_text(encoding="utf-8"))
    raise SystemExit(f"no authored descriptor matches {want}")


def verify_page(run, fingerprint, manifest, ballast_name):
    cache = run / "atlas/.incant/cache/textures"
    meta = json.loads((cache / f"{fingerprint}.json").read_text(encoding="utf-8"))
    ktx = read_ktx2(cache / f"{meta['content_sha256']}.ktx2")
    descriptor_name, descriptor = descriptor_for(run, meta)
    atlas = meta["atlas"]
    w, h = atlas["size"]
    p = atlas["padding"]
    page = ktx["level0"]
    result = {"fingerprint": fingerprint, "descriptor": descriptor_name, "content_sha256": meta["content_sha256"],
              "size": [w, h], "padding": p,
              "ktx_size": [ktx["width"], ktx["height"]], "mip_levels": ktx["levels"],
              "frames": len(atlas["frames"]), "dependencies": meta["dependencies"]}
    covered = bytearray(w * h)
    content_bad = extrusion_bad = overlap = size_bad = 0
    examples = []
    for fid, frame in atlas["frames"].items():
        authored = descriptor["frames"][fid]
        path = ballast_name if authored["name"] == "ballast" else authored["path"]
        src = image(run / "art" / path)
        rx, ry, rw, rh = authored.get("region") or [0, 0, src.width, src.height]
        x, y, fw, fh = frame["region"]
        if (fw, fh) != (rw, rh) or frame["name"] != authored["name"]:
            size_bad += 1
            continue
        for dy in range(-p, fh + p):
            for dx in range(-p, fw + p):
                px, py = x + dx, y + dy
                i = py * w + px
                if covered[i]:
                    overlap += 1
                covered[i] = 1
                want = src.px(rx + min(max(dx, 0), fw - 1), ry + min(max(dy, 0), fh - 1))
                got = tuple(page[4 * i:4 * i + 4])
                if got != want:
                    inner = 0 <= dx < fw and 0 <= dy < fh
                    content_bad += inner
                    extrusion_bad += not inner
                    if len(examples) < 8:
                        examples.append({"frame": authored["name"], "at": [px, py], "want": want, "got": got})
    empty_bad = sum(1 for i in range(w * h) if not covered[i] and page[4 * i:4 * i + 4] != b"\0\0\0\0")
    result["frames_missing_from_layout"] = sorted(descriptor["frames"][k]["name"] for k in descriptor["frames"]
                                                  if k not in atlas["frames"])
    result.update(frame_size_or_name_mismatches=size_bad, content_mismatches=content_bad,
                  extrusion_mismatches=extrusion_bad, overlapping_texels=overlap,
                  nonzero_empty_texels=empty_bad, covered_texels=sum(covered), examples=examples,
                  regions={manifest["frames"][k]["name"]: v["region"] for k, v in atlas["frames"].items()})
    result["pass"] = (ktx["width"], ktx["height"]) == (w, h) and not (size_bad or content_bad or extrusion_bad
                                                                     or overlap or empty_bad
                                                                     or result["frames_missing_from_layout"])
    return result


# ------------------------------------------------------ geometry helpers --

def scale_of(shot):
    """Output px per logical px for a pixel-perfect shot name like pp-S-960x540."""
    w = int(shot.rsplit("-", 1)[1].split("x")[0])
    return {320: 1, 960: 3}.get(w, 1)


def strip_texel(rgb, phase):
    r, g, b = rgb
    if r % 2 or g % 2 or not (60 <= r <= 186) or not (60 <= g <= 186):
        return None
    return (g - 60) // 2 * 64 + (r - 60) // 2 - phase, b


def expected_index(t, length, m, flip):
    """Documented nearest rule along one axis: region-local texel, a 1/1024 screen-px
    bias toward increasing screen coordinate, clamped to the region."""
    bias = 1.0 / (1024.0 * m)
    k = math.floor(length - t - bias) if flip else math.floor(t + bias)
    return min(max(k, 0), length - 1)


def check_strip(img, axis, start, cross, thickness, length, m, flip, offset, base, phase, frame_size):
    """Decode every covered pixel of one strip sprite in a 1-output-px-per-logical-px frame.
    axis 0: strip runs along x starting at `start` (float), occupying rows cross..cross+thickness*m.
    Returns mismatches, run-length and cut statistics."""
    fw, fh = frame_size
    along = fw if axis == 0 else fh
    bad, examples, sequence = 0, [], []
    first = math.ceil(start - 0.5)
    for u in range(max(0, first), along):
        t = (u + 0.5 - start) / m
        if t >= length:
            break
        if t < 0:
            continue
        want = expected_index(t, length, m, flip) + offset
        got_row = None
        for j in range(thickness):
            for jj in range(m):
                v = cross + j * m + jj
                if not (0 <= v < (fh if axis == 0 else fw)):
                    continue
                rgb = img.rgb(u, v) if axis == 0 else img.rgb(v, u)
                decoded = strip_texel(rgb, phase)
                expected_b = base[j] if isinstance(base, (list, tuple)) else base
                if decoded != (want, expected_b):
                    bad += 1
                    if len(examples) < 6:
                        examples.append({"pixel": [u, v] if axis == 0 else [v, u], "want": [want, expected_b],
                                         "got": list(decoded) if decoded else list(rgb)})
                if j == 0 and jj == 0:
                    got_row = decoded[0] if decoded else None
        sequence.append(got_row)
    runs = []
    for value in sequence:
        if runs and runs[-1][0] == value:
            runs[-1][1] += 1
        else:
            runs.append([value, 1])
    interior = runs[1:-1]
    steps = {runs[i + 1][0] - runs[i][0] for i in range(len(runs) - 1)
             if runs[i][0] is not None and runs[i + 1][0] is not None}
    return {"pixels": len(sequence), "mismatches": bad, "examples": examples,
            "visible_texels": [runs[0][0], runs[-1][0]] if runs else None,
            "interior_run_lengths": sorted({r[1] for r in interior}),
            "first_run": runs[0] if runs else None, "last_run": runs[-1] if runs else None,
            "steps": sorted(steps)}


def strip_cases(scene):
    """Every strip sprite on the pixel-perfect W, V, X and Y frames, with its decode spec."""
    cases = []
    for name, meta in scene["meta"].items():
        frame = meta["frame"]
        if frame not in ("hstrip", "hrow", "hmid", "vstrip", "vcol", "vmid") or meta["board"] not in "WVXY":
            continue
        x, y, w, h = meta["rect"]
        horizontal = frame.startswith("h")
        length = 1100 if frame in ("hstrip", "hrow", "vstrip", "vcol") else 700
        thickness = 4 if frame in ("hstrip", "vstrip") else 1
        m = (w if horizontal else h) / length
        offset = 100 if frame in ("hmid", "vmid") else 0
        if horizontal:
            base = {"hstrip": [100, 120, 140, 160], "hrow": 120, "hmid": 140}[frame]
        else:
            base = {"vstrip": [180, 190, 200, 210], "vcol": 190, "vmid": 200}[frame]
        cases.append({"name": name, "board": meta["board"], "axis": 0 if horizontal else 1,
                      "start": x if horizontal else y, "cross": int(y if horizontal else x),
                      "thickness": thickness, "length": length, "m": int(round(m)), "flip": any(meta["flip"]),
                      "offset": offset, "base": base})
    return cases


def cut_positions(case):
    """Screen positions of the 512-texel span cuts, measured from the low (screen-start) edge."""
    m, length = case["m"], case["length"]
    return [case["start"] + 512 * k * m for k in range(1, math.ceil(length / 512))]


# ------------------------------------------------- exact image predictions --

def source_of(run, manifest, frame_name, ballast):
    fid, f = next((k, v) for k, v in manifest["frames"].items() if v["name"] == frame_name)
    path = ballast if frame_name == "ballast" else f["path"]
    src = image(run / "art" / path)
    rect = f["region"] or [0, 0, src.width, src.height]
    return src, rect


def orientation_texel(o, a, b, w, h):
    """Screen offset (a, b) inside the on-screen rectangle -> source pixel of a w x h sprite."""
    return {"NONE": (a, b), "FX": (w - 1 - a, b), "MIR": (w - 1 - a, b), "FY": (a, h - 1 - b),
            "FXY": (w - 1 - a, h - 1 - b), "R180": (w - 1 - a, h - 1 - b), "R90": (w - 1 - b, a),
            "R270": (b, h - 1 - a), "R90FX": (b, a)}[o]


def check_exact(img, scale, items, background, exclude=()):
    """items: (label, src, rect, (left, top, W, H) logical, mapper(a, b) -> source px coords or None).
    Pixels whose source texel lies outside the display identity range are skipped, as are
    pixels inside `exclude` rectangles (logical px) that other sprites deliberately cover."""
    results = {}
    for label, src, rect, (left, top, W, H), mapper in items:
        # Pixel-perfect output is an exact integer replication of the reference frame.
        u0, v0 = scale * math.ceil(left - 0.5), scale * math.ceil(top - 0.5)
        bad = skipped = 0
        example = None
        for b in range(round(H * scale)):
            for a in range(round(W * scale)):
                if any(ex <= (u0 + a + 0.5) / scale < ex + ew and ey <= (v0 + b + 0.5) / scale < ey + eh
                       for ex, ey, ew, eh in exclude):
                    skipped += 1
                    continue
                sx, sy = mapper(a, b)
                r, g, bl, al = src.px(rect[0] + sx, rect[1] + sy)
                if al == 0:
                    want = background
                elif al == 255 and max(r, g, bl) <= IDENTITY_MAX:
                    want = (r, g, bl)
                else:
                    skipped += 1
                    continue
                got = img.rgb(u0 + a, v0 + b)
                if got != want:
                    bad += 1
                    example = example or {"pixel": [u0 + a, v0 + b], "want": want, "got": got}
        results[label] = {"mismatches": bad, "skipped_outside_identity": skipped, "example": example}
    return results


def upscale(m_x, m_y):
    return lambda a, b: (a // m_x, b // m_y)


def nine_mapper(w_px, h_px, border, density, ppu=16):
    """Exact nearest mapping for the 24 x 24 panel at integer border scale; stretched
    regions are uniform along their stretch axis, so any centre texel represents them."""
    left, right, top, bottom = (b * ppu / density for b in border)
    def axis(c, size, lo, hi, src_lo, src_hi, src_size, centre):
        if c < lo:
            return int(c * src_lo / lo)
        if c >= size - hi:
            return src_size - src_hi + int((c - (size - hi)) * src_hi / hi)
        return centre
    return lambda a, b: (axis(a, w_px, left, right, border[0], border[1], 24, 10),
                         axis(b, h_px, top, bottom, border[2], border[3], 24, 10))


def flip_mapper(mapper, w, h, fx, fy):
    return lambda a, b: mapper(w - 1 - a if fx else a, h - 1 - b if fy else b)


def composite_check(img, scale, sprites, region, background, entity_ids, reverse=False):
    """Painter-order prediction for opaque/binary-alpha sprites at exact placements:
    sort by (layer, order, stable entity ID), draw, compare every pixel of `region`.
    reverse=True draws in the opposite order: a negative control for discrimination."""
    x0, y0, w, h = region
    canvas = [[background] * w for _ in range(h)]
    for spr in sorted(sprites, key=lambda s: (s["layer"], s["order"], entity_ids[s["name"]]), reverse=reverse):
        left, top, W, H = spr["rect"]
        for b in range(H):
            for a in range(W):
                cx, cy = left + a - x0, top + b - y0
                if not (0 <= cx < w and 0 <= cy < h):
                    continue
                r, g, bl, al = spr["src"].px(spr["srect"][0] + a * spr["srect"][2] // W,
                                             spr["srect"][1] + b * spr["srect"][3] // H)
                if al == 255:
                    canvas[cy][cx] = (r, g, bl)
                elif al != 0:
                    canvas[cy][cx] = None
    bad = unknown = 0
    example = None
    for cy in range(h):
        for cx in range(w):
            want = canvas[cy][cx]
            if want is None or max(want) > IDENTITY_MAX:
                unknown += 1
                continue
            got = img.rgb(x0 + cx, y0 + cy)
            if got != want:
                bad += 1
                example = example or {"pixel": [x0 + cx, y0 + cy], "want": want, "got": got}
    return {"mismatches": bad, "unpredicted": unknown, "example": example}


# ------------------------------------------------- linear attribution --

def sprite_corners(meta):
    """World-space XY corners of a board sprite from its authored placement."""
    b = meta["board"]
    if meta["rect"] is not None:
        x, y, w, h = meta["rect"]
        return [(sc.wx(b, x), sc.wy(b, y)), (sc.wx(b, x + w), sc.wy(b, y)), (sc.wx(b, x + w), sc.wy(b, y + h)),
                (sc.wx(b, x), sc.wy(b, y + h))]
    cx, cy = sc.wx(b, meta["center"][0]), sc.wy(b, meta["center"][1])
    w, h = (v / sc.PPU for v in meta["size_px"])
    a = math.radians(meta["angle"])
    return [(cx + dx * math.cos(a) - dy * math.sin(a), cy + dx * math.sin(a) + dy * math.cos(a))
            for dx, dy in ((-w / 2, -h / 2), (w / 2, -h / 2), (w / 2, h / 2), (-w / 2, h / 2))]


def project(camera, size, point):
    """Screen position of a world XY point for an authored camera (ortho or pixel-perfect)."""
    spec = sc.CAMERAS[camera]
    bx, by = sc.BOARD_SPEC[spec["board"]][:2]
    cx, cy = bx + spec["offset"][0], by + spec["offset"][1]
    w, h = size
    proj = spec["projection"]
    if proj["kind"] == "orthographic":
        scale = h / proj["vertical_size"]
    else:
        ref = proj["reference_resolution"]
        scale = proj["pixels_per_unit"] * min(w // ref[0], h // ref[1])
    r = math.radians(spec["roll"])
    dx, dy = point[0] - cx, point[1] - cy
    rx, ry = dx * math.cos(r) + dy * math.sin(r), -dx * math.sin(r) + dy * math.cos(r)
    return w / 2 + rx * scale, h / 2 - ry * scale


def inside(poly, x, y):
    hit = False
    for (x1, y1), (x2, y2) in zip(poly, poly[1:] + poly[:1]):
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            hit = not hit
    return hit


def attribute(pixels, camera, size, scene, board, linear_only=True):
    """Assign each differing pixel to the (linear) sprites whose screen quad holds its centre."""
    quads = {n: [project(camera, size, c) for c in sprite_corners(m)] for n, m in scene["meta"].items()
             if m["board"] == board and (m["sampling"] == "linear" or not linear_only)
             and (m["rect"] is not None or m["center"] is not None) and ":label:" not in n
             and not n.endswith(":background")}
    owners, levels = {}, {}
    for x, y, d in pixels:
        names = [n for n, q in quads.items() if inside(q, x + 0.5, y + 0.5)]
        if not names:  # rim: replicated pixel-perfect coverage reaches up to 1.5 px past a 3x quad
            names = sorted({f"rim:{n}" for n, q in quads.items() for dx in (-1.5, 0, 1.5) for dy in (-1.5, 0, 1.5)
                            if inside(q, x + 0.5 + dx, y + 0.5 + dy)}) or ["no sprite"]
        key = "+".join(sorted(names))
        owners[key] = owners.get(key, 0) + 1
        levels[key] = max(levels.get(key, 0), d)
    return {k: {"pixels": owners[k], "max_level": levels[k]} for k in sorted(owners)}


# --------------------------------------------------------------- analysis --

def analyse(run):
    helper = json.loads((run / "helper.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    scene = json.loads((run / "scene-atlas.json").read_text(encoding="utf-8"))
    states = helper["states"]
    shots = lambda name: run / "states" / name  # noqa: E731
    report = {"run": run.name, "binary": {k: helper[k] for k in ("binary_source_commit", "binary_sha256_start",
                                                                 "binary_sha256_end", "binary_unchanged_every_call",
                                                                 "engine_calls", "screenshots",
                                                                 "failed_screenshots")}}
    fp = {k: v["fingerprint"] for k, v in states.items()}
    report["fingerprints"] = fp

    # ---- cooked pages
    pages = {}
    ballast_of = {fp["s0-r1"]: "ballast-v1.png", fp["s1-v2"]: "ballast-v2.png", fp["s5-v3"]: "ballast-v1.png"}
    for name, case in states["s8-render-failures"]["cases"].items():
        if "import" in case:
            ballast_of[case["import"]["asset"]["sha256"]] = "ballast-v1.png"
    for f, ballast in ballast_of.items():
        pages[f] = verify_page(run, f, manifest, ballast)
    report["pages"] = pages
    regions = {f: p["regions"] for f, p in pages.items()}
    moved = {}
    for label, f in (("v1->v2", fp["s1-v2"]), ("v1->v3", fp["s5-v3"])):
        a, b = regions[fp["s0-r1"]], regions[f]
        moved[label] = {"moved": sorted(k for k in a if a[k][:2] != b[k][:2]),
                        "unmoved": sorted(k for k in a if a[k][:2] == b[k][:2]),
                        "page_size": [pages[fp["s0-r1"]]["size"], pages[f]["size"]]}
    report["relocation"] = {k: {"moved_count": len(v["moved"]), "unmoved": v["unmoved"], "page_size": v["page_size"]}
                            for k, v in moved.items()}
    targets = ["hstrip", "hrow", "hmid", "vstrip", "vcol", "vmid", "card7", "panel", "probe", "odd", "halo", "solid",
               "wisp0", "orb", "label:STABLE FRAMES"]
    report["target_regions"] = {t: {"v1": regions[fp["s0-r1"]][t], "v2": regions[fp["s1-v2"]][t],
                                    "v3": regions[fp["s5-v3"]][t]} for t in targets}

    # ---- differential rendering
    board_masks = {"pp-B-320x180": {"ballast": (10, 12, 300, 10), "raw-card7": (10, 67, 36, 42)}}

    def compare_sets(a_dir, b_dir, names, masks=None):
        out = {}
        for n in names:
            out[n] = compare(shots(a_dir) / f"{n}.png", shots(b_dir) / f"{n}.png", (masks or {}).get(n))
        return out

    full = [f"{n}-{w}x{h}" for n, (w, h) in sc.FULL]
    view = [f"{n}-{w}x{h}" for n, (w, h) in sc.VIEW]
    subset = [f"{n}-{w}x{h}" for n, (w, h) in sc.SUBSET]
    small = [f"{n}-{w}x{h}" for n, (w, h) in sc.PP_SMALL]
    diff = {
        "repeat_s0_r1_r2": compare_sets("s0-r1", "s0-r2", full + view),
        "control_vs_atlas_v1": compare_sets("control", "s0-r1", full + view),
        "v2_vs_v1": compare_sets("s0-r1", "s1-v2", full, board_masks),
        "undo_vs_v1": compare_sets("s0-r1", "s2-undo", subset),
        "redo_vs_v2": compare_sets("s1-v2", "s3-redo", subset),
        "watch_revert_vs_v1": compare_sets("s0-r1", "s4-watch-revert", subset),
        "v3_vs_v1": compare_sets("s0-r1", "s5-v3", full, board_masks),
        "after_rejections_vs_v3": compare_sets("s5-v3", "s6-after-rejections", small),
        "source_free_vs_v3": compare_sets("s5-v3", "s9-source-free", full),
    }
    for name in ("render-fail-shrink-panel", "render-fail-drop-cardB"):
        diff[f"{name}_undone_vs_v3"] = compare_sets("s5-v3", f"s8-{name}-undone", small)
    diff["frame-on-ordinary_undone_vs_v3"] = compare_sets("s5-v3", "s8-frame-on-ordinary-undone", ["pp-S-320x180"])
    report["differential"] = diff
    summary = {}
    for k, v in diff.items():
        changed = {n: c for n, c in v.items() if not c["identical"]}
        summary[k] = {"compared": len(v), "identical": len(v) - len(changed),
                      "changed": {n: {"pixels": c.get("pixels"), "max_level": c.get("max_level"),
                                      "by_region": c.get("by_region")} for n, c in changed.items()}}
    report["differential_summary"] = summary
    linear = {}
    for label, (a_dir, b_dir) in {"control_vs_atlas_v1": ("control", "s0-r1"), "v2_vs_v1": ("s0-r1", "s1-v2"),
                                  "v3_vs_v1": ("s0-r1", "s5-v3")}.items():
        for shot in [n for n in full + view if n.split("-")[1] == "L"]:
            if not (shots(a_dir) / f"{shot}.png").exists() or not (shots(b_dir) / f"{shot}.png").exists():
                continue
            pixels = []
            compare(shots(a_dir) / f"{shot}.png", shots(b_dir) / f"{shot}.png", collect=pixels)
            if pixels:
                camera, dims = shot.rsplit("-", 1)
                size = tuple(int(v) for v in dims.split("x"))
                linear.setdefault(label, {})[shot] = attribute(pixels, camera, size, scene, "L")
    report["linear_attribution"] = linear
    # Ordinary orthographic at exactly 3x against pixel-perfect 3x (same projection scale).
    pp_vs_ortho = {}
    for board in ("S", "N"):
        pixels = []
        res = compare(shots("s0-r1") / f"pp-{board}-960x540.png", shots("s0-r1") / f"o3-{board}-960x540.png",
                      collect=pixels)
        if pixels:
            res["by_sprite"] = attribute(pixels, f"o3-{board}", (960, 540), scene, board, linear_only=False)
        pp_vs_ortho[board] = res
    report["pixel_perfect_vs_ortho3"] = pp_vs_ortho

    # ---- decoded strips across span cuts (pixel-perfect 1:1 frames)
    phase = manifest["strip"]["phase"]
    strips = {}
    for case in strip_cases(scene):
        png = shots("s0-r1") / {"W": "pp-W-1280x200.png", "V": "pp-V-320x1080.png", "X": "pp-X-1024x832.png",
                                "Y": "pp-Y-832x1024.png"}[case["board"]]
        img = image(png)
        res = check_strip(img, case["axis"], case["start"], case["cross"], case["thickness"], case["length"],
                          case["m"], case["flip"], case["offset"], case["base"], phase, (img.width, img.height))
        res["cuts_px"] = [c for c in cut_positions(case) if 0 <= c < (img.width if case["axis"] == 0 else img.height)]
        around = []
        for c in res["cuts_px"]:
            u = math.ceil(c - 0.5)
            pick = (lambda q: strip_texel(img.rgb(q, case["cross"]) if case["axis"] == 0
                                          else img.rgb(case["cross"], q), phase))
            around.append({"cut_px": c, "before": pick(u - 1)[0] if u >= 1 else None, "at": pick(u)[0]})
        res["around_cuts"] = around
        res["m"], res["flip"], res["start"] = case["m"], case["flip"], case["start"]
        strips[case["name"]] = res
    report["strips"] = strips
    report["strips_summary"] = {"cases": len(strips), "mismatching_cases": sorted(k for k, v in strips.items()
                                                                                 if v["mismatches"]),
                                "decoded_pixels": sum(v["pixels"] for v in strips.values())}

    # ---- exact predictions at aligned / tie placements
    bg = image(shots("s0-r1") / "pp-S-320x180.png").rgb(0, 0)
    ballast = "ballast-v1.png"
    exact = {}
    for shot in ("pp-S-320x180", "pp-S-960x540", "pp-B-320x180", "pp-B-960x540"):
        k = scale_of(shot)
        img = image(shots("s0-r1") / f"{shot}.png")
        items = []
        for name, meta in scene["meta"].items():
            if meta["board"] != shot[3] or meta["frame"] in (None, "solid", "ballast") or meta["rect"] is None:
                continue
            src, rect = source_of(run, manifest, meta["frame"], ballast)
            x, y, w, h = meta["rect"]
            if w % rect[2] or h % rect[3]:
                continue
            items.append((name, src, rect, (x, y, w, h), (lambda mx, my: upscale(mx, my))(w * k // rect[2],
                                                                                        h * k // rect[3])))
        exact[shot] = check_exact(img, k, items, bg)
    for shot in ("pp-T-320x180", "pp-T-960x540"):
        k = scale_of(shot)
        img = image(shots("s0-r1") / f"{shot}.png")
        items = []
        for name, meta in scene["meta"].items():
            if meta["board"] != "T" or meta["frame"] not in ("probe", "odd", "card3"):
                continue
            src, rect = source_of(run, manifest, meta["frame"], ballast)
            w, h = meta["size_px"]
            if "orientation" in meta:
                o, centre = meta["orientation"], meta["center"]
                W, H = (h, w) if o in ("R90", "R270", "R90FX") else (w, h)
                mapper = (lambda o_, w_, h_, m_, k_: lambda a, b: tuple(
                    c // m_ for c in orientation_texel(o_, a // k_, b // k_, w_, h_)))(o, w, h, w // rect[2], k)
                items.append((name, src, rect, (centre[0] - W / 2, centre[1] - H / 2, W, H), mapper))
            else:  # pivot cards: exact rectangles, some on half-pixel (tie) rows
                x, y, _, _ = meta["rect"]
                items.append((name, src, rect, (x, y, w, h), (lambda k_: lambda a, b: (a // k_, b // k_))(k)))
        markers = [tuple(m["rect"]) for n, m in scene["meta"].items() if n.startswith("T:pivot") and
                   n.endswith(":origin")]
        exact[shot] = check_exact(img, k, items, bg, exclude=markers)
    for shot in ("pp-N-320x180", "pp-N-960x540"):
        k = scale_of(shot)
        img = image(shots("s0-r1") / f"{shot}.png")
        src, rect = source_of(run, manifest, "panel", ballast)
        items = []
        for name, (fx, fy, density) in {"N:native": (0, 0, 16), "N:stretch": (0, 0, 16), "N:flipx": (1, 0, 16),
                                        "N:flipy": (0, 1, 16), "N:flipxy": (1, 1, 16), "N:density8": (0, 0, 8),
                                        "N:simple24": (0, 0, 16)}.items():
            meta = scene["meta"][name]
            x, y, w, h = meta["rect"]
            base = nine_mapper(w, h, sc.PANEL_BORDER, density)
            mapper = (lambda base_, w_, h_, fx_, fy_, k_: lambda a, b: flip_mapper(base_, w_, h_, fx_, fy_)(
                a // k_, b // k_))(base, w, h, fx, fy, k)
            items.append((name, src, rect, (x, y, w, h), mapper))
        for name, (fx, fy) in {"N:rot180": (1, 1), "N:mirror": (1, 0)}.items():
            meta = scene["meta"][name]
            w, h = meta["size_px"]
            centre = meta["center"]
            base = nine_mapper(w, h, sc.PANEL_BORDER, 16)
            mapper = (lambda base_, w_, h_, fx_, fy_, k_: lambda a, b: flip_mapper(base_, w_, h_, fx_, fy_)(
                a // k_, b // k_))(base, w, h, fx, fy, k)
            items.append((name, src, rect, (centre[0] - w / 2, centre[1] - h / 2, w, h), mapper))
        exact[shot] = check_exact(img, k, items, bg)
    report["exact"] = exact
    report["exact_summary"] = {shot: {"sprites": len(v), "mismatching": sorted(n for n, r in v.items()
                                                                              if r["mismatches"]),
                                      "skipped_pixels": sum(r["skipped_outside_identity"] for r in v.values())}
                               for shot, v in exact.items()}

    # ---- painter order
    order = {}
    img = image(shots("s0-r1") / "pp-O-320x180.png")
    groups = {"LAYER": ["O:layer-top", "O:layer-under"], "ORDER": ["O:order-top", "O:order-under"],
              "ID TIE": ["O:tie-first", "O:tie-second"], "Z IGNORED": ["O:z-near", "O:z-far-top"],
              "BATCH": [f"O:batch{k}" for k in range(5)], "WISP STACK": [f"O:wisp-stack{k}" for k in range(4)]}
    badge = image(run / "art/badge.png")
    for group, names in groups.items():
        sprites = []
        for n in names:
            meta = scene["meta"][n]
            if meta["frame"] is None:
                src, srect = badge, [0, 0, 16, 16]
            else:
                src, srect = source_of(run, manifest, meta["frame"], ballast)
            sprites.append({"name": n, "layer": meta["layer"], "order": meta["order"], "rect": meta["rect"],
                            "src": src, "srect": srect})
        xs = [s["rect"][0] for s in sprites] + [s["rect"][0] + s["rect"][2] for s in sprites]
        ys = [s["rect"][1] for s in sprites] + [s["rect"][1] + s["rect"][3] for s in sprites]
        region = (min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys))
        order[group] = composite_check(img, 1, sprites, region, bg, scene["ids"])
        order[group]["reversed_order_control_mismatches"] = composite_check(img, 1, sprites, region, bg,
                                                                            scene["ids"], reverse=True)["mismatches"]
    report["painter_order"] = order

    # ---- linear fringes (hidden RGB under zero alpha must not tint edges)
    fringe = {}
    rects = {"pp-L": {"halo3x": (160, 19, 72, 72), "halo2.5x-light": (240, 21, 60, 60), "orb-light": (84, 23, 64, 64)},
             "pp-O": {"halo-on-light": (284, 85, 24, 24), "orb-on-light": (250, 81, 32, 32)}}
    for shot in ("pp-L-320x180", "pp-L-960x540", "pp-O-320x180"):
        k = scale_of(shot)
        img = image(shots("s0-r1") / f"{shot}.png")
        res = {}
        for label, (x, y, w, h) in rects[shot[:4]].items():
            green = magenta = 0
            for v in range(y * k, (y + h) * k):
                for u in range(x * k, (x + w) * k):
                    r, g, b = img.rgb(u, v)
                    green += g > r + 2  # every authored colour here has G < R
                    magenta += label.startswith("orb") and b > g + 2  # orb and light plate have B < G
            res[label] = {"green_tinted": green, "magenta_tinted": magenta}
        fringe[shot] = res
    report["fringe"] = fringe

    # ---- raw rectangle after relocation: what the authored pixel rectangle now shows
    raw = {}
    for state in ("s1-v2", "s5-v3"):
        rect = helper["atlas"]["raw_rect_card7_v1"]
        p = pages[fp[state]]["padding"]
        overl = [n for n, r in regions[fp[state]].items()
                 if r[0] - p < rect[0] + rect[2] and rect[0] < r[0] + r[2] + p and r[1] - p < rect[1] + rect[3]
                 and rect[1] < r[1] + r[3] + p]
        raw[state] = {"rect": rect, "frames_or_extrusion_overlapping_now": overl,
                      "card7_now": regions[fp[state]]["card7"]}
    report["raw_rect"] = raw

    # ---- watcher, failures, Undo/Redo
    report["watch"] = summarize_watch(states, fp)
    report["rejections"] = {k: {"exit": v["exit"], "authored_unchanged": v["authored_unchanged"],
                                "message": v["stderr"].splitlines()[-1] if v["stderr"] else ""}
                            for k, v in states["s6-after-rejections"]["imports"].items()}
    report["render_failures"] = {
        k: {"import_exit": v.get("import", {}).get("exit"),
            "validate": v.get("validate"),
            "shots": {n: {"exit": s["exit"], "written": s["written"], "message": s["stderr"].splitlines()[-1]
                          if s["stderr"] else ""} for n, s in v["shots"].items()},
            "fingerprint_after_undo": v.get("fingerprint_after_undo")}
        for k, v in states["s8-render-failures"]["cases"].items()}
    report["undo_redo"] = {"history_after_v2": helper["history"]["after_v2"], "final": helper["history"]["final"],
                           "undo": states["s2-undo"]["rpc"], "redo": states["s3-redo"]["rpc"],
                           "fp_undo_equals_v1": fp["s2-undo"] == fp["s0-r1"],
                           "fp_redo_equals_v2": fp["s3-redo"] == fp["s1-v2"]}
    report["authored"] = {k: v.get("authored") for k, v in states.items()}
    report["source_free"] = {"removed": states["s9-source-free"]["removed"],
                             "reimport_exit": states["s9-source-free"]["reimport_without_sources"]["exit"],
                             "reimport_message": states["s9-source-free"]["reimport_without_sources"]["stderr"]}
    report["limits"] = {k: {"exit": v["exit"], "size": v.get("details", {}).get("atlas", {}).get("size"),
                            "message": v["stderr"]} for k, v in helper["limits"].items()}
    report["imports"] = {k: {"changed": v["changed"], "cache_hit": v["cache_hit"], "fingerprint": v["asset"]["sha256"],
                             "page": [v["details"]["width"], v["details"]["height"]],
                             "mip_levels": v["details"]["mip_levels"], "format": v["details"]["format"]}
                         for k, v in helper["imports"].items()}
    return report


def summarize_watch(states, fp):
    out = {}
    for key in ("s4-watch-revert", "s7-watch-fail"):
        w = states[key]["watch"]
        events = [e for e in w["events"] if e.get("event") == "assets"]
        imports = [i for e in events for i in e["report"]["imports"]]
        diags = [d for e in events for d in e["report"]["diagnostics"]]
        cleared = [c for e in events for c in e["report"]["cleared"]]
        snapshots = [e["assets"] for e in events]
        out[key] = {"exit": w["exit"], "stderr": w["stderr"], "unapplied_actions": w["unapplied_actions"],
                    "imports": [{"changed": i["changed"], "cache_hit": i["cache_hit"],
                                 "fingerprint": i["asset"]["sha256"]} for i in imports],
                    "diagnostics": diags, "cleared": cleared,
                    "runtime_errors": [e["runtime_error"] for e in events if e["runtime_error"]],
                    "runtime_changes": [e["runtime_changes"] for e in events],
                    "atlas_runtime_versions": [
                        {k: (v["fingerprint"][:12], v["generation"]) for k, v in snap.items() if v["kind"] == "texture"}
                        for snap in snapshots],
                    "final_fingerprint": fp[key]}
    return out


def cross_run(a, b):
    files_a = {p.relative_to(a): sha(p) for p in sorted((a / "states").rglob("*.png"))}
    files_b = {p.relative_to(b): sha(p) for p in sorted((b / "states").rglob("*.png"))}
    pages_a = {p.name: sha(p) for p in (a / "atlas/.incant/cache/textures").glob("*.ktx2")}
    pages_b = {p.name: sha(p) for p in (b / "atlas/.incant/cache/textures").glob("*.ktx2")}
    return {"frames": len(files_a), "same_names": files_a.keys() == files_b.keys(),
            "identical": sum(files_a[k] == files_b.get(k) for k in files_a),
            "different": sorted(str(k) for k in files_a if files_a[k] != files_b.get(k)),
            "cooked_pages_identical": pages_a == pages_b, "cooked_pages": len(pages_a)}


def analyse_extra(run):
    """Board Z of atlas_extra.py: large packed origin (x = 6266) and minified linear frames."""
    helper = json.loads((run / "helper.json").read_text(encoding="utf-8"))
    scene = json.loads((run / "scene-extra.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    phase, bases = manifest["strip"]["phase"], manifest["strip"]["h_base"]
    sc.BOARD_SPEC["Z"] = (0.0, 0.0, 1280, 240)
    sc.CAMERAS.update({
        "pp-Z": {"board": "Z", "offset": (0, 0), "roll": 0, "projection": sc.pixel((1280, 240))},
        "of-Z": {"board": "Z", "offset": (0.0371, -0.0123), "roll": 0, "projection": sc.ortho(240 / 18)},
        "rl-Z": {"board": "Z", "offset": (0, 0), "roll": 3, "projection": sc.ortho(20.0)}})
    report = {"binary": {k: helper.get(k) for k in ("binary_sha256_start", "binary_sha256_end",
                                                   "binary_unchanged_every_call", "engine_calls")},
              "wide_page": helper["imports"]["wide"]["details"]["atlas"]}
    img = image(run / "states/atlas-r1/pp-Z-1280x240.png")
    strips = {}
    for name, meta in scene["meta"].items():
        if "strip_offset" not in meta or meta["sampling"] != "nearest":
            continue
        x, y, w, h = meta["rect"]
        m = int(round(h))
        res = check_strip(img, 0, x, int(y), 1, 1040, m, meta["flip"][0], meta["strip_offset"],
                          bases[meta["strip_row"]], phase, (img.width, img.height))
        res["cuts_px"] = [x + 512 * m]
        strips[name] = res
    report["strips"] = strips
    diff = {}
    for shot in ("pp-Z-1280x240", "of-Z-1280x240", "rl-Z-1280x320"):
        pixels = []
        res = compare(run / f"states/control/{shot}.png", run / f"states/atlas-r1/{shot}.png", collect=pixels)
        res["repeat_identical"] = sha(run / f"states/atlas-r1/{shot}.png") == sha(run / f"states/atlas-r2/{shot}.png")
        if pixels:
            camera, dims = shot.rsplit("-", 1)
            res["by_sprite"] = attribute(pixels, camera, tuple(int(v) for v in dims.split("x")), scene, "Z",
                                         linear_only=False)
        res.pop("histogram", None)
        diff[shot] = res
    report["control_vs_atlas"] = diff
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("second", type=Path, nargs="?")
    parser.add_argument("--extra", action="store_true", help="RUN is an atlas_extra.py output")
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit(f"refusing to overwrite {args.out}")
    if args.extra:
        report = analyse_extra(args.run.resolve())
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=1, default=list) + "\n", encoding="utf-8")
        print(json.dumps({"strips": {k: (v["mismatches"], v["pixels"], v["first_run"], v["interior_run_lengths"])
                                     for k, v in report["strips"].items()},
                          "control_vs_atlas": {k: {x: v.get(x) for x in ("identical", "pixels", "max_level",
                                                                          "repeat_identical", "by_sprite")}
                                               for k, v in report["control_vs_atlas"].items()}}, indent=1))
        return
    report = analyse(args.run.resolve())
    if args.second:
        report["cross_run"] = cross_run(args.run.resolve(), args.second.resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=1, default=list) + "\n", encoding="utf-8")
    brief = {"pages_pass": {k[:12]: v["pass"] for k, v in report["pages"].items()},
             "relocation": {k: v["moved_count"] for k, v in report["relocation"].items()},
             "differential": {k: f"{v['identical']}/{v['compared']} identical; changed "
                                 f"{ {n: (c['pixels'], c['max_level']) for n, c in v['changed'].items()} }"
                              for k, v in report["differential_summary"].items()},
             "strips": report["strips_summary"],
             "exact": {k: v["mismatching"] for k, v in report["exact_summary"].items()},
             "painter_order": {k: v["mismatches"] for k, v in report["painter_order"].items()},
             "fringe": report["fringe"], "cross_run": report.get("cross_run", {}).get("different")}
    print(json.dumps(brief, indent=1, default=list))


if __name__ == "__main__":
    main()
