#!/usr/bin/env python3
"""Independent checks of a handoff 0039 run, from PNG, KTX2 and JSON records only.

  * differential: tile frames against the ordinary-sprite control, and every later
    state against the state it must reproduce (byte equality, then pixel statistics);
  * model: every engine pixel against tile_model's prediction from the scene
    description and the original source images, with ties classified;
  * integer upscale: each 3x pixel-perfect frame against its own 1x frame;
  * cooked pages: every packed, extruded and empty texel of each cooked atlas
    version against the authored sources, and frame relocation between versions;
  * workflow records: imports, rejections, history, validation, fingerprints,
    binary hash checks and the capture ledger.

Nothing here calls the engine. Usage (repo root):
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_analyse.py RUN OUT.json
"""

import hashlib
import json
import math
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
ROOT = HELPERS.parents[2]
sys.path.insert(0, str(HELPERS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import pngio  # noqa: E402
import tile_model as tm  # noqa: E402
import tile_scene as ts  # noqa: E402

GUARD = (230, 0, 230)
# (label, state folder, reference folders in priority order, expectation)
PAIRS = [
    ("tile vs ordinary-sprite control, s0", "s0--tile", ["s0--control-s0"], "identical"),
    ("tile vs ordinary-sprite control, s1", "s1--tile", ["s1--control-s1"], "identical"),
    ("fresh-process repeat of s0", "s0-repeat--tile", ["s0--tile"], "identical"),
    ("s1 command edit against s0", "s1--tile", ["s0--tile"], "A and B change, C identical"),
    ("Undo of the s1 edit against s0", "s2-undo--tile", ["s0--tile"], "identical"),
    ("Redo of the s1 edit against s1", "s3-redo--tile", ["s1--tile"], "identical"),
    ("accepted v2 repack against before", "s4-repack--tile", ["s3-redo--tile", "s0--tile"], "identical"),
    ("rejected missing-frame reimport against before", "s5-rejected--tile", ["s4-repack--tile"], "identical"),
    ("Undo of the repack import (v1 page)", "s6-import-undo--tile", ["s4-repack--tile"], "identical"),
    ("Redo of the repack import (v2 page)", "s7-import-redo--tile", ["s4-repack--tile"], "identical"),
    ("sources removed against cached baseline", "s8-source-free--tile", ["s4-repack--tile"], "identical"),
]
MODEL = [("s0--tile", "s0", ["pp1-A", "pp1-B", "pp1-C", "of25-A", "of25-B", "of25-C", "o3-C"]),
         ("s1--tile", "s1", ["pp1-A", "pp1-B", "of25-A", "of25-B"])]
_IMAGES = {}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def image(path):
    key = str(path)
    if key not in _IMAGES:
        _IMAGES[key] = pngio.read(path)
    return _IMAGES[key]


def shots(run, folder):
    """view -> (PNG path, capture record entry) for one state folder."""
    record = json.loads((run / "states" / folder / "record.json").read_text(encoding="utf-8"))
    out = {}
    for view, entry in record["shots"].items():
        w, h = ts.VIEWS[view][1]
        out[view] = (run / "states" / folder / f"{view}-{w}x{h}.png", entry)
    return out


def pixel_diff(a, b):
    ia, ib = image(a), image(b)
    count, worst, box = 0, 0, None
    for y in range(ia.height):
        if ia.rows[y] == ib.rows[y]:
            continue
        for x in range(ia.width):
            pa, pb = ia.px(x, y), ib.px(x, y)
            if pa != pb:
                count += 1
                worst = max(worst, max(abs(p - q) for p, q in zip(pa, pb)))
                box = [x, y, x, y] if box is None else [min(box[0], x), min(box[1], y),
                                                        max(box[2], x), max(box[3], y)]
    return {"pixels": count, "max_level": worst, "bbox": box}


def differential(run):
    out = []
    for label, folder, refs, expectation in PAIRS:
        current = shots(run, folder)
        references = [shots(run, r) for r in refs]
        views = {}
        for view, (path, entry) in current.items():
            ref = next(((r[view], name) for r, name in zip(references, refs) if view in r), None)
            if ref is None:
                continue
            (rpath, rentry), rname = ref
            same = entry["sha256"] == rentry["sha256"]
            views[view] = {"reference": f"{rname}/{rpath.name}", "identical": same}
            if not same:
                views[view].update(pixel_diff(path, rpath))
        identical = sum(v["identical"] for v in views.values())
        out.append({"label": label, "state": folder, "expectation": expectation, "views": views,
                    "identical": identical, "compared": len(views)})
    return out


def magenta_scan(run):
    """Guard magenta surrounds every source region and hides under every zero-alpha
    texel; no rendered pixel anywhere may be magenta-like."""
    frames, hits = 0, {}
    for png in sorted((run / "states").glob("*/*.png")):
        img = image(png)
        frames += 1
        count = 0
        for row in img.rows:
            for x in range(img.width):
                r, g, b = row[4 * x], row[4 * x + 1], row[4 * x + 2]
                count += r >= 200 and g <= 40 and b >= 200
        if count:
            hits[str(png.relative_to(run))] = count
    return {"frames": frames, "frames_with_magenta": hits}


def upscale(run):
    out = {}
    for folder in sorted(p.name for p in (run / "states").iterdir()):
        s = shots(run, folder)
        for board in "ABC":
            if f"pp1-{board}" in s and f"pp3-{board}" in s:
                one, three = image(s[f"pp1-{board}"][0]), image(s[f"pp3-{board}"][0])
                bad = sum(1 for y in range(three.height) for x in range(three.width)
                          if three.px(x, y) != one.px(x // 3, y // 3))
                out[f"{folder}/{board}"] = bad
    return {"frames": len(out), "mismatching_pixels": sum(out.values()), "per_frame": out}


# ------------------------------------------------------------------ model --

def tie_detail(prepared, x, y):
    """Which item edge or texel boundary lies nearest to this pixel centre."""
    best = None
    for p in prepared:
        bx0, by0, bx1, by1 = p.bbox
        if not (bx0 - 1 <= x <= bx1 + 1 and by0 - 1 <= y <= by1 + 1):
            continue
        u, v = p.local(x, y)
        margin = p.edge_margin(u, v)
        if margin < -tm.DELTA:
            continue
        candidates = [("coverage edge", abs(margin))]
        if margin > -tm.DELTA:
            tu, tv = (u - p.rect[0]) * p.su, (p.rect[3] - v) * p.sv
            candidates.append(("texel boundary x", abs(tu - round(tu)) / math.hypot(*p.du)))
            candidates.append(("texel boundary y", abs(tv - round(tv)) / math.hypot(*p.dv)))
        kind, distance = min(candidates, key=lambda c: c[1])
        if best is None or distance < best["distance_px"]:
            best = {"item": p.item["name"], "kind": kind, "distance_px": round(distance, 7)}
    return best


def model_checks(run):
    results = []
    for folder, state, views in MODEL:
        description, textures = tm.load(run, state)
        s = shots(run, folder)
        items = tm.draw_items(description, textures)
        for view in views:
            engine = image(s[view][0])
            (w, h), colours, ambiguous = tm.predict(description, textures, view)
            camera, (width, height) = description["views"][view]
            screen, _ = tm.view_affine(description["cameras"][camera], width, height)
            prepared = [tm.Prepared(item, screen) for item in items]
            entry = {"state": folder, "view": view, "pixels": w * h, "ambiguous": len(ambiguous),
                     "exact_matches": 0, "ties_as_rule": 0, "ties_other_side": 0, "unexplained": 0,
                     "guard_pixels": 0, "other_side": [], "unexplained_examples": []}
            for y in range(h):
                row = engine.rows[y]
                for x in range(w):
                    got = (row[4 * x], row[4 * x + 1], row[4 * x + 2])
                    entry["guard_pixels"] += got == GUARD
                    index = y * w + x
                    if index in ambiguous:
                        if got == colours[index]:
                            entry["ties_as_rule"] += 1
                        elif got in ambiguous[index]:
                            entry["ties_other_side"] += 1
                            entry["other_side"].append({"pixel": [x, y], "engine": got, "rule": colours[index],
                                                        **(tie_detail(prepared, x + 0.5, y + 0.5) or {})})
                        else:
                            entry["unexplained"] += 1
                            entry["unexplained_examples"].append({"pixel": [x, y], "engine": got,
                                                                  "model": colours[index]})
                    elif got == colours[index]:
                        entry["exact_matches"] += 1
                    else:
                        entry["unexplained"] += 1
                        if len(entry["unexplained_examples"]) < 20:
                            entry["unexplained_examples"].append({"pixel": [x, y], "engine": got,
                                                                  "model": colours[index]})
            results.append(entry)
    return results


# ----------------------------------------------------------- cooked pages --

def read_ktx2(path):
    data = Path(path).read_bytes()
    if data[:12] != b"\xabKTX 20\xbb\r\n\x1a\n":
        raise ValueError(f"{path}: not KTX2")
    vk, _, w, h, _, _, _, levels, sc = struct.unpack("<9I", data[12:48])
    off, length, _ = struct.unpack("<3Q", data[80:104])
    if vk != 43 or sc != 0 or length != w * h * 4:
        raise ValueError(f"{path}: expected uncompressed R8G8B8A8_SRGB level 0, got vk={vk} sc={sc}")
    return {"width": w, "height": h, "levels": levels, "level0": data[off:off + length]}


def by_hash(folder, pattern):
    return {sha(p): p for p in sorted(folder.glob(pattern))}


def verify_pages(run):
    cache = run / "tile/.incant/cache/textures"
    art = run / "art"
    descriptors = by_hash(art, "*.atlas.json")
    images = by_hash(art, "*.png")
    pages = {}
    for meta_path in sorted(cache.glob("*.json")):
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        if "atlas" not in meta:
            continue
        descriptor_path = descriptors[meta["dependency"]["sha256"]]
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        sources = {d["path"]: images[d["sha256"]] for d in meta["dependencies"]}
        ktx = read_ktx2(cache / f"{meta['content_sha256']}.ktx2")
        layout = meta["atlas"]
        w, h = layout["size"]
        pad = layout["padding"]
        page = ktx["level0"]
        covered = bytearray(w * h)
        content_bad = extrusion_bad = overlap = 0
        for fid, frame in layout["frames"].items():
            authored = descriptor["frames"][fid]
            src = image(sources[f"art/{authored['path']}"])
            rx, ry, rw, rh = authored["region"] or [0, 0, src.width, src.height]
            x, y, fw, fh = frame["region"]
            assert (fw, fh) == (rw, rh) and frame["name"] == authored["name"]
            for dy in range(-pad, fh + pad):
                for dx in range(-pad, fw + pad):
                    i = (y + dy) * w + (x + dx)
                    overlap += covered[i]
                    covered[i] = 1
                    want = src.px(rx + min(max(dx, 0), fw - 1), ry + min(max(dy, 0), fh - 1))
                    if tuple(page[4 * i:4 * i + 4]) != want:
                        inner = 0 <= dx < fw and 0 <= dy < fh
                        content_bad += inner
                        extrusion_bad += not inner
        empty_bad = sum(1 for i in range(w * h) if not covered[i] and page[4 * i:4 * i + 4] != b"\0\0\0\0")
        pages[meta["fingerprint"]] = {
            "descriptor": descriptor_path.name, "ballast_source": sources["art/ballast.png"].name,
            "content_sha256": meta["content_sha256"], "size": [w, h], "padding": pad,
            "ktx_size": [ktx["width"], ktx["height"]], "mip_levels": ktx["levels"], "frames": len(layout["frames"]),
            "regions": {f["name"]: f["region"] for f in layout["frames"].values()},
            "content_mismatches": content_bad, "extrusion_mismatches": extrusion_bad,
            "overlapping_texels": overlap, "nonzero_empty_texels": empty_bad,
            "pass": [ktx["width"], ktx["height"]] == [w, h] and not (content_bad or extrusion_bad or overlap or
                                                                     empty_bad)}
    return pages


def relocation(pages, a, b):
    ra, rb = pages[a]["regions"], pages[b]["regions"]
    moved = {n: [ra[n], rb[n]] for n in ra if n in rb and ra[n][:2] != rb[n][:2]}
    return {"from": a, "to": b, "moved": len(moved), "common": len(set(ra) & set(rb)), "regions": moved}


# --------------------------------------------------------- workflow records --

def workflow(run):
    build = json.loads((run / "build.json").read_text(encoding="utf-8"))
    atlas = build["tile"]["ids"]["atlas"]
    steps = []
    for path in sorted((run / "steps").glob("*.json")):
        r = json.loads(path.read_text(encoding="utf-8"))
        entry = {"step": path.stem, "authored_changed": r["authored_before"] != r["authored_after"],
                 "atlas_fingerprint": [r["fingerprints_before"][atlas], r["fingerprints_after"][atlas]],
                 "history_length": len(r["history_after"]),
                 "new_cache_files": sorted(set(r["cache_after"]) - set(r["cache_before"]))}
        if "edit" in r:
            entry["transaction_id"] = r["edit"]["transaction_id"]
            entry["commands"] = [{"op": c["op"], "entity_id": c["entity_id"], "component": c["component"]}
                                 for c in r["edit"]["request"]["params"]["commands"]]
        if "history" in r:
            entry["history"] = r["history"]
        for key in ("import", "reimport_without_sources"):
            if key in r:
                imp = r[key]
                lines = imp["stderr"].splitlines()
                entry[key] = {"exit": imp["exit"], "first_line": lines[0] if lines else "",
                              "reference_lines": len(lines) - 1 if imp["exit"] else 0,
                              "authored_unchanged": imp["authored_unchanged"]}
                if imp["exit"] == 0:
                    entry[key].update(changed=imp["changed"], cache_hit=imp["cache_hit"],
                                      diagnostics=imp["diagnostics"])
        if "validate" in r:
            entry["validate"] = {"exit": r["validate"]["exit"], "valid": r["validate"]["reply"]["valid"],
                                 "diagnostics": len(r["validate"]["reply"]["diagnostics"])}
        if "removed" in r:
            entry["removed"] = r["removed"]
        steps.append(entry)
    return {"atlas_asset": atlas, "v1_fingerprint": build["tile"]["fingerprints"][atlas],
            "build_validate": {k: build[k]["validate"]["exit"] for k in ("tile", "control-s0", "control-s1")},
            "art_matches_committed": build["art_matches_committed"], "steps": steps}


def engine_calls(run):
    expected = json.loads((ROOT / "artifacts/tools/binary.json").read_text(encoding="utf-8"))["sha256"]
    calls = [json.loads(line) for line in (run.parent / "engine-calls.jsonl").read_text().splitlines()]
    mine = [c for c in calls if any(f"/{run.name}/" in a for a in c["argv"])]
    kinds = {}
    for c in mine:
        kinds[c["argv"][0]] = kinds.get(c["argv"][0], 0) + 1
    return {"calls": len(mine), "by_command": kinds,
            "all_hash_checked": all(c["sha256_before"] == c["sha256_after"] == expected for c in calls),
            "all_calls_any_run": len(calls), "nonzero_exits": [c["label"] for c in mine if c["exit"]]}


def ledger(run):
    entries = [json.loads(line) for line in (run.parent / "ledger.jsonl").read_text().splitlines()]
    mine = [e for e in entries if e["run"] == run.name]
    return {"frames": sum(e["frames"] for e in mine), "raw_bytes": sum(e["raw"] for e in mine),
            "packet_frames": sum(e["frames"] for e in entries), "packet_raw_bytes": sum(e["raw"] for e in entries)}


def main():
    run, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2])
    if out.exists():
        raise SystemExit(f"refusing to overwrite {out}")
    pages = verify_pages(run)
    work = workflow(run)
    v1 = work["v1_fingerprint"]
    v2 = next(s["atlas_fingerprint"][1] for s in work["steps"] if s["step"].endswith("repack"))
    rejected = next(s for s in work["steps"] if s["step"].endswith("reject"))["new_cache_files"]
    result = {"run": run.name, "differential": differential(run), "upscale": upscale(run),
              "magenta": magenta_scan(run),
              "model": model_checks(run), "pages": pages, "relocation_v1_v2": relocation(pages, v1, v2),
              "rejected_page_files": rejected, "workflow": work, "engine": engine_calls(run),
              "ledger": ledger(run)}
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    summary = {"pairs": {p["label"]: f"{p['identical']}/{p['compared']}" for p in result["differential"]},
               "upscale_mismatches": result["upscale"]["mismatching_pixels"],
               "magenta": result["magenta"],
               "model": {f"{m['state']}:{m['view']}": [m["exact_matches"], m["ambiguous"], m["ties_other_side"],
                                                       m["unexplained"]] for m in result["model"]},
               "pages": {k[:12]: v["pass"] for k, v in pages.items()},
               "moved": result["relocation_v1_v2"]["moved"], "engine": result["engine"], "ledger": result["ledger"]}
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
