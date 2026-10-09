#!/usr/bin/env python3
"""Real-engine physics look-dev for handoff 0022.

Builds a disposable project under artifacts/0022-physics/ using only engine
interfaces, then records an actual fall with the CLI `play` command:

  1. `incant_headless init` creates an empty project.
  2. This script writes static glTF geometry (box, UV sphere, Y capsule) whose
     visual dimensions are baked into the vertices; nothing is scaled.
  3. `incant_headless import` registers each model through the command bus.
  4. `incant_headless rpc --journal ...` creates every entity with one
     `command.execute` transaction (RigidBody/Collider sized to match the mesh,
     unit-scale scene roots), then `project.save`.
  5. `incant_headless validate` and `incant_headless play --camera` record frames.

The frames are the engine's own GPU output of the Rapier simulation. Nothing here
simulates physics or draws images. Usage (repo root, after the release build):

  python3 -I handoffs/0022-physics-inspector/tools/physics_lookdev.py [out-dir]
"""

import json
import math
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "target/release/incant_headless"
OUT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "artifacts/0022-physics"
PROJECT = OUT / "physics.incant.json"
JOURNAL = PROJECT.with_suffix(".journal.jsonl")
ALL = 0xFFFFFFFF


def run(*args):
    result = subprocess.run([str(CLI), *map(str, args)], capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"{args[0]} failed:\n{result.stdout}\n{result.stderr}")
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
    """Y capsule; half_height excludes the hemispheres (radius 0 height = sphere)."""
    positions, normals, indices = [], [], []
    rows = []
    # Upper hemisphere rings (pole to equator), then lower (equator to pole).
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


