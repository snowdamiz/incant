"""Exercise the imported-model receipt with the actual checkpoint11 field set."""
from pathlib import Path
from contextlib import redirect_stdout
import io
import json
import runpy
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zlib

ROOT = Path(__file__).resolve().parents[2]
PROBE = runpy.run_path(str(ROOT / 'tools/platforms/render-check.py'))
validate = PROBE['validate_model_geometry']
edit = PROBE['edit']
# Actual Windows checkpoint11 response. This model-only fixture has no text.
GEOMETRY = {
    'diagnostic_entities': 0, 'model_draw_calls': 1, 'model_entities': 2,
    'model_triangles': 2, 'primitive_instances': 2, 'sprite_entities': 0,
    'text_entities': 0, 'text_glyph_quads': 0, 'text_gpu_bytes': 0,
    'text_missing_glyphs': 0, 'tile_cells': 0, 'tilemap_entities': 0,
}


class ModelGeometryReceipt(unittest.TestCase):
    def test_observed_model_only_response_is_accepted(self):
        validate(GEOMETRY)

    def test_all_counts_and_complete_field_set_are_required(self):
        for field in GEOMETRY:
            with self.subTest(field=field):
                wrong = dict(GEOMETRY)
                wrong[field] += 1
                with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
                    validate(wrong)
                missing = dict(GEOMETRY)
                del missing[field]
                with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
                    validate(missing)
        with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
            validate(dict(GEOMETRY, unknown_count=0))


class RendererEditRevision(unittest.TestCase):
    def response(self, responses):
        return subprocess.CompletedProcess([], 0, ''.join(json.dumps(r) + '\n' for r in responses), '')

    def test_reopened_edits_carry_returned_revision_and_preserve_explicit_journal(self):
        revisions = []
        def rpc(argv, **kwargs):
            self.assertEqual(argv, ['binary', 'rpc', 'project', '--journal', 'journal'])
            requests = [json.loads(line) for line in kwargs['input'].splitlines()]
            self.assertEqual([r['method'] for r in requests], ['command.execute', 'project.save'])
            self.assertEqual(requests[1]['params'], {})
            expected = requests[0]['params']['expected_revision']
            self.assertEqual(expected, len(revisions))
            revisions.append(expected + 1)
            return self.response([{'id': 1, 'result': {'revision': revisions[-1]}},
                                  {'id': 2, 'result': {'saved': True}}])
        with patch('subprocess.run', side_effect=rpc):
            revision = edit('binary', 'project', [{'op': 'first'}], journal='journal')
            self.assertEqual(revision, 1)
            self.assertEqual(edit('binary', 'project', [{'op': 'second'}], revision, 'journal'), 2)
        self.assertEqual(revisions, [1, 2])

    def test_rpc_failure_retains_message_and_code_presence_without_retry(self):
        for error in [{'message': 'expected revision 0, current 1'},
                      {'code': -32000, 'message': 'stale revision'}]:
            with self.subTest(error=error):
                result = self.response([{'id': 1, 'error': error}, {'id': 2, 'result': {'saved': True}}])
                with patch('subprocess.run', return_value=result) as rpc:
                    with self.assertRaises(ValueError) as raised:
                        edit('binary', 'project', [])
                detail = json.loads(str(raised.exception).split(': ', 1)[1])
                self.assertEqual(detail['errors'], [{'id': 1, 'error': error, 'code_present': 'code' in error}])
                self.assertEqual(detail['response_count'], 2)
                self.assertEqual(rpc.call_count, 1)

    def test_main_carries_first_receipt_into_second_edit_after_capture(self):
        # Synthetic renderer transport exercises the real main/edit call chain;
        # this test never renders or counts as a hardware/appearance gate.
        def png(value):
            def chunk(kind, data):
                return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
            header = struct.pack('>IIBBBBB', 320, 180, 8, 2, 0, 0, 0)
            pixels = (b'\0' + bytes([value]) * 320 * 3) * 180
            return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(pixels)) + chunk(b'IEND', b'')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, output = root / 'binary', root / 'output'
            binary.touch()
            revision = 0
            seen = []
            def run(argv, **kwargs):
                nonlocal revision
                if argv[1] == 'init':
                    Path(argv[2]).write_text(json.dumps({'scenes': {'scene': {'id': 'scene', 'entities': {'parent': {}, 'child': {}, 'third': {}}}}}))
                    return subprocess.CompletedProcess(argv, 0)
                if argv[1] == 'screenshot':
                    Path(argv[3]).write_bytes(png(1))
                    return subprocess.CompletedProcess(argv, 0)
                self.assertEqual(argv[1], 'rpc')
                requests = [json.loads(line) for line in kwargs['input'].splitlines()]
                actual = requests[0]['params']['expected_revision']
                seen.append(actual)
                if actual != revision:
                    return self.response([{'id': 1, 'error': {'message': 'stale persisted revision'}}, {'id': 2, 'result': {'saved': True}}])
                revision += 1
                return self.response([{'id': 1, 'result': {'revision': revision}}, {'id': 2, 'result': {'saved': True}}])
            def first_capture(argv, **kwargs):
                self.assertEqual(argv[1], 'screenshot')
                Path(argv[3]).write_bytes(png(0))
                return json.dumps({'mocked_renderer_transport': True})
            with patch('sys.argv', ['render-check', '--binary', str(binary), '--output', str(output)]), \
                    patch('subprocess.run', side_effect=run), \
                    patch('subprocess.check_output', side_effect=first_capture), \
                    patch.dict(PROBE['main'].__globals__, {'model_probe': lambda *args: {'mocked': True}}), \
                    redirect_stdout(io.StringIO()):
                PROBE['main']()
            self.assertEqual(seen, [0, 1])
            self.assertEqual(revision, 2)
            report = json.loads((output / 'report.json').read_text())
            self.assertTrue(report['hierarchy_matches_flattened'])
            self.assertTrue(report['hierarchy_differs_from_initial'])

    def test_incomplete_or_malformed_edit_save_receipts_refuse(self):
        valid = [{'id': 1, 'result': {'revision': 1}}, {'id': 2, 'result': {'saved': True}}]
        candidates = [valid[:1], [None, valid[1]], [valid[1], valid[0]],
                      [{'id': True, 'result': {'revision': 1}}, valid[1]],
                      [{'id': 1.0, 'result': {'revision': 1}}, valid[1]],
                      [{'id': 1, 'result': {}}, valid[1]],
                      [valid[0], {'id': 2, 'result': {'saved': False}}]]
        for revision in [True, 1.0, -1, None, '1']:
            candidates.append([{'id': 1, 'result': {'revision': revision}}, valid[1]])
        for responses in candidates:
            with self.subTest(responses=responses), patch('subprocess.run', return_value=self.response(responses)):
                with self.assertRaises(ValueError):
                    edit('binary', 'project', [])


if __name__ == '__main__':
    unittest.main()
