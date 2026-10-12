"""Small triangle-mesh toolkit for the Core Sample art generators.

A Mesh holds positions, normals, UVs and a triangle index list. Triangles are
oriented explicitly: every add call takes an outward reference direction and
flips the winding when needed, so generated surfaces are counter-clockwise from
outside (glTF front faces) without relying on construction order.
"""
import math


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def scale(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def length(a):
    return math.sqrt(dot(a, a))


def normalize(a):
    n = length(a)
    return (0.0, 1.0, 0.0) if n < 1e-12 else (a[0] / n, a[1] / n, a[2] / n)


def quat_axis(axis, degrees):
    axis = normalize(axis)
    half = math.radians(degrees) * 0.5
    s = math.sin(half)
    return (axis[0] * s, axis[1] * s, axis[2] * s, math.cos(half))


def quat_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return (aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
            aw * bw - ax * bx - ay * by - az * bz)


def quat_rotate(q, v):
    x, y, z, w = q
    u = (x, y, z)
    t = scale(cross(u, v), 2.0)
    return add(add(v, scale(t, w)), cross(u, t))


class Mesh:
    def __init__(self):
        self.positions, self.normals, self.uvs, self.indices = [], [], [], []

    def __len__(self):
        return len(self.positions)

    def vertex(self, position, normal=(0.0, 1.0, 0.0), uv=(0.0, 0.0)):
        self.positions.append(tuple(float(v) for v in position))
        self.normals.append(tuple(float(v) for v in normal))
        self.uvs.append((float(uv[0]), float(uv[1])))
        return len(self.positions) - 1

    def tri(self, a, b, c, outward=None):
        if outward is not None:
            p = self.positions
            n = cross(sub(p[b], p[a]), sub(p[c], p[a]))
            if dot(n, outward) < 0:
                b, c = c, b
        self.indices.extend((a, b, c))

    def quad(self, a, b, c, d, outward=None):
        """Quad a-b-c-d in order around its boundary."""
        self.tri(a, b, c, outward)
        self.tri(a, c, d, outward)

    def merge(self, other):
        base = len(self.positions)
        self.positions.extend(other.positions)
        self.normals.extend(other.normals)
        self.uvs.extend(other.uvs)
        self.indices.extend(i + base for i in other.indices)
        return self

    def transformed(self, translation=(0, 0, 0), rotation=(0, 0, 0, 1), scale_xyz=(1, 1, 1)):
        out = Mesh()
        for p, n, uv in zip(self.positions, self.normals, self.uvs):
            q = quat_rotate(rotation, (p[0] * scale_xyz[0], p[1] * scale_xyz[1], p[2] * scale_xyz[2]))
            m = quat_rotate(rotation, normalize((n[0] / scale_xyz[0], n[1] / scale_xyz[1], n[2] / scale_xyz[2])))
            out.vertex(add(q, translation), m, uv)
        out.indices = list(self.indices)
        return out

    def smooth_normals(self, weld=None):
        """Area-weighted vertex normals. With weld (metres), coincident vertices share them."""
        acc = [(0.0, 0.0, 0.0)] * len(self.positions)
        keys = None
        if weld:
            keys = [tuple(round(c / weld) for c in p) for p in self.positions]
            groups = {}
        for i in range(0, len(self.indices), 3):
            a, b, c = self.indices[i:i + 3]
            p = self.positions
            n = cross(sub(p[b], p[a]), sub(p[c], p[a]))
            for v in (a, b, c):
                acc[v] = add(acc[v], n)
        if keys is not None:
            for v, k in enumerate(keys):
                groups[k] = add(groups.get(k, (0.0, 0.0, 0.0)), acc[v])
            acc = [groups[k] for k in keys]
        self.normals = [normalize(n) for n in acc]
        return self

    def flat(self):
        """Duplicate vertices per triangle and assign face normals (faceted look)."""
        out = Mesh()
        for i in range(0, len(self.indices), 3):
            a, b, c = self.indices[i:i + 3]
            p = self.positions
            n = normalize(cross(sub(p[b], p[a]), sub(p[c], p[a])))
            for v in (a, b, c):
                out.vertex(p[v], n, self.uvs[v])
            out.indices.extend((len(out) - 3, len(out) - 2, len(out) - 1))
        return out

    def bounds(self):
        xs, ys, zs = zip(*self.positions)
        return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))

    def triangles(self):
        return len(self.indices) // 3


