"""Terrain: canyon floor, the L-shaped upper mesa, two sand ramps, the canyon walls
and far buttes beyond the north rim.

One sedimentary column (STRATA) is shared by every cliff, so each layer sits at
the same world height everywhere: the 1.6 m bench that forms the walkable upper
mesa is a pale resistant band, and the canyon walls continue the column up to a
grey-buff caprock. Cliff UVs are (arc length, world height), so strata never
stretch. Every surface stays on or behind its collision face, except the upper
mesa's 10 cm caprock lip, which overhangs the floor-side edge.
"""
import math

from . import level, noise as N, pngio, raster as R
from .mesh import Mesh, triangulate

# (top of band in metres, sRGB colour, hardness 0..1: hard bands stand proud)
STRATA = [
    (0.30, '#7A4535', 0.5), (0.48, '#B08367', 0.9), (1.25, '#A45D41', 0.0), (1.42, '#8D4A37', 0.3),
    (1.62, '#AF8466', 1.0), (2.35, '#AE6849', 0.1), (2.55, '#B99376', 0.9), (3.60, '#9C503A', 0.0),
    (3.85, '#AD7853', 0.7), (5.10, '#A7624A', 0.1), (5.32, '#B58F73', 0.9), (6.10, '#AC6347', 0.2),
    (9.00, '#A38A72', 1.0),
]
CLIFF_TILE = (8.0, 8.0)       # metres covered by one cliff texture (u, v)
TOP_TILE = 6.0                # metres per repeat of the mesa-top texture
FLOOR_RECT = (-12.5, 12.5, -20.9, 26.0)  # x0, x1, z0, z1 of the uniquely textured floor
LIP = 0.10
CAP_DEPTH = 9.0


def _band(y):
    for i, (top, colour, hard) in enumerate(STRATA):
        if y <= top:
            return i
    return len(STRATA) - 1


def hardness(y, soft=0.06):
    """Smoothed band hardness at height y."""
    total, weight = 0.0, 0.0
    for k in (-1.0, -0.5, 0.0, 0.5, 1.0):
        w = 1.0 - abs(k) * 0.6
        total += STRATA[_band(y + k * soft)][2] * w
        weight += w
    return total / weight


def strata_colour(y, blend=0.02):
    i = _band(y)
    colour = R.hex_linear(STRATA[i][1])
    below = STRATA[i - 1][0] if i else -1.0
    if i and y - below < blend:
        colour = N.mix3(R.hex_linear(STRATA[i - 1][1]), colour, 0.5 + 0.5 * (y - below) / blend)
    top = STRATA[i][0]
    if i + 1 < len(STRATA) and top - y < blend:
        colour = N.mix3(R.hex_linear(STRATA[i + 1][1]), colour, 0.5 + 0.5 * (top - y) / blend)
    return colour


# ---------------------------------------------------------------- textures

def _cells(x, y, seed):
    """Non-periodic Voronoi in unit cells: (nearest, second nearest) distances."""
    ix, iy = math.floor(x), math.floor(y)
    best, second = 9.0, 9.0
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            cx, cy = ix + dx, iy + dy
            d = math.hypot(cx + 0.15 + 0.7 * N.unit(seed, cx, cy) - x, cy + 0.15 + 0.7 * N.unit(seed + 1, cx, cy) - y)
            if d < best:
                best, second = d, best
            elif d < second:
                second = d
    return best, second


def _periodic_cells(u, v, cells, seed):
    """Periodic Voronoi over the unit square: (nearest, second, owner hash)."""
    x, y = u * cells, v * cells
    ix, iy = math.floor(x), math.floor(y)
    best, second, owner = 9.0, 9.0, 0
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            cx, cy = ix + dx, iy + dy
            wx, wy = cx % cells, cy % cells
            d = math.hypot(cx + 0.15 + 0.7 * N.unit(seed, wx, wy) - x, cy + 0.15 + 0.7 * N.unit(seed + 1, wx, wy) - y)
            if d < best:
                best, second, owner = d, best, N.hash32(seed, wx, wy)
            elif d < second:
                second = d
    return best, second, owner


