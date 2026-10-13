"""Static set dressing: hoodoos, boulders and blocks, the fallen slab, cairns,
sage, talus, the survey camp, sample stands and the lift headframe. Everything
is merged per material into the canyon model, one draw per material.

Colour language shared with the actors: safety yellow marks the surveyor's
equipment (stands, pad rim, hook, rover deck), cyan is reserved for the samples,
ember orange for the chasers, and the land keeps the strata reds and ochres.
"""
import math

from . import level, noise as N, pngio, raster as R, terrain as T
from .mesh import Mesh, beam, cylinder, lathe, quat_axis, quat_mul, rock, rounded_box


def _place(mesh, at, yaw=0.0, pitch=0.0):
    q = quat_mul(quat_axis((0, 1, 0), yaw), quat_axis((1, 0, 0), pitch)) if pitch else quat_axis((0, 1, 0), yaw)
    return mesh.transformed(at, q)


# ---------------------------------------------------------------- textures

def rock_texture(size=256, seed=701):
    mott = R.field(64, 64, lambda u, v: N.fbm2(u * 4, v * 4, seed, 4, period=(4, 4)))
    fine = R.field(128, 128, lambda u, v: N.fbm2(u * 16, v * 16, seed + 1, 3, period=(16, 16)))
    a, b, c = R.hex_linear('#A8694B'), R.hex_linear('#8F5741'), R.hex_linear('#BE8762')
    varnish = R.hex_linear('#5A382E')
    image = []
    for row in range(size):
        v = (row + 0.5) / size
        line = []
        for col in range(size):
            u = (col + 0.5) / size
            m, f = R.sample(mott, u, v), R.sample(fine, u, v)
            colour = N.mix3(a, b, N.smoothstep(-0.3, 0.4, m))
            colour = N.mix3(colour, c, 0.45 * N.smoothstep(0.2, 0.7, f))
            colour = N.mix3(colour, varnish, 0.35 * N.smoothstep(0.35, 0.75, m * 0.7 + f * 0.6))
            g = 0.97 + 0.06 * N.unit(seed + 2, col, row)
            line.append(tuple(k * g for k in colour))
        image.append(line)
    return image


