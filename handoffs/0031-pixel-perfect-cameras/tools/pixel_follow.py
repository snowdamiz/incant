#!/usr/bin/env python3
"""Followed-geometry look-dev for the corrected pixel-perfect camera (handoff 0031).

Builds a disposable project in a NEW output directory with public commands only,
then renders it with one supplied binary, verifying its sha256 against its
metadata before and after every engine call:

  corrected   artifacts/tools/incant_headless             + binary.json
  foundation  artifacts/tools/foundation-incant_headless  + foundation-binary.json

The foundation binary is run only as the labelled "before" baseline.

Twelve configurations cover four logical resolutions (even/odd width x even/odd
height) and three inherited rolls (0, 90 deg, 0.37 rad), all at 32 px/m. Each has:
  * a copy of the original pixel test card (pixel_lookdev.card) at its own depth;
  * a rig carrying the roll, with the camera as its child at local (0, 0, 10);
  * two followers, an 8 x 8 px and a 5 x 3 px quad, rotated with the rig and placed
    on the rolled logical-pixel grid at fixed camera-plane offsets.
Configurations sit 100 m apart in depth; each camera's 50 m far plane sees only its own.

One strict-TypeScript behaviour moves every rig and follower together through the
same camera-plane path: 16 legs of 8 ticks at 1/8 logical pixel per tick, along
+-x, +-y and the four diagonals, visiting -1..1 px on both axes. Every leg holds an
exact half-pixel tie. Each configuration is captured at its logical size every 2
ticks (65 frames). One rolled odd/odd configuration is also captured at 768 x 432
(3x with odd spare pixels) every 4 ticks. Authored project bytes must not change.

Usage (repo root):
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_follow.py OUT_DIR [--binary foundation]
OUT_DIR must not exist.
"""

import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # reuse the original card geometry and glTF writer
import pixel_lookdev as card_kit  # noqa: E402

ROOT = HERE.parents[2]
BINARIES = {
    "corrected": (ROOT / "artifacts/tools/incant_headless", ROOT / "artifacts/tools/binary.json"),
    "foundation": (ROOT / "artifacts/tools/foundation-incant_headless", ROOT / "artifacts/tools/foundation-binary.json"),
}
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

PPU = 32
EYE = 10.0
NEAR, FAR, FOV = 0.1, 50.0, 50.0
SLAB = 100.0
RESOLUTIONS = {"320x180": (320, 180), "255x143": (255, 143), "256x143": (256, 143), "255x144": (255, 144)}
ROLLS = {"r0": 0.0, "r90": math.pi / 2, "r0.37": 0.37}
# Followers: lower-left corner on the camera-plane grid (right, up) in logical px,
# size in px, emissive colour. Colours do not occur on the card.
FOLLOWERS = {"a": ((-40, -30), (8, 8), (0.55, 0.20, 1.00)), "b": ((37, 17), (5, 3), (0.70, 1.00, 0.05))}
LEGS = [(1, 0), (-1, 0), (-1, 0), (1, 0), (0, 1), (0, -1), (0, -1), (0, 1),
        (1, 1), (-1, -1), (-1, -1), (1, 1), (1, -1), (-1, 1), (-1, 1), (1, -1)]
TICKS_PER_LEG = 8  # 1/8 px per tick: each leg moves one logical pixel
TICKS = len(LEGS) * TICKS_PER_LEG
EVERY = 2
SCALED = {"config": "255x143-r0.37", "size": (768, 432), "every": 4}


def verify(binary):
    cli, meta = BINARIES[binary]
    expected = json.loads(meta.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(cli.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"{cli.name} sha256 {actual} != {meta.name} {expected}")
    return actual


def run(*args):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:3]]} failed:\n{result.stdout}\n{result.stderr}")
    return result


