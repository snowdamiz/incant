#!/usr/bin/env python3
"""Real-engine compound collider look-dev for handoff 0024.

Builds a disposable project in a NEW output directory using only engine
interfaces, then records actual simulation with the CLI `play` command:

  1. `incant_headless init` creates an empty project.
  2. This script writes static glTF models. A compound's model is the union of
     its parts' primitive meshes, each baked at the part's own local offset and
     rotation, so the rendered geometry is exactly the authored collider.
     Every physics root has unit scale.
  3. `incant_headless import` registers each model through the command bus.
  4. `incant_headless rpc --journal ...` creates every entity in one
     `command.execute` transaction (provenance in the journal), then
     `project.save`.
  5. `node tools/build_script.mjs` compiles compound_course.ts (characters walk
     with the engine's read-only computeCharacterMotion; props are dynamic
     bodies under engine gravity, never scripted).
  6. `incant_headless validate`, then `play --compiled-script --camera` once per
     requested run. Frames are the engine's own GPU output; nothing is faked.

Usage (repo root):

  python3 -I handoffs/0024-compound-inspector/tools/compound_lookdev.py OUT_DIR \\
      [--run NAME:CAMERA:CAPTURE_EVERY ...]

CAMERA is `overview`, `arch` (looking back through the arch opening) or `props`.
The output directory must not exist.
"""

import argparse
import json
import math
import struct
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "artifacts/tools/incant_headless"
BEHAVIOR = Path(__file__).resolve().with_name("compound_course.ts")
ALL = 0xFFFFFFFF
SECONDS = 5.0
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

# Lanes run along +X and are stacked along Z (near the overview camera = +Z): the low
# terrace nearest, the arch behind it, the wall at the back, so nothing tall hides another lane.
LANE_Z = {"terrace": 2.4, "arch": 0.0, "wall": -2.4}
START_X = -3.5
CAPSULE_HALF, CAPSULE_RADIUS = 0.5, 0.3  # 1.6 m tall character

PALETTE = {
    "backdrop": ((0.09, 0.09, 0.10), 0.98),
    "floor": ((0.24, 0.24, 0.23), 0.92),
    "lane": ((0.17, 0.17, 0.17), 0.95),
    "character": ((0.86, 0.82, 0.74), 0.45),
    "arch": ((0.62, 0.52, 0.40), 0.85),
    "terrace": ((0.20, 0.34, 0.52), 0.7),
    "wall": ((0.22, 0.42, 0.30), 0.8),
    "stool": ((0.82, 0.50, 0.10), 0.55),
    "dumbbell": ((0.62, 0.14, 0.12), 0.4),
}


def run(*args, cwd=ROOT):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(f"{args[:2]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


# ---- math ----------------------------------------------------------------------------

def quat_axis(axis, degrees):
    s = math.sin(math.radians(degrees) / 2)
    return [axis[0] * s, axis[1] * s, axis[2] * s, math.cos(math.radians(degrees) / 2)]


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def rotate(q, v):
    x, y, z, w = q
    # v' = v + 2w(q×v) + 2 q×(q×v)
    cx, cy, cz = y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0]
    dx, dy, dz = y * cz - z * cy, z * cx - x * cz, x * cy - y * cx
    return (v[0] + 2 * (w * cx + dx), v[1] + 2 * (w * cy + dy), v[2] + 2 * (w * cz + dz))


def look(eye, target):
    """Rotation for a -Z-forward camera at eye looking at target (yaw then pitch)."""
    d = [target[i] - eye[i] for i in range(3)]
    yaw = math.degrees(math.atan2(-d[0], -d[2]))
    pitch = math.degrees(math.atan2(d[1], math.hypot(d[0], d[2])))
    return quat_mul(quat_axis((0, 1, 0), yaw), quat_axis((1, 0, 0), pitch))


# ---- geometry (meters, baked; +Y up) -------------------------------------------------

def box_mesh(hx, hy, hz):
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


def capsule_mesh(half_height, radius, rings=12, segments=32):
    """Y capsule; half_height excludes the hemispheres (0 gives a sphere)."""
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


