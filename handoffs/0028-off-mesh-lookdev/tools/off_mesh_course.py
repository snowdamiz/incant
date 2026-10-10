#!/usr/bin/env python3
"""Real-engine off-mesh link look-dev for handoff 0028.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then records actual motion with the CLI `play` command.

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json
     before and after every engine invocation.
  2. `incant_headless init` creates an empty project.
  3. This script writes original mathematical glTF geometry (feet-origin walker,
     terraced platforms, render-only link pads and marker dots).
  4. `incant_headless import` registers every model through the command bus.
  5. `incant_headless rpc --journal ...` creates every entity in one
     `command.execute` transaction, then `project.save`. No project JSON is
     edited directly. The three platform MeshRenderers are the navigation
     sources (`geometry: "mesh"`); there are no RigidBody or Collider components.
  6. Strict `tsc -p tools/tsconfig.json`, then `node tools/build_script.mjs`.
  7. `incant_headless validate`, then `play`:
       reference   log-only full course (0..--ticks)
       reports     log-only runs ending at the ticks around each link toggle;
                   stdout `state.navigation` carries generation/tile reuse
       resume      a `--save-output` run that stops mid-way through the return
                   drop, then `--load-save` to finish; compared with reference
       --run NAME:CAMERA:EVERY        full-course capture
       --window NAME:CAMERA:EVENT:BEFORE:TICKS:EVERY
                   save at (EVENT tick - BEFORE), reopen and capture TICKS ticks.
                   EVENT is `launch-<key>-<n>` / `land-<key>-<n>` / `mid-ret`
                   resolved from the reference log (n counts from 1).
     Frames are the engine's own GPU output; logs are committed script logs.

Usage (repo root):

  caffeinate -s -i python3 -I handoffs/0028-off-mesh-lookdev/tools/off_mesh_course.py OUT_DIR \
      [--ticks N] [--run ...] [--window ...]

OUT_DIR must not exist.
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
BEHAVIOR = Path(__file__).resolve().with_name("off_mesh_course.ts")
TSCONFIG = Path(__file__).resolve().with_name("tsconfig.json")
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024
ATTEMPTS = 4  # per play run; the script host has a 50 ms wall-clock tick budget

# x east, z south (toward the cameras), +Y up. Must match off_mesh_course.ts.
FLOOR_Y = -1.5  # dark trench floor; not a navigation source
PLATFORMS = {  # name: (x0, x1, z0, z1, top)
    "West": (-7.0, -3.5, -2.0, 2.0, 0.0),
    "Centre": (-1.75, 1.75, -2.0, 2.0, 0.5),
    "East": (3.5, 7.0, -2.0, 2.0, 0.0),
}
RADIUS, BODY_HEIGHT = 0.25, 1.3
LINKS = [  # id, key, start, end, bidirectional, enabled
    ("01JA2NAV000000000000000001", "up", (-3.9, 0.0, -1.0), (-1.35, 0.5, -1.0), False, True),
    ("01JA2NAV000000000000000002", "bridge", (1.35, 0.5, 0.0), (3.9, 0.0, 0.0), True, True),
    ("01JA2NAV000000000000000003", "ret", (-1.35, 0.5, 1.0), (-3.9, 0.0, 1.0), False, False),
]
SNAP = 0.3
NAVIGATION = {"min": [-8, -0.5, -3], "max": [8, 1.5, 3], "cell_size": 0.1, "cell_height": 0.05,
              "tile_cells": 32, "agent_radius": RADIUS, "agent_height": BODY_HEIGHT,
              "max_climb": 0.2, "max_slope_degrees": 45}
START, GOAL = (-6.2, 0.0, 0.0), (6.2, 0.0, 0.0)
POOLS = {"walk": 80, "up": 24, "bridge": 48, "ret": 24}

# Neutral structure; one hue per link role. Amber = directed jump-up, teal =
# bidirectional bridge, green/grey = return drop enabled/disabled.
PALETTE = {
    "floor": ((0.032, 0.034, 0.038), 0.95),
    "side": ((0.30, 0.30, 0.29), 0.85),
    "top_a": ((0.56, 0.55, 0.52), 0.9),
    "top_b": ((0.50, 0.49, 0.46), 0.9),
    "walker": ((0.90, 0.89, 0.86), 0.5),
    "visor": ((0.06, 0.06, 0.07), 0.4),
    "goal": ((0.96, 0.96, 0.94), 0.6),
    "walk_dot": ((0.18, 0.18, 0.19), 0.7),
    "up": ((0.93, 0.58, 0.12), 0.5),
    "bridge": ((0.10, 0.62, 0.66), 0.5),
    "ret": ((0.30, 0.70, 0.30), 0.5),
    "ret_off": ((0.36, 0.36, 0.36), 0.8),
}

CAMERAS = {
    # name: (eye, target, vertical fov degrees)
    "overview": ((3.2, 7.4, 10.4), (0.0, -0.45, 0.25), 42),
    # Near-orthographic side profile from 40 m: arc shapes and feet heights read
    # true; the z = +1 return lane is magnified by only 40 / 39 = 1.026.
    "profile": ((0.0, 0.25, 40.0), (0.0, 0.25, 0.0), 12),
    "west-gap": ((-0.6, 2.6, 6.2), (-2.6, 0.25, 0.0), 38),
    "east-gap": ((4.6, 2.6, 6.2), (2.6, 0.25, 0.0), 38),
    # Near-level telephoto on the goal mark: feet height against the platform top.
    "goal-feet": ((6.2, 0.4, 9.0), (6.2, 0.62, 0.0), 10),
}


def run(*args, cwd=ROOT):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=cwd)
    if result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:2]]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(CLI.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


# ---- geometry (meters, baked; +Y up) -------------------------------------------------

def box(x0, x1, y0, y1, z0, z1, skip=()):
    positions, normals, indices = [], [], []
    c = ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2)
    half = ((x1 - x0) / 2, (y1 - y0) / 2, (z1 - z0) / 2)
    faces = {"+x": ((1, 0, 0), (0, 1, 0)), "-x": ((-1, 0, 0), (0, 1, 0)), "+y": ((0, 1, 0), (0, 0, 1)),
             "-y": ((0, -1, 0), (0, 0, 1)), "+z": ((0, 0, 1), (0, 1, 0)), "-z": ((0, 0, -1), (0, 1, 0))}
    for key, (n, u) in faces.items():
        if key in skip:
            continue
        v = (n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0])
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple(c[i] + (n[i] + su * u[i] + sv * v[i]) * half[i] for i in range(3)))
            normals.append(n)
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def tiles(x0, x1, z0, z1, y, parity, size=0.5):
    """Checker tiles of `size` whose (i + j) % 2 == parity, on a world-aligned grid."""
    positions, normals, indices = [], [], []
    for i in range(int(math.floor(x0 / size)), int(math.ceil(x1 / size))):
        for j in range(int(math.floor(z0 / size)), int(math.ceil(z1 / size))):
            if (i + j) % 2 != parity:
                continue
            a, b = max(x0, i * size), min(x1, (i + 1) * size)
            c, d = max(z0, j * size), min(z1, (j + 1) * size)
            if b - a < 1e-9 or d - c < 1e-9:
                continue
            base = len(positions)
            for x, z in ((a, c), (a, d), (b, d), (b, c)):
                positions.append((x, y, z))
                normals.append((0, 1, 0))
            indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def disc(radius, y, segments=28):
    positions, normals, indices = [(0, y, 0)], [(0, 1, 0)], []
    for s in range(segments):
        t = 2 * math.pi * s / segments
        positions.append((radius * math.cos(t), y, radius * math.sin(t)))
        normals.append((0, 1, 0))
    for s in range(segments):
        indices += [0, 1 + (s + 1) % segments, 1 + s]
    return positions, normals, indices


def ring(inner, outer, y, segments=36):
    positions, normals, indices = [], [], []
    for s in range(segments):
        t = 2 * math.pi * s / segments
        for r in (inner, outer):
            positions.append((r * math.cos(t), y, r * math.sin(t)))
            normals.append((0, 1, 0))
    for s in range(segments):
        a, b, c, d = 2 * s, 2 * s + 1, 2 * ((s + 1) % segments), 2 * ((s + 1) % segments) + 1
        indices += [a, c, b, b, c, d]
    return positions, normals, indices


def chevron(length, width, thickness, y):
    """Flat arrowhead pointing +X, centred on the origin, `thickness` wide stroke."""
    positions, normals, indices = [], [], []
    tip, back = length / 2, -length / 2
    for side in (1, -1):
        outer = [(back, side * width / 2), (tip, 0.0)]
        inner = [(back + thickness * 1.6, side * width / 2), (tip - thickness * 1.6, 0.0)]
        quad = [outer[0], outer[1], inner[1], inner[0]]
        if side < 0:
            quad.reverse()
        base = len(positions)
        for x, z in quad:
            positions.append((x, y, z))
            normals.append((0, 1, 0))
        indices += [base, base + 2, base + 1, base, base + 3, base + 2]
    return positions, normals, indices


def octahedron(r):
    verts = [(r, 0, 0), (-r, 0, 0), (0, r, 0), (0, -r, 0), (0, 0, r), (0, 0, -r)]
    faces = [(0, 2, 4), (4, 2, 1), (1, 2, 5), (5, 2, 0), (4, 3, 0), (1, 3, 4), (5, 3, 1), (0, 3, 5)]
    positions, normals, indices = [], [], []
    for f in faces:
        a, b, c = (verts[i] for i in f)
        n = [a[i] + b[i] + c[i] for i in range(3)]
        length = math.sqrt(sum(v * v for v in n))
        base = len(positions)
        for p in (a, b, c):
            positions.append(p)
            normals.append(tuple(v / length for v in n))
        indices += [base, base + 1, base + 2]
    return positions, normals, indices


def capsule(radius, height, rings=10, segments=28):
    """Feet-origin capsule of total `height`."""
    half = height / 2 - radius
    centre = height / 2
    rows = [((math.pi / 2) * (1 - r / rings), centre + half) for r in range(rings + 1)]
    rows += [(-(math.pi / 2) * (r / rings), centre - half) for r in range(rings + 1)]
    positions, normals, indices = [], [], []
    for phi, offset in rows:
        for s in range(segments + 1):
            theta = 2 * math.pi * s / segments
            nrm = (math.cos(phi) * math.cos(theta), math.sin(phi), math.cos(phi) * math.sin(theta))
            positions.append((nrm[0] * radius, nrm[1] * radius + offset, nrm[2] * radius))
            normals.append(nrm)
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
        "asset": {"version": "2.0", "generator": "handoff 0028 off_mesh_course.py"},
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


def yaw_toward(a, b):
    """Rotation about +Y turning local +X toward b - a in the XZ plane."""
    return quat_axis((0, 1, 0), math.degrees(math.atan2(-(b[2] - a[2]), b[0] - a[0])))


def behind(a, b, distance=0.42):
    """Point `distance` behind launch point a, away from b, in the XZ plane (arrow spot)."""
    d = math.hypot(b[0] - a[0], b[2] - a[2])
    return (a[0] - distance * (b[0] - a[0]) / d, a[1], a[2] - distance * (b[2] - a[2]) / d)


def transform(t, rotation=(0, 0, 0, 1)):
    return {"translation": [round(v, 6) for v in t], "rotation": [round(v, 9) for v in rotation],
            "scale": [1, 1, 1]}


def new_ulid(n):
    """Deterministic, well-formed ULIDs so reruns produce identical documents."""
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA20FF00" + suffix


def rpc(project, journal, requests):
    result = subprocess.run([str(CLI), "rpc", str(project), "--journal", str(journal)],
                            input="".join(json.dumps(r) + "\n" for r in requests),
                            capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"rpc failed: {result.stderr}")
    return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]


def build(out):
    project = out / "offmesh.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Off-mesh Link Look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, primitives):
        path = write_gltf(models, name, primitives)
        imported = json.loads(run(CLI, "import", project, path.relative_to(out)))
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    counter = iter(range(1, 100_000))
    entities, ids = [], {}

    def entity(name, components):
        entity_id = new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": None, "components": components})
        ids[name] = entity_id
        return entity_id

    def static(name, asset, at=(0, 0, 0), rotation=(0, 0, 0, 1), shadows=True):
        return entity(name, {"Transform": transform(at, rotation),
                             "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": shadows}})

    floor = importer("floor", [(box(-12, 12, FLOOR_Y - 0.1, FLOOR_Y, -9, 6, skip=("-y",)), "floor")])
    static("Trench floor", floor, shadows=False)
    sources = []
    for name, (x0, x1, z0, z1, top) in PLATFORMS.items():
        asset = importer(f"platform_{name.lower()}", [
            (box(x0, x1, FLOOR_Y, top, z0, z1, skip=("+y", "-y")), "side"),
            (tiles(x0, x1, z0, z1, top, 0), "top_a"), (tiles(x0, x1, z0, z1, top, 1), "top_b")])
        sources.append({"entity": static(f"Platform {name}", asset), "geometry": "mesh"})

    # Render-only link pads at the AUTHORED endpoints, 4-8 mm proud of the tops.
    pad = {k: importer(f"pad_{k}", [(ring(0.13, 0.19, 0.004), k)]) for k in ("up", "bridge", "ret", "ret_off")}
    arrow = {k: importer(f"arrow_{k}", [(chevron(0.30, 0.30, 0.06, 0.006), k)]) for k in ("up", "bridge", "ret", "ret_off")}
    for link_id, key, start, end, bidirectional, enabled in LINKS:
        if key == "ret":
            for state, k in (("on", "ret"), ("off", "ret_off")):
                visible = (state == "on") == enabled
                for end_name, at in (("launch", start), ("landing", end)):
                    static(f"Return pad {end_name} {state}", pad[k], at if visible else (0, -40, 0), shadows=False)
                static(f"Return arrow {state}", arrow[k], behind(start, end) if visible else (0, -40, 0),
                       yaw_toward(start, end), shadows=False)
            continue
        static(f"Link pad {key} launch", pad[key], start, shadows=False)
        static(f"Link pad {key} landing", pad[key], end, shadows=False)
        direction = yaw_toward(start, end)
        static(f"Link arrow {key} launch", arrow[key], behind(start, end), direction, shadows=False)
        if bidirectional:
            static(f"Link arrow {key} reverse", arrow[key], behind(end, start), yaw_toward(end, start), shadows=False)

    goal = importer("goal", [(ring(0.24, 0.30, 0.004), "goal")])
    static("Start mark", goal, START, shadows=False)
    static("Goal mark", goal, GOAL, shadows=False)

    walk_dot = importer("walk_dot", [(disc(0.045, 0.0), "walk_dot")])
    jump_dot = {k: importer(f"jump_dot_{k}", [(octahedron(0.045), k)]) for k in ("up", "bridge", "ret")}
    for key, count in POOLS.items():
        for i in range(count):
            label = f"Trail walk {i:03d}" if key == "walk" else f"Trail jump {key} {i:03d}"
            static(label, walk_dot if key == "walk" else jump_dot[key], (0, -40, 0), shadows=False)

    walker = importer("walker", [(capsule(RADIUS, BODY_HEIGHT), "walker"),
                                 (box(-0.09, 0.09, 0.86, 1.0, -RADIUS - 0.02, -RADIUS + 0.06), "visor")])
    entity("Walker", {"Transform": transform(START, quat_axis((0, 1, 0), -90)),
                      "MeshRenderer": {"mesh": walker, "materials": [], "cast_shadows": True}})
    links = [{"id": i, "start": list(s), "end": list(e), "snap_distance": SNAP, "bidirectional": b,
              "enabled": en, "extra_cost": 0} for i, _, s, e, b, en in LINKS]
    entity("Navigation", {"NavigationMesh": {"settings": NAVIGATION, "sources": sources, "links": links}})

    cameras = {}
    for name, (eye, target, fov) in CAMERAS.items():
        cameras[name] = entity(f"Camera · {name}", {
            "Camera": {"fov_degrees": fov, "near": 0.1, "far": 120},
            "Transform": transform(eye, look(eye, target))})
    sun = quat_mul(quat_axis((0, 1, 0), 200), quat_axis((1, 0, 0), -58))
    entity("Sun", {"DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.6, "shadows": {"distance": 40}},
                   "Transform": transform((0, 0, 0), sun)})

    commands = [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities]
    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Off-mesh link look-dev course"}},
        {"id": 3, "method": "project.save"},
    ]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    run("node", ROOT / "node_modules/typescript/bin/tsc", "-p", TSCONFIG)
    script = out / "off_mesh_course.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    geometry = {
        "scene": scene_id, "platforms": PLATFORMS, "floor_y": FLOOR_Y, "radius": RADIUS, "height": BODY_HEIGHT,
        "links": links, "navigation": NAVIGATION, "start": START, "goal": GOAL,
        "cameras": {k: {"id": cameras[k], "eye": v[0], "target": v[1], "fov": v[2]} for k, v in CAMERAS.items()},
        "entities": len(entities), "walker": ids["Walker"], "nav": ids["Navigation"],
        "physics_components": 0,
    }
    (out / "geometry.json").write_text(json.dumps(geometry, indent=1), encoding="utf-8")
    return project, script, cameras


def check_budget(name, ticks, every):
    frames = 1 + math.ceil(ticks / every)
    if frames > MAX_FRAMES or frames * WIDTH * HEIGHT * 4 > MAX_RAW:
        raise SystemExit(f"run {name}: {frames} frames exceeds the capture budget")


def play(project, script, out, name, ticks, camera=None, every=1, load=None, save=None):
    logs = out / f"{name}.logs.jsonl"
    args = [CLI, "play", project, "--ticks", ticks, "--compiled-script", script, "--log-output", logs]
    if camera is not None:
        check_budget(name, ticks, every)
        args += ["--output", out / name, "--camera", camera, "--capture-every", every,
                 "--width", WIDTH, "--height", HEIGHT]
    if load:
        args += ["--load-save", load]
    if save:
        args += ["--save-output", save]
    # A host that suspends the process mid-tick fails the 50 ms wall-clock script
    # budget. Failed attempts are kept as NAME.failed-K and recorded.
    failures = []
    for attempt in range(1, ATTEMPTS + 1):
        before = verify_binary()
        result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
        after = verify_binary()
        if result.returncode == 0:
            break
        last = logs.read_text(encoding="utf-8").splitlines()[-1:] if logs.exists() else []
        failures.append({"attempt": attempt, "stderr": result.stderr.strip()[-400:],
                         "last_logged_tick": json.loads(last[0])["tick"] if last else None})
        for path in (out / name, logs):
            if path.exists():
                path.rename(path.with_name(f"{path.name}.failed-{attempt}"))
        if save and Path(save).exists():
            Path(save).rename(Path(save).with_name(f"{Path(save).name}.failed-{attempt}"))
    else:
        raise SystemExit(f"play {name} failed {ATTEMPTS} times: {failures}")
    (out / f"{name}.stdout.json").write_text(result.stdout, encoding="utf-8")
    return {"name": name, "ticks": ticks, "camera": camera, "every": every, "load": str(load) if load else None,
            "save": str(save) if save else None, "binary_before": before, "binary_after": after,
            "failed_attempts": failures}


def events(logs):
    found = []
    for line in Path(logs).read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        if entry["level"] == "info":
            found.append(json.loads(entry["message"]))
    return found


def resolve(event_name, evs):
    """`launch-<key>-<n>`, `land-<key>-<n>` or `enable`/`disable`/`arrive-<n>`; returns its tick."""
    if event_name == "mid-ret":
        launch = resolve("launch-ret-1", evs)
        land = resolve("land-ret-1", evs)
        return (launch + land) // 2
    parts = event_name.split("-")
    kind, n = parts[0], int(parts[-1]) if parts[-1].isdigit() else 1
    key = parts[1] if len(parts) == 3 else None
    matches = [e["t"] for e in evs if e["ev"] == kind and (key is None or e.get("key") == key)]
    if len(matches) < n:
        raise SystemExit(f"event {event_name} not found")
    return matches[n - 1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--ticks", type=int, default=1000)
    parser.add_argument("--run", action="append", default=[], help="NAME:CAMERA:EVERY")
    parser.add_argument("--window", action="append", default=[], help="NAME:CAMERA:EVENT:BEFORE:TICKS:EVERY")
    args = parser.parse_args()
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    runs = [r.split(":") for r in args.run]
    windows = [w.split(":") for w in args.window]
    for name, camera, every in runs:
        if camera not in CAMERAS or not every.isdigit() or int(every) < 1:
            raise SystemExit(f"invalid --run {name}:{camera}:{every}")
        check_budget(name, args.ticks, int(every))
    for name, camera, _event, before, ticks, every in windows:
        if camera not in CAMERAS or not all(v.isdigit() for v in (before, ticks, every)) or int(every) < 1:
            raise SystemExit(f"invalid --window {name}")
        check_budget(name, int(ticks), int(every))
    verify_binary()
    out.mkdir(parents=True)
    project, script, cameras = build(out)
    authored = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (project, project.with_suffix(".journal.jsonl"))}
    records = [play(project, script, out, "reference", args.ticks)]
    evs = events(out / "reference.logs.jsonl")
    enable, disable = resolve("enable", evs), resolve("disable", evs)
    report_ticks = sorted({0, 1, 2, enable - 1, enable, enable + 1, enable + 2, disable - 1, disable, disable + 1, disable + 2})
    for t in report_ticks:
        records.append(play(project, script, out, f"report-t{t:04d}", t))
    mid = resolve("mid-ret", evs)
    save = out / f"resume-t{mid:04d}.save.json"
    records.append(play(project, script, out, "resume.prefix", mid, save=save))
    records.append(play(project, script, out, "resume", args.ticks - mid, load=save))
    for name, camera, every in runs:
        records.append(play(project, script, out, name, args.ticks, cameras[camera], int(every)))
    for name, camera, event, before, ticks, every in windows:
        start = resolve(event, evs) - int(before)
        wsave = out / f"{name}.start-t{start:04d}.save.json"
        records.append(play(project, script, out, f"{name}.prefix", start, save=wsave))
        records.append(play(project, script, out, name, int(ticks), cameras[camera], int(every), load=wsave))
    after = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (project, project.with_suffix(".journal.jsonl"))}
    summary = {"project": str(project), "ticks": args.ticks, "runs": runs, "windows": windows,
               "enable_tick": enable, "disable_tick": disable, "mid_ret_save_tick": mid,
               "report_ticks": report_ticks, "authored_before": authored, "authored_after": after,
               "authored_unchanged": authored == after, "plays": records, "binary_sha256": verify_binary()}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({k: summary[k] for k in ("ticks", "enable_tick", "disable_tick", "mid_ret_save_tick",
                                              "authored_unchanged", "binary_sha256")}))


if __name__ == "__main__":
    main()
