"""Scene descriptions for handoff 0064 material review. Pure data; no I/O.

Units are metres; the floor is y = 0. Cameras and lights look along local -Z.
Each board is its own project, authored only through incant_headless:

  M  material maps (base colour, metallic-roughness, normal, occlusion,
     emissive), opaque/mask/blend, single/double-sided and mirrored instances,
     mipmapped versus unmipmapped minification, directional shadow cascades,
     an equirectangular environment, one point and one spot light
  L  120 moving local lights (96 point, 24 spot) over lit geometry: four mask
     words; Velocity moves every light each tick so cluster masks change
  Z1 one textured sphere with no authored light or environment (preview key)
  Z0 a camera only (empty scene)
  R  a textured panel whose image is reimported (version 1 then version 2)
  S  (run r2) low sun over a plain floor with casters out to 49 m: cascades and fade
"""

import math

CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
PALETTE = [(1.0, 0.25, 0.2), (1.0, 0.6, 0.15), (0.95, 0.95, 0.2), (0.3, 1.0, 0.3), (0.2, 0.9, 0.9),
           (0.25, 0.45, 1.0), (0.7, 0.3, 1.0), (1.0, 0.3, 0.75)]


def ulid(prefix, n):
    digits = ""
    for _ in range(26 - len(prefix)):
        digits = CROCKFORD[n % 32] + digits
        n //= 32
    return prefix + digits


def ulid_hex(text):
    value = 0
    for ch in text:
        value = value * 32 + CROCKFORD.index(ch)
    return f"{value:032x}"


def quat_axis(axis, degrees):
    h = math.radians(degrees) / 2
    q = [0.0, 0.0, 0.0, math.cos(h)]
    q[axis] = math.sin(h)
    return q


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def look(direction):
    """Rotation taking local -Z to the given direction (yaw about +Y after pitch about +X)."""
    length = math.sqrt(sum(v * v for v in direction))
    dx, dy, dz = (v / length for v in direction)
    pitch = math.degrees(math.asin(max(-1.0, min(1.0, dy))))
    yaw = math.degrees(math.atan2(-dx, -dz)) if abs(dx) + abs(dz) > 1e-9 else 0.0
    return quat_mul(quat_axis(1, yaw), quat_axis(0, pitch))


def look_at(eye, target):
    return look([t - e for t, e in zip(target, eye)])


class Board:
    def __init__(self, key, prefix, title):
        self.key, self.prefix, self.title = key, prefix, title
        self.items, self.by_name = [], {}

    def add(self, name, *, at=(0.0, 0.0, 0.0), rotation=None, scale=(1.0, 1.0, 1.0), **components):
        assert name not in self.by_name, name
        item = {"name": name, "slot": len(self.items) + 1, "at": [float(v) for v in at],
                "rotation": rotation or [0.0, 0.0, 0.0, 1.0], "scale": [float(v) for v in scale], **components}
        self.items.append(item)
        self.by_name[name] = item
        return item

    def model(self, name, model, *, shadows=True, **kw):
        return self.add(name, mesh={"model": model, "cast_shadows": shadows}, **kw)

    def camera(self, name, eye, target, *, fov=50.0, far=60.0, ortho=None):
        projection = {"kind": "perspective"} if ortho is None else {"kind": "orthographic", "vertical_size": ortho}
        return self.add(name, at=eye, rotation=look_at(eye, target),
                        camera={"fov_degrees": fov, "near": 0.1, "far": far, "projection": projection})

    def id(self, name):
        return ulid(self.prefix, self.by_name[name]["slot"])

    def models(self):
        return sorted({i["mesh"]["model"] for i in self.items if "mesh" in i})

    def textures(self):
        return sorted({i["environment"]["texture"] for i in self.items if "environment" in i})