def cliff_textures(width=1024, height=512, seed=101):
    """Albedo rows and a height grid (for the normal map) for every strata cliff.

    u spans 8 m horizontally (tiling), v spans 0..8 m of world height. Soft beds
    carry cross-bedding, hard beds carry vertical joints, varnish streaks hang
    below ledges, and mottling breaks up every band.
    """
    tu, tv = CLIFF_TILE
    wobble = R.field(64, 32, lambda u, v: N.fbm2(u * 4, v * 3, seed, 3, period=(4, 3)))
    streak = R.field(512, 16, lambda u, v: N.fbm2(u * 48, v * 2, seed + 1, 3, period=(48, 2)))
    mottle = R.field(128, 64, lambda u, v: N.fbm2(u * 8, v * 8, seed + 2, 4, period=(8, 8)))
    fine = R.field(256, 128, lambda u, v: N.fbm2(u * 32, v * 32, seed + 3, 3, period=(32, 32)))
    joints = R.field(512, 32, lambda u, v: N.fbm2(u * 64, v * 4, seed + 4, 2, period=(64, 4)))
    varnish_c = R.hex_linear('#4D2F27')
    albedo, heights = [], []
    for row in range(height):
        v = (row + 0.5) / height
        y_row = v * tv
        line, hline = [], []
        for col in range(width):
            u = (col + 0.5) / width
            y = y_row + 0.06 * R.sample(wobble, u, v)
            band = _band(y)
            hard = hardness(y)
            colour = strata_colour(y)
            m, f = R.sample(mottle, u, v), R.sample(fine, u, v)
            shade = 1.0 + 0.09 * m + 0.035 * f
            # Cross-bedding in soft beds: fine laminae at a per-bed angle.
            angle = (N.unit(seed + 5, band) - 0.5) * 0.7
            lam = math.sin((u * tu * math.sin(angle) + y * math.cos(angle)) * 70.0 + 3.0 * f)
            shade *= 1.0 + 0.03 * lam * (1.0 - hard)
            # Vertical joints in hard beds.
            j = R.sample(joints, u, v)
            joint = (1.0 - N.smoothstep(0.0, 0.035, abs(j))) * hard
            shade *= 1.0 - 0.28 * joint
            # Desert varnish hanging below the ledges.
            hang = max(0.0, hardness(y + 0.35) - hard * 0.6)
            s = N.smoothstep(0.12, 0.5, R.sample(streak, u, v))
            varnish = s * (0.25 + 0.75 * hang) * (0.6 + 0.4 * N.smoothstep(-0.2, 0.4, m))
            colour = N.mix3(colour, varnish_c, 0.42 * varnish)
            # Soft beds just below a hard ledge sit in its overhang shadow.
            shade *= 1.0 - 0.16 * max(0.0, hardness(y + 0.08) - hard)
            shade *= 0.985 + 0.03 * N.unit(seed, col, row)
            line.append((colour[0] * shade, colour[1] * shade, colour[2] * shade))
            hline.append(3.0 * hard + 0.9 * m + 0.35 * f + 0.2 * lam * (1.0 - hard) - 1.6 * joint)
        albedo.append(line)
        heights.append(hline)
    return albedo, heights


