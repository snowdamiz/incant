#!/usr/bin/env python3
"""Verify the one actual Windows cook observation, without clearing checkpoint32."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
P=argparse.ArgumentParser();P.add_argument('--originals',action='store_true');a=P.parse_args()
D=Path(__file__).resolve().parent;R=D.parents[3];h=lambda b:hashlib.sha256(b).hexdigest()
m=json.loads((D/'manifest.json').read_bytes());assert h(Path(__file__).read_bytes())==m['verifier_sha256']
b=(D/'records.json.gz').read_bytes();assert h(b)==m['archive_sha256'];x=json.loads(gzip.decompress(b));assert len(x['records'])==m['records']
for digest,b64 in x['payloads'].items():assert h(base64.b64decode(b64,validate=True))==digest
q={r['path']:r for r in x['records']};assert len(q)==len(x['records'])
def raw(name):
 r=q[name];b=base64.b64decode(x['payloads'][r['sha256']],validate=True);assert len(b)==r['bytes'];return b
for name in q:
 if a.originals:assert (Path(m['evidence_root'])/name).read_bytes()==raw(name),name
read=lambda n:raw(n).decode('utf-8-sig');obj=lambda n:json.loads(raw(n))
run=obj('diagnostic-run-final.json');job=obj('diagnostic-job-final.json')
assert run['id']==38516305407 and job['id']==115695776420
assert run['head_sha']==m['source'] and run['event']=='workflow_dispatch' and run['run_attempt']==1
assert run['conclusion']==job['conclusion']=='success' and run['status']==job['status']=='completed'
assert job['run_id']==run['id'] and read('windows-artifact/source.txt').strip()==m['source']
assert read('windows-artifact/test-exit.txt').strip()=='0'
log=read('windows-artifact/run.log');assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log)==[('1','0','0')]
rows=[json.loads(v) for v in re.findall(r'INCANT_NATIVE_COOK_PHASES (\{[^\r\n]+\})',log)]
assert rows==json.loads(read('windows-artifact/phases.json')) and len(rows)==2
names=['driver_entry','compiler_ready','input_ready','snapshot_ready','typecheck_done','transform_done','artifact_ready','publish_done']
for r,n,outcome in zip(rows,[8,3],['exited_success','refused']):
 assert r['version']==1 and r['state']=='valid_prefix' and r['outcome']==outcome
 assert [p['phase']for p in r['phases']]==names[:n]
 times=[p['elapsed_us']for p in r['phases']];assert times==sorted(times) and all(type(t)is int and 0<=t<=86400000000 for t in times)
 assert 0<r['parent_elapsed_us']<8000000
assert rows[0]['parent_elapsed_us']==2442570 and rows[1]['parent_elapsed_us']==783991
# The rejected raw-SHA addressing request created no run; only one branch
# dispatch was accepted, and its actual source was independently checked.
assert obj('rejected-sha-no-runs.json')['runs']==[]
assert len(obj('diagnostic-run-list.json'))==1
assert obj('diagnostic-run-list.json')[0]['databaseId']==run['id']
workflow=subprocess.check_output(['git','show',m['source']+':.github/workflows/typescript-cook-diagnostic.yml'],cwd=R)
assert h(workflow)==m['workflow_sha256']
assert b'timeout-minutes: 35' in workflow and b'--test fallback -- authored_attachments_never_invoke_or_silently_fall_back_to_the_host_function --exact --nocapture --test-threads=1' in workflow
cook=subprocess.check_output(['git','show',m['source']+':crates/incant_cook/src/typescript.rs'],cwd=R)
assert b'const DEADLINE: Duration = Duration::from_secs(8);' in cook
print(json.dumps({'verified':True,'run':run['id'],'head':m['source'],'test_cases':1,'observations':2,'successful_cook_us':2442570,'expected_refusal_us':783991,'checkpoint32':'retained failure, not cleared'}))