def shape_mesh(shape):
    if shape["type"] == "box":
        return box_mesh(*shape["half_extents"])
    if shape["type"] == "sphere":
        return capsule_mesh(0.0, shape["radius"])
    if shape["type"] == "capsule":
        return capsule_mesh(shape["half_height"], shape["radius"])
    raise ValueError(shape)


def compound_mesh(parts):
    """Union of the parts' meshes, each placed by its authored local offset and rotation."""
    positions, normals, indices = [], [], []
    for part in parts:
        p, n, i = shape_mesh(part["shape"])
        base = len(positions)
        q, t = part["rotation"], part["translation"]
        positions += [tuple(a + b for a, b in zip(rotate(q, v), t)) for v in p]
        normals += [rotate(q, v) for v in n]
        indices += [base + k for k in i]
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
        "asset": {"version": "2.0", "generator": "handoff 0024 compound_lookdev.py"},
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

IDENTITY = [0, 0, 0, 1]


def part(serial, translation, shape, rotation=IDENTITY):
    return {"id": new_ulid(500 + serial, "01JA2CPART"), "translation": [round(v, 6) for v in translation],
            "rotation": [round(v, 9) for v in rotation], "shape": shape}


def box(hx, hy, hz):
    return {"type": "box", "half_extents": [hx, hy, hz]}


def ramp_part(serial, x0, y0, x1, y1, half_z, thickness=0.12):
    """Box part whose top face runs from (x0, y0) to (x1, y1) in the entity's XY plane."""
    length = math.hypot(x1 - x0, y1 - y0)
    angle = math.atan2(y1 - y0, x1 - x0)
    nx, ny = -math.sin(angle), math.cos(angle)
    cx = (x0 + x1) / 2 - nx * thickness / 2
    cy = (y0 + y1) / 2 - ny * thickness / 2
    return part(serial, (cx, cy, 0), box(length / 2, thickness / 2, half_z), quat_axis((0, 0, 1), math.degrees(angle)))


# Upright 2 m drop by default. Tilted drops (e.g. --stool 9:0.05) reproduce the
# suspected compound toppling defect reported in result.md.
STOOL_DROP = {"tilt": 0.0, "height": 2.0}


def compounds():
    """(name, palette key, entity translation, entity rotation, parts, dynamic)."""
    # Arch across the arch lane: piers either side of the lane, a lintel, and a keystone
    # turned 45° about X so it reads as a diamond to cameras along the lane.
    arch = [
        part(1, (0, 0.9, -0.75), box(0.25, 0.9, 0.25)),
        part(2, (0, 0.9, 0.75), box(0.25, 0.9, 0.25)),
        part(3, (0, 1.95, 0), box(0.3, 0.15, 1.0)),
        part(4, (0, 2.1, 0), box(0.2, 0.2, 0.2), quat_axis((1, 0, 0), 45)),
    ]
    # Terrace: one fixed body made of a 17° ramp (a rotated local part), a 0.5 m deck and a
    # low step down, which the character walks over continuously.
    terrace = [
        ramp_part(10, -1.6, 0.0, 0.0, 0.5, 0.7),
        part(11, (0.7, 0.25, 0), box(0.7, 0.25, 0.7)),
        part(12, (1.7, 0.125, 0), box(0.3, 0.125, 0.7)),
    ]
    # L-shaped wall: a long run and a return turned 35° about Y at its far end.
    wall = [
        part(20, (0.6, 0.5, 0), box(2.4, 0.5, 0.1)),
        part(21, (3.0 + 0.6 * math.cos(math.radians(35)), 0.5, 0.6 * math.sin(math.radians(35))),
             box(0.6, 0.5, 0.1), quat_axis((0, 1, 0), -35)),
    ]
    # Stool: seat plus three capsule legs; dropped tilted so it lands on two legs and rights itself.
    legs = [(0.22 * math.cos(math.radians(a)), 0.22 * math.sin(math.radians(a))) for a in (90, 210, 330)]
    stool = [part(30, (0, 0.47, 0), box(0.3, 0.04, 0.3))] + [
        part(31 + i, (x, 0.22, z), {"type": "capsule", "half_height": 0.17, "radius": 0.04})
        for i, (x, z) in enumerate(legs)
    ]
    # Dumbbell: a bar (capsule turned 90° about Z, so it lies along local X) and two spheres.
    dumbbell = [
        part(40, (0, 0, 0), {"type": "capsule", "half_height": 0.35, "radius": 0.05}, quat_axis((0, 0, 1), 90)),
        part(41, (-0.42, 0, 0), {"type": "sphere", "radius": 0.16}),
        part(42, (0.42, 0, 0), {"type": "sphere", "radius": 0.16}),
    ]
    return [
        ("Arch", "arch", (0.6, 0, LANE_Z["arch"]), IDENTITY, arch, False),
        ("Terrace", "terrace", (0.4, 0, LANE_Z["terrace"]), IDENTITY, terrace, False),
        ("L wall", "wall", (-1.0, 0, LANE_Z["wall"] - 0.75), IDENTITY, wall, False),
        # Released from rest; engine gravity does the rest.
        ("Falling stool", "stool", (3.9, STOOL_DROP["height"], -1.2),
         quat_mul(quat_axis((0, 1, 0), 25), quat_axis((1, 0, 0), STOOL_DROP["tilt"])), stool, True),
        ("Falling dumbbell", "dumbbell", (2.6, 1.8, 3.7),
         quat_mul(quat_axis((0, 1, 0), -30), quat_axis((0, 0, 1), 18)), dumbbell, True),
    ]


