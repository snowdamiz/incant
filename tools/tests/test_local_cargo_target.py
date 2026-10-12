import concurrent.futures
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import local_cargo_target as target


class LocalCargoTargetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / 'targets'
        self.first = self.checkout('first')
        self.second = self.checkout('second')

    def checkout(self, name):
        path = self.base / name
        path.mkdir()
        (path / '.git').write_text('gitdir: shared-test-repository\n')
        return path

    def test_two_checkouts_cannot_share_target_and_repeat_keeps_artifacts(self):
        first = target.select(self.root, self.first)
        second = target.select(self.root, self.second)
        self.assertNotEqual(first, second)
        artifact = first / 'dependency.rmeta'
        artifact.write_bytes(b'retained dependency')
        self.assertEqual(target.select(self.root, self.first), first)
        self.assertEqual(artifact.read_bytes(), b'retained dependency')

    def test_symlink_alias_resolves_to_same_checkout(self):
        alias = self.base / 'alias'
        try:
            alias.symlink_to(self.first, target_is_directory=True)
        except OSError as error:
            self.skipTest(str(error))
        self.assertEqual(target.select(self.root, alias), target.select(self.root, self.first))

    def test_copied_stale_owner_is_refused_without_touching_artifacts(self):
        first = target.select(self.root, self.first)
        second = target.select(self.root, self.second)
        (first / target.MARKER).write_bytes((second / target.MARKER).read_bytes())
        artifact = first / 'dependency.rmeta'
        artifact.write_bytes(b'original')
        with self.assertRaisesRegex(ValueError, 'ownership does not match'):
            target.select(self.root, self.first)
        self.assertEqual(artifact.read_bytes(), b'original')

    def test_unowned_nonempty_and_malformed_metadata_refuse(self):
        path = target.select(self.root, self.first)
        marker = path / target.MARKER
        marker.unlink()
        (path / 'dependency.rmeta').write_bytes(b'original')
        with self.assertRaisesRegex(ValueError, 'unowned nonempty'):
            target.select(self.root, self.first)
        for invalid in [b'{', b'\xff', b'[]', b'null',
                        json.dumps({'version': True, 'checkout': str(self.first.resolve())}).encode()]:
            marker.write_bytes(invalid)
            with self.assertRaises(ValueError):
                target.select(self.root, self.first)
            self.assertEqual(marker.read_bytes(), invalid)

    def test_concurrent_claim_publishes_one_complete_owner(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            paths = list(pool.map(lambda _: target.select(self.root, self.first), range(32)))
        self.assertEqual(len(set(paths)), 1)
        self.assertEqual(json.loads((paths[0] / target.MARKER).read_text()),
                         {'version': 1, 'checkout': str(self.first.resolve())})
        self.assertFalse(list(self.root.glob('.incant-owner-*')))

    def test_cli_refuses_without_stdout(self):
        path = target.select(self.root, self.first)
        (path / target.MARKER).write_text('{}')
        result = subprocess.run([sys.executable, target.__file__, '--root', str(self.root),
                                 '--checkout', str(self.first)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, '')
        self.assertIn('ownership does not match', result.stderr)


if __name__ == '__main__':
    unittest.main()
