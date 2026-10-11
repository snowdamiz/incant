"""CPU-only no-child unit tests for mandatory per-launch slot rejection."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from engine import CaptureClosed, Guard


class GuardTests(unittest.TestCase):
    def test_each_child_requires_current_grant_and_exact_binary(self):
        with tempfile.TemporaryDirectory() as root:
            grant = Path(root) / "ready"
            guard = Guard("frozen", grant, "source:hash")
            with patch("subprocess.Popen", return_value="child") as spawn:
                with self.assertRaises(CaptureClosed):
                    guard.Popen(["frozen", "screenshot"])
                spawn.assert_not_called()
                grant.write_text("different:hash")
                with self.assertRaises(CaptureClosed):
                    guard.Popen(["frozen", "import"])
                grant.write_text("source:hash")
                with self.assertRaises(CaptureClosed):
                    guard.Popen(["other", "rpc"])
                spawn.assert_not_called()
                self.assertEqual(guard.Popen(["frozen", "rpc"]), "child")
                grant.unlink()
                with self.assertRaises(CaptureClosed):
                    guard.Popen(["frozen", "screenshot"])
                self.assertEqual(spawn.call_count, 1)

    def test_baseline_and_candidate_grants_do_not_authorize_each_other(self):
        with tempfile.TemporaryDirectory() as root:
            grant = Path(root) / "ready"
            grant.write_text("baseline:hash")
            with patch("subprocess.Popen") as spawn:
                with self.assertRaises(CaptureClosed):
                    Guard("candidate", grant, "candidate:hash").Popen(["candidate", "init"])
                spawn.assert_not_called()


if __name__ == "__main__":
    unittest.main()