def write_gltf(name, mesh, color, roughness=0.7):
    positions, normals, indices = mesh
    pos = b"".join(struct.pack("<3f", *p) for p in positions)
    nor = b"".join(struct.pack("<3f", *n) for n in normals)
    idx = b"".join(struct.pack("<I", i) for i in indices)
    (OUT / "models" / f"{name}.bin").write_bytes(pos + nor + idx)
    lo = [min(p[i] for p in positions) for i in range(3)]
    hi = [max(p[i] for p in positions) for i in range(3)]
    document = {
        "asset": {"version": "2.0", "generator": "handoff 0022 physics_lookdev.py"},
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
    path = OUT / "models" / f"{name}.gltf"
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


def transform(t, rotation=(0, 0, 0, 1)):
    return {"translation": list(t), "rotation": list(rotation), "scale": [1, 1, 1]}


def body(motion="dynamic", **extra):
    value = {"motion": motion, "gravity_scale": 1, "linear_damping": 0, "angular_damping": 0.05, "can_sleep": True, "ccd": True}
    value.update(extra)
    return value


def collider(shape, **extra):
    value = {"shape": shape, "density": 1000, "friction": 0.6, "restitution": 0, "sensor": False, "memberships": ALL, "filter": ALL}
    value.update(extra)
    return value


def main():
    if not CLI.exists():
        raise SystemExit("Build first: ./tools/cargo build -p incant_headless --release --locked")
    if OUT.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    (OUT / "models").mkdir(parents=True)
    run("init", PROJECT, "--name", "Physics Look-dev", "--entities", "0")

    # Visual dimensions are the collider dimensions; every root keeps unit scale.
    shapes = {
        "floor": (box(6, 0.1, 4), (0.46, 0.46, 0.44), 0.9),
        "crate": (box(0.4, 0.4, 0.4), (0.78, 0.45, 0.22), 0.75),
        "ball": (capsule(0, 0.35), (0.24, 0.45, 0.85), 0.35),
        "pill": (capsule(0.35, 0.25), (0.33, 0.68, 0.42), 0.55),
        "ghost": (capsule(0, 0.3), (0.72, 0.58, 0.9), 0.5),
    }
    assets = {}
    for name, (mesh, color, rough) in shapes.items():
        path = write_gltf(name, mesh, color, rough)
        imported = json.loads(run("import", PROJECT, path.relative_to(OUT)))
        assets[name] = imported["asset"]["id"] if "asset" in imported else imported["id"]

    scene_id = json.loads(PROJECT.read_text())["scenes"]
    scene_id = next(iter(scene_id))
    tilt = quat_mul(quat_axis((0, 0, 1), 28), quat_axis((1, 0, 0), 17))
    entities = [
        ("Floor", {"Collider": collider({"type": "box", "half_extents": [6, 0.1, 4]}, friction=0.8)},
         transform((0, -0.1, 0)), "floor"),
        ("Crate", {"RigidBody": body(), "Collider": collider({"type": "box", "half_extents": [0.4, 0.4, 0.4]}, density=420),
                   "AngularVelocity": {"angular": [0, 2, 1.2]}}, transform((-1.6, 2.4, 0), tilt), "crate"),
        ("Ball", {"RigidBody": body(), "Collider": collider({"type": "sphere", "radius": 0.35}, restitution=0.6)},
         transform((0, 3.2, 0.3)), "ball"),
        ("Pill", {"RigidBody": body(), "Collider": collider({"type": "capsule", "half_height": 0.35, "radius": 0.25}, friction=0.4)},
         transform((1.6, 2.0, -0.2), quat_axis((0, 0, 1), 55)), "pill"),
        # filter 0: collides with no group, so it must fall through the floor.
        ("Ghost (filter 0)", {"RigidBody": body(), "Collider": collider({"type": "sphere", "radius": 0.3}, filter=0)},
         transform((3.2, 2.6, 0.6)), "ghost"),
    ]
    commands = []
    ids = {}
    for name, components, xform, asset in entities:
        components = dict(components)
        components["Transform"] = xform
        components["MeshRenderer"] = {"mesh": assets[asset], "materials": [], "cast_shadows": True}
        commands.append({"op": "create_entity", "scene_id": scene_id,
                         "entity": {"id": new_ulid(len(commands)), "name": name, "parent": None, "components": components}})
        ids[name] = commands[-1]["entity"]["id"]
    camera_id = new_ulid(90)
    commands.append({"op": "create_entity", "scene_id": scene_id, "entity": {
        "id": camera_id, "name": "Look-dev camera", "parent": None, "components": {
            "Camera": {"fov_degrees": 45, "near": 0.1, "far": 100},
            "Transform": transform((0.4, 2.2, 8.2), quat_axis((1, 0, 0), -10))}}})
    sun = quat_mul(quat_axis((0, 1, 0), 235), quat_axis((1, 0, 0), -48))
    commands.append({"op": "create_entity", "scene_id": scene_id, "entity": {
        "id": new_ulid(91), "name": "Sun", "parent": None, "components": {
            "DirectionalLight": {"color": [1, 0.96, 0.9], "intensity": 3.2, "shadows": {"distance": 30}},
            "Transform": transform((0, 0, 0), sun)}}})

    revision = json.loads(rpc([{"id": 1, "method": "project.read"}])[0])["result"]["revision"]
    responses = rpc([
        {"id": 2, "method": "command.execute", "params": {"commands": commands, "expected_revision": revision,
                                                          "description": "Physics look-dev scene"}},
        {"id": 3, "method": "project.save"},
    ])
    for line in responses:
        reply = json.loads(line)
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run("validate", PROJECT)
    frames = OUT / "play"
    report = run("play", PROJECT, "--seconds", "2.5", "--output", frames, "--camera", camera_id,
                 "--capture-every", "6", "--width", "960", "--height", "540")
    (OUT / "play-stdout.json").write_text(report)
    (OUT / "ids.json").write_text(json.dumps({"camera": camera_id, "entities": ids, "assets": assets}, indent=1))
    print(json.dumps({"project": str(PROJECT), "frames": str(frames), "camera": camera_id}))


def rpc(requests):
    result = subprocess.run([str(CLI), "rpc", str(PROJECT), "--journal", str(JOURNAL)],
                            input="".join(json.dumps(r) + "\n" for r in requests),
                            capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"rpc failed: {result.stderr}")
    return [line for line in result.stdout.splitlines() if line.strip()]


def new_ulid(n):
    """Deterministic, well-formed ULIDs so reruns produce identical documents."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA2PHYS0" + suffix


if __name__ == "__main__":
    main()