def pad_texture(size=256, seed=709):
    """Lift pad: dark steel tread plate, a safety-yellow rim and a centre ring."""
    steel, steel_hi = R.hex_linear('#3C4047'), R.hex_linear('#555B63')
    yellow, worn = R.hex_linear('#E0AE36'), R.hex_linear('#9C7D3A')
    image = []
    for row in range(size):
        line = []
        for col in range(size):
            x = (col + 0.5) / size * 2 - 1
            y = (row + 0.5) / size * 2 - 1
            r = math.hypot(x, y)
            gx, gy = (x + y) * 18, (x - y) * 18
            tread = (abs(gx - round(gx)) < 0.12) or (abs(gy - round(gy)) < 0.12)
            colour = steel_hi if tread else steel
            if 0.84 < r < 0.97 or 0.30 < r < 0.36:
                colour = N.mix3(yellow, worn, 0.35 * N.unit(seed, col // 3, row // 3))
            g = 0.93 + 0.12 * N.unit(seed + 1, col, row)
            line.append(tuple(k * g for k in colour))
        image.append(line)
    return image


# ---------------------------------------------------------------- shapes

def hoodoo(spec):
    """Eroded spire: a flared base, irregular bulges, a pinched neck and a wide cap."""
    x, y0, z = spec['at']
    r, height, seed = spec['radius'], spec['height'], spec['seed']
    steps = max(10, int(height / 0.15))
    k1, k2 = 1.6 + 1.4 * N.unit(seed, 1), 3.1 + 1.6 * N.unit(seed, 2)
    p1, p2 = N.unit(seed, 3) * math.tau, N.unit(seed, 4) * math.tau
    profile = []
    for i in range(steps + 1):
        t = i / steps
        y = height * t
        bulge = 1.0 + 0.13 * math.sin(math.tau * k1 * t + p1) + 0.07 * math.sin(math.tau * k2 * t + p2)
        taper = 1.0 - 0.28 * t
        flare = 1.0 + 0.32 * max(0.0, 1.0 - t / 0.12) ** 2
        neck = 1.0 - 0.38 * N.smoothstep(0.7, 0.93, t) * (1.0 - N.smoothstep(0.96, 1.0, t))
        ledge = 0.9 + 0.1 * T.hardness(y0 + y)
        profile.append((r * bulge * taper * flare * neck * ledge, y0 + y))

    def radial(angle, y, j):
        return r * (0.1 * N.fbm2(math.cos(angle) * 1.5 + seed, math.sin(angle) * 1.5 + y * 0.8, seed, 3)
                    - 0.06 * max(0.0, math.sin(angle * 5 + seed)) ** 6)
    body = lathe(profile, 20, radial, u_scale=math.tau * r / T.CLIFF_TILE[0], v_scale=1.0 / T.CLIFF_TILE[1])
    lean = quat_axis((math.cos(seed), 0, math.sin(seed)), 2.5 * (N.unit(seed, 5) - 0.3))
    body = body.transformed((x, 0.0, z), lean)
    top_r = profile[-1][0]
    cap = rock(top_r * 1.75, (1.0, 0.4, 0.92), 0.2, seed, 2)
    cap = cap.transformed((x + 0.05 * r, y0 + height + top_r * 0.32, z - 0.04 * r), quat_axis((0, 1, 0), seed * 17))
    return body, cap


def boulder(spec):
    x, y, z = spec['at']
    r = spec['radius']
    mesh = rock(r, (1.0, 0.86, 0.94), 0.2, spec['seed'], 2)
    return mesh.transformed((x, y + r * 0.55, z), quat_axis((0, 1, 0), spec['seed'] * 29))


def block(spec):
    """Angular sandstone block fallen from a ledge (matches its box collider)."""
    x, y, z = spec['at']
    half = spec['half']
    mesh = rounded_box(half, seed=spec['seed'], roughness=0.22, exponent=0.16, divisions=5)
    return _place(mesh, (x, y + half[1] * 0.85, z), spec['yaw'])


def slab():
    """A fallen ledge segment: layered (strata UVs by world height), broken ends."""
    x, y, z = level.SLAB['at']
    half = level.SLAB['half']
    mesh = rock(1.0, (half[0] * 1.02, half[1] * 1.12, half[2] * 1.04), 0.2, 61, 1).flat()
    mesh = _place(mesh, (x, y + half[1] * 0.92, z), level.SLAB['yaw'], 2.0)
    mesh.uvs = [(p[0] * 0.5 / T.CLIFF_TILE[0] + p[2] * 0.5 / T.CLIFF_TILE[0], p[1] / T.CLIFF_TILE[1])
                for p in mesh.positions]
    return mesh


def cairn(x, y, z, seed):
    mesh = Mesh()
    h = 0.0
    for k in range(5):
        r = 0.22 - 0.032 * k + 0.02 * N.unit(seed, k)
        stone = rock(r, (1.0, 0.42, 0.9), 0.15, seed * 5 + k, 1)
        mesh.merge(stone.transformed((x + 0.02 * (N.unit(seed, k, 2) - 0.5), y + h + r * 0.38, z),
                                     quat_axis((0, 1, 0), 70 * k + seed)))
        h += r * 0.78
    return mesh


def _blocked(x, z, keep_out):
    return any(math.hypot(x - kx, z - kz) < kr for kx, kz, kr in keep_out)


def _keep_out():
    out = [(s[0], s[2], 1.6) for s in level.SAMPLES]
    out += [(level.PLAYER_START[0], level.PLAYER_START[2], 2.2), (level.EXIT[0], level.EXIT[2], 2.4)]
    for ramp in level.RAMPS.values():
        cx = sum(ramp['x']) / 2
        for k in range(7):
            z = ramp['z_low'] + (ramp['z_high'] - ramp['z_low']) * k / 6
            out.append((cx, z, 2.0))
    for h in level.HOODOOS:
        out.append((h['at'][0], h['at'][2], h['radius'] + 0.2))
    for b in level.BOULDERS:
        out.append((b['at'][0], b['at'][2], b['radius'] + 0.1))
    for b in level.BLOCKS:
        out.append((b['at'][0], b['at'][2], max(b['half']) + 0.1))
    for c in level.CAIRNS:
        out.append((c[0], c[2], 0.5))
    camp = level.CAMP
    out += [(camp['tent']['at'][0], camp['tent']['at'][2], 1.6), (camp['rover']['at'][0], camp['rover']['at'][2], 1.4)]
    return out


def _edges():
    """(a, b, normal toward the walkable side, ground height) for base scatter."""
    out = []
    for piece in T.bench_outline():
        for a, b in zip(piece, piece[1:]):
            dx, dz = b[0] - a[0], b[1] - a[1]
            n = math.hypot(dx, dz)
            out.append((a, b, (-dz / n, dx / n), level.FLOOR_Y))
    for wall in level.WALLS.values():
        for a, b in zip(wall, wall[1:]):
            dx, dz = b[0] - a[0], b[1] - a[1]
            n = math.hypot(dx, dz)
            ground = level.UPPER_Y if (a[1] + b[1]) / 2 < level.NORTH_PLATEAU['z'][1] else level.FLOOR_Y
            out.append(((a[0], a[1]), (b[0], b[1]), (-dz / n, dx / n), ground))
    return out


def scatter(seed, count, band, keep_out):
    """Deterministic points near cliff bases: (x, ground, z, distance from face, index)."""
    edges = _edges()
    lengths = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b, _, _ in edges]
    total = sum(lengths)
    points, attempt = [], 0
    while len(points) < count and attempt < count * 40:
        attempt += 1
        t = N.unit(seed, attempt, 1) * total
        for (a, b, (nx, nz), ground), length in zip(edges, lengths):
            if t <= length:
                f = t / length
                d = band[0] + (band[1] - band[0]) * N.unit(seed, attempt, 2) ** 1.6
                x = a[0] + (b[0] - a[0]) * f + nx * d
                z = a[1] + (b[1] - a[1]) * f + nz * d
                if z > level.SOUTH_LIMIT_Z + 6 or _blocked(x, z, keep_out):
                    break
                points.append((x, ground, z, d, attempt))
                break
            t -= length
    return points


def sage(x, y, z, seed, size):
    mesh = Mesh()
    blobs = 3 + int(N.unit(seed, 1) * 3)
    for k in range(blobs):
        a = N.unit(seed, k, 2) * math.tau
        d = size * 0.45 * N.unit(seed, k, 3)
        r = size * (0.32 + 0.22 * N.unit(seed, k, 4))
        blob = rock(r, (1.0, 0.72, 1.0), 0.32, seed * 7 + k, 1)
        mesh.merge(blob.transformed((x + math.cos(a) * d, y + r * 0.45, z + math.sin(a) * d)))
    return mesh


def talus(x, y, z, seed):
    r = 0.07 + 0.13 * N.unit(seed, 5) ** 1.5
    pebble = rock(r, (1.0, 0.6 + 0.3 * N.unit(seed, 6), 1.0), 0.3, seed, 0)
    return pebble.transformed((x, y + r * 0.25, z), quat_axis((0, 1, 0), seed * 41))


def bench_rubble(seed=1501, count=46):
    """Loose caprock blocks and sage on the mesa top, just behind the bench edge (decor only)."""
    rocks, plants = Mesh(), Mesh()
    pieces = T.bench_outline()
    edges = [(a, b) for piece in pieces for a, b in zip(piece, piece[1:])]
    lengths = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in edges]
    total = sum(lengths)
    keep = _keep_out()
    for k in range(count):
        t = N.unit(seed, k) * total
        for (a, b), length in zip(edges, lengths):
            if t <= length:
                f = t / length
                dx, dz = (b[0] - a[0]) / length, (b[1] - a[1]) / length
                inset = 0.25 + 0.9 * N.unit(seed, k, 2) ** 1.5      # behind the edge, on the mesa
                x = a[0] + dx * length * f - (-dz) * inset
                z = a[1] + dz * length * f - dx * inset
                if not _blocked(x, z, keep):
                    if N.unit(seed, k, 3) < 0.62:
                        rocks.merge(talus(x, level.UPPER_Y, z, seed + k).transformed((0, 0, 0)))
                    else:
                        plants.merge(sage(x, level.UPPER_Y, z, seed + k, 0.26 + 0.2 * N.unit(seed, k, 4)))
                break
            t -= length
    return rocks, plants