CAMERAS = {
    "overview": ((-5.8, 6.8, 9.8), (0.8, 0.2, -0.3)),
    # Low, down the arch lane from beyond its end: the character walks toward the camera
    # through the opening, framed by the piers and lintel.
    "arch": ((6.4, 1.3, 0.55), (-2.5, 0.95, 0.0)),
    "props": ((8.2, 2.6, 5.6), (3.1, 0.35, 1.1)),
}


def transform(t, rotation=IDENTITY):
    return {"translation": [round(v, 6) for v in t], "rotation": [round(v, 9) for v in rotation], "scale": [1, 1, 1]}


def collider(shape, density=1000):
    return {"shape": shape, "density": density, "friction": 0.6, "restitution": 0.05 if density != 1000 else 0,
            "sensor": False, "memberships": ALL, "filter": ALL}


def body(motion):
    return {"motion": motion, "gravity_scale": 1 if motion == "dynamic" else 0, "linear_damping": 0,
            "angular_damping": 0.05 if motion == "dynamic" else 0, "can_sleep": motion != "kinematic", "ccd": motion == "dynamic"}


def build(out):
    project = out / "compound.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Compound Look-dev", "--entities", "0")

    assets, entities = {}, []

    def model(name, key, mesh):
        color, rough = PALETTE[key]
        path = write_gltf(models, name, mesh, color, rough)
        assets[name] = import_asset(project, out, path)
        return assets[name]

    entities.append(("Backdrop", {  # decorative ground beyond the stage; no collider
        "Transform": transform((0.5, -0.215, 0)),
        "MeshRenderer": {"mesh": model("backdrop", "backdrop", box_mesh(30, 0.01, 30)), "materials": [], "cast_shadows": False},
    }))
    floor = (5.4, 0.1, 4.2)
    entities.append(("Floor", {
        "Transform": transform((0.5, -0.1, 0)),
        "MeshRenderer": {"mesh": model("floor", "floor", box_mesh(*floor)), "materials": [], "cast_shadows": False},
        "Collider": collider({"type": "box", "half_extents": list(floor)}),
    }))
    for lane, z in LANE_Z.items():
        strip = model(f"lane_{lane}", "lane", box_mesh(4.9, 0.005, 0.55))
        entities.append((f"Lane strip · {lane}", {
            "Transform": transform((0.5, 0.005, z)),
            "MeshRenderer": {"mesh": strip, "materials": [], "cast_shadows": False},
        }))
    compound_records = {}
    for name, key, at, rotation, parts, dynamic in compounds():
        mesh = model(key, key, compound_mesh(parts))
        density = {"stool": 500, "dumbbell": 1200}.get(key, 1000)
        components = {
            "Transform": transform(at, rotation),
            "MeshRenderer": {"mesh": mesh, "materials": [], "cast_shadows": True},
            "RigidBody": body("dynamic" if dynamic else "fixed"),
            "Collider": collider({"type": "compound", "parts": parts}, density),
        }
        if dynamic:
            components["Velocity"] = {"linear": [0, 0, 0]}
        entities.append((name, components))
        compound_records[name] = {"parts": len(parts), "dynamic": dynamic}

    character = model("character", "character", capsule_mesh(CAPSULE_HALF, CAPSULE_RADIUS))
    start_y = CAPSULE_HALF + CAPSULE_RADIUS + 0.03
    for lane, z in LANE_Z.items():
        start_z = z + 0.35 if lane == "wall" else z
        entities.append((f"Character · {lane}", {
            "Transform": transform((START_X, start_y, start_z)),
            "MeshRenderer": {"mesh": character, "materials": [], "cast_shadows": True},
            "RigidBody": body("kinematic"),
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
    replies = rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Compound collider look-dev scene"}},
        {"id": 3, "method": "project.save"},
    ])
    for reply in replies:
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    script = out / "compound_course.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    (out / "ids.json").write_text(json.dumps(
        {"scene": scene_id, "cameras": cameras, "entities": ids, "assets": assets, "compounds": compound_records,
         "rpc": replies}, indent=1))
    return project, script, cameras


