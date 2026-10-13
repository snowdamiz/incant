"""Scene descriptions for handoff 0066 cluster review. Pure data; no I/O.

The ULID, quaternion, Board and entity helpers are copied from handoff 0064's
committed tools/boards.py (read-only reuse); the boards are new. Units are
metres; the floor is y = 0. Cameras and lights look along local -Z. Each board
is its own project, authored only through incant_headless (the command bus):

  C  4096 local lights (3072 point, 1024 down-facing spot) in a 64 x 64 carpet
     over a floor with spheres and pillars, behind one dim directional key (a
     directional prefix, so local mask bit k is packed light k + 1). 4063 lights
     carry a Velocity; 33 static point lights were chosen so that, seen from the
     perspective camera at 960x540, each projected centre lies within 4 px of a
     64-pixel cluster tile edge (one within 6 px of a tile corner). The review
     despawns lights at runtime to reach 33, 32, 31, 1 and 0 local lights.
     Perspective and orthographic cameras.
  D  minimal directional-only control: floor, sphere, cube, one directional
     light; no local light, no environment.
"""

import math

CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
PALETTE = [(1.0, 0.25, 0.2), (1.0, 0.6, 0.15), (0.95, 0.95, 0.2), (0.3, 1.0, 0.3), (0.2, 0.9, 0.9),
           (0.25, 0.45, 1.0), (0.7, 0.3, 1.0), (1.0, 0.3, 0.75)]
# Review framing the boundary selection is computed for (the main persp frame).
FRAME = (960, 540)
TILE = 64
NEAR, PERSP_FAR, ORTHO_FAR = 0.1, 60.0, 40.0
CARPET = 64  # 64 x 64 = 4096 local lights


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


def rotate(q, v):
    """Rotate vector v by unit quaternion q = [x, y, z, w]."""
    x, y, z, w = q
    t = [2 * (y * v[2] - z * v[1]), 2 * (z * v[0] - x * v[2]), 2 * (x * v[1] - y * v[0])]
    return [v[0] + w * t[0] + (y * t[2] - z * t[1]), v[1] + w * t[1] + (z * t[0] - x * t[2]),
            v[2] + w * t[2] + (x * t[1] - y * t[0])]


def project(camera, point, size=FRAME):
    """Pixel position (y down) and forward depth of a world point, as look_to_rh + perspective_rh
    (vertical FOV) or orthographic_rh with the authored vertical size place it."""
    eye, q, cam = camera["at"], camera["rotation"], camera["camera"]
    forward, up = rotate(q, (0.0, 0.0, -1.0)), rotate(q, (0.0, 1.0, 0.0))
    right = [forward[1] * up[2] - forward[2] * up[1], forward[2] * up[0] - forward[0] * up[2],
             forward[0] * up[1] - forward[1] * up[0]]
    d = [p - e for p, e in zip(point, eye)]
    x, y, depth = (sum(a * b for a, b in zip(d, axis)) for axis in (right, up, forward))
    aspect = size[0] / size[1]
    if cam["projection"]["kind"] == "orthographic":
        half = cam["projection"]["vertical_size"] / 2
        ndc = (x / (half * aspect), y / half)
    else:
        tangent = math.tan(math.radians(cam["fov_degrees"]) / 2)
        ndc = (x / (depth * tangent * aspect), y / (depth * tangent))
    return (ndc[0] + 1) / 2 * size[0], (1 - ndc[1]) / 2 * size[1], depth


def slice_bounds(far):
    return [NEAR * (far / NEAR) ** (k / 24) for k in range(25)]


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
                        camera={"fov_degrees": fov, "near": NEAR, "far": far, "projection": projection})

    def id(self, name):
        return ulid(self.prefix, self.by_name[name]["slot"])

    def models(self):
        return sorted({i["mesh"]["model"] for i in self.items if "mesh" in i})

    def textures(self):
        return sorted({i["environment"]["texture"] for i in self.items if "environment" in i})


def carpet_light(i, j):
    """Light (i, j) of the 64 x 64 carpet: kind, position, parameters and velocity."""
    at = (-17.6 + 0.55 * i, 0.3 + 0.2 * ((i + 2 * j) % 3), 0.5 - 0.55 * j)
    angle = math.radians((i * 37 + j * 91) % 360)
    speed = 0.6 + 0.2 * ((i + j) % 3)
    velocity = [speed * math.cos(angle), 0.0, speed * math.sin(angle)]
    if (i + j) % 4 == 0:
        warm = (i + j) % 8 == 0
        return "spot", at, {"color": [1.0, 0.78, 0.5] if warm else [0.5, 0.7, 1.0], "intensity": 3.0,
                            "range": 1.2, "inner_degrees": 20.0, "outer_degrees": 35.0}, velocity
    return "point", at, {"color": list(PALETTE[(i * 5 + j * 3) % 8]),
                         "intensity": 0.5 + 0.25 * ((i + 2 * j) % 4),
                         "range": 0.55 + 0.1 * ((3 * i + 5 * j) % 4)}, velocity


