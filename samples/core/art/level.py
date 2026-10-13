"""Core Sample level layout: the single source for visuals, collision and roles.

Coordinates: metres, +Y up, +X east (screen right), -Z north (into the screen).
The camera never rotates, so the level is composed for a north-facing view.

Walkable ground is two flat levels joined by two ramps: the canyon floor at
FLOOR_Y and an L-shaped upper mesa (north plateau + east shelf) at UPPER_Y.
Collision is a few static compound colliders of boxes, spheres and capsules;
the navigation mesh is generated from exactly those colliders. Ramps are single
rotated boxes whose top faces meet the floor and the mesa edge flush, so the
character controller (no autostep) and the navmesh agree; no off-mesh links.
Visual cliffs sit on or slightly behind every collision face.
"""
import math

from . import ids

FLOOR_Y = 0.0
UPPER_Y = 1.6
CENTRE = 0.62               # capsule centre above ground (radius 0.3 + half 0.3 + skin)
CAPSULE = {'type': 'capsule', 'half_height': 0.3, 'radius': 0.3}

# Play area, collision faces of the outer canyon walls and the camp boundary.
PLAY_X = (-10.5, 10.5)
PLAY_Z = (-20.6, 13.0)
NORTH_PLATEAU = {'x': (-11.6, 11.6), 'z': (-21.0, -12.5)}
EAST_SHELF = {'x': (5.0, 11.6), 'z': (-12.5, -2.5)}

# Ramps rise toward -Z from (z_low, FLOOR_Y) to (z_high, UPPER_Y) across x. Three ramps keep
# chaser detours short (the controller's navmesh searches have a fixed visit budget).
RAMPS = {
    'east': {'x': (6.3, 8.9), 'z_low': 2.5, 'z_high': -2.5},
    'centre': {'x': (-2.6, 0.0), 'z_low': -7.0, 'z_high': -12.5},
    'west': {'x': (-10.6, -7.8), 'z_low': -7.0, 'z_high': -12.5},
}

# Outer canyon walls: inner-face polylines of (x, z, cap height). The west wall runs
# north and the east wall runs south, so each face looks right of its direction of
# travel, into the canyon. Both step down toward the open north rim, where the
# plateau ends at a drop and the lift stands against the sky.
WALLS = {
    'west': [(-10.5, 26.0, 7.4), (-10.4, 16.0, 7.6), (-10.9, 12.0, 7.3), (-12.1, 9.6, 7.1), (-11.9, 6.6, 6.9),
             (-10.6, 4.1, 6.7), (-10.3, 0.5, 6.4), (-10.8, -3.4, 6.1), (-11.5, -6.0, 5.8), (-10.9, -9.0, 5.4),
             (-10.6, -12.6, 4.9), (-10.9, -15.6, 4.3), (-10.7, -18.2, 3.7), (-11.6, -20.6, 3.0),
             (-11.8, -21.3, 2.9)],
    'east': [(11.7, -21.3, 3.1), (11.5, -20.6, 3.2), (10.7, -17.6, 3.6), (11.0, -14.0, 4.1),
             (10.5, -10.0, 4.6), (10.9, -6.0, 5.0), (11.6, -2.0, 5.4), (10.9, 1.5, 5.7), (10.4, 5.0, 6.0),
             (11.0, 8.0, 6.2), (11.4, 11.5, 6.4), (10.6, 15.0, 6.7), (10.6, 26.0, 6.9)],
}
RIM_Z = -21.0            # north edge of the upper plateau (a drop beyond)
SOUTH_LIMIT_Z = 13.0

# Gameplay roles (capsule centres / pickup sensor centres / exit point).
PLAYER_START = (-0.5, FLOOR_Y + CENTRE, 10.0)
CAMERA_OFFSET = (0.0, 3.3, 7.4)
CAMERA_PITCH_DEGREES = -15.0
CAMERA_FOV_DEGREES = 50.0
EXIT = (0.5, UPPER_Y + CENTRE, -17.4)
SAMPLES = [
    (-6.2, FLOOR_Y + CENTRE, 5.2),
    (3.3, FLOOR_Y + CENTRE, -10.3),
    (8.6, UPPER_Y + CENTRE, -9.4),
    (-8.0, UPPER_Y + CENTRE, -16.8),
]
ENEMIES = [
    (-3.4, FLOOR_Y + CENTRE, -5.0),
    (-4.2, UPPER_Y + CENTRE, -16.0),
    (8.2, UPPER_Y + CENTRE, -15.6),
]