def top_texture(size=512, seed=211):
    """Tileable slickrock: smooth weathered sandstone with faint cross-bedding arcs,
    a few long low-contrast joints, and wind-blown sand in the hollows."""
    sand = R.field(64, 64, lambda u, v: N.fbm2(u * 3, v * 3, seed, 4, period=(3, 3)))
    mott = R.field(128, 128, lambda u, v: N.fbm2(u * 6, v * 6, seed + 1, 4, period=(6, 6)))
    warp = R.field(64, 64, lambda u, v: N.fbm2(u * 2, v * 2, seed + 2, 3, period=(2, 2)))
    joint_field = R.field(256, 256, lambda u, v: N.fbm2(u * 3, v * 3, seed + 3, 2, period=(3, 3)))
    rock_a, rock_b = R.hex_linear('#B9906F'), R.hex_linear('#A87C5E')
    line_c, sand_a, sand_b = R.hex_linear('#93684F'), R.hex_linear('#CE9F75'), R.hex_linear('#C4946A')
    image, heights = [], []
    for row in range(size):
        v = (row + 0.5) / size
        line, hline = [], []
        for col in range(size):
            u = (col + 0.5) / size
            m = R.sample(mott, u, v)
            rock = N.mix3(rock_a, rock_b, N.smoothstep(-0.4, 0.5, m))
            # Cross-bedding: thin arcs following a warped contour field.
            arc = math.sin((v + 0.35 * R.sample(warp, u, v)) * math.tau * 9.0)
            bedding = N.smoothstep(0.86, 0.98, arc)
            rock = N.mix3(rock, line_c, 0.35 * bedding)
            # Sparse joints: the zero contour of a smooth field, thin and soft.
            j = abs(R.sample(joint_field, u, v))
            joint = 1.0 - N.smoothstep(0.0, 0.012, j)
            rock = N.mix3(rock, line_c, 0.45 * joint)
            drift = N.smoothstep(-0.05, 0.3, R.sample(sand, u, v))
            sandc = N.mix3(sand_a, sand_b, 0.5 + 0.5 * m)
            colour = N.mix3(rock, sandc, drift)
            g = 0.99 + 0.02 * N.unit(seed + 5, col, row)
            line.append(tuple(c * g for c in colour))
            hline.append((1.0 - drift) * (0.5 + 0.3 * m - 0.4 * joint - 0.15 * bedding))
        image.append(line)
        heights.append(hline)
    return image, heights


def _segment_distance(px, pz, a, b):
    ax, az = a[0], a[1]
    bx, bz = b[0], b[1]
    dx, dz = bx - ax, bz - az
    t = max(0.0, min(1.0, ((px - ax) * dx + (pz - az) * dz) / (dx * dx + dz * dz)))
    return math.hypot(px - (ax + t * dx), pz - (az + t * dz))


def _polyline_distance(px, pz, points):
    return min(_segment_distance(px, pz, points[i], points[i + 1]) for i in range(len(points) - 1))


WASH = [(-0.4, -12.6), (-1.4, -9.0), (-0.2, -5.6), (-2.2, -1.4), (-3.6, 2.4), (-2.4, 6.2), (-4.2, 10.4),
        (-5.0, 14.0), (-3.8, 18.5), (-4.8, 26.0)]