def mesa_scatter(seed=1601, rocks=26, plants=14):
    """Loose stones and sage across the upper mesa interior (decor only, kept off routes)."""
    stones, sage_mesh = Mesh(), Mesh()
    keep = _keep_out() + [(x, z, 2.0) for x, _, z in level.ENEMIES]
    for ramp in level.RAMPS.values():
        keep.append((sum(ramp['x']) / 2, ramp['z_high'] - 1.2, 2.2))
    areas = [((-10.2, 10.2), (-20.4, -13.2)), ((5.6, 11.0), (-12.0, -3.2))]
    placed, attempt = 0, 0
    while placed < rocks + plants and attempt < 2000:
        attempt += 1
        (x0, x1), (z0, z1) = areas[0] if N.unit(seed, attempt, 0) < 0.72 else areas[1]
        x = x0 + (x1 - x0) * N.unit(seed, attempt, 1)
        z = z0 + (z1 - z0) * N.unit(seed, attempt, 2)
        if _blocked(x, z, keep) or abs(x - level.EXIT[0]) < 3.0 and z < -13.0:
            continue
        if placed < rocks:
            stones.merge(talus(x, level.UPPER_Y, z, seed + attempt))
        else:
            sage_mesh.merge(sage(x, level.UPPER_Y, z, seed + attempt, 0.28 + 0.22 * N.unit(seed, attempt, 3)))
        placed += 1
    return stones, sage_mesh


