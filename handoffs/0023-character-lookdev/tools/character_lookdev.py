#!/usr/bin/env python3
"""Real-engine character movement look-dev for handoff 0023.

Builds a disposable project in a NEW output directory using only engine
interfaces, then records actual motion with the CLI `play` command:

  1. `incant_headless init` creates an empty project.
  2. This script writes static glTF geometry (boxes and a Y capsule) with the
     visual dimensions baked into the vertices; every physics root has unit scale.
  3. `incant_headless import` registers each model through the command bus.
  4. `incant_headless rpc --journal ...` creates all entities in one
     `command.execute` transaction, with colliders that match the meshes, then
     `project.save`.
  5. `node tools/build_script.mjs` compiles character_course.ts.
  6. `incant_headless validate`, then `play --compiled-script --camera` once per
     requested run. Frames are the engine's own GPU output; nothing is faked.

Usage (repo root):

  python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py OUT_DIR \
      [--run NAME:CAMERA:CAPTURE_EVERY ...]

CAMERA is `overview`, `contact` (jump and step climb) or `exit` (step-off). The output directory must not exist.
"""

import argparse
import json
import math
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "artifacts/tools/incant_headless"
BEHAVIOR = Path(__file__).resolve().with_name("character_course.ts")
ALL = 0xFFFFFFFF
SECONDS = 5.0
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

# Lanes run along +X and are stacked along Z (near camera = +Z).
LANE_Z = {"jump": 3.6, "step": 1.8, "block": 0.0, "ramp": -1.8, "wall": -3.6}
START_X = -3.5
CAPSULE_HALF, CAPSULE_RADIUS = 0.5, 0.3  # 1.6 m tall character
STEP_H, BLOCK_H, HURDLE_H, RAMP_H = 0.2, 0.6, 0.35, 0.5

PALETTE = {
    "floor": ((0.26, 0.26, 0.25), 0.92),
    "lane": ((0.19, 0.19, 0.19), 0.95),
    "character": ((0.86, 0.82, 0.74), 0.45),
    "step": ((0.13, 0.30, 0.58), 0.7),
    "block": ((0.60, 0.13, 0.10), 0.7),
    "wall": ((0.30, 0.33, 0.38), 0.8),
    "hurdle": ((0.80, 0.48, 0.06), 0.6),
    "ramp": ((0.14, 0.42, 0.22), 0.7),
}


def run(*args, cwd=ROOT):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(f"{args[:2]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


# ---- geometry (meters, baked; +Y up) -------------------------------------------------

def box(hx, hy, hz):
    positions, normals, indices = [], [], []
    faces = [((1, 0, 0), (0, 1, 0)), ((-1, 0, 0), (0, 1, 0)), ((0, 1, 0), (0, 0, 1)),
             ((0, -1, 0), (0, 0, 1)), ((0, 0, 1), (0, 1, 0)), ((0, 0, -1), (0, 1, 0))]
    half = (hx, hy, hz)
    for n, u in faces:
        v = (n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0])
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple((n[i] + su * u[i] + sv * v[i]) * half[i] for i in range(3)))
            normals.append(n)
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def capsule(half_height, radius, rings=12, segments=32):
    """Y capsule; half_height excludes the hemispheres."""
    positions, normals, indices = [], [], []
    rows = []
    for hemi, offset in ((1, half_height), (-1, -half_height)):
        for r in range(rings + 1):
            phi = (math.pi / 2) * (1 - r / rings) if hemi == 1 else -(math.pi / 2) * (r / rings)
            rows.append((phi, offset))
    for phi, offset in rows:
        for s in range(segments + 1):
            theta = 2 * math.pi * s / segments
            n = (math.cos(phi) * math.cos(theta), math.sin(phi), math.cos(phi) * math.sin(theta))
            positions.append((n[0] * radius, n[1] * radius + offset, n[2] * radius))
            normals.append(n)
    width = segments + 1
    for row in range(len(rows) - 1):
        for s in range(segments):
            a, b = row * width + s, (row + 1) * width + s
            indices += [a, a + 1, b, b, a + 1, b + 1]
    return positions, normals, indices


def write_gltf(models, name, mesh, color, roughness):
    positions, normals, indices = mesh
    pos = b"".join(struct.pack("<3f", *p) for p in positions)
    nor = b"".join(struct.pack("<3f", *n) for n in normals)
    idx = b"".join(struct.pack("<I", i) for i in indices)
    (models / f"{name}.bin").write_bytes(pos + nor + idx)
    lo = [min(p[i] for p in positions) for i in range(3)]
    hi = [max(p[i] for p in positions) for i in range(3)]
    document = {
        "asset": {"version": "2.0", "generator": "handoff 0023 character_lookdev.py"},
        "buffers": [{"uri": f"{name}.bin", "byteLength": len(pos) + len(nor) + len(idx)}],
        "bufferViews": [
            {"buffer": 0, "byteLength": len(pos), "target": 34962},
            {"buffer": 0, "byteOffset": len(pos), "byteLength": len(nor), "target": 34962},
            {"buffer": 0, "byteOffset": len(pos) + len(nor), "byteLength": len(idx), "target": 34963},
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(positions), "type": "VEC3", "min": lo, "max": hi},
            {"bufferView": 1, "componentType": 5126, "count": len(normals), "type": "VEC3"},
            {"bufferView": 2, "componentType": 5125, "count": len(indices), "type": "SCALAR"},
        ],
        "meshes": [{"name": name, "primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1}, "indices": 2, "material": 0}]}],
        "materials": [{"name": name, "pbrMetallicRoughness": {"baseColorFactor": [*color, 1], "metallicFactor": 0, "roughnessFactor": roughness}}],
        "nodes": [{"name": name, "mesh": 0}],
        "scenes": [{"nodes": [0]}],
        "scene": 0,
    }
    path = models / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1))
    return path


