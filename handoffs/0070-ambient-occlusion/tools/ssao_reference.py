#!/usr/bin/env python3
"""Independent CPU reference for the fixed 16x4 screen-space accessibility.

Standard library only; written from the documented contract and ao.wgsl, not
from the engine's Rust code, and run in f64. Surfaces are analytic view-space
rectangles, so the depth/normal "attachments" at texel centres are exact ray
intersections. Every decision the shader makes (clip tests, viewport tests,
texel selection, silhouettes, the three blocking comparisons) is evaluated with
a margin; a ray whose outcome depends on a decision closer than the tolerance is
"ambiguous". A pixel's blocked-ray count is then an exact integer interval
[k_min, k_max]; an implementation agrees when its inferred interval meets it.

  python3 -I -B ssao_reference.py --selftest
"""

import math
import struct
import sys

DIRECTIONS_COUNT = 16
STEPS = 4
POSITION_TOLERANCE = 2e-3  # view units; bias is 1e-2, f32 reconstruction ~1e-5
PIXEL_TOLERANCE = 4e-3     # pixels; f32 projection error ~1e-4


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


def directions():
    """Cosine-weighted equal-probability strata, bit-reversed angles (math.rs)."""
    out = []
    for i in range(DIRECTIONS_COUNT):
        u = f32((i + 0.5) / DIRECTIONS_COUNT)
        v = int(f"{i:032b}"[::-1], 2) / 4294967296.0
        phi = 2 * math.pi * v
        s = f32(math.sqrt(u))
        out.append((f32(s * math.cos(phi)), f32(s * math.sin(phi)), f32(math.sqrt(f32(1 - u)))))
    return out


DIRECTIONS = directions()


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def normalize(a):
    length = math.sqrt(dot(a, a))
    return (a[0] / length, a[1] / length, a[2] / length)


class Camera:
    """glam perspective_rh / orthographic_rh with WebGPU depth [0, 1]."""

    def __init__(self, orthographic, rect, fov=1.0, near=0.1, far=100.0, half_height=4.0):
        self.orthographic, self.rect = orthographic, rect
        self.near, self.far = near, far
        aspect = rect[2] / rect[3]
        if orthographic:
            self.hh, self.hw = half_height, half_height * aspect
        else:
            self.t = math.tan(fov / 2)
            self.h = 1 / self.t
            self.w = self.h / aspect
            self.aspect = aspect

    def clip(self, q):
        if self.orthographic:
            r = 1 / (self.near - self.far)
            return (q[0] / self.hw, q[1] / self.hh, r * q[2] + r * self.near, 1.0)
        r = self.far / (self.near - self.far)
        return (self.w * q[0], self.h * q[1], r * q[2] + r * self.near, -q[2])

    def ray(self, pixel):
        """Eye-ray origin and direction (z = -1) through a framebuffer position."""
        x, y, w, h = self.rect
        ndc = ((pixel[0] - x) / w * 2 - 1, 1 - (pixel[1] - y) / h * 2)
        if self.orthographic:
            return (ndc[0] * self.hw, ndc[1] * self.hh, 0.0), (0.0, 0.0, -1.0)
        return (0.0, 0.0, 0.0), (ndc[0] * self.t * self.aspect, ndc[1] * self.t, -1.0)


class Rect:
    """Axis-aligned view-space rectangle at depth z with the prepass normal."""

    def __init__(self, name, x, y, z, normal=(0.0, 0.0, 1.0)):
        self.name, self.x, self.y, self.z, self.normal = name, x, y, z, normal


class Scene:
    def __init__(self, camera, rects, edges=()):
        self.camera, self.rects = camera, rects
        # Extra interior coverage boundaries (e.g. a MASK texel edge at x).
        self.edges = edges
        self.cache = {}

    def _hit(self, pixel):
        origin, direction = self.camera.ray(pixel)
        best, best_z = None, -math.inf
        for rect in self.rects:
            t = (rect.z - origin[2]) / direction[2]
            if t <= 0:
                continue
            p = (origin[0] + t * direction[0], origin[1] + t * direction[1], rect.z)
            if rect.x[0] <= p[0] < rect.x[1] and rect.y[0] <= p[1] < rect.y[1] and rect.z > best_z:
                best, best_z = (rect, p), rect.z
        return best

    def candidates(self, at):
        """Possible (point, normal) surfaces at the texel centre, one entry when
        coverage is unambiguous; None stands for cleared depth (background)."""
        key = (at[0], at[1])
        if key not in self.cache:
            centre = (at[0] + 0.5, at[1] + 0.5)
            found = {}
            for dx, dy in ((0, 0), (PIXEL_TOLERANCE, 0), (-PIXEL_TOLERANCE, 0), (0, PIXEL_TOLERANCE),
                           (0, -PIXEL_TOLERANCE)):
                hit = self._hit((centre[0] + dx, centre[1] + dy))
                name = None if hit is None else hit[0].name
                if name not in found:
                    # Exact centre geometry for each surface that may own the texel.
                    found[name] = None if hit is None else (self._point(hit[0], centre), hit[0].normal)
            entries = list(found.values())
            centre_hit = self._hit(centre)
            if centre_hit is not None:
                for axis, value in self.edges:
                    if abs(centre_hit[1][axis] - value) < 1e-3:
                        entries.append(None)  # the cut-out side may be discarded
            self.cache[key] = entries
        return self.cache[key]

    def _point(self, rect, pixel):
        origin, direction = self.camera.ray(pixel)
        t = (rect.z - origin[2]) / direction[2]
        return (origin[0] + t * direction[0], origin[1] + t * direction[1], rect.z)


