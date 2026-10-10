#!/usr/bin/env python3
"""Self-checks for reference.py against published contract values. Standard library only.

Usage (repo root): python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import reference  # noqa: E402


def sprite(**overrides):
    value = {"texture": "test", "region": [16, 32, 32, 16], "size": [4.0, 2.0], "pivot": [0.25, 0.5],
             "tint": [1.0] * 4, "flip_x": False, "flip_y": False, "mode": {"kind": "simple"}}
    value.update(overrides)
    return value


class Tessellation(unittest.TestCase):
    """The same inputs and expectations as incant_2d's own unit tests."""

    def test_subregion_anchor_and_flips(self):
        a = reference.tessellate(sprite(), 128, 64)
        self.assertEqual(len(a), 6)
        self.assertEqual(a[0], (-1.0, -1.0, 0.125, 0.75))
        self.assertEqual(a[2], (3.0, 1.0, 0.375, 0.5))
        b = reference.tessellate(sprite(flip_x=True, flip_y=True), 128, 64)
        self.assertEqual([v[:2] for v in a], [v[:2] for v in b])
        self.assertEqual(b[0][2:], (0.375, 0.5))
        self.assertEqual(b[2][2:], (0.125, 0.75))

    def test_nine_slice_density_flips_and_compression(self):
        mode = {"kind": "nine_slice", "border": [4, 8, 2, 6], "pixels_per_unit": 4.0}
        a = reference.tessellate(sprite(mode=mode), 128, 64)
        self.assertEqual(len(a), 36)  # zero-height vertical centre omitted
        self.assertTrue(any(v[:2] == (0.0, 0.5) for v in a))
        flipped = reference.tessellate(sprite(mode=mode, flip_x=True, flip_y=True), 128, 64)
        self.assertEqual(flipped[1][:2], (1.0, -1.0))
        self.assertEqual(flipped[1][2:], (40 / 128, 32 / 64))
        self.assertEqual(flipped[2][:2], (1.0, -0.5))
        self.assertEqual(flipped[2][2:], (40 / 128, 34 / 64))
        small = reference.tessellate(sprite(mode=mode, size=[1.5, 1.0]), 128, 64)
        self.assertEqual(len(small), 24)
        area = 0.0
        for k in range(0, len(small), 3):
            (ax, ay), (bx, by), (cx, cy) = (v[:2] for v in small[k:k + 3])
            area += ((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)) * 0.5
        self.assertAlmostEqual(area, 1.5, places=12)


class Display(unittest.TestCase):
    """Values stated in the tone-map shader's own commentary."""

    def test_identity_below_shoulder_and_documented_highlights(self):
        self.assertEqual(reference.tone_map([0.5, 0.25, 0.79]), [0.5, 0.25, 0.79])
        for radiance, shown in ((1.0, 243), (2.0, 252), (4.0, 254)):
            self.assertEqual(reference.display((radiance, radiance, radiance, 1.0))[0], shown)
        self.assertEqual(reference.display((1.0, 0.0, 0.0, 1.0)), (243, 30, 30))

    def test_transparent_black_shows_the_backdrop(self):
        self.assertEqual(reference.display((0.0, 0.0, 0.0, 0.0)), (20, 21, 25))

    def test_half_float_rounding(self):
        self.assertEqual(reference.f16(0.1), 0.0999755859375)
        self.assertEqual(reference.f16(1.0), 1.0)
        self.assertEqual(reference.f16(70000.0), 65504.0)


