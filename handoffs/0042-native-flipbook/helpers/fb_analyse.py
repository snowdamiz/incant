#!/usr/bin/env python3
"""CPU analysis of one handoff 0042 run. Standard library only; no engine calls.

  reports      every sidecar flipbook state (frame, playing, completed) against the
               closed-form contract model, plus an exact-Fraction cross-check for
               uncontrolled sequences; binding/topology generations
  moments      animated frames against independently authored static controls
               (whole frame, pixel-perfect and orthographic)
  crops        each static-position animated entity at every other captured tick
               against a control moment in which every dynamic entity touching
               the crop shows the same frame
  steadiness   pixels that change between captures must lie inside the union of
               animated/moving entities' screen quads (2 px dilation)
  identity     the frame each entity shows, read from its unique interior colour,
               against the model and the sidecar report
  magenta      hidden zero-alpha RGB or the sheet's opaque guard reaching the screen
  repeat       subset (fewer synchronisations) and source-free captures against main
  atlas        decoded cooked level-0 page: packed origins, content, extrusion

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/fb_analyse.py RUN OUT.json
"""

import hashlib
import json
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
sys.path.insert(0, str(HELPERS))
import fb_model as fm  # noqa: E402
import fb_png  # noqa: E402
import fb_scene as fs  # noqa: E402

_IMAGES = {}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def image(run, capture, tick):
    path = run / "captures" / capture / f"tick-{tick:05d}.png"
    if path not in _IMAGES:
        _IMAGES[path] = fb_png.read(path)
    return _IMAGES[path]


def report(run, capture):
    return json.loads((run / "captures" / capture / "report.json").read_text(encoding="utf-8"))


def diff_pixels(a, b):
    out = []
    for y in range(a.height):
        if a.rows[y] == b.rows[y]:
            continue
        for x in range(a.width):
            if a.px(x, y) != b.px(x, y):
                out.append((x, y))
    return out


# ----------------------------------------------------------------- reports --

def check_reports(run, sc):
    anims = [i for i in sc.items if i["kind"] == "anim"]
    by_hex = {fs.ulid_hex(fs.eid(sc, i["name"])): i for i in anims}
    out = {}
    for capture in ("main", "ortho", "subset", "subset-source-free"):
        rep = report(run, capture)
        ticks = [c["tick"] for c in rep["captures"]]
        model = fs.expected_states(sc, ticks)
        mismatches, checked, fraction = [], 0, 0
        generations = set()
        for c in rep["captures"]:
            generations.add((c["binding_generation"], c["topology_generation"]))
            seen = set()
            for f in c["flipbooks"]:
                item = by_hex[f["entity"]]
                seen.add(item["name"])
                got = (f["frame"], f["playing"], f["completed"])
                want = model[item["name"]][c["tick"]]
                checked += 1
                if got != tuple(want):
                    mismatches.append({"tick": c["tick"], "entity": item["name"], "report": got, "model": want})
                if not item["controls"] and item["playing"]:
                    fraction += 1
                    if tuple(fs.fraction_check(item, c["tick"])) != got:
                        mismatches.append({"tick": c["tick"], "entity": item["name"], "report": got,
                                           "fraction": fs.fraction_check(item, c["tick"])})
            missing = sorted({i["name"] for i in anims} - seen)
            if missing:
                mismatches.append({"tick": c["tick"], "missing": missing})
        out[capture] = {"adapter": rep["adapter"], "tick_rate": rep["tick_rate"], "ticks": ticks,
                        "states_checked": checked, "fraction_checked": fraction, "mismatches": mismatches,
                        "generations": sorted(generations), "scope": rep["scope"]}
    return out


def state_table(run, sc):
    """Per controlled/once/numeric entity: tick -> [frame name, playing, completed] from the report."""
    rep = report(run, "main")
    names = {fs.ulid_hex(fs.eid(sc, i["name"])): i for i in sc.items if i["kind"] == "anim"}
    table = {}
    for c in rep["captures"]:
        for f in c["flipbooks"]:
            item = names[f["entity"]]
            if item["name"][0] in "CE" or item["name"].startswith("D3"):
                table.setdefault(item["name"], {})[c["tick"]] = [item["frames"][f["frame"]], f["playing"],
                                                                 f["completed"]]
    return table