def lathe(profile, segments, radial=None, u_scale=1.0, v_mode='height', v_scale=1.0,
          cap_top=False, cap_bottom=False, phase=0.0, arc=math.tau):
    """Revolve [(radius, y)] around +Y. radial(angle, y, index) may perturb the radius.

    UV: u = turn fraction * u_scale; v = y * v_scale (v_mode 'height') or cumulative
    profile length * v_scale ('length'). The seam duplicates the first column.
    """
    mesh = Mesh()
    rings = []
    run = 0.0
    for j, (r, y) in enumerate(profile):
        if j:
            run += math.hypot(r - profile[j - 1][0], y - profile[j - 1][1])
        ring = []
        for i in range(segments + 1):
            t = i / segments
            angle = phase + t * arc
            rr = r + (radial(angle % math.tau, y, j) if radial else 0.0)
            p = (rr * math.cos(angle), y, rr * math.sin(angle))
            v = y * v_scale if v_mode == 'height' else run * v_scale
            ring.append(mesh.vertex(p, (math.cos(angle), 0.0, math.sin(angle)), (t * u_scale, v)))
        rings.append(ring)
    for j in range(len(rings) - 1):
        for i in range(segments):
            a, b = rings[j][i], rings[j][i + 1]
            c, d = rings[j + 1][i + 1], rings[j + 1][i]
            centre = scale(add(mesh.positions[a], mesh.positions[c]), 0.5)
            out = (centre[0], 0.0, centre[2])
            if length(out) < 1e-9:
                out = (0.0, 1.0 if j else -1.0, 0.0)
            mesh.quad(a, b, c, d, out)
    mesh.smooth_normals(weld=1e-5)
    for ring, sign in ((rings[-1], 1.0) if cap_top else (None, 0), (rings[0], -1.0) if cap_bottom else (None, 0)):
        if ring is None:
            continue
        ys = [mesh.positions[k][1] for k in ring]
        centre = (0.0, sum(ys) / len(ys), 0.0)
        c = mesh.vertex(centre, (0, sign, 0), (0.5, 0.5))
        rim = []
        for k in ring[:-1]:
            p = mesh.positions[k]
            rim.append(mesh.vertex(p, (0, sign, 0), (0.5 + p[0] * 0.5, 0.5 + p[2] * 0.5)))
        for i in range(len(rim)):
            mesh.tri(c, rim[i], rim[(i + 1) % len(rim)], (0, sign, 0))
    return mesh


def box(half, uv_scale=1.0, centre=(0, 0, 0)):
    """Axis-aligned box with hard edges; UVs are planar per face in metres * uv_scale."""
    mesh = Mesh()
    hx, hy, hz = half
    faces = [((1, 0, 0), (0, 0, -1), (0, 1, 0)), ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
             ((0, 1, 0), (1, 0, 0), (0, 0, -1)), ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
             ((0, 0, 1), (1, 0, 0), (0, 1, 0)), ((0, 0, -1), (-1, 0, 0), (0, 1, 0))]
    for n, u, v in faces:
        corners = []
        for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            p = tuple(centre[k] + n[k] * half[k] + u[k] * su * half[k] + v[k] * sv * half[k] for k in range(3))
            uu = sum(p[k] * u[k] for k in range(3)) * uv_scale
            vv = sum(p[k] * v[k] for k in range(3)) * uv_scale
            corners.append(mesh.vertex(p, n, (uu, -vv)))
        mesh.quad(*corners, outward=n)
    return mesh


def icosphere(subdivisions):
    t = (1.0 + math.sqrt(5.0)) / 2.0
    verts = [normalize(v) for v in [(-1, t, 0), (1, t, 0), (-1, -t, 0), (1, -t, 0), (0, -1, t), (0, 1, t),
                                    (0, -1, -t), (0, 1, -t), (t, 0, -1), (t, 0, 1), (-t, 0, -1), (-t, 0, 1)]]
    faces = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4), (11, 10, 2),
             (10, 7, 6), (7, 1, 8), (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9), (4, 9, 5),
             (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    for _ in range(subdivisions):
        cache, next_faces = {}, []

        def middle(a, b):
            key = (min(a, b), max(a, b))
            if key not in cache:
                verts.append(normalize(scale(add(verts[a], verts[b]), 0.5)))
                cache[key] = len(verts) - 1
            return cache[key]
        for a, b, c in faces:
            ab, bc, ca = middle(a, b), middle(b, c), middle(c, a)
            next_faces += [(a, ab, ca), (b, bc, ab), (c, ca, bc), (ab, bc, ca)]
        faces = next_faces
    return verts, faces


def rock(radius, squash=(1.0, 1.0, 1.0), roughness=0.25, seed=0, subdivisions=2, displace=None,
         uv_scale=0.6):
    """Displaced icosphere boulder with flat-ish facets; planar UVs from the dominant axis."""
    from . import noise
    verts, faces = icosphere(subdivisions)
    moved = []
    for v in verts:
        n = noise.fbm3(v[0] * 1.7 + seed * 3.1, v[1] * 1.7, v[2] * 1.7, seed=seed, octaves=3)
        r = radius * (1.0 + roughness * n)
        p = (v[0] * r * squash[0], v[1] * r * squash[1], v[2] * r * squash[2])
        if displace:
            p = displace(p)
        moved.append(p)
    mesh = Mesh()
    for a, b, c in faces:
        pa, pb, pc = moved[a], moved[b], moved[c]
        n = normalize(cross(sub(pb, pa), sub(pc, pa)))
        axis = max(range(3), key=lambda k: abs(n[k]))
        idx = []
        for p in (pa, pb, pc):
            if axis == 0:
                uv = (p[2], p[1])
            elif axis == 1:
                uv = (p[0], p[2])
            else:
                uv = (p[0], p[1])
            idx.append(mesh.vertex(p, n, (uv[0] * uv_scale, uv[1] * uv_scale)))
        mesh.tri(*idx, outward=n)
    return mesh


def polygon_area(points):
    return 0.5 * sum(points[i][0] * points[(i + 1) % len(points)][1] - points[(i + 1) % len(points)][0] * points[i][1]
                     for i in range(len(points)))


def triangulate(points):
    """Ear clipping for a simple polygon [(x, z)]; returns index triples (CCW in x,z)."""
    n = len(points)
    order = list(range(n))
    if polygon_area(points) < 0:
        order.reverse()

    def inside(p, a, b, c):
        def s(p1, p2, p3):
            return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
        d1, d2, d3 = s(p, a, b), s(p, b, c), s(p, c, a)
        return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))
    result, guard = [], 0
    while len(order) > 3 and guard < 10000:
        guard += 1
        for k in range(len(order)):
            i0, i1, i2 = order[k - 1], order[k], order[(k + 1) % len(order)]
            a, b, c = points[i0], points[i1], points[i2]
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 1e-12:
                continue
            if any(inside(points[j], a, b, c) for j in order if j not in (i0, i1, i2)):
                continue
            result.append((i0, i1, i2))
            order.pop(k)
            break
        else:
            raise ValueError('polygon triangulation failed')
    result.append(tuple(order))
    return result


