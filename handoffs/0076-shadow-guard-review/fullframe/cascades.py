"""Pre-execution derivation for 0076's full-frame stationary coverage. CPU only,
standard library; a derivation, not a measurement.

It replicates crates/incant_render/src/shadows/cascade.rs::fit (f636a2d5,
unchanged math from 0aa02559) for the stationary pose at the full 960x540
output, without the receiver guard (the baseline's temporal-off fit) and with
guard(rect) = (1/960, 1/540) (the candidate's temporal-off fit, and both sources'
temporal-on fit). For each cascade it reports the radius quantization, the
texel-snapped centre and the depth range (caster bounds extended by the frustum
slice corners), and so which cascades the candidate can change in the
temporal-off reference. Constants are the frozen fixture's (cross-checked by
reproducing 0072's cascade_texels.txt full-frame line), with f32 rounding where
the renderer stores f32 values.

  python3 -I -B cascades.py
"""
import json
import math
import struct

MAP_SIZE, CASCADES = 1024, 4
NEAR, FAR, SHADOW_DISTANCE = 0.1, 120.0, 60.0
CAMERA_AT, PITCH = (0.4, 4.2, 8.5), -0.42
SUN = (38.0, 240.0)  # elevation, azimuth
# Shadow-caster batch bounds (world AABBs): the look-dev mesh (ground quad x +-14,
# z 10..-110; tallest point the ramp post top at 1.6 sin 25deg + 0.6 + 0.75), the
# mover cube at its start and the grille card quad.
RAMP_POST_TOP = 1.6 * math.sin(math.radians(25)) + 0.6 + 0.75
CASTERS = [((-14.0, 0.0, -110.0), (14.0, RAMP_POST_TOP, 10.0)),
           ((-3.1, 0.5, -1.5), (-2.1, 1.5, -0.5)),
           ((1.4, 0.15, -0.6), (3.4, 2.15, -0.6))]


def f32(v):
    return struct.unpack("f", struct.pack("f", v))[0]


def rot_x(a, v):
    c, s = math.cos(a), math.sin(a)
    return (v[0], c * v[1] - s * v[2], s * v[1] + c * v[2])


def rot_y(a, v):
    c, s = math.cos(a), math.sin(a)
    return (c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2])


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def norm(a):
    n = math.sqrt(dot(a, a))
    return tuple(x / n for x in a)


def boundaries():
    near, far = NEAR, min(FAR, SHADOW_DISTANCE)
    b = [near] * (CASCADES + 1)
    for i in range(1, CASCADES + 1):
        t = i / CASCADES
        b[i] = 0.5 * (near * (far / near) ** t + near + (far - near) * t)
    b[CASCADES] = far
    return b


def light_axes():
    """glam look_to_rh(0, forward, Y): light-space x = s.p, y = u.p, z = -f.p."""
    forward = norm(rot_y(math.radians(SUN[1]), rot_x(-math.radians(SUN[0]), (0.0, 0.0, -1.0))))
    s = norm(cross(forward, (0.0, 1.0, 0.0)))
    u = cross(s, forward)
    return forward, s, u


def fit(guard):
    """Per cascade: radius, quantized radius, texel, snapped centre, depth range."""
    aspect = f32(960 / 540)
    vertical = math.tan(f32(math.radians(50.0)) * 0.5)
    right, up, back = (rot_x(PITCH, axis) for axis in ((1, 0, 0), (0, 1, 0), (0, 0, 1)))
    forward, s, u = light_axes()
    caster = [-dot(forward, (x, y, z)) for lo, hi in CASTERS for x in (lo[0], hi[0])
              for y in (lo[1], hi[1]) for z in (lo[2], hi[2])]
    b = boundaries()
    rows = []
    for i in range(CASCADES):
        start = b[0] if i == 0 else b[i] - 0.1 * (b[i] - b[i - 1])
        end = b[i + 1]
        corners = []
        for depth in (start, end):
            for y in (-1, 1):
                for x in (-1, 1):
                    v = (x * vertical * aspect * depth * (1 + guard[0]), y * vertical * depth * (1 + guard[1]), -depth)
                    corners.append(tuple(CAMERA_AT[k] + right[k] * v[0] + up[k] * v[1] + back[k] * v[2]
                                         for k in range(3)))
        centre = tuple(sum(c[k] for c in corners) / 8 for k in range(3))
        radius = max(math.dist(c, centre) for c in corners)
        quantized = math.ceil(radius * 16) / 16 * MAP_SIZE / (MAP_SIZE - 4)
        texel = 2 * quantized / MAP_SIZE
        cx, cy = dot(s, centre) / texel, dot(u, centre) / texel
        corner_z = [-dot(forward, c) for c in corners]
        lo, hi = min(min(caster), min(corner_z)), max(max(caster), max(corner_z))
        rows.append({"start": start, "end": end, "radius": radius, "radius_x16_fraction": radius * 16 % 1,
                     "quantized_radius": quantized, "texel": texel,
                     "snapped_centre": [round(cx) * texel, round(cy) * texel],
                     "centre_fraction_from_half": [abs(cx % 1 - 0.5), abs(cy % 1 - 0.5)],
                     "near_z": -hi - max(texel, 1.0), "far_z": -lo + max(texel, 1.0),
                     "depth_range_from_frustum": [min(corner_z) < min(caster), max(corner_z) > max(caster)]})
    return rows


def main():
    off = fit((0.0, 0.0))
    on = fit((f32(1 / 960), f32(1 / 540)))
    b = boundaries()
    out = {"cascade_boundaries": b,
           "blend_starts": [e - 0.1 * (e - s) for s, e in zip(b[:-1], b[1:])],
           "cascades": []}
    for i, (a, g) in enumerate(zip(off, on)):
        out["cascades"].append({
            "cascade": i, "view_depth": [a["start"], a["end"]],
            "texel_unguarded": a["texel"], "texel_guarded": g["texel"],
            "texel_changes": a["texel"] != g["texel"],
            "centre_changes": a["snapped_centre"] != g["snapped_centre"],
            "depth_range_changes": (a["near_z"], a["far_z"]) != (g["near_z"], g["far_z"]),
            "depth_range_set_by_frustum_corners": {"unguarded": a["depth_range_from_frustum"],
                                                   "guarded": g["depth_range_from_frustum"]},
            "near_far_unguarded": [a["near_z"], a["far_z"]], "near_far_guarded": [g["near_z"], g["far_z"]],
            "radius_x16_fraction": [a["radius_x16_fraction"], g["radius_x16_fraction"]],
            "centre_fraction_from_half": [a["centre_fraction_from_half"], g["centre_fraction_from_half"]]})
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
