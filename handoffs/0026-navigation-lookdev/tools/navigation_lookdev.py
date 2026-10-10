#!/usr/bin/env python3
"""Real-engine navigation look-dev for handoff 0026.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then records actual motion with the CLI `play` command:

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json.
  2. `incant_headless init` creates an empty project.
  3. This script writes original mathematical glTF geometry with the visual
     dimensions baked into the vertices; every physics root has unit scale.
  4. `incant_headless import` registers each model through the command bus.
  5. `incant_headless rpc --journal ...` creates every entity in one
     `command.execute` transaction (colliders match the meshes exactly, and a
     NavigationMesh lists its explicit same-scene sources), then `project.save`.
  6. `node tools/build_script.mjs` compiles navigation_course.ts.
  7. `incant_headless validate`, then `play --compiled-script --camera
     --log-output` once per requested run. Frames are the engine's own GPU output.
  8. Re-verifies the binary hash.

Usage (repo root):

  python3 -I handoffs/0026-navigation-lookdev/tools/navigation_lookdev.py OUT_DIR \
      [--ticks N] [--run NAME:CAMERA:CAPTURE_EVERY ...]

CAMERA is one of overview, plan, pillar, pillar_south, ridge, profile. OUT_DIR must not exist.
"""

import argparse
import hashlib
import json
import math
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "artifacts/tools/incant_headless"
BINARY_JSON = ROOT / "artifacts/tools/binary.json"
BEHAVIOR = Path(__file__).resolve().with_name("navigation_course.ts")
ALL = 0xFFFFFFFF
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024
POOL = 48  # route marker entities per pool; the behavior logs an error if a route exceeds it

# Room: x east (-8..8), z south / toward the default camera (-5..5), +Y up.
FLOOR_HALF = (8.0, 0.1, 5.0)
WALL_H, WALL_T = 1.0, 0.3
CAPSULE_HALF, CAPSULE_RADIUS = 0.5, 0.3  # 1.6 m character
START, GOAL, POCKET = (-6.5, 0.0, 1.2), (6.6, 0.0, 1.2), (6.5, 0.0, 4.2)
BARRIER_HALF = (0.15, WALL_H / 2, 0.8)
BARRIER_PARKED = ((-0.45, WALL_H / 2, 3.4), 0.0)  # stands against the partition, south of doorway A
BARRIER_CLOSED = ((0.0, WALL_H / 2, 1.2), 0.0)  # fills doorway A, z 0.4..2.0

NAVIGATION = {
    "min": [-8, -1, -5], "max": [8, 2.5, 5], "cell_size": 0.1, "cell_height": 0.05,
    "tile_cells": 32, "agent_radius": 0.4, "agent_height": 1.7, "max_climb": 0.25,
    "max_slope_degrees": 45,
}

# Neutral greys for structure; one warm accent for the live route; a cool grey for
# the superseded route; muted blue for the edited piece; brick for the null target.
PALETTE = {
    "floor_a": ((0.30, 0.30, 0.29), 0.92),
    "floor_b": ((0.34, 0.34, 0.33), 0.92),
    "wall": ((0.46, 0.45, 0.43), 0.85),
    "pillar": ((0.40, 0.39, 0.37), 0.6),
    "ridge": ((0.36, 0.35, 0.33), 0.85),
    "barrier": ((0.20, 0.36, 0.60), 0.7),
    "character": ((0.88, 0.85, 0.79), 0.45),
    "route": ((0.96, 0.64, 0.16), 0.5),
    "old_route": ((0.62, 0.65, 0.70), 0.6),
    "start_pad": ((0.62, 0.62, 0.60), 0.8),
    "goal_pad": ((0.70, 0.47, 0.14), 0.7),
    "pocket_pad": ((0.62, 0.17, 0.13), 0.7),
}

CAMERAS = {
    "overview": ((-0.6, 9.6, 10.4), (0.3, 0.0, 0.4)),
    "plan": ((0.0, 15.2, 0.001), (0.0, 0.0, 0.0)),
    "pillar": ((-6.4, 1.45, -4.6), (-3.4, 0.55, 0.7)),
    # Added in revision 4: the route has passed SOUTH of the pillar since v7, so the
    # original north-side "pillar" camera (kept for comparison) sees it occluded.
    "pillar_south": ((-6.2, 1.45, 5.8), (-3.0, 0.55, 1.4)),
    "ridge": ((1.6, 1.15, -7.4), (1.4, 0.25, -3.4)),
    # Narrow lens, perpendicular to the doorway-B ridge crossing: route heights in profile.
    "profile": ((1.3, 0.55, -8.2), (1.3, 0.12, -3.45), 18),
}