def floor_texture(size=1024, seed=307):
    x0, x1, z0, z1 = FLOOR_RECT
    big = R.field(96, 96, lambda u, v: N.fbm2(u * 5, v * 9, seed, 4))
    mid = R.field(256, 256, lambda u, v: N.fbm2(u * 22, v * 40, seed + 1, 3))
    warp = R.field(128, 128, lambda u, v: N.fbm2(u * 9, v * 16, seed + 2, 2))
    clumps = R.field(256, 256, lambda u, v: N.fbm2(u * 40, v * 76, seed + 3, 2))
    edges = [((-12.5, -12.5), (5.0, -12.5)), ((5.0, -12.5), (5.0, -2.5)), ((5.0, -2.5), (12.5, -2.5))]
    walls = [[(x, z) for x, z, _ in w] for w in level.WALLS.values()]

    def world(u, v):
        return x0 + u * (x1 - x0), z0 + v * (z1 - z0)

    def near(u, v):
        x, z = world(u, v)
        d = min(_segment_distance(x, z, a, b) for a, b in edges)
        return min(d, min(_polyline_distance(x, z, p) for p in walls))

    def contact(u, v):
        x, z = world(u, v)
        ao = 1.0 - 0.16 * (1.0 - N.smoothstep(0.0, 1.0, near(u, v)))
        for h in level.HOODOOS:
            if h['at'][1] == level.FLOOR_Y:
                d = max(0.0, math.hypot(x - h['at'][0], z - h['at'][2]) - h['radius'])
                ao *= 1.0 - 0.2 * (1.0 - N.smoothstep(0.0, 0.7, d))
        for b in level.BOULDERS:
            if b['at'][1] == level.FLOOR_Y:
                d = max(0.0, math.hypot(x - b['at'][0], z - b['at'][2]) - b['radius'] * 0.9)
                ao *= 1.0 - 0.18 * (1.0 - N.smoothstep(0.0, 0.45, d))
        return ao
    ao_grid = R.field(256, 256, contact)
    near_grid = R.field(256, 256, near)
    wash_grid = R.field(256, 256, lambda u, v: _polyline_distance(*world(u, v), WASH))
    sand_a, sand_b, sand_c = R.hex_linear('#CC9F76'), R.hex_linear('#C1916A'), R.hex_linear('#D6AC84')
    playa, crack_c = R.hex_linear('#D3B08C'), R.hex_linear('#9C7357')
    pebble = [R.hex_linear(c) for c in ('#9C6E55', '#8A5F4A', '#BE9573', '#D8BE9E')]
    image = []
    for row in range(size):
        v = (row + 0.5) / size
        line = []
        for col in range(size):
            u = (col + 0.5) / size
            x, z = world(u, v)
            b = R.sample(big, u, v, False, False)
            m = R.sample(mid, u, v, False, False)
            colour = N.mix3(sand_a, sand_b, N.smoothstep(-0.35, 0.45, b))
            colour = N.mix3(colour, sand_c, 0.4 * N.smoothstep(0.05, 0.55, m))
            w = R.sample(warp, u, v, False, False)
            ripple = math.sin(x * 6.5 + z * 1.4 + w * 7.0)
            shade = 1.0 + 0.022 * ripple * N.smoothstep(-0.25, 0.35, b)
            wash = R.sample(wash_grid, u, v, False, False) + 0.3 * m
            wmask = 1.0 - N.smoothstep(0.55, 1.25, wash)
            if wmask > 0.0:
                d1, d2 = _cells(x / 0.38, z / 0.38, seed + 4)
                crack = 1.0 - N.smoothstep(0.0, 0.05, d2 - d1)
                mud = N.mix3(playa, crack_c, crack * 0.75)
                colour = N.mix3(colour, mud, wmask * 0.8)
            nearness = 1.0 - N.smoothstep(0.1, 1.2, R.sample(near_grid, u, v, False, False))
            clump = N.smoothstep(0.15, 0.55, R.sample(clumps, u, v, False, False))
            density = 0.002 + 0.06 * nearness + 0.02 * clump
            cell_x, cell_y = col // 2, row // 2
            if N.unit(seed + 7, cell_x, cell_y) < density:
                colour = pebble[N.hash32(seed + 8, cell_x, cell_y) % 4]
                shade *= 0.9 + 0.15 * N.unit(seed + 9, cell_x, cell_y)
            shade *= 0.99 + 0.02 * N.unit(seed + 10, col, row)
            shade *= R.sample(ao_grid, u, v, False, False)
            line.append(tuple(c * shade for c in colour))
        image.append(line)
    return image


# ---------------------------------------------------------------- geometry

def _bisector(a, b):
    nx, nz = a[0] + b[0], a[1] + b[1]
    length = math.hypot(nx, nz)
    nx, nz = nx / length, nz / length
    cos_half = nx * b[0] + nz * b[1]
    return nx / max(cos_half, 0.5), nz / max(cos_half, 0.5)   # miter


