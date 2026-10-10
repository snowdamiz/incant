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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("art", type=Path, nargs="?")
    parser.add_argument("out", type=Path)
    parser.add_argument("--repro", action="store_true")
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    if args.repro:
        summary = repro(args.out.resolve())
        (args.out / "packing-repro.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
        print(json.dumps(summary, indent=1))
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
