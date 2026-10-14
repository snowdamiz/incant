#!/usr/bin/env python3
"""Verify portable payloads and, optionally, every retained local external file."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[4]; HERE=Path(__file__).resolve().parent

def digest(data): return hashlib.sha256(data).hexdigest()
def file_digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        while block:=f.read(1024*1024): h.update(block)
    return h.hexdigest()

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--external',action='store_true');a=parser.parse_args()
    m=json.loads((HERE/'manifest.json').read_text());archive=(HERE/'artifacts.json.gz').read_bytes()
    assert digest(archive)==m['archive_sha256'];stored=json.loads(gzip.decompress(archive))['payloads'];payloads={k:base64.b64decode(v,validate=True) for k,v in stored.items()}
    assert all(digest(v)==k for k,v in payloads.items())
    for r in m['records']:
        if r['embedded']:
            b=payloads[r['sha256']];assert len(b)==r['bytes']
        if a.external:
            p=Path(m['raw_root'])/r['path'];assert p.stat().st_size==r['bytes'];assert file_digest(p)==r['sha256'],p
    for r in m['sources']:
        b=subprocess.check_output(['git','show',m['source_commit']+':'+r['path']],cwd=ROOT);assert digest(b)==r['sha256'],r['path']
    rows={r['path']:r for r in m['records']}
    def data(p):return json.loads(payloads[rows[p]['sha256']])
    native=data('native-001/receipt.json');assert native['passed'] and native['observation_checks']==28 and native['scripted_entities']==10000
    assert native['original_entity_states_equal']==46 and native['pcm_equal'] and native['logs_equal']
    for p in ('native-001/reference-command.json','native-001/stress-command.json'):
        r=data(p);assert r['exit_code']==0 and r['owned_process_reaped'] and r['failure'] is None
    for p in ('authored-det-001/rpc-author.json','rpc-reopen-002.json','fast-stress-002/rpc-mode.json','fast-stress-002/rpc-reopen.json'):
        r=data(p);assert r['exit_code']==0 and r['owned_process_reaped'] and r['failure'] is None
    assert data('authored-det-001/rpc-reopen.json')['exit_code']==-9
    assert data('author-fast-001.json')['exit_code']==-2
    fast=data('fast-stress-002/receipt.json');assert fast['scripted_entities']==10000 and fast['undo_redo_reopen'] and not fast['runtime_execution']
    project=data('fast-stress-002/authoring/core.json');entities=next(iter(project['scenes'].values()))['entities']
    assert len(entities)==10045 and sum('Script' in e['components'] for e in entities.values())==10000
    assert project['settings']['physics_mode']=='fast'
    deterministic=data('authored-det-001/authoring/core.json')
    del project['settings']['physics_mode']
    assert project==deterministic
    reference=data('native-001/reference.stdout');actual=data('native-001/stress.stdout')
    assert not actual['gpu_submitted'] and actual['tick']==65 and actual['script_source_parsing']
    assert reference['logs']==actual['logs'] and reference['pcm_sha256']==actual['pcm_sha256']
    assert payloads[rows['native-001/reference.pcm']['sha256']]==payloads[rows['native-001/stress.pcm']['sha256']]
    for key,value in reference['observations']['entities'].items():
        assert actual['observations']['entities'][key]==value
    checks=actual['observations']['checks'];assert len(checks)==28 and all(r['passed'] for r in checks)
    def residue(n,m):
        q,r=divmod(n,m);return q*m*(m-1)//2+r*(r-1)//2
    for row in checks:
        if row['expect']['type']=='entity_count': assert row['actual']==10045;continue
        n=int(row['name'].split('-')[2]);phase=(row['tick']%64-32)/64
        expected=([9999,residue(9999,97)/9999-48+phase,residue(9999,13)/9999-6-phase] if n==0 else
                  [n%97-48+phase,n//97%97-48,n%13-6-phase])
        assert all(abs(a-b)<=1e-9 for a,b in zip(row['actual'],expected))
    print(json.dumps({'passed':True,'records':len(m['records']),'payloads':len(payloads),'sources':len(m['sources']),'external_checked':a.external}))

if __name__=='__main__':main()
