"""Late-afternoon sky: one equirectangular radiance image drives both the visible
emissive dome and the EnvironmentLight, so reflections, ambient light and the
backdrop agree. The renderer has no sky pass and a dark display backdrop, so the
dome is real geometry (inside-facing, unlit-equivalent: black base, emissive).

Layout follows the engine's environment decoder: u = atan2(z, x) / 2pi + 0.5
(+X at the centre), v = acos(y) / pi (north pole on the first row).
The upper sky behind the HUD band stays a deep, even blue for text contrast;
warmth lives near the horizon and around the sun in the west-south-west.
"""
import math

from . import gltf, noise as N, pngio, raster as R
from .mesh import Mesh

SUN_AZIMUTH = 212.0       # degrees clockwise from north (-Z); east is +X
SUN_ELEVATION = 38.0
DOME_RADIUS = 440.0


def sun_direction():
    a, e = math.radians(SUN_AZIMUTH), math.radians(SUN_ELEVATION)
    return (math.sin(a) * math.cos(e), math.sin(e), -math.cos(a) * math.cos(e))


def _mesas(azimuth, seed=1401):
    """Distant mesa skyline: elevation (degrees) of the far horizon at an azimuth."""
    h = 0.0
    for k in range(9):
        centre = 360.0 * N.unit(seed, k, 1)
        width = 9.0 + 26.0 * N.unit(seed, k, 2)
        top = 0.6 + 2.6 * N.unit(seed, k, 3)
        d = abs((azimuth - centre + 180.0) % 360.0 - 180.0)
        shoulder = N.smoothstep(width, width * 0.72, d)
        h = max(h, top * shoulder)
    return h + 0.35 + 0.18 * N.fbm2(azimuth / 9.0, 0.0, seed, 3)


def radiance(d):
    """Linear RGB radiance seen along unit world direction d."""
    x, y, z = d
    elevation = math.degrees(math.asin(max(-1.0, min(1.0, y))))
    azimuth = math.degrees(math.atan2(x, -z)) % 360.0
    sun = sun_direction()
    cos_sun = max(-1.0, min(1.0, x * sun[0] + y * sun[1] + z * sun[2]))
    angle_sun = math.degrees(math.acos(cos_sun))
    toward_sun = 0.5 + 0.5 * math.cos(math.radians(azimuth - SUN_AZIMUTH))
    # Deep, even blue above ~6 degrees (the HUD band), a short warm haze below.
    zenith, band, low = R.hex_linear('#284379'), R.hex_linear('#3A5893'), R.hex_linear('#6578A6')
    haze = N.mix3(R.hex_linear('#B9A3A6'), R.hex_linear('#E6C29E'), toward_sun ** 1.5)
    if elevation >= 0.0:
        sky = N.mix3(band, zenith, N.smoothstep(14.0, 70.0, elevation))
        sky = N.mix3(low, sky, N.smoothstep(1.0, 7.0, elevation))
        sky = N.mix3(haze, sky, N.smoothstep(0.0, 1.8, elevation) ** 0.9)
        cloud = N.fbm2(azimuth / 18.0, elevation / 1.6, 1409, 4)
        cover = N.smoothstep(0.2, 0.55, cloud) * N.smoothstep(0.8, 2.0, elevation) * \
            (1.0 - N.smoothstep(3.0, 5.0, elevation))
        lit = N.mix3(R.hex_linear('#C2A7B2'), R.hex_linear('#F3D0B0'), toward_sun)
        sky = N.mix3(sky, lit, 0.5 * cover)
        glow = math.exp(-(angle_sun / 26.0) ** 2) * 0.45 + math.exp(-(angle_sun / 6.0) ** 2) * 0.6
        sky = (sky[0] + glow * 1.0, sky[1] + glow * 0.8, sky[2] + glow * 0.58)
        if angle_sun < 1.2:
            sky = (1.0, 0.97, 0.9)
        far = _mesas(azimuth)
        if elevation < far:
            mesa = N.mix3(R.hex_linear('#7E6A7C'), R.hex_linear('#B08C82'), toward_sun)
            face = 0.9 + 0.1 * N.smoothstep(far - 0.4, far, elevation)
            sky = N.mix3(sky, tuple(c * face for c in mesa), 0.8)
    else:
        # A deep valley beyond the rim: hazy near the horizon, warmer and darker below.
        t = min(1.0, -elevation / 25.0)
        mott = 0.5 + 0.5 * N.fbm2(azimuth / 7.0, elevation / 1.5, 1411, 4)
        valley = N.mix3(R.hex_linear('#9C889C'), R.hex_linear('#6A4A44'), N.smoothstep(0.0, 0.8, t))
        valley = tuple(c * (0.88 + 0.2 * mott) for c in valley)
        sky = N.mix3(N.mix3(haze, R.hex_linear('#93768A'), 0.6), valley, N.smoothstep(0.0, 0.25, t))
    return tuple(min(1.0, max(0.0, c)) for c in sky)


def image(width, height):
    rows = []
    for row in range(height):
        theta = (row + 0.5) / height * math.pi
        line = []
        for col in range(width):
            phi = ((col + 0.5) / width - 0.5) * math.tau
            d = (math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi))
            line.append(radiance(d))
        rows.append(line)
    return rows


def dome(segments=48, rings=24):
    """Inside-facing sphere with UVs matching the equirectangular layout."""
    mesh = Mesh()
    grid = []
    for j in range(rings + 1):
        theta = j / rings * math.pi
        row = []
        for i in range(segments + 1):
            u = i / segments
            phi = (u - 0.5) * math.tau
            d = (math.sin(theta) * math.cos(phi), math.cos(theta), math.sin(theta) * math.sin(phi))
            row.append(mesh.vertex(tuple(c * DOME_RADIUS for c in d), tuple(-c for c in d), (u, j / rings)))
        grid.append(row)
    for j in range(rings):
        for i in range(segments):
            a, b, c, d = grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i]
            centre = mesh.positions[a]
            inward = tuple(-v for v in mesh.positions[c])
            mesh.quad(a, b, c, d, inward)
    return mesh


def build(out):
    sky_rows = image(1024, 512)
    pngio.write(out / 'sky_dome.png', 1024, 512, 3, R.to_rows(sky_rows))
    env_rows = image(512, 256)
    pngio.write(out / 'sky_environment.png', 512, 256, 3, R.to_rows(env_rows))
    material = gltf.Material('Sky', base=(0.0, 0.0, 0.0, 1.0), roughness=1.0, emissive=(1.0, 1.0, 1.0),
                             emissive_texture='sky_dome.png', sampler='clamp')
    mesh = dome()
    writer = gltf.Writer()
    writer.add_root(gltf.Node('Sky dome', [(mesh, material)]))
    writer.save(out, 'sky')
    return gltf.stats([(mesh, material)])