def dome_tent():
    """Two-pole dome: canvas shell, a slate rain fly on the crown and a dark doorway."""
    t = level.CAMP['tent']
    radius, height = t['radius'], t['height']
    profile = [(radius * math.sqrt(max(0.0, 1.0 - (k / 10) ** 2)) * (1.0 + 0.04 * math.sin(k)), height * k / 10)
               for k in range(11)]
    profile[-1] = (0.0, height)
    canvas = lathe(profile, 24)
    fly = lathe([(radius * 0.84 + 0.02, height * 0.56), (radius * 0.6 + 0.02, height * 0.8),
                 (radius * 0.32 + 0.02, height * 0.95), (0.0, height + 0.02)], 24)
    door = lathe([(radius * 0.99 + 0.012, 0.0), (radius * 0.93 + 0.012, height * 0.38),
                  (radius * 0.72 + 0.012, height * 0.7)], 8, phase=math.radians(60), arc=math.radians(46))
    pose = (t['at'][0], level.FLOOR_Y, t['at'][2])
    return _place(canvas, pose, t['yaw']), _place(fly, pose, t['yaw']), _place(door, pose, t['yaw'])


def rover():
    """Parked six-wheel sample rover: white hull, yellow deck box, a mast and lamp."""
    r = level.CAMP['rover']
    hx, hy, hz = r['half']
    hull = rounded_box((hx, hy * 0.6, hz), exponent=0.18, divisions=4).transformed((0, 0.62, 0))
    deck = rounded_box((hx * 0.7, 0.14, hz * 0.32), exponent=0.2, divisions=3).transformed((0, 0.98, hz * 0.45))
    wheels, dark = Mesh(), Mesh()
    for side in (-1, 1):
        for zz in (-hz * 0.72, 0.0, hz * 0.72):
            w = cylinder(0.21, -0.075, 0.075, 14).transformed((side * (hx + 0.06), 0.21, zz),
                                                               quat_axis((0, 0, 1), 90))
            wheels.merge(w)
    mast = beam((0.25, 0.9, -hz * 0.55), (0.25, 1.75, -hz * 0.55), 0.05)
    dish = lathe([(0.0, 0.0), (0.16, 0.03), (0.18, 0.06)], 12).transformed((0.25, 1.78, -hz * 0.55),
                                                                          quat_axis((1, 0, 0), -35))
    dark.merge(mast).merge(dish)
    lamp = rounded_box((0.05, 0.04, 0.04), exponent=0.2, divisions=2).transformed((0.25, 1.62, -hz * 0.55 - 0.05))
    pose = (r['at'][0], level.FLOOR_Y, r['at'][2])
    return [_place(m, pose, r['yaw']) for m in (hull, deck, wheels, dark, lamp)]


