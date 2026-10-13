"""CPU bound for SSAO self-occlusion on the composed-scene sphere (design aid).

Mirrors tools/effects/src/gltf.rs `sphere(24, 48)` (radius 0.5, smooth analytic
vertex normals, zero-area pole triangles skipped) under the composed.rs entity
scale 1.2. For a receiver point p with interpolated shading normal n, ao.wgsl
counts a surface point S only if dot(S - p, n) > bias. The largest such rise over
every mesh point S is a linear maximum over the convex hull, so it is attained at
a vertex. The mesh is invariant under a 7.5 degree rotation about +Y, so one
sector column covers every triangle. Prints JSON; stdlib only, deterministic.
Python f64 replaces the engine's f32 vertex math (differences near 1e-7).
"""

import json
import math
import sys

RINGS, SECTORS, SCALE, BIAS = 24, 48, 1.2, 0.01


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(a):
    length = math.sqrt(dot(a, a))
    return (a[0] / length, a[1] / length, a[2] / length)


def mesh():
    positions, normals = [], []
    for y in range(RINGS + 1):
        theta = math.pi * y / RINGS
        for x in range(SECTORS + 1):
            phi = math.tau * x / SECTORS
            n = (math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi))
            positions.append(tuple(v * 0.5 for v in n))
            normals.append(n)
    row = SECTORS + 1
    triangles = []
    for y in range(RINGS):
        for x in range(SECTORS):
            a = y * row + x
            b = a + row
            for tri in ((a, a + 1, b), (a + 1, b + 1, b)):
                p = [positions[i] for i in tri]
                g = cross(sub(p[1], p[0]), sub(p[2], p[0]))
                if dot(g, g) < 1e-12:  # same radius-0.5 test as gltf.rs
                    continue
                triangles.append((x, tri))
    return [tuple(v * SCALE for v in p) for p in positions], normals, triangles


def column_bound(positions, normals, triangles, n):
    """Largest rise and shading/face angle over sector-0 triangles, n-step barycentrics."""
    cos25 = math.cos(math.radians(25))
    radius = 0.5 * SCALE
    rise, angle = -math.inf, 0.0
    for x, tri in triangles:
        if x != 0:
            continue
        p3 = [positions[i] for i in tri]
        n3 = [normals[i] for i in tri]
        face = unit(cross(sub(p3[1], p3[0]), sub(p3[2], p3[0])))
        if dot(face, p3[0]) < 0:
            face = (-face[0], -face[1], -face[2])
        centre = unit(tuple(sum(p[k] for p in p3) for k in range(3)))
        # Exact candidate set: the maximiser of dot(V, n) is the vertex nearest n
        # in angle (under 12 degrees here); vertices beyond 25 degrees cannot win.
        candidates = [v for v in positions if dot(v, centre) >= radius * cos25]
        for i in range(n + 1):
            for j in range(n + 1 - i):
                w = (i / n, j / n, (n - i - j) / n)
                p = tuple(sum(w[m] * p3[m][k] for m in range(3)) for k in range(3))
                shading = unit(tuple(sum(w[m] * n3[m][k] for m in range(3)) for k in range(3)))
                angle = max(angle, math.degrees(math.acos(min(1.0, dot(shading, face)))))
                rise = max(rise, max(dot(v, shading) for v in candidates) - dot(p, shading))
    return rise, angle


def main():
    positions, normals, triangles = mesh()
    column = sum(1 for x, _ in triangles if x == 0)
    levels = {}
    for n in (16, 32, 64):
        rise, angle = column_bound(positions, normals, triangles, n)
        levels[str(n)] = {"max_rise": round(rise, 7), "max_shading_face_angle_deg": round(angle, 4)}
    finest = levels["64"]["max_rise"]
    report = {
        "mesh": "gltf::sphere(24, 48) x entity scale 1.2 (radius 0.6)",
        "triangles": len(triangles),
        "evaluated": f"{column} triangles of sector column 0 (7.5 degree rotational symmetry)",
        "barycentric_levels": levels,
        "min_ray_elevation_deg": round(math.degrees(math.asin(math.sqrt(1 - 15.5 / 16))), 4),
        "plane_bias": BIAS,
        "rise_below_bias": finest < BIAS,
    }
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")
    return 0 if report["rise_below_bias"] else 1


if __name__ == "__main__":
    sys.exit(main())