def play(project, script, out, name, camera, every, capture=True):
    ticks = math.ceil(SECONDS * 60)
    frames = 1 + math.ceil(ticks / every)
    if frames > MAX_FRAMES or frames * WIDTH * HEIGHT * 4 > MAX_RAW:
        raise SystemExit(f"run {name}: {frames} frames exceeds the capture budget")
    started = time.monotonic()
    capture_args = (["--output", out / name, "--camera", camera, "--capture-every", every,
                     "--width", WIDTH, "--height", HEIGHT] if capture else [])
    report = run(CLI, "play", project, "--seconds", SECONDS, "--compiled-script", script,
                 *capture_args, "--log-output", out / f"{name}.logs.jsonl")
    elapsed = time.monotonic() - started
    (out / f"{name}.stdout.json").write_text(report)
    (out / f"{name}.timing.json").write_text(json.dumps(
        {"wall_seconds": round(elapsed, 3), "ticks": ticks, "frames": frames if capture else 0,
         "raw_bytes": frames * WIDTH * HEIGHT * 4 if capture else 0,
         "size": [WIDTH, HEIGHT], "capture_every": every}))


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


def new_ulid(n, prefix="01JA2CMPND"):
    """Deterministic, well-formed ULIDs so reruns produce identical documents."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return prefix + suffix


def main():
    global CLI
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--run", action="append", default=[], help="NAME:CAMERA:CAPTURE_EVERY")
    parser.add_argument("--stool", help="TILT_DEGREES:DROP_HEIGHT for the stool (default 0:2.0)")
    parser.add_argument("--binary", type=Path, default=CLI, help="explicit headless executable")
    parser.add_argument("--no-captures", action="store_true", help="build and replay numerically; no GPU frames")
    args = parser.parse_args()
    CLI = args.binary.resolve()
    if args.stool:
        tilt, height = (float(v) for v in args.stool.split(":"))
        if not (0 <= tilt <= 90 and 0.05 <= height <= 5):
            raise SystemExit("--stool needs 0..90 degrees and 0.05..5 m")
        STOOL_DROP.update(tilt=tilt, height=height)
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    runs = [r.split(":") for r in args.run] or [["overview", "overview", "4"]]
    for name, camera, every in runs:
        if camera not in CAMERAS or not every.isdigit() or int(every) < 1:
            raise SystemExit(f"invalid --run {name}:{camera}:{every}")
    project, script, cameras = build(out)
    for name, camera, every in runs:
        play(project, script, out, name, cameras[camera], int(every), capture=not args.no_captures)
    print(json.dumps({"project": str(project), "runs": [r[0] for r in runs]}))


if __name__ == "__main__":
    main()