def crates():
    wood, straps = Mesh(), Mesh()
    for c in level.CAMP['crates']:
        half = c['half']
        box = rounded_box(half, seed=3, roughness=0.0, exponent=0.12, divisions=3)
        wood.merge(_place(box, (c['at'][0], half[1], c['at'][2]), c['yaw']))
        for zz in (-half[2] * 0.55, half[2] * 0.55):
            strap = rounded_box((half[0] + 0.012, half[1] + 0.012, 0.025), exponent=0.1, divisions=2)
            straps.merge(_place(strap.transformed((0, 0, zz)), (c['at'][0], half[1], c['at'][2]), c['yaw']))
    return wood, straps


def lamp():
    x, _, z = level.CAMP['lamp']['at']
    h = level.CAMP['lamp']['height']
    pole = cylinder(0.045, 0.0, h, 10).transformed((x, 0.0, z))
    pole.merge(cylinder(0.16, 0.0, 0.05, 12).transformed((x, 0.0, z)))
    glass = rounded_box((0.09, 0.13, 0.09), exponent=0.2, divisions=3).transformed((x, h + 0.02, z))
    hood = cylinder(0.14, 0.0, 0.05, 12, radius_top=0.05).transformed((x, h + 0.15, z))
    return pole.merge(hood), glass


def holder(x, y, z):
    """Sample stand: three splayed legs and a ring that cradles the hovering core."""
    legs, ring = Mesh(), Mesh()
    for k in range(3):
        a = math.tau * k / 3 + 0.4
        foot = (x + math.cos(a) * 0.3, y, z + math.sin(a) * 0.3)
        top = (x + math.cos(a) * 0.19, y + 0.27, z + math.sin(a) * 0.19)
        legs.merge(beam(foot, top, 0.04))
    profile = [(0.19, y + 0.25), (0.24, y + 0.25), (0.24, y + 0.31), (0.19, y + 0.31), (0.19, y + 0.25)]
    ring.merge(lathe(profile, 20).transformed((x, 0.0, z)))
    return legs, ring


def headframe():
    """Lift headframe: a vertical cage over the pad, a tapering tower and a crown."""
    x, y, z = level.LIFT['at']
    leg_r, cage, height = level.LIFT['leg_radius'], level.LIFT['cage'], level.LIFT['height']
    top_r = 0.5
    steel, yellow, pad = Mesh(), Mesh(), Mesh()
    corners = [math.radians(45 + 90 * i) for i in range(4)]

    def at(radius, h, angle):
        return (x + radius * math.cos(angle), y + h, z + radius * math.sin(angle))
    for a in corners:
        steel.merge(beam(at(leg_r, 0.0, a), at(leg_r, cage, a), 0.14))
        steel.merge(beam(at(leg_r, cage, a), at(top_r, height, a), 0.12))
        foot = rounded_box((0.17, 0.05, 0.17), exponent=0.2, divisions=2)
        steel.merge(foot.transformed(at(leg_r, 0.03, a)))
    mid = (leg_r - (leg_r - top_r) * 0.45, cage + (height - cage) * 0.45)
    for r, h in ((leg_r, cage), mid, (top_r, height)):
        for i in range(4):
            steel.merge(beam(at(r, h, corners[i]), at(r, h, corners[(i + 1) % 4]), 0.09))
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        for (r0, h0), (r1, h1) in (((leg_r, cage), mid), (mid, (top_r, height))):
            steel.merge(beam(at(r0, h0, a), at(r1, h1, b), 0.06))
            steel.merge(beam(at(r0, h0, b), at(r1, h1, a), 0.06))
    crown = rounded_box((0.55, 0.16, 0.55), exponent=0.15, divisions=2).transformed((x, y + height + 0.1, z))
    steel.merge(crown)
    wheel = lathe([(0.0, -0.05), (0.42, -0.05), (0.42, 0.05), (0.0, 0.05)], 20)
    steel.merge(wheel.transformed((x + 0.3, y + height + 0.55, z), quat_axis((0, 0, 1), 90)))
    steel.merge(beam((x, y + height + 0.1, z), (x, y + height + 1.0, z), 0.08))
    steel.merge(cylinder(0.018, y + 2.75, y + height + 0.1, 6, caps=False).transformed((x, 0.0, z)))
    yellow.merge(rounded_box((0.13, 0.17, 0.13), exponent=0.2, divisions=2).transformed((x, y + 2.6, z)))
    pad_r = level.LIFT['pad_radius']
    steel.merge(lathe([(pad_r, y - 0.12), (pad_r, y + 0.03)], 32).transformed((x, 0.0, z)))
    centre = pad.vertex((x, y + 0.03, z), (0, 1, 0), (0.5, 0.5))
    ring = []
    for k in range(48):
        a = math.tau * k / 48
        ring.append(pad.vertex((x + pad_r * math.cos(a), y + 0.03, z + pad_r * math.sin(a)), (0, 1, 0),
                               (0.5 + 0.5 * math.cos(a), 0.5 + 0.5 * math.sin(a))))
    for k in range(48):
        pad.tri(centre, ring[k], ring[(k + 1) % 48], (0, 1, 0))
    beacon = lathe([(0.0, 0.0), (0.16, 0.02), (0.17, 0.2), (0.0, 0.25)], 14)
    beacon = beacon.transformed((x, y + height + 1.0, z))
    return steel, yellow, pad, beacon


