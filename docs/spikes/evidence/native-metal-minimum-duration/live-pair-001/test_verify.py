"""Bounded saved-evidence controls; no compiler, player, GPU or window."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('pair_packet',HERE/'verify.py')
packet=importlib.util.module_from_spec(spec);spec.loader.exec_module(packet)


class SavedPair(unittest.TestCase):
    def setUp(self):
        self.manifest,self.records=packet.load()

    def test_original_failed_treatment_and_arithmetic_replay(self):
        _,_,result=packet.validate(self.manifest,self.records)
        self.assertEqual(result['treatment']['original_controller_status'],'failed')
        self.assertGreater(result['differences']['presentation_span_ns'],0)
        self.assertEqual(result['differences']['dropped_wall_ticks'],4)

    def test_failed_treatment_cannot_be_relabelled(self):
        key=self.manifest['original_root']+'/window-minimum-duration60-001/result.json'
        value=packet.decode(self.records[key]);value['status']='complete'
        self.records[key]=json.dumps(value).encode()
        with self.assertRaisesRegex(ValueError,'Original native acceptance/cleanup'):
            packet.validate(self.manifest,self.records)

    def test_adoption_claim_refuses(self):
        self.manifest['policy_adopted']=True
        with self.assertRaisesRegex(ValueError,'Unsupported adoption/gate claim'):
            packet.validate(self.manifest,self.records)

    def test_missing_zero_reordered_or_boundary_callbacks_are_not_filtered(self):
        packet.validate(self.manifest,self.records)
        import analyze
        raw=packet.decode(self.records[self.manifest['original_root']+'/window-ordinary-001/presentations.json'])
        attempts={a['id']:a for a in raw['host']['attempts']};rows=raw['rows']
        index=next(i for i,r in enumerate(rows) if r['requested_host_step']==10)
        for change in ['missing','zero','reorder','boundary']:
            edited=copy.deepcopy(rows)
            if change=='missing':edited.pop(index)
            elif change=='zero':edited[index].update(outcome='reported_zero',presented_seconds=None)
            elif change=='reorder':edited[index],edited[index+1]=edited[index+1],edited[index]
            else:edited[index]['segment']+=1
            with self.assertRaises(ValueError):analyze.cadence(edited,attempts)

    def test_independent_clock_offsets_leave_relative_proxy_unchanged(self):
        packet.validate(self.manifest,self.records)
        import analyze
        raw=packet.decode(self.records[self.manifest['original_root']+'/window-ordinary-001/presentations.json'])
        attempts={a['id']:a for a in raw['host']['attempts']};rows=raw['rows']
        expected=analyze.cadence(rows,attempts);edited=copy.deepcopy(rows);host=copy.deepcopy(attempts)
        for row in edited:
            if row['presented_seconds'] is not None:row['presented_seconds']+=1000
        for attempt in host.values():
            if attempt['host_timing']['finished_ns'] is not None:attempt['host_timing']['finished_ns']+=10**12
        actual=analyze.cadence(edited,host)
        self.assertEqual(expected['relative_presentation_minus_host_span_proxy_ns'],
                         actual['relative_presentation_minus_host_span_proxy_ns'])

    def test_oversized_archive_refuses_before_decompression(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            (root/'manifest.json').write_bytes((HERE/'manifest.json').read_bytes())
            with (root/'records.json.gz').open('wb') as stream:
                stream.seek(8*1024**2);stream.write(b'x')
            with patch.object(packet,'HERE',root),self.assertRaisesRegex(ValueError,'Input exceeds byte cap'):
                packet.load()


if __name__=='__main__':unittest.main()
