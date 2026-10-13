"""Filtered continuous accessibility intervals for the provisional R1/R3 candidate.

Design aid for ssao-filter-preparation.md; standard library only. It layers the
eeeac1d4 candidate (Bayer frame rotation, upper-half step jitter, two separable
plane-weighted five-tap passes with R8 writes) on the UNCHANGED scalar probe
oracle tools/ssao_reference.py, imported read-only:

* raw stage: for each coverage candidate of a texel, the rotated/jittered kernel
  is walked with ssao_reference._step, which keeps its three-valued tolerance
  bands; the blocked-ray count is an exact integer interval [k_lo, k_hi];
* every R8 write (AO pass -> T0, H pass -> T1, V pass -> T0) is its own rounding
  stage: a value interval maps to every code a correctly rounded write may store
  (both codes at exact ties) widened only by MEAN_EPS for f32 evaluation;
* each filter tap contributes a value interval (from the previous stage) and a
  weight interval: same-plane taps c*[1 - PLANE_DELTA/tau, 1], a cleared-depth
  tap 0, an ambiguous-coverage tap the hull of its alternatives' weights. The
  centre weight is exactly 1 (the shader's explicit centre term). The weighted
  mean's extremes over the weight box are attained at box vertices, which are
  enumerated exactly;
* a texel whose centre lies outside the viewport is the pass's white clear
  (255), decided exactly: texel centres and the viewport rect are f32-exact.

Nothing is dropped: ambiguous texels keep the hull of their alternatives and
are counted by kind. A filtered pass may also take explicit input planes (the
read-back R8 codes of the previous stage) so each GPU stage can be checked on
its own inputs.
"""

import math
import struct
import sys
from itertools import product
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parent.parent / "tools"
sys.path.insert(0, str(TOOLS))
import ssao_reference as ref  # noqa: E402  (pinned by build004; imported, never modified)

SIZE = (321, 193)
RECT = (13.25, 17.5, 280.5, 160.5)
BIAS, RADIUS = 0.01, 2.0
BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))
TAPS = ((-2, 0.5), (-1, 1.0), (1, 1.0), (2, 0.5))  # the centre has explicit weight 1
PLANE_DELTA = 1e-4  # view units: f32 depth/position reconstruction of one plane
MEAN_EPS = 2e-4     # codes: f32 weighted mean (<= 5 terms) and the R8 load (code/255)
DECISIVE_WIDTH = 3  # codes: final width bound when no decision in the support is ambiguous


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


# math.rs rotations(): alpha = (2 pi / 16) (b + 0.5) / 16, cos/sin rounded once to f32.
ROTATIONS = [(f32(math.cos(math.tau / 16 * (b + 0.5) / 16)), f32(math.sin(math.tau / 16 * (b + 0.5) / 16)))
             for b in range(16)]


def rotation(x, y):
    return ROTATIONS[BAYER[y % 4][x % 4]]


def jitter(x, y):
    """(33 + 2 B[x mod 4][y mod 4]) / 64: the transposed step phase, exact in f32."""
    return (33 + 2 * BAYER[x % 4][y % 4]) / 64


def r8_codes(lo, hi):
    """Every code a round-to-nearest R8 write may store for a value in [lo, hi]."""
    c_lo = max(0, math.ceil(255 * lo - 0.5 - MEAN_EPS))
    c_hi = min(255, math.floor(255 * hi + 0.5 + MEAN_EPS))
    return c_lo, c_hi


def mean_bounds(centre, taps):
    """Extremes of (a0 + sum w_i a_i) / (1 + sum w_i).

    centre: (lo, hi) value with weight exactly 1; taps: [((lo, hi), (w_lo, w_hi))].
    The mean is increasing in every value, so lower values give the minimum and
    upper values the maximum; over the weight box a linear-fractional function
    is extremal at a vertex, enumerated for every tap whose weight is uncertain.
    """
    fixed = [(v, w[0]) for v, w in taps if w[0] == w[1]]
    loose = [(v, w) for v, w in taps if w[0] != w[1]]
    lo = hi = None
    for corner in product(*[(w[0], w[1]) for _, w in loose]):
        terms = fixed + [(v, wc) for (v, _), wc in zip(loose, corner)]
        mass = 1.0 + sum(w for _, w in terms)
        low = (centre[0] + sum(w * v[0] for v, w in terms)) / mass
        high = (centre[1] + sum(w * v[1] for v, w in terms)) / mass
        lo = low if lo is None else min(lo, low)
        hi = high if hi is None else max(hi, high)
    return max(0.0, lo), min(1.0, hi)


