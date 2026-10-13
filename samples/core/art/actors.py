"""Gameplay actors: the surveyor (player), the ember mites (chasers) and the
sample cores (pickups). Each model's origin is its collider centre, so visuals
sit on the capsule/sensor without child offsets.

The controller never rotates bodies, so the chasers are radially symmetric and
the surveyor is designed to be read from behind: the third-person camera sees
its safety-yellow sample canister, antenna and hover ring. Looping idle clips
(bob, sway, orbit, spin) are rigid node animation only; they never move a
collider.
"""
import math

from . import gltf, level, noise as N, pngio, raster as R
from .mesh import Mesh, beam, cylinder, lathe, normalize, quat_axis, rock, rounded_box

WHITE = '#ECE8DF'
YELLOW = '#E2AE34'


def _loop(times, values):
    return [float(t) for t in times], values


def _bob(node, amplitude, period, base=(0.0, 0.0, 0.0), steps=8):
    times = [period * k / steps for k in range(steps + 1)]
    values = [(base[0], base[1] + amplitude * math.sin(math.tau * k / steps), base[2]) for k in range(steps + 1)]
    return (node, 'translation', times, values)


def _spin(node, period, axis=(0, 1, 0), steps=8, swing=None):
    times = [period * k / steps for k in range(steps + 1)]
    if swing is None:
        values = [quat_axis(axis, -360.0 * k / steps) for k in range(steps + 1)]
    else:
        values = [quat_axis(axis, swing * math.sin(math.tau * k / steps)) for k in range(steps + 1)]
    return (node, 'rotation', times, values)


# ---------------------------------------------------------------- surveyor

def surveyor(out):
    white = gltf.Material('Surveyor shell', base=R.hex_linear(WHITE) + (1.0,), roughness=0.42)
    visor = gltf.Material('Visor glass', base=R.hex_linear('#16202C') + (1.0,), metallic=0.3, roughness=0.14)
    yellow = gltf.Material('Sample canister', base=R.hex_linear(YELLOW) + (1.0,), roughness=0.5)
    dark = gltf.Material('Rubber and steel', base=R.hex_linear('#2F3339') + (1.0,), metallic=0.4, roughness=0.55)
    glow = gltf.Material('Thruster glow', base=(0.0, 0.0, 0.0, 1.0), roughness=0.5, emissive=(1.0, 0.8, 0.42))
    shell_profile = [(0.0, -0.44), (0.16, -0.43), (0.25, -0.38), (0.30, -0.26), (0.315, -0.08), (0.305, 0.08),
                     (0.27, 0.24), (0.2, 0.36), (0.1, 0.42), (0.0, 0.44)]
    shell = lathe(shell_profile, 28)
    # Visor: a dark band wrapping the front (-Z) of the shell.
    visor_profile = [(0.29, 0.06), (0.305, 0.12), (0.3, 0.2), (0.272, 0.27)]
    band = lathe([(r + 0.012, y) for r, y in visor_profile], 14, phase=math.radians(-152), arc=math.radians(124))
    canister = cylinder(0.105, -0.2, 0.2, 16).transformed((0.0, 0.0, 0.27))
    cap = cylinder(0.112, 0.2, 0.25, 16, radius_top=0.09).transformed((0.0, 0.0, 0.27))
    cap.merge(cylinder(0.112, -0.25, -0.2, 16, radius_top=0.112).transformed((0.0, 0.0, 0.27)))
    strap = Mesh()
    for y in (-0.12, 0.12):
        strap.merge(lathe([(0.312, y - 0.02), (0.318, y), (0.312, y + 0.02)], 20, phase=math.radians(40),
                          arc=math.radians(100)))
    seam = lathe([(0.311, -0.165), (0.318, -0.15), (0.311, -0.135)], 28)
    pods = Mesh()
    for side in (-1, 1):
        pod = rounded_box((0.05, 0.09, 0.08), exponent=0.25, divisions=3)
        pods.merge(pod.transformed((side * 0.315, -0.02, 0.02)))
    body = gltf.Node('Body', [(shell, white), (band, visor), (canister, yellow),
                              (cap.merge(strap).merge(pods).merge(seam), dark)],
                     rotation=quat_axis((1, 0, 0), -5.0))
    rod = cylinder(0.012, 0.0, 0.28, 6)
    tip = lathe([(0.0, 0.27), (0.03, 0.29), (0.03, 0.32), (0.0, 0.34)], 10)
    antenna = gltf.Node('Antenna', [(rod, dark), (tip, glow)], translation=(0.08, 0.36, 0.1))
    body.children.append(antenna)
    ring = lathe([(0.15, -0.5), (0.24, -0.5), (0.24, -0.46), (0.15, -0.46), (0.15, -0.5)], 24)
    ring_glow = lathe([(0.0, -0.505), (0.2, -0.505)], 24)
    thruster = gltf.Node('Thruster', [(ring, dark), (ring_glow, glow)])
    root = gltf.Node('Surveyor', children=[body, thruster])
    writer = gltf.Writer()
    writer.add_root(root)
    writer.add_animation('Idle hover', [_bob('Body', 0.022, 1.6), _spin('Antenna', 2.4, (1, 0, 0), swing=7.0)])
    writer.save(out, 'surveyor')
    return gltf.stats([(shell, white), (band, visor), (canister, yellow), (cap, dark), (rod, dark), (tip, glow),
                       (ring, dark), (ring_glow, glow)])


# ---------------------------------------------------------------- ember mite

