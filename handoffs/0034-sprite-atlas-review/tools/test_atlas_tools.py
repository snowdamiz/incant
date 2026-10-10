#!/usr/bin/env python3
"""Self-checks for the handoff 0034 analysis helpers. Standard library only.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/test_atlas_tools.py
"""

import struct
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(TOOLS.parents[2] / "handoffs/0032-sprite-rendering/tools"))
import atlas_analyse as an  # noqa: E402
import atlas_art  # noqa: E402
import atlas_scene as sc  # noqa: E402
import pngio  # noqa: E402


class TieRule(unittest.TestCase):
    def test_exact_ties_take_the_increasing_screen_side(self):
        # Unflipped: texel index grows with screen x, so a tie at t = 5 selects texel 5.
        self.assertEqual(an.expected_index(5.0, 10, 1, False), 5)
        # Flipped: s = 10 - 5 = 5 decreases with screen x; the right-hand texel is 4.
        self.assertEqual(an.expected_index(5.0, 10, 1, True), 4)

    def test_one_raster_step_before_a_tie_stays_on_the_earlier_texel(self):
        self.assertEqual(an.expected_index(5.0 - 1 / 256, 10, 1, False), 4)
        self.assertEqual(an.expected_index(5.0 + 1 / 256, 10, 1, True), 4)
        self.assertEqual(an.expected_index(5.0 - 1 / 256, 10, 1, True), 5)

    def test_band_stays_one_quarter_raster_step_at_every_magnification(self):
        for m in (1, 8, 32):
            step = (1 / 256) / m  # one 1/256 px raster step, in texels
            self.assertEqual(an.expected_index(5.0 - step, 10, m, False), 4, m)  # a real non-tie is kept
            self.assertEqual(an.expected_index(5.0 - step / 8, 10, m, False), 5, m)  # inside the 1/1024 px band

    def test_clamped_to_region(self):
        self.assertEqual(an.expected_index(10.0, 10, 1, False), 9)
        self.assertEqual(an.expected_index(0.0, 10, 1, True), 9)


class StripColours(unittest.TestCase):
    def test_every_texel_names_itself_and_cuts_lie_mid_ramp(self):
        seen = set()
        for i in range(atlas_art.STRIP_LEN):
            r, g, b, _ = atlas_art.strip_colour(i, 0, 120)
            self.assertLessEqual(max(r, g, b), an.IDENTITY_MAX)
            self.assertEqual(an.strip_texel((r, g, b), atlas_art.STRIP_PHASE), (i, 120))
            seen.add((r, g))
        self.assertEqual(len(seen), atlas_art.STRIP_LEN)
        for cut in (512, 1024):
            before = atlas_art.strip_colour(cut - 1, 0, 0)
            after = atlas_art.strip_colour(cut, 0, 0)
            self.assertEqual((after[0] - before[0], after[1] - before[1]), (2, 0))

    def test_odd_values_are_rejected(self):
        self.assertIsNone(an.strip_texel((61, 60, 100), atlas_art.STRIP_PHASE))


class Orientations(unittest.TestCase):
    def test_mappings_are_permutations_matching_flips_and_quarter_turns(self):
        w, h = 3, 2
        for o in sc.ORIENTATIONS:
            W, H = (h, w) if o in ("R90", "R270", "R90FX") else (w, h)
            texels = {an.orientation_texel(o, a, b, w, h) for a in range(W) for b in range(H)}
            self.assertEqual(texels, {(x, y) for x in range(w) for y in range(h)}, o)
        # A counter-clockwise quarter turn on screen puts the top-right texel at the top-left.
        self.assertEqual(an.orientation_texel("R90", 0, 0, w, h), (w - 1, 0))
        self.assertEqual(an.orientation_texel("FX", 0, 0, w, h), (w - 1, 0))
        self.assertEqual(an.orientation_texel("R180", 0, 0, w, h), (w - 1, h - 1))


class NineSlice(unittest.TestCase):
    def test_native_size_is_identity_and_stretch_keeps_corners(self):
        native = an.nine_mapper(24, 24, sc.PANEL_BORDER, 16)
        for a in range(24):
            for b in range(24):
                x, y = native(a, b)
                if not (3 <= a < 18) and not (4 <= b < 16):
                    self.assertEqual((x, y), (a, b))
        stretch = an.nine_mapper(72, 44, sc.PANEL_BORDER, 16)
        self.assertEqual(stretch(71, 43), (23, 23))
        self.assertEqual(stretch(66, 36), (18, 16))
        self.assertEqual(an.nine_mapper(72, 44, sc.PANEL_BORDER, 8)(71, 43), (23, 23))
        self.assertEqual(an.nine_mapper(72, 44, sc.PANEL_BORDER, 8)(1, 1), (0, 0))


class Ktx2(unittest.TestCase):
    def test_reader_returns_level_zero_and_rejects_other_formats(self):
        w, h = 2, 1
        level0 = bytes([1, 2, 3, 255, 4, 5, 6, 0])
        header = b"\xabKTX 20\xbb\r\n\x1a\n" + struct.pack("<9I", 43, 1, w, h, 0, 0, 1, 1, 0)
        header += struct.pack("<4I2Q", 0, 0, 0, 0, 0, 0)
        index = struct.pack("<3Q", 104, len(level0), len(level0))
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "t.ktx2"
            path.write_bytes(header + index + level0)
            k = an.read_ktx2(path)
            self.assertEqual((k["width"], k["height"], k["level0"]), (2, 1, level0))
            path.write_bytes(header.replace(struct.pack("<I", 43), struct.pack("<I", 37), 1) + index + level0)
            with self.assertRaises(ValueError):
                an.read_ktx2(path)


class Compare(unittest.TestCase):
    def test_identical_and_differing_frames(self):
        with tempfile.TemporaryDirectory() as d:
            a, b, c = (Path(d) / n for n in ("a.png", "b.png", "c.png"))
            pngio.write_rgba(a, 2, 2, [(10, 10, 10, 255)] * 4)
            pngio.write_rgba(b, 2, 2, [(10, 10, 10, 255)] * 4)
            pngio.write_rgba(c, 2, 2, [(10, 10, 10, 255)] * 3 + [(12, 10, 10, 255)])
            self.assertTrue(an.compare(a, b)["identical"])
            pixels = []
            res = an.compare(a, c, {"corner": (1, 1, 1, 1)}, collect=pixels)
            self.assertEqual((res["pixels"], res["max_level"], res["by_region"], pixels),
                             (1, 2, {"corner": 1}, [(1, 1, 2)]))


class Ulids(unittest.TestCase):
    def test_generated_ids_are_canonical(self):
        alphabet = set("0123456789ABCDEFGHJKMNPQRSTVWXYZ")
        for n in (1, 31, 32, 4095):
            for value in (atlas_art.frame_id(n), sc.entity_id(n)):
                self.assertEqual(len(value), 26)
                self.assertTrue(set(value) <= alphabet and value[0] in "01234567")


if __name__ == "__main__":
    unittest.main(verbosity=1)