class TieBand(unittest.TestCase):
    """The corrected nearest rule (source c2c7758), one axis at a time."""

    def test_exact_ties_take_the_increasing_screen_side(self):
        self.assertEqual(reference.tie_band_index(3.0, 1.0, 0.0, 7), (3, None))   # texels grow rightward
        self.assertEqual(reference.tie_band_index(3.0, -1.0, 0.0, 7), (2, None))  # flipped: texel 2 lies right
        self.assertEqual(reference.tie_band_index(7.0, -1.0, 0.0, 7), (6, None))  # clamped to the region

    def test_band_is_one_1024th_of_a_texel(self):
        self.assertEqual(reference.tie_band_index(3.0 - 0.9 / 1024, 1.0, 0.0, 7)[0], 3)
        self.assertEqual(reference.tie_band_index(3.0 - 1.1 / 1024, 1.0, 0.0, 7)[0], 2)

    def test_dominant_axis_and_equal_slopes(self):
        # A quarter turn: this texel axis follows screen Y only.
        self.assertEqual(reference.tie_band_index(3.0, 0.0, -1.0, 7), (2, None))
        # Equal slopes: X wins, and an opposite-signed Y makes the pixel ambiguous.
        index, alternative = reference.tie_band_index(3.0, 0.5, -0.5, 7)
        self.assertEqual((index, alternative), (3, 2))


class BandRules(unittest.TestCase):
    """Fixed-texel band (c2c7758) against the derivative-scaled band (f9aa1da) at 8x."""

    def setUp(self):
        self.default_rule = reference.NEAREST_RULE

    def tearDown(self):
        reference.NEAREST_RULE = self.default_rule

    def test_one_raster_step_past_a_tie_at_8x(self):
        t = 3.0 - (1 / 256) / 8  # an edge one 1/256 px step past a pixel centre, 8 px per texel
        reference.NEAREST_RULE = "tie-band"
        self.assertEqual(reference.tie_band_index(t, 1 / 8, 0.0, 7)[0], 3)  # band swallows the non-tie
        reference.NEAREST_RULE = "pixel-band"
        self.assertEqual(reference.tie_band_index(t, 1 / 8, 0.0, 7)[0], 2)  # exact geometry

    def test_exact_ties_at_8x_agree(self):
        for rule in ("tie-band", "pixel-band"):
            reference.NEAREST_RULE = rule
            self.assertEqual(reference.tie_band_index(3.0, 1 / 8, 0.0, 7), (3, None))
            self.assertEqual(reference.tie_band_index(3.0, -1 / 8, 0.0, 7), (2, None))

    def test_large_coordinates_are_knife_edges_not_predictions(self):
        # Near t = 4096 the scaled band is below f32 resolution: flag, do not guess.
        reference.NEAREST_RULE = "pixel-band"
        index, alternative = reference.tie_band_index(4095.0, -1 / 8, 0.0, 4096)
        self.assertEqual(index, 4094)
        self.assertIsNotNone(alternative)