# ----------------------------------------------------------------- moments --

def moments(run):
    out = {}
    for camera, capture, ticks in (("pp", "main", fs.MOMENTS), ("ortho", "ortho", fs.ORTHO_TICKS)):
        for tick in ticks:
            animated = run / "captures" / capture / f"tick-{tick:05d}.png"
            control = run / "captures" / f"ctl-t{tick:05d}-{camera}" / "tick-00000.png"
            diff = diff_pixels(image(run, capture, tick), image(run, f"ctl-t{tick:05d}-{camera}", 0))
            out[f"{camera}-t{tick}"] = {"animated_sha256": sha(animated), "control_sha256": sha(control),
                                        "file_identical": sha(animated) == sha(control),
                                        "differing_pixels": len(diff), "first": diff[:16]}
    return out


# ------------------------------------------------------------------- crops --

def dynamic_items(sc):
    moving = {i["name"] for i in sc.items if i["v"] is not None}
    names = set()
    for item in sc.items:
        chain, node = [], item
        while node:
            chain.append(node["name"])
            node = sc.by_name[node["parent"]] if node["parent"] else None
        if item["kind"] == "anim" or (item["kind"] in ("sprite",) and moving & set(chain)):
            names.add(item["name"])
    return names, moving


def moves(sc, name):
    node = sc.by_name[name]
    while node:
        if node["v"] is not None:
            return True
        node = sc.by_name[node["parent"]] if node["parent"] else None
    return False


def crops(run, sc):
    w, h = fs.VIEW
    camera = fm.Camera(sc, "camera-pp", w, h)
    dynamic, _ = dynamic_items(sc)
    ticks = fs.MAIN_TICKS
    model = fs.expected_states(sc, sorted(set(ticks) | set(fs.MOMENTS)))
    quads = {t: fm.quads(sc, t, camera) for t in sorted(set(ticks) | set(fs.MOMENTS))}
    results, uncovered = [], []
    for item in sc.items:
        if item["kind"] != "anim" or moves(sc, item["name"]):
            continue
        crop = fm.bbox(quads[0][item["name"]]["screen"], w, h, pad=2)
        x0, y0, x1, y1 = crop
        for tick in ticks:
            if tick in fs.MOMENTS:
                continue
            chosen = None
            for moment in fs.MOMENTS:
                # Every dynamic entity touching the crop at either tick must match:
                # same frame for fixed entities; moving entities must stay clear.
                touching = [n for n in dynamic
                            if any(_overlaps(fm.bbox(quads[t][n]["screen"], w, h, pad=2), crop)
                                   for t in (tick, moment))]
                if all(not moves(sc, n) and (sc.by_name[n]["kind"] != "anim"
                                             or model[n][moment][0] == model[n][tick][0]) for n in touching):
                    chosen = moment
                    break
            if chosen is None:
                uncovered.append({"entity": item["name"], "tick": tick})
                continue
            a = image(run, "main", tick).crop(x0, y0, x1 - x0, y1 - y0)
            b = image(run, f"ctl-t{chosen:05d}-pp", 0).crop(x0, y0, x1 - x0, y1 - y0)
            results.append({"entity": item["name"], "tick": tick, "moment": chosen,
                            "frame": item["frames"][model[item["name"]][tick][0]], "crop": [x0, y0, x1, y1],
                            "identical": a.key() == b.key()})
    return {"compared": len(results), "identical": sum(r["identical"] for r in results),
            "different": [r for r in results if not r["identical"]], "uncovered": uncovered,
            "entities": sorted({r["entity"] for r in results})}


def _overlaps(a, b):
    return a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]


# -------------------------------------------------------------- steadiness --

def steadiness(run, sc):
    w, h = fs.VIEW
    camera = fm.Camera(sc, "camera-pp", w, h)
    dynamic, _ = dynamic_items(sc)
    union = set()
    for tick in fs.MAIN_TICKS:
        q = fm.quads(sc, tick, camera)
        for name in dynamic:
            if tick == 0 or moves(sc, name):
                union |= fm.mask(q[name]["screen"], w, h, dilate=2)
    base = image(run, "main", 0)
    changed, outside = set(), set()
    for tick in fs.MAIN_TICKS[1:]:
        for p in diff_pixels(base, image(run, "main", tick)):
            changed.add(p)
            if p not in union:
                outside.add(p)
    return {"captures": len(fs.MAIN_TICKS), "dynamic_mask_pixels": len(union),
            "static_pixels": w * h - len(union), "changed_pixels": len(changed),
            "changed_outside_dynamic_mask": len(outside), "outside": sorted(outside)[:32]}