def textures(out):
    pngio.write(out / 'rock_albedo.png', 256, 256, 3, R.to_rows(rock_texture()))
    pngio.write(out / 'pad_albedo.png', 256, 256, 3, R.to_rows(pad_texture()))


def parts():
    """Set-dressing meshes keyed by world material name."""
    keys = ('strata', 'rock', 'sage', 'sage_dark', 'canvas', 'fly', 'dark', 'white', 'crate', 'steel', 'yellow',
            'pad', 'lantern', 'beacon')
    out = {k: Mesh() for k in keys}
    for spec in level.HOODOOS:
        body, cap = hoodoo(spec)
        out['strata'].merge(body)
        out['rock'].merge(cap)
    for spec in level.BOULDERS:
        out['rock'].merge(boulder(spec))
    for spec in level.BLOCKS:
        out['rock'].merge(block(spec))
    out['strata'].merge(slab())
    for i, (x, y, z) in enumerate(level.CAIRNS):
        out['rock'].merge(cairn(x, y, z, 50 + i))
    keep = _keep_out()
    for x, y, z, d, k in scatter(811, 22, (0.2, 1.3), keep):
        size = 0.3 + 0.28 * N.unit(811, k, 9)
        out['sage' if N.unit(811, k, 10) < 0.6 else 'sage_dark'].merge(sage(x, y, z, k, size))
    for x, y, z, d, k in scatter(907, 110, (0.0, 0.6), keep):
        out['rock'].merge(talus(x, y, z, k))
    rubble, plants = bench_rubble()
    out['rock'].merge(rubble)
    out['sage'].merge(plants)
    stones, more = mesa_scatter()
    out['rock'].merge(stones)
    out['sage_dark'].merge(more)
    canvas, fly, door = dome_tent()
    out['canvas'].merge(canvas)
    out['fly'].merge(fly)
    out['dark'].merge(door)
    hull, deck, wheels, mast, lamp_glass = rover()
    out['white'].merge(hull)
    out['yellow'].merge(deck)
    out['dark'].merge(wheels)
    out['steel'].merge(mast)
    out['lantern'].merge(lamp_glass)
    wood, straps = crates()
    out['crate'].merge(wood)
    out['dark'].merge(straps)
    pole, glass = lamp()
    out['steel'].merge(pole)
    out['lantern'].merge(glass)
    for sx, sy, sz in level.SAMPLES:
        legs, ring = holder(sx, sy - level.CENTRE, sz)
        out['steel'].merge(legs)
        out['yellow'].merge(ring)
    steel, yellow, pad, beacon = headframe()
    out['steel'].merge(steel)
    out['yellow'].merge(yellow)
    out['pad'].merge(pad)
    out['beacon'].merge(beacon)
    return out