def inside(rect, p):
    return rect[0] <= p[0] < rect[0] + rect[2] and rect[1] <= p[1] < rect[1] + rect[3]


def near_bound(rect, p):
    return min(abs(p[0] - rect[0]), abs(p[0] - rect[0] - rect[2]),
               abs(p[1] - rect[1]), abs(p[1] - rect[1] - rect[3])) < PIXEL_TOLERANCE


def _condition(margin, strict):
    """Three-valued comparison margin > 0 (strict) or >= 0 with a tolerance band."""
    if margin > POSITION_TOLERANCE:
        return True
    if margin < -POSITION_TOLERANCE:
        return False
    return "?"


def _blocks(surface, p, n, q, radius, bias):
    if surface is None:
        return False
    delta = sub(surface, p)
    values = (_condition(surface[2] - (q[2] + bias), False),
              _condition(dot(delta, n) - bias, True),
              _condition(radius * radius - dot(delta, delta), False))
    if False in values:
        return False
    if "?" in values:
        return "?"
    return True


def _texels(probe):
    """Every texel the probe may land in when it sits on a texel boundary."""
    xs, ys = {math.floor(probe[0])}, {math.floor(probe[1])}
    for values, coordinate in ((xs, probe[0]), (ys, probe[1])):
        if coordinate - math.floor(coordinate) < PIXEL_TOLERANCE:
            values.add(math.floor(coordinate) - 1)
        if math.ceil(coordinate) - coordinate < PIXEL_TOLERANCE:
            values.add(math.floor(coordinate) + 1)
    return [(x, y) for x in xs for y in ys]


def _step(scene, p, n, q, radius, bias):
    """True (blocks), False (open, keep stepping) or "?" for one probe."""
    camera, rect = scene.camera, scene.camera.rect
    cw = camera.clip(q)
    clipped = cw[3] <= 0 or cw[2] < 0 or cw[2] >= cw[3]
    if min(abs(cw[3]), abs(cw[2]), abs(cw[2] - cw[3])) < 1e-9:
        return "?"
    if clipped:
        return False
    uv = (cw[0] / cw[3] * 0.5 + 0.5, cw[1] / cw[3] * -0.5 + 0.5)
    probe = (rect[0] + uv[0] * rect[2], rect[1] + uv[1] * rect[3])
    outcomes = set()
    if near_bound(rect, probe):
        outcomes.add(False)  # the probe may be outside the viewport
    if inside(rect, probe) or near_bound(rect, probe):
        for texel in _texels(probe):
            if not inside(rect, (texel[0] + 0.5, texel[1] + 0.5)):
                outcomes.add(False)
                continue
            for surface in scene.candidates(texel):
                outcomes.add(_blocks(None if surface is None else surface[0], p, n, q, radius, bias))
    else:
        outcomes.add(False)
    if outcomes == {True}:
        return True
    if outcomes == {False}:
        return False
    return "?"


def pixel_interval(scene, at, radius=2.0, bias=0.01, unlimited=False):
    """[k_min, k_max] blocked rays at framebuffer texel `at`, or None when the
    pixel is background or a silhouette. `unlimited` replaces the screen-space
    lookup with the analytic first surface along the eye ray through each probe
    (no viewport limit), for the off-screen limitation report only."""
    own = scene.candidates(at)
    if len(own) != 1 or own[0] is None:
        return None
    p, n = own[0]
    axis = (0.0, 1.0, 0.0) if abs(n[2]) > 0.9 else (0.0, 0.0, 1.0)
    tangent = normalize(cross(axis, n))
    bitangent = cross(n, tangent)
    k_min = k_max = 0
    for k in DIRECTIONS:
        d = tuple(tangent[i] * k[0] + bitangent[i] * k[1] + n[i] * k[2] for i in range(3))
        maybe, blocked = False, False
        for step in range(1, STEPS + 1):
            q = tuple(p[i] + d[i] * radius * step / STEPS for i in range(3))
            if unlimited:
                outcome = _blocks(_first_surface_along_view(scene, q), p, n, q, radius, bias)
            else:
                outcome = _step(scene, p, n, q, radius, bias)
            if outcome is True:
                blocked = True
                break
            if outcome == "?":
                maybe = True
        if blocked:
            k_min += 1
            k_max += 1
        elif maybe:
            k_max += 1
    return (k_min, k_max)


