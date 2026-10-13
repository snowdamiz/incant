"""Bounded CPU model of the 0070 SSAO quality repair (design aid, stdlib only).

Mirrors ao.wgsl's single-layer test, the eeeac1d4 design (Bayer frame rotation,
step jitter, plane-weighted 5-tap H/V filter with R8 rounding) and the analytic
rectangle fixtures (tools/ssao_reference.py, Astra's quality_geometry.rs):
321x193 target, fractional viewport (13.25, 17.5, 280.5, 160.5), 1 rad
perspective or half-height-4 orthographic camera. Planar fixtures only; every
receiver normal is +Z, so shading and geometric normals coincide. f64 model with
the exact f32 uniform inputs; not a GPU rounding oracle. One full raw grid per
configuration (no per-pixel re-render).
"""

import math
import struct

SIZE = (321, 193)
RECT = (13.25, 17.5, 280.5, 160.5)
BIAS = 0.01
RADIUS = 2.0
TAN = math.tan(0.5)
BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))
TAPS = (0.5, 1.0, 1.0, 1.0, 0.5)


def bayer(order):
    """Recursive Bayer matrix: B_2n = [[4B, 4B+2], [4B+3, 4B+1]]; bayer(4) == BAYER."""
    b = [[0]]
    while len(b) < order:
        n = len(b)
        b = [[4 * b[y % n][x % n] + ((0, 2), (3, 1))[y // n][x // n] for x in range(2 * n)] for y in range(2 * n)]
    return tuple(tuple(r) for r in b)


def taps(order):
    """Symmetric window giving every residue mod `order` total weight 1."""
    return (0.5,) + (1.0,) * (order - 1) + (0.5,)


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def directions():
    """math.rs directions(): f32 strata, bit-reversed azimuths."""
    out = []
    for i in range(16):
        u = f32((i + 0.5) / 16)
        angle = math.tau * int(f"{i:032b}"[::-1], 2) / 4294967296.0
        root = f32(math.sqrt(u))
        out.append((f32(root * math.cos(angle)), f32(root * math.sin(angle)), f32(math.sqrt(f32(1.0 - u)))))
    return out


def rotations(strata=16):
    """(cos, sin) of alpha = (2 pi / 16) (b + 0.5) / strata, rounded to f32."""
    return [(f32(math.cos(math.tau / 16 * (b + 0.5) / strata)),
             f32(math.sin(math.tau / 16 * (b + 0.5) / strata))) for b in range(strata)]


DIRECTIONS = directions()
ROTATIONS = rotations()


def r8(value):
    """Unorm8 store/load: round half away from zero, as Rust f64::round."""
    return math.floor(value * 255.0 + 0.5) / 255.0


class Camera:
    def __init__(self, orthographic, rect=RECT):
        self.orthographic = orthographic
        self.x, self.y, self.w, self.h = rect
        self.aspect = self.w / self.h

    def ray(self, px, py):
        nx = (px - self.x) / self.w * 2 - 1
        ny = (py - self.y) / self.h * 2 - 1
        if self.orthographic:
            return (nx * 4 * self.aspect, -ny * 4, 0.0), (0.0, 0.0, -1.0)
        return (0.0, 0.0, 0.0), (nx * TAN * self.aspect, -ny * TAN, -1.0)

    def project(self, qx, qy, qz):
        if not (-qz >= 0.1 and -qz < 100.0):
            return None
        if self.orthographic:
            nx, ny = qx / (4 * self.aspect), qy / 4
        else:
            nx, ny = qx / (-qz * TAN * self.aspect), qy / (-qz * TAN)
        sx = self.x + (nx * 0.5 + 0.5) * self.w
        sy = self.y + (-ny * 0.5 + 0.5) * self.h
        if self.x <= sx < self.x + self.w and self.y <= sy < self.y + self.h:
            return sx, sy
        return None


RECEIVER = ((-8.0, 8.0), (-6.0, 6.0), -4.0)
SLAB = ((0.4, 3.0), (-1.5, 1.5), -3.0)


def fixture_rects(name):
    """(x-range, y-range, z) per owner; owner 0 is always the receiver."""
    if name == "flat":
        return [RECEIVER]
    if name == "white_slab":
        return [RECEIVER, SLAB]
    if name == "edge_slab":
        return [RECEIVER, ((2.2, 6.0), SLAB[1], -3.0)]
    if name.startswith("stripes:"):
        # stripes:x0,y0,y1 - four 0.1-wide stripes at z = -3, pitch 0.3.
        x0, y0, y1 = (float(v) for v in name.split(":")[1].split(","))
        return [RECEIVER] + [((x0 + 0.3 * i, x0 + 0.3 * i + 0.1), (y0, y1), -3.0) for i in range(4)]
    raise KeyError(name)


class Grid:
    """First analytic surface at every texel centre inside the viewport."""

    def __init__(self, name, orthographic):
        self.name, self.camera = name, Camera(orthographic)
        self.w, self.h = SIZE
        rects = fixture_rects(name)
        self.hits = []
        for y in range(self.h):
            for x in range(self.w):
                cx, cy = x + 0.5, y + 0.5
                c = self.camera
                if not (c.x <= cx < c.x + c.w and c.y <= cy < c.y + c.h):
                    self.hits.append(None)
                    continue
                o, d = c.ray(cx, cy)
                best = None
                for owner, ((x0, x1), (y0, y1), z) in enumerate(rects):
                    t = (z - o[2]) / d[2]
                    px, py = o[0] + d[0] * t, o[1] + d[1] * t
                    if -100.0 < z <= -0.1 and x0 <= px < x1 and y0 <= py < y1:
                        if best is None or z > best[2]:
                            best = (px, py, z, owner)
                self.hits.append(best)

    def at(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.hits[y * self.w + x]
        return None

    def interior(self, x, y, band=2):
        """Pixel and its Chebyshev-`band` neighbourhood all hit the same owner."""
        own = self.at(x, y)
        if own is None:
            return False
        for dy in range(-band, band + 1):
            for dx in range(-band, band + 1):
                q = self.at(x + dx, y + dy)
                if q is None or q[3] != own[3]:
                    return False
        return True


def raw(grid, rotate=True, jitter=True, rotation_table=None, pixels=None, steps_override=None, full_stratum=False,
        steps=4, tile=4):
    """Raw accessibility 1 - k/16 per texel (1 for background/outside).

    rotation_table: per-pixel (cos, sin) source, default the 16-entry Bayer table.
    steps_override: optional callable (x, y) -> t for jitter experiments.
    pixels: optional set of texel indices to evaluate (others stay None).
    full_stratum: jitter t = (b + 0.5)/16 over the whole step stratum instead
    of the eeeac1d4 upper half t = 0.5 + 0.5 (b + 0.5)/16.
    steps/tile: probes per ray and Bayer order (tile**2 rotation/jitter strata).
    """
    order = bayer(tile)
    strata = tile * tile
    table = (ROTATIONS if tile == 4 else rotations(strata)) if rotation_table is None else rotation_table
    w, project, hits = grid.w, grid.camera.project, grid.hits
    r2 = RADIUS * RADIUS
    out = [None] * len(hits)
    for index, hit in enumerate(hits):
        if pixels is not None and index not in pixels:
            continue
        y, x = divmod(index, w)
        if hit is None:
            out[index] = 1.0
            continue
        px, py, pz = hit[0], hit[1], hit[2]
        if rotate:
            cos, sin = table(x, y) if callable(table) else table[order[y % tile][x % tile]]
        else:
            cos, sin = 1.0, 0.0
        if steps_override is not None:
            t = steps_override(x, y)
        else:
            b = order[x % tile][y % tile]
            t = ((b + 0.5) / strata if full_stratum else 0.5 + 0.5 * (b + 0.5) / strata) if jitter else 1.0
        count = 0
        for kx, ky, kz in DIRECTIONS:
            dx = kx * cos - ky * sin  # +Z receiver: tangent X, bitangent Y
            dy = kx * sin + ky * cos
            for j in range(steps):
                s = RADIUS * (j + t) / steps
                qx, qy, qz = px + dx * s, py + dy * s, pz + kz * s
                at = project(qx, qy, qz)
                if at is None:
                    continue
                sx, sy = int(math.floor(at[0])), int(math.floor(at[1]))
                if not (0 <= sx < w and 0 <= sy < grid.h):
                    continue
                surface = hits[sy * w + sx]
                if surface is None:
                    continue
                ex, ey, ez = surface[0] - px, surface[1] - py, surface[2] - pz
                if surface[2] >= qz + BIAS and ez > BIAS and ex * ex + ey * ey + ez * ez <= r2:
                    count += 1
                    break
        out[index] = 1.0 - count / 16
    return out


def filter_pass(grid, values, horizontal, taps=TAPS, plane=True, quantize=True, pixels=None):
    w, hits = grid.w, grid.hits
    out = [None] * len(values)
    half = len(taps) // 2
    for index, hit in enumerate(hits):
        if pixels is not None and index not in pixels:
            continue
        y, x = divmod(index, w)
        if hit is None:
            out[index] = 1.0
            continue
        tau = max(BIAS, -hit[2] * 0.01)
        total = mass = 0.0
        for tap, c in enumerate(taps):
            offset = tap - half
            qx, qy = (x + offset, y) if horizontal else (x, y + offset)
            q = grid.at(qx, qy)
            if q is None or values[qy * w + qx] is None:
                continue
            weight = c * (min(1.0, max(0.0, 1.0 - abs(q[2] - hit[2]) / tau)) if plane else 1.0)
            mass += weight
            total += weight * values[qy * w + qx]
        value = min(1.0, max(0.0, total / mass))
        out[index] = r8(value) if quantize else value
    return out


def filtered(grid, values, taps=TAPS, plane=True, quantize=True, pixels=None):
    """R8 input, H pass then V pass. pixels: output texel indices wanted; the
    H pass then runs on their vertical +-2 neighbourhood."""
    q = [None if v is None else (r8(v) if quantize else v) for v in values]
    half = len(taps) // 2
    h_pixels = None if pixels is None else {i + d * grid.w for i in pixels for d in range(-half, half + 1)}
    horizontal = filter_pass(grid, q, True, taps, plane, quantize, h_pixels)
    return filter_pass(grid, horizontal, False, taps, plane, quantize, pixels)


def support(grid, pixels, half=2):
    """Raw texels needed to filter `pixels`: the (2 half + 1)^2 neighbourhood."""
    return {i + dy * grid.w + dx for i in pixels for dy in range(-half, half + 1) for dx in range(-half, half + 1)
            if 0 <= i % grid.w + dx < grid.w and 0 <= i // grid.w + dy < grid.h}


def converged(grid, pixels, strata=(32, 16), full_stratum=False, steps=4):
    """Expectation of the repaired raw estimator over alpha uniform in one
    azimuth stratum [0, 2 pi / 16) and t uniform in [0.5, 1) (or [0, 1) with
    full_stratum): midpoint rule with strata = (alpha strata, t strata),
    continuous (no f32 table rounding). Independent of the Bayer tile size."""
    m_alpha, n_t = strata
    total = [0.0 if i in pixels else None for i in range(len(grid.hits))]
    for a in range(m_alpha):
        alpha = math.tau / 16 * (a + 0.5) / m_alpha
        cs = (math.cos(alpha), math.sin(alpha))
        for b in range(n_t):
            t = (b + 0.5) / n_t if full_stratum else 0.5 + 0.5 * (b + 0.5) / n_t
            values = raw(grid, rotation_table=lambda x, y: cs, pixels=pixels, steps_override=lambda x, y: t,
                         steps=steps)
            for i in pixels:
                total[i] += values[i]
    return [None if v is None else v / (m_alpha * n_t) for v in total]


def n7_steps(grid, values, band=2):
    """Astra's N7: pixel and both forward neighbours Chebyshev-`band` interior;
    count the pixel when either forward difference is >= 1/16."""
    w, count = grid.w, 0
    inside = [[grid.interior(x, y, band) for x in range(grid.w)] for y in range(grid.h)]
    for y in range(grid.h - 1):
        for x in range(grid.w - 1):
            if inside[y][x] and inside[y][x + 1] and inside[y + 1][x]:
                i = y * w + x
                if max(abs(values[i] - values[i + 1]), abs(values[i] - values[i + w])) >= 1 / 16:
                    count += 1
    return count


def n6_row(grid, values, row=97, x0=100, x1=171, band=2):
    """Astra's N6: largest |step| over adjacent pairs (x, x+1), x in [x0, x1),
    both Chebyshev-`band` interior. Returns (max, x of the max pair)."""
    best, where = 0.0, None
    for x in range(x0, x1):
        if grid.interior(x, row, band) and grid.interior(x + 1, row, band):
            step = abs(values[row * grid.w + x + 1] - values[row * grid.w + x])
            if step > best:
                best, where = step, x
    return best, where


def continuous_directions(n_u=24, n_phi=64):
    """Cosine-weighted product grid: u = sin^2(theta) and phi midpoints."""
    return [(math.sqrt((i + 0.5) / n_u) * math.cos(math.tau * (j + 0.5) / n_phi),
             math.sqrt((i + 0.5) / n_u) * math.sin(math.tau * (j + 0.5) / n_phi),
             math.sqrt(1 - (i + 0.5) / n_u)) for i in range(n_u) for j in range(n_phi)]


def raw_directions(grid, pixels, directions_list, fractions):
    """Same single-layer test with explicit +Z-frame directions and step
    fractions of the radius; returns {texel index: 1 - blocked/len(directions)}."""
    w, project, hits, r2 = grid.w, grid.camera.project, grid.hits, RADIUS * RADIUS
    out = {}
    for index in pixels:
        hit = hits[index]
        if hit is None:
            out[index] = 1.0
            continue
        px, py, pz = hit[0], hit[1], hit[2]
        count = 0
        for dx, dy, dz in directions_list:
            for f in fractions:
                s = RADIUS * f
                qx, qy, qz = px + dx * s, py + dy * s, pz + dz * s
                at = project(qx, qy, qz)
                if at is None:
                    continue
                sx, sy = int(math.floor(at[0])), int(math.floor(at[1]))
                if not (0 <= sx < w and 0 <= sy < grid.h):
                    continue
                surface = hits[sy * w + sx]
                if surface is None:
                    continue
                ex, ey, ez = surface[0] - px, surface[1] - py, surface[2] - pz
                if surface[2] >= qz + BIAS and ez > BIAS and ex * ex + ey * ey + ez * ez <= r2:
                    count += 1
                    break
        out[index] = 1.0 - count / len(directions_list)
    return out

