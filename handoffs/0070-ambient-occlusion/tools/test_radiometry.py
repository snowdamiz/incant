#!/usr/bin/env python3
"""CPU-only radiometric contracts; no renderer/device construction or raw assets."""
import copy
import math
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze as a


def rgba(radiance):
    byte = a._byte(a.tone([radiance] * 3)[0])
    return bytes((byte, byte, byte, 255))


def metadata(orthographic=False):
    return {"scene": {"key_light": 0.0, "environment": {"linear_byte": 179, "intensity": 1.0},
                      "projection": "orthographic" if orthographic else "perspective"},
            "viewport": {"rect": list(a.RECT)}}


def variable_specular_fixture():
    on, black = bytearray(), bytearray()
    for y in range(a.SIZE[1]):
        for x in range(a.SIZE[0]):
            # View-dependent reflectance, independent of the reference algorithm.
            reflectance = 0.03 + 0.04 * (x / a.SIZE[0]) ** 2
            specular = a.LIGHT * reflectance
            black.extend(rgba(specular))
            on.extend(rgba((a.LIGHT - specular) * (1 - 4 / 16) + specular))
    off = bytes(rgba(a.LIGHT) * (a.SIZE[0] * a.SIZE[1]))
    return [a.Frame(a.SIZE, data, metadata()) for data in (bytes(on), off, bytes(black))]


class Radiometry(unittest.TestCase):
    def test_gray_curve_round_trip_on_both_sides_of_shoulder(self):
        for radiance in (0, 0.03, 0.79999, 0.8, 0.80001, 1, 4, 100):
            display = a.tone([radiance] * 3)[0]
            self.assertAlmostEqual(a.inverse_gray_tone(display), radiance, places=9)

    def test_interval_contains_scene_radiance_not_display_radiance(self):
        for radiance in (0, 0.03, 0.7, 0.79, 0.8, 0.81, 1, 1.2, 2, 4):
            bounds = a.scene_intervals(rgba(radiance))
            self.assertTrue(all(lo <= radiance <= hi for lo, hi in bounds), (radiance, bounds))
        self.assertLess(a.code_interval(rgba(1.2)[0])[1], 1.2)

    def test_no_colored_shoulder_scalar_inverse(self):
        with self.assertRaisesRegex(ValueError, "colored"):
            a.scene_intervals((250, 100, 100, 255))
        self.assertEqual(a.scene_intervals((100, 90, 80, 255)),
                         [a.code_interval(v) for v in (100, 90, 80)])

    def test_refuse_saturated_nonfinite_or_nonopaque_domains(self):
        for value in (-1, 1, math.nan, math.inf):
            with self.assertRaises(ValueError):
                a.inverse_gray_tone(value)
        for pixel in ((255, 255, 255, 255), (80, 80, 80, 128), (256, 80, 80, 255), (80, 80, 80)):
            with self.assertRaises(ValueError):
                a.scene_intervals(pixel)

    def test_fallback_direct_diffuse_explains_false_ao_attenuation(self):
        diffuse, specular, direct, k = 0.67, 0.03, 0.489, 12
        on = rgba(diffuse * (1 - k / 16) + specular + direct)
        off = rgba(diffuse + specular + direct)
        wrong, _ = a.infer_k(on, off, rgba(specular), 1)
        correct, reason = a.infer_k(on, off, rgba(specular + direct), 1)
        self.assertIsNone(reason)
        self.assertTrue(correct[0] <= k <= correct[1], correct)
        factor = diffuse / (diffuse + direct)
        self.assertTrue(wrong[0] <= k * factor <= wrong[1], wrong)
        self.assertLess(wrong[1], k)

    def test_per_pixel_nonuniform_specular_basis_is_valid(self):
        frames = variable_specular_fixture()
        self.assertFalse(a.furnace(frames[2])["uniform"])
        self.assertTrue(all(a.basis_contract(frames, a.ref.numeric_scene("white_slab", False)).values()))
        for x, y in ((20, 30), (100, 70), (200, 100), (280, 150)):
            interval, reason = a.infer_k(*(f.px(x, y) for f in frames), 1)
            self.assertIsNone(reason)
            self.assertTrue(interval[0] <= 4 <= interval[1], interval)

    def test_wrong_metadata_and_unmatched_normals_fail(self):
        original = variable_specular_fixture()
        for field, bad in (("key_light", 2.0), ("projection", "orthographic"),
                           ("environment", {"linear_byte": 178, "intensity": 1.0})):
            frames = copy.deepcopy(original)
            frames[0].meta["scene"][field] = bad
            result = a.basis_contract(frames, a.ref.numeric_scene("white_slab", False))
            self.assertFalse(result["explicit_ibl_only_matched_metadata"])
        frames = copy.deepcopy(original)
        del frames[2].meta["scene"]["key_light"]
        self.assertFalse(a.basis_contract(frames, a.ref.numeric_scene("white_slab", False))[
            "explicit_ibl_only_matched_metadata"])
        self.assertFalse(a.basis_contract(original, a.ref.numeric_scene("reversed_normal", False))[
            "matched_facing_shading_normals"])

    def test_invalid_basis_cannot_satisfy_white_furnace(self):
        frames = variable_specular_fixture()
        frames[1].data = rgba(1.2) * (a.SIZE[0] * a.SIZE[1])
        self.assertFalse(a.basis_contract(frames, a.ref.numeric_scene("white_slab", False))[
            "white_furnace_per_pixel"])
        frames[2].data = frames[1].data
        self.assertFalse(a.basis_contract(frames, a.ref.numeric_scene("white_slab", False))[
            "positive_per_pixel_diffuse"])

    def test_hdr_half_alpha_relation_with_quantized_bounds(self):
        for source, delta, pane in ((1.25, 0.25, 0.2), (0.7, 0.25, 0.14), (1.5, 0.4, 0.3)):
            pixels = [rgba(v) for v in (source, source - delta, 0.5 * (source + pane),
                                        0.5 * (source - delta + pane))]
            co, ca, mo, ma = [a.scene_intervals(p)[1] for p in pixels]
            dc = (co[0] - ca[1], co[1] - ca[0]); dm = (mo[0] - ma[1], mo[1] - ma[0])
            self.assertGreater(dc[0], 0.03)
            self.assertLessEqual(dm[0] / dc[1], 0.5)
            self.assertGreaterEqual(dm[1] / dc[0], 0.5)
        # Real retained four-code example: old display-linear algebra is not affine.
        co, ca, mo, ma = (248, 244, 231, 213)
        old = (a.LINEAR[mo] - a.LINEAR[ma]) / (a.LINEAR[co] - a.LINEAR[ca])
        new = ((a.inverse_gray_tone(a.LINEAR[mo]) - a.inverse_gray_tone(a.LINEAR[ma])) /
               (a.inverse_gray_tone(a.LINEAR[co]) - a.inverse_gray_tone(a.LINEAR[ca])))
        self.assertGreater(old, 2)
        self.assertTrue(0.4 <= new <= 0.6)


if __name__ == "__main__":
    unittest.main()