def engine(binary, *args, stdin=None):
    before = verify(binary)
    result = subprocess.run([str(BINARIES[binary][0]), *map(str, args)], input=stdin,
                            capture_output=True, text=True, cwd=ROOT)
    if verify(binary) != before:
        raise SystemExit("binary changed during an engine call")
    if result.returncode != 0:
        raise SystemExit(f"{args[:2]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def roll_quaternion(angle):
    return [0.0, 0.0, math.sin(angle / 2), math.cos(angle / 2)]


def roll_matrix(q):
    """Upper-left 2x2 of glam's DMat3::from_quat for a pure-Z quaternion, same operation order."""
    z2 = q[2] + q[2]
    zz, wz = q[2] * z2, q[3] * z2
    return [1.0 - zz, -wz, wz, 1.0 - zz]  # [m00, m01, m10, m11]: world = M * (right, up)


def place(m, right_px, up_px, z, q):
    x, y = right_px / PPU, up_px / PPU
    return {"translation": [m[0] * x + m[1] * y, m[2] * x + m[3] * y, z], "rotation": q, "scale": [1, 1, 1]}


def path(k):
    """Camera-plane position after k ticks, in eighths of a logical pixel."""
    r = u = 0
    for i in range(k):
        dr, du = LEGS[i // TICKS_PER_LEG]
        r, u = r + dr, u + du
    return r, u


def quad(w_px, h_px):
    w, h = w_px / PPU, h_px / PPU
    return [(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, h, 0.0), (0.0, h, 0.0)], [0, 1, 2, 0, 2, 3]


def new_ulid(n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA31FF00" + suffix


BEHAVIOR = """
function eighths(k: number): [number, number] {
  let r = 0;
  let u = 0;
  for (let i = 0; i < k; i++) {
    const [dr, du] = LEGS[Math.floor(i / TICKS_PER_LEG)]!;
    r += dr;
    u += du;
  }
  return [r, u];
}

function place(c: Config, right: number, up: number, z: number): Transform {
  const x = right / PPU;
  const y = up / PPU;
  return { translation: [c.m[0] * x + c.m[1] * y, c.m[2] * x + c.m[3] * y, z], rotation: c.q, scale: [1, 1, 1] };
}

export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const k = state.tick + 1; // commanded now, visible from the next frame
    state.tick++;
    if (k > LEGS.length * TICKS_PER_LEG) return;
    const [er, eu] = eighths(k);
    const right = er / 8;
    const up = eu / 8;
    for (const c of CONFIGS) {
      const set = (entity_id: string, value: Transform) =>
        api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id, component: 'Transform', value });
      set(c.rig, place(c, right, up, c.z));
      set(c.a, place(c, right + FOLLOW_A[0], up + FOLLOW_A[1], c.z + 0.02));
      set(c.b, place(c, right + FOLLOW_B[0], up + FOLLOW_B[1], c.z + 0.02));
    }
  },
});
"""


def build(out, binary):
    project = out / "follow.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    engine(binary, "init", project, "--name", "Pixel-perfect follow look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, by_colour):
        path = card_kit.write_gltf(models, name, by_colour)
        imported = json.loads(engine(binary, "import", project, path.relative_to(out)))
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    card_asset = importer("card", card_kit.card())
    follower_assets = {}
    for key, (_, (w, h), colour) in FOLLOWERS.items():
        card_kit.PALETTE[f"follower_{key}"] = colour
        follower_assets[key] = importer(f"follower_{key}", {f"follower_{key}": [quad(w, h)]})

    counter = iter(range(1, 10_000))
    entities, configs = [], {}

    def entity(name, components, parent=None):
        entity_id = new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": parent, "components": components})
        return entity_id

    def mesh(asset, transform):
        return {"Transform": transform, "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": False}}

    for index, ((res_key, (w, h)), (roll_key, angle)) in enumerate(
            (r, a) for r in RESOLUTIONS.items() for a in ROLLS.items()):
        key = f"{res_key}-{roll_key}"
        z = -SLAB * index
        q = roll_quaternion(angle)
        m = roll_matrix(q)
        entity(f"Card {key}", mesh(card_asset, {"translation": [0, 0, z], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}))
        rig = entity(f"Rig {key}", {"Transform": place(m, 0, 0, z, q)})
        camera = entity(f"Camera {key}", {
            "Camera": {"fov_degrees": FOV, "near": NEAR, "far": FAR,
                       "projection": {"kind": "pixel_perfect", "pixels_per_unit": PPU, "reference_resolution": [w, h]}},
            "Transform": {"translation": [0, 0, EYE], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}}, parent=rig)
        followers = {}
        for fkey, ((fr, fu), _, _) in FOLLOWERS.items():
            followers[fkey] = entity(f"Follower {fkey} {key}", mesh(follower_assets[fkey], place(m, fr, fu, z + 0.02, q)))
        configs[key] = {"index": index, "resolution": [w, h], "roll": angle, "z": z, "q": q, "m": m,
                        "rig": rig, "camera": camera, "followers": followers}

    revision = json.loads(engine(binary, "rpc", project, "--journal", journal,
                                 stdin=json.dumps({"id": 1, "method": "project.read"}) + "\n").splitlines()[0])
    requests = [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
            "expected_revision": revision["result"]["revision"],
            "description": "Pixel-perfect follow look-dev scene (handoff 0031)"}},
        {"id": 3, "method": "project.save"}]
    for line in engine(binary, "rpc", project, "--journal", journal,
                       stdin="".join(json.dumps(r) + "\n" for r in requests)).splitlines():
        if line.strip() and "error" in json.loads(line):
            raise SystemExit(f"RPC error: {line}")
    engine(binary, "validate", project)

    behaviour = [{"rig": c["rig"], "a": c["followers"]["a"], "b": c["followers"]["b"], "z": c["z"],
                  "m": c["m"], "q": c["q"]} for c in configs.values()]
    source = out / "follow.ts"
    source.write_text(
        "import type { ScriptApi, Transform } from "
        + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n"
        + "type Config = { rig: string; a: string; b: string; z: number;\n"
        + "  m: [number, number, number, number]; q: [number, number, number, number] };\n"
        + f"const SCENE_ID = {json.dumps(scene_id)};\n"
        + f"const PPU = {PPU};\nconst TICKS_PER_LEG = {TICKS_PER_LEG};\n"
        + f"const LEGS: [number, number][] = {json.dumps(LEGS)};\n"
        + f"const FOLLOW_A: [number, number] = {json.dumps(list(FOLLOWERS['a'][0]))};\n"
        + f"const FOLLOW_B: [number, number] = {json.dumps(list(FOLLOWERS['b'][0]))};\n"
        + f"const CONFIGS: Config[] = {json.dumps(behaviour)};\n" + BEHAVIOR, encoding="utf-8")
    run("node", ROOT / "node_modules/typescript/bin/tsc", "--strict", "--noEmit", "--target", "ES2022",
        "--module", "ESNext", "--moduleResolution", "bundler", source)
    script = out / "follow.js"
    run("node", ROOT / "tools/build_script.mjs", source, script)
    scene = {"scene": scene_id, "binary": binary, "ppu": PPU, "eye": EYE, "slab": SLAB, "legs": LEGS,
             "ticks_per_leg": TICKS_PER_LEG, "ticks": TICKS, "every": EVERY, "scaled": SCALED,
             "followers": {k: {"corner": v[0], "size": v[1], "colour": v[2]} for k, v in FOLLOWERS.items()},
             "configs": configs, "entities": len(entities)}
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, script, configs


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--binary", choices=sorted(BINARIES), default="corrected")
    args = parser.parse_args()
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    frames = 1 + math.ceil(TICKS / EVERY)
    for w, h in [*RESOLUTIONS.values(), SCALED["size"]]:
        if frames > MAX_FRAMES or frames * w * h * 4 > MAX_RAW:
            raise SystemExit("capture exceeds the budget")
    start = verify(args.binary)
    out.mkdir(parents=True)
    project, script, configs = build(out, args.binary)
    authored = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    plays = {}
    captures = [(key, c["resolution"], EVERY) for key, c in configs.items()]
    captures.append((SCALED["config"], SCALED["size"], SCALED["every"]))
    for key, (w, h), every in captures:
        target = out / f"follow-{key}-{w}x{h}"
        stdout = engine(args.binary, "play", project, "--ticks", TICKS, "--compiled-script", script,
                        "--camera", configs[key]["camera"], "--capture-every", every,
                        "--width", w, "--height", h, "--output", target)
        report = json.loads((target / "report.json").read_text(encoding="utf-8"))
        plays[target.name] = {"frames": {p.name: sha(p) for p in sorted(target.glob("*.png"))},
                              "completed": report.get("completed"), "passed": report.get("passed"),
                              "script_commands": report.get("script_commands"), "stdout_bytes": len(stdout)}
    after = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    summary = {"binary": args.binary, "binary_sha256_start": start, "binary_sha256_end": verify(args.binary),
               "authored_before": authored, "authored_after": after, "authored_unchanged": authored == after,
               "plays": plays}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary": args.binary, "sha256": start, "binary_unchanged": start == summary["binary_sha256_end"],
                      "authored_unchanged": summary["authored_unchanged"], "captures": len(plays),
                      "frames": sum(len(v["frames"]) for v in plays.values()),
                      "all_completed": all(v["completed"] and v["passed"] for v in plays.values())}))


if __name__ == "__main__":
    main()