class FilterOracle:
    """Raw, H and V code intervals of one scene at one strength."""

    def __init__(self, scene, strength=1.0, radius=RADIUS, bias=BIAS):
        self.scene, self.strength, self.radius, self.bias = scene, strength, radius, bias
        self.rect = scene.camera.rect
        self._k, self._raw, self._h, self._v = {}, {}, {}, {}
        self.events = {"probe_ambiguous_texels": set(), "coverage_ambiguous_texels": set(),
                       "uncertain_inclusion_taps": 0, "partial_weight_taps": 0,
                       "raw_tie_texels": set(), "h_two_or_more_codes": set(), "v_widths": {}}

    # ---------------------------------------------------------------- geometry
    def inside(self, x, y):
        return ref.inside(self.rect, (x + 0.5, y + 0.5))

    def candidates(self, x, y):
        """Coverage alternatives at the texel centre; None is cleared depth."""
        return self.scene.candidates((x, y))

    # ---------------------------------------------------------------- raw stage
    def k_interval(self, x, y, surface):
        key = (x, y, surface)
        if key in self._k:
            return self._k[key]
        p, n = surface
        axis = (0.0, 1.0, 0.0) if abs(n[2]) > 0.9 else (0.0, 0.0, 1.0)
        tangent = ref.normalize(ref.cross(axis, n))
        bitangent = ref.cross(n, tangent)
        c, s = rotation(x, y)
        rt = tuple(tangent[i] * c + bitangent[i] * s for i in range(3))
        rb = tuple(bitangent[i] * c - tangent[i] * s for i in range(3))
        t = jitter(x, y)
        k_lo = k_hi = 0
        for k in ref.DIRECTIONS:
            d = tuple(rt[i] * k[0] + rb[i] * k[1] + n[i] * k[2] for i in range(3))
            blocked = maybe = False
            for step in range(1, ref.STEPS + 1):
                q = tuple(p[i] + d[i] * (self.radius * (step - 1 + t) / ref.STEPS) for i in range(3))
                outcome = ref._step(self.scene, p, n, q, self.radius, self.bias)
                if outcome is True:
                    blocked = True
                    break
                if outcome == "?":
                    maybe = True
            k_lo += blocked
            k_hi += blocked or maybe
        self._k[key] = (k_lo, k_hi)
        return self._k[key]

    def raw(self, x, y):
        """Code interval written by the AO pass at texel (x, y)."""
        if (x, y) in self._raw:
            return self._raw[(x, y)]
        if not self.inside(x, y):
            codes = (255, 255)  # white clear outside the viewport
        else:
            options = self.candidates(x, y)
            if len(options) > 1:
                self.events["coverage_ambiguous_texels"].add((x, y))
            spans = []
            for surface in options:
                if surface is None:
                    spans.append((255, 255))  # cleared depth: the shader returns 1
                    continue
                k_lo, k_hi = self.k_interval(x, y, surface)
                if k_lo != k_hi:
                    self.events["probe_ambiguous_texels"].add((x, y))
                value = (max(0.0, 1 - self.strength * k_hi / 16), max(0.0, 1 - self.strength * k_lo / 16))
                span = r8_codes(*value)
                if value[0] == value[1] and span[0] != span[1]:
                    self.events["raw_tie_texels"].add((x, y))
                spans.append(span)
            codes = (min(s[0] for s in spans), max(s[1] for s in spans))
        self._raw[(x, y)] = codes
        return codes

    # ---------------------------------------------------------------- filter stages
    def tap_weight(self, centre, x, y, coefficient):
        """Weight interval of tap texel (x, y) for a centre surface (p, n)."""
        if not self.inside(x, y):
            return None  # skipped by the shader's inside(center) test: no term at all
        p, n = centre
        tau = max(self.bias, 0.01 * (-p[2]))
        bounds = []
        for surface in self.candidates(x, y):
            if surface is None:
                bounds.append((0.0, 0.0))  # cleared depth is skipped
                continue
            q = surface[0]
            dist = abs(ref.dot(ref.sub(q, p), n))
            w_lo = coefficient * min(1.0, max(0.0, 1 - (dist + PLANE_DELTA) / tau))
            w_hi = coefficient * min(1.0, max(0.0, 1 - max(0.0, dist - PLANE_DELTA) / tau))
            bounds.append((w_lo, w_hi))
        w = (min(b[0] for b in bounds), max(b[1] for b in bounds))
        if w[0] == 0.0 < w[1]:
            self.events["uncertain_inclusion_taps"] += 1
        elif 0.0 < w[0] and w[1] < coefficient * (1 - 2 * PLANE_DELTA / tau):
            self.events["partial_weight_taps"] += 1
        return w

    def pass_value(self, x, y, horizontal, source):
        """Value interval of one filter pass at texel (x, y); source(x, y) gives
        the input code interval (oracle or read-back plane)."""
        if not self.inside(x, y):
            return (1.0, 1.0)
        centre_codes = source(x, y)
        centre = (centre_codes[0] / 255, centre_codes[1] / 255)
        result = None
        for surface in self.candidates(x, y):
            if surface is None:
                span = (1.0, 1.0)  # cleared depth: the pass returns 1
            else:
                taps = []
                for offset, coefficient in TAPS:
                    tx, ty = (x + offset, y) if horizontal else (x, y + offset)
                    w = self.tap_weight(surface, tx, ty, coefficient)
                    if w is None or w[1] == 0.0:
                        continue
                    codes = source(tx, ty)
                    taps.append(((codes[0] / 255, codes[1] / 255), w))
                span = mean_bounds(centre, taps)
            result = span if result is None else (min(result[0], span[0]), max(result[1], span[1]))
        return result

    def h(self, x, y):
        if (x, y) not in self._h:
            codes = r8_codes(*self.pass_value(x, y, True, self.raw))
            if codes[1] - codes[0] >= 2:
                self.events["h_two_or_more_codes"].add((x, y))
            self._h[(x, y)] = codes
        return self._h[(x, y)]

    def v(self, x, y):
        if (x, y) not in self._v:
            self._v[(x, y)] = r8_codes(*self.pass_value(x, y, False, self.h))
            width = self._v[(x, y)][1] - self._v[(x, y)][0]
            self.events["v_widths"][width] = self.events["v_widths"].get(width, 0) + 1
        return self._v[(x, y)]

    def decisive(self, x, y):
        """No ambiguous probe, coverage or inclusion decision in the 5x5 support."""
        for dy in range(-2, 3):
            for dx in range(-2, 3):
                tx, ty = x + dx, y + dy
                if not self.inside(tx, ty):
                    continue
                options = self.candidates(tx, ty)
                if len(options) != 1:
                    return False
                if options[0] is not None:
                    k_lo, k_hi = self.k_interval(tx, ty, options[0])
                    if k_lo != k_hi:
                        return False
        return True

    def summary(self):
        e = self.events
        return {"probe_ambiguous_texels": len(e["probe_ambiguous_texels"]),
                "coverage_ambiguous_texels": len(e["coverage_ambiguous_texels"]),
                "raw_tie_texels": len(e["raw_tie_texels"]),
                "uncertain_inclusion_taps": e["uncertain_inclusion_taps"],
                "partial_weight_taps": e["partial_weight_taps"],
                "h_two_or_more_code_texels": len(e["h_two_or_more_codes"]),
                "final_width_histogram": dict(sorted(e["v_widths"].items()))}


