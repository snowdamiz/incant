#!/usr/bin/env python3
"""Independent checks of an atlas_followup.py run, measured from PNG and KTX2 bytes.

Reuses atlas_analyse.py's page verifier, byte/pixel comparison, decoded strip, exact,
painter-order and fringe checks. Adds the follow-up's own comparisons, and compares
v1 and control frames against round one's preserved frames from the pre-followup
binary (the same scene, art and cameras).

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_followup_analyse.py RUN ROUND1_RUN OUT.json
"""

import argparse
import json
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import atlas_analyse as an  # noqa: E402
import atlas_scene as sc  # noqa: E402

TARGETS = ["hstrip", "hrow", "hmid", "vstrip", "vcol", "vmid", "card7", "panel", "probe", "odd", "halo", "orb",
           "ballast"]


def names(plan):
    return [f"{n}-{w}x{h}" for n, (w, h) in plan]


def summary_of(diff):
    out = {}
    for label, shots in diff.items():
        changed = {n: c for n, c in shots.items() if not c["identical"]}
        out[label] = {"compared": len(shots), "identical": len(shots) - len(changed),
                      "changed": {n: {"pixels": c.get("pixels"), "max_level": c.get("max_level"),
                                      "by_region": c.get("by_region")} for n, c in changed.items()}}
    return out


def unproject(camera, size, sx, sy):
    """World XY under an orthographic camera for screen point (sx, sy); also output px per logical px."""
    import math
    spec = sc.CAMERAS[camera]
    bx, by = sc.BOARD_SPEC[spec["board"]][:2]
    cx, cy = bx + spec["offset"][0], by + spec["offset"][1]
    w, h = size
    scale = h / spec["projection"]["vertical_size"]
    r = math.radians(spec["roll"])
    rx, ry = (sx - w / 2) / scale, (h / 2 - sy) / scale
    return cx + rx * math.cos(r) - ry * math.sin(r), cy + rx * math.sin(r) + ry * math.cos(r), scale / sc.PPU


def knife_edges(a, b, shot, scene, region_sizes, background):
    """Classify every pixel that differs between two frames of the same unrotated-sprite board
    seen through a rolled/fractional orthographic camera: the pixel centre's distance (in
    output px) to the nearest covering sprite's outer edge and to its nearest texel boundary.
    A changed pixel more than `TIE` px from both is reported as unexplained."""
    import math
    tie = 0.01
    pixels = []
    an.compare(a, b, collect=pixels)
    ia, ib = an.image(a), an.image(b)
    camera, dims = shot.rsplit("-", 1)
    size = tuple(int(v) for v in dims.split("x"))
    board = sc.CAMERAS[camera]["board"]
    sprites = {n: m for n, m in scene["meta"].items()
               if m["board"] == board and m["rect"] is not None and m["frame"] in region_sizes and ":label:" not in n
               and not n.endswith(":background")}
    out = {"changed": len(pixels), "edge_ties": 0, "texel_ties": 0, "unexplained": [], "max_edge_px": 0.0,
           "max_texel_px": 0.0, "background_inside": 0}
    for x, y, _ in pixels:
        wx_, wy_, k = unproject(camera, size, x + 0.5, y + 0.5)
        best = None
        for n, m in sprites.items():
            bx, by, bw, bh = m["rect"]
            t, u = (wx_ - sc.wx(board, bx)) * sc.PPU, (sc.wy(board, by) - wy_) * sc.PPU
            edge = min(t, bw - t, u, bh - u)
            sw, sh = region_sizes[m["frame"]]
            tx, ty = t * sw / bw, u * sh / bh
            texel = min(abs(tx - round(tx)) * bw / sw, abs(ty - round(ty)) * bh / sh)
            if best is None or edge > best[0]:
                best = (edge, texel, n)
        edge, texel, name = best
        edge_px, texel_px = abs(edge) * k, texel * k
        if edge_px <= tie:
            out["edge_ties"] += 1
            out["max_edge_px"] = max(out["max_edge_px"], edge_px)
        elif texel_px <= tie:
            out["texel_ties"] += 1
            out["max_texel_px"] = max(out["max_texel_px"], texel_px)
        else:
            out["unexplained"].append({"px": [x, y], "sprite": name, "edge_px": edge_px, "texel_px": texel_px,
                                       "a": ia.rgb(x, y), "b": ib.rgb(x, y)})
        if edge > 0 and edge_px > tie and ib.rgb(x, y) == background:
            out["background_inside"] += 1
    out["tie_tolerance_px"] = tie
    out["max_edge_px"], out["max_texel_px"] = round(out["max_edge_px"], 6), round(out["max_texel_px"], 6)
    return out