def _resample(points, spacing):
    """Columns along a polyline: (x, z, arc length, nx, nz, t) with right-hand normals.

    Interior vertices use the mitred bisector normal so neighbouring segments share
    a seam; a closed polyline (first point repeated last) also closes its seam.
    `t` is the fractional vertex index, for interpolating per-vertex values.
    """
    closed = tuple(points[0][:2]) == tuple(points[-1][:2])
    seg_normals = []
    for a, b in zip(points, points[1:]):
        dx, dz = b[0] - a[0], b[1] - a[1]
        n = math.hypot(dx, dz)
        seg_normals.append((-dz / n, dx / n))
    cols, run = [], 0.0
    for i, (a, b) in enumerate(zip(points, points[1:])):
        seg = math.hypot(b[0] - a[0], b[1] - a[1])
        steps = max(1, round(seg / spacing))
        for k in range(steps + (1 if i == len(points) - 2 else 0)):
            t = k / steps
            if k == 0 and i > 0:
                nx, nz = _bisector(seg_normals[i - 1], seg_normals[i])
            elif closed and ((k == 0 and i == 0) or (k == steps and i == len(points) - 2)):
                nx, nz = _bisector(seg_normals[-1], seg_normals[0])
            else:
                nx, nz = seg_normals[i]
            cols.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, run + seg * t, nx, nz, i + t))
        run += seg
    return cols


def cliff_strip(points, y0, top, seed, lip=False, spacing=0.32, step=0.17, crest=0.0, deep=0.0, talus=None):
    """Strata face looking right of the polyline's direction of travel.

    `top` is a constant or a function of the fractional vertex index; `crest`
    adds a ragged skyline (metres of noise on the top edge). Returns the mesh,
    its columns and the top (rim) vertex position of every column.
    """
    cols = _resample(points, spacing)
    top_at = top if callable(top) else (lambda t: top)
    tops = [top_at(c[5]) + crest * N.fbm2(c[2] * 0.45, 0.0, seed + 3, 3) for c in cols]
    peak = max(tops)
    rows = []
    y = y0 - 0.08
    while y < peak - 0.24:
        rows.append(y)
        y += step
    count = len(rows) + 4
    mesh = Mesh()
    grid, rims = [], []
    for (x, z, run, nx, nz, _), y1 in zip(cols, tops):
        heights = [min(r, y1 - 0.24) for r in rows] + [y1 - 0.2, y1 - 0.11, y1 - 0.05, y1]
        line = []
        buttress = 0.5 + 0.5 * N.fbm2(run * 0.11, 0.5, seed + 4, 3)
        for j, y in enumerate(heights):
            base = -0.1 - 0.16 * (0.5 + 0.5 * N.fbm2(run * 0.2, 0.0, seed, 2))
            relief = -(0.2 + 0.12 * deep) * (1.0 - hardness(y))
            jitter = 0.07 * N.fbm2(run * 1.1, y * 2.4, seed + 1, 3)
            groove = -(0.16 + 0.22 * deep) * max(0.0, N.perlin2(run * 0.8, y * 0.2, seed + 2)) ** 2 * 4.0
            bay = -0.55 * deep * buttress * N.smoothstep(0.3, 2.2, y - y0)
            d = min(-0.02, base + relief + jitter + groove + bay)
            if lip:
                # Flush top edge (meets the walkable top), then a rounded caprock nose.
                if j >= len(heights) - 3:
                    d = (0.05, 0.14, 0.0)[j - len(heights) + 3]
            elif y > y1 - 0.25:
                d = d + (-0.03 - d) * N.smoothstep(y1 - 0.25, y1 - 0.04, y)
            if y < y0 + 0.1:
                d = min(d, -0.06)
            if talus:
                # (height where the cliff gives way to scree, run per metre of drop)
                d += talus[1] * max(0.0, talus[0] - y) ** 1.15
            line.append(mesh.vertex((x + nx * d, y, z + nz * d), (nx, 0.0, nz),
                                    (run / CLIFF_TILE[0], y / CLIFF_TILE[1])))
        assert len(line) == count
        grid.append(line)
        rims.append(mesh.positions[line[-1]])
    for i in range(len(cols) - 1):
        nx, nz = cols[i][3], cols[i][4]
        for j in range(count - 1):
            a, b, c, d = grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]
            mesh.quad(a, b, c, d, (nx, 0.0, nz))
    mesh.smooth_normals(weld=1e-4)
    return mesh, cols, rims


def flat_polygon(points, y, tile, up=True):
    mesh = Mesh()
    idx = [mesh.vertex((x, y, z), (0, 1 if up else -1, 0), (x / tile, z / tile)) for x, z in points]
    for a, b, c in triangulate(points):
        mesh.tri(idx[a], idx[b], idx[c], (0, 1 if up else -1, 0))
    return mesh