def board_m():
    b = Board("M", "01JA64MA00", "material maps, alpha, culling, shadows, environment, minification")
    b.camera("camera-wide", (0.0, 3.2, 9.8), (0.0, 0.5, -1.0))
    b.camera("camera-close", (-1.3, 1.15, 3.6), (-1.1, 0.7, -0.8), fov=45.0)
    b.model("floor-mip", "floor-mip", at=(-6.0, 0.0, -8.0), scale=(12.0, 1.0, 24.0), shadows=False)
    b.model("floor-nomip", "floor-nomip", at=(6.0, 0.0, -8.0), scale=(12.0, 1.0, 24.0), shadows=False)
    r = 0.55
    for x, name, model in ((-4.0, "S1-checker", "sphere-checker"), (-2.7, "S2-metallic-roughness-map", "sphere-mr"),
                           (-1.4, "S3-chrome", "sphere-chrome"), (-0.1, "S4-red-dielectric", "sphere-red")):
        b.model(name, model, at=(x, r, 1.6), scale=(r, r, r))
    b.model("S5-cube-behind-glass", "cube-checker", at=(1.2, 0.4, 0.7), rotation=quat_axis(1, 30.0),
            scale=(0.8, 0.8, 0.8))
    b.model("S5-glass-blend", "sphere-glass", at=(1.2, r, 1.6), scale=(r, r, r), shadows=False)
    b.model("S6-checker-mirrored", "sphere-checker", at=(2.5, r, 1.6), scale=(-r, r, r))
    b.model("S7-white-cube", "cube-white", at=(3.8, 0.45, 1.6), rotation=quat_axis(1, 20.0), scale=(0.9, 0.9, 0.9))
    q, y = 0.6, 0.75
    rows = [(-4.6, "Q1-normal-map", "quad-normal", None, (q, q, q)),
            (-3.25, "Q2-occlusion-map", "quad-occlusion", None, (q, q, q)),
            (-1.9, "Q3-emissive-map", "quad-emissive", None, (q, q, q)),
            (-0.55, "Q4-alpha-mask-double", "quad-mask", None, (q, q, q)),
            (0.8, "Q5-single-front", "quad-arrow-single", None, (q, q, q)),
            (2.15, "Q6-single-back-culled", "quad-arrow-single", quat_axis(1, 180.0), (q, q, q)),
            (3.5, "Q7-double-back", "quad-arrow-double", quat_axis(1, 180.0), (q, q, q)),
            (4.85, "Q8-single-mirrored", "quad-arrow-single", None, (-q, q, q))]
    for x, name, model, rotation, scale in rows:
        b.model(name, model, at=(x, y, -0.8), rotation=rotation, scale=scale)
    b.model("Q4-red-behind-mask", "sphere-red", at=(-0.55, 0.45, -1.7), scale=(0.45, 0.45, 0.45))
    b.add("sun-shadowed", rotation=quat_mul(quat_axis(1, -25.0), quat_axis(0, -48.0)),
          directional={"color": [1.0, 0.95, 0.88], "intensity": 2.2, "shadows": {"distance": 30.0}})
    b.add("point-warm", at=(-1.25, 1.1, 0.4), point={"color": [1.0, 0.6, 0.3], "intensity": 3.0, "range": 3.0})
    spot_at = (3.2, 2.4, 1.2)
    b.add("spot-cool", at=spot_at, rotation=look_at(spot_at, (3.0, 0.4, -0.8)),
          spot={"color": [0.4, 0.7, 1.0], "intensity": 12.0, "range": 6.0, "inner_degrees": 14.0,
                "outer_degrees": 24.0})
    b.add("environment-sky", environment={"texture": "sky.png", "intensity": 1.0, "rotation_degrees": 30.0})
    return b


def board_l():
    b = Board("L", "01JA64KX00", "120 moving local lights")
    b.camera("camera-persp", (0.0, 9.5, 8.5), (0.0, 0.0, -5.0), far=50.0)
    b.camera("camera-ortho", (0.0, 14.0, -5.0), (0.0, 0.0, -5.0), far=40.0, ortho=16.0)
    b.model("floor", "floor-gray", at=(0.0, 0.0, -4.0), scale=(26.0, 1.0, 22.0), shadows=False)
    for i, x in enumerate((-6.0, -3.0, 0.0, 3.0, 6.0)):
        for j, z in enumerate((-9.0, -5.0, -1.0)):
            b.model(f"ball-{i}-{j}", "sphere-white", at=(x, 0.6, z), scale=(0.6, 0.6, 0.6))
    for k, (x, z) in enumerate(((-4.5, -3.0), (4.5, -7.0), (0.0, -11.0))):
        b.model(f"pillar-{k}", "cube-white", at=(x, 1.0, z), scale=(0.6, 2.0, 0.6))
    b.add("key-dim", rotation=look((0.3, -1.0, -0.4)), directional={"color": [0.6, 0.65, 0.8], "intensity": 0.12})
    b.add("environment-dim", environment={"texture": "sky-dim.png", "intensity": 0.25, "rotation_degrees": 0.0})
    for j in range(8):
        for i in range(12):
            angle = math.radians((i * 37 + j * 91) % 360)
            speed = 0.6 + 0.3 * ((i + j) % 3)
            b.add(f"point-{i:02d}-{j}", at=(-8.8 + 1.6 * i, 0.6 + 0.3 * ((i + j) % 3), -10.6 + 1.6 * j),
                  point={"color": list(PALETTE[(i * 5 + j * 3) % 8]), "intensity": 0.6 + 0.4 * ((i + 2 * j) % 3),
                         "range": 1.6 + 0.35 * ((i * 3 + j * 5) % 4)},
                  velocity=[speed * math.cos(angle), 0.0, speed * math.sin(angle)])
    for k in range(24):
        a = math.radians(15.0 * k)
        eye = (7.5 * math.sin(a), 3.0, -5.0 + 7.5 * math.cos(a))
        target = (3.0 * math.sin(a), 0.0, -5.0 + 3.0 * math.cos(a))
        b.add(f"spot-{k:02d}", at=eye, rotation=look_at(eye, target),
              spot={"color": [1.0, 0.75, 0.45] if k % 2 == 0 else [0.45, 0.65, 1.0], "intensity": 45.0,
                    "range": 8.0, "inner_degrees": 10.0, "outer_degrees": 18.0},
              velocity=[-0.8 * math.cos(a), 0.0, 0.8 * math.sin(a)])
    return b


