"""CPU-only regression: a withdrawn delta gate cannot launch another child."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from delta_normal import GuardedSubprocess, engine


class GateTests(unittest.TestCase):
    def test_missing_or_wrong_grant_never_starts_a_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate, marker = root / "grant", root / "child"
            guard = GuardedSubprocess(gate, "source:sha", sys.executable)
            command = [sys.executable, "-c", "from pathlib import Path; import sys; Path(sys.argv[1]).write_text('started')", str(marker)]
            for grant in [None, "different-source:sha", "source:wrong-sha"]:
                if grant is not None:
                    gate.write_text(grant)
                with self.assertRaises(engine.EngineError):
                    guard.Popen(command)
                self.assertFalse(marker.exists())

    def test_rechecks_each_launch_and_preserves_completed_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate, marker = root / "grant", root / "child"
            gate.write_text("source:sha\n")
            guard = GuardedSubprocess(gate, "source:sha", sys.executable)
            command = [sys.executable, "-c", "from pathlib import Path; import sys; Path(sys.argv[1]).write_text('completed')", str(marker)]
            process = guard.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            process.communicate(timeout=10)
            self.assertEqual(process.returncode, 0)
            self.assertEqual(marker.read_text(), "completed")
            gate.unlink()
            marker.unlink()
            with self.assertRaises(engine.EngineError):
                guard.Popen(command)
            self.assertFalse(marker.exists())

    def test_refuses_binary_substitution_even_with_open_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            gate = Path(directory) / "grant"
            gate.write_text("source:sha")
            guard = GuardedSubprocess(gate, "source:sha", "/expected/candidate")
            with self.assertRaises(engine.EngineError):
                guard.Popen([sys.executable, "-c", "pass"])


if __name__ == "__main__":
    unittest.main()
