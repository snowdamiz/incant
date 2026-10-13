"""Read-only source/check/archive verification; never rewrites prior receipts."""
from pathlib import Path
import argparse,base64,gzip,hashlib,json,re
P=argparse.ArgumentParser();P.add_argument('--verify-only',action='store_true',required=True);P.parse_args()
R=Path(__file__).resolve().parents[4]
M=json.loads((R/'docs/spikes/evidence/timestamp-diagnostic-capabilities.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
A=M['archive'];raw=(R/A['path']).read_bytes();assert h(raw)==A['sha256'];a=json.loads(gzip.decompress(raw));assert len(a['records'])==A['records'] and len(a['payloads'])==A['payloads']
blobs={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
for k,v in blobs.items():assert h(v)==k
by_path={}
for row in a['records']:
 b=blobs[row['sha256']];assert len(b)==row['bytes'];p=Path(row['path']);by_path[str(p)]=b
 if p.exists():assert p.read_bytes()==b,str(p)
F=M['freeze'];raw=(R/F['path']).read_bytes();assert h(raw)==F['sha256'];f=json.loads(raw);assert len(f['files'])==F['count']
for row in f['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']
assert h((R/'crates/incant_render/tests/timestamp_backend_probe.rs').read_bytes())==M['source']['sha256']
assert h((Path(__file__).parent/'repair.patch').read_bytes())==M['source']['patch_sha256']
assert len(M['checks'])==7
for c in M['checks']:
 assert c['exit']==0 and c['reaped']
 for k in ['stdout','stderr']:assert h(by_path[c[k]['path']])==c[k]['sha256']
counts=[]
for line in by_path[next(c for c in M['checks'] if c['name']=='tests')['stdout']['path']].decode().splitlines():
 m=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',line)
 if m:counts.append(tuple(map(int,m.groups())))
assert counts==[(2,0,1)],counts
for b in M['binaries']:
 p=Path(b['path']);assert h(p.read_bytes())==b['sha256'] and p.stat().st_size==b['bytes']
log=next(v for p,v in by_path.items() if p.endswith('checkpoint22/failures/checks.log')).decode()
assert 'assertion failed: adapter.features().contains(features)' in log
assert 'Timestamp contract: typed Unsupported; ordinary frame completed' in log
assert 'Apple Paravirtual device' in log and 'supported=false' in log
print(json.dumps(dict(verified=True,records=A['records'],payloads=A['payloads'],sources=F['count'],commands=len(M['checks']),cpu_cases=2,gpu_execution=False)))
