"""Portable parser and owned-process controls; no compiler, GPU or window."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('dx12_probe', ROOT / 'tools/probes/dx12-pipeline-library.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class ProbeControls(unittest.TestCase):
    def cpu_text(self):
        names = ['test pipeline_cache::' + name + ' ... ok' for name in sorted(probe.REQUIRED_TESTS)]
        return '\n'.join(names + [f'test result: ok. {len(names)} passed; 0 failed; 0 ignored;'])

    def test_required_real_descriptor_COM_and_ACL_test_membership(self):
        text = self.cpu_text()
        self.assertEqual(probe.validate_cpu_log(text), len(probe.REQUIRED_TESTS))
        for changed in (text.replace(' ... ok', ' ... FAILED', 1), text.replace('0 ignored', '1 ignored'),
                        text.replace('0 failed', '1 failed'), text + '\n' + text,
                        text + '\n' + text.splitlines()[0],
                        'test result: ok. 0 passed; 0 failed; 0 ignored;'):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                probe.validate_cpu_log(changed)

    def test_actual_artifact_features_path_and_uniqueness_are_required(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp)
            path = target / 'renderer.exe'
            path.write_bytes(b'synthetic linked bytes')
            row = {'reason': 'compiler-artifact', 'target': {'name': 'incant_render'},
                   'profile': {'test': True}, 'executable': str(path), 'features': sorted(probe.BASE_FEATURES)}
            self.assertEqual(probe.artifact(json.dumps(row), False, target)[0], path)
            with self.assertRaises(ValueError): probe.artifact(json.dumps(row), True, target)
            for key, value in (('executable', str(target.parent / 'escape.exe')),
                               ('features', sorted(probe.BASE_FEATURES | {'physics-fast-simd8'}))):
                changed = dict(row, **{key: value})
                with self.subTest(key=key), self.assertRaises(ValueError):
                    probe.artifact(json.dumps(changed), False, target)
            with self.assertRaises(ValueError): probe.artifact(json.dumps(row) + '\n' + json.dumps(row), False, target)

    def counts(self):
        return dict.fromkeys(probe.COUNTERS, 0) | {'disabled': False, 'overflowed': False, 'last_error': None}

    def test_native_hits_never_accept_fallback_missing_disabled_or_overflow(self):
        counts = self.counts() | {'stream_hits': 10, 'compute_hits': 1}
        probe.validate_counts(counts, 'reuse-stream')
        for key, value in (('fallback_attempts', 1), ('stores', 1), ('miss_or_mismatch', 1),
                           ('compute_hits', 0), ('classic_hits', 1), ('stream_hits', 0),
                           ('disabled', True), ('overflowed', True), ('last_error', 'failure'),
                           ('native_load_errors', 1), ('serialization_errors', 1), ('store_errors', 1),
                           ('stream_hits', True), ('stream_hits', -1), ('stream_hits', 2**64)):
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                probe.validate_counts(counts | {key: value}, 'reuse-stream')
        probe.validate_counts(self.counts() | {'classic_hits': 10, 'compute_hits': 1}, 'reuse-classic')
        probe.validate_counts(self.counts() | {'fallback_attempts': 10, 'stores': 10, 'miss_or_mismatch': 10,
            'last_error': 'Native { operation: "LoadPipeline", code: -2147024809 }'}, 'seed')

    def test_late_refusal_retains_reason_and_cannot_store_or_fall_back(self):
        counts = self.counts() | {'strict_refusals': 1, 'miss_or_mismatch': 1,
                                'last_error': 'Native { operation: "LoadPipeline", code: -2147024809 }'}
        probe.validate_counts(counts, 'reuse-stream', final=True)
        probe.validate_counts(counts | {'disabled': True, 'miss_or_mismatch': 0, 'last_error': None}, 'disabled', final=True)
        for key, value in (('strict_refusals', 0), ('fallback_attempts', 1), ('stores', 1),
                           ('last_error', None), ('miss_or_mismatch', 0), ('disabled', True)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                probe.validate_counts(counts | {key: value}, 'reuse-stream', final=True)

    def frame(self):
        raw = bytearray([0, 0, 0, 255] * (160 * 96))
        for channel in range(3):
            for pixel in range(60): raw[((channel * 60 + pixel) * 4) + channel] = 240
        return raw

    def test_raw_parity_rejects_changed_pixel_alpha_size_and_no_effect_fixture(self):
        with tempfile.TemporaryDirectory() as temp:
            a, b = Path(temp) / 'a.rgba', Path(temp) / 'b.rgba'
            a.write_bytes(self.frame()); b.write_bytes(self.frame())
            probe.compare_frames([a, b])
            for raw in (self.frame()[:-1], bytearray([0, 0, 0, 255] * (160 * 96))):
                b.write_bytes(raw)
                with self.assertRaises(ValueError): probe.compare_frames([a, b])
            for offset in (0, 3):
                raw = self.frame(); raw[offset] ^= 1; b.write_bytes(raw)
                with self.assertRaises(ValueError): probe.compare_frames([a, b])
                a.write_bytes(raw)
                if offset == 3:
                    with self.assertRaises(ValueError): probe.compare_frames([a, b])
                a.write_bytes(self.frame())

    def test_cleanup_confirmation_requires_original_stop_return(self):
        class Tree:
            def __init__(self): self.process = type('Process', (), {'pid': 123})()
            def stop(self): raise OSError('cleanup refused')
        rows = []
        observer = probe.CleanupObserver(Tree, rows, lambda: None)
        observer.install()
        try:
            tree = Tree()
            with self.assertRaises(OSError): tree.stop()
            self.assertFalse(rows[0]['cleanup_confirmed'])
            self.assertIn('cleanup refused', rows[0]['cleanup_error'])
        finally: observer.restore()

    def test_receipt_failure_during_construction_still_retires_owned_child(self):
        stopped = []
        class Tree:
            def __init__(self): self.process = type('Process', (), {'pid': 123})()
            def stop(self): stopped.append(True)
        observer = probe.CleanupObserver(Tree, [], mock.Mock(side_effect=OSError('receipt failed')))
        observer.install()
        try:
            with self.assertRaises(OSError): Tree()
            self.assertEqual(stopped, [True])
            self.assertTrue(observer.rows[0]['cleanup_confirmed'])
        finally: observer.restore()

    def test_harmless_real_child_success_and_timeout_are_reaped_by_existing_owner(self):
        tree = probe.native_process._WindowsTree if os.name == 'nt' else probe.native_process._PosixTree
        rows = []
        observer = probe.CleanupObserver(tree, rows, lambda: None)
        observer.install()
        try:
            self.assertEqual(probe.native_process.run([sys.executable, '-c', 'print("done")']).stdout, 'done\n')
            with self.assertRaises(__import__('subprocess').TimeoutExpired):
                probe.native_process.run([sys.executable, '-c', 'import time; time.sleep(30)'], timeout=0.1)
            self.assertEqual(len(rows), 2)
            self.assertTrue(all(row['cleanup_confirmed'] for row in rows))
        finally: observer.restore()

    def test_workflow_remains_manual_and_uploads_failed_receipts(self):
        text = (ROOT / '.github/workflows/dx12-pipeline-library-diagnostic.yml').read_text()
        self.assertIn('workflow_dispatch:', text)
        self.assertNotIn('pull_request:', text)
        self.assertNotIn('push:', text)
        self.assertIn('if: always()', text)
        self.assertIn('exit $LASTEXITCODE', text)
        self.assertIn('permissions:\n  contents: read', text)
        self.assertIn('--expected-source "$env:GITHUB_SHA"', text)


if __name__ == '__main__':
    unittest.main()