def run(*args, cwd=ROOT):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(f"{args[:2]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(CLI.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


# ---- geometry (meters, baked; +Y up) -------------------------------------------------

def box(hx, hy, hz, offset=(0, 0, 0), skip_top=False):
    positions, normals, indices = [], [], []
    faces = [((1, 0, 0), (0, 1, 0)), ((-1, 0, 0), (0, 1, 0)), ((0, 1, 0), (0, 0, 1)),
             ((0, -1, 0), (0, 0, 1)), ((0, 0, 1), (0, 1, 0)), ((0, 0, -1), (0, 1, 0))]
    half = (hx, hy, hz)
    for n, u in faces:
        if skip_top and n == (0, 1, 0):
            continue
        v = (n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0])
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple((n[i] + su * u[i] + sv * v[i]) * half[i] + offset[i] for i in range(3)))
            normals.append(n)
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def checker_top(hx, hy, hz, parity):
    """1 m tiles of the slab's top face (y = +hy) whose (i + j) % 2 == parity."""
    positions, normals, indices = [], [], []
    for i in range(int(2 * hx)):
        for j in range(int(2 * hz)):
            if (i + j) % 2 != parity:
                continue
            x0, z0 = -hx + i, -hz + j
            base = len(positions)
            for x, z in ((x0, z0), (x0, z0 + 1), (x0 + 1, z0 + 1), (x0 + 1, z0)):
                positions.append((x, hy, z))
                normals.append((0, 1, 0))
            indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def cylinder(radius, half_height, segments=32):
    positions, normals, indices = [], [], []
    for y, ny in ((half_height, 1), (-half_height, -1)):
        center = len(positions)
        positions.append((0, y, 0))
        normals.append((0, ny, 0))
        for s in range(segments):
            t = 2 * math.pi * s / segments
            positions.append((radius * math.cos(t), y, radius * math.sin(t)))
            normals.append((0, ny, 0))
        for s in range(segments):
            a, b = center + 1 + s, center + 1 + (s + 1) % segments
            indices += [center, b, a] if ny > 0 else [center, a, b]
    base = len(positions)
    for s in range(segments + 1):
        t = 2 * math.pi * s / segments
        for y in (half_height, -half_height):
            positions.append((radius * math.cos(t), y, radius * math.sin(t)))
            normals.append((math.cos(t), 0, math.sin(t)))
    for s in range(segments):
        a = base + 2 * s
        indices += [a, a + 2, a + 1, a + 1, a + 2, a + 3]
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


def write_gltf(models, name, primitives):
    """primitives: [(mesh, palette key)], one glTF material per primitive."""
    blob, views, accessors, prims, materials = b"", [], [], [], []
    for k, ((positions, normals, indices), key) in enumerate(primitives):
        color, roughness = PALETTE[key]
        for data, target in ((b"".join(struct.pack("<3f", *p) for p in positions), 34962),
                             (b"".join(struct.pack("<3f", *n) for n in normals), 34962),
                             (b"".join(struct.pack("<I", i) for i in indices), 34963)):
            views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(data), "target": target})
            blob += data
        lo = [min(p[i] for p in positions) for i in range(3)]
        hi = [max(p[i] for p in positions) for i in range(3)]
        accessors += [
            {"bufferView": 3 * k, "componentType": 5126, "count": len(positions), "type": "VEC3", "min": lo, "max": hi},
            {"bufferView": 3 * k + 1, "componentType": 5126, "count": len(normals), "type": "VEC3"},
            {"bufferView": 3 * k + 2, "componentType": 5125, "count": len(indices), "type": "SCALAR"},
        ]
        prims.append({"attributes": {"POSITION": 3 * k, "NORMAL": 3 * k + 1}, "indices": 3 * k + 2, "material": k})
        materials.append({"name": key, "pbrMetallicRoughness": {
            "baseColorFactor": [*color, 1], "metallicFactor": 0, "roughnessFactor": roughness}})
    (models / f"{name}.bin").write_bytes(blob)
    document = {
        "asset": {"version": "2.0", "generator": "handoff 0026 navigation_lookdev.py"},
        "buffers": [{"uri": f"{name}.bin", "byteLength": len(blob)}],
        "bufferViews": views, "accessors": accessors,
        "meshes": [{"name": name, "primitives": prims}], "materials": materials,
        "nodes": [{"name": name, "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0,
    }
    path = models / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1), encoding="utf-8")
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