# Bench promontories: rock noses standing proud of the 1.6 m edge. Each is one
# collision box flush with the bench top; the cliff outline below follows them.
PROMONTORIES = [
    {'x': (-5.2, -2.6), 'z': (-12.5, -11.5)},
    {'x': (1.4, 2.9), 'z': (-12.5, -11.9)},
    {'x': (4.2, 5.0), 'z': (-9.0, -6.2)},
]

# Static obstacles: hoodoos (capsule colliders), round boulders (spheres), angular
# blocks (boxes), a low fallen slab the player can jump but chasers must route
# around, trail cairns, the rim stones and the survey camp.
HOODOOS = [
    {'at': (0.9, FLOOR_Y, -3.4), 'radius': 0.9, 'height': 4.9, 'seed': 11},
    {'at': (-4.3, FLOOR_Y, -8.4), 'radius': 0.7, 'height': 3.4, 'seed': 23},
    {'at': (-10.8, FLOOR_Y, 8.6), 'radius': 0.75, 'height': 4.1, 'seed': 37},
    {'at': (8.9, UPPER_Y, -17.9), 'radius': 0.72, 'height': 3.3, 'seed': 41},
    {'at': (-10.2, UPPER_Y, -16.6), 'radius': 0.55, 'height': 2.4, 'seed': 43},
]
BOULDERS = [
    {'at': (-7.7, FLOOR_Y, 3.3), 'radius': 0.8, 'seed': 3},
    {'at': (-4.9, FLOOR_Y, 7.0), 'radius': 0.5, 'seed': 5},
    {'at': (9.6, UPPER_Y, -5.6), 'radius': 0.6, 'seed': 13},
    {'at': (2.2, FLOOR_Y, 4.9), 'radius': 0.42, 'seed': 29},
    {'at': (-4.2, UPPER_Y, -20.4), 'radius': 0.55, 'seed': 31},
    {'at': (9.9, UPPER_Y, -20.3), 'radius': 0.65, 'seed': 33},
]
BLOCKS = [
    {'at': (10.1, FLOOR_Y, 5.3), 'half': (0.75, 0.55, 0.6), 'yaw': 24.0, 'seed': 7},
    {'at': (-9.5, FLOOR_Y, -3.0), 'half': (0.7, 0.6, 0.9), 'yaw': -14.0, 'seed': 9},
    {'at': (-6.4, FLOOR_Y, -11.3), 'half': (0.6, 0.45, 0.55), 'yaw': 37.0, 'seed': 15},
    {'at': (-9.6, UPPER_Y, -19.9), 'half': (0.8, 0.5, 0.6), 'yaw': 12.0, 'seed': 17},
    {'at': (5.0, UPPER_Y, -20.2), 'half': (0.7, 0.42, 0.5), 'yaw': -20.0, 'seed': 19},
    {'at': (7.3, FLOOR_Y, -0.9), 'half': (0.45, 0.35, 0.5), 'yaw': 8.0, 'seed': 21},
]
SLAB = {'at': (-1.7, FLOOR_Y, 3.6), 'half': (1.35, 0.22, 0.42), 'yaw': 8.0}
CAIRNS = [(-3.6, FLOOR_Y, 0.9), (5.6, FLOOR_Y, 3.4), (-7.4, FLOOR_Y, -5.6), (2.0, UPPER_Y, -14.6)]
CAMP = {
    'tent': {'at': (3.7, FLOOR_Y, 7.9), 'radius': 1.05, 'height': 1.2, 'yaw': -25.0},
    'rover': {'at': (6.3, FLOOR_Y, 10.6), 'half': (0.62, 0.42, 0.9), 'yaw': 28.0},
    'crates': [{'at': (2.0, FLOOR_Y, 9.6), 'half': (0.3, 0.26, 0.3), 'yaw': 18.0},
               {'at': (2.55, FLOOR_Y, 10.05), 'half': (0.24, 0.2, 0.24), 'yaw': -9.0}],
    'lamp': {'at': (5.2, FLOOR_Y, 6.6), 'height': 2.2},
}
LIFT = {'at': (EXIT[0], UPPER_Y, EXIT[2]), 'pad_radius': 1.3, 'leg_radius': 1.6, 'cage': 2.6, 'height': 6.4}


def _quat(yaw=0.0, pitch=0.0):
    """Rotation = yaw about +Y applied after pitch about +X (both degrees)."""
    sp, cp = math.sin(math.radians(pitch) / 2), math.cos(math.radians(pitch) / 2)
    sy, cy = math.sin(math.radians(yaw) / 2), math.cos(math.radians(yaw) / 2)
    return (cy * sp, sy * cp, -sy * sp, cy * cp)