# ---------------------------------------------------------------- identity --

def identity(run, sc, manifest):
    w, h = fs.VIEW
    camera = fm.Camera(sc, "camera-pp", w, h)
    colours = {name: tuple(f["interior"]) + (255,) for name, f in manifest["frames"].items()}
    rep = {c["tick"]: {f["entity"]: f for f in c["flipbooks"]} for c in report(run, "main")["captures"]}
    model = fs.expected_states(sc, fs.MAIN_TICKS)
    results, skipped = [], []
    for tick in fs.MAIN_TICKS:
        img = image(run, "main", tick)
        q = fm.quads(sc, tick, camera)
        for item in sc.items:
            if item["kind"] != "anim":
                continue
            if item["tint"][:3] != [1.0, 1.0, 1.0]:
                skipped.append({"entity": item["name"], "reason": "tinted"})
                continue
            poly = q[item["name"]]["screen"]
            x0, y0, x1, y1 = fm.bbox(poly, w, h)
            counts = dict.fromkeys(colours, 0)
            for y in range(y0, y1):
                for x in range(x0, x1):
                    if fm.inside(poly, x + 0.5, y + 0.5):
                        p = img.px(x, y)
                        for name, colour in colours.items():
                            if p == colour:
                                counts[name] += 1
            best = max(counts, key=counts.get)
            others = sum(v for k, v in counts.items() if k != best)
            expected = item["frames"][model[item["name"]][tick][0]]
            reported = item["frames"][rep[tick][fs.ulid_hex(fs.eid(sc, item["name"]))]["frame"]]
            results.append({"tick": tick, "entity": item["name"], "identified": best if counts[best] else None,
                            "pixels": counts[best], "other_frame_pixels": others, "expected": expected,
                            "reported": reported})
    hidden = [r for r in results if r["identified"] is None]
    visible = [r for r in results if r["identified"] is not None]
    agree = [r for r in visible if r["identified"] == r["expected"] == r["reported"]]
    return {"checked": len(visible), "agree": len(agree),
            "disagree": [r for r in visible if r not in agree],
            "no_interior_visible": sorted({r["entity"] for r in hidden}),
            "no_interior_visible_count": len(hidden),
            "hidden_report_equals_model": all(r["expected"] == r["reported"] for r in hidden),
            "min_pixels": min(r["pixels"] for r in agree) if agree else 0,
            "max_other_frame_pixels": max(r["other_frame_pixels"] for r in results),
            "skipped": sorted({s["entity"] for s in skipped})}


# ----------------------------------------------------------------- magenta --

def magenta(run):
    out = {}
    for folder in sorted((run / "captures").iterdir()):
        for png in sorted(folder.glob("*.png")):
            img = fb_png.read(png) if png not in _IMAGES else _IMAGES[png]
            count = 0
            for row in img.rows:
                for i in range(0, len(row), 4):
                    if row[i] >= 200 and row[i + 1] <= 40 and row[i + 2] >= 200:
                        count += 1
            out[f"{folder.name}/{png.name}"] = count
    return {"images": len(out), "with_magenta": {k: v for k, v in out.items() if v}}


# ------------------------------------------------------------------ repeat --

def repeat(run):
    out = {}
    main = {c["tick"]: c for c in report(run, "main")["captures"]}
    subset = {c["tick"]: c for c in report(run, "subset")["captures"]}
    free = {c["tick"]: c for c in report(run, "subset-source-free")["captures"]}
    for tick in fs.SUBSET_TICKS:
        paths = {name: run / "captures" / name / f"tick-{tick:05d}.png" for name in ("main", "subset",
                                                                                     "subset-source-free")}
        out[tick] = {"subset_equals_main_png": sha(paths["subset"]) == sha(paths["main"]),
                     "source_free_equals_subset_png": sha(paths["subset-source-free"]) == sha(paths["subset"]),
                     "subset_equals_main_report": subset[tick] == main[tick],
                     "source_free_equals_subset_report": free[tick] == subset[tick]}
    return {"ticks": out, "all": all(all(v.values()) for v in out.values())}


