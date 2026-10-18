"""Retained real export and coherent negative timer/checkpoint evidence controls."""
import copy
import json
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools'))
from probes.native_wasm_checkpoint import timer_portable as verify
from probes.native_wasm_checkpoint import portable as raw
from probes.native_wasm_checkpoint import oracle


class TimerExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.original, cls.outcome = verify.verify()

    def setUp(self):
        self.records = dict(self.original)

    def data(self, name):
        return raw.strict_json(raw.payload(self.records[name]))

    def put(self, name, value):
        self.records[name] = raw.entry(json.dumps(value).encode())

    def test_actual_export_records_and_scope(self):
        self.assertEqual(self.outcome['actual_native_exit'], 0)
        self.assertEqual(self.outcome['positive_processes'], 4)
        self.assertEqual(self.outcome['precise_refusals'], 6)
        self.assertTrue(self.outcome['timer_schedules_and_clock_exact'])
        self.assertFalse(self.outcome['external_artifacts_rechecked'])
        self.assertFalse(self.outcome['process_absence_reobserved'])
        self.assertFalse(self.outcome['performance_or_phase_gate'])

    def test_missing_output_refuses_before_replay(self):
        del self.records['output/baseline.pcm']
        with self.assertRaisesRegex(ValueError, 'Complete record inventory'):
            raw.validate_records(self.manifest, self.records)

    def test_wrong_native_producer_refuses(self):
        source = self.data('source.json'); source['head'] = '0' * 40
        self.put('source.json', source)
        with self.assertRaisesRegex(ValueError, 'Exact native producing source'):
            verify.replay(self.records)

    def test_failed_terminal_cannot_be_reclassified(self):
        terminal = self.data('terminal-closeout.json'); terminal['actual_exec_terminal_exit'] = 1
        self.put('terminal-closeout.json', terminal)
        with self.assertRaisesRegex(ValueError, 'Actual successful native terminal'):
            verify.replay(self.records)

    def test_missing_observed_process_refuses(self):
        terminal = self.data('terminal-closeout.json'); terminal['absence'] = terminal['absence'][:-1]
        self.put('terminal-closeout.json', terminal)
        with self.assertRaisesRegex(ValueError, 'Exact observed PID absence'):
            verify.replay(self.records)

    def test_coherent_pending_timer_corruption_refuses(self):
        saved = bytearray(raw.payload(self.records['output/prefix.save']))
        start = 128 + struct.unpack_from('<I', saved, 80)[0]
        # First lexical entry is the future timer. Change due tick and reseal the
        # ordinary checkpoint integrity field, leaving the pending event wrong.
        struct.pack_into('<Q', saved, start + 104 + 32 + 72, 102)
        saved[96:128] = bytes.fromhex(oracle.sha(saved[:96] + saved[128:]))
        self.records['output/prefix.save'] = raw.entry(bytes(saved), binary=True)
        with self.assertRaisesRegex(ValueError, 'Exact pending timer schedule'):
            verify.replay(self.records)

    def test_legacy_state_cannot_count_as_timer_evidence(self):
        saved = bytearray(raw.payload(self.records['output/prefix.save']))
        lengths = list(struct.unpack_from('<4I', saved, 80)); start = 128 + lengths[0]
        legacy = saved[start + 8:start + 104]
        saved[start:start + lengths[1]] = legacy
        lengths[1] = len(legacy); struct.pack_into('<4I', saved, 80, *lengths)
        saved[96:128] = bytes.fromhex(oracle.sha(saved[:96] + saved[128:]))
        self.records['output/prefix.save'] = raw.entry(bytes(saved), binary=True)
        with self.assertRaisesRegex(ValueError, 'Current timer-bearing checkpoint'):
            verify.replay(self.records)

    def test_unbound_managed_log_refuses(self):
        owner = self.data('output/owner.json')
        name = 'output/' + owner['calls'][-1]['stderr']['path']
        self.records[name] = raw.entry(b'Error: unrelated refusal\n')
        with self.assertRaisesRegex(ValueError, 'Actual log identity'):
            verify.replay(self.records)

    def test_unsafe_record_path_and_duplicate_json_refuse(self):
        with self.assertRaisesRegex(ValueError, 'Duplicate JSON key'):
            raw.strict_json('{"passed":false,"passed":true}')
        self.records['../escape'] = raw.entry(b'')
        manifest = copy.deepcopy(self.manifest)
        manifest['records']['../escape'] = raw.identity(b'')
        with self.assertRaisesRegex(ValueError, 'Safe inert record path'):
            raw.validate_records(manifest, self.records)


if __name__ == '__main__':
    unittest.main()
