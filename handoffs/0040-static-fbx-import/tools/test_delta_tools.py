"""CPU-only tests for the tangent-delta helpers (no engine process is launched).

Run: python3 -I handoffs/0040-static-fbx-import/tools/test_delta_tools.py
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import delta_numbers  # noqa: E402
import delta_release  # noqa: E402


def record(**kw):
    base = {"seq": 1, "run": "d", "label": "x", "pid": 10, "exit": 0, "timed_out": False, "kind": "rpc",
            "binary_sha256_before": delta_release.CANDIDATE["sha256"], "binary_sha256_after": delta_release.CANDIDATE["sha256"]}
    base.update(kw)
    return base


class DeltaToolTests(unittest.TestCase):
    def test_bevel_helper_reproduces_original_inverted_measurement(self):
        # The original frozen-binary frame must still read as inverted (top - bottom < 0).
        original = delta_numbers.ORIGINAL / "textures/frames/controls_ortho.png"
        table = delta_numbers.bevel_table(original, delta_numbers.CONTROLS, -3.2, -1.0)
        self.assertGreater(table["gltf_supplied_tangent"]["top_minus_bottom"], 60)
        self.assertLess(table["fbx_normal_cube"]["top_minus_bottom"], -60)

    def test_release_refuses_running_or_unexited_children(self):
        with self.assertRaises(SystemExit):
            delta_release.summary([record()], running=[123])
        with self.assertRaises(SystemExit):
            delta_release.summary([record(exit=None)], running=[])
        done = delta_release.summary([record(), record(seq=2, kind="screenshot", output_bytes=5, adapter="GPU")], running=[])
        self.assertEqual((done["captures"], done["adapters"], done["candidate_hash_checked_every_child"]), (1, ["GPU"], True))

    def test_release_detects_wrong_candidate_hash(self):
        done = delta_release.summary([record(binary_sha256_after="0" * 64)], running=[])
        self.assertFalse(done["candidate_hash_checked_every_child"])

    def test_inventory_query_is_read_only_and_empty_for_unknown_path(self):
        self.assertEqual(delta_release.running_candidates(Path("/nonexistent/claude-0040-candidate")), [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
