"""Actual portable packet and coherent recorded-data refusal controls."""
import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools'))
from probes.native_wasm_checkpoint import portable as verify


class PortableCheckpointTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.original, cls.outcome = verify.verify()

    def setUp(self):
        self.records = dict(self.original)

    def data(self, name):
        return verify.strict_json(verify.payload(self.records[name]))

    def put(self, name, value):
        self.records[name] = verify.entry(json.dumps(value, sort_keys=True).encode())

    def bound(self, name):
        return {key: self.records[name][key] for key in ('bytes', 'sha256')}

    def reseal(self):
        owner = self.data('output/owner.json')
        for call in owner['calls']:
            for stream in ('stdout', 'stderr'):
                if stream in call:
                    call[stream]['sha256'] = self.records['output/' + call[stream]['path']]['sha256']
        self.put('output/owner.json', owner)
        result = self.data('result.json')
        result['owner_sha256'] = self.records['output/owner.json']['sha256']
        result['process_inventory_sha256'] = self.records['output/process-inventory.jsonl']['sha256']
        self.put('result.json', result)
        terminal = self.data('terminal-closeout.json')
        for name, field in [('result.json', 'result'), ('output/owner.json', 'owner'),
                            ('output/process-inventory.jsonl', 'inventory')]:
            terminal[field].update(self.bound(name))
        self.put('terminal-closeout.json', terminal)
        receipt = self.data('correction/receipt.json')
        receipt['original_result'].update(self.bound('result.json'))
        receipt['original_terminal_closeout'].update(self.bound('terminal-closeout.json'))
        self.put('correction/receipt.json', receipt)
        packet = self.data('packet.json')
        for name, expected in packet['files'].items():
            expected.update(self.bound(name))
        self.put('packet.json', packet)
        grant = self.data('root-grant.json'); grant['packet_sha256'] = self.records['packet.json']['sha256']
        self.put('root-grant.json', grant)
        original = self.data('original-records.json')
        for name, expected in original.items():
            if name in self.records:
                expected.update(self.bound(name))
        self.put('original-records.json', original)
        manifest = copy.deepcopy(self.manifest)
        manifest['records'] = {name: self.bound(name) for name in self.records}
        verify.validate_records(manifest, self.records)

    def rejects(self, message):
        self.reseal()
        with self.assertRaisesRegex(ValueError, message):
            verify.replay(self.records)

    def test_actual_records_and_honest_scope(self):
        self.assertEqual(self.outcome['positive_processes'], 4)
        self.assertEqual(self.outcome['precise_refusals'], 6)
        self.assertEqual(self.outcome['original_native_exit'], 1)
        self.assertFalse(self.outcome['external_artifact_bytes_rechecked'])
        self.assertFalse(self.outcome['filesystem_process_absence_reobserved'])

    def test_missing_stream_record_refuses(self):
        del self.records['output/019.stdout.log']
        with self.assertRaisesRegex(ValueError, 'Complete record inventory'):
            verify.validate_records(self.manifest, self.records)

    def test_changed_merged_stream_without_owner_binding_refuses(self):
        self.records['output/019.stdout.log'] = verify.entry(b'changed symbol output')
        original = self.data('original-records.json')
        original['output/019.stdout.log'].update(self.bound('output/019.stdout.log'))
        self.put('original-records.json', original)
        with self.assertRaisesRegex(ValueError, 'log identity'):
            verify.replay(self.records)

    def test_missing_ordinary_stderr_cannot_be_reclassified(self):
        owner = self.data('output/owner.json'); del owner['calls'][27]['stderr']
        self.put('output/owner.json', owner)
        self.rejects('Seven merged')

    def test_changed_refusal_stream_rejected_after_receipt_reseal(self):
        changed = 'Error: unrelated failure\n'
        self.records['output/031.stderr.log'] = verify.entry(changed.encode())
        for name in ('output/corrupt-refusal.json', 'output/corrupt-process.json'):
            row = self.data(name); row['stderr'] = changed; self.put(name, row)
        self.rejects('Exact checkpoint refusal')

    def test_wrong_native_source_head_refuses_even_after_rehash(self):
        source = self.data('source.json'); source['head'] = '0' * 40; self.put('source.json', source)
        self.rejects('Exact source producers')

    def test_changed_sdk_provenance_refuses(self):
        guest = self.data('output/guest.json')
        guest['sdk']['source_sha256']['sdk/rust/src/persistent.rs'] = '0' * 64
        self.put('output/guest.json', guest)
        self.rejects('Guest SDK provenance')

    def test_wrong_correction_producer_refuses(self):
        receipt = self.data('correction/receipt.json'); receipt['verifier_source'] = verify.HEAD
        self.put('correction/receipt.json', receipt)
        self.rejects('Correction provenance')

    def test_original_failure_cannot_be_relabeled_success(self):
        result = self.data('result.json'); result['passed'] = True; self.put('result.json', result)
        self.rejects('Original native failure retained')

    def test_empty_or_omitted_observed_pid_refuses(self):
        for omission in (lambda rows: [], lambda rows: rows[:-1]):
            self.records = dict(self.original)
            terminal = self.data('terminal-closeout.json'); terminal['absence'] = omission(terminal['absence'])
            self.put('terminal-closeout.json', terminal)
            self.rejects('Exact observed PID absence')

    def test_altered_recorded_player_identity_refuses(self):
        original = self.data('original-records.json')
        original['output/package/player/incant_game_player']['sha256'] = '0' * 64
        self.put('original-records.json', original)
        self.rejects('Recorded artifact inventory')

    def test_duplicate_keys_and_unsafe_paths_refuse(self):
        with self.assertRaisesRegex(ValueError, 'Duplicate JSON'):
            verify.strict_json('{"passed":false,"passed":true}')
        self.records['../escaped'] = verify.entry(b'')
        manifest = copy.deepcopy(self.manifest)
        manifest['records']['../escaped'] = self.bound('../escaped')
        with self.assertRaisesRegex(ValueError, 'Safe inert record path'):
            verify.validate_records(manifest, self.records)


if __name__ == '__main__':
    unittest.main()