# ---- scene ----------------------------------------------------------------------------

def quat_axis(axis, degrees):
    s = math.sin(math.radians(degrees) / 2)
    return [axis[0] * s, axis[1] * s, axis[2] * s, math.cos(math.radians(degrees) / 2)]


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def look(eye, target):
    """Rotation for a -Z-forward camera at eye looking at target (yaw then pitch)."""
    d = [target[i] - eye[i] for i in range(3)]
    yaw = math.degrees(math.atan2(-d[0], -d[2]))
    pitch = math.degrees(math.atan2(d[1], math.hypot(d[0], d[2])))
    return quat_mul(quat_axis((0, 1, 0), yaw), quat_axis((1, 0, 0), pitch))


def transform(t, rotation=(0, 0, 0, 1)):
    return {"translation": [round(v, 6) for v in t], "rotation": [round(v, 9) for v in rotation], "scale": [1, 1, 1]}


def collider(shape):
    return {"shape": shape, "density": 1000, "friction": 0.6, "restitution": 0, "sensor": False,
            "memberships": ALL, "filter": ALL}


def ramp(x0, y0, x1, y1, half_z, thickness=0.1):
    """Box whose top face runs from (x0, y0) to (x1, y1); returns (half extents, center, rotation)."""
    length = math.hypot(x1 - x0, y1 - y0)
    angle = math.atan2(y1 - y0, x1 - x0)
    nx, ny = -math.sin(angle), math.cos(angle)
    cx = (x0 + x1) / 2 - nx * thickness / 2
    cy = (y0 + y1) / 2 - ny * thickness / 2
    return (length / 2, thickness / 2, half_z), (cx, cy), quat_axis((0, 0, 1), math.degrees(angle))


def pieces():
    """(name, mesh key, half extents, center, rotation). Mesh dims equal collider dims."""
    out = [("Floor", (5.5, 0.1, 4.6), (0.4, -0.1, 0.0), None)]
    for lane, z in LANE_Z.items():
        out.append((f"Lane strip · {lane}", (5.0, 0.005, 0.62), (0.4, 0.005, z), None))
    z = LANE_Z["step"]
    out.append(("Low step 0.20 m", (1.4, STEP_H / 2, 0.7), (1.4, STEP_H / 2, z), None))
    z = LANE_Z["block"]
    out.append(("Tall block 0.60 m", (0.4, BLOCK_H / 2, 0.7), (0.4, BLOCK_H / 2, z), None))
    z = LANE_Z["wall"]
    out.append(("Wall", (4.0, 0.5, 0.1), (1.0, 0.5, z - 0.75), None))
    z = LANE_Z["jump"]
    out.append(("Hurdle 0.35 m", (0.08, HURDLE_H / 2, 0.7), (0.0, HURDLE_H / 2, z), None))
    z = LANE_Z["ramp"]
    up = ramp(-1.6, 0.0, 0.0, RAMP_H, 0.7)
    down = ramp(1.2, RAMP_H, 2.2, 0.0, 0.7)
    out.append(("Ramp up 17°", up[0], (up[1][0], up[1][1], z), up[2]))
    out.append(("Ramp top", (0.6, RAMP_H / 2, 0.7), (0.6, RAMP_H / 2, z), None))
    out.append(("Ramp down 27°", down[0], (down[1][0], down[1][1], z), down[2]))
    return out


def mesh_key(name):
    for key in ("Floor", "Lane strip", "Low step", "Tall block", "Wall", "Hurdle", "Ramp"):
        if name.startswith(key):
            return {"Floor": "floor", "Lane strip": "lane", "Low step": "step", "Tall block": "block",
                    "Wall": "wall", "Hurdle": "hurdle", "Ramp": "ramp"}[key]
    raise ValueError(name)


CAMERAS = {
    "overview": ((-5.6, 6.2, 9.6), (0.9, 0.0, -0.4)),
    "contact": ((3.6, 1.3, 7.4), (-0.3, 0.55, 2.6)),
    "exit": ((7.6, 1.5, -1.9), (2.4, 0.5, 2.4)),
}


