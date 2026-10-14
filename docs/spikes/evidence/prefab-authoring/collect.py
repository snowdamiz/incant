#!/usr/bin/env python3
"""Lossless prefab check retention; optional external copied-binary verification."""
import argparse, base64, gzip, hashlib, json, pathlib, subprocess
P=argparse.ArgumentParser();P.add_argument('--root',type=pathlib.Path,required=True);P.add_argument('--verify-only',action='store_true');P.add_argument('--originals',action='store_true');P.add_argument('--local-binaries',action='store_true');a=P.parse_args()
R=a.root.resolve();E=pathlib.Path('/Users/andreyyurlov/.codex/incant-evidence/prefab-authoring-001');D=R/'docs/spikes/evidence/prefab-authoring';M=D/'manifest.json';A=D/'records.json.gz'
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(x):return (json.dumps(x,indent=2,sort_keys=True)+'\n').encode()
if not a.verify_only:
 D.mkdir(parents=True,exist_ok=True);records=[];payloads={};checks=[];binaries=[]
 for p in sorted(E.rglob('*')):
  if not p.is_file() or 'binaries' in p.parts or p.name.startswith('.') :continue
  b=p.read_bytes();h=sha(b);records.append({'path':str(p.relative_to(E)),'sha256':h,'bytes':len(b)});payloads[h]=base64.b64encode(b).decode()
  if p.name=='receipt.json':
   x=json.loads(b);checks.append({'name':p.parent.name,'exit':x.get('exit'),'reaped':x.get('reaped'),'source_unchanged':x.get('source_unchanged'),'receipt_sha256':h})
   binaries.extend(x.get('binaries',[]))
 inputs=[]
 for s in subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],cwd=R,text=True).splitlines():
  p=R/s
  if p.is_file() and (s.endswith(('.rs','.toml','.lock','.wgsl')) or s=='tools/cargo' or s.startswith('schemas/') or (s.startswith('sdk/ts/') and s.endswith('.ts'))):
   inputs.append({'path':s,'sha256':sha(p.read_bytes())})
 body={'records':records,'payloads':payloads};A.write_bytes(gzip.compress(dump(body),mtime=0))
 manifest={'version':1,'collector_sha256':sha(pathlib.Path(__file__).read_bytes()),'base':'fdd480355ba70b28bbebe35f58b8af188f0abb82','archive':A.name,'archive_sha256':sha(A.read_bytes()),'records':len(records),'payloads':len(payloads),'checks':checks,'binaries':binaries,'final_inputs':inputs,'scope':'Fixed-topology linked prefab authoring; default CPU and native cooked consumer. No GUI, GPU, performance or release gate claim.'};M.write_bytes(dump(manifest))
m=json.loads(M.read_bytes());assert sha(pathlib.Path(__file__).read_bytes())==m['collector_sha256'];assert sha(A.read_bytes())==m['archive_sha256'];body=json.loads(gzip.decompress(A.read_bytes()));assert len(body['records'])==m['records'] and len(body['payloads'])==m['payloads']
for h,b in body['payloads'].items():assert sha(base64.b64decode(b))==h
for r in body['records']:
 b=base64.b64decode(body['payloads'][r['sha256']]);assert len(b)==r['bytes']
 if a.originals:assert (E/r['path']).read_bytes()==b
for r in m['final_inputs']:assert sha((R/r['path']).read_bytes())==r['sha256'],r['path']
for c in m['checks']:
 assert c['reaped'] is True and c['source_unchanged'] is True,c
 raw=base64.b64decode(body['payloads'][c['receipt_sha256']]);x=json.loads(raw);assert x['exit']==c['exit']
 if c['exit']!=0:assert c['name'] in {'focused-001','deletion-negative-001','lint-001'},c
for r in m['binaries']:
 if a.local_binaries:
  p=pathlib.Path(r['path']);assert sha(p.read_bytes())==r['sha256'] and p.stat().st_mode&0o111,r['path']

# Every historical copied source still matches its original freeze, independently
# of subsequent live edits. Baseline rows are preserved for source-relevance review.
record_map={r['path']:r for r in body['records']}
for r in body['records']:
 if r['path'].endswith('/freeze.json'):
  f=json.loads(base64.b64decode(body['payloads'][r['sha256']]))
  for row in f.get('source',[]):
   key=r['path'].removesuffix('freeze.json')+'source/'+row['path']
   assert record_map[key]['sha256']==row['sha256'],key

expected_failures={'focused-001':101,'deletion-negative-001':101,'lint-001':101}
assert {c['name']:c['exit'] for c in m['checks'] if c['exit']}==expected_failures
for name in ['lean-001/checks.json','lean-002/checks.json']:
 x=json.loads(base64.b64decode(body['payloads'][record_map[name]['sha256']]))
 assert len(x)==5 and all(c['exit']==0 and c['reaped'] for c in x)
full=base64.b64decode(body['payloads'][record_map['doc-cmd-all-001/output.log']['sha256']]).decode()
import re
summaries=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',full)
assert [sum(int(x[i]) for x in summaries) for i in range(3)]==[87,0,0]
print(json.dumps({'records':m['records'],'payloads':m['payloads'],'inputs':len(m['final_inputs']),'checks':len(m['checks']),'binaries':len(m['binaries']),'verified':True}))
