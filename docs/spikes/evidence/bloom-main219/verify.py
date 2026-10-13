"""Read-only bloom composition, prior appearance provenance and current GPU audit."""
from pathlib import Path
import argparse,base64,gzip,hashlib,json,re,subprocess
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
R=Path(__file__).resolve().parents[4];D=Path(__file__).parent;E=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/bloom-main219-001');M=json.loads((R/'docs/spikes/evidence/bloom-main219.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
A=M['archive'];b=(R/A['path']).read_bytes();assert h(b)==A['sha256'] and len(b)==A['bytes'];a=json.loads(gzip.decompress(b));assert len(a['records'])==A['records'] and len(a['payloads'])==A['payloads'];payloads={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
for k,v in payloads.items():assert h(v)==k
raw={}
for row in a['records']:
 b=payloads[row['sha256']];assert len(b)==row['bytes'];path=row['path'];raw[path]=b
 if path.startswith('git:'):
  _,rev,p=path.split(':',2);assert git('show',rev+':'+p)==b
 elif Path(path).exists():assert Path(path).read_bytes()==b,path
def data(p):return json.loads(raw[str(p)])
f=M['freeze'];b=(R/f['path']).read_bytes();assert h(b)==f['sha256'];F=json.loads(b);assert len(F['files'])==3109
for row in F['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']
source=data(E.parent/'bloom-main219-source-001/freeze.json');assert len(source['files'])==14
for row in source['files']:
 p=row['path'];actual=git('show',M['source_commit']+':'+p) if p.startswith('docs/') else (R/p).read_bytes();assert h(actual)==row['sha256']
 if row['exact_producer']:assert actual==git('show',M['producer']+':'+p)
assert not (R/'crates/incant_render/src/ambient_occlusion').exists()
for p in ['filter.wgsl','output.wgsl','config.rs','targets.rs']:
 p='crates/incant_render/src/bloom/'+p;assert (R/p).read_bytes()==git('show',M['producer']+':'+p)
for row in M['reviews'].values():assert h((R/row['path']).read_bytes())==row['sha256']
assert len(M['checks'])==12
for c in M['checks']:
 assert c['exit']==0 and c['reaped']
 for key in ['stdout','stderr']:assert h(raw[c[key]['path']])==c[key]['sha256']
for name,count,cooked in [('default-tests',137,False),('cooked-tests',166,True)]:
 c=next(x for x in M['checks'] if x['name']==name);text=raw[c['stdout']['path']].decode();counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text);assert len(counts)==10 and tuple(sum(int(r[i]) for r in counts) for i in range(3))==(count,0,103)
 found=[]
 for line in text.splitlines():
  if line.startswith('{'):
   try:v=json.loads(line)
   except json.JSONDecodeError:continue
   if v.get('reason')=='compiler-artifact' and v['target']['name']=='incant_render' and v.get('executable'):
    assert ('cooked-output' in v['features'])==cooked;found.append(v)
 assert len(found)==1
for row in M['external_binaries']:
 p=Path(row['path']);assert h(p.read_bytes())==row['sha256'] and p.stat().st_size==row['bytes']
release=M['release'];assert release['released'] and release['success'] and release['all_children_reaped'] and release['frame_debit']==8
assert len(M['gpu'])==4;frames=0
for c in M['gpu']:
 assert c['exit']==0 and c['reaped'] and c['error'] is None
 for k in ['stdout','stderr']:assert h(raw[c[k]['path']])==c[k]['sha256']
 out=raw[c['stdout']['path']].decode();err=raw[c['stderr']['path']].decode()
 if c['label'].endswith('.list'):
  assert '1 test, 0 benchmarks' in out;continue
 assert '1 passed; 0 failed; 0 ignored;' in out and 'backend: Metal' in err and 'timestamps=true' in err
 samples=[json.loads(s.split('FIXED_JOINED_TIMING ',1)[1]) for s in err.splitlines() if s.startswith('FIXED_JOINED_TIMING ')]
 assert [s['sequence'] for s in samples]==[1,2,3,4]
 for s in samples:
  assert s['frame']['nanoseconds']>0 and s['timestamp_period_ns']>0
  for i in [0,3,6]:assert s['passes'][i]['nanoseconds']>0
  for span in s['passes']:
   if span:assert s['frame']['start_ticks']<=span['start_ticks']<=span['end_ticks']<=s['frame']['end_ticks']
 frames+=len(samples)
assert frames==8
attempt=data(E/'gpu/attempt.json');assert attempt['runner_sha256']=='52fb97f441ae2ee85175bf9f8bfb2d31875e3068e51d7b54966dcecd25de11c9' and attempt['max_frames']==8
final=json.loads((D/'0075-bloom004-final-root-20261013.json').read_text());assert final['claude_visual_verdict']=='Bloom pass; SSAO still rejected';assert h((D/'producer-appearance-review.md').read_bytes())==final['report_sha256']
root=Path('/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004')
for row in final['verified_retained_records']:
 b=raw[str(root/row['name'])];assert h(b)==row['sha256'] and len(b)==row['bytes']
pre=json.loads((D/'0075-bloom004-execution-preaudit-root-20261013.json').read_text())
for path,sha in pre['verified_records'].items():
 if path in raw:assert h(raw[path])==sha
 else:assert any(x['path']==path and x['sha256']==sha for x in M['external_binaries'])
paths=git('diff','--name-only','931998df4d9ceb3d076e242938cddc653c75aecc',M['base']).decode().splitlines();assert len(paths)==4 and all(p=='STATUS.md' or p.startswith('docs/') for p in paths)
print(json.dumps(dict(verified=True,records=A['records'],payloads=A['payloads'],frozen=3109,checks=12,default=137,cooked=166,current_measured_frames=frames,appearance='prior Claude bloom only; no current pixel review')))
