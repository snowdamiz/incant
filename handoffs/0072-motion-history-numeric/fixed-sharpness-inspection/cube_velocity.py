"""Reviewer derivation, CPU only: screen position and per-frame velocity of the moving
cube's trailing (left) silhouette in the motion-object case, from the frozen fixture's
constants (camera at the still pose, 960x540, fov 50; mover centre MOVER_START +
0.35 k along x, half extent 0.5). The trailing silhouette is the cube's left face,
whose near edge (z = centre + 0.5) projects furthest left. A derivation, not a
measurement.

  python3 -I -B cube_velocity.py
"""
import math

CAMERA_AT, PITCH, FOV, W, H = (0.4, 4.2, 8.5), -0.42, math.radians(50.0), 960.0, 540.0
MOVER_START, MOVER_STEP, HALF = (-2.6, 1.0, -1.0), 0.35, 0.5


def project(p):
    d = [p[k] - CAMERA_AT[k] for k in range(3)]
    c, s = math.cos(PITCH), math.sin(PITCH)
    right, up, back = (1.0, 0.0, 0.0), (0.0, c, s), (0.0, -s, c)
    x, y, z = (sum(d[k] * a[k] for k in range(3)) for a in (right, up, back))
    t = math.tan(FOV / 2)
    return ((x / (-z * t * W / H)) + 1) / 2 * W, (1 - y / (-z * t)) / 2 * H


def main():
    for k in (6, 7, 8):
        cx = MOVER_START[0] + MOVER_STEP * k
        rows = []
        for y in (0.6, 1.0, 1.4):
            near = project((cx - HALF, y, MOVER_START[2] + HALF))
            far = project((cx - HALF, y, MOVER_START[2] - HALF))
            rows.append(f"y={y}: near-left edge x {near[0]:.3f} (row {near[1]:.1f}), far-left x {far[0]:.3f}")
        print(f"frame {k}: " + "; ".join(rows))
    for y in (0.6, 1.0, 1.4):
        a = project((MOVER_START[0] + MOVER_STEP * 7 - HALF, y, MOVER_START[2] + HALF))[0]
        b = project((MOVER_START[0] + MOVER_STEP * 8 - HALF, y, MOVER_START[2] + HALF))[0]
        print(f"trailing edge velocity f7->f8 at y={y}: {b - a:.3f} px/frame (fraction {(b - a) % 1:.3f})")


if __name__ == "__main__":
    main()