# ------------------------------------------------------------------- atlas --

def read_ktx2(path):
    data = Path(path).read_bytes()
    if data[:12] != b"\xabKTX 20\xbb\r\n\x1a\n":
        raise ValueError(f"{path}: not KTX2")
    vk, _, w, h, _, _, _, levels, sc = struct.unpack("<9I", data[12:48])
    off, length, _ = struct.unpack("<3Q", data[80:104])
    if vk != 43 or sc != 0 or length != w * h * 4:
        raise ValueError(f"{path}: expected uncompressed R8G8B8A8_SRGB level 0, got vk={vk} sc={sc}")
    return {"width": w, "height": h, "levels": levels, "level0": data[off:off + length]}


def atlas(run):
    cache = run / "anim/.incant/cache/textures"
    art = run / "art"
    for meta_path in sorted(cache.glob("*.json")):
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        if "atlas" in meta:
            break
    else:
        raise SystemExit("no atlas page in the cache")
    layout = meta["atlas"]
    ktx = read_ktx2(cache / f"{meta['content_sha256']}.ktx2")
    w, h = layout["size"]
    pad = layout["padding"]
    page = ktx["level0"]
    covered = bytearray(w * h)
    bad = {"content": 0, "extrusion": 0, "overlap": 0, "empty": 0}
    frames = {}
    for fid, frame in layout["frames"].items():
        src = fb_png.read(art / f"{frame['name']}.png")
        x, y, fw, fh = frame["region"]
        assert (fw, fh) == (src.width, src.height)
        frames[frame["name"]] = {"id": fid, "region": frame["region"], "origin_nonzero": x > 0 and y > 0}
        for py in range(y - pad, y + fh + pad):
            for px in range(x - pad, x + fw + pad):
                i = py * w + px
                if covered[i]:
                    bad["overlap"] += 1
                covered[i] = 1
                sx, sy = min(max(px - x, 0), fw - 1), min(max(py - y, 0), fh - 1)
                want = src.px(sx, sy)
                got = tuple(page[4 * i:4 * i + 4])
                inside_frame = x <= px < x + fw and y <= py < y + fh
                if got != want:
                    bad["content" if inside_frame else "extrusion"] += 1
    for i in range(w * h):
        if not covered[i] and page[4 * i + 3] != 0:
            bad["empty"] += 1
    return {"fingerprint": meta["fingerprint"], "ktx2_sha256": meta["content_sha256"], "size": [w, h],
            "padding": pad, "levels": ktx["levels"], "frames": frames, "mismatches": bad,
            "all_origins_nonzero": all(f["origin_nonzero"] for f in frames.values())}


# ------------------------------------------------------------------- model --

def unmodeled(sc, name):
    """Entities whose pixels the coarse CPU model does not predict exactly."""
    node = sc.by_name[name]
    if node.get("sampling") == "linear" or node.get("mode", {}).get("kind") == "nine_slice" \
            or node.get("tint", [1.0] * 4) != [1.0, 1.0, 1.0, 1.0]:
        return True
    while node:
        if node["r"] != 0.0 or abs(node["s"][0]) != abs(node["s"][1]):
            return True
        node = sc.by_name[node["parent"]] if node["parent"] else None
    return False