def board_z1():
    b = Board("Z1", "01JA64ZE00", "one textured sphere, no authored light or environment")
    b.camera("camera", (0.0, 0.3, 3.2), (0.0, 0.0, 0.0), fov=45.0, far=20.0)
    b.model("sphere", "sphere-checker", rotation=quat_axis(1, -30.0))
    return b


def board_z0():
    b = Board("Z0", "01JA64ZE10", "camera only")
    b.camera("camera", (0.0, 0.0, 3.0), (0.0, 0.0, 0.0), fov=45.0, far=20.0)
    return b


def board_r():
    b = Board("R", "01JA64RE00", "reimported panel image")
    b.camera("camera", (0.0, 0.0, 3.0), (0.0, 0.0, 0.0), fov=45.0, far=20.0)
    b.model("panel", "quad-panel", at=(-0.35, 0.0, 0.0), scale=(0.9, 0.9, 0.9))
    b.model("unchanged-sphere", "sphere-checker", at=(1.25, -0.45, 0.3), scale=(0.45, 0.45, 0.45))
    b.add("key", rotation=look((-0.3, -0.5, -1.0)), directional={"color": [1.0, 1.0, 1.0], "intensity": 2.0})
    return b


def board_s():
    """Low sun over a plain floor; casters from 3 m to 49 m cross every cascade and the 40 m fade."""
    b = Board("S", "01JA64SH00", "directional shadow cascades and distance fade")
    b.camera("camera", (0.0, 4.0, 10.0), (0.0, 0.0, -12.0), far=80.0)
    b.model("floor", "floor-gray", at=(0.0, 0.0, -18.0), scale=(30.0, 1.0, 48.0), shadows=False)
    for k, z in enumerate((3.0, -3.0, -9.0, -15.0, -21.0, -27.0, -33.0, -39.0)):
        b.model(f"pillar-{k}-z{int(z)}", "cube-white", at=(-2.5 if k % 2 == 0 else 2.5, 1.25, z),
                scale=(0.5, 2.5, 0.5))
    b.model("sphere-near", "sphere-red", at=(-0.6, 0.6, 1.5), scale=(0.6, 0.6, 0.6))
    b.model("sphere-mid", "sphere-checker", at=(1.0, 0.8, -6.0), scale=(0.8, 0.8, 0.8))
    b.model("mask-caster", "quad-mask", at=(-1.0, 1.3, -1.5), scale=(0.9, 0.9, 0.9))
    b.model("cube-mirrored", "cube-checker", at=(0.6, 0.5, -12.0), rotation=quat_axis(1, 35.0),
            scale=(-1.0, 1.0, 1.0))
    b.add("sun-low", rotation=look((0.6, -0.45, -0.66)),
          directional={"color": [1.0, 0.93, 0.82], "intensity": 2.5, "shadows": {"distance": 40.0}})
    b.add("environment-fill", environment={"texture": "sky.png", "intensity": 0.35, "rotation_degrees": 30.0})
    return b


BOARDS = {"M": board_m, "L": board_l, "Z1": board_z1, "Z0": board_z0, "R": board_r}
# Added after the first GPU runs (shadows were too faint to review on board M); authored as run r2.
SHADOW_BOARDS = {"S": board_s}


def local_light_names(board):
    return [i["name"] for i in board.items if "point" in i] + [i["name"] for i in board.items if "spot" in i]


def entity_values(board, ids):
    """create_entity values. ids maps model names and texture file names to imported asset IDs."""
    out = []
    for item in board.items:
        components = {"Transform": {"translation": item["at"], "rotation": item["rotation"], "scale": item["scale"]}}
        if "camera" in item:
            components["Camera"] = item["camera"]
        if "mesh" in item:
            components["MeshRenderer"] = {"mesh": ids[item["mesh"]["model"]], "materials": [],
                                          "cast_shadows": item["mesh"]["cast_shadows"]}
        if "directional" in item:
            components["DirectionalLight"] = item["directional"]
        if "point" in item:
            components["PointLight"] = item["point"]
        if "spot" in item:
            components["SpotLight"] = item["spot"]
        if "environment" in item:
            env = item["environment"]
            components["EnvironmentLight"] = {"texture": ids[env["texture"]], "intensity": env["intensity"],
                                              "rotation_degrees": env["rotation_degrees"]}
        if "velocity" in item:
            components["Velocity"] = {"linear": item["velocity"]}
        out.append({"id": board.id(item["name"]), "name": item["name"], "parent": None, "components": components})
    return out


def describe(board):
    return {"key": board.key, "title": board.title, "items": board.items,
            "ids": {i["name"]: board.id(i["name"]) for i in board.items},
            "hex": {i["name"]: ulid_hex(board.id(i["name"])) for i in board.items}}
