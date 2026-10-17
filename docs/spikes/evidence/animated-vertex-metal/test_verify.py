"""Coherent data mutations keep hashes aligned so semantic controls must refuse."""
import copy
import json
from pathlib import Path
import unittest
import verify


def put(records,name,value):records[name]=(json.dumps(value,indent=2)+'\n').encode()


def cohere(records):
    for phase in ('cpu','gpu'):
        owner=json.loads(records[phase+'-output/owner.json'])
        for call in owner['calls']:
            for stream in ('stdout','stderr'):
                call[stream]['sha256']=verify.sha(records[phase+'-output/'+call[stream]['path']])
        put(records,phase+'-output/owner.json',owner)
        result=json.loads(records[phase+'-result.json'])
        result['owner_sha256']=verify.sha(records[phase+'-output/owner.json'])
        for name,item in result['retained'].items():
            raw=records[phase+'-output/'+name];item['bytes']=len(raw);item['sha256']=verify.sha(raw)
        put(records,phase+'-result.json',result)
        close=json.loads(records[phase+'-terminal-closeout.json'])
        close['result_sha256']=verify.sha(records[phase+'-result.json'])
        close['owner_sha256']=result['owner_sha256']
        put(records,phase+'-terminal-closeout.json',close)
    review=json.loads(records['root-review.json'])
    for name in ('cpu-result.json','gpu-result.json'):review['records'][name]=verify.sha(records[name])
    put(records,'root-review.json',review)


class PortableControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.original,cls.manifest=verify.load(Path(__file__).parent)

    def test_actual_recorded_replay(self):
        self.assertTrue(verify.validate(self.original,self.manifest)['passed'])

    def test_exact_cleanup_inventory_is_required(self):
        for phase in ('cpu','gpu'):
            for empty in (False,True):
                with self.subTest(phase=phase,empty=empty):
                    records=copy.deepcopy(self.original)
                    key=phase+'-terminal-closeout.json';row=json.loads(records[key])
                    row['absence']=[] if empty else row['absence'][1:]
                    put(records,key,row);cohere(records)
                    with self.assertRaisesRegex(ValueError,'absence inventory'):verify.validate(records,self.manifest)

    def test_coherent_pixel_and_runtime_overclaims_refused(self):
        cases=[('geometry','full_frame_matches_fresh',False),('geometry','first_replacement_has_previous',True),
               ('geometry','continued_clock_seconds',.6),('geometry','vm_counter_after',2),
               ('tangents','pixel_oracle',{'kind':'tangent_only_shading','interior_changed':36,'material_image_position_normal_uv_identical':False,'old_w':1,'new_w':-1}),
               ('clip','shift_pixels',13)]
        for kind,field,value in cases:
            with self.subTest(kind=kind,field=field):
                records=copy.deepcopy(self.original);key='gpu-output/'+kind+'/proof.json'
                row=json.loads(records[key]);row[field]=value;put(records,key,row);cohere(records)
                with self.assertRaises(ValueError):verify.validate(records,self.manifest)

    def test_coherent_changed_fresh_or_retained_pixels_refused(self):
        for name in ('fresh_reference','invalid_topology_retained'):
            with self.subTest(name=name):
                records=copy.deepcopy(self.original);key='gpu-output/geometry/'+name+'.rgba'
                raw=bytearray(records[key]);raw[0]^=1;records[key]=bytes(raw);cohere(records)
                with self.assertRaises(ValueError):verify.validate(records,self.manifest)

    def test_success_stream_and_record_inventory_required(self):
        records=copy.deepcopy(self.original)
        records['gpu-output/000.stdout.log']=b'test result: FAILED. 0 passed; 1 failed;\n';cohere(records)
        with self.assertRaisesRegex(ValueError,'GPU test success'):verify.validate(records,self.manifest)
        records=copy.deepcopy(self.original);del records['gpu-output/000.stderr.log']
        with self.assertRaisesRegex(ValueError,'inventory'):verify.validate(records,self.manifest)

    def test_provenance_and_artifact_cannot_be_relabelled(self):
        manifest=copy.deepcopy(self.manifest);manifest['producer_head']='0'*40
        with self.assertRaisesRegex(ValueError,'producer'):verify.validate(self.original,manifest)
        manifest=copy.deepcopy(self.manifest);manifest['external_executable']['sha256']='0'*64
        with self.assertRaisesRegex(ValueError,'artifact identity'):verify.validate(self.original,manifest)

    def test_boolean_wrapper_identity_is_not_a_pid(self):
        records=copy.deepcopy(self.original)
        for key in ('cpu-output/owner.json','cpu-result.json','cpu-terminal-closeout.json'):
            row=json.loads(records[key]);row['wrapper_pid']=True;put(records,key,row)
        cohere(records)
        with self.assertRaisesRegex(ValueError,'wrapper identity'):verify.validate(records,self.manifest)


if __name__=='__main__':unittest.main()
