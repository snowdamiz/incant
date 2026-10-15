"""Data-only refusal controls; no native process or archived code execution."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('partial_verifier', Path(__file__).with_name('verify.py'))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.originals = verify.load()

    def changed(self, name, mutate):
        records = dict(self.originals)
        key = verify.BASE + 'window-001/' + name
        value = verify.decode(records[key])
        mutate(value)
        records[key] = json.dumps(value)
        self.rebind(records, name)
        return records

    def rebind(self, records, name):
        # Keep outer output/closeout hashes coherent so tests exercise semantics.
        base = verify.BASE + 'window-001/'
        result = verify.decode(records[base + 'result.json'])
        if name != 'result.json':
            raw = records[base + name].encode()
            result['artifacts'][name] = {'bytes': len(raw), 'sha256': verify.digest(raw)}
            records[base + 'result.json'] = json.dumps(result)
        closeout = verify.decode(records[base + 'root-closeout.json'])
        closeout['result_sha256'] = verify.digest(records[base + 'result.json'].encode())
        records[base + 'root-closeout.json'] = json.dumps(closeout)

    def test_original_partial_run_replays(self):
        result = verify.validate(self.manifest, self.originals)
        self.assertEqual(result['inactive_snapshot_count'], 93)
        self.assertFalse(result['full_gameplay_parity'])

    def test_completed_route_relabel_refuses(self):
        records = self.changed('result.json', lambda r: r.update(status='complete', exit_code=0))
        with self.assertRaisesRegex(ValueError, 'Failed run'):
            verify.validate(self.manifest, records)

    def test_timing_summary_mutation_refuses(self):
        records = self.changed('host-stages.json', lambda r: r['summary']['reserve'].update(valid_subset_max_ns=1))
        with self.assertRaisesRegex(ValueError, 'Recomputed host analysis'):
            verify.validate(self.manifest, records)

    def test_inactive_tail_cannot_claim_active_frames(self):
        records = dict(self.originals)
        name = 'window-state.jsonl'
        key = verify.BASE + 'window-001/' + name
        rows = [verify.decode(line) for line in records[key].splitlines()]
        for row in rows:
            if row['kind'] == 'snapshot' and row['state']['host_steps'] == 263:
                row['state']['native_focused'] = True
        records[key] = '\n'.join(json.dumps(row) for row in rows) + '\n'
        self.rebind(records, name)
        with self.assertRaisesRegex(ValueError, 'Inactive tail'):
            verify.validate(self.manifest, records)

    def test_gameplay_creation_claim_refuses(self):
        records = self.changed('compilations.json', lambda r: r['snapshot']['rows'][0].update(phase='Gameplay'))
        with self.assertRaisesRegex(ValueError, 'Observed native call scope'):
            verify.validate(self.manifest, records)

    def test_performance_claim_refuses(self):
        manifest = copy.deepcopy(self.manifest)
        manifest['performance_gate_passed'] = True
        with self.assertRaisesRegex(ValueError, 'Unsupported acceptance claim'):
            verify.validate(manifest, self.originals)


if __name__ == '__main__':
    unittest.main(verbosity=2)
