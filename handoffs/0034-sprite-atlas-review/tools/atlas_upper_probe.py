#!/usr/bin/env python3
"""Upper-span tie probe for handoff 0034: does an atlas frame inherit the shared limit?

Handoff 0032 (b9eca69) measured exact-tie failures at span texels 236-511 from 20x on
ordinary sprites. This probe draws the main atlas's 1100 x 1 strip frames (hrow, vcol)
at 36x, 40x, 48x and 56x with exact-tie placements, windows covering span texels 236 to
511 of the first span, in one 1920 x 1080 pixel-perfect frame per axis. Rows are 4 px
across, so the magnification applies along the strip only. The same entities are drawn
in a control project from hstrip.png / vstrip.png with explicit rectangles; atlas and
control frames are compared byte for byte and every pixel is decoded against the
documented nearest rule. Four frames in total, frozen binary, hash-checked calls.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_upper_probe.py OUT_DIR
"""

import argparse
import json
import math
import shutil
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(ROOT / "handoffs/0032-sprite-rendering/tools"))
import atlas_analyse as an  # noqa: E402
import atlas_lookdev as lk  # noqa: E402
import atlas_scene as sc  # noqa: E402

W, H = 1920, 1080
sc.BOARD_SPEC["U"] = (0.0, 0.0, W, H)
sc.BOARD_SPEC["UV"] = (500.0, 500.0, W, H)
sc.CAMERAS["pp-U"] = {"board": "U", "offset": (0, 0), "roll": 0, "projection": sc.pixel((W, H))}
sc.CAMERAS["pp-UV"] = {"board": "UV", "offset": (0, 0), "roll": 0, "projection": sc.pixel((W, H))}
PLAN = [("pp-U", (W, H)), ("pp-UV", (W, H))]
MAGS = (36, 40, 48, 56)
FIRST, LAST = 236, 511


def windows(m, along):
    span = along // m - 1
    return [a for a in range(FIRST, LAST + 1, span)]


def build(project, kind, manifest, textures):
    s, cams = scene(kind, manifest, textures)
    lk.create_entities(project, s, f"Upper-span tie probe, {kind} (handoff 0034)")
    return s, cams


def scene(kind, manifest, textures):
    by_name = {f["name"]: (fid, f) for fid, f in manifest["frames"].items()}
    if kind == "atlas":
        source = lambda n: (textures["atlas"], {"frame": by_name[n][0]})  # noqa: E731
    else:
        source = lambda n: (textures[by_name[n][1]["path"]], by_name[n][1]["region"])  # noqa: E731
    s = sc.Scene(sc.Binding(kind, textures, manifest))
    for board in ("U", "UV"):
        s.sprite(f"{board}:background", board, None, (-16, -16, W + 32, H + 32), tint=sc.tint_of(sc.BG),
                 layer=-100, texture=source("solid"))
    rows, cross = [], 4
    for m in MAGS:
        for a in windows(m, W):
            rows.append(("U", m, a, cross))
            cross += 6
    cols, cross = [], 4
    for m in MAGS:
        for a in windows(m, H):
            cols.append(("UV", m, a, cross))
            cross += 6
    for board, m, a, c in rows:
        start = 0.5 - a * m  # texel a begins on a pixel centre: every texel boundary is an exact tie
        s.sprite(f"U:{m}x:{a}", board, "hrow", (start, c, 1100 * m, 4), pivot=(0, 1), texture=source("hrow"))
        s.meta[f"U:{m}x:{a}"].update(m=m, window=a, axis=0, cross=c, start=start)
    for board, m, a, c in cols:
        start = 0.5 - a * m
        s.sprite(f"UV:{m}x:{a}", board, "vcol", (c, start, 4, 1100 * m), pivot=(0, 1), texture=source("vcol"))
        s.meta[f"UV:{m}x:{a}"].update(m=m, window=a, axis=1, cross=c, start=start)
    cams = {}
    for name, _ in PLAN:
        spec = sc.CAMERAS[name]
        bx, by = sc.BOARD_SPEC[spec["board"]][:2]
        value = {"fov_degrees": sc.FOV, "near": sc.NEAR, "far": sc.FAR, "projection": spec["projection"]}
        cams[name] = s.entity(f"camera:{name}", {"Camera": value, "Transform": sc.transform((bx, by, sc.EYE_Z))})
    return s, cams