def floor_uv(x, z):
    x0, x1, z0, z1 = FLOOR_RECT
    return (x - x0) / (x1 - x0), (z - z0) / (z1 - z0)


def floor_mesh(step=1.0):
    x0, x1, z0, z1 = FLOOR_RECT
    nx, nz = round((x1 - x0) / step), round((z1 - z0) / step)
    mesh = Mesh()
    grid = []
    for i in range(nx + 1):
        row = []
        for j in range(nz + 1):
            x, z = x0 + (x1 - x0) * i / nx, z0 + (z1 - z0) * j / nz
            row.append(mesh.vertex((x, level.FLOOR_Y, z), (0, 1, 0), floor_uv(x, z)))
        grid.append(row)
    for i in range(nx):
        for j in range(nz):
            mesh.quad(grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1], (0, 1, 0))
    return mesh


def bench_outline():
    """Floor-side edges of the upper mesa, floor on the right of travel, split at ramp heads."""
    e, w, c = level.RAMPS['east'], level.RAMPS['west'], level.RAMPS['centre']
    zp, zs, xw = level.NORTH_PLATEAU['z'][1], level.EAST_SHELF['z'][1], level.EAST_SHELF['x'][0]
    p1, p2, p3 = level.PROMONTORIES
    west = [(w['x'][1], zp), (p1['x'][0], zp), (p1['x'][0], p1['z'][1]), (p1['x'][1], p1['z'][1]),
            (p1['x'][1], zp)]
    middle = [(c['x'][1], zp), (p2['x'][0], zp), (p2['x'][0], p2['z'][1]), (p2['x'][1], p2['z'][1]),
              (p2['x'][1], zp), (xw, zp), (xw, p3['z'][0]), (p3['x'][0], p3['z'][0]), (p3['x'][0], p3['z'][1]),
              (xw, p3['z'][1]), (xw, zs), (e['x'][0], zs)]
    return [west, middle, [(e['x'][1], zs), (12.4, zs)]]


def upper_top():
    west, middle, east = bench_outline()
    rim = level.RIM_Z
    zp = level.NORTH_PLATEAU['z'][1]
    # Around the mesa top: rim, east wall side, then the bench outline back to the
    # west wall; the straight gaps between pieces are the three ramp heads.
    points = [(-12.6, rim), (12.6, rim), (12.6, east[1][1])] + list(reversed(east)) + \
        list(reversed(middle)) + list(reversed(west)) + [(-12.6, zp)]
    return flat_polygon(points, level.UPPER_Y, TOP_TILE)


def ramps():
    """Sand ramps drifted against the bench: floor-textured, with narrow sand skirts
    falling to the floor along both open sides (at most 25 cm outside collision)."""
    sand = Mesh()
    for ramp in level.RAMPS.values():
        x0, x1 = ramp['x']
        rows = []
        for k in range(9):
            t = k / 8
            z = ramp['z_low'] + (ramp['z_high'] - ramp['z_low']) * t
            y = level.ramp_height(ramp, z)
            w = 0.25 * (y - level.FLOOR_Y) / (level.UPPER_Y - level.FLOOR_Y) + 0.02
            row = []
            for xx, yy in ((x0 - w, level.FLOOR_Y - 0.02), (x0, y), (x1, y), (x1 + w, level.FLOOR_Y - 0.02)):
                row.append(sand.vertex((xx, yy, z), (0, 1, 0), floor_uv(xx, z)))
            rows.append(row)
        for a, b in zip(rows, rows[1:]):
            for i in range(3):
                sand.quad(a[i], a[i + 1], b[i + 1], b[i], (0, 1, 0))
    return sand.smooth_normals()


