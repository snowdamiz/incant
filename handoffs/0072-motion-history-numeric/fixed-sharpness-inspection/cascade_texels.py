"""Reviewer derivation, CPU only, standard library: the directional cascade extent that
crates/incant_render/src/shadows/cascade.rs::fit (a143b5c0) gives the review stage with
and without the temporal receiver guard (temporal/jitter.rs::guard = 1/width, 1/height),
for the full 960x540 output and the [37, 19, 885, 501] viewport; and where the
grille card's shadow falls (view depth, cascade, viewport pixels) at the still pose and
the last pan pose. Constants are copied from the frozen fixture
(visual/review/temporal_visual_review.rs). A derivation, not a measurement.

  python3 -I -B cascade_texels.py
"""
import math

MAP_SIZE, CASCADES = 1024, 4
FOV, NEAR, FAR, SHADOW_DISTANCE = math.radians(50.0), 0.1, 120.0, 60.0
CAMERA_AT, PITCH = (0.4, 4.2, 8.5), -0.42
CARD_AT, CARD_HALF = (2.4, 1.15, -0.6), 1.0  # 2x2 quad facing +Z, scale 1
SUN_ELEVATION, SUN_AZIMUTH = 38.0, 240.0
VIEWPORT = (37.0, 19.0, 885.0, 501.0)


def boundaries():
    near, far = NEAR, min(FAR, SHADOW_DISTANCE)
    b = [near] * (CASCADES + 1)
    for i in range(1, CASCADES + 1):
        t = i / CASCADES
        b[i] = 0.5 * (near * (far / near) ** t + near + (far - near) * t)
    b[CASCADES] = far
    return b


def extents(aspect, guard):
    """Per cascade: (start, end, sphere radius, quantized radius, world texel)."""
    b = boundaries()
    vertical = math.tan(FOV * 0.5)
    rows = []
    for i in range(CASCADES):
        start = b[0] if i == 0 else b[i] - 0.1 * (b[i] - b[i - 1])
        end = b[i + 1]
        corners = [(x * vertical * aspect * d * (1 + guard[0]), y * vertical * d * (1 + guard[1]), -d)
                   for d in (start, end) for y in (-1, 1) for x in (-1, 1)]
        centre = [sum(c[k] for c in corners) / 8 for k in range(3)]
        radius = max(math.dist(c, centre) for c in corners)
        quantized = math.ceil(radius * 16) / 16 * MAP_SIZE / (MAP_SIZE - 4)
        rows.append((start, end, radius, quantized, 2 * quantized / MAP_SIZE))
    return rows


def rot_x(a, v):
    c, s = math.cos(a), math.sin(a)
    return (v[0], c * v[1] - s * v[2], s * v[1] + c * v[2])


def rot_y(a, v):
    c, s = math.cos(a), math.sin(a)
    return (c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2])


def camera(step):
    """camera_world(step): Ry(-0.012 k) * Rx(pitch), translated 0.12 k along x."""
    position = (CAMERA_AT[0] + 0.12 * step, CAMERA_AT[1], CAMERA_AT[2])
    basis = [rot_y(-0.012 * step, rot_x(PITCH, axis)) for axis in ((1, 0, 0), (0, 1, 0), (0, 0, 1))]
    return position, basis


def project(point, step):
    position, (right, up, back) = camera(step)
    d = [point[k] - position[k] for k in range(3)]
    x, y, z = (sum(d[k] * axis[k] for k in range(3)) for axis in (right, up, back))
    depth = -z
    aspect = VIEWPORT[2] / VIEWPORT[3]
    ndc = (x / (depth * math.tan(FOV / 2) * aspect), y / (depth * math.tan(FOV / 2)))
    return depth, (VIEWPORT[0] + (ndc[0] + 1) / 2 * VIEWPORT[2], VIEWPORT[1] + (1 - ndc[1]) / 2 * VIEWPORT[3])


def main():
    print("Cascade boundaries (view depth):", " ".join(f"{v:.3f}" for v in boundaries()))
    for label, (w, h) in (("full 960x540", (960.0, 540.0)), ("viewport 885x501", (885.0, 501.0))):
        off, on = extents(w / h, (0.0, 0.0)), extents(w / h, (1 / w, 1 / h))
        print(f"== {label}, aspect {w / h:.5f}")
        for i, (a, b) in enumerate(zip(off, on)):
            flag = "texel CHANGES" if a[4] != b[4] else "same texel"
            print(f"  cascade {i} [{a[0]:.3f}, {a[1]:.3f}]: radius off {a[2]:.4f} -> {a[3]:.4f}, "
                  f"guarded {b[2]:.4f} -> {b[3]:.4f}; texel off {a[4]:.6f} on {b[4]:.6f} "
                  f"({100 * (b[4] / a[4] - 1):+.3f}%) {flag}")
    # Sun: Ry(azimuth) * Rx(-elevation) applied to the light's -Z.
    sun = rot_y(math.radians(SUN_AZIMUTH), rot_x(-math.radians(SUN_ELEVATION), (0.0, 0.0, -1.0)))
    print(f"sun travel direction {tuple(round(v, 4) for v in sun)}; grille texel "
          f"{2 * CARD_HALF / 128:.6f} world units")
    card = [(CARD_AT[0] + sx * CARD_HALF, CARD_AT[1] + sy * CARD_HALF, CARD_AT[2]) for sx in (-1, 1) for sy in (-1, 1)]
    ground = [(p[0] - p[1] / sun[1] * sun[0], 0.0, p[2] - p[1] / sun[1] * sun[2]) for p in card]
    b = boundaries()
    for step, pose in ((0, "still pose"), (8, "last pan pose (f24)")):
        print(f"== shadow of the card on the ground (y = 0), {pose}")
        for p in ground:
            depth, (sx, sy) = project(p, step)
            cascade = max(i for i in range(CASCADES) if depth >= (b[0] if i == 0 else b[i] - 0.1 * (b[i] - b[i - 1])))
            print(f"  ground ({p[0]:+.3f}, {p[2]:+.3f}): view depth {depth:.3f} (cascade {cascade} or blend), "
                  f"viewport pixel ({sx:.1f}, {sy:.1f})")


if __name__ == "__main__":
    main()
