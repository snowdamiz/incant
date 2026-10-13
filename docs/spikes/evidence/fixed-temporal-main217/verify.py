"""Read-only fixed-only composition, CPU, recook and bounded GPU receipt audit."""
from pathlib import Path
import argparse,base64,gzip,hashlib,json,re,subprocess
P=argparse.ArgumentParser();P.add_argument('--verify-only',required=True,action='store_true');P.parse_args()
R=Path(__file__).resolve().parents[4];D=Path(__file__).parent;B=Path('/Users/andreyyurlov/.codex/incant-integration-evidence')
M=json.loads((R/'docs/spikes/evidence/fixed-temporal-main217.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
A=M['archive'];raw=(R/A['path']).read_bytes();assert h(raw)==A['sha256'] and len(raw)==A['bytes'];a=json.loads(gzip.decompress(raw));assert len(a['records'])==A['records'] and len(a['payloads'])==A['payloads'];blobs={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
for k,b in blobs.items():assert h(b)==k
by={}
for row in a['records']:
 b=blobs[row['sha256']];assert len(b)==row['bytes'];p=Path(row['path']);by[str(p)]=b
 if p.exists():assert p.read_bytes()==b,str(p)
def data(path):return json.loads(by[str(path)])
def log_counts(name,checks):
 row=next(c for c in checks if c['name']==name);text=by[row['stdout']['path']].decode();rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text);assert rows
 return tuple(sum(int(row[i]) for row in rows) for i in range(3))
for key in ['freeze','initial_freeze']:
 f=M[key];raw=(R/f['path']).read_bytes();assert h(raw)==f['sha256'];rows=json.loads(raw)['files'];assert len(rows)==f['count']
 if key=='freeze':
  for row in rows:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']
 else:
  for row in rows:assert h(git('show','725ee636c3ed1f6958f0c00c0ff15ae573cc99fc:'+row['path']))==row['sha256'],row['path']
assert M['test_only_delta']==['crates/incant_render/src/temporal_joined_tests.rs','crates/incant_render/src/temporal_retained_tests.rs']
assert len(M['checks'])==16 and len(M['followup_checks'])==6
for c in M['checks']+M['followup_checks']:
 assert c['exit']==0 and c['reaped']
 for k in ['stdout','stderr']:assert h(by[c[k]['path']])==c[k]['sha256']
assert log_counts('shader-cooker',M['checks'])==(30,0,0)
assert log_counts('default-tests',M['checks'])==(124,0,98)
assert log_counts('cooked-tests',M['checks'])==(153,0,98)
assert log_counts('default-tests',M['followup_checks'])==(115,0,41)
assert log_counts('cooked-tests',M['followup_checks'])==(144,0,41)
O=M['offline'];assert O['complete'] and not O['gpu_execution'] and len(O['commands'])==30
OD=B/'fixed-temporal-main217-001/offline'
for c in O['commands']:
 assert c['exit_code']==0
 for k in ['stdout','stderr']:assert h(by[str(OD/(c['name']+'.'+k))])==c[k+'_sha256']
for row in O['artifacts']:
 b=by[str(OD/row['path'])];assert h(b)==row['sha256'] and len(b)==row['bytes']
programs={p.split('/')[0] for p in [r['path'] for r in O['artifacts']] if '/' in p}
assert programs=={'temporal-motion','temporal-depth','temporal-resolve','temporal-copy','temporal-cluster-shaders'},programs
for row in M['external_binaries']:
 p=Path(row['path']);assert h(p.read_bytes())==row['sha256'] and p.stat().st_size==row['bytes']
for row in M['reviews'].values():assert h((R/row['path']).read_bytes())==row['sha256']
for path in git('ls-tree','-r','--name-only',M['accepted_fixed_producer'],'crates/incant_render/src/temporal').decode().splitlines():assert (R/path).read_bytes()==git('show',M['accepted_fixed_producer']+':'+path),path
for path in ['crates/incant_render/src/motion/shader.wgsl','crates/incant_render/src/lighting/temporal_source.rs']:assert (R/path).read_bytes()==git('show',M['accepted_fixed_producer']+':'+path)
assert (R/'crates/incant_render/src/motion/owner.rs').read_bytes()==git('show',M['fixed_motion_owner']+':crates/incant_render/src/motion/owner.rs')
G=B/'fixed-temporal-main217-002/gpu';release=M['gpu_release'];assert release['released'] and release['success'] and release['all_children_reaped'] and release['frame_debit']==8
assert len(M['gpu'])==4;actual=[]
for c in M['gpu']:
 assert c['exit']==0 and c['reaped'] and c['error'] is None
 for k in ['stdout','stderr']:assert h(by[c[k]['path']])==c[k]['sha256']
 if c['label'].endswith('.list'):assert '1 test, 0 benchmarks' in by[c['stdout']['path']].decode();continue
 out=by[c['stdout']['path']].decode();err=by[c['stderr']['path']].decode();assert '1 passed; 0 failed; 0 ignored;' in out
 assert 'backend: Metal' in err and 'timestamps=true' in err
 samples=[json.loads(s.split('FIXED_JOINED_TIMING ',1)[1]) for s in err.splitlines() if s.startswith('FIXED_JOINED_TIMING ')]
 assert [s['sequence'] for s in samples]==[1,2,3,4]
 for s in samples:
  assert s['frame']['nanoseconds']>0 and s['timestamp_period_ns']>0
  for i in [0,3,6]:assert s['passes'][i]['nanoseconds']>0
  for p in s['passes']:
   if p:assert s['frame']['start_ticks']<=p['start_ticks']<=p['end_ticks']<=s['frame']['end_ticks']
 actual+=samples
assert len(actual)==8
attempt=data(G/'attempt.json');assert attempt['runner_sha256']=='96bc16293ac1ef52a26790bfd99630e9e877aad1bafa91f9ae2cb06026a850a6' and attempt['max_frames']==8
print(json.dumps(dict(verified=True,records=A['records'],payloads=A['payloads'],frozen_inputs=M['freeze']['count'],checks=22,offline_commands=30,measured_gpu_frames=len(actual),appearance='producer fixed-only; no current pixel review')))