class Spans(unittest.TestCase):
    """Bounded spans with a flat integer base (source 0494b40)."""

    def setUp(self):
        self.default_rule = reference.NEAREST_RULE

    def tearDown(self):
        reference.NEAREST_RULE = self.default_rule

    def test_cuts_start_at_the_first_vertex_in_either_direction(self):
        self.assertEqual(reference.span_cuts(0.0, 1000.0), [(0.0, 0.0), (0.512, 512.0), (1.0, 1000.0)])
        self.assertEqual(reference.span_cuts(1000.0, 0.0), [(0.0, 1000.0), (0.512, 488.0), (1.0, 0.0)])
        self.assertEqual(reference.span_cuts(4097.0, 0.0)[-2:], [(4096 / 4097, 1.0), (1.0, 0.0)])
        self.assertEqual(reference.span_cuts(5.0, 5.0), [(0.0, 5.0), (1.0, 5.0)])

    def test_geometry_invariants_from_the_candidate_unit_test(self):
        # The same sprites as geometry.rs's own test: flips, a collapsed nine-slice
        # border and both a large and a tiny size, on an 8192 x 8192 region.
        for flipped in (False, True):
            for mode in ({"kind": "simple"}, {"kind": "nine_slice", "border": [511, 1, 17, 3], "pixels_per_unit": 8.0}):
                for size in ([1024.0, 1024.0], [0.5, 0.25]):
                    value = sprite(region=[0, 0, 8192, 8192], size=size, pivot=[0.25, 0.75], flip_x=flipped,
                                   flip_y=flipped, mode=mode)
                    original = reference.tessellate(value, 8192, 8192)
                    spans = reference.nearest_spans(original, value["region"], 8192, 8192)
                    self.assertLessEqual(len(spans), 1944)
                    self.assertEqual(len(spans) % 6, 0)
                    area = 0.0
                    for k in range(0, len(spans), 3):
                        a, b, c = spans[k:k + 3]
                        self.assertTrue(a[4:] == b[4:] == c[4:])  # flat base per triangle
                        area += abs((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5
                    self.assertAlmostEqual(area, size[0] * size[1], delta=1e-9 * max(area, 1.0))
                    for v in spans:
                        self.assertTrue(0.0 <= v[2] <= 512.0 and 0.0 <= v[3] <= 512.0)
                        texel = (v[4] + v[2], v[5] + v[3])
                        self.assertTrue(any(all(
                            -1e-10 <= (v[i] - q[0][i]) / (q[2][i] - q[0][i]) <= 1 + 1e-10
                            and abs((q[0][2 + i] + (q[2][2 + i] - q[0][2 + i]) * (v[i] - q[0][i]) / (q[2][i] - q[0][i]))
                                    * 8192 - texel[i]) < 1e-8 for i in range(2))
                            for q in (original[j:j + 6] for j in range(0, len(original), 6))))

    def test_small_regions_keep_their_quads_and_region_local_texels(self):
        for mode in ({"kind": "simple"}, {"kind": "nine_slice", "border": [4, 8, 2, 6], "pixels_per_unit": 4.0}):
            value = sprite(flip_x=True, mode=mode)
            original = reference.tessellate(value, 128, 64)
            spans = reference.nearest_spans(original, value["region"], 128, 64)
            self.assertEqual([v[:2] for v in spans], [v[:2] for v in original])  # one span per quad
            for v, o in zip(spans, original):
                self.assertEqual((v[4] + v[2], v[5] + v[3]), (round(o[2] * 128 - 16), round(o[3] * 64 - 32)))
            if mode["kind"] == "simple":
                self.assertEqual({v[4:] for v in spans}, {(0.0, 0.0)})

    def test_exact_ties_at_large_coordinates_become_predictions(self):
        # Texel 4096 of a flipped strip at 8x: base 3584 + span texel 512.
        reference.NEAREST_RULE = "span-band"
        self.assertEqual(reference.tie_band_index(4096.0, -1 / 8, 0.0, 8192, local=512.0), (4095, None))
        self.assertEqual(reference.tie_band_index(4096.0, 1 / 8, 0.0, 8192, local=0.0), (4096, None))
        # The region-local rule leaves the same pixel to f32 rounding.
        reference.NEAREST_RULE = "pixel-band"
        self.assertIsNotNone(reference.tie_band_index(4096.0, -1 / 8, 0.0, 8192)[1])


class Raster(unittest.TestCase):
    def test_shared_diagonal_covers_each_pixel_exactly_once(self):
        frame = reference.Frame(16, 16)
        hits = [0] * 256

        def shade(i, attr, amb, inside):
            hits[i] += inside
        quad = [(0.0, 0.0), (16.0, 0.0), (16.0, 16.0), (0.0, 16.0)]
        for tri in ((0, 1, 2), (0, 2, 3)):
            reference.raster_triangle(frame, [quad[k] for k in tri], [(0.0,)] * 3, shade)
        self.assertEqual(set(hits), {1})

    def test_top_left_rule_on_pixel_centre_edges(self):
        frame = reference.Frame(8, 8)
        covered = set()

        def shade(i, attr, amb, inside):
            if inside:
                covered.add((i % 8, i // 8))
        quad = [(0.5, 0.5), (3.5, 0.5), (3.5, 3.5), (0.5, 3.5)]  # edges through pixel centres
        for tri in ((0, 1, 2), (0, 2, 3)):
            reference.raster_triangle(frame, [quad[k] for k in tri], [(0.0,)] * 3, shade)
        self.assertEqual(covered, {(x, y) for x in range(3) for y in range(3)})


if __name__ == "__main__":
    unittest.main(verbosity=1)
