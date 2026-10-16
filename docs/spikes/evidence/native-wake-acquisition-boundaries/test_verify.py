"""Portable reader controls; no native code, player or capture executes."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('wake_packet', HERE / 'verify.py')
packet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packet)


class BoundedReaders(unittest.TestCase):
    def test_oversized_archive_refuses_before_gzip_or_json_decode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'manifest.json').write_bytes((HERE / 'manifest.json').read_bytes())
            # Real oversized file, not a forged length field in otherwise small JSON.
            with (root / 'records.json.gz').open('wb') as stream:
                stream.seek(2 * 1024**2)
                stream.write(b'x')
            with patch.object(packet, 'HERE', root), self.assertRaisesRegex(ValueError, 'Input exceeds byte cap'):
                packet.load()

    def test_manifest_and_original_exact_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'record'
            path.write_bytes(b'exact')
            self.assertEqual(packet.read_bounded(path, 5), b'exact')
            with path.open('ab') as stream:
                stream.write(b'!')
            with self.assertRaisesRegex(ValueError, 'Input exceeds byte cap'):
                packet.read_bounded(path, 5)
            with path.open('wb') as stream:
                stream.seek(256 * 1024)
                stream.write(b'x')
            with self.assertRaisesRegex(ValueError, 'Input exceeds byte cap'):
                packet.read_bounded(path, 256 * 1024)


if __name__ == '__main__':
    unittest.main()