def decode(img, s, phase, board):
    out = {}
    for name, meta in s.meta.items():
        if "window" not in meta or meta["board"] != board:
            continue
        m, start, axis, cross = meta["m"], meta["start"], meta["axis"], meta["cross"]
        base = 120 if axis == 0 else 190
        along = img.width if axis == 0 else img.height
        ties = tie_bad = other_bad = 0
        examples = []
        for u in range(along):
            t = (u + 0.5 - start) / m
            if not 0 <= t < 1100:
                continue
            want = an.expected_index(t, 1100, m, False)
            exact_tie = t == math.floor(t)
            ties += exact_tie
            for c in range(4):
                rgb = img.rgb(u, cross + c) if axis == 0 else img.rgb(cross + c, u)
                got = an.strip_texel(rgb, phase)
                if got != (want, base):
                    if exact_tie:
                        tie_bad += 1
                    else:
                        other_bad += 1
                    if len(examples) < 4:
                        examples.append({"px": u, "want": want, "got": got[0] if got else list(rgb),
                                         "span_texel": want % 512, "tie": exact_tie})
        out[name] = {"m": m, "window": meta["window"], "tie_pixel_columns": ties, "tie_mismatched_pixels": tie_bad,
                     "other_mismatched_pixels": other_bad, "examples": examples}
    return out


def column_stats(dirs):
    """Exact per-column tie statistics (centre row of each 4 px strip) for probe directories."""
    res = {}
    for d in dirs:
        manifest = json.loads((d / "art/manifest.json").read_text(encoding="utf-8"))
        phase = manifest["strip"]["phase"]
        s, _ = scene("atlas", manifest, {"atlas": "-", "hstrip.png": "-", "vstrip.png": "-", "sheet.png": "-"})
        decoded = json.loads((d / "decode.json").read_text(encoding="utf-8"))
        centres = decoded.get("board_centres") or {"U": [0.0, 0.0], "UV": [500.0, 500.0]}
        for kind in ("atlas", "control"):
            for shot, board in (("pp-U-1920x1080", "U"), ("pp-UV-1920x1080", "UV")):
                img = an.image(d / f"states/{kind}/{shot}.png")
                for meta in s.meta.values():
                    if meta.get("board") != board or "window" not in meta:
                        continue
                    m, start, axis, cross = meta["m"], meta["start"], meta["axis"], meta["cross"]
                    key = f"{d.name}|{kind}|{'x' if axis == 0 else 'y'}|{m}x"
                    st = res.setdefault(key, {"run": d.name, "kind": kind, "screen_axis": "x" if axis == 0 else "y",
                                              "source_axis": "x" if axis == 0 else "y", "m": m,
                                              "board_centre": centres[board], "exact_ties": 0, "failed": 0,
                                              "failed_lower": 0, "failed_span_texels": [], "failed_below_58": [],
                                              "non_tie_mismatches": 0})
                    for u in range(img.width if axis == 0 else img.height):
                        t = (u + 0.5 - start) / m
                        if not 0 <= t < 1100:
                            continue
                        want = an.expected_index(t, 1100, m, False)
                        got = an.strip_texel(img.rgb(u, cross + 1) if axis == 0 else img.rgb(cross + 1, u), phase)[0]
                        if t == math.floor(t):
                            st["exact_ties"] += 1
                            if got != want:
                                st["failed"] += 1
                                st["failed_lower"] += got == want - 1
                                st["failed_span_texels"].append(want % 512)
                                if want % 512 < 58:
                                    st["failed_below_58"].append(want)
                        elif got != want:
                            st["non_tie_mismatches"] += 1
    for v in res.values():
        f = v.pop("failed_span_texels")
        v["failed_span_texel_range"] = [min(f), max(f)] if f else None
    return res


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--decode-only", action="store_true", help="re-decode an existing OUT_DIR's saved frames")
    parser.add_argument("--swap", action="store_true",
                        help="put the vertical board at the world origin and the horizontal one at (500, 500)")
    parser.add_argument("--stats", type=Path, nargs="+", metavar="PROBE_DIR",
                        help="write exact per-column tie statistics of decoded probe directories to OUT_DIR (a file)")
    args = parser.parse_args()
    if args.stats:
        if args.out.exists():
            raise SystemExit(f"refusing to overwrite {args.out}")
        args.out.write_text(json.dumps(column_stats([d.resolve() for d in args.stats]), indent=1) + "\n",
                            encoding="utf-8")
        return
    if args.swap:
        sc.BOARD_SPEC["U"], sc.BOARD_SPEC["UV"] = (500.0, 500.0, W, H), (0.0, 0.0, W, H)
    out = args.out.resolve()
    if args.decode_only:
        # A summary-naming slip (since fixed) wrote one run's summary under its last row
        # name; read that file when helper.json is absent. Its runs completed under the
        # per-call hash guard, which aborts on any binary change.
        source = out / "helper.json" if (out / "helper.json").exists() else out / "UV:56x:506"
        helper = json.loads(source.read_text(encoding="utf-8"))
        manifest = json.loads((out / "art/manifest.json").read_text(encoding="utf-8"))
        results = {}
        for kind in ("atlas", "control"):
            s, _ = scene(kind, manifest, {"atlas": "-", "hstrip.png": "-", "vstrip.png": "-", "sheet.png": "-"})
            results[kind] = {"shots": helper["results"][kind]["shots"], "decoded": {
                n: decode(an.image(out / "states" / kind / f"{n}-{w}x{h}.png"), s, manifest["strip"]["phase"],
                          sc.CAMERAS[n]["board"]) for n, (w, h) in PLAN}}
        base = {k: helper[k] for k in ("binary_sha256_start", "binary_sha256_end", "binary_unchanged_every_call",
                                       "engine_calls", "planned", "ledger_after", "board_centres") if k in helper}
        base["summary_source"] = source.name
        summarize(out, base, results, name="decode.json")
        return
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    frames, raw = lk.plan_cost([PLAN, PLAN])
    used = lk.ledger_used()
    if used[0] + frames > lk.PACKET_FRAMES or used[1] + raw > lk.PACKET_RAW:
        raise SystemExit(f"budget: needs {frames} frames / {raw / 2**20:.1f} MiB; used {used}")
    start = lk.verify_binary()
    out.mkdir(parents=True)
    lk.run([sys.executable, "-I", TOOLS / "atlas_art.py", out / "art"])
    art = out / "art"
    manifest = json.loads((art / "manifest.json").read_text(encoding="utf-8"))
    phase = manifest["strip"]["phase"]
    summary = {"binary_sha256_start": start, "planned": {"frames": frames, "raw_bytes": raw},
               "board_centres": {b: sc.BOARD_SPEC[b][:2] for b in ("U", "UV")}}
    results = {}
    for kind in ("atlas", "control"):
        project = lk.Project(out / kind, kind)
        (project.dir / "art").mkdir(parents=True)
        lk.engine("init", project.file, "--name", f"Upper-span probe {kind}", "--entities", "0")
        for name in lk.SOURCES:
            shutil.copyfile(art / name, project.dir / "art" / name)
        shutil.copyfile(art / "ballast-v1.png", project.dir / "art/ballast.png")
        if kind == "atlas":
            shutil.copyfile(art / "atlas-v1.atlas.json", project.dir / "art/atlas.atlas.json")
            textures = {"atlas": project.import_("art/atlas.atlas.json", usage="color")["asset"]["id"]}
        else:
            textures = {n: project.import_(f"art/{n}", usage="color")["asset"]["id"]
                        for n in ("hstrip.png", "vstrip.png", "sheet.png")}
        s, cams = build(project, kind, manifest, textures)
        shots = lk.Shooter(out, out.name, cams).shots(project, kind, PLAN)
        results[kind] = {"shots": shots, "decoded": {
            n: decode(an.image(out / "states" / kind / f"{n}-{w}x{h}.png"), s, phase, sc.CAMERAS[n]["board"])
            for n, (w, h) in PLAN}}
    summarize(out, summary, results)


