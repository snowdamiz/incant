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