def _box(part, centre, half, yaw=0.0, pitch=0.0):
    return {'id': part, 'translation': [round(v, 6) for v in centre],
            'rotation': [round(v, 9) for v in _quat(yaw, pitch)],
            'shape': {'type': 'box', 'half_extents': [round(v, 6) for v in half]}}


def ramp_box(ramp):
    """Rotated box whose top face runs from (z_low, FLOOR_Y) to (z_high, UPPER_Y)."""
    run = ramp['z_low'] - ramp['z_high']
    rise = UPPER_Y - FLOOR_Y
    slope = math.atan2(rise, run)
    extend_low = 0.6                       # bury the low end below the floor
    z0 = ramp['z_low'] + extend_low
    y0 = FLOOR_Y - extend_low * math.tan(slope)
    z1, y1 = ramp['z_high'], UPPER_Y       # end flush at the mesa edge, no lip
    thickness = 0.5
    top_mid = ((z0 + z1) / 2, (y0 + y1) / 2)
    normal = (math.cos(slope), math.sin(slope))  # (y, z) components of the top normal
    centre = ((ramp['x'][0] + ramp['x'][1]) / 2, top_mid[1] - normal[0] * thickness / 2,
              top_mid[0] - normal[1] * thickness / 2)
    half = ((ramp['x'][1] - ramp['x'][0]) / 2, thickness / 2, math.hypot(z0 - z1, y0 - y1) / 2)
    return centre, half, math.degrees(slope)


def ramp_height(ramp, z):
    """Walkable surface height of a ramp at z (clamped to its run)."""
    t = (ramp['z_low'] - z) / (ramp['z_low'] - ramp['z_high'])
    return FLOOR_Y + max(0.0, min(1.0, t)) * (UPPER_Y - FLOOR_Y)


def _id(prefix, index):
    """Stable collider part IDs (canonical 26-character Crockford text)."""
    return ids.part(int(prefix) * 1000 + index)


