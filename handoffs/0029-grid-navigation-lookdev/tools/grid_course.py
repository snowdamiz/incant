#!/usr/bin/env python3
"""Real-engine weighted grid navigation look-dev for handoff 0029.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then records actual motion with the CLI `play` command.

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json
     before and after every engine invocation.
  2. `incant_headless init` creates an empty project.
  3. This script writes original mathematical glTF geometry (cell tiles, wall
     blocks, door slabs, actor puck, route/trail markers).
  4. `incant_headless import` registers every model through the command bus.
  5. `incant_headless rpc --journal ...` creates every entity, including the
     NavigationGrid, in one `command.execute` transaction, then `project.save`.
     No project JSON is edited directly. No RigidBody or Collider components.
  6. Strict `tsc -p tools/tsconfig.json`, then `node tools/build_script.mjs`.
  7. `incant_headless validate`, then `play`:
       reference   log-only full course (0..--ticks)
       repeat      a second log-only full course, compared with reference
       resume      a `--save-output` run that stops mid-way through the
                   unreachable wait, then `--load-save` to finish
       --run NAME:CAMERA:EVERY        full-course capture
       --window NAME:CAMERA:EVENT:BEFORE:TICKS:EVERY
                   save at (EVENT tick - BEFORE), reopen and capture TICKS ticks.
                   EVENT is an event kind from the reference log, optionally
                   suffixed -N for its Nth occurrence (default 1).
     Frames are the engine's own GPU output; logs are committed script logs.

Usage (repo root):

  python3 -I handoffs/0029-grid-navigation-lookdev/tools/grid_course.py OUT_DIR \
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

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CLI = ROOT / "artifacts/tools/incant_headless"
BINARY_JSON = ROOT / "artifacts/tools/binary.json"
BEHAVIOR = HERE / "grid_course.ts"
TSCONFIG = HERE / "tsconfig.json"
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024
ATTEMPTS = 4  # per play run; the script host has a 50 ms executing-thread CPU budget

# ---- layout (must match grid_course.ts; trace_check.py verifies the logged copy) ----
# Cell (c, r): c is the column (world +X, east), r is the row (world +Z, south).
# Row-major index r * W + c. Cell centre in world metres: (c - 7.5, 0, r - 4.5).
W, H = 16, 10
FLOOR, MUD = 1, 5
START, GOAL = (2, 4), (13, 5)
DOOR_A, GATE_B = (8, 2), (8, 7)
MUD_CELLS = [(7, 6), (7, 7), (7, 8), (8, 7), (9, 6), (9, 7), (9, 8),  # gate B approach
             (10, 3), (11, 3), (10, 4), (11, 4), (11, 2), (12, 3)]  # marsh east of door A
GAUGE = (2, 8)  # plus-shaped corner gauge: four blocked arms around an open pocket
GAUGE_ARMS = [(2, 7), (1, 8), (3, 8), (2, 9)]


def authored_costs():
    costs = [FLOOR] * (W * H)
    for r in range(H):
        costs[r * W + 8] = 0  # dividing wall
    costs[DOOR_A[1] * W + DOOR_A[0]] = FLOOR
    for c, r in MUD_CELLS:
        costs[r * W + c] = MUD
    for c, r in GAUGE_ARMS:
        costs[r * W + c] = 0
    return costs


def centre(cell, y=0.0):
    return (cell[0] - 7.5, y, cell[1] - 4.5)


ACTOR_RADIUS = 0.30  # < half a cell: a centre-to-centre move never reaches a blocked cell
ACTOR_HEIGHT = 0.32
WALL_HEIGHT = 0.55
TILE_INSET = 0.035  # visible gap between cell tiles
POOLS = {"route_dot": 40, "route_bar": 40, "trail": 56, "gauge_dot": 8, "gauge_bar": 8}

# Neutral structure; one hue per meaning.
PALETTE = {
    "base": ((0.10, 0.105, 0.11), 0.95),       # gaps between tiles, plinth
    "floor": ((0.70, 0.69, 0.66), 0.9),       # weight 1
    "mud": ((0.44, 0.30, 0.15), 0.95),        # weight 5
    "wall_top": ((0.085, 0.09, 0.10), 0.8),    # blocked (weight 0)
    "wall_side": ((0.14, 0.15, 0.16), 0.85),
    "door": ((0.86, 0.26, 0.20), 0.6),        # door/gate closed by a grid command
    "actor": ((0.95, 0.95, 0.93), 0.45),
    "nose": ((0.07, 0.07, 0.08), 0.5),
    "route": ((0.08, 0.66, 0.70), 0.5),       # current returned route
    "trail": ((0.22, 0.22, 0.24), 0.7),       # cells actually visited
    "goal": ((0.22, 0.78, 0.36), 0.5),
    "start": ((0.95, 0.95, 0.93), 0.5),
    "wait": ((0.98, 0.70, 0.10), 0.45),       # stopped: destination unreachable
    "gauge": ((0.58, 0.40, 0.90), 0.5),       # independent blocked-corner queries
}

CAMERAS = {
    # name: (eye, target, vertical fov degrees)
    # Near-orthographic plan from 40 m; the whole 16 x 10 m grid plus a margin.
    "plan": ((0.0, 40.0, 0.0), (0.0, 0.0, 0.0), 16.5),
    "oblique": ((4.5, 10.5, 12.5), (0.0, -0.2, 0.4), 40),
    "gate": ((-2.2, 5.6, 7.4), (-0.3, 0.0, 1.2), 40),
    "door": ((-2.0, 8.2, 6.6), (-1.6, 0.0, -0.6), 44),
    "gauge": ((-5.6, 8.5, 7.6), (-5.0, 0.0, 3.6), 30),
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


# ---- geometry (metres, baked; +Y up) ---------------------------------------------------

def merge(*meshes):
    positions, normals, indices = [], [], []
    for p, n, i in meshes:
        base = len(positions)
        positions += p
        normals += n
        indices += [base + k for k in i]
    return positions, normals, indices


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


def quad_y(x0, x1, z0, z1, y):
    positions = [(x0, y, z0), (x0, y, z1), (x1, y, z1), (x1, y, z0)]
    return positions, [(0, 1, 0)] * 4, [0, 1, 2, 0, 2, 3]


def disc(radius, y, segments=28):
    positions, normals, indices = [(0, y, 0)], [(0, 1, 0)], []
    for s in range(segments):
        t = 2 * math.pi * s / segments
        positions.append((radius * math.cos(t), y, radius * math.sin(t)))
        normals.append((0, 1, 0))
    for s in range(segments):
        indices += [0, 1 + (s + 1) % segments, 1 + s]
    return positions, normals, indices


def ring(inner, outer, y, segments=40):
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


def cylinder(radius, y0, y1, segments=40):
    positions, normals, indices = [], [], []
    for s in range(segments + 1):
        t = 2 * math.pi * s / segments
        n = (math.cos(t), 0, math.sin(t))
        positions += [(radius * n[0], y0, radius * n[2]), (radius * n[0], y1, radius * n[2])]
        normals += [n, n]
    for s in range(segments):
        a = 2 * s
        indices += [a, a + 1, a + 2, a + 2, a + 1, a + 3]
    top = disc(radius, y1, segments)
    return merge((positions, normals, indices), top)


def triangle_y(points, y):
    positions = [(x, y, z) for x, z in points]
    return positions, [(0, 1, 0)] * 3, [0, 2, 1]


def cross(size, width, y):
    """Flat X mark, centred on the origin."""
    half, w = size / 2, width / 2
    arms = []
    for angle in (45, -45):
        a = math.radians(angle)
        ca, sa = math.cos(a), math.sin(a)
        corners = [(-half, -w), (half, -w), (half, w), (-half, w)]
        positions = [(x * ca - z * sa, y, x * sa + z * ca) for x, z in corners]
        arms.append((positions, [(0, 1, 0)] * 4, [0, 2, 1, 0, 3, 2]))
    return merge(*arms)


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
        "asset": {"version": "2.0", "generator": "handoff 0029 grid_course.py"},
        "buffers": [{"uri": f"{name}.bin", "byteLength": len(blob)}],
        "bufferViews": views, "accessors": accessors,
        "meshes": [{"name": name, "primitives": prims}], "materials": materials,
        "nodes": [{"name": name, "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0,
    }
    path = models / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1), encoding="utf-8")
    return path


# ---- scene ------------------------------------------------------------------------------

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
    yaw = math.degrees(math.atan2(-d[0], -d[2])) if math.hypot(d[0], d[2]) > 1e-9 else 0.0
    pitch = math.degrees(math.atan2(d[1], math.hypot(d[0], d[2])))
    return quat_mul(quat_axis((0, 1, 0), yaw), quat_axis((1, 0, 0), pitch))


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
    return "01JA2G0029" + suffix


def rpc(project, journal, requests):
    result = subprocess.run([str(CLI), "rpc", str(project), "--journal", str(journal)],
                            input="".join(json.dumps(r) + "\n" for r in requests),
                            capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"rpc failed: {result.stderr}")
    return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]


def cell_tiles(cells, y):
    meshes = []
    for c, r in cells:
        x, _, z = centre((c, r))
        meshes.append(quad_y(x - 0.5 + TILE_INSET, x + 0.5 - TILE_INSET, z - 0.5 + TILE_INSET, z + 0.5 - TILE_INSET, y))
    return merge(*meshes)


def build(out):
    project = out / "grid.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Weighted Grid Look-dev", "--entities", "0")
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

    costs = authored_costs()
    all_cells = [(c, r) for r in range(H) for c in range(W)]
    floor_cells = [cell for cell in all_cells if costs[cell[1] * W + cell[0]] == FLOOR]
    mud_cells = [cell for cell in all_cells if costs[cell[1] * W + cell[0]] == MUD]
    wall_cells = [cell for cell in all_cells if costs[cell[1] * W + cell[0]] == 0]

    # Plinth under the grid (its top shows through the tile gaps), then weighted tiles.
    plinth = importer("plinth", [(box(-8.25, 8.25, -0.35, 0.0, -5.25, 5.25, skip=("-y",)), "base")])
    static("Plinth", plinth, shadows=False)
    tiles = importer("tiles", [(cell_tiles(floor_cells, 0.002), "floor"), (cell_tiles(mud_cells, 0.002), "mud")])
    static("Cell tiles", tiles, shadows=False)
    blocks = [box(c - 8.0, c - 7.0, 0.0, WALL_HEIGHT, r - 5.0, r - 4.0, skip=("-y",)) for c, r in wall_cells]
    walls = importer("walls", [(merge(*[(p, n, i) for p, n, i in blocks]), "wall_side")])
    static("Blocked cells", walls)
    # Darker tops would need a second material per face; a flat lid reads blocked from above.
    lids = importer("wall_lids", [(cell_tiles([(c, r) for c, r in wall_cells], WALL_HEIGHT + 0.001), "wall_top")])
    static("Blocked cell lids", lids, shadows=False)

    # Door slab: the whole cell, slightly lower than the wall so it reads as a door.
    slab = importer("door_slab", [(box(-0.5 + 0.02, 0.5 - 0.02, 0.0, WALL_HEIGHT * 0.82, -0.47, 0.47, skip=("-y",)), "door")])
    for name in ("Door A", "Gate B"):
        static(f"{name} slab", slab, (0, -40, 0))  # open at start; the behavior moves it

    start = importer("start", [(ring(0.30, 0.36, 0.006), "start")])
    static("Start mark", start, centre(START), shadows=False)
    goal = importer("goal", [(ring(0.33, 0.43, 0.006), "goal"), (disc(0.09, 0.006), "goal")])
    static("Goal mark", goal, centre(GOAL), shadows=False)

    route_dot = importer("route_dot", [(disc(0.085, 0.012), "route")])
    route_bar = importer("route_bar", [(quad_y(0.0, 1.0, -0.028, 0.028, 0.010), "route")])
    trail = importer("trail_dot", [(disc(0.065, 0.008), "trail")])
    gauge_dot = importer("gauge_dot", [(disc(0.085, 0.012), "gauge")])
    gauge_bar = importer("gauge_bar", [(quad_y(0.0, 1.0, -0.028, 0.028, 0.010), "gauge")])
    gauge_ring = importer("gauge_ring", [(ring(0.22, 0.28, 0.008), "gauge")])
    gauge_cross = importer("gauge_cross", [(cross(0.34, 0.07, 0.0), "gauge")])
    pool_assets = {"route_dot": route_dot, "route_bar": route_bar, "trail": trail,
                   "gauge_dot": gauge_dot, "gauge_bar": gauge_bar}
    for key, count in POOLS.items():
        for i in range(count):
            static(f"Pool {key} {i:03d}", pool_assets[key], (0, -40, 0), shadows=False)
    for name in ("Gauge null start", "Gauge null end", "Gauge detour start", "Gauge detour end"):
        static(name, gauge_ring, (0, -40, 0), shadows=False)
    static("Gauge null cross", gauge_cross, (0, -40, 0), shadows=False)

    wait = importer("wait_ring", [(ring(0.38, 0.46, 0.014), "wait")])
    static("Wait ring", wait, (0, -40, 0), shadows=False)
    # Actor puck: local +X is forward; the nose wedge on top shows facing from above.
    nose = triangle_y([(0.24, 0.0), (-0.06, 0.13), (-0.06, -0.13)], ACTOR_HEIGHT + 0.004)
    actor = importer("actor", [(cylinder(ACTOR_RADIUS, 0.0, ACTOR_HEIGHT), "actor"), (nose, "nose")])
    entity("Actor", {"Transform": transform(centre(START)),
                     "MeshRenderer": {"mesh": actor, "materials": [], "cast_shadows": True}})
    entity("Grid", {"NavigationGrid": {"dimensions": [W, H], "costs": costs}})

    cameras = {}
    for name, (eye, target, fov) in CAMERAS.items():
        cameras[name] = entity(f"Camera · {name}", {
            "Camera": {"fov_degrees": fov, "near": 0.1, "far": 120},
            "Transform": transform(eye, look(eye, target))})
    sun = quat_mul(quat_axis((0, 1, 0), 215), quat_axis((1, 0, 0), -62))
    entity("Sun", {"DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.6, "shadows": {"distance": 40}},
                   "Transform": transform((0, 0, 0), sun)})

    commands = [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities]
    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Weighted grid look-dev course"}},
        {"id": 3, "method": "project.save"},
    ]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    run("node", ROOT / "node_modules/typescript/bin/tsc", "-p", TSCONFIG)
    script = out / "grid_course.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    saved = json.loads(project.read_text(encoding="utf-8"))["scenes"][scene_id]["entities"]
    geometry = {
        "scene": scene_id, "dimensions": [W, H], "costs": costs, "start": START, "goal": GOAL,
        "door_a": DOOR_A, "gate_b": GATE_B, "gauge": GAUGE, "gauge_arms": GAUGE_ARMS,
        "actor_radius": ACTOR_RADIUS, "actor_height": ACTOR_HEIGHT, "wall_height": WALL_HEIGHT,
        "cameras": {k: {"id": cameras[k], "eye": v[0], "target": v[1], "fov": v[2]} for k, v in CAMERAS.items()},
        "entities": len(entities), "actor": ids["Actor"], "grid": ids["Grid"],
        "provenance_present": all(saved[e["id"]].get("provenance") is not None for e in entities),
        "physics_components": sum(1 for e in entities for c in e["components"] if c in ("RigidBody", "Collider")),
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
    # A host that suspends the process mid-tick can exceed the script CPU budget.
    # Failed attempts are kept as NAME.failed-K and recorded, never hidden.
    failures = []
    for attempt in range(1, ATTEMPTS + 1):
        before = verify_binary()
        result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
        after = verify_binary()
        if result.returncode == 0:
            break
        last = logs.read_text(encoding="utf-8").splitlines()[-1:] if logs.exists() else []
        failures.append({"attempt": attempt, "stderr": result.stderr.strip()[-600:],
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
    kind, _, n = event_name.rpartition("-")
    if not n.isdigit():
        kind, n = event_name, "1"
    matches = [e["t"] for e in evs if e["ev"] == kind]
    if len(matches) < int(n):
        raise SystemExit(f"event {event_name} not found")
    return matches[int(n) - 1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--ticks", type=int, default=900)
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
    authored_paths = (project, project.with_suffix(".journal.jsonl"))
    authored = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in authored_paths}
    records = [play(project, script, out, "reference", args.ticks),
               play(project, script, out, "repeat", args.ticks)]
    evs = events(out / "reference.logs.jsonl")
    blocked, reopen = resolve("unreachable", evs), resolve("reopen", evs)
    mid = (blocked + reopen) // 2
    save = out / f"resume-t{mid:04d}.save.json"
    records.append(play(project, script, out, "resume.prefix", mid, save=save))
    records.append(play(project, script, out, "resume", args.ticks - mid, load=save))
    for name, camera, every in runs:
        records.append(play(project, script, out, name, args.ticks, cameras[camera], int(every)))
    for name, camera, event, before, ticks, every in windows:
        start = max(0, resolve(event, evs) - int(before))
        wsave = out / f"{name}.start-t{start:04d}.save.json"
        if start > 0:
            records.append(play(project, script, out, f"{name}.prefix", start, save=wsave))
        records.append(play(project, script, out, name, int(ticks), cameras[camera], int(every),
                            load=wsave if start > 0 else None))
        records[-1]["start_tick"] = start
    after = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in authored_paths}
    summary = {"project": str(project), "ticks": args.ticks, "runs": runs, "windows": windows,
               "first_unreachable_tick": blocked, "reopen_tick": reopen, "resume_save_tick": mid,
               "authored_before": authored, "authored_after": after, "authored_unchanged": authored == after,
               "plays": records, "binary_sha256": verify_binary()}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({k: summary[k] for k in ("ticks", "first_unreachable_tick", "reopen_tick",
                                              "resume_save_tick", "authored_unchanged", "binary_sha256")}))


if __name__ == "__main__":
    main()
