"""Saved-data refusal controls; no engine, GPU, window or subprocess execution."""
import copy
import json
import unittest
import verify


class SavedDataControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest,cls.records=verify.load()

    def inputs(self):
        get=lambda name:json.loads(self.records[verify.BASE+'window-002/'+name])
        return [get('presentations.json'),get('host-stages.json'),get('stdout.json'),
                get('callback-coverage.json'),get('compilations.json'),
                [json.loads(line) for line in self.records[verify.BASE+'window-002/window-state.jsonl'].splitlines()]]

    def test_complete_historical_replay(self):
        result=verify.validate(self.manifest,self.records)
        self.assertEqual(result,json.loads((verify.HERE/'summary.json').read_text()))
        self.assertFalse(result['performance_gate_passed'])

    def test_changed_duration_is_not_a_recomputed_fact(self):
        values=self.inputs()
        values[1]['summary']['reserve']['valid_subset_p95_nearest_rank_ns']+=1
        with self.assertRaisesRegex(ValueError,'Independent aggregate'):
            verify.calculate(*values)

    def test_zero_callback_cannot_be_silently_repaired(self):
        values=self.inputs()
        values[0]['rows'][0]['outcome']='presented'
        with self.assertRaisesRegex(ValueError,'Callback outcomes'):
            verify.calculate(*values)

    def test_different_scheduler_batches_do_not_reuse_this_summary(self):
        values=self.inputs()
        values[0]['host']['attempts'][2]['host_timing']['schedule']['due']=2
        with self.assertRaisesRegex(ValueError,'scheduler batch'):
            verify.calculate(*values)

    def test_first_failure_cannot_be_relabelled_complete(self):
        records=dict(self.records);key=verify.BASE+'window-001/result.json'
        previous=json.loads(records[key]);previous['status']='complete';records[key]=json.dumps(previous)
        with self.assertRaisesRegex(ValueError,'Prior partial failure'):
            verify.validate(self.manifest,records)

    def test_complete_host_data_does_not_pass_display_or_performance(self):
        for field in ['performance_gate_passed','display_cadence_measured']:
            changed=copy.deepcopy(self.manifest);changed[field]=True
            with self.assertRaisesRegex(ValueError,'Unsupported gate'):
                verify.validate(changed,self.records)

    def test_ambiguous_nonfinite_json_refuses(self):
        for text in ['{"a":1,"a":2}','{"a":NaN}','{"a":1e400}']:
            with self.assertRaises(ValueError):verify.decode(text)


if __name__=='__main__':unittest.main(verbosity=2)
