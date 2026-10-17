#!/usr/bin/env python3
"""Portable corruption controls; no compiler, native process or window."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import verify

class Packet(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest,cls.records,_=verify.load()

    def test_valid_packet(self):
        verify.validate(self.manifest,self.records)

    def mutate(self,name,change):
        records=dict(self.records);value=json.loads(records[name]);change(value)
        records[name]=json.dumps(value).encode()
        with self.assertRaises((ValueError,KeyError,TypeError)):
            verify.validate(self.manifest,records)

    def test_preserve_original_wrapper_failure(self):
        self.mutate('window-001/result.json',lambda v:v.update(status='complete',exit_code=0))

    def test_gameplay_divergence_refused(self):
        self.mutate('window-001/stdout.json',lambda v:v['observations'].update(passed=False))

    def test_unknown_audio_field_refused(self):
        self.mutate('window-001/stdout.json',lambda v:v['audio_device']['diagnostics'].update(unknown=0))

    def test_audio_boolean_counter_refused(self):
        self.mutate('window-001/stdout.json',lambda v:v['audio_device']['diagnostics'].update(route_change_notifications=True))

    def test_partial_native_route_refused(self):
        self.mutate('window-001/stdout.json',lambda v:v.update(host_steps=2155))

    def test_tail_partition_mutation_refused(self):
        self.mutate('saved-reanalysis-001/result.json',lambda v:v['same_frame_tail_partitions']['wake']['sums_ns'].update(before_new_events_ns=0))

    def test_display_gate_overclaim_refused(self):
        m=copy.deepcopy(self.manifest);m['continuous_display_qualified']=True
        with self.assertRaises(ValueError):verify.validate(m,self.records)

    def test_oversized_read_refused(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'oversized';p.write_bytes(b'x'*9)
            with self.assertRaises(ValueError):verify.bounded(p,8)

    def test_duplicate_and_nonfinite_json_refused(self):
        for value in (b'{"x":1,"x":2}',b'{"x":NaN}'):
            with self.subTest(value=value),self.assertRaises(ValueError):verify.decode(value)

if __name__=='__main__':unittest.main()