def analyse(run, round1):
    helper = json.loads((run / "helper.json").read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    scene = json.loads((run / "scene-atlas.json").read_text(encoding="utf-8"))
    states = helper["states"]
    fp = {k: v["fingerprint"] for k, v in states.items() if "fingerprint" in v}
    shots = lambda name: run / "states" / name  # noqa: E731
    report = {"run": run.name, "round1_run": round1.name,
              "binary": {k: helper[k] for k in ("binary", "binary_source_commit", "binary_sha256_start",
                                                "binary_sha256_end", "binary_unchanged_every_call", "engine_calls",
                                                "screenshots", "failed_screenshots", "complete")},
              "fingerprints": fp}

    # ---- cooked pages under the bounded skyline packer
    pages = {}
    for label, state, ballast in (("v1", "f0-v1-r1", "ballast-v1.png"), ("v2", "f1-v2", "ballast-v2.png"),
                                  ("v3", "f4-v3", "ballast-v1.png"), ("drop-unused", "f6-drop-unused",
                                                                      "ballast-v1.png")):
        pages[label] = an.verify_page(run, fp[state], manifest, ballast)
    report["pages"] = pages
    regions = {k: p["regions"] for k, p in pages.items()}
    report["relocation"] = {}
    for a, b in (("v1", "v2"), ("v1", "v3"), ("v3", "drop-unused")):
        common = [k for k in regions[a] if k in regions[b]]
        moved = [k for k in common if regions[a][k][:2] != regions[b][k][:2]]
        report["relocation"][f"{a}->{b}"] = {"moved": len(moved), "common": len(common),
                                            "unmoved": sorted(set(common) - set(moved)),
                                            "page_size": [pages[a]["size"], pages[b]["size"]],
                                            "only_in_first": sorted(set(regions[a]) - set(regions[b]))}
    report["target_regions"] = {t: {k: regions[k].get(t) for k in regions} for t in TARGETS}

    # ---- differential rendering
    full, subset, small = names(sc.FULL), names(sc.SUBSET), names(sc.PP_SMALL)
    meta = scene["meta"]
    board_b = {"pp-B-320x180": {"ballast": tuple(meta["B:ballast"]["rect"]),
                                "raw-card7": tuple(meta["B:raw-card7"]["rect"])}}
    badges = {"pp-O-320x180": {n: tuple(meta[n]["rect"]) for n in ("O:batch1", "O:batch3")}}

    def compare_sets(a, b, shot_names, masks=None, base=run):
        return {n: an.compare(base / "states" / a / f"{n}.png", shots(b) / f"{n}.png", (masks or {}).get(n))
                for n in shot_names}

    diff = {
        "repeat_r1_r2_fresh_processes": compare_sets("f0-v1-r1", "f0-v1-r2", subset),
        "control_vs_atlas_v1": compare_sets("control", "f0-v1-r1", full),
        "v2_vs_v1": compare_sets("f0-v1-r1", "f1-v2", full, board_b),
        "undo_vs_v1": compare_sets("f0-v1-r1", "f2-undo", subset),
        "redo_vs_v2": compare_sets("f1-v2", "f3-redo", subset),
        "v3_vs_v1": compare_sets("f0-v1-r1", "f4-v3", full, board_b),
        "after_rejections_vs_v3": compare_sets("f4-v3", "f5-after-rejections", small),
        "drop_unused_vs_v3": compare_sets("f4-v3", "f6-drop-unused", small, board_b),
        "watch_mixed_vs_v3": compare_sets("f4-v3", "f7-watch-mixed", small, badges),
        "badge_restored_vs_v3": compare_sets("f4-v3", "f7-badge-restored", small),
        "source_free_vs_v3": compare_sets("f4-v3", "f9-source-free", full),
        # Round one rendered the same scene on the pre-followup binary.
        "round1_v1_vs_followup_v1": compare_sets("s0-r1", "f0-v1-r1", full, board_b, base=round1),
        "round1_control_vs_followup_control": compare_sets("control", "control", full, base=round1),
    }
    report["differential"] = diff
    report["differential_summary"] = summary_of(diff)
    linear = {}
    for label, (a_base, a_dir, b_dir) in {"control_vs_atlas_v1": (run, "control", "f0-v1-r1"),
                                          "v2_vs_v1": (run, "f0-v1-r1", "f1-v2"),
                                          "v3_vs_v1": (run, "f0-v1-r1", "f4-v3"),
                                          "round1_v1_vs_followup_v1": (round1, "s0-r1", "f0-v1-r1")}.items():
        for shot in [n for n in full if n.split("-")[1] == "L"]:
            pixels = []
            an.compare(a_base / "states" / a_dir / f"{shot}.png", shots(b_dir) / f"{shot}.png", collect=pixels)
            if pixels:
                camera, dims = shot.rsplit("-", 1)
                linear.setdefault(label, {})[shot] = an.attribute(pixels, camera, tuple(
                    int(v) for v in dims.split("x")), scene, "L")
    report["linear_attribution"] = linear
    # Rolled views whose nearest pixels changed between builds: classify every changed pixel.
    region_sizes = {f["name"]: (f["region"] or [0, 0, 1200, 40])[2:] for f in manifest["frames"].values()}
    bg = an.image(shots("f0-v1-r1") / "pp-S-320x180.png").rgb(0, 0)
    knives = {}
    for label, kind_old, kind_new in (("control", "control", "control"), ("atlas", "s0-r1", "f0-v1-r1")):
        for shot in ("rl-W-1280x288", "rl-V-360x1080", "rl-B-640x360"):
            knives[f"{label}:{shot}"] = knife_edges(round1 / "states" / kind_old / f"{shot}.png",
                                                    shots(kind_new) / f"{shot}.png", shot, scene, region_sizes, bg)
    report["round1_rolled_view_changes"] = knives

    # ---- decoded checks on the follow-up v1 frames (64-texel spans, skyline page)
    report.update(an.decoded_checks(run, scene, manifest, shots("f0-v1-r1")))

    # ---- what the raw v1 rectangle addresses after each repack
    raw = {}
    rect = helper["atlas"]["raw_rect_card7_v1"]
    for label in ("v2", "v3", "drop-unused"):
        p = pages[label]["padding"]
        overlap = [n for n, r in regions[label].items()
                   if r[0] - p < rect[0] + rect[2] and rect[0] < r[0] + r[2] + p and r[1] - p < rect[1] + rect[3]
                   and rect[1] < r[1] + r[3] + p]
        raw[label] = {"rect": rect, "frames_or_extrusion_overlapping_now": overlap, "card7_now": regions[label]["card7"]}
    report["raw_rect"] = raw

    # ---- public workflow records
    report["rejections"] = {k: {"exit": v["exit"], "authored_unchanged": v.get("authored_unchanged"),
                                "revision_unchanged": v.get("revision_unchanged"),
                                "asset_fingerprints_unchanged": v.get("asset_fingerprints_unchanged"),
                                "new_cache_files": len(v.get("new_cache_files", [])),
                                "message": v["stderr"]} for k, v in helper["rejections"].items()}
    report["validations"] = {}
    for k, v in helper["validations"].items():
        reply = v["reply"] or {}
        diags = reply.get("diagnostics", [])
        codes = {}
        for d in diags:
            codes[f"{d['code']}:{d['severity']}"] = codes.get(f"{d['code']}:{d['severity']}", 0) + 1
        report["validations"][k] = {"exit": v["exit"], "valid": reply.get("valid"), "codes": codes,
                                    "first_error": next((d for d in diags if d["severity"] == "error"), None),
                                    "warnings": [d for d in diags if d["severity"] == "warning"],
                                    "stderr": v["stderr"]}
    w = states["f7-watch-mixed"]["watch"]
    events = [e for e in w["events"] if e.get("event") == "assets"]
    report["watch_mixed"] = {
        "exit": w["exit"], "unapplied_actions": w["unapplied_actions"],
        "imports": [{"path": i["asset"]["path"], "changed": i["changed"], "cache_hit": i["cache_hit"],
                     "fingerprint": i["asset"]["sha256"]} for e in events for i in e["report"]["imports"]],
        "diagnostics": [d for e in events for d in e["report"]["diagnostics"]],
        "cleared": [c for e in events for c in e["report"]["cleared"]],
        "runtime_errors": [e["runtime_error"] for e in events if e["runtime_error"]],
        "atlas_fingerprint_after": fp["f7-watch-mixed"], "authored_changed": states["f7-watch-mixed"]["authored_changed"]}
    report["undo_redo"] = {"v2": {"undo_equals_v1": fp["f2-undo"] == fp["f0-v1-r1"],
                                  "redo_equals_v2": fp["f3-redo"] == fp["f1-v2"]},
                           "drop-unused": {k: states["f6-drop-unused"][k][:16] for k in
                                           ("fingerprint", "fingerprint_after_undo", "fingerprint_after_redo",
                                            "fingerprint_final")},
                           "drop_unused_dropped": states["f6-drop-unused"]["dropped"]}
    report["frame_on_ordinary_render"] = {k: {"exit": v["exit"], "written": v["written"], "message": v["stderr"]}
                                          for k, v in states["f8-frame-on-ordinary"]["shots"].items()}
    report["source_free"] = {"removed": states["f9-source-free"]["removed"],
                             "reimport_exit": states["f9-source-free"]["reimport_without_sources"]["exit"],
                             "reimport_message": states["f9-source-free"]["reimport_without_sources"]["stderr"]}
    report["imports"] = {k: {"exit": v.get("exit"), "changed": v.get("changed"), "cache_hit": v.get("cache_hit"),
                             "fingerprint": v.get("asset", {}).get("sha256"),
                             "page": v.get("details", {}).get("atlas", {}).get("size")}
                         for k, v in helper["imports"].items()}
    report["history"] = [(h["description"], h["actor"]["actor"]) for h in helper["history"]]
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("round1", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit(f"refusing to overwrite {args.out}")
    report = analyse(args.run.resolve(), args.round1.resolve())
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=1, default=list) + "\n", encoding="utf-8")
    print(json.dumps({
        "pages_pass": {k: v["pass"] for k, v in report["pages"].items()},
        "pages": {k: v["size"] for k, v in report["pages"].items()},
        "relocation": {k: (v["moved"], v["common"]) for k, v in report["relocation"].items()},
        "differential": {k: f"{v['identical']}/{v['compared']} identical; changed "
                            f"{ {n: (c['pixels'], c['max_level'], c['by_region']) for n, c in v['changed'].items()} }"
                         for k, v in report["differential_summary"].items()},
        "strips": report["strips_summary"],
        "exact": {k: (v["sprites"], v["mismatching"]) for k, v in report["exact_summary"].items()},
        "painter_order": {k: (v["mismatches"], v["reversed_order_control_mismatches"])
                          for k, v in report["painter_order"].items()},
        "fringe": report["fringe"], "raw_rect": report["raw_rect"]}, indent=1, default=list))


if __name__ == "__main__":
    main()
