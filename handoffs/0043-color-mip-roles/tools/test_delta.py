"""CPU-only tests for the 0043 fixture and runner helpers. No engine child starts."""
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import delta  # noqa: E402
import fixtures  # noqa: E402


class FixtureTests(unittest.TestCase):
    def test_card_variants_keep_rgb_and_differ_only_in_alpha(self):
        mixed, zero, one = (fixtures.card(kind) for kind in ("mixed", "zero", "one"))
        self.assertEqual([p[:3] for p in mixed], [p[:3] for p in zero])
        self.assertEqual([p[:3] for p in mixed], [p[:3] for p in one])
        self.assertEqual({p[3] for p in zero}, {0})
        self.assertEqual({p[3] for p in one}, {255})
        self.assertEqual({p[3] for p in mixed}, {0, 51, 102, 153, 204, 255})
        self.assertEqual(mixed, fixtures.REF.card_base_pixels())

    def test_linear_control_carries_the_card_alpha(self):
        self.assertEqual([p[3] for p in fixtures.metallic_roughness()], [p[3] for p in fixtures.card("mixed")])

    def test_cells_are_pixel_aligned_and_start_on_the_same_stripe_phase(self):
        cells = fixtures.cells()
        self.assertEqual(len(cells), 3 * (len(fixtures.BASE) + len(fixtures.EMISSIVE)))
        rects = {name: fixtures.cell_rect(cell) for name, cell in cells.items()}
        for name, (left, top, right, bottom) in rects.items():
            self.assertEqual((right - left, bottom - top), (80, 80), name)
            self.assertEqual(left % 44, 0, name)  # dark 22 px stripe begins at every cell's left edge
            self.assertTrue(0 <= left and right <= 960 and 0 <= top and bottom <= 540, name)
        self.assertLess(fixtures.lod(1), 0)
        self.assertLess(0, fixtures.lod(2))
        self.assertGreater(fixtures.lod(6), 2)

    def test_sky_zones_and_alpha_one_control(self):
        w, _ = fixtures.SKY_SIZE
        sky, control = fixtures.sky(), fixtures.sky(alpha_one=True)
        at = lambda pixels, x, y: pixels[y * w + x]  # noqa: E731
        self.assertEqual(at(sky, 100, 50)[3], 0)
        self.assertEqual(at(sky, 600, 50)[3], 128)
        self.assertEqual(at(sky, 576, 224)[3], 255)
        self.assertEqual(at(sky, 100, 400)[3], 255)
        self.assertEqual([p[:3] for p in sky], [p[:3] for p in control])
        self.assertEqual({p[3] for p in control}, {255})


class RunnerLimitTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        root = Path(self.directory.name)
        self.patches = [patch.object(delta, "LEDGER", root / "ledger.jsonl"),
                        patch.object(delta, "LOCK", root / "RELEASED"),
                        patch.object(delta, "TIMELINE", root / "timeline.jsonl")]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.directory.cleanup()

    def write_shots(self, count):
        with delta.LEDGER.open("a") as handle:
            for i in range(count):
                handle.write(json.dumps({"seq": i + 1, "kind": "screenshot", "exit": i % 2}) + "\n")

    def test_failed_launches_count_toward_the_frame_cap(self):
        self.write_shots(19)
        delta.assert_open("screenshot")
        self.write_shots(1)
        with self.assertRaises(SystemExit):
            delta.assert_open("screenshot")
        delta.assert_open("rpc")  # CPU children are not frames

    def test_release_lock_refuses_before_configuring_any_binary(self):
        delta.LOCK.write_text("released\n")
        with patch.object(delta, "harness") as harness:
            for kind in ("screenshot", "import", "rpc", "validate", "init"):
                with self.assertRaises(SystemExit):
                    delta.timed("candidate", kind, "after release", lambda engine: engine.call())
            harness.assert_not_called()
        self.assertFalse(delta.TIMELINE.exists())


if __name__ == "__main__":
    unittest.main()
