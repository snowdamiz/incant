#!/usr/bin/env python3
"""Near-tie follow probe for the pixel-perfect camera (handoff 0031).

pixel_follow.py moves in exact 1/8-pixel steps, so every tie it meets is exact.
This probe places the camera and its two grid-aligned followers a tiny distance
delta beside a half-pixel tie, one position per tick, to measure how wide a band
around each tie still disagrees between camera snapping (f64, CPU) and raster
coverage (GPU sub-pixel precision). Positions in logical pixels, camera plane:

  right+  right = +0.5 + delta, up = 0
  right-  right = -0.5 + delta, up = 0
  up+     right = 0,            up = +0.5 + delta

for delta = 0 and +-{1e-15, 1e-14, 1e-12, 1e-9, 1e-6, 1e-5, 1e-4, 5e-4, 1e-3, 1.5e-3,
1.9e-3, 1.95e-3, 2e-3, 2.5e-3, 3e-3, 3.9e-3, 4e-3, 6e-3, 1e-2}. Each position is
held for one tick and captured (118 frames per configuration, frame N shows
position N - 1; frame 0 is the authored start at 0, 0). Configurations: 320x180
at roll 0 and 0.37 rad, and 255x143 at roll 0, built with pixel_follow.py's
geometry, followers and binaries, with sha256 checks around every engine call.

Usage (repo root):
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_neartie.py OUT_DIR [--binary foundation]
OUT_DIR must not exist.
"""

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import pixel_follow as kit  # noqa: E402

CONFIGS = {"320x180-r0": ((320, 180), 0.0), "320x180-r0.37": ((320, 180), 0.37), "255x143-r0": ((255, 143), 0.0)}
DELTAS = [1e-15, 1e-14, 1e-12, 1e-9, 1e-6, 1e-5, 1e-4, 5e-4, 1e-3, 1.5e-3, 1.9e-3, 1.95e-3, 2e-3, 2.5e-3,
          3e-3, 3.9e-3, 4e-3, 6e-3, 1e-2]
SIGNED = [0.0] + [d for x in DELTAS for d in (x, -x)]
SERIES = {"right+": lambda d: (0.5 + d, 0.0), "right-": lambda d: (-0.5 + d, 0.0), "up+": lambda d: (0.0, 0.5 + d)}


def positions():
    """[(series, delta, right_px, up_px)] in tick order."""
    return [(name, d, *fn(d)) for name, fn in SERIES.items() for d in SIGNED]


BEHAVIOR = """
export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const t = state.tick;
    state.tick++;
    if (t >= POSITIONS.length) return;
    const [right, up] = POSITIONS[t]!;
    for (const c of CONFIGS) {
      const set = (entity_id: string, value: Transform) =>
        api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id, component: 'Transform', value });
      set(c.rig, place(c, right, up, c.z));
      set(c.a, place(c, right + FOLLOW_A[0], up + FOLLOW_A[1], c.z + 0.02));
      set(c.b, place(c, right + FOLLOW_B[0], up + FOLLOW_B[1], c.z + 0.02));
    }
  },
});

function place(c: Config, right: number, up: number, z: number): Transform {
  const x = right / PPU;
  const y = up / PPU;
  return { translation: [c.m[0] * x + c.m[1] * y, c.m[2] * x + c.m[3] * y, z], rotation: c.q, scale: [1, 1, 1] };
}
"""