def rounded_box(half, seed=0, roughness=0.0, exponent=0.3, divisions=6, uv_scale=0.6):
    """Superellipsoid-like slab: a subdivided cube pushed toward a rounded box, then
    displaced along its normals by noise. UVs are planar per face (dominant axis)."""
    from . import noise
    mesh = Mesh()
    faces = [((1, 0, 0), (0, 0, -1), (0, 1, 0)), ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
             ((0, 1, 0), (1, 0, 0), (0, 0, -1)), ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
             ((0, 0, 1), (1, 0, 0), (0, 1, 0)), ((0, 0, -1), (-1, 0, 0), (0, 1, 0))]
    for n, u, v in faces:
        grid = []
        for i in range(divisions + 1):
            row = []
            for j in range(divisions + 1):
                a, b = -1 + 2 * i / divisions, -1 + 2 * j / divisions
                c = tuple(n[k] + u[k] * a + v[k] * b for k in range(3))
                # Round toward the inscribed superellipsoid.
                m = max(abs(c[0]), abs(c[1]), abs(c[2]))
                d = normalize(c)
                s = tuple(math.copysign(abs(d[k]) ** exponent, d[k]) for k in range(3))
                q = tuple(c[k] / m * 0.35 + s[k] * 0.65 for k in range(3))
                p = [q[k] * half[k] for k in range(3)]
                if roughness:
                    w = noise.fbm3(q[0] * 2.1, q[1] * 2.1, q[2] * 2.1, seed, 3)
                    p = [p[k] + d[k] * roughness * w * min(half) for k in range(3)]
                axis = max(range(3), key=lambda k: abs(n[k]))
                uv = (p[2], p[1]) if axis == 0 else (p[0], p[2]) if axis == 1 else (p[0], p[1])
                row.append(mesh.vertex(tuple(p), n, (uv[0] * uv_scale, uv[1] * uv_scale)))
            grid.append(row)
        for i in range(divisions):
            for j in range(divisions):
                mesh.quad(grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1], n)
    # Faces do not share vertices; weld positions for smooth normals.
    return mesh.smooth_normals(weld=1e-4)


def cylinder(radius, y0, y1, segments=12, caps=True, radius_top=None):
    top = radius if radius_top is None else radius_top
    return lathe([(radius, y0), (top, y1)], segments, cap_top=caps, cap_bottom=caps)


def beam(a, b, width, depth=None):
    """Square-section box from point a to point b (struts, legs, braces)."""
    depth = width if depth is None else depth
    axis = sub(b, a)
    length_ab = length(axis)
    y = normalize(axis)
    helper = (0.0, 0.0, 1.0) if abs(y[2]) < 0.9 else (1.0, 0.0, 0.0)
    x = normalize(cross(y, helper))
    z = cross(x, y)
    unit_box = box((width / 2, length_ab / 2, depth / 2), uv_scale=1.0)
    out = Mesh()
    centre = scale(add(a, b), 0.5)
    for p, n, uv in zip(unit_box.positions, unit_box.normals, unit_box.uvs):
        q = add(centre, add(add(scale(x, p[0]), scale(y, p[1])), scale(z, p[2])))
        m = add(add(scale(x, n[0]), scale(y, n[1])), scale(z, n[2]))
        out.vertex(q, m, uv)
    out.indices = list(unit_box.indices)
    # Re-orient: the basis may be left-handed.
    fixed = Mesh()
    fixed.positions, fixed.normals, fixed.uvs = out.positions, out.normals, out.uvs
    for i in range(0, len(out.indices), 3):
        a_, b_, c_ = out.indices[i:i + 3]
        fixed.tri(a_, b_, c_, out.normals[a_])
    return fixed
