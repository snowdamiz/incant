#!/usr/bin/env python3
"""Real-engine pixel-perfect camera look-dev for handoff 0031.

Builds a disposable project in a NEW output directory using only public engine
interfaces, then renders it with the supplied frozen binary:

  1. Verifies artifacts/tools/incant_headless against artifacts/tools/binary.json
     before and after every engine invocation.
  2. `init` creates an empty project. An original mathematical "pixel test card"
     (emissive quads at exact 1/32 m logical-pixel edges: 1 px frame ring, corner
     markers, 1-4 px bar gauges, 1/2/4 px checkers, a disc, three diagonal slopes,
     a 1 px crosshair and a small mover) is written as glTF and registered with
     `import`.
  3. One `rpc command.execute` transaction with provenance creates every entity,
     then `project.save` and `validate`. No project JSON is edited directly.
  4. `screenshot --camera` renders every authored camera at several output sizes,
     twice each into separate new files (repeat hash check). Sizes smaller than
     the logical resolution are expected to fail; the exit status and message are
     recorded and no image may be written.
  5. Strict-TypeScript behaviours move a live pixel-perfect camera 1/256 m per
     tick (1/8 logical pixel) and a mover object at the same rate. Script "x" moves
     along +X and switches projection (pixel_perfect -> orthographic ->
     pixel_perfect -> perspective -> pixel_perfect); scripts "diag" and "diag2"
     move along (-X, +Y) and (+X, -Y). All changes are ordinary `set_component` commands. `play` captures
     each run twice; frames are compared by hash.

Usage (repo root):  python3 -I handoffs/0031-pixel-perfect-cameras/tools/pixel_lookdev.py OUT_DIR
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
MAX_FRAMES, MAX_RAW = 128, 256 * 1024 * 1024

PPU = 32
LOGICAL = (320, 180)
HALF_W, HALF_H = LOGICAL[0] / PPU / 2, LOGICAL[1] / PPU / 2  # 5 m x 2.8125 m
EYE_Z = 10.0
NEAR, FAR, FOV = 0.1, 50.0, 50.0
PIXEL = {"kind": "pixel_perfect", "pixels_per_unit": PPU, "reference_resolution": list(LOGICAL)}

# Emissive colours on a black, fully metallic base: flat, light-independent pixels.
PALETTE = {
    "bg": (0.03, 0.035, 0.05),
    "white": (1.0, 1.0, 1.0),
    "amber": (1.0, 0.55, 0.08),
    "red": (1.0, 0.08, 0.06),
    "green": (0.10, 1.0, 0.20),
    "blue": (0.15, 0.30, 1.0),
    "cyan": (0.08, 0.85, 1.0),
    "magenta": (1.0, 0.15, 0.85),
}

# Every output size: (width, height). The base camera is rendered at all of them.
BASE_SIZES = [
    (320, 180),   # 1x, native logical
    (640, 360),   # 2x
    (960, 540),   # 3x
    (961, 541),   # 3x, one spare pixel each way (goes right/bottom)
    (1001, 563),  # 3x, odd spare 41 x 23
    (800, 600),   # 4:3, 2x, letterbox top/bottom and pillars
    (360, 640),   # portrait, 1x
    (1280, 360),  # ultrawide 32:9, 2x, pillarbox
    (1283, 723),  # odd, 4x
]
DETAIL_SIZES = [(320, 180), (960, 540)]
# Cameras whose logical resolution does not fit 320 x 180 use their own native size.
SIZE_OVERRIDES = {"ppu32-256x224": [(256, 224), (960, 540)], "ppu16-160x90": [(160, 90), (320, 180), (960, 540)]}
# Outputs smaller than the logical resolution in either dimension must fail explicitly.
FAIL_SHOTS = [("base", (319, 180)), ("base", (320, 179)), ("ppu32-256x224", (320, 180))]

# name: (translation, rotation quaternion, projection, parent rig or None)
C = 1.0 / PPU


def roll(degrees):
    s = math.sin(math.radians(degrees) / 2)
    return [0.0, 0.0, s, math.cos(math.radians(degrees) / 2)]


IDENT = [0.0, 0.0, 0.0, 1.0]
CAMERAS = {
    "base": ((0, 0, EYE_Z), IDENT, PIXEL, None),
    "x+0.25": ((0.25 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "x+0.49": ((0.49 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "x-0.49": ((-0.49 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "x+0.50": ((0.50 * C, 0, EYE_Z), IDENT, PIXEL, None),  # exact ties: which way each axis rounds
    "x-0.50": ((-0.50 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "y+0.50": ((0, 0.50 * C, EYE_Z), IDENT, PIXEL, None),
    "y-0.50": ((0, -0.50 * C, EYE_Z), IDENT, PIXEL, None),
    "x+0.51": ((0.51 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "x+1.00": ((1.00 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "x+1.49": ((1.49 * C, 0, EYE_Z), IDENT, PIXEL, None),
    "y+0.49": ((0, 0.49 * C, EYE_Z), IDENT, PIXEL, None),
    "y-0.51": ((0, -0.51 * C, EYE_Z), IDENT, PIXEL, None),
    "xy+2.3": ((2.3 * C, 2.3 * C, EYE_Z), IDENT, PIXEL, None),
    "depth+3.3": ((0, 0, EYE_Z + 3.3), IDENT, PIXEL, None),  # forward-axis motion only
    "roll90": ((0, 0, EYE_Z), roll(90), PIXEL, None),
    "rig90-base": ((0, 0, EYE_Z), IDENT, PIXEL, "rig90-base"),
    "rig90+0.3": ((0, 0, EYE_Z), IDENT, PIXEL, "rig90+0.3"),
    "rig90+0.7": ((0, 0, EYE_Z), IDENT, PIXEL, "rig90+0.7"),
    "roll10": ((0, 0, EYE_Z), roll(10), PIXEL, None),
    "ortho": ((0, 0, EYE_Z), IDENT, {"kind": "orthographic", "vertical_size": LOGICAL[1] / PPU}, None),
    "persp": ((0, 0, EYE_Z), IDENT, None, None),
    "ppu16-160x90": ((0, 0, EYE_Z), IDENT, {"kind": "pixel_perfect", "pixels_per_unit": 16, "reference_resolution": [160, 90]}, None),
    "ppu32-256x224": ((0, 0, EYE_Z), IDENT, {"kind": "pixel_perfect", "pixels_per_unit": 32, "reference_resolution": [256, 224]}, None),
    "ppu32-255x143": ((0, 0, EYE_Z), IDENT, {"kind": "pixel_perfect", "pixels_per_unit": 32, "reference_resolution": [255, 143]}, None),
}
# Parent rigs: roll 90 deg about Z; the offset is world X, which is the rolled camera's vertical.
RIGS = {"rig90-base": 0.0, "rig90+0.3": 0.3 * C, "rig90+0.7": 0.7 * C}

# Live scripts. Camera and mover advance together by 1/256 m (1/8 logical pixel) per
# tick along `direction`; projection switches apply at the listed ticks.
STEP = 1.0 / 256
LIVE_SCRIPTS = {
    "x": {"direction": (1, 0), "ticks": 100,
          "switches": {64: {"kind": "orthographic", "vertical_size": LOGICAL[1] / PPU}, 72: PIXEL, 80: None, 88: PIXEL}},
    "diag": {"direction": (-1, 1), "ticks": 48, "switches": {}},
    "diag2": {"direction": (1, -1), "ticks": 48, "switches": {}},
}
# run key: (script, output size, capture every N ticks)
LIVE_RUNS = {"1x": ("x", (320, 180), 1), "3x-odd": ("x", (961, 541), 4), "diag-1x": ("diag", (320, 180), 1),
             "diag2-1x": ("diag2", (320, 180), 1)}
MOVER_AT = (40, 151)  # top-left logical pixel of the 8 x 8 mover in the base frame


def run(*args, check=True):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT)
    if check and result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:3]]} failed:\n{result.stdout}\n{result.stderr}")
    return result


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = hashlib.sha256(CLI.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


def engine(*args, check=True):
    before = verify_binary()
    result = run(CLI, *args, check=check)
    after = verify_binary()
    return result, before == after


# ---- geometry (logical pixel coordinates, origin top-left of the base frame) -------

def wx(px):
    return -HALF_W + px / PPU


def wy(py):
    return HALF_H - py / PPU


def quad_world(x0, x1, y0, y1, z):
    return [(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)], [0, 1, 2, 0, 2, 3]


def rect(px, py, w, h, z=0.0):
    return quad_world(wx(px), wx(px + w), wy(py + h), wy(py), z)


def line(p0, p1, width, z=0.0):
    """A straight strip of `width` logical pixels from p0 to p1 (pixel coords)."""
    (x0, y0), (x1, y1) = (wx(p0[0]), wy(p0[1])), (wx(p1[0]), wy(p1[1]))
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy)
    nx, ny = -dy / length * width / PPU / 2, dx / length * width / PPU / 2
    pts = [(x0 - nx, y0 - ny, z), (x1 - nx, y1 - ny, z), (x1 + nx, y1 + ny, z), (x0 + nx, y0 + ny, z)]
    # Keep counter-clockwise winding seen from +Z.
    cross = (pts[1][0] - pts[0][0]) * (pts[2][1] - pts[0][1]) - (pts[1][1] - pts[0][1]) * (pts[2][0] - pts[0][0])
    return pts, ([0, 1, 2, 0, 2, 3] if cross > 0 else [0, 2, 1, 0, 3, 2])


def disc(cx, cy, r, segments=64, z=0.0):
    pts = [(wx(cx), wy(cy), z)]
    for i in range(segments):
        a = 2 * math.pi * i / segments
        pts.append((wx(cx + r * math.cos(a)), wy(cy - r * math.sin(a)), z))
    idx = []
    for i in range(segments):
        idx += [0, 1 + i, 1 + (i + 1) % segments]
    return pts, idx


def merge(parts):
    positions, indices = [], []
    for pts, idx in parts:
        base = len(positions)
        positions += pts
        indices += [base + i for i in idx]
    return positions, indices


def card():
    """Colour -> list of (positions, indices) parts."""
    parts = {k: [] for k in PALETTE}
    w, h = LOGICAL
    # Background plate behind everything, far larger than any view so camera motion shows no edge.
    parts["bg"].append(quad_world(-40, 40, -40, 40, -1.0))
    # 1 px frame ring on the outermost logical pixels: any crop or offset breaks it.
    parts["amber"] += [rect(0, 0, w, 1), rect(0, h - 1, w, 1), rect(0, 1, 1, h - 2), rect(w - 1, 1, 1, h - 2)]
    # Corner markers (orientation and flip check).
    parts["red"].append(rect(2, 2, 6, 6))
    parts["green"].append(rect(w - 8, 2, 6, 6))
    parts["blue"].append(rect(2, h - 8, 6, 6))
    parts["white"].append(rect(w - 8, h - 8, 6, 6))
    # Vertical bar gauge: 4 bars of width n with gaps of n, for n = 1..4.
    x = 12
    for n in (1, 2, 3, 4):
        for i in range(4):
            parts["white"].append(rect(x + 2 * n * i, 10, n, 24))
        x += 7 * n + 8
    # Horizontal bar gauge.
    y = 44
    for n in (1, 2, 3, 4):
        for i in range(4):
            parts["cyan"].append(rect(12, y + 2 * n * i, 24, n))
        y += 7 * n + 6
    # Checkers of 1, 2 and 4 px cells, 16 x 16 px each.
    for k, (n, ox) in enumerate(((1, 120), (2, 140), (4, 160))):
        for j in range(16 // n):
            for i in range(16 // n):
                if (i + j) % 2 == 0:
                    parts["white"].append(rect(ox + i * n, 10 + j * n, n, n))
    # Disc, radius 24 px, and a 1 px crosshair.
    parts["magenta"].append(disc(240, 66, 24))
    parts["white"] += [rect(160, 150, 1, 28), rect(270, 120, 30, 1)]
    # Diagonal strips, 2 px wide: 45 deg, 1:2 and 17 deg (no rational pixel step).
    parts["amber"].append(line((60, 140), (140, 60), 2))
    parts["white"].append(line((90, 140), (190, 90), 2))
    parts["cyan"].append(line((120, 140), (120 + 90 * math.cos(math.radians(17)), 140 - 90 * math.sin(math.radians(17))), 2))
    return parts


def write_gltf(models, name, by_colour):
    blob, views, accessors, prims, materials = b"", [], [], [], []
    for key, plist in by_colour.items():
        if not plist:
            continue
        positions, indices = merge(plist)
        normals = [(0, 0, 1)] * len(positions)
        k = len(prims)
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
        materials.append({"name": key, "pbrMetallicRoughness": {"baseColorFactor": [0, 0, 0, 1], "metallicFactor": 1},
                          "emissiveFactor": list(PALETTE[key])})
    (models / f"{name}.bin").write_bytes(blob)
    document = {"asset": {"version": "2.0", "generator": "handoff 0031 pixel_lookdev.py"},
                "buffers": [{"uri": f"{name}.bin", "byteLength": len(blob)}],
                "bufferViews": views, "accessors": accessors,
                "meshes": [{"name": name, "primitives": prims}], "materials": materials,
                "nodes": [{"name": name, "mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0}
    path = models / f"{name}.gltf"
    path.write_text(json.dumps(document, indent=1), encoding="utf-8")
    return path


def transform(t, rotation=IDENT):
    return {"translation": [round(v, 12) for v in t], "rotation": [round(v, 12) for v in rotation], "scale": [1, 1, 1]}


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
    return "01JA31AA00" + suffix


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
    const t = state.tick;
    state.tick++;
    if (t >= STEPS.length) return;
    const step = STEPS[t]!;
    const camera = api.query('Camera').find((e) => e.id === LIVE_ID)!.components.Camera as Camera;
    if (step.switch) {
      const value: Camera = { fov_degrees: camera.fov_degrees, near: camera.near, far: camera.far };
      if (step.projection) value.projection = step.projection;
      api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Camera', value });
    }
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: LIVE_ID, component: 'Transform', value: step.camera });
    api.command({ op: 'set_component', scene_id: SCENE_ID, entity_id: MOVER_ID, component: 'Transform', value: step.mover });
    api.log(JSON.stringify({ t, x: step.camera.translation[0], kind: step.switch ? (step.projection?.kind ?? 'legacy') : null }));
  },
});
"""


