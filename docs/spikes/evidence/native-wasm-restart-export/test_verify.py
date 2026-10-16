"""Coherent data mutations: rehash records/receipts before testing semantic refusals."""
import copy
import unittest

import verify
from oracle import sha


class ReplayControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest, cls.records, cls.result = verify.verify()

    def setUp(self):
        # Text strings are immutable; replacing touched rows avoids copying large logs.
        self.records = dict(self.records)

    def data(self, name):
        return verify.strict_json(self.records[name]['text'])

    def put(self, name, value):
        import json
        self.records[name] = verify.entry(json.dumps(value, sort_keys=True))

    def text(self, name, value):
        self.records[name] = verify.entry(value)

    def reseal(self):
        owner = self.data('evidence/owner.json')
        for call in owner['calls']:
            for stream in ('stdout', 'stderr'):
                if stream in call:
                    call[stream]['sha256'] = self.records['evidence/' + call[stream]['path']]['sha256']
        self.put('evidence/owner.json', owner)
        closeout = self.data('closeout.json')
        for name, key in [('evidence/owner.json', 'owner_sha256'), ('evidence/result.json', 'result_sha256'),
                          ('wrapper-result.json', 'wrapper_result_sha256'), ('packet.json', 'packet_sha256')]:
            closeout[key] = self.records[name]['sha256']
        for index in range(3):
            report = self.records[f'evidence/run-{index}.json']
            closeout['runs'][index]['report'].update({k: report[k] for k in ('bytes', 'sha256')})
            pcm = bytes.fromhex(self.records[f'pcm/run-{index}.hex']['text'].strip())
            closeout['runs'][index]['pcm'].update(bytes=len(pcm), sha256=sha(pcm))
        self.put('closeout.json', closeout)
        root = self.data('root-independent-review.json')
        root['raw_owner_sha256'] = self.records['evidence/owner.json']['sha256']
        self.put('root-independent-review.json', root)
        manifest = copy.deepcopy(self.manifest)
        manifest['records'] = {name: {k: value[k] for k in ('bytes', 'sha256')}
                               for name, value in self.records.items()}
        verify.validate_records(manifest, self.records)

    def rejects(self, reason):
        self.reseal()
        with self.assertRaisesRegex(ValueError, reason):
            verify.replay(self.records)

    def mutate_report(self, change):
        for index in range(3):
            value = self.data(f'evidence/run-{index}.json')
            change(value)
            self.put(f'evidence/run-{index}.json', value)
            self.put(f'evidence/{index + 23:03d}.stdout.log', value)

    def test_actual_packet(self):
        self.assertEqual(self.result['restarts_each'], 3)

    def test_missing_restart_rejected_even_with_equal_runs(self):
        self.mutate_report(lambda report: report['session']['restarts'].pop())
        self.rejects('replacement boundaries')

    def test_reused_vm_observation_rejected(self):
        self.mutate_report(lambda report: report['observations']['checks'][1].update(actual=[2, 0, 0]))
        self.rejects('Fresh VM observation')

    def test_final_world_not_reset_rejected(self):
        self.mutate_report(lambda report: next(iter(report['observations']['entities'].values()))
                           .update(translation=[1, 0, 0]))
        self.rejects('Fresh final world')

    def test_boolean_counter_rejected(self):
        self.mutate_report(lambda report: report['session']['restarts'][0].update(host_step=True))
        self.rejects('replacement boundaries')

    def test_changed_pcm_with_updated_report_hash_rejected(self):
        pcm = b'\x01' + bytes(19199)
        self.text('pcm/run-1.hex', pcm.hex())
        value = self.data('evidence/run-1.json')
        value['pcm_sha256'] = sha(pcm)
        self.put('evidence/run-1.json', value)
        self.put('evidence/024.stdout.log', value)
        self.rejects('Exact silent PCM')

    def test_atomic_undo_failure_rejected(self):
        value = self.data('evidence/rpc-4.stderr.json')
        value['requests'][5]['response']['result']['project'] = value['requests'][3]['response']['result']['project']
        self.put('evidence/rpc-4.stderr.json', value)
        self.rejects('Atomic Undo')

    def test_reopen_mismatch_rejected(self):
        value = self.data('evidence/rpc-5.stderr.json')
        value['requests'][0]['response']['result']['revision'] = 7
        self.put('evidence/rpc-5.stderr.json', value)
        self.rejects('Journal reopen')

    def test_native_play_success_rejected(self):
        owner = self.data('evidence/owner.json')
        owner['calls'][4]['exit'] = 0
        self.put('evidence/owner.json', owner)
        self.rejects('Actual terminal')

    def test_wrong_checkpoint_refusal_rejected(self):
        self.text('evidence/026.stderr.log', 'Error: unrelated failure\n')
        self.rejects('Exact refusal')

    def test_unreaped_child_rejected(self):
        owner = self.data('evidence/owner.json')
        owner['owned'][0]['cleanup_confirmed'] = False
        self.put('evidence/owner.json', owner)
        self.rejects('Owned process cleanup')

    def test_weaker_actual_codegen_rejected(self):
        name = 'evidence/013.stderr.log'
        self.text(name, self.records[name]['text'].replace('-C lto=fat', '-C lto=thin'))
        self.rejects('Shipping codegen')

    def test_fixture_feature_rejected(self):
        name = 'evidence/013.stdout.log'
        self.text(name, self.records[name]['text'].replace('"static-module"', '"static-conformance"'))
        self.rejects('Exact actual artifact features')

    def test_authoring_dependency_rejected(self):
        name = 'evidence/012.stdout.log'
        self.text(name, self.records[name]['text'] + 'incant_doc v0.0.1|\n')
        self.rejects('Authoring/JIT normal dependency')

    def test_source_compiler_symbol_rejected(self):
        name = 'evidence/014.stdout.log'
        self.text(name, self.records[name]['text'] + '\n__JS_EvalInternal\n')
        self.rejects('Source compiler symbol')

    def test_runtime_source_file_rejected(self):
        value = self.data('evidence/package-files.json')
        value['guest.wasm'] = {'bytes': 1, 'sha256': '0' * 64, 'executable': False}
        self.put('evidence/package-files.json', value)
        self.rejects('Exact source-free runtime inventory')

    def test_observed_resource_overrun_rejected(self):
        value = self.data('evidence/owner.json')
        value['peaks']['target'] = value['target_ceiling_bytes'] + 1
        self.put('evidence/owner.json', value)
        self.rejects('Sampled limits')

    def test_missing_record_rejected(self):
        del self.records['evidence/026.stderr.log']
        with self.assertRaisesRegex(ValueError, 'Complete record inventory'):
            verify.validate_records(self.manifest, self.records)

    def test_duplicate_json_key_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Duplicate JSON key'):
            verify.strict_json('{"passed": false, "passed": true}')


if __name__ == '__main__':
    unittest.main()
