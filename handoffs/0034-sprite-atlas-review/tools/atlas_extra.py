#!/usr/bin/env python3
"""Large packed origin and minified-linear probe for handoff 0034. Standard library only.

The main atlas packs horizontal strips at x <= 19. This probe cooks a second atlas
whose maximum height (8) forces seven 1040-texel strip frames side by side on one
7308-wide page, so the last frame's packed origin is x = 6266 and its internal
512-texel cut lands at page x = 6778. Each frame starts at a different strip texel
(10 k), so a wrong frame is identifiable. The main v1 descriptor supplies the halo
and card 7 for minified linear sampling (level zero, no atlas mips).

Board Z is rendered by a pixel-perfect camera, a fractional orthographic camera and a
3-degree rolled one, in the atlas project twice (fresh processes) and in a control
project that draws the same entities from hstrip.png / cards.png / halo.png with
explicit rectangles. Every call goes through the frozen binary with hash checks.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_extra.py OUT_DIR
"""

import argparse
import json
import shutil
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import atlas_lookdev as lk  # noqa: E402
import atlas_scene as sc  # noqa: E402

sc.BOARD_SPEC["Z"] = (0.0, 0.0, 1280, 240)
sc.CAMERAS.update({
    "pp-Z": {"board": "Z", "offset": (0, 0), "roll": 0, "projection": sc.pixel((1280, 240))},
    "of-Z": {"board": "Z", "offset": (0.0371, -0.0123), "roll": 0, "projection": sc.ortho(240 / 18)},
    "rl-Z": {"board": "Z", "offset": (0, 0), "roll": 3, "projection": sc.ortho(20.0)},
})
PLAN = [("pp-Z", (1280, 240)), ("of-Z", (1280, 240)), ("rl-Z", (1280, 320))]
WIDE_ID = "01JA34AA000000000000000003"
WIDE_LEN = 1040


def wide_frames():
    return {f"01JA34FR0000000000000W000{k}": {"name": f"w{k}", "path": "hstrip.png",
                                             "region": [3 + 10 * k, 2 + k % 4, WIDE_LEN, 1]} for k in range(7)}


def board_z(s, wide, main):
    """wide/main: frame name -> (texture, region) for the active project."""
    b = "Z"
    s.sprite("Z:background", b, None, (-16, -16, 1312, 272), tint=sc.tint_of(sc.BG), layer=-100,
             texture=main("solid"))
    y = 8
    rows = [("w6", 1, False, 0.0, 100), ("w6", 1, False, 0.5, 100), ("w6", 1, True, 0.0, 100),
            ("w6", 1, True, 0.5, 100), ("w3", 1, False, 0.0, 100)]
    rows += [("w6", 8, flip, dx, 512 - 512 * 8) for flip in (False, True) for dx in (0.0, 0.5)]
    for k, (frame, m, flip, dx, x) in enumerate(rows):
        s.sprite(f"Z:{k}:{frame}:{m}x:{'flip' if flip else 'none'}:{dx}", b, frame,
                 (x + dx, y, WIDE_LEN * m, m), pivot=(0, 1), flip_x=flip, texture=wide(frame))
        s.meta[f"Z:{k}:{frame}:{m}x:{'flip' if flip else 'none'}:{dx}"].update(strip_offset=int(frame[1:]) * 10,
                                                                             strip_row=int(frame[1:]) % 4)
        y += m + 4
    s.sprite("Z:w6-linear-1x", b, "w6", (100, y, WIDE_LEN, 1), pivot=(0, 1), sampling="linear", texture=wide("w6"))
    y += 6
    s.sprite("Z:w6-linear-2.5x", b, "w6", (100, y, WIDE_LEN * 2.5, 2.5), pivot=(0, 1), sampling="linear",
             texture=wide("w6"))
    y += 8
    x = 100
    for frame, size in (("halo", 6), ("halo", 9), ("halo", 12), ("card7", 3), ("card7", 6)):
        w, h = (size, size) if frame == "halo" else (size, round(size * 14 / 12, 3))
        for sampling in ("linear", "nearest"):
            s.sprite(f"Z:min:{frame}:{size}:{sampling}", b, frame, (x, y, w, h), pivot=(0, 0), sampling=sampling,
                     texture=main(frame))
            x += 20
    return s


