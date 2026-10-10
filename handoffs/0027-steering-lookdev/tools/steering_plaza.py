#!/usr/bin/env python3
"""Real-engine local-avoidance look-dev for handoff 0027.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then records actual motion with the CLI `play` command:

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json.
  2. `incant_headless init` creates an empty project.
  3. This script writes original mathematical glTF geometry. Walker meshes are
     capsules of exactly the steering radius (0.25 m) and height (1.8 m), with
     their feet at the entity origin; every root has unit scale.
  4. `incant_headless import` registers each model through the command bus.
  5. `incant_headless rpc --journal ...` creates every entity in one
     `command.execute` transaction, then `project.save`.
  6. Strict `tsc --noEmit`, then `node tools/build_script.mjs` compiles the behavior.
  7. `incant_headless validate`, then `play` runs:
       full    NAME:CAMERA:EVERY            ticks 0..--ticks from the authored start
       window  NAME:CAMERA:START:TICKS:EVERY  a `--save-output` run of START ticks
               (no capture), then `--load-save` for TICKS ticks with the camera.
       CAMERA may be `none` for a log-only run.
     Frames are the engine's own GPU output; logs are committed script logs.
  8. Re-verifies the binary hash before and after every play run.

Usage (repo root):

  caffeinate -s -i python3 -I handoffs/0027-steering-lookdev/tools/steering_plaza.py OUT_DIR \
      [--ticks N] [--run NAME:CAMERA:EVERY ...] [--window NAME:CAMERA:START:TICKS:EVERY ...]

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
BEHAVIOR = Path(__file__).resolve().with_name("steering_plaza.ts")
TSCONFIG = Path(__file__).resolve().with_name("tsconfig.json")
ALL = 0xFFFFFFFF
WIDTH, HEIGHT = 960, 540
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024
ATTEMPTS = 4  # per play run; see play() on wall-clock deadlines and host sleep

# Plaza: x east, z south (toward the overview camera), +Y up. Must match the behavior.
FLOOR_HALF = 14.0
RADIUS, BODY_HEIGHT = 0.25, 1.8
SPACING = 0.9  # 5x5 block pitch
GROUPS = {  # start block centre, travel to the goal block
    "W": ((-10.0, 0.0), (20.0, 0.0)),
    "E": ((10.0, 0.0), (-20.0, 0.0)),
    "N": ((0.0, -10.0), (0.0, 20.0)),
    "S": ((0.0, 10.0), (0.0, -20.0)),
}
TRACKED = ("2-2", "0-4")  # row-col cells that carry a dark cap and drop trail markers
TRAIL_POOL = 136  # 136 drops x 20 ticks covers a 2720-tick run
PLINTH_R, PLINTH_H = 1.0, 0.9
GANTRY_X, POST_Z, POST_HALF = -5.5, 4.5, 0.15
BEAM = (2.1, 2.35, 0.15, 4.65)  # min_y, max_y, half_x, half_z

# Neutral greys for structure; one muted hue per walker group, separated by
# luminance as well as hue (amber light, bone lightest, blue mid, brick dark).
PALETTE = {
    "floor_a": ((0.25, 0.25, 0.24), 0.92),
    "floor_b": ((0.29, 0.29, 0.28), 0.92),
    "zone": ((0.19, 0.19, 0.19), 0.9),
    "plinth": ((0.50, 0.49, 0.46), 0.7),
    "plinth_top": ((0.60, 0.59, 0.56), 0.7),
    "gantry": ((0.40, 0.39, 0.37), 0.6),
    "cap": ((0.07, 0.07, 0.08), 0.5),
    "W": ((0.93, 0.60, 0.15), 0.5),
    "E": ((0.20, 0.38, 0.68), 0.5),
    "N": ((0.86, 0.84, 0.78), 0.5),
    "S": ((0.58, 0.17, 0.12), 0.5),
}

CAMERAS = {
    # name: (eye, target, vertical fov degrees)
    "overview": ((0.0, 24.0, 17.0), (0.0, 0.0, 1.0), 50),
    "plan": ((0.0, 31.0, 0.001), (0.0, 0.0, 0.0), 44),
    # Near-orthographic telephotos from 40 m: a 1.8 m body top is magnified by only
    # 40 / 38.2 = 1.047, so gaps between footprints are not visibly understated.
    "plinth": ((0.0, 40.0, 0.001), (0.0, 0.0, 0.0), 10),
    "endpoint": ((0.9, 40.0, -9.0 + 0.001), (0.9, 0.0, -9.0), 10),
    "crossing": ((-6.0, 8.5, 7.5), (-0.3, 0.0, 0.3), 42),
    "gantry": ((-9.8, 1.9, 6.4), (-5.3, 1.1, 0.3), 50),
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

def box(hx, hy, hz, offset=(0, 0, 0), skip_top=False):
    positions, normals, indices = [], [], []
    faces = [((1, 0, 0), (0, 1, 0)), ((-1, 0, 0), (0, 1, 0)), ((0, 1, 0), (0, 0, 1)),
             ((0, -1, 0), (0, 0, 1)), ((0, 0, 1), (0, 1, 0)), ((0, 0, -1), (0, 1, 0))]
    half = (hx, hy, hz)
    for n, u in faces:
        if skip_top and n == (0, 1, 0):
            continue  # hidden under coplanar-adjacent tiles; avoids z-fighting
        v = (n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0])
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple((n[i] + su * u[i] + sv * v[i]) * half[i] + offset[i] for i in range(3)))
            normals.append(n)
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def tiles(half, parity, y=0.0):
    """1 m floor tiles whose (i + j) % 2 == parity, top face at y."""
    positions, normals, indices = [], [], []
    n = int(2 * half)
    for i in range(n):
        for j in range(n):
            if (i + j) % 2 != parity:
                continue
            x0, z0 = -half + i, -half + j
            base = len(positions)
            for x, z in ((x0, z0), (x0, z0 + 1), (x0 + 1, z0 + 1), (x0 + 1, z0)):
                positions.append((x, y, z))
                normals.append((0, 1, 0))
            indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def disc(radius, y, segments=24):
    positions, normals, indices = [(0, y, 0)], [(0, 1, 0)], []
    for s in range(segments):
        t = 2 * math.pi * s / segments
        positions.append((radius * math.cos(t), y, radius * math.sin(t)))
        normals.append((0, 1, 0))
    for s in range(segments):
        indices += [0, 1 + (s + 1) % segments, 1 + s]
    return positions, normals, indices


def prism(points, y0, y1):
    """Vertical prism over a convex CCW-in-(x,z) polygon; sides plus bottom cap."""
    positions, normals, indices = [], [], []
    n = len(points)
    for i in range(n):
        (ax, az), (bx, bz) = points[i], points[(i + 1) % n]
        ex, ez = bx - ax, bz - az
        length = math.hypot(ex, ez)
        normal = (ez / length, 0, -ex / length)  # outward for CCW in (x, z); matches the winding
        base = len(positions)
        for x, y, z in ((ax, y0, az), (bx, y0, bz), (bx, y1, bz), (ax, y1, az)):
            positions.append((x, y, z))
            normals.append(normal)
        indices += [base, base + 2, base + 1, base, base + 3, base + 2]
    return positions, normals, indices


def cap(points, y, up=True):
    positions = [(x, y, z) for x, z in points]
    normals = [(0, 1 if up else -1, 0)] * len(points)
    indices = []
    for i in range(1, len(points) - 1):
        indices += [0, i + 1, i] if up else [0, i, i + 1]
    return positions, normals, indices


def capsule_part(radius, height, top, rings=10, segments=28):
    """Feet-origin capsule of total `height`; top=True returns the upper hemisphere only,
    top=False the cylinder plus lower hemisphere. The union is the full capsule."""
    half = height / 2 - radius
    centre = height / 2
    positions, normals, indices = [], [], []
    rows = []
    if top:
        for r in range(rings + 1):
            rows.append(((math.pi / 2) * (1 - r / rings), centre + half))
    else:
        rows.append((0.0, centre + half))
        for r in range(rings + 1):
            rows.append((-(math.pi / 2) * (r / rings), centre - half))
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


def octagon():
    return [(PLINTH_R * math.cos(math.pi / 8 + i * math.pi / 4), PLINTH_R * math.sin(math.pi / 8 + i * math.pi / 4))
            for i in range(8)]


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
        "asset": {"version": "2.0", "generator": "handoff 0027 steering_plaza.py"},
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


def transform(t, rotation=(0, 0, 0, 1)):
    return {"translation": [round(v, 6) for v in t], "rotation": [round(v, 9) for v in rotation],
            "scale": [1, 1, 1]}


def walkers():
    """(group, cell, start_xz, goal_xz, tracked) in a fixed order."""
    out = []
    for group, ((cx, cz), (tx, tz)) in GROUPS.items():
        for r in range(5):
            for c in range(5):
                # Rows run along the travel direction, columns across it.
                along, across = (r - 2) * SPACING, (c - 2) * SPACING
                if tx != 0:
                    x, z = cx + along, cz + across
                else:
                    x, z = cx + across, cz + along
                cell = f"{r}-{c}"
                out.append((group, cell, (x, z), (x + tx, z + tz), cell in TRACKED))
    return out


def build(out):
    project = out / "steering.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    run(CLI, "init", project, "--name", "Steering Look-dev Plaza", "--entities", "0")
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

    def static(name, asset, at=(0, 0, 0), shadows=True):
        return entity(name, {"Transform": transform(at),
                             "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": shadows}})

    # Floor slab (top at y = 0) with 1 m checker tiles.
    floor = importer("floor", [(box(FLOOR_HALF, 0.1, FLOOR_HALF, (0, -0.1, 0), skip_top=True), "floor_a"),
                               (tiles(FLOOR_HALF, 0), "floor_a"), (tiles(FLOOR_HALF, 1), "floor_b")])
    static("Floor", floor, shadows=False)
    # Start/goal zones: render-only 4.6 m pads, 5 mm proud of the floor. Decals stack in
    # 5 mm steps (pads 5 mm, goal and trail dots 10 mm) so 40 m telephotos do not z-fight.
    zone = importer("zone", [(box(2.3, 0.0025, 2.3, (0, 0.0025, 0)), "zone")])  # top 5 mm
    for group, ((cx, cz), _) in GROUPS.items():
        static(f"Zone {group}", zone, (cx, 0, cz), shadows=False)

    octo = octagon()
    plinth = importer("plinth", [(prism(octo, 0, PLINTH_H), "plinth"), (cap(octo, PLINTH_H), "plinth_top")])
    static("Plinth", plinth)
    post = importer("post", [(box(POST_HALF, BEAM[1] / 2, POST_HALF, (0, BEAM[1] / 2, 0)), "gantry")])
    static("Gantry post north", post, (GANTRY_X, 0, -POST_Z))
    static("Gantry post south", post, (GANTRY_X, 0, POST_Z))
    beam = importer("beam", [(box(BEAM[2], (BEAM[1] - BEAM[0]) / 2, BEAM[3], (0, (BEAM[0] + BEAM[1]) / 2, 0)), "gantry")])
    static("Gantry beam (overhead)", beam, (GANTRY_X, 0, 0))

    bodies, capped, goals, trails = {}, {}, {}, {}
    for group in GROUPS:
        bodies[group] = importer(f"walker_{group}", [(capsule_part(RADIUS, BODY_HEIGHT, False), group),
                                                      (capsule_part(RADIUS, BODY_HEIGHT, True), group)])
        capped[group] = importer(f"walker_{group}_tracked", [(capsule_part(RADIUS, BODY_HEIGHT, False), group),
                                                              (capsule_part(RADIUS, BODY_HEIGHT, True), "cap")])
        goals[group] = importer(f"goal_{group}", [(disc(0.08, 0.010), group)])
        trails[group] = importer(f"trail_{group}", [(disc(0.055, 0.0), group)])

    layout = walkers()
    for group, cell, (x, z), (gx, gz), tracked in layout:
        static(f"Goal {group} {cell}", goals[group], (gx, 0, gz), shadows=False)
        if tracked:
            for i in range(TRAIL_POOL):
                static(f"Trail {group} {cell} {i:03d}", trails[group], (0, -30, 0), shadows=False)
    for group, cell, (x, z), _, tracked in layout:
        name = f"Walker {group} {cell}" + (" tracked" if tracked else "")
        entity(name, {
            "Transform": transform((x, 0, z)),
            "MeshRenderer": {"mesh": (capped if tracked else bodies)[group], "materials": [], "cast_shadows": True},
            "Velocity": {"linear": [0, 0, 0]},
        })

    cameras = {}
    for name, (eye, target, fov) in CAMERAS.items():
        cameras[name] = entity(f"Camera · {name}", {
            "Camera": {"fov_degrees": fov, "near": 0.1, "far": 120},
            "Transform": transform(eye, look(eye, target))})
    sun = quat_mul(quat_axis((0, 1, 0), 215), quat_axis((1, 0, 0), -70))  # 70 deg sun: 0.65 m shadows from 1.8 m bodies
    entity("Sun", {"DirectionalLight": {"color": [1, 0.97, 0.92], "intensity": 2.4, "shadows": {"distance": 60}},
                   "Transform": transform((0, 0, 0), sun)})

    commands = [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities]
    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": revision, "description": "Steering look-dev plaza"}},
        {"id": 3, "method": "project.save"},
    ]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    run(CLI, "validate", project)
    run("node", ROOT / "node_modules/typescript/bin/tsc", "-p", TSCONFIG)
    script = out / "steering_plaza.js"
    run("node", ROOT / "tools/build_script.mjs", BEHAVIOR, script)
    geometry = {
        "scene": scene_id, "cameras": {k: {"id": cameras[k], "eye": v[0], "target": v[1], "fov": v[2]}
                                       for k, v in CAMERAS.items()},
        "radius": RADIUS, "height": BODY_HEIGHT, "spacing": SPACING,
        "walkers": [{"name": f"Walker {g} {c}" + (" tracked" if t else ""), "id": ids[f"Walker {g} {c}" + (" tracked" if t else "")],
                     "start": s, "goal": goal} for g, c, s, goal, t in layout],
        "plinth": {"vertices": octo, "height": PLINTH_H},
        "posts": [[GANTRY_X, -POST_Z], [GANTRY_X, POST_Z]], "post_half": POST_HALF, "beam": BEAM,
        "entities": len(entities),
    }
    (out / "geometry.json").write_text(json.dumps(geometry, indent=1), encoding="utf-8")
    return project, script, cameras


def check_budget(name, ticks, every):
    frames = 1 + math.ceil(ticks / every)
    if frames > MAX_FRAMES or frames * WIDTH * HEIGHT * 4 > MAX_RAW:
        raise SystemExit(f"run {name}: {frames} frames exceeds the capture budget")


def play(project, script, out, name, camera, every, ticks, load=None, save=None):
    args = [CLI, "play", project, "--ticks", ticks, "--compiled-script", script,
            "--log-output", out / f"{name}.logs.jsonl"]
    if camera is not None:
        check_budget(name, ticks, every)
        args += ["--output", out / name, "--camera", camera, "--capture-every", every,
                 "--width", WIDTH, "--height", HEIGHT]
    if load:
        args += ["--load-save", load]
    if save:
        args += ["--save-output", save]
    # The script host enforces a 50 ms WALL-CLOCK budget per tick. A host that
    # suspends the process mid-tick (e.g. a locked Mac entering maintenance sleep)
    # fails the play with "script failed". Failed attempts are kept beside the
    # output as NAME.failed-K and recorded; the retry starts again from tick 0 (or
    # from the same save) into a fresh path. Run under `caffeinate -s -i`.
    failures = []
    for attempt in range(1, ATTEMPTS + 1):
        before = verify_binary()
        result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
        after = verify_binary()
        if result.returncode == 0:
            break
        logs = out / f"{name}.logs.jsonl"
        last = logs.read_text(encoding="utf-8").splitlines()[-1:] if logs.exists() else []
        failures.append({"attempt": attempt, "stderr": result.stderr.strip()[-300:],
                         "last_logged_tick": json.loads(last[0])["tick"] if last else None})
        for path in (out / name, logs, out / f"{name}.stdout.json"):
            if path.exists():
                path.rename(path.with_name(f"{path.name}.failed-{attempt}"))
        if save and Path(save).exists():
            Path(save).rename(Path(save).with_name(f"{Path(save).name}.failed-{attempt}"))
    else:
        raise SystemExit(f"play {name} failed {ATTEMPTS} times: {failures}")
    (out / f"{name}.stdout.json").write_text(result.stdout, encoding="utf-8")
    return {"name": name, "ticks": ticks, "every": every, "load": str(load) if load else None,
            "binary_before": before, "binary_after": after, "failed_attempts": failures}


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
    return "01JA2STR00" + suffix


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--ticks", type=int, default=2700)
    parser.add_argument("--run", action="append", default=[], help="NAME:CAMERA:EVERY (CAMERA may be none)")
    parser.add_argument("--window", action="append", default=[], help="NAME:CAMERA:START:TICKS:EVERY")
    args = parser.parse_args()
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    runs = [r.split(":") for r in args.run]
    windows = [w.split(":") for w in args.window]
    for name, camera, every in runs:
        if camera != "none" and camera not in CAMERAS or not every.isdigit() or int(every) < 1:
            raise SystemExit(f"invalid --run {name}:{camera}:{every}")
        if camera != "none":
            check_budget(name, args.ticks, int(every))
    for name, camera, start, ticks, every in windows:
        if camera not in CAMERAS or not all(v.isdigit() and int(v) >= 1 for v in (start, ticks, every)):
            raise SystemExit(f"invalid --window {':'.join([name, camera, start, ticks, every])}")
        check_budget(name, int(ticks), int(every))
    verify_binary()
    out.mkdir(parents=True)
    project, script, cameras = build(out)
    records = []
    for name, camera, every in runs:
        records.append(play(project, script, out, name, None if camera == "none" else cameras[camera],
                            int(every), args.ticks))
    for name, camera, start, ticks, every in windows:
        save = out / f"{name}.start.save.json"
        records.append(play(project, script, out, f"{name}.prefix", None, 1, int(start), save=save))
        records.append(play(project, script, out, name, cameras[camera], int(every), int(ticks), load=save))
    summary = {"project": str(project), "ticks": args.ticks, "runs": runs, "windows": windows,
               "plays": records, "binary_sha256": verify_binary()}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({k: summary[k] for k in ("ticks", "runs", "windows", "binary_sha256")}))


if __name__ == "__main__":
    main()
