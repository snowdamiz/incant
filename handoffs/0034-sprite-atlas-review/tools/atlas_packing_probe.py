#!/usr/bin/env python3
"""Import-only packing probe for handoff 0034: page size versus maximum_size.

Imports the v1 descriptor (same frames, same sources) with several maximum sizes into
one new scratch project through the public `import` command and records each cooked
page size. No frames are rendered. The binary hash is checked around every call.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py ART_DIR OUT_DIR
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py --repro OUT_DIR
ART_DIR is a run's art/ directory (atlas_art.py output); OUT_DIR must not exist.
--repro builds the minimal three-frame reproduction of a rejected set that fits.
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py ART_DIR OUT_DIR --sweep
--sweep checks nested-bound monotonicity over a grid of maxima for both sets.
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py --residual OUT_DIR
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py ART_DIR OUT.json --feasible
--residual imports a four-frame set that fits 1217 x 1104; --feasible proves (engine-free)
that the 130-frame v1 set fits 1217 x 1104.
"""

import argparse
import json
import shutil
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import atlas_lookdev as lk  # noqa: E402

sys.path.insert(0, str(TOOLS.parents[2] / "handoffs/0032-sprite-rendering/tools"))
import pngio  # noqa: E402

MAXIMA = [(2048, 2048), (1204, 1170), (2048, 1170), (1300, 1200), (1600, 2600), (4096, 1024), (1204, 1148),
          (1204, 1147)]


def repro(out):
    """Three solid frames, padding 2. A layout inside 1204 x 1148 exists (wide at (2, 2),
    col at (2, 46), row at (10, 46)); the packer places row first and rejects the set."""
    project = lk.Project(out, "repro")
    (project.dir / "art").mkdir(parents=True)
    lk.engine("init", project.file, "--name", "Packing repro", "--entities", "0")
    sizes = {"wide": (1200, 40, (90, 60, 140, 255)), "row": (1100, 4, (200, 80, 80, 255)),
             "col": (4, 1100, (80, 200, 120, 255))}
    for name, (w, h, colour) in sizes.items():
        pngio.write_rgba(project.dir / "art" / f"{name}.png", w, h, [colour] * (w * h))
    ids = {"wide": "01JA34FR00000000000000R001", "row": "01JA34FR00000000000000R002",
           "col": "01JA34FR00000000000000R003"}
    rows = []
    for maximum in ([1204, 1148], [1204, 2048], [1204, 1200]):
        d = {"version": 1, "id": "01JA34AA00000000000000R001", "maximum_size": maximum, "padding": 2,
             "frames": {ids[n]: {"name": n, "path": f"{n}.png"} for n in sizes}}
        name = f"art/repro-{maximum[0]}x{maximum[1]}.atlas.json"
        (project.dir / name).write_text(json.dumps(d, indent=1), encoding="utf-8")
        e = project.import_(name, usage="color", expect_error=True)
        atlas = e.get("details", {}).get("atlas", {})
        rows.append({"maximum_size": maximum, "exit": e["exit"], "page": atlas.get("size"),
                     "regions": {f["name"]: f["region"] for f in atlas.get("frames", {}).values()},
                     "message": e["stderr"].splitlines()[-1] if e["stderr"] else ""})
    return {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
            "binary_sha256": lk.verify_binary(), "results": rows,
            "existing_layout_for_1204x1148": {"wide": [2, 2, 1200, 40], "col": [2, 46, 4, 1100],
                                              "row": [10, 46, 1100, 4]}}


SWEEP_W = [1204, 1221, 1222, 1240, 1300, 1600, 2048, 2730, 4096]
SWEEP_H = [1024, 1104, 1147, 1148, 1170, 1200, 1500, 2048, 3000]