def _first_surface_along_view(scene, q):
    """Nearest analytic surface on the eye ray through q, ignoring the viewport."""
    camera = scene.camera
    if camera.orthographic:
        origin, direction = (q[0], q[1], 0.0), (0.0, 0.0, -1.0)
    elif q[2] < 0:
        origin, direction = (0.0, 0.0, 0.0), (q[0] / -q[2], q[1] / -q[2], -1.0)
    else:
        return None
    best, best_z = None, -math.inf
    for rect in scene.rects:
        t = (rect.z - origin[2]) / direction[2]
        if t <= 0:
            continue
        p = (origin[0] + t * direction[0], origin[1] + t * direction[1], rect.z)
        if rect.x[0] <= p[0] < rect.x[1] and rect.y[0] <= p[1] < rect.y[1] and rect.z > best_z:
            best, best_z = p, rect.z
    return best


def accessibility_interval(interval, strength=1.0):
    k_min, k_max = interval
    return (max(0.0, 1 - strength * k_max / 16), max(0.0, 1 - strength * k_min / 16))


# ------------------------------------------------------------------ fixtures

def numeric_scene(name, orthographic, rect=(13.25, 17.5, 280.5, 160.5)):
    """The helper's analytic geometry (tools/effects/src/numeric.rs constants)."""
    camera = Camera(orthographic, rect)
    receiver = Rect("receiver", (-8.0, 8.0), (-6.0, 6.0), -4.0)
    slab = Rect("slab", (0.4, 3.0), (-1.5, 1.5), -3.0)
    if name in ("white_slab", "black_slab", "mask_equal", "double_sided_back_receiver",
                "mirrored_receiver", "baked_slab_near", "lifecycle_slab"):
        return Scene(camera, [receiver, slab])
    if name == "flat":
        return Scene(camera, [receiver])
    if name == "reversed_normal":
        return Scene(camera, [Rect("receiver", (-8.0, 8.0), (-6.0, 6.0), -4.0, (0.0, 0.0, -1.0)), slab])
    if name in ("far_slab", "baked_slab_far"):
        return Scene(camera, [receiver, Rect("slab", (0.4, 3.0), (-1.5, 1.5), -1.0)])
    if name == "edge_slab":
        return Scene(camera, [receiver, Rect("slab", (2.2, 6.0), (-1.5, 1.5), -3.0)])
    if name == "mask_cutout":
        # Texel boundary u = 0.5 at x = 1.7; only the opaque right half remains.
        return Scene(camera, [receiver, Rect("slab", (1.7, 3.0), (-1.5, 1.5), -3.0)], edges=((0, 1.7),))
    if name == "mask_below":
        return Scene(camera, [receiver])
    raise KeyError(name)


def selftest():
    """Reproduce the engine CPU test's documented claims at its analytic point."""
    kernel = DIRECTIONS
    assert all(abs(dot(k, k) - 1) < 1e-6 and k[2] > 0 for k in kernel)
    assert abs(sum(k[2] ** 2 for k in kernel) / 16 - 0.5) < 1e-6
    results = {}
    for orthographic in (False, True):
        rect = (0.0, 0.0, 512.0, 512.0)
        camera = Camera(orthographic, rect)
        receiver = Rect("receiver", (-50.0, 50.0), (-50.0, 50.0), -4.0)
        half_plane = Rect("slab", (0.4, 50.0), (-50.0, 50.0), -3.0)
        centre = (255, 255)  # texel whose centre (255.5, 255.5) sits beside x = 0
        flat = pixel_interval(Scene(camera, [receiver]), centre)
        blocked = pixel_interval(Scene(camera, [receiver, half_plane]), centre)
        reversed_ = pixel_interval(
            Scene(camera, [Rect("receiver", (-50.0, 50.0), (-50.0, 50.0), -4.0, (0.0, 0.0, -1.0)), half_plane]),
            centre)
        assert flat == (0, 0), (orthographic, flat)
        assert blocked[0] == blocked[1] and 0 < blocked[0] < 16, (orthographic, blocked)
        assert reversed_ == (0, 0), (orthographic, reversed_)
        lo, hi = accessibility_interval(blocked, 0.5)
        assert abs(lo - (1 + (1 - blocked[0] / 16)) / 2) < 1e-12 and lo == hi
        results["orthographic" if orthographic else "perspective"] = {
            "flat": flat, "near_slab": blocked, "reversed_normal": reversed_}
    # The helper fixture: occlusion near the slab edge, none far away.
    scene = numeric_scene("white_slab", False)
    far_pixel = (40, 40)
    assert pixel_interval(scene, far_pixel) == (0, 0)
    return results


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        import json
        print(json.dumps(selftest(), indent=1))
    else:
        raise SystemExit(__doc__)