def transform(t, rotation=(0, 0, 0, 1), scale=(1, 1, 1)):
    return {"translation": [round(v, 6) for v in t], "rotation": [round(v, 9) for v in rotation],
            "scale": list(scale)}


def collider(shape):
    return {"shape": shape, "density": 1000, "friction": 0.6, "restitution": 0, "sensor": False,
            "memberships": ALL, "filter": ALL}


def span_box(name, x0, x1, z0, z1, height=WALL_H, key="wall"):
    half = ((x1 - x0) / 2, height / 2, (z1 - z0) / 2)
    return {"name": name, "key": key, "shape": "box", "half": half,
            "center": ((x0 + x1) / 2, height / 2, (z0 + z1) / 2), "yaw": 0.0, "nav": "collider"}


def pieces():
    """Static, source-listed structure. Every mesh equals its collider exactly."""
    t = WALL_T / 2
    out = [
        {"name": "Floor", "key": "floor", "shape": "box", "half": FLOOR_HALF,
         "center": (0, -FLOOR_HALF[1], 0), "yaw": 0.0, "nav": "mesh"},
        span_box("Partition north", -t, t, -5.0, -4.4),
        span_box("Partition middle", -t, t, -2.8, 0.4),
        span_box("Partition south", -t, t, 2.0, 5.0),
        span_box("Screen wall", 3.6 - t, 3.6 + t, -3.4, 0.2),
        span_box("Pocket wall west of slit", 5.0, 6.25, 3.0 - t, 3.0 + t),
        span_box("Pocket wall east of slit", 6.75, 8.0, 3.0 - t, 3.0 + t),
        span_box("Pocket side wall", 5.0 - t, 5.0 + t, 3.0 - t, 5.0),
        {**span_box("Ridge 0.15 m", 1.0, 1.6, -5.0, 5.0, height=0.15, key="ridge"), "nav": "mesh"},
        {"name": "Pillar", "key": "pillar", "shape": "capsule", "half_height": 0.7, "radius": 0.45,
         "center": (-3.6, 0.7, 1.0), "yaw": 0.0, "nav": "collider"},
        {"name": "Barrier", "key": "barrier", "shape": "box", "half": BARRIER_HALF,
         "center": BARRIER_PARKED[0], "yaw": BARRIER_PARKED[1], "nav": "collider"},
    ]
    return out


def mesh_for(piece):
    if piece["shape"] == "capsule":
        return [(capsule(piece["half_height"], piece["radius"]), piece["key"])]
    if piece["key"] == "floor":
        hx, hy, hz = piece["half"]
        return [(box(hx, hy, hz, skip_top=True), "floor_a"),
                (checker_top(hx, hy, hz, 0), "floor_a"), (checker_top(hx, hy, hz, 1), "floor_b")]
    return [(box(*piece["half"]), piece["key"])]