def sweep(art, out):
    """Monotonicity of the chosen page under nested bounds, for the 130-frame v1 set and
    the three-frame reproduction: a fitting smaller bound must fit every containing
    bound with no larger page area. Bounds over 4,194,304 texels are rejected by
    validation and skipped here."""
    project = lk.Project(out, "sweep")
    (project.dir / "art").mkdir(parents=True)
    lk.engine("init", project.file, "--name", "Packing sweep", "--entities", "0")
    for name in lk.SOURCES:
        shutil.copyfile(art / name, project.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", project.dir / "art/ballast.png")
    sets = {"v1-130": json.loads((art / "atlas-v1.atlas.json").read_text(encoding="utf-8"))}
    sizes = {"wide": (1200, 40, (90, 60, 140, 255)), "row": (1100, 4, (200, 80, 80, 255)),
             "col": (4, 1100, (80, 200, 120, 255))}
    for name, (w, h, colour) in sizes.items():
        pngio.write_rgba(project.dir / "art" / f"{name}.png", w, h, [colour] * (w * h))
    sets["repro-3"] = {"version": 1, "id": "01JA34AA00000000000000R001", "maximum_size": [1, 1], "padding": 2,
                       "frames": {f"01JA34FR00000000000000R00{i + 1}": {"name": n, "path": f"{n}.png"}
                                  for i, n in enumerate(sizes)}}
    results = {}
    for set_name, base in sets.items():
        rows = []
        for w in SWEEP_W:
            for h in SWEEP_H:
                if w * h > 4 * 1024 * 1024:
                    continue
                name = f"art/{set_name}-{w}x{h}.atlas.json"
                (project.dir / name).write_text(json.dumps(dict(base, maximum_size=[w, h]), indent=1),
                                                encoding="utf-8")
                e = project.import_(name, usage="color", expect_error=True)
                page = e.get("details", {}).get("atlas", {}).get("size")
                rows.append({"maximum_size": [w, h], "exit": e["exit"], "page": page,
                             "message": e["stderr"].splitlines()[-1] if e["stderr"] else ""})
        violations = []
        for a in rows:
            for b in rows:
                if a is b or not (a["maximum_size"][0] <= b["maximum_size"][0]
                                  and a["maximum_size"][1] <= b["maximum_size"][1]):
                    continue
                if a["page"] and not b["page"]:
                    violations.append({"fits": a["maximum_size"], "rejected": b["maximum_size"]})
                elif a["page"] and b["page"] and b["page"][0] * b["page"][1] > a["page"][0] * a["page"][1]:
                    violations.append({"smaller": a["maximum_size"], "page": a["page"],
                                       "larger": b["maximum_size"], "larger_page": b["page"]})
        results[set_name] = {"bounds": len(rows), "fitting": sum(1 for r in rows if r["page"]),
                             "distinct_pages": sorted({tuple(r["page"]) for r in rows if r["page"]}),
                             "monotonic_violations": violations, "rows": rows}
    return {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
            "binary_sha256": lk.verify_binary(), "sets": results}


def residual(out):
    """Four solid frames, padding 2: 1200x40, 1100x4, 4x1100 and 1x1100. A layout inside
    1217 x 1104 exists (wide at (2, 2), the columns beside it at (1206, 2) and (1214, 2),
    the row below the wide frame at (2, 46)); the bounded candidate widths skip 1217."""
    project = lk.Project(out, "residual")
    (project.dir / "art").mkdir(parents=True)
    lk.engine("init", project.file, "--name", "Packing residual", "--entities", "0")
    sizes = {"wide": (1200, 40, (90, 60, 140, 255)), "row": (1100, 4, (200, 80, 80, 255)),
             "col": (4, 1100, (80, 200, 120, 255)), "thin": (1, 1100, (220, 200, 80, 255))}
    for name, (w, h, colour) in sizes.items():
        pngio.write_rgba(project.dir / "art" / f"{name}.png", w, h, [colour] * (w * h))
    ids = {n: f"01JA34FR00000000000000S00{i + 1}" for i, n in enumerate(sizes)}
    rows = []
    for maximum in ([1217, 1104], [1221, 1104], [1222, 1104], [1240, 1104], [1221, 1148], [2048, 2048]):
        d = {"version": 1, "id": "01JA34AA00000000000000S001", "maximum_size": maximum, "padding": 2,
             "frames": {ids[n]: {"name": n, "path": f"{n}.png"} for n in sizes}}
        name = f"art/residual-{maximum[0]}x{maximum[1]}.atlas.json"
        (project.dir / name).write_text(json.dumps(d, indent=1), encoding="utf-8")
        e = project.import_(name, usage="color", expect_error=True)
        atlas = e.get("details", {}).get("atlas", {})
        rows.append({"maximum_size": maximum, "exit": e["exit"], "page": atlas.get("size"),
                     "regions": {f["name"]: f["region"] for f in atlas.get("frames", {}).values()},
                     "message": e["stderr"].splitlines()[-1] if e["stderr"] else ""})
    layout = {"wide": [2, 2, 1200, 40], "col": [1206, 2, 4, 1100], "thin": [1214, 2, 1, 1100],
              "row": [2, 46, 1100, 4]}
    padded = [[x - 2, y - 2, w + 4, h + 4] for x, y, w, h in layout.values()]
    valid = all(x >= 0 and y >= 0 and x + w <= 1217 and y + h <= 1104 for x, y, w, h in padded) and not any(
        a[0] < b[0] + b[2] and b[0] < a[0] + a[2] and a[1] < b[1] + b[3] and b[1] < a[1] + a[3]
        for i, a in enumerate(padded) for b in padded[i + 1:])
    return {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
            "binary_sha256": lk.verify_binary(), "results": rows,
            "existing_layout_1217x1104": layout, "existing_layout_valid": valid}


def feasible(art, page=(1217, 1104)):
    """Engine-free proof that the 130-frame v1 set fits `page` under the engine's own layout
    rules (padded rectangles inside the page, pairwise disjoint): the ballast on top-left,
    the two 1104-texel columns beside it, everything else shelf-packed below the ballast."""
    import struct
    d = json.loads((art / "atlas-v1.atlas.json").read_text(encoding="utf-8"))
    pad = d["padding"]

    def png_size(path):
        return struct.unpack(">II", path.read_bytes()[16:24])
    items = {}
    for fid, f in d["frames"].items():
        w, h = (f["region"][2], f["region"][3]) if "region" in f else png_size(art / "ballast-v1.png")
        items[f["name"]] = (w + 2 * pad, h + 2 * pad)
    placed = {}

    def put(name, x, y):
        placed[name] = (x, y) + items[name]
    put("ballast", 0, 0)
    put("vstrip", 1204, 0)
    put("vcol", 1212, 0)
    put("vmid", 0, 44)
    put("hstrip", 5, 44)
    put("hrow", 5, 52)
    put("hmid", 5, 57)
    x, y, shelf = 5, 62, 0
    for (w, h), name in sorted(((v, k) for k, v in items.items() if k not in placed), reverse=True,
                               key=lambda t: (t[0][1], t[0][0], t[1])):
        if x + w > 1204:
            x, y, shelf = 5, y + shelf, 0
        placed[name] = (x, y, w, h)
        x, shelf = x + w, max(shelf, h)
    rects = list(placed.values())
    inside = all(rx + rw <= page[0] and ry + rh <= page[1] for rx, ry, rw, rh in rects)
    disjoint = not any(a[0] < b[0] + b[2] and b[0] < a[0] + a[2] and a[1] < b[1] + b[3] and b[1] < a[1] + a[3]
                       for i, a in enumerate(rects) for b in rects[i + 1:])
    return {"page": list(page), "padding": pad, "frames": len(placed), "of": len(items), "inside_page": inside,
            "disjoint": disjoint, "padded_rects": placed}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("art", type=Path, nargs="?")
    parser.add_argument("out", type=Path)
    parser.add_argument("--repro", action="store_true")
    parser.add_argument("--sweep", action="store_true", help="nested-bound monotonicity sweep (needs ART_DIR)")
    parser.add_argument("--residual", action="store_true", help="four-frame reproduction of a residual NoFit")
    parser.add_argument("--feasible", action="store_true",
                        help="engine-free proof that the v1 set fits 1217x1104 (needs ART_DIR); writes OUT_DIR as a file")
    lk.add_selectors(parser)
    args = parser.parse_args()
    lk.apply_selectors(args)
    if args.out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    if args.feasible:
        result = feasible(args.art)
        args.out.write_text(json.dumps(result, indent=1), encoding="utf-8")
        print(json.dumps({k: result[k] for k in ("page", "frames", "of", "inside_page", "disjoint")}))
        return
    if args.residual:
        summary = residual(args.out.resolve())
        (args.out / "packing-residual.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
        print(json.dumps(summary, indent=1))
        return
    if args.repro:
        summary = repro(args.out.resolve())
        (args.out / "packing-repro.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
        print(json.dumps(summary, indent=1))
        return
    if args.sweep:
        summary = sweep(args.art, args.out.resolve())
        (args.out / "packing-sweep.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
        print(json.dumps({k: {x: v[x] for x in ("bounds", "fitting", "distinct_pages", "monotonic_violations")}
                          for k, v in summary["sets"].items()} | {"unchanged": summary["binary_unchanged_every_call"],
                                                                 "calls": summary["engine_calls"]}, indent=1))
        return
    project = lk.Project(args.out.resolve(), "packing")
    (project.dir / "art").mkdir(parents=True)
    lk.engine("init", project.file, "--name", "Packing probe", "--entities", "0")
    for name in lk.SOURCES:
        shutil.copyfile(args.art / name, project.dir / "art" / name)
    shutil.copyfile(args.art / "ballast-v1.png", project.dir / "art/ballast.png")
    base = json.loads((args.art / "atlas-v1.atlas.json").read_text(encoding="utf-8"))
    results = []
    for w, h in MAXIMA:
        name = f"art/max-{w}x{h}.atlas.json"
        (project.dir / name).write_text(json.dumps(dict(base, maximum_size=[w, h]), indent=1), encoding="utf-8")
        entry = project.import_(name, usage="color", expect_error=True)
        row = {"maximum_size": [w, h], "exit": entry["exit"]}
        if entry["exit"] == 0:
            atlas = entry["details"]["atlas"]
            page = atlas["size"]
            used = sum((f["region"][2] + 2 * atlas["padding"]) * (f["region"][3] + 2 * atlas["padding"])
                       for f in atlas["frames"].values())
            row.update(page=page, page_texels=page[0] * page[1], frame_and_extrusion_texels=used,
                       coverage=round(used / (page[0] * page[1]), 4), mip_levels=entry["details"]["mip_levels"])
        else:
            row["message"] = entry["stderr"].splitlines()[-1] if entry["stderr"] else ""
        results.append(row)
    summary = {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
               "binary_sha256": lk.verify_binary(), "results": results}
    (project.dir / "packing-probe.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