def ember_texture(size=128, seed=1201):
    """Albedo + emissive for the mite crust: dark basalt split by glowing seams."""
    from .terrain import _cells
    base_a, base_b = R.hex_linear('#2D2723'), R.hex_linear('#3B322C')
    albedo, emissive = [], []
    for row in range(size):
        a_line, e_line = [], []
        for col in range(size):
            u, v = (col + 0.5) / size, (row + 0.5) / size
            d1, d2 = _cells(u * 5.0, v * 5.0, seed)
            seam = 1.0 - N.smoothstep(0.0, 0.07, d2 - d1)
            tone = N.unit(seed, col // 4, row // 4)
            albedo_c = N.mix3(base_a, base_b, tone)
            a_line.append(N.mix3(albedo_c, (0.05, 0.02, 0.01), seam))
            heat = seam * (0.7 + 0.3 * N.unit(seed + 3, col // 2, row // 2))
            e_line.append((heat * 1.0, heat * 0.36, heat * 0.08))
        albedo.append(a_line)
        emissive.append(e_line)
    return albedo, emissive


def mite(out):
    albedo, emissive = ember_texture()
    pngio.write(out / 'mite_albedo.png', 128, 128, 3, R.to_rows(albedo))
    pngio.write(out / 'mite_emissive.png', 128, 128, 3, R.to_rows(emissive))
    crust = gltf.Material('Basalt crust', roughness=0.75, base_texture='mite_albedo.png', emissive=(1.0, 1.0, 1.0),
                          emissive_texture='mite_emissive.png')
    glow = gltf.Material('Ember band', base=(0.05, 0.02, 0.01, 1.0), roughness=0.5, emissive=(1.0, 0.38, 0.09))
    shard_m = gltf.Material('Obsidian shard', base=R.hex_linear('#1E1B1C') + (1.0,), metallic=0.2, roughness=0.2)
    core = rock(0.31, (1.0, 0.92, 1.0), 0.16, 5, 2, uv_scale=1.4)
    band = lathe([(0.29, -0.035), (0.335, -0.012), (0.335, 0.012), (0.29, 0.035)], 28)
    body = gltf.Node('Core', [(core, crust), (band, glow)], translation=(0.0, 0.05, 0.0))
    shards = Mesh()
    for k in range(3):
        a = math.tau * k / 3
        s = rock(0.07, (0.55, 1.9, 0.55), 0.1, 30 + k, 0).flat()
        tilt = quat_axis((math.cos(a + 1.5), 0, math.sin(a + 1.5)), 25)
        shards.merge(s.transformed((0.52 * math.cos(a), 0.06 * (k - 1), 0.52 * math.sin(a)), tilt))
    orbit = gltf.Node('Orbit', [(shards, shard_m)], translation=(0.0, 0.05, 0.0))
    root = gltf.Node('Ember mite', children=[body, orbit])
    writer = gltf.Writer()
    writer.add_root(root)
    writer.add_animation('Hover', [_bob('Core', 0.045, 1.2, base=(0.0, 0.05, 0.0)), _spin('Orbit', 3.0)])
    writer.save(out, 'mite')
    return gltf.stats([(core, crust), (band, glow), (shards, shard_m)])


# ---------------------------------------------------------------- sample core

def core_textures(size=128, seed=1301):
    """Banded rock core with cyan mineral veins (albedo + emissive)."""
    bands = ['#6E4636', '#9A6A4E', '#5B3A2E', '#B08562', '#74503E', '#4E3329']
    albedo, emissive = [], []
    for row in range(size):
        v = (row + 0.5) / size
        a_line, e_line = [], []
        for col in range(size):
            u = (col + 0.5) / size
            wob = 0.03 * N.perlin2(u * 4, v * 2, seed, (4, 2))
            k = int((v + wob) * 9) % len(bands)
            base = R.hex_linear(bands[k])
            vein = abs(((v + wob) * 9) % 1.0 - 0.5)
            lit = 1.0 - N.smoothstep(0.03, 0.11, vein)
            lit *= 0.65 + 0.35 * N.smoothstep(-0.3, 0.3, N.perlin2(u * 8, v * 8, seed + 1, (8, 8)))
            a_line.append(N.mix3(base, (0.1, 0.55, 0.55), lit * 0.6))
            e_line.append((0.12 * lit, 0.92 * lit, 0.88 * lit))
        albedo.append(a_line)
        emissive.append(e_line)
    return albedo, emissive


def sample_core(out):
    albedo, emissive = core_textures()
    pngio.write(out / 'core_albedo.png', 128, 128, 3, R.to_rows(albedo))
    pngio.write(out / 'core_emissive.png', 128, 128, 3, R.to_rows(emissive))
    rock_m = gltf.Material('Core sample', roughness=0.6, base_texture='core_albedo.png', emissive=(1.0, 1.0, 1.0),
                           emissive_texture='core_emissive.png')
    steel = gltf.Material('Collar steel', base=R.hex_linear('#9AA0A8') + (1.0,), metallic=0.9, roughness=0.32)
    core = lathe([(0.0, -0.27), (0.135, -0.27), (0.135, 0.27), (0.0, 0.27)], 18, v_scale=1.0 / 0.54)
    collars = Mesh()
    for y in (-0.27, 0.245):
        collars.merge(cylinder(0.152, y - 0.02, y + 0.045, 18))
    spin = gltf.Node('Spin', [(core, rock_m), (collars, steel)])
    root = gltf.Node('Sample core', children=[spin])
    writer = gltf.Writer()
    writer.add_root(root)
    writer.add_animation('Float', [_bob('Spin', 0.035, 2.2), _spin('Spin', 5.0)])
    writer.save(out, 'sample')
    return gltf.stats([(core, rock_m), (collars, steel)])