def colliders():
    """Static compound colliders keyed by entity role: {name: [parts]}."""
    ground = [
        _box(_id('0', 1), (0.0, FLOOR_Y - 0.5, -3.0), (16.0, 0.5, 22.0)),
        # North plateau: flush with its floor-side edge, extended only past the rim.
        _box(_id('0', 2), (0.0, (FLOOR_Y + UPPER_Y) / 2, (NORTH_PLATEAU['z'][0] - 0.6 + NORTH_PLATEAU['z'][1]) / 2),
             (12.5, (UPPER_Y - FLOOR_Y) / 2, (NORTH_PLATEAU['z'][1] - NORTH_PLATEAU['z'][0] + 0.6) / 2)),
        _box(_id('0', 3), (sum(EAST_SHELF['x']) / 2 + 0.35, (FLOOR_Y + UPPER_Y) / 2, sum(EAST_SHELF['z']) / 2),
             ((EAST_SHELF['x'][1] - EAST_SHELF['x'][0]) / 2 + 0.35, (UPPER_Y - FLOOR_Y) / 2,
              (EAST_SHELF['z'][1] - EAST_SHELF['z'][0]) / 2)),
    ]
    for index, name in enumerate(('east', 'west', 'centre')):
        centre, half, slope = ramp_box(RAMPS[name])
        ground.append(_box(_id('1', index + 1), centre, half, pitch=slope))
    for index, p in enumerate(PROMONTORIES):
        # Overlap 0.1 m into the bench so the coplanar tops share no seam gap.
        x0, x1 = p['x']
        z0, z1 = p['z']
        if x1 >= EAST_SHELF['x'][0] - 1e-6:
            x1 += 0.1
        else:
            z0 -= 0.1
        ground.append(_box(_id('0', 10 + index), ((x0 + x1) / 2, (FLOOR_Y + UPPER_Y) / 2, (z0 + z1) / 2),
                           ((x1 - x0) / 2, (UPPER_Y - FLOOR_Y) / 2, (z1 - z0) / 2)))
    walls, n = [], 0
    for name, wall in WALLS.items():
        for (ax, az, at), (bx, bz, bt) in zip(wall, wall[1:]):
            n += 1
            dx, dz = bx - ax, bz - az
            seg = math.hypot(dx, dz)
            yaw = math.degrees(math.atan2(-dz, dx))
            fx, fz = -dz / seg, dx / seg          # face normal: right of travel, into the canyon
            thick = 2.0
            centre = ((ax + bx) / 2 - fx * thick / 2, 3.0, (az + bz) / 2 - fz * thick / 2)
            walls.append(_box(_id('2', n), centre, (seg / 2 + 0.6, 3.0, thick / 2), yaw=yaw))
    walls.append(_box(_id('2', 80), (0.0, UPPER_Y + 1.5, RIM_Z - 0.5), (13.0, 1.5, 0.5)))
    walls.append(_box(_id('2', 90), (0.0, 1.5, SOUTH_LIMIT_Z + 1.0), (11.5, 1.5, 1.0)))
    rocks = []
    for i, h in enumerate(HOODOOS):
        x, y, z = h['at']
        half = max(0.05, h['height'] * 0.5 - h['radius'])
        rocks.append({'id': _id('3', i + 1), 'translation': [x, y + h['height'] * 0.5, z], 'rotation': [0, 0, 0, 1],
                      'shape': {'type': 'capsule', 'half_height': round(half, 6), 'radius': h['radius']}})
    for i, b in enumerate(BOULDERS):
        x, y, z = b['at']
        rocks.append({'id': _id('4', i + 1), 'translation': [x, y + b['radius'] * 0.55, z], 'rotation': [0, 0, 0, 1],
                      'shape': {'type': 'sphere', 'radius': b['radius']}})
    for i, b in enumerate(BLOCKS):
        x, y, z = b['at']
        rocks.append(_box(_id('4', 50 + i), (x, y + b['half'][1] * 0.85, z), b['half'], yaw=b['yaw']))
    x, y, z = SLAB['at']
    rocks.append(_box(_id('5', 1), (x, y + SLAB['half'][1], z), SLAB['half'], yaw=SLAB['yaw']))
    for i, (x, y, z) in enumerate(CAIRNS):
        rocks.append({'id': _id('5', 10 + i), 'translation': [x, y + 0.4, z], 'rotation': [0, 0, 0, 1],
                      'shape': {'type': 'capsule', 'half_height': 0.18, 'radius': 0.22}})
    t = CAMP['tent']
    rocks.append({'id': _id('6', 1), 'translation': [t['at'][0], t['at'][1] + 0.2, t['at'][2]], 'rotation': [0, 0, 0, 1],
                  'shape': {'type': 'sphere', 'radius': t['radius']}})
    r = CAMP['rover']
    rocks.append(_box(_id('6', 5), (r['at'][0], r['half'][1] + 0.12, r['at'][2]), r['half'], yaw=r['yaw']))
    for i, c in enumerate(CAMP['crates']):
        rocks.append(_box(_id('6', 2 + i), (c['at'][0], c['half'][1], c['at'][2]), c['half'], yaw=c['yaw']))
    lx, _, lz = CAMP['lamp']['at']
    rocks.append({'id': _id('6', 9), 'translation': [lx, CAMP['lamp']['height'] / 2, lz], 'rotation': [0, 0, 0, 1],
                  'shape': {'type': 'capsule', 'half_height': CAMP['lamp']['height'] / 2 - 0.12, 'radius': 0.12}})
    for i in range(4):
        angle = math.radians(45 + 90 * i)
        x = LIFT['at'][0] + LIFT['leg_radius'] * math.cos(angle)
        z = LIFT['at'][2] + LIFT['leg_radius'] * math.sin(angle)
        rocks.append({'id': _id('7', i + 1), 'translation': [round(x, 6), UPPER_Y + LIFT['cage'] / 2, round(z, 6)],
                      'rotation': [0, 0, 0, 1],
                      'shape': {'type': 'capsule', 'half_height': LIFT['cage'] / 2 - 0.14, 'radius': 0.14}})
    return {'ground': ground, 'walls': walls, 'rocks': rocks}


def navigation_settings():
    """Chosen by sweeping ~39k start/end pairs over floor, ramps and mesa with the
    controller's own query (snap 2 m, 2048-visit budget): zero errors, worst case
    1319 visits and 41 route points. Finer 0.2 m cells or 6.4 m tiles produced
    `smoothed navigation segment left its polygon` failures on the ramps."""
    return {'min': [PLAY_X[0] - 1.2, FLOOR_Y - 0.5, PLAY_Z[0] - 0.4],
            'max': [PLAY_X[1] + 1.2, UPPER_Y + 2.6, PLAY_Z[1] + 0.4],
            'cell_size': 0.25, 'cell_height': 0.1, 'tile_cells': 128, 'agent_radius': 0.35,
            'agent_height': 1.2, 'max_climb': 0.25, 'max_slope_degrees': 30.0}