def canyon_walls(seed=401):
    """Both canyon walls (ragged, stepping down to the rim) and their caprock tops."""
    faces, caps = Mesh(), Mesh()
    for k, (name, wall) in enumerate(level.WALLS.items()):
        tops = [p[2] for p in wall]

        def top(t, tops=tops):
            i = min(int(t), len(tops) - 2)
            f = t - i
            return tops[i] + (tops[i + 1] - tops[i]) * N.smoothstep(0.25, 0.75, f)
        face, cols, rims = cliff_strip(wall, level.FLOOR_Y, top, seed + k, spacing=0.45, step=0.26, crest=0.35,
                                       deep=1.0)
        faces.merge(face)
        inner, outer = [], []
        for (x, z, run, nx, nz, _), rim in zip(cols, rims):
            inner.append(caps.vertex(rim, (0, 1, 0), (rim[0] / TOP_TILE, rim[2] / TOP_TILE)))
            fx, fz = x - nx * CAP_DEPTH, z - nz * CAP_DEPTH
            outer.append(caps.vertex((fx, rim[1] + 0.15, fz), (0, 1, 0), (fx / TOP_TILE, fz / TOP_TILE)))
        for i in range(len(cols) - 1):
            caps.quad(inner[i], inner[i + 1], outer[i + 1], outer[i], (0, 1, 0))
    caps.smooth_normals()
    return faces, caps


def upper_cliffs(seed=503):
    """1.6 m bench edges facing the floor, split around both ramps."""
    mesh = Mesh()
    for k, piece in enumerate(bench_outline()):
        face, _, _ = cliff_strip(piece, level.FLOOR_Y, level.UPPER_Y, seed + k, lip=True)
        mesh.merge(face)
    return mesh


# Beyond the rim the land drops 13.6 m to a hazy valley floor (VALLEY_Y). Far buttes
# stand on it 210-330 m out: (x, z, half x, half z, top y, optional upper tier as
# (scale, extra height)). From the mesa they rise only a few degrees above the horizon.
VALLEY_Y = -12.0
VALLEY_RADIUS = 430.0
FAR_BUTTES = [(-95.0, -210.0, 22.0, 12.0, 6.0, (0.5, 4.0)), (60.0, -250.0, 28.0, 14.0, 9.0, (0.45, 5.0)),
              (-10.0, -330.0, 45.0, 12.0, 4.0, None), (170.0, -290.0, 26.0, 16.0, 10.0, (0.55, 4.0)),
              (-200.0, -270.0, 32.0, 14.0, 7.0, None)]


def _butte(cx, cz, hx, hz, y0, y1, seed, corners=16):
    points = []
    for k in range(corners):
        a = -k / corners * math.tau            # clockwise in x-z: outward normals
        c, s = math.cos(a), math.sin(a)
        r = 1.0 + 0.12 * N.fbm2(c * 1.3 + seed, s * 1.3, seed, 2)
        sq = (math.copysign(abs(c) ** 0.45, c), math.copysign(abs(s) ** 0.45, s))
        points.append((cx + hx * r * sq[0], cz + hz * r * sq[1]))
    points.append(points[0])
    face, _, rims = cliff_strip(points, y0, y1, seed, spacing=5.0, step=2.0, crest=0.8, deep=3.0,
                               talus=(y1 - 6.0, 0.6) if y0 <= VALLEY_Y else None)
    face.uvs = [(u, v * 0.25) for u, v in face.uvs]      # broad bands at distance
    top = flat_polygon([(p[0], p[2]) for p in rims[:-1]], max(p[1] for p in rims) - 0.2, TOP_TILE)
    return face, top


def far_buttes(seed=601):
    sides, tops = Mesh(), Mesh()
    for i, (cx, cz, hx, hz, y1, tier) in enumerate(FAR_BUTTES):
        face, top = _butte(cx, cz, hx, hz, VALLEY_Y - 1.0, y1, seed + i)
        sides.merge(face)
        if tier:
            scale, extra = tier
            face2, top2 = _butte(cx + hx * 0.15, cz, hx * scale, hz * scale, y1 - 0.5, y1 + extra, seed + 20 + i, 12)
            sides.merge(face2)
            tops.merge(top2)
        tops.merge(top)
    return sides, tops


