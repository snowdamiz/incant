#!/usr/bin/env python3
"""Write the review-helper plans for handoff 0066 from an authored run. CPU only.

Adapted from handoff 0064's committed tools/plan.py (read-only reuse).

  python3 -I -B plan.py <run-dir>

Writes <run>/plans/{main-dry,main,source-free}.json and <run>/plans/index.json
with their hashes. `main-dry` (no PNGs) and `main` run once per variant;
`source-free` runs on the candidate after author.py source-free has deleted
boards C and D's sources. Every frame is <=960x540; PNG names are listed
explicitly and counted (6 per main run, 1 source-free: 13 of the 24 allowed).

Board C, perspective camera, one retained world:
  t0, t15, t30 at 960x540          4096 local lights moving; no grid resize (15x9x24, 128 words)
  t30 at 640x360, 333x197          grid resize (10x6, 6x4)
  t30 at 960x540, viewport 640x360 offset (37,41) inside the target (10x6)
  t30 at 960x540 again             back to 15x9: must repeat the first t30 frame
  t31  4063 despawned -> 33        2 words; also at 512x288 (8x5)
  t32  -> 32, t33 -> 31            1 word (the 33rd, then 32nd, boundary light removed)
  t34  -> 1 (the tile-corner light), t35 -> 0 (directional only), t36 unchanged
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

WIDE, HALF = (960, 540), (480, 270)
VIEWPORT = [37.0, 41.0, 640.0, 360.0]


def frame(tick, size, png=None, controls=True, viewport=None):
    value = {"tick": tick, "width": size[0], "height": size[1], "controls": controls}
    if viewport:
        value["viewport"] = viewport
    if png:
        value["png"] = png
    return value


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    run = Path(sys.argv[1])
    build = c.read_json(run / "build.json")
    empty = run / "empty-cache"
    empty.mkdir(exist_ok=True)

    def step(label, key, camera, frames, despawn=()):
        board = build["boards"][key]
        cache = run / key / ".incant/cache"
        models, textures = cache / "models", cache / "textures"
        return {"label": label, "scene": board["cooked"][key]["path"],
                "models": str(models if models.is_dir() else empty),
                "textures": str(textures if textures.is_dir() else empty),
                "camera": board["cameras"][camera], "frames": frames, "despawn": list(despawn)}

    board = build["boards"]["C"]
    hexes = board["local_lights"]
    boundary, corner = board["boundary_lights"], board["corner_light"]
    if len(hexes) != 4096 or len(boundary) != 33 or corner in boundary[-2:]:
        raise SystemExit("board C does not match the planned counts")
    despawn = [
        {"tick": 31, "entities": [h for n, h in hexes.items() if n not in boundary]},       # 4096 -> 33
        {"tick": 32, "entities": [hexes[boundary[-1]]]},                                     # 33 -> 32
        {"tick": 33, "entities": [hexes[boundary[-2]]]},                                     # 32 -> 31
        {"tick": 34, "entities": [hexes[n] for n in boundary[:-2] if n != corner]},          # 31 -> 1
        {"tick": 35, "entities": [hexes[corner]]},                                           # 1 -> 0
    ]
    counts = [4096 - len(despawn[0]["entities"])]
    for d in despawn[1:]:
        counts.append(counts[-1] - len(d["entities"]))
    if counts != [33, 32, 31, 1, 0]:
        raise SystemExit(f"despawn sequence reaches {counts}")
    persp = [
        frame(0, WIDE, "C-persp-n4096-t000.png"), frame(15, WIDE), frame(30, WIDE, "C-persp-n4096-t030.png"),
        frame(30, (640, 360)), frame(30, (333, 197)),
        frame(30, WIDE, "C-persp-n4096-t030-viewport.png", viewport=VIEWPORT),
        frame(30, WIDE, controls=False),
        frame(31, WIDE, "C-persp-n33.png"), frame(31, (512, 288)),
        frame(32, WIDE), frame(33, WIDE), frame(34, WIDE), frame(35, WIDE), frame(36, WIDE, controls=False)]
    main_plan = {"png_budget": 6, "steps": [
        step("C-persp", "C", "camera-persp", persp, despawn),
        step("C-ortho", "C", "camera-ortho", [frame(0, WIDE), frame(30, WIDE, "C-ortho-n4096-t030.png"),
                                              frame(30, HALF)]),
        step("D", "D", "camera", [frame(0, WIDE, "D-directional-only.png"), frame(0, HALF)]),
        step("C-persp-again", "C", "camera-persp", [frame(0, WIDE, controls=False)]),
    ]}
    source_free = {"png_budget": 1, "steps": [
        step("C-persp", "C", "camera-persp",
             [frame(0, WIDE, controls=False), frame(30, WIDE, controls=False),
              frame(31, WIDE, "C-persp-n33-source-free.png", controls=False)], despawn[:1]),
        step("C-ortho", "C", "camera-ortho", [frame(30, WIDE, controls=False)]),
        step("D", "D", "camera", [frame(0, WIDE, controls=False)]),
    ]}
    # The dry plan is main without PNG names: it exercises every step (despawns,
    # rebuilds, resizes, controls) once per variant before any frame budget is
    # spent, and its hashes repeat the main run's across processes.
    dry = json.loads(json.dumps(main_plan))
    dry["png_budget"] = 0
    for s in dry["steps"]:
        for f in s["frames"]:
            f.pop("png", None)
    plans = run / "plans"
    plans.mkdir(exist_ok=False)
    index = {}
    for name, plan in (("main-dry", dry), ("main", main_plan), ("source-free", source_free)):
        pngs = [f["png"] for s in plan["steps"] for f in s["frames"] if "png" in f]
        if len(pngs) != plan["png_budget"]:
            raise SystemExit(f"{name}: PNG count {len(pngs)} != budget {plan['png_budget']}")
        if any(f["width"] > c.MAX_W or f["height"] > c.MAX_H for s in plan["steps"] for f in s["frames"]):
            raise SystemExit("frame exceeds 960x540")
        path = plans / f"{name}.json"
        c.write_new_json(path, plan)
        index[name] = {"path": str(path), "sha256": c.sha(path), "pngs": pngs,
                       "frames": sum(len(s["frames"]) for s in plan["steps"]),
                       "controls": sum(f["controls"] for s in plan["steps"] for f in s["frames"])}
    index["source-free"]["source_free_boards"] = ["C", "D"]
    index["despawn_counts"] = counts
    c.write_new_json(plans / "index.json", index)
    print(json.dumps({k: {x: y for x, y in v.items() if x != "path"} if isinstance(v, dict) else v
                      for k, v in index.items()}, indent=1))


if __name__ == "__main__":
    main()