def build(project, art, kind, manifest, textures):
    by_name = {f["name"]: (fid, f) for fid, f in manifest["frames"].items()}
    wide_by_name = {f["name"]: (fid, f) for fid, f in wide_frames().items()}
    if kind == "atlas":
        wide = lambda n: (textures["wide"], {"frame": wide_by_name[n][0]})  # noqa: E731
        main = lambda n: (textures["atlas"], {"frame": by_name[n][0]})  # noqa: E731
    else:
        wide = lambda n: (textures["hstrip.png"], wide_by_name[n][1]["region"])  # noqa: E731
        main = lambda n: (textures[by_name[n][1]["path"]], by_name[n][1]["region"])  # noqa: E731
    s = sc.Scene(sc.Binding(kind, textures, manifest))
    board_z(s, wide, main)
    cams = {}
    for name, (w, h) in PLAN:
        spec = sc.CAMERAS[name]
        value = {"fov_degrees": sc.FOV, "near": sc.NEAR, "far": sc.FAR, "projection": spec["projection"]}
        cams[name] = s.entity(f"camera:{name}", {"Camera": value, "Transform": sc.transform(
            (spec["offset"][0], spec["offset"][1], sc.EYE_Z), sc.quat_z(spec["roll"]))})
    lk.create_entities(project, s, f"Atlas large-origin probe, {kind} (handoff 0034)")
    return s, cams


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    frames, raw = lk.plan_cost([PLAN, PLAN, PLAN])
    used = lk.ledger_used()
    if used[0] + frames > lk.PACKET_FRAMES or used[1] + raw > lk.PACKET_RAW:
        raise SystemExit(f"budget: needs {frames} frames / {raw / 2**20:.1f} MiB; used {used}")
    start = lk.verify_binary()
    out.mkdir(parents=True)
    lk.run([sys.executable, "-I", TOOLS / "atlas_art.py", out / "art"])
    art = out / "art"
    manifest = json.loads((art / "manifest.json").read_text(encoding="utf-8"))
    wide = {"version": 1, "id": WIDE_ID, "maximum_size": [8192, 8], "padding": 2,
            "frames": {fid: {"name": f["name"], "path": f["path"], "region": f["region"]}
                       for fid, f in wide_frames().items()}}
    summary = {"binary_sha256_start": start, "planned": {"frames": frames, "raw_bytes": raw}}

    atlas = lk.Project(out / "atlas", "extra")
    (atlas.dir / "art").mkdir(parents=True)
    lk.engine("init", atlas.file, "--name", "Atlas extra", "--entities", "0")
    for name in lk.SOURCES:
        shutil.copyfile(art / name, atlas.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png")
    shutil.copyfile(art / "atlas-v1.atlas.json", atlas.dir / "art/atlas.atlas.json")
    (atlas.dir / "art/wide.atlas.json").write_text(json.dumps(wide, indent=1), encoding="utf-8")
    imports = {"wide": atlas.import_("art/wide.atlas.json", usage="color"),
               "main": atlas.import_("art/atlas.atlas.json", usage="color")}
    textures = {"wide": imports["wide"]["asset"]["id"], "atlas": imports["main"]["asset"]["id"]}
    s, cams = build(atlas, art, "atlas", manifest, textures)
    shooter = lk.Shooter(out, out.name, cams)
    summary.update(imports=imports, shots={"r1": shooter.shots(atlas, "atlas-r1", PLAN),
                                           "r2": shooter.shots(atlas, "atlas-r2", PLAN)})
    (out / "scene-extra.json").write_text(json.dumps({"entities": s.entities, "meta": s.meta, "ids": s.ids,
                                                       "cameras": cams}, indent=1), encoding="utf-8")

    control = lk.Project(out / "control", "control")
    (control.dir / "art").mkdir(parents=True)
    lk.engine("init", control.file, "--name", "Atlas extra control", "--entities", "0")
    ctextures = {}
    for name in ("hstrip.png", "cards.png", "halo.png", "sheet.png"):
        shutil.copyfile(art / name, control.dir / "art" / name)
        ctextures[name] = control.import_(f"art/{name}", usage="color")["asset"]["id"]
    cs, ccams = build(control, art, "control", manifest, ctextures)
    if [e["id"] for e in cs.entities] != [e["id"] for e in s.entities]:
        raise SystemExit("control entity IDs differ")
    summary["control_shots"] = lk.Shooter(out, out.name, ccams).shots(control, "control", PLAN)
    summary["binary_sha256_end"] = lk.verify_binary()
    summary["binary_unchanged_every_call"] = lk.CALLS["unchanged"] and start == summary["binary_sha256_end"]
    summary["engine_calls"] = lk.CALLS["count"]
    summary["ledger_after"] = lk.ledger_used()
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary_unchanged": summary["binary_unchanged_every_call"], "engine_calls": lk.CALLS["count"],
                      "wide_page": imports["wide"]["details"]["atlas"]["size"],
                      "w6": imports["wide"]["details"]["atlas"]["frames"]["01JA34FR0000000000000W0006"]["region"],
                      "ledger": summary["ledger_after"]}))


if __name__ == "__main__":
    main()
