"""Small coherent-mutation controls for portable record admission."""
import copy
import json
from pathlib import Path
import unittest
import verify


def put(records,name,value):records[name]=(json.dumps(value,indent=2)+'\n').encode()


def cohere(records):
    outcome=json.loads(records['window-output/workflow/result.json'])
    outcome['window']=json.loads(records['window-output/workflow/native-window.json'])
    put(records,'window-output/workflow/result.json',outcome)
    result=json.loads(records['window-result.json'])
    result['proof_sha256']=verify.sha(records['window-output/workflow/result.json'])
    for name,row in result['retained'].items():
        key='window-output/'+name
        if key in records:row['bytes']=len(records[key]);row['sha256']=verify.sha(records[key])
    put(records,'window-result.json',result)
    close=json.loads(records['window-terminal-closeout.json']);close['result_sha256']=verify.sha(records['window-result.json'])
    put(records,'window-terminal-closeout.json',close)
    review=json.loads(records['root-review.json'])
    for field,name in [('window_result_sha256','window-result.json'),('window_terminal_closeout_sha256','window-terminal-closeout.json'),('actual_workflow_sha256','window-output/workflow/result.json')]:review[field]=verify.sha(records[name])
    put(records,'root-review.json',review)


class EssentialControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.records,cls.manifest=verify.load(Path(__file__).parent)

    def test_actual_data_replays(self):self.assertTrue(verify.validate(self.records,self.manifest)['passed'])

    def test_changed_clock_and_unearned_scope_refused(self):
        for scope in (False,True):
            r=copy.deepcopy(self.records);key='window-output/workflow/native-window.json';p=json.loads(r[key])
            if scope:p['appearance_review']=True
            else:p['animation_step']['after']['playback']['seconds']=.5
            put(r,key,p);cohere(r)
            with self.assertRaises(ValueError):verify.validate(r,self.manifest)

    def test_lost_topology_retention_or_journal_reopen_refused(self):
        r=copy.deepcopy(self.records);key='window-output/workflow/native-window.json';p=json.loads(r[key]);p['stages'][4]['accepted_fingerprint']=p['stages'][0]['accepted_fingerprint'];put(r,key,p);cohere(r)
        with self.assertRaisesRegex(ValueError,'retention'):verify.validate(r,self.manifest)
        r=copy.deepcopy(self.records);key='window-output/workflow/processes.json';rows=json.loads(r[key]);last=json.loads(rows[-1]['stdout']);last['result']['revision']=7;rows[-1]['stdout']=json.dumps(last)+'\n';put(r,key,rows);cohere(r)
        with self.assertRaisesRegex(ValueError,'journal revision'):verify.validate(r,self.manifest)

    def test_empty_or_omitted_terminal_identity_refused(self):
        for empty in (False,True):
            r=copy.deepcopy(self.records);key='window-terminal-closeout.json';c=json.loads(r[key]);c['absence']=[] if empty else c['absence'][1:];put(r,key,c);cohere(r)
            with self.assertRaisesRegex(ValueError,'terminal absence'):verify.validate(r,self.manifest)

    def test_changed_artifact_or_missing_stream_refused(self):
        m=copy.deepcopy(self.manifest);m['external_artifacts']['incant_editor']['sha256']='0'*64
        with self.assertRaisesRegex(ValueError,'artifact identities'):verify.validate(self.records,m)
        r=copy.deepcopy(self.records);del r['cpu-output/005.stdout.log']
        with self.assertRaisesRegex(ValueError,'retained data'):verify.validate(r,self.manifest)


if __name__=='__main__':unittest.main()
