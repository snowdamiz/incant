"""Scene authoring helpers: deterministic ULIDs, quaternions and typed commands.

Everything here produces `command.execute` payloads for the public RPC; nothing
writes a project or journal file directly.
"""
import math

CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"


def ulid(group, n):
    """Deterministic canonical ULID: 01JA40 + 2-char group + 18-char counter."""
    digits = []
    for _ in range(18):
        digits.append(CROCKFORD[n % 32])
        n //= 32
    if n or len(group) != 2 or any(c not in CROCKFORD for c in group):
        raise ValueError("ulid out of range")
    return "01JA40" + group + "".join(reversed(digits))


def normalize(v):
    length = math.sqrt(sum(c * c for c in v))
    return [c / length for c in v]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def quat_from_basis(x, y, z):
    """Quaternion [x, y, z, w] of the rotation whose matrix columns are x, y, z."""
    m00, m10, m20 = x
    m01, m11, m21 = y
    m02, m12, m22 = z
    trace = m00 + m11 + m22
    if trace > 0:
        s = math.sqrt(trace + 1.0) * 2
        q = [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s]
    elif m00 > m11 and m00 > m22:
        s = math.sqrt(1.0 + m00 - m11 - m22) * 2
        q = [0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    elif m11 > m22:
        s = math.sqrt(1.0 + m11 - m00 - m22) * 2
        q = [(m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s]
    else:
        s = math.sqrt(1.0 + m22 - m00 - m11) * 2
        q = [(m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s]
    return normalize(q)


def look_rotation(eye, target, up=(0.0, 1.0, 0.0)):
    """Camera rotation: local -Z toward target, local +Y toward up."""
    forward = normalize([t - e for t, e in zip(target, eye)])
    z = [-c for c in forward]
    x = normalize(cross(up, z))
    y = cross(z, x)
    return quat_from_basis(x, y, z)


def quat_axis_angle(axis, degrees):
    axis = normalize(axis)
    h = math.radians(degrees) / 2
    return [axis[0] * math.sin(h), axis[1] * math.sin(h), axis[2] * math.sin(h), math.cos(h)]


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def transform(translation=(0, 0, 0), rotation=(0, 0, 0, 1), scale=(1, 1, 1)):
    return {"translation": [float(c) for c in translation], "rotation": [float(c) for c in rotation],
            "scale": [float(c) for c in scale]}


def entity(entity_id, name, components, parent=None):
    value = {"id": entity_id, "name": name, "components": components}
    if parent:
        value["parent"] = parent
    return value


def model_entity(entity_id, name, asset_id, xform, parent=None):
    return entity(entity_id, name, {"Transform": xform,
                                    "MeshRenderer": {"mesh": asset_id, "materials": [], "cast_shadows": False}}, parent)


def camera_entity(entity_id, name, eye, target, fov=35.0, near=0.02, far=200.0, ortho=None, up=(0.0, 1.0, 0.0)):
    camera = {"fov_degrees": fov, "near": near, "far": far}
    if ortho is not None:
        camera["projection"] = {"kind": "orthographic", "vertical_size": ortho}
    return entity(entity_id, name, {"Transform": transform(eye, look_rotation(eye, target, up)), "Camera": camera})


def create(scene_id, entities):
    return [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities]


def execute(commands, revision, description):
    return {"method": "command.execute",
            "params": {"commands": commands, "expected_revision": revision, "description": description}}