def build(out):
    project = out / "character.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Character Look-dev", "--entities", "0")

    # Each piece gets its own model so its baked vertices match its collider exactly.
    assets, entities = {}, []
    for i, (name, half, center, rotation) in enumerate(pieces()):
        key = mesh_key(name)
        color, rough = PALETTE[key]
        model = f"piece{i:02d}_{key}"
        path = write_gltf(models, model, box(*half), color, rough)
        assets[model] = import_asset(project, out, path)
        components = {
            "Transform": transform(center, rotation or (0, 0, 0, 1)),
            "MeshRenderer": {"mesh": assets[model], "materials": [], "cast_shadows": key not in ("floor", "lane")},
        }
        if key != "lane":  # strips are decals 5 mm above the floor; no collider
            components["Collider"] = collider({"type": "box", "half_extents": list(half)})
        entities.append((name, components))
    color, rough = PALETTE["character"]
    path = write_gltf(models, "character", capsule(CAPSULE_HALF, CAPSULE_RADIUS), color, rough)
    assets["character"] = import_asset(project, out, path)
    start_y = CAPSULE_HALF + CAPSULE_RADIUS + 0.03
    for lane, z in LANE_Z.items():
        start_z = z + 0.35 if lane == "wall" else z
        entities.append((f"Character · {lane}", {
            "Transform": transform((START_X, start_y, start_z)),
            "MeshRenderer": {"mesh": assets["character"], "materials": [], "cast_shadows": True},
            "RigidBody": {"motion": "kinematic", "gravity_scale": 0, "linear_damping": 0,
                          "angular_damping": 0, "can_sleep": False, "ccd": False},
            "Collider": collider({"type": "capsule", "half_height": CAPSULE_HALF, "radius": CAPSULE_RADIUS}),
            "Velocity": {"linear": [0, 0, 0]},
        }))

    scene_id = next(iter(json.loads(project.read_text())["scenes"]))
    commands, ids = [], {}
    for name, components in entities:
        entity_id = new_ulid(len(commands))
        ids[name] = entity_id
        commands.append({"op": "create_entity", "scene_id": scene_id,
                         "entity": {"id": entity_id, "name": name, "parent": None, "components": components}})
    cameras = {}
    for i, (name, (eye, target)) in enumerate(CAMERAS.items()):
        cameras[name] = new_ulid(90 + i)
        commands.append({"op": "create_entity", "scene_id": scene_id, "entity": {
            "id": cameras[name], "name": f"Camera · {name}", "parent": None, "components": {
                "Camera": {"fov_degrees": 42, "near": 0.1, "far": 100},
                "Transform": transform(eye, look(eye, target))}}})
    sun = quat_mul(quat_axis((0, 1, 0), 215), quat_axis((1, 0, 0), -52))
    commands.append({"op": "create_entity", "scene_id": scene_id, "entity": {
        "id": new_ulid(99), "name": "Sun", "parent": None, "components": {
            "DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.4, "shadows": {"distance": 30}},
            "Transform": transform((0, 0, 0), sun)}}})

    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Character look-dev course"}},
        {"id": 3, "method": "project.save"},
    ]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    script = out / "character_course.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    (out / "ids.json").write_text(json.dumps(
        {"scene": scene_id, "cameras": cameras, "entities": ids, "assets": assets}, indent=1))
    return project, script, cameras


def play(project, script, out, name, camera, every):
    ticks = math.ceil(SECONDS * 60)
    frames = 1 + math.ceil(ticks / every)
    if frames > MAX_FRAMES or frames * WIDTH * HEIGHT * 4 > MAX_RAW:
        raise SystemExit(f"run {name}: {frames} frames exceeds the capture budget")
    report = run(CLI, "play", project, "--seconds", SECONDS, "--compiled-script", script,
                 "--output", out / name, "--camera", camera, "--capture-every", every,
                 "--width", WIDTH, "--height", HEIGHT, "--log-output", out / f"{name}.logs.jsonl")
    (out / f"{name}.stdout.json").write_text(report)


def import_asset(project, out, path):
    imported = json.loads(run(CLI, "import", project, path.relative_to(out)))
    return imported["asset"]["id"] if "asset" in imported else imported["id"]


def rpc(project, journal, requests):
    result = subprocess.run([str(CLI), "rpc", str(project), "--journal", str(journal)],
                            input="".join(json.dumps(r) + "\n" for r in requests),
                            capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"rpc failed: {result.stderr}")
    return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]


def new_ulid(n):
    """Deterministic, well-formed ULIDs so reruns produce identical documents."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA2CHAR0" + suffix


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--run", action="append", default=[], help="NAME:CAMERA:CAPTURE_EVERY")
    args = parser.parse_args()
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    runs = [r.split(":") for r in args.run] or [["overview", "overview", "3"]]
    for name, camera, every in runs:
        if camera not in CAMERAS or not every.isdigit() or int(every) < 1:
            raise SystemExit(f"invalid --run {name}:{camera}:{every}")
    project, script, cameras = build(out)
    for name, camera, every in runs:
        play(project, script, out, name, cameras[camera], int(every))
    print(json.dumps({"project": str(project), "runs": [r[0] for r in runs]}))


if __name__ == "__main__":
    main()
