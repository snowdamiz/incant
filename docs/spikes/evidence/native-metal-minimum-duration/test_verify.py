"""Saved-data refusal controls; no native execution."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('minimum_duration_packet', HERE / 'verify.py')
packet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packet)


class SavedPacket(unittest.TestCase):
    def test_complete_saved_packet(self):
        packet.validate(*packet.load())

    def test_failed_selection_cannot_be_relabelled_as_success(self):
        manifest, records = packet.load()
        result = packet.decode(records['cpu-002/result.json'])
        result['status'] = 'complete'
        records['cpu-002/result.json'] = json.dumps(result).encode()
        with self.assertRaisesRegex(ValueError, 'Cargo selection failure retained'):
            packet.validate(manifest, records)

    def test_wrong_HAL_feature_graph_refuses(self):
        manifest, records = packet.load()
        result = packet.decode(records['cpu-003/result.json'])
        result['stages'][0]['HAL_cargo_artifact']['features'].remove('diagnostic-minimum-duration')
        records['cpu-003/result.json'] = json.dumps(result).encode()
        with self.assertRaisesRegex(ValueError, 'Exact primary HAL features'):
            packet.validate(manifest, records)

    def test_runtime_scope_overclaim_refuses(self):
        manifest, records = packet.load()
        manifest['native_window_executed'] = True
        with self.assertRaisesRegex(ValueError, 'Unsupported runtime or performance claim'):
            packet.validate(manifest, records)

    def test_oversized_compressed_input_refuses_before_decode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'manifest.json').write_bytes((HERE / 'manifest.json').read_bytes())
            with (root / 'records.json.gz').open('wb') as stream:
                stream.seek(4 * 1024**2)
                stream.write(b'x')
            with patch.object(packet, 'HERE', root), self.assertRaisesRegex(ValueError, 'Input exceeds byte cap'):
                packet.load()

    def test_original_read_exact_bound_and_trailing_byte(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'record'
            path.write_bytes(b'exact')
            self.assertEqual(packet.read_bounded(path, 5), b'exact')
            path.write_bytes(b'exact!')
            with self.assertRaisesRegex(ValueError, 'Input exceeds byte cap'):
                packet.read_bounded(path, 5)


if __name__ == '__main__':
    unittest.main()
