#!/usr/bin/env python3
"""Real-engine orthographic camera look-dev for handoff 0030.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then renders it with the supplied binary:

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json
     before and after every engine invocation.
  2. `init` creates an empty project; original mathematical glTF (checker floor,
     two depth rails, three identical 1 x 1.6 x 1 m pillars in three hues) is
     written here and registered with `import`.
  3. One `rpc command.execute` transaction creates every entity, then
     `project.save`. No project JSON is edited directly.
  4. `screenshot --camera` renders every authored camera at 960x540 (16:9) and
     800x600 (4:3), twice each, into separate new files (repeat hash check).
  5. A strict-TypeScript behavior switches one live camera through ordinary
     `set_component` commands (perspective -> orthographic -> moved along its
     forward axis -> smaller vertical size -> explicit perspective). `play`
     captures it twice; frames are compared by hash.

All cameras share one oblique view direction (yaw 35 deg, pitch -30 deg) toward
TARGET; they differ only in distance along that axis, projection and size.

Usage (repo root):  python3 -I handoffs/0030-orthographic-cameras/tools/ortho_lookdev.py OUT_DIR
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
SIZES = {"16x9": (960, 540), "4x3": (800, 600)}
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

TARGET = (0.0, 0.8, -3.0)
YAW, PITCH = 35.0, -30.0
NEAR, FAR, FOV = 0.1, 80.0, 40.0
PILLAR = (1.0, 1.6, 1.0)  # identical authored size for every pillar
PILLARS = {"near": ((0.0, 4.0), "red"), "middle": ((0.0, -3.0), "teal"), "far": ((0.0, -10.0), "amber")}
RAILS = (-1.5, 1.5)  # x of two parallel rails running along world Z
RAIL_Z = (-15.0, 8.0)
FLOOR = (-11.0, 11.0, -18.0, 10.0)

PALETTE = {  # linear base colour, roughness
    "tile_a": ((0.30, 0.30, 0.29), 0.9),
    "tile_b": ((0.22, 0.22, 0.215), 0.9),
    "rail": ((0.16, 0.30, 0.78), 0.5),
    "red": ((0.72, 0.13, 0.10), 0.35),
    "teal": ((0.06, 0.50, 0.48), 0.35),
    "amber": ((0.80, 0.48, 0.08), 0.35),
}

# name: (distance along the view axis, projection or None for legacy perspective)
CAMERAS = {
    "persp-d16": (16, None),
    "ortho-d16-v9": (16, {"kind": "orthographic", "vertical_size": 9}),
    "ortho-d24-v9": (24, {"kind": "orthographic", "vertical_size": 9}),
    "ortho-d34-v9": (34, {"kind": "orthographic", "vertical_size": 9}),
    "ortho-d16-v6": (16, {"kind": "orthographic", "vertical_size": 6}),
    "ortho-d16-v13": (16, {"kind": "orthographic", "vertical_size": 13}),
    "ortho-d5-v9": (5, {"kind": "orthographic", "vertical_size": 9}),  # near-plane clipping review
}
# Live camera script: tick -> (distance, projection). Commands apply on the next tick.
LIVE = [(0, 16, None), (10, 16, {"kind": "orthographic", "vertical_size": 9}),
        (20, 24, {"kind": "orthographic", "vertical_size": 9}),
        (30, 24, {"kind": "orthographic", "vertical_size": 6}), (40, 24, {"kind": "perspective"})]
LIVE_TICKS, LIVE_EVERY = 50, 5


def run(*args):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:3]]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(CLI.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


def engine(*args):
    before = verify_binary()
    out = run(CLI, *args)
    after = verify_binary()
    return out, before == after


# ---- geometry ----------------------------------------------------------------------

def box(x0, x1, y0, y1, z0, z1):
    positions, normals, indices = [], [], []
    c = ((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2)
    half = ((x1 - x0) / 2, (y1 - y0) / 2, (z1 - z0) / 2)
    for n, u in (((1, 0, 0), (0, 1, 0)), ((-1, 0, 0), (0, 1, 0)), ((0, 1, 0), (0, 0, 1)),
                 ((0, -1, 0), (0, 0, 1)), ((0, 0, 1), (0, 1, 0)), ((0, 0, -1), (0, 1, 0))):
        v = (n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0])
        base = len(positions)
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            positions.append(tuple(c[i] + (n[i] + su * u[i] + sv * v[i]) * half[i] for i in range(3)))
            normals.append(n)
        indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def tiles(x0, x1, z0, z1, parity, size=1.0):
    positions, normals, indices = [], [], []
    for i in range(int(math.floor(x0 / size)), int(math.ceil(x1 / size))):
        for j in range(int(math.floor(z0 / size)), int(math.ceil(z1 / size))):
            if (i + j) % 2 != parity:
                continue
            base = len(positions)
            for x, z in ((i, j), (i, j + 1), (i + 1, j + 1), (i + 1, j)):
                positions.append((x * size, 0.0, z * size))
                normals.append((0, 1, 0))
            indices += [base, base + 1, base + 2, base, base + 2, base + 3]
    return positions, normals, indices


def write_gltf(models, name, primitives):
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
    document = {"asset": {"version": "2.0", "generator": "handoff 0030 ortho_lookdev.py"},
                "buffers": [{"uri": f"{name}.bin", "byteLength": len(blob)}],
                "bufferViews": views, "accessors": accessors,
                "meshes": [{"name": name, "primitives": prims}], "materials": materials,
                "nodes": [{"name": name, "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0}
    path = models / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1), encoding="utf-8")
    return path


# ---- camera math (shared with analyse.py) -------------------------------------------

def quat_axis(axis, degrees):
    s = math.sin(math.radians(degrees) / 2)
    return [axis[0] * s, axis[1] * s, axis[2] * s, math.cos(math.radians(degrees) / 2)]


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def view_rotation():
    return quat_mul(quat_axis((0, 1, 0), YAW), quat_axis((1, 0, 0), PITCH))


def back_axis():
    """Camera local +Z in world space (the camera looks along -Z)."""
    y, p = math.radians(YAW), math.radians(PITCH)
    return (math.sin(y) * math.cos(p), -math.sin(p), math.cos(y) * math.cos(p))


def eye_at(distance):
    b = back_axis()
    return tuple(TARGET[i] + distance * b[i] for i in range(3))


def transform(t, rotation=(0, 0, 0, 1)):
    return {"translation": [round(v, 9) for v in t], "rotation": [round(v, 12) for v in rotation], "scale": [1, 1, 1]}


def camera_value(projection):
    value = {"fov_degrees": FOV, "near": NEAR, "far": FAR}
    if projection is not None:
        value["projection"] = projection
    return value


def new_ulid(n):
    alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    suffix = ""
    for _ in range(16):
        suffix = alphabet[n % 32] + suffix
        n //= 32
    return "01JA30AA00" + suffix


def rpc(project, journal, requests):
    before = verify_binary()
    result = subprocess.run([str(CLI), "rpc", str(project), "--journal", str(journal)],
                            input="".join(json.dumps(r) + "\n" for r in requests),
                            capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0 or verify_binary() != before:
        raise SystemExit(f"rpc failed: {result.stderr}")
    replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    for reply in replies:
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    return replies


BEHAVIOR = """
export default defineBehavior<{ tick: number }>({
  initialState: { tick: 0 },
  update(api: ScriptApi, _dt, state) {
    const step = STEPS.find((s) => s.tick === state.tick);
    state.tick++;
    if (!step) return;
    const camera = api.query('Camera').find((e) => e.id === LIVE_ID)!.components.Camera as Camera;
    const value: Camera = { fov_degrees: camera.fov_degrees, near: camera.near, far: camera.far };
    if (step.projection) value.projection = step.projection;
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Camera', value });
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Transform', value: step.transform });
    api.log(JSON.stringify({ t: state.tick - 1, step: step.label }));
  },
});
"""


def build(out):
    project = out / "ortho.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    engine("init", project, "--name", "Orthographic Look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, primitives):
        path = write_gltf(models, name, primitives)
        imported = json.loads(engine("import", project, path.relative_to(out))[0])
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    counter = iter(range(1, 10_000))
    entities, ids = [], {}

    def entity(name, components):
        entity_id = new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": None, "components": components})
        ids[name] = entity_id
        return entity_id

    def static(name, asset, at=(0, 0, 0), shadows=True):
        return entity(name, {"Transform": transform(at), "MeshRenderer": {"mesh": asset, "materials": [], "cast_shadows": shadows}})

    x0, x1, z0, z1 = FLOOR
    floor = importer("floor", [(tiles(x0, x1, z0, z1, 0), "tile_a"), (tiles(x0, x1, z0, z1, 1), "tile_b")])
    static("Floor", floor, shadows=False)
    rail = importer("rail", [(box(-0.08, 0.08, 0.0, 0.14, RAIL_Z[0], RAIL_Z[1]), "rail")])
    for x in RAILS:
        static(f"Rail {x:+.1f}", rail, (x, 0, 0))
    w, h, d = PILLAR
    for name, ((x, z), hue) in PILLARS.items():
        asset = importer(f"pillar_{hue}", [(box(-w / 2, w / 2, 0, h, -d / 2, d / 2), hue)])
        static(f"Pillar {name}", asset, (x, 0, z))

    sun = quat_mul(quat_axis((0, 1, 0), -50), quat_axis((1, 0, 0), -52))
    entity("Sun", {"DirectionalLight": {"color": [1, 0.96, 0.9], "intensity": 2.2, "shadows": {"distance": 70}},
                   "Transform": transform((0, 0, 0), sun)})
    entity("Warm lamp", {"PointLight": {"color": [1, 0.72, 0.42], "intensity": 6, "range": 4.5},
                         "Transform": transform((1.3, 1.1, -3.0))})
    entity("Cool lamp", {"PointLight": {"color": [0.5, 0.7, 1], "intensity": 6, "range": 4.5},
                         "Transform": transform((-1.3, 1.1, -10.0))})

    rotation = view_rotation()
    cameras = {name: entity(f"Camera {name}", {"Camera": camera_value(projection),
                                               "Transform": transform(eye_at(dist), rotation)})
               for name, (dist, projection) in CAMERAS.items()}
    live = entity("Camera live", {"Camera": camera_value(None), "Transform": transform(eye_at(16), rotation)})

    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
            "expected_revision": revision, "description": "Orthographic look-dev scene"}},
        {"id": 3, "method": "project.save"}])
    engine("validate", project)

    steps = [{"tick": t, "label": f"t{t}-d{dist}-" + (p["kind"] + (f"-v{p['vertical_size']}" if "vertical_size" in p else "") if p else "legacy"),
              "projection": p, "transform": transform(eye_at(dist), rotation)} for t, dist, p in LIVE]
    source = out / "live_camera.ts"
    source.write_text(
        "import type { Camera, ScriptApi, Transform } from "
        + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n"
        + "type CameraProjection_ = NonNullable<Camera['projection']>;\n"
        + f"const SCENE_ID = {json.dumps(scene_id)};\nconst LIVE_ID = {json.dumps(live)};\n"
        + "const STEPS: { tick: number; label: string; projection: CameraProjection_ | null; transform: Transform }[] = "
        + json.dumps(steps) + ";\n" + BEHAVIOR, encoding="utf-8")
    run("node", ROOT / "node_modules/typescript/bin/tsc", "--strict", "--noEmit", "--target", "ES2022",
        "--module", "ESNext", "--moduleResolution", "bundler", source)
    script = out / "live_camera.js"
    run("node", ROOT / "tools/build_script.mjs", source, script)
    scene = {"scene": scene_id, "target": TARGET, "yaw": YAW, "pitch": PITCH, "near": NEAR, "far": FAR, "fov": FOV,
             "pillar": PILLAR, "pillars": PILLARS, "rails": RAILS, "rail_z": RAIL_Z, "floor": FLOOR,
             "cameras": {k: {"id": cameras[k], "distance": v[0], "projection": v[1], "eye": eye_at(v[0])}
                         for k, v in CAMERAS.items()},
             "live": {"id": live, "steps": steps, "ticks": LIVE_TICKS, "every": LIVE_EVERY},
             "rotation": rotation, "entities": len(entities)}
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, script, cameras, live


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    frames = 1 + math.ceil(LIVE_TICKS / LIVE_EVERY)
    for w, h in SIZES.values():
        if frames > MAX_FRAMES or frames * w * h * 4 > MAX_RAW:
            raise SystemExit("live capture exceeds the budget")
    start = verify_binary()
    out.mkdir(parents=True)
    project, script, cameras, live = build(out)
    authored = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    shots, unchanged = {}, True
    for size, (w, h) in SIZES.items():
        for name, camera in cameras.items():
            hashes = []
            for repeat in (1, 2):
                png = out / "shots" / f"{name}-{size}-r{repeat}.png"
                png.parent.mkdir(exist_ok=True)
                _, same = engine("screenshot", project, png, "--camera", camera, "--width", w, "--height", h)
                unchanged &= same
                hashes.append(sha(png))
            shots[f"{name}-{size}"] = {"r1": hashes[0], "r2": hashes[1], "equal": hashes[0] == hashes[1]}
    plays = {}
    for size, (w, h) in SIZES.items():
        runs = []
        for repeat in (1, 2):
            target = out / f"live-{size}-r{repeat}"
            stdout, same = engine("play", project, "--ticks", LIVE_TICKS, "--compiled-script", script,
                                  "--camera", live, "--capture-every", LIVE_EVERY, "--width", w, "--height", h,
                                  "--output", target, "--log-output", out / f"live-{size}-r{repeat}.logs.jsonl")
            unchanged &= same
            (out / f"live-{size}-r{repeat}.stdout.json").write_text(stdout, encoding="utf-8")
            runs.append({p.name: sha(p) for p in sorted(target.glob("*.png"))})
        plays[size] = {"frames": runs[0], "repeat_equal": runs[0] == runs[1], "count": len(runs[0])}
    after = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    summary = {"binary_sha256_start": start, "binary_sha256_end": verify_binary(), "binary_unchanged_every_call": unchanged,
               "authored_before": authored, "authored_after": after, "authored_unchanged": authored == after,
               "screenshots": shots, "plays": plays}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary_unchanged": unchanged and start == summary["binary_sha256_end"],
                      "authored_unchanged": summary["authored_unchanged"],
                      "screenshot_repeats_equal": all(v["equal"] for v in shots.values()),
                      "play_repeats_equal": {k: v["repeat_equal"] for k, v in plays.items()}}))


if __name__ == "__main__":
    main()