def boundary_selection(camera):
    """33 static point lights whose projected centres sit within 4 px of a tile edge, spread by
    farthest-point sampling from the corner light nearest the frame centre (within 6 px of a tile
    corner). Deterministic; returns (names in carpet order, per-light geometry)."""
    width, height = FRAME
    bounds = slice_bounds(PERSP_FAR)
    candidates, corners = [], []
    for j in range(CARPET):
        for i in range(CARPET):
            kind, at, params, _ = carpet_light(i, j)
            if kind != "point":
                continue
            px, py, depth = project(camera, at)
            if not (24 <= px <= width - 24 and 24 <= py <= height - 24 and 3.0 <= depth <= 30.0):
                continue
            dx, dy = abs(px - TILE * round(px / TILE)), abs(py - TILE * round(py / TILE))
            entry = {"name": f"light-{j:02d}-{i:02d}", "px": round(px, 3), "py": round(py, 3),
                     "depth": round(depth, 4), "range": params["range"],
                     "tile_edge_px": round(min(dx, dy), 3), "tile_corner": dx <= 6 and dy <= 6,
                     "nearest_slice_boundary_m": round(min(abs(depth - b) for b in bounds), 4)}
            if min(dx, dy) <= 4:
                candidates.append(entry)
            if dx <= 6 and dy <= 6:
                corners.append(entry)
    centre = (width / 2, height / 2)
    corner = min(corners, key=lambda e: (e["px"] - centre[0]) ** 2 + (e["py"] - centre[1]) ** 2)
    chosen = [corner]
    pool = [e for e in candidates if e["name"] != corner["name"]]
    while len(chosen) < 33:
        best = max(pool, key=lambda e: (min((e["px"] - c["px"]) ** 2 + (e["py"] - c["py"]) ** 2
                                            for c in chosen), e["name"]))
        chosen.append(best)
        pool.remove(best)
    chosen.sort(key=lambda e: e["name"])  # carpet (= ID = packing) order
    return chosen, corner["name"]


def board_c():
    b = Board("C", "01JA66CA00", "4096 local lights over cluster tile and slice boundaries")
    persp = b.camera("camera-persp", (0.0, 6.0, 7.5), (0.0, 0.0, -9.0), fov=55.0, far=PERSP_FAR)
    b.camera("camera-ortho", (0.0, 25.0, -14.0), (0.0, 0.0, -14.0), far=ORTHO_FAR, ortho=20.0)
    b.model("floor", "floor-gray", at=(0.0, 0.0, -16.0), scale=(40.0, 1.0, 40.0), shadows=False)
    for k, (x, z) in enumerate(((-4.0, -3.0), (3.0, -6.0), (-2.0, -11.0), (5.0, -15.0), (-6.0, -20.0))):
        b.model(f"sphere-{k}", "sphere-white", at=(x, 0.8, z), scale=(0.8, 0.8, 0.8))
    for k, (x, z) in enumerate(((0.0, -8.0), (-8.0, -14.0), (8.0, -10.0))):
        b.model(f"pillar-{k}", "cube-white", at=(x, 1.5, z), scale=(0.5, 3.0, 0.5))
    b.add("key-dim", rotation=look((0.3, -1.0, -0.4)), directional={"color": [0.6, 0.65, 0.8], "intensity": 0.1})
    b.add("environment-dim", environment={"texture": "sky-dim.png", "intensity": 0.2, "rotation_degrees": 0.0})
    chosen, corner = boundary_selection(persp)
    static = {e["name"] for e in chosen}
    down = look((0.0, -1.0, 0.0))
    for j in range(CARPET):
        for i in range(CARPET):
            kind, at, params, velocity = carpet_light(i, j)
            name = f"light-{j:02d}-{i:02d}"
            extra = {} if name in static else {"velocity": velocity}
            if kind == "spot":
                b.add(name, at=at, rotation=down, spot=params, **extra)
            else:
                b.add(name, at=at, point=params, **extra)
    b.boundary = chosen
    b.corner = corner
    return b


def board_d():
    b = Board("D", "01JA66DA00", "minimal directional-only control")
    b.camera("camera", (0.0, 2.2, 5.5), (0.0, 0.6, 0.0), fov=45.0, far=30.0)
    b.model("floor", "floor-gray", at=(0.0, 0.0, -2.0), scale=(12.0, 1.0, 12.0), shadows=False)
    b.model("sphere", "sphere-white", at=(-1.2, 0.6, 0.0), scale=(0.6, 0.6, 0.6))
    b.model("cube", "cube-white", at=(1.2, 0.5, -0.5), rotation=quat_axis(1, 25.0))
    b.add("sun", rotation=look((-0.4, -0.8, -0.5)), directional={"color": [1.0, 0.95, 0.88], "intensity": 2.0})
    return b


BOARDS = {"C": board_c, "D": board_d}


def local_light_names(board):
    """Local lights in ID (slot) order, which is the renderer's packing and mask-bit order."""
    return [i["name"] for i in board.items if "point" in i or "spot" in i]


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
    out = {"key": board.key, "title": board.title, "items": board.items,
           "ids": {i["name"]: board.id(i["name"]) for i in board.items},
           "hex": {i["name"]: ulid_hex(board.id(i["name"])) for i in board.items}}
    if hasattr(board, "boundary"):
        out["boundary_lights"] = board.boundary
        out["corner_light"] = board.corner
        out["slice_bounds_persp"] = slice_bounds(PERSP_FAR)
    return out