def summarize(out, summary, results, name="helper.json"):
    same = {n: results["atlas"]["shots"][f"{n}-{w}x{h}"]["sha256"] == results["control"]["shots"][f"{n}-{w}x{h}"]["sha256"]
            for n, (w, h) in PLAN}
    totals = {}
    for kind in results:
        for shot, rows in results[kind]["decoded"].items():
            for r in rows.values():
                key = f"{kind}:{shot}:{r['m']}x"
                t = totals.setdefault(key, {"tie_columns": 0, "tie_mismatched_pixels": 0, "other_mismatched_pixels": 0,
                                            "failing_windows": []})
                t["tie_columns"] += r["tie_pixel_columns"]
                t["tie_mismatched_pixels"] += r["tie_mismatched_pixels"]
                t["other_mismatched_pixels"] += r["other_mismatched_pixels"]
                if r["tie_mismatched_pixels"] or r["other_mismatched_pixels"]:
                    t["failing_windows"].append({"window": r["window"], "examples": r["examples"]})
    summary.update(results=results, atlas_equals_control=same, totals=totals)
    if name == "helper.json":
        summary.update(binary_sha256_end=lk.verify_binary(), engine_calls=lk.CALLS["count"],
                       binary_unchanged_every_call=lk.CALLS["unchanged"], ledger_after=lk.ledger_used())
    if (out / name).exists():
        raise SystemExit(f"refusing to overwrite {out / name}")
    (out / name).write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"atlas_equals_control": same, "binary_unchanged": summary.get("binary_unchanged_every_call"),
                      "totals": {k: {x: v[x] for x in ("tie_columns", "tie_mismatched_pixels",
                                                        "other_mismatched_pixels")} | {
                          "failing_windows": [w["window"] for w in v["failing_windows"]]}
                          for k, v in totals.items()}, "ledger": summary.get("ledger_after")}, indent=1))


if __name__ == "__main__":
    main()
