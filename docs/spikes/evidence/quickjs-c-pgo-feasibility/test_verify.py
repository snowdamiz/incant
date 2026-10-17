"""Saved-data refusal controls; no archived code or native process executes."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('verify', Path(__file__).with_name('verify.py'))
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class ReplayControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.records = verify.load(Path(__file__).parent)

    def mutate(self, name, change):
        records = dict(self.records)
        value = json.loads(records[name])
        change(value)
        records[name] = json.dumps(value).encode()
        with self.assertRaises(ValueError):
            verify.validate(self.manifest, records)

    def test_actual_packet(self):
        self.assertEqual(verify.validate(self.manifest, self.records)['actual_calls'], 57)

    def test_historical_failure_cannot_be_relabelled(self):
        self.mutate('run-003/result.json', lambda d: d.update(status='complete'))

    def test_specific_refusal_cannot_disappear(self):
        def change(calls):
            next(c for c in calls if c['name'] == 'stale-profile')['exit'] = 0
        self.mutate('run-004/managed-calls.json', change)

    def test_core_or_performance_scope_refused(self):
        self.mutate('run-004/result.json', lambda d: d.update(performance_claim=True))

    def test_heldout_mutation_refused(self):
        self.mutate('run-004/result.json', lambda d: d['heldout']['reports']['use'].update(actual=97915))

    def test_cleanup_unknown_is_not_absence(self):
        self.mutate('closeout-004.json', lambda d: d['independent_process_absence'][0].update(group_absent=False))

    def test_oversized_archive_refused_before_decode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'manifest.json').write_text(json.dumps(self.manifest))
            (root / 'records.json.gz').write_bytes(b'x' * (verify.PACKED_CAP + 1))
            with self.assertRaisesRegex(ValueError, 'exceeds bound'):
                verify.load(root)

    def test_oversized_manifest_and_duplicate_json_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'manifest.json').write_bytes(b' ' * (verify.MANIFEST_CAP + 1))
            with self.assertRaisesRegex(ValueError, 'exceeds bound'):
                verify.load(root)
        with self.assertRaises(ValueError):
            verify.decode(b'{"a":1,"a":2}')


if __name__ == '__main__':
    unittest.main()