def build(out, binary):
    project = out / "neartie.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    kit.engine(binary, "init", project, "--name", "Pixel-perfect near-tie probe", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, by_colour):
        path = kit.card_kit.write_gltf(models, name, by_colour)
        imported = json.loads(kit.engine(binary, "import", project, path.relative_to(out)))
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    card_asset = importer("card", kit.card_kit.card())
    follower_assets = {}
    for key, (_, (w, h), colour) in kit.FOLLOWERS.items():
        kit.card_kit.PALETTE[f"follower_{key}"] = colour
        follower_assets[key] = importer(f"follower_{key}", {f"follower_{key}": [kit.quad(w, h)]})
    counter = iter(range(1, 10_000))
    entities, configs = [], {}

    def entity(name, components, parent=None):
        entity_id = kit.new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": parent, "components": components})
        return entity_id

    def mesh(asset, transform):
        return {"Transform": transform, "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": False}}

    for index, (key, ((w, h), angle)) in enumerate(CONFIGS.items()):
        z = -kit.SLAB * index
        q = kit.roll_quaternion(angle)
        m = kit.roll_matrix(q)
        entity(f"Card {key}", mesh(card_asset, {"translation": [0, 0, z], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}))
        rig = entity(f"Rig {key}", {"Transform": kit.place(m, 0, 0, z, q)})
        camera = entity(f"Camera {key}", {
            "Camera": {"fov_degrees": kit.FOV, "near": kit.NEAR, "far": kit.FAR,
                       "projection": {"kind": "pixel_perfect", "pixels_per_unit": kit.PPU, "reference_resolution": [w, h]}},
            "Transform": {"translation": [0, 0, kit.EYE], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}}, parent=rig)
        followers = {f: entity(f"Follower {f} {key}", mesh(follower_assets[f], kit.place(m, fr, fu, z + 0.02, q)))
                     for f, ((fr, fu), _, _) in kit.FOLLOWERS.items()}
        configs[key] = {"index": index, "resolution": [w, h], "roll": angle, "z": z, "q": q, "m": m,
                        "rig": rig, "camera": camera, "followers": followers}
    revision = json.loads(kit.engine(binary, "rpc", project, "--journal", journal,
                                     stdin=json.dumps({"id": 1, "method": "project.read"}) + "\n").splitlines()[0])
    requests = [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
            "expected_revision": revision["result"]["revision"],
            "description": "Pixel-perfect near-tie probe (handoff 0031)"}},
        {"id": 3, "method": "project.save"}]
    for line in kit.engine(binary, "rpc", project, "--journal", journal,
                           stdin="".join(json.dumps(r) + "\n" for r in requests)).splitlines():
        if line.strip() and "error" in json.loads(line):
            raise SystemExit(f"RPC error: {line}")
    kit.engine(binary, "validate", project)
    plan = positions()
    behaviour = [{"rig": c["rig"], "a": c["followers"]["a"], "b": c["followers"]["b"], "z": c["z"],
                  "m": c["m"], "q": c["q"]} for c in configs.values()]
    source = out / "neartie.ts"
    source.write_text(
        "import type { ScriptApi, Transform } from "
        + json.dumps((kit.ROOT / "sdk/ts/src/index").as_posix()) + ";\n"
        + "type Config = { rig: string; a: string; b: string; z: number;\n"
        + "  m: [number, number, number, number]; q: [number, number, number, number] };\n"
        + f"const SCENE_ID = {json.dumps(scene_id)};\nconst PPU = {kit.PPU};\n"
        + f"const FOLLOW_A: [number, number] = {json.dumps(list(kit.FOLLOWERS['a'][0]))};\n"
        + f"const FOLLOW_B: [number, number] = {json.dumps(list(kit.FOLLOWERS['b'][0]))};\n"
        + f"const POSITIONS: [number, number][] = {json.dumps([[r, u] for _, _, r, u in plan])};\n"
        + f"const CONFIGS: Config[] = {json.dumps(behaviour)};\n" + BEHAVIOR, encoding="utf-8")
    kit.run("node", kit.ROOT / "node_modules/typescript/bin/tsc", "--strict", "--noEmit", "--target", "ES2022",
            "--module", "ESNext", "--moduleResolution", "bundler", source)
    script = out / "neartie.js"
    kit.run("node", kit.ROOT / "tools/build_script.mjs", source, script)
    scene = {"scene": scene_id, "binary": binary, "ppu": kit.PPU,
             "followers": {k: {"corner": v[0], "size": v[1], "colour": v[2]} for k, v in kit.FOLLOWERS.items()},
             "positions": [{"series": s, "delta": d, "right": r, "up": u} for s, d, r, u in plan],
             "configs": configs, "entities": len(entities)}
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, script, configs, len(plan)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--binary", choices=sorted(kit.BINARIES), default="corrected")
    args = parser.parse_args()
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    ticks = len(positions())
    for (w, h), _ in CONFIGS.values():
        if ticks + 1 > kit.MAX_FRAMES or (ticks + 1) * w * h * 4 > kit.MAX_RAW:
            raise SystemExit("capture exceeds the budget")
    start = kit.verify(args.binary)
    out.mkdir(parents=True)
    project, script, configs, ticks = build(out, args.binary)
    authored = {p.name: kit.sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    plays = {}
    for key, config in configs.items():
        w, h = config["resolution"]
        target = out / f"neartie-{key}-{w}x{h}"
        kit.engine(args.binary, "play", project, "--ticks", ticks, "--compiled-script", script,
                   "--camera", config["camera"], "--capture-every", 1, "--width", w, "--height", h, "--output", target)
        report = json.loads((target / "report.json").read_text(encoding="utf-8"))
        plays[target.name] = {"frames": {p.name: kit.sha(p) for p in sorted(target.glob("*.png"))},
                              "completed": report.get("completed"), "passed": report.get("passed")}
    after = {p.name: kit.sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    summary = {"binary": args.binary, "binary_sha256_start": start, "binary_sha256_end": kit.verify(args.binary),
               "authored_before": authored, "authored_after": after, "authored_unchanged": authored == after,
               "plays": plays}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary": args.binary, "sha256": start, "binary_unchanged": start == summary["binary_sha256_end"],
                      "authored_unchanged": summary["authored_unchanged"], "captures": len(plays),
                      "frames": sum(len(v["frames"]) for v in plays.values()),
                      "all_completed": all(v["completed"] and v["passed"] for v in plays.values())}))


if __name__ == "__main__":
    main()