def plane_source(plane, size=SIZE):
    """Exact code 'interval' from a read-back R8 plane (bytes, row-major)."""
    width = size[0]

    def source(x, y):
        if not (0 <= x < size[0] and 0 <= y < size[1]):
            return (255, 255)
        code = plane[y * width + x]
        return (code, code)
    return source


# ------------------------------------------------------------------ fixtures beyond numeric_scene

STRIPE_PLACES = [((-0.45 + 0.3 * i, 0.5, -3.0), (0.1, 2.0, 1.0)) for i in range(4)]  # (centre, scale)
SPHERE_PLACE = ((0.0, 0.0, -3.0), 1.2)  # gltf::sphere(24, 48) radius 0.5 x scale 1.2, gray 0.5:
# a white dielectric under the uniform furnace matches the environment radiance, so gray keeps
# the sphere distinguishable from any background for the coverage precondition.


def stripes_scene(orthographic, rect=RECT):
    """N8 fixture: the receiver plus four 0.1-wide stripes, pitch 0.3, y in [-0.5, 1.5), z = -3."""
    camera = ref.Camera(orthographic, rect)
    rects = [ref.Rect("receiver", (-8.0, 8.0), (-6.0, 6.0), -4.0)]
    for i, (centre, scale) in enumerate(STRIPE_PLACES):
        rects.append(ref.Rect(f"stripe{i}", (centre[0] - scale[0] / 2, centre[0] + scale[0] / 2),
                              (centre[1] - scale[1] / 2, centre[1] + scale[1] / 2), centre[2]))
    return ref.Scene(camera, rects)


def scene(name, orthographic):
    return stripes_scene(orthographic) if name == "stripes" else ref.numeric_scene(name, orthographic)