def build(out):
    project = out / "pixel.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    models = out / "models"
    models.mkdir(parents=True)
    engine("init", project, "--name", "Pixel-perfect Look-dev", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))

    def importer(name, by_colour):
        path = write_gltf(models, name, by_colour)
        imported = json.loads(engine("import", project, path.relative_to(out))[0].stdout)
        return imported["asset"]["id"] if "asset" in imported else imported["id"]

    counter = iter(range(1, 10_000))
    entities, ids = [], {}

    def entity(name, components, parent=None):
        entity_id = new_ulid(next(counter))
        entities.append({"id": entity_id, "name": name, "parent": parent, "components": components})
        ids[name] = entity_id
        return entity_id

    card_asset = importer("card", card())
    entity("Test card", {"Transform": transform((0, 0, 0)),
                         "MeshRenderer": {"mesh": card_asset, "materials": [], "cast_shadows": False}})
    mover_asset = importer("mover", {"green": [rect(MOVER_AT[0], MOVER_AT[1], 8, 8, 0.01)]})
    mover = entity("Mover", {"Transform": transform((0, 0, 0)),
                             "MeshRenderer": {"mesh": mover_asset, "materials": [], "cast_shadows": False}})

    rigs = {name: entity(f"Rig {name}", {"Transform": transform((offset, 0, 0), roll(90))})
            for name, offset in RIGS.items()}
    cameras = {}
    for name, (t, rot, projection, rig) in CAMERAS.items():
        cameras[name] = entity(f"Camera {name}", {"Camera": camera_value(projection), "Transform": transform(t, rot)},
                               parent=rigs[rig] if rig else None)
    live = entity("Camera live", {"Camera": camera_value(PIXEL), "Transform": transform((0, 0, EYE_Z))})

    revision = rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
            "expected_revision": revision, "description": "Pixel-perfect look-dev scene (handoff 0031)"}},
        {"id": 3, "method": "project.save"}])
    engine("validate", project)

    scripts = {}
    for key, spec in LIVE_SCRIPTS.items():
        steps, (ux, uy) = [], spec["direction"]
        for t in range(spec["ticks"]):
            k = (t + 1) * STEP  # position commanded at tick t, visible from the next tick
            switch = t in spec["switches"]
            steps.append({"camera": transform((ux * k, uy * k, EYE_Z)), "mover": transform((ux * k, uy * k, 0)),
                          "switch": switch, "projection": spec["switches"][t] if switch else None})
        source = out / f"live_{key}.ts"
        source.write_text(
            "import type { Camera, ScriptApi, Transform } from "
            + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n"
            + "type Projection = NonNullable<Camera['projection']>;\n"
            + f"const SCENE_ID = {json.dumps(scene_id)};\nconst LIVE_ID = {json.dumps(live)};\n"
            + f"const MOVER_ID = {json.dumps(mover)};\n"
            + "const STEPS: { camera: Transform; mover: Transform; switch: boolean; projection: Projection | null }[] = "
            + json.dumps(steps) + ";\n" + BEHAVIOR, encoding="utf-8")
        run("node", ROOT / "node_modules/typescript/bin/tsc", "--strict", "--noEmit", "--target", "ES2022",
            "--module", "ESNext", "--moduleResolution", "bundler", source)
        scripts[key] = out / f"live_{key}.js"
        run("node", ROOT / "tools/build_script.mjs", source, scripts[key])
    scene = {"scene": scene_id, "ppu": PPU, "logical": LOGICAL, "eye_z": EYE_Z, "near": NEAR, "far": FAR, "fov": FOV,
             "palette": PALETTE, "mover_at": MOVER_AT,
             "cameras": {k: {"id": cameras[k], "translation": v[0], "rotation": v[1], "projection": v[2],
                             "rig": v[3], "rig_offset": RIGS.get(v[3]) if v[3] else None} for k, v in CAMERAS.items()},
             "live": {"id": live, "mover": mover, "step": STEP,
                      "scripts": {k: {"direction": v["direction"], "ticks": v["ticks"],
                                      "switches": {str(t): p for t, p in v["switches"].items()}}
                                  for k, v in LIVE_SCRIPTS.items()},
                      "runs": {k: {"script": v[0], "size": v[1], "every": v[2]} for k, v in LIVE_RUNS.items()}},
             "entities": len(entities)}
    (out / "scene.json").write_text(json.dumps(scene, indent=1), encoding="utf-8")
    return project, scripts, cameras, live


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
    for script_key, (w, h), every in LIVE_RUNS.values():
        frames = 1 + math.ceil(LIVE_SCRIPTS[script_key]["ticks"] / every)
        if frames > MAX_FRAMES or frames * w * h * 4 > MAX_RAW:
            raise SystemExit("live capture exceeds the budget")
    start = verify_binary()
    out.mkdir(parents=True)
    project, scripts, cameras, live = build(out)
    authored = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    shots, failures, unchanged = {}, {}, True
    (out / "shots").mkdir()
    plan = [("base", s) for s in BASE_SIZES] + [
        (n, s) for n in cameras if n != "base" for s in SIZE_OVERRIDES.get(n, DETAIL_SIZES)]
    for name, (w, h) in plan:
        hashes = []
        for repeat in (1, 2):
            png = out / "shots" / f"{name}-{w}x{h}-r{repeat}.png"
            _, same = engine("screenshot", project, png, "--camera", cameras[name], "--width", w, "--height", h)
            unchanged &= same
            hashes.append(sha(png))
        shots[f"{name}-{w}x{h}"] = {"r1": hashes[0], "r2": hashes[1], "equal": hashes[0] == hashes[1]}
    for name, (w, h) in FAIL_SHOTS:
        png = out / "shots" / f"{name}-{w}x{h}-expected-failure.png"
        result, same = engine("screenshot", project, png, "--camera", cameras[name], "--width", w, "--height", h,
                              check=False)
        unchanged &= same
        failures[f"{name}-{w}x{h}"] = {"exit": result.returncode, "stderr": result.stderr.strip()[-400:],
                                "stdout": result.stdout.strip()[-400:], "image_written": png.exists()}
    plays = {}
    for key, (script_key, (w, h), every) in LIVE_RUNS.items():
        runs = []
        for repeat in (1, 2):
            target = out / f"live-{key}-r{repeat}"
            result, same = engine("play", project, "--ticks", LIVE_SCRIPTS[script_key]["ticks"],
                                  "--compiled-script", scripts[script_key],
                                  "--camera", live, "--capture-every", every, "--width", w, "--height", h,
                                  "--output", target, "--log-output", out / f"live-{key}-r{repeat}.logs.jsonl")
            unchanged &= same
            (out / f"live-{key}-r{repeat}.stdout.json").write_text(result.stdout, encoding="utf-8")
            runs.append({p.name: sha(p) for p in sorted(target.glob("*.png"))})
        plays[key] = {"frames": runs[0], "repeat_equal": runs[0] == runs[1], "count": len(runs[0])}
    after = {p.name: sha(p) for p in (project, project.with_suffix(".journal.jsonl"))}
    summary = {"binary_sha256_start": start, "binary_sha256_end": verify_binary(), "binary_unchanged_every_call": unchanged,
               "authored_before": authored, "authored_after": after, "authored_unchanged": authored == after,
               "screenshots": shots, "expected_failures": failures, "plays": plays}
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary_unchanged": unchanged and start == summary["binary_sha256_end"],
                      "authored_unchanged": summary["authored_unchanged"],
                      "screenshot_repeats_equal": all(v["equal"] for v in shots.values()),
                      "screenshots": len(shots),
                      "failures": {k: (v["exit"], v["image_written"]) for k, v in failures.items()},
                      "play_repeats_equal": {k: v["repeat_equal"] for k, v in plays.items()}}))


if __name__ == "__main__":
    main()
