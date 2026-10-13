"""Seeded, deterministic lattice noise for procedural art (no global random state).

All functions are pure: equal arguments give equal results on every run. Values
are roughly in -1..1 (gradient noise) or 0..1 (hash/value helpers).
"""
import math


def hash32(*values):
    """Integer hash of a tuple of ints (32-bit avalanche, platform independent)."""
    h = 0x811C9DC5
    for v in values:
        v &= 0xFFFFFFFF
        for shift in (0, 8, 16, 24):
            h ^= (v >> shift) & 0xFF
            h = (h * 0x01000193) & 0xFFFFFFFF
    h ^= h >> 16
    h = (h * 0x7FEB352D) & 0xFFFFFFFF
    h ^= h >> 15
    h = (h * 0x846CA68B) & 0xFFFFFFFF
    h ^= h >> 16
    return h


def unit(*values):
    """Hash to a float in [0, 1)."""
    return hash32(*values) / 4294967296.0


def _fade(t):
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0)


def _grad2(seed, ix, iy):
    angle = unit(seed, ix, iy) * math.tau
    return math.cos(angle), math.sin(angle)


def perlin2(x, y, seed=0, period=None):
    """2D gradient noise; optional integer period (px, py) makes it tile."""
    x0, y0 = math.floor(x), math.floor(y)
    fx, fy = x - x0, y - y0
    u, v = _fade(fx), _fade(fy)

    def corner(ix, iy, dx, dy):
        if period:
            ix %= period[0]
            iy %= period[1]
        gx, gy = _grad2(seed, ix, iy)
        return gx * dx + gy * dy

    n00 = corner(x0, y0, fx, fy)
    n10 = corner(x0 + 1, y0, fx - 1, fy)
    n01 = corner(x0, y0 + 1, fx, fy - 1)
    n11 = corner(x0 + 1, y0 + 1, fx - 1, fy - 1)
    a = n00 + u * (n10 - n00)
    b = n01 + u * (n11 - n01)
    return (a + v * (b - a)) * 1.41421356


def fbm2(x, y, seed=0, octaves=4, lacunarity=2.0, gain=0.5, period=None):
    total, amplitude, norm = 0.0, 1.0, 0.0
    for octave in range(octaves):
        p = None if period is None else (period[0] * int(lacunarity ** octave), period[1] * int(lacunarity ** octave))
        total += amplitude * perlin2(x, y, seed + octave * 101, p)
        norm += amplitude
        x *= lacunarity
        y *= lacunarity
        amplitude *= gain
    return total / norm


def _grad3(seed, ix, iy, iz):
    z = unit(seed, ix, iy, iz) * 2.0 - 1.0
    angle = unit(seed + 7, ix, iy, iz) * math.tau
    r = math.sqrt(max(0.0, 1.0 - z * z))
    return r * math.cos(angle), r * math.sin(angle), z


def perlin3(x, y, z, seed=0):
    x0, y0, z0 = math.floor(x), math.floor(y), math.floor(z)
    fx, fy, fz = x - x0, y - y0, z - z0
    u, v, w = _fade(fx), _fade(fy), _fade(fz)
    total = 0.0
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                gx, gy, gz = _grad3(seed, x0 + dx, y0 + dy, z0 + dz)
                dot = gx * (fx - dx) + gy * (fy - dy) + gz * (fz - dz)
                weight = (u if dx else 1 - u) * (v if dy else 1 - v) * (w if dz else 1 - w)
                total += weight * dot
    return total * 1.5


def fbm3(x, y, z, seed=0, octaves=4, lacunarity=2.0, gain=0.5):
    total, amplitude, norm = 0.0, 1.0, 0.0
    for octave in range(octaves):
        total += amplitude * perlin3(x, y, z, seed + octave * 131)
        norm += amplitude
        x, y, z = x * lacunarity, y * lacunarity, z * lacunarity
        amplitude *= gain
    return total / norm


def smoothstep(edge0, edge1, x):
    if edge0 == edge1:
        return 0.0 if x < edge0 else 1.0
    t = max(0.0, min(1.0, (x - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)


def lerp(a, b, t):
    return a + (b - a) * t


def mix3(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t)
