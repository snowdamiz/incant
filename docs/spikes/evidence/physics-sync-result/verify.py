#!/usr/bin/env python3
"""Read-only replay of the three-test repair; never builds or runs a player."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
P=Path(__file__).resolve().parent;R=P.parents[3];H=lambda b:hashlib.sha256(b).hexdigest()
p=argparse.ArgumentParser();p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
m=json.loads((P/'manifest.json').read_text());z=(P/'artifacts.json.gz').read_bytes();assert H(z)==m['archive_sha256'];raw={h:base64.b64decode(b,validate=True) for h,b in json.loads(gzip.decompress(z))['payloads'].items()};assert all(H(b)==h for h,b in raw.items())
rows={r['path']:r for r in m['records']};assert len(rows)==len(m['records'])
for r in rows.values():
 assert len(raw[r['sha256']])==r['bytes']
 if a.originals:assert Path(r['original']).read_bytes()==raw[r['sha256']]
def data(n):return json.loads(raw[rows[n]['sha256']])
f=data('freeze.json');assert f['head']==m['source']
for name,sha in f['files'].items():assert H((R/name).read_bytes())==sha,name
results=data('results.json');assert len(results)==m['checks']==5
for c in results:
 assert c==data(c['name']+'.json') and c['exit']==0 and c['error'] is None
 assert c['freeze_sha256']==rows['freeze.json']['sha256']
 for suffix,sha in c['logs'].items():assert sha==rows[c['name']+'.'+suffix]['sha256']
log=raw[rows['tests.stdout']['sha256']].decode();counts=[int(x) for x in re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;',log)];assert counts==m['affected_test_counts'] and len(counts)==3
assert data('released.json')['released']
job=data('job.json');assert job['head_sha']==m['candidate'] and job['id']==m['failed_job'] and job['conclusion']=='failure'
failed=raw[rows['job.log']['sha256']].decode();assert 'unused `std::result::Result` that must be used' in failed
for file in ('compound.rs','character.rs','runtime.rs'):assert 'crates/incant_physics/tests/'+file in failed
if a.local_binaries:
 for b in m['binaries']:
  q=Path(rows['freeze.json']['original']).parent/b['path'];assert q.is_file() and q.stat().st_size==b['bytes'] and H(q.read_bytes())==b['sha256']
print(json.dumps({'passed':True,'records':len(rows),'inputs':len(f['files']),'checks':len(results),'cases':sum(counts),'groups':counts,'binaries':len(m['binaries']),'originals_checked':a.originals,'local_binaries_checked':a.local_binaries}))