def build(out):
    project = out / "navigation.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Navigation Look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, primitives):
        path = write_gltf(models, name, primitives)
        imported = json.loads(run(CLI, "import", project, path.relative_to(out)))
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    counter = iter(range(1, 10_000))
    entities, sources, assets, ids = [], [], {}, {}

    def entity(name, components):
        entity_id = new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": None, "components": components})
        return entity_id

    for i, piece in enumerate(pieces()):
        asset = importer(f"piece{i:02d}_{piece['key']}", mesh_for(piece))
        assets[piece["name"]] = asset
        if piece["shape"] == "capsule":
            shape = {"type": "capsule", "half_height": piece["half_height"], "radius": piece["radius"]}
        else:
            shape = {"type": "box", "half_extents": list(piece["half"])}
        entity_id = entity(piece["name"], {
            "Transform": transform(piece["center"], quat_axis((0, 1, 0), piece["yaw"])),
            "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": piece["key"] != "floor"},
            "Collider": collider(shape),
        })
        sources.append({"entity": entity_id, "geometry": piece["nav"]})
        ids[piece["name"]] = entity_id

    # Pads are flat render-only discs (no collider, not navigation sources).
    for name, key, at, radius in (("Start pad", "start_pad", START, 0.34), ("Goal pad", "goal_pad", GOAL, 0.38),
                                  ("Pocket target (expected null)", "pocket_pad", POCKET, 0.34)):
        asset = importer(key, [(cylinder(radius, 0.003), key)])
        entity(name, {"Transform": transform((at[0], 0.004, at[2])),
                      "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": False}})

    # Route marker pools: behavior-placed geometry, parked far below the floor.
    for pool, key in (("active", "route"), ("old", "old_route")):
        dot = importer(f"{pool}_dot", [(cylinder(0.11, 0.008, 24), key)])
        link = importer(f"{pool}_link", [(box(0.5, 0.004, 0.04), key)])
        for i in range(POOL):
            for kind, asset in (("dot", dot), ("link", link)):
                entity(f"Route {pool} {kind} {i:02d}", {
                    "Transform": transform((0, -30, 0)),
                    "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": False}})

    character = importer("character", [(capsule(CAPSULE_HALF, CAPSULE_RADIUS), "character")])
    agent = entity("Agent", {
        "Transform": transform((START[0], CAPSULE_HALF + CAPSULE_RADIUS + 0.03, START[2])),
        "MeshRenderer": {"mesh": character, "materials": [], "cast_shadows": True},
        "RigidBody": {"motion": "kinematic", "gravity_scale": 0, "linear_damping": 0,
                      "angular_damping": 0, "can_sleep": False, "ccd": False},
        "Collider": collider({"type": "capsule", "half_height": CAPSULE_HALF, "radius": CAPSULE_RADIUS}),
        "Velocity": {"linear": [0, 0, 0]},
    })
    nav = entity("Navigation", {"NavigationMesh": {"settings": NAVIGATION, "sources": sources}})
    cameras = {}
    for name, (eye, target, *fov) in CAMERAS.items():
        cameras[name] = entity(f"Camera · {name}", {
            "Camera": {"fov_degrees": fov[0] if fov else 42, "near": 0.1, "far": 100},
            "Transform": transform(eye, look(eye, target))})
    sun = quat_mul(quat_axis((0, 1, 0), 215), quat_axis((1, 0, 0), -55))
    entity("Sun", {"DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.4, "shadows": {"distance": 40}},
                   "Transform": transform((0, 0, 0), sun)})

    commands = [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities]
    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Navigation look-dev room"}},
        {"id": 3, "method": "project.save"},
    ]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    script = out / "navigation_course.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    geometry = {
        "scene": scene_id, "agent": agent, "navigation": nav, "cameras": cameras, "assets": assets, "ids": ids,
        "settings": NAVIGATION, "start": START, "goal": GOAL, "pocket": POCKET,
        "capsule": {"half_height": CAPSULE_HALF, "radius": CAPSULE_RADIUS},
        "pieces": pieces(), "barrier_closed": {"center": BARRIER_CLOSED[0], "yaw": BARRIER_CLOSED[1]},
    }
    (out / "geometry.json").write_text(json.dumps(geometry, indent=1), encoding="utf-8")
    return project, script, cameras


def play(project, script, out, name, camera, every, ticks):
    frames = 1 + math.ceil(ticks / every)
    if frames > MAX_FRAMES or frames * WIDTH * HEIGHT * 4 > MAX_RAW:
        raise SystemExit(f"run {name}: {frames} frames exceeds the capture budget")
    report = run(CLI, "play", project, "--ticks", ticks, "--compiled-script", script,
                 "--output", out / name, "--camera", camera, "--capture-every", every,
                 "--width", WIDTH, "--height", HEIGHT, "--log-output", out / f"{name}.logs.jsonl")
    (out / f"{name}.stdout.json").write_text(report, encoding="utf-8")


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
    return "01JA2NAV00" + suffix


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--ticks", type=int, default=560)
    parser.add_argument("--run", action="append", default=[], help="NAME:CAMERA:CAPTURE_EVERY")
    args = parser.parse_args()
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    runs = [r.split(":") for r in args.run] or [["overview", "overview", "5"]]
    for name, camera, every in runs:
        if camera not in CAMERAS or not every.isdigit() or int(every) < 1:
            raise SystemExit(f"invalid --run {name}:{camera}:{every}")
        if 1 + math.ceil(args.ticks / int(every)) > MAX_FRAMES:
            raise SystemExit(f"run {name}: exceeds {MAX_FRAMES} frames")
    before = verify_binary()
    out.mkdir(parents=True)
    project, script, cameras = build(out)
    for name, camera, every in runs:
        play(project, script, out, name, cameras[camera], int(every), args.ticks)
    after = verify_binary()
    summary = {"project": str(project), "runs": [r[0] for r in runs], "ticks": args.ticks,
               "binary_sha256_before": before, "binary_sha256_after": after}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
