from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from play_native import executable


class PhysicsPlay(unittest.TestCase):
    def test_only_the_unique_requested_target_binary_can_be_launched(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            target = 'aarch64-apple-darwin'
            path = root / target / 'ci/incant_headless'
            path.parent.mkdir(parents=True)
            path.write_bytes(b'fixture identity only; never executed')
            artifact = {'reason': 'compiler-artifact', 'target': {'name': 'incant_headless', 'kind': ['bin']},
                        'executable': str(path)}
            self.assertEqual(executable([artifact], root, target), path)
            for invalid in ([], [artifact, artifact],
                            [{**artifact, 'target': {'name': 'foreign', 'kind': ['bin']}}],
                            [{**artifact, 'target': {'name': 'incant_headless', 'kind': ['lib']}}]):
                with self.assertRaises(ValueError):
                    executable(invalid, root, target)
            host = root / 'ci/incant_headless'
            host.parent.mkdir()
            host.write_bytes(b'host artifact')
            with self.assertRaisesRegex(ValueError, 'outside selected target'):
                executable([{**artifact, 'executable': str(host)}], root, target)


if __name__ == '__main__':
    unittest.main()