def model_check(run, sc, manifest):
    """Independent nearest-sampling model from the written contract and the source
    PNGs (not the cooked atlas): plates, tilemap, simple nearest sprites with flips,
    a parent reflection, sheet regions, 1x-4x and moving sprites. Linear, nine-slice,
    rotated, sheared and tinted entities are masked (2 px) and not predicted."""
    w, h = fs.VIEW
    camera = fm.Camera(sc, "camera-pp", w, h)
    art = fm.load_art(run / "art")
    masked_names = sorted(i["name"] for i in sc.items if i["kind"] in ("sprite", "anim") and unmodeled(sc, i["name"]))
    # The renderer's clear colour is not part of the sprite contract: read it once
    # from a corner outside every plate and require it to be uniform there.
    background = image(run, "main", 0).px(0, 0)[:3]
    plates = [fm.quads(sc, 0, camera)[f"plate-{b}"]["screen"] for b in "ABCDE"]
    gap = [(x, y) for y in range(h) for x in range(w) if not any(fm.inside(p, x + 0.5, y + 0.5) for p in plates)]
    out = {}
    for tick in fs.MAIN_TICKS:
        q = fm.quads(sc, tick, camera)
        masked = set()
        for name in masked_names:
            masked |= fm.mask(q[name]["screen"], w, h, dilate=2)
        rows = fm.preview(sc, tick, camera, art, manifest, background)
        img = image(run, "main", tick)
        bad = []
        for y in range(h):
            for x in range(w):
                if (x, y) not in masked and tuple(rows[y][4 * x:4 * x + 4]) != img.px(x, y):
                    bad.append((x, y))
        out[tick] = {"compared": w * h - len(masked), "mismatched": len(bad), "first": bad[:16]}
    gap_uniform = all(image(run, "main", 0).px(x, y)[:3] == background for x, y in gap)
    return {"masked_entities": masked_names, "clear_colour": list(background), "gap_pixels": len(gap),
            "gap_uniform_at_tick0": gap_uniform, "ticks": out,
            "compared": sum(v["compared"] for v in out.values()),
            "mismatched": sum(v["mismatched"] for v in out.values())}


# ------------------------------------------------------------ cross checks --

def cross_texture(run, sc):
    """Same frames from the atlas and from the ordinary sheet at the same configuration."""
    w, h = fs.VIEW
    camera = fm.Camera(sc, "camera-pp", w, h)
    q = fm.quads(sc, 0, camera)
    pairs = [("A1-nearest", "A6-sheet"), ("A2-linear", "A16-sheet-linear")]
    out = {}
    for a, b in pairs:
        ba, bb = fm.bbox(q[a]["screen"], w, h, 1), fm.bbox(q[b]["screen"], w, h, 1)
        if (ba[2] - ba[0], ba[3] - ba[1]) != (bb[2] - bb[0], bb[3] - bb[1]):
            out[f"{a}/{b}"] = "different crop sizes"
            continue
        same = []
        for tick in fs.MAIN_TICKS:
            img = image(run, "main", tick)
            ca = img.crop(ba[0], ba[1], ba[2] - ba[0], ba[3] - ba[1])
            cb = img.crop(bb[0], bb[1], bb[2] - bb[0], bb[3] - bb[1])
            same.append(ca.key() == cb.key())
        out[f"{a}/{b}"] = {"identical_ticks": sum(same), "ticks": len(same)}
    return out


def main():
    run, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2])
    if out.exists():
        raise SystemExit(f"refusing to overwrite {out}")
    sc = fs.build()
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    analysis = {
        "run": run.name,
        "reports": check_reports(run, sc),
        "moments": moments(run),
        "crops": crops(run, sc),
        "steadiness": steadiness(run, sc),
        "identity": identity(run, sc, manifest),
        "magenta": magenta(run),
        "repeat": repeat(run),
        "atlas": atlas(run),
        "cross_texture": cross_texture(run, sc),
        "model": model_check(run, sc, manifest),
        "states": state_table(run, sc),
    }
    m = analysis["moments"]
    analysis["summary"] = {
        "report_mismatches": sum(len(v["mismatches"]) for v in analysis["reports"].values()),
        "report_states_checked": sum(v["states_checked"] for v in analysis["reports"].values()),
        "moments_identical": sum(v["differing_pixels"] == 0 for v in m.values()), "moments": len(m),
        "crops_identical": analysis["crops"]["identical"], "crops": analysis["crops"]["compared"],
        "changed_outside_dynamic_mask": analysis["steadiness"]["changed_outside_dynamic_mask"],
        "identity_agree": analysis["identity"]["agree"], "identity_checked": analysis["identity"]["checked"],
        "magenta_images": len(analysis["magenta"]["with_magenta"]),
        "repeat_all_identical": analysis["repeat"]["all"],
        "atlas_mismatches": analysis["atlas"]["mismatches"],
        "model_compared": analysis["model"]["compared"], "model_mismatched": analysis["model"]["mismatched"],
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(analysis, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(analysis["summary"], indent=1))


if __name__ == "__main__":
    main()
