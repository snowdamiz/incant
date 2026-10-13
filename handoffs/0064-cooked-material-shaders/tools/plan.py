#!/usr/bin/env python3
"""Write the review-helper plans for handoff 0064 from an authored run. CPU only.

  python3 -I -B plan.py <run-dir>            # run r1: main-dry, main, source-free
  python3 -I -B plan.py <run-dir> shadow     # run r2: shadow-dry, shadow (board S)

Writes <run>/plans/{main-dry,main,source-free}.json and <run>/plans/index.json
with their hashes. `main-dry` (no PNGs) and `main` run once per variant;
`source-free` runs on the
candidate after author.py source-free has deleted the boards' sources.
Every frame is <=960x540; PNG names are listed explicitly and counted.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

WIDE, HALF, SMALL = (960, 540), (480, 270), (320, 180)


def frame(tick, size, png=None, controls=True):
    value = {"tick": tick, "width": size[0], "height": size[1], "controls": controls}
    if png:
        value["png"] = png
    return value


def main():
    if len(sys.argv) not in (2, 3) or sys.argv[2:] not in ([], ["shadow"]):
        raise SystemExit(__doc__)
    run = Path(sys.argv[1])
    build = c.read_json(run / "build.json")
    if sys.argv[2:] == ["shadow"]:
        return shadow(run, build)
    empty = run / "empty-cache"
    empty.mkdir(exist_ok=True)

    def step(label, key, scene, camera, frames, despawn=()):
        board = build["boards"][key]
        cache = run / key / ".incant/cache"
        models, textures = cache / "models", cache / "textures"
        return {"label": label, "scene": board["cooked"][scene]["path"],
                "models": str(models if models.is_dir() else empty),
                "textures": str(textures if textures.is_dir() else empty),
                "camera": board["cameras"][camera], "frames": frames, "despawn": list(despawn)}

    lights = list(build["boards"]["L"]["local_lights"].values())
    first = lights[1::2]  # 60 of 120: every other local light
    kept = lights[0::2]
    second, third = kept[:28], kept[28:]  # 60 -> 32 -> 0 local lights
    main_plan = {"png_budget": 9, "steps": [
        step("M-wide", "M", "M", "camera-wide", [frame(0, WIDE, "M-wide.png")]),
        step("M-close", "M", "M", "camera-close", [frame(0, WIDE, "M-close.png")]),
        step("L-persp", "L", "L", "camera-persp", [
            frame(0, WIDE, "L-persp-t000.png"), frame(45, WIDE), frame(90, WIDE, "L-persp-t090.png"),
            frame(90, (640, 360)), frame(90, SMALL), frame(91, WIDE), frame(92, WIDE, "L-persp-t092.png"),
            frame(93, WIDE), frame(94, WIDE, controls=False)],
             [{"tick": 91, "entities": first}, {"tick": 92, "entities": second}, {"tick": 93, "entities": third}]),
        step("L-ortho", "L", "L", "camera-ortho", [frame(0, WIDE), frame(90, WIDE, "L-ortho-t090.png")]),
        step("Z1", "Z1", "Z1", "camera", [frame(0, HALF, "Z1.png")]),
        step("Z0", "Z0", "Z0", "camera", [frame(0, SMALL)]),
        step("M-wide-again", "M", "M", "camera-wide", [frame(0, WIDE, controls=False)]),
        step("R-v1", "R", "R-v1", "camera", [frame(0, HALF, "R-v1.png")]),
        step("R-v2", "R", "R-v2", "camera", [frame(0, HALF, "R-v2.png")]),
        step("R-v1-again", "R", "R-v1", "camera", [frame(0, HALF, controls=False)]),
    ]}
    source_free = {"png_budget": 1, "steps": [
        step("M-wide", "M", "M", "camera-wide", [frame(0, WIDE, "M-wide-source-free.png", controls=False)]),
        step("M-close", "M", "M", "camera-close", [frame(0, WIDE, controls=False)]),
        step("L-persp", "L", "L", "camera-persp", [frame(0, WIDE, controls=False), frame(90, WIDE, controls=False)]),
        step("Z1", "Z1", "Z1", "camera", [frame(0, HALF, controls=False)]),
        step("R-v1", "R", "R-v1", "camera", [frame(0, HALF, controls=False)]),
        step("R-v2", "R", "R-v2", "camera", [frame(0, HALF, controls=False)]),
    ]}
    # The dry plan is main without PNG names: it exercises every step (despawns,
    # rebuilds, controls) once per variant before any frame budget is spent, and
    # its hashes repeat the main run's across processes.
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
                       "frames": sum(len(s["frames"]) for s in plan["steps"])}
    c.write_new_json(plans / "index.json", index)
    print(json.dumps(index, indent=1))


def shadow(run, build):
    """Run r2: board S only, one 960x540 frame with controls; dry first, then with a PNG."""
    board = build["boards"]["S"]
    cache = run / "S/.incant/cache"
    step = {"label": "S-shadows", "scene": board["cooked"]["S"]["path"], "models": str(cache / "models"),
            "textures": str(cache / "textures"), "camera": board["cameras"]["camera"], "despawn": []}
    plans = run / "plans"
    plans.mkdir(exist_ok=False)
    index = {}
    for name, png in (("shadow-dry", None), ("shadow", "S-shadows.png")):
        plan = {"png_budget": 1 if png else 0, "steps": [{**step, "frames": [frame(0, WIDE, png)]}]}
        path = plans / f"{name}.json"
        c.write_new_json(path, plan)
        index[name] = {"path": str(path), "sha256": c.sha(path), "pngs": [png] if png else [], "frames": 1}
    c.write_new_json(plans / "index.json", index)
    print(json.dumps(index, indent=1))


if __name__ == "__main__":
    main()