def valley_mesh(rings=18, segments=48):
    """Disc of hazy valley floor below the rim; u is the normalized distance."""
    mesh = Mesh()
    centre = (0.0, VALLEY_Y, level.RIM_Z)
    grid = []
    for j in range(rings + 1):
        r = VALLEY_RADIUS * (j / rings) ** 1.6
        row = []
        for i in range(segments + 1):
            a = math.tau * i / segments
            row.append(mesh.vertex((centre[0] + r * math.cos(a), VALLEY_Y, centre[2] + r * math.sin(a)), (0, 1, 0),
                                   (min(0.999, r / VALLEY_RADIUS), i / segments)))
        grid.append(row)
    for j in range(rings):
        for i in range(segments):
            mesh.quad(grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i], (0, 1, 0))
    return mesh


def valley_texture(width=256, height=128, seed=171):
    """Aerial perspective across the valley (u = distance, v = bearing): warm dusk earth
    near, sky haze far, with low ridges and washes that fade into the haze."""
    near, mid, far = R.hex_linear('#5B4446'), R.hex_linear('#7A6070'), R.hex_linear('#A08CA0')
    ridge_c, wash_c = R.hex_linear('#4A3739'), R.hex_linear('#86707A')
    terrain = R.field(64, 64, lambda u, v: N.fbm2(u * 10, v * 16, seed, 4, period=(10, 16)))
    image = []
    for row in range(height):
        v = (row + 0.5) / height
        line = []
        for col in range(width):
            u = (col + 0.5) / width
            base = N.mix3(N.mix3(near, mid, N.smoothstep(0.0, 0.35, u)), far, N.smoothstep(0.3, 1.0, u))
            n = R.sample(terrain, u, v, False, True)
            fade = 1.0 - N.smoothstep(0.25, 0.85, u)
            base = N.mix3(base, ridge_c, 0.45 * fade * N.smoothstep(0.15, 0.5, n))
            base = N.mix3(base, wash_c, 0.35 * fade * N.smoothstep(0.15, 0.5, -n))
            line.append(base)
        image.append(line)
    return image


def far_texture(size=256, seed=151):
    """Low-contrast, hazed strata for distant buttes (aerial perspective)."""
    haze = R.hex_linear('#8F8AA6')
    mottle = R.field(32, 32, lambda u, v: N.fbm2(u * 4, v * 4, seed, 3, period=(4, 4)))
    image = []
    for row in range(size):
        v = (row + 0.5) / size
        line = []
        for col in range(size):
            u = (col + 0.5) / size
            colour = N.mix3(strata_colour(v * CLIFF_TILE[1]), haze, 0.68)
            shade = 1.0 + 0.05 * R.sample(mottle, u, v)
            line.append(tuple(c * shade for c in colour))
        image.append(line)
    return image


def textures(out):
    pngio.write(out / 'far_albedo.png', 256, 256, 3, R.to_rows(far_texture()))
    pngio.write(out / 'valley.png', 256, 128, 3, R.to_rows(valley_texture()))
    cliff, cliff_h = cliff_textures()
    pngio.write(out / 'cliff_albedo.png', len(cliff[0]), len(cliff), 3, R.to_rows(cliff))
    pngio.write(out / 'cliff_normal.png', len(cliff[0]), len(cliff), 3, R.normal_rows(cliff_h, 0.55))
    top, top_h = top_texture()
    pngio.write(out / 'top_albedo.png', len(top), len(top), 3, R.to_rows(top))
    pngio.write(out / 'top_normal.png', len(top), len(top), 3, R.normal_rows(top_h, 1.2))
    floor = floor_texture()
    pngio.write(out / 'floor_albedo.png', len(floor), len(floor), 3, R.to_rows(floor))


def parts():
    """Terrain meshes keyed by world material name."""
    walls, caps = canyon_walls()
    cliffs = upper_cliffs().merge(walls)
    far_sides, far_tops = far_buttes()
    return {'floor': floor_mesh().merge(ramps()), 'top': upper_top().merge(caps), 'strata': cliffs,
            'far': far_sides.merge(far_tops), 'valley': valley_mesh()}
