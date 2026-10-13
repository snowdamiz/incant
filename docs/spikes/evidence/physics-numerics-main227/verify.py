"""Read-only current-main numerical admission source and execution audit."""
from pathlib import Path
import argparse, base64, gzip, hashlib, json, re, subprocess, tomllib

arg=argparse.ArgumentParser()
arg.add_argument('--verify-only',action='store_true',required=True)
arg.parse_args()
R=Path(__file__).resolve().parents[4]
M=json.loads((R/'docs/spikes/evidence/physics-numerics-main227.json').read_text())
h=lambda b:hashlib.sha256(b).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=R)
def file(row):
 b=(R/row['path']).read_bytes();assert h(b)==row['sha256'],row['path'];return b
def archive(row):
 b=file(row);assert len(b)==row['bytes'];a=json.loads(gzip.decompress(b))
 assert len(a['records'])==row['records'] and len(a['payloads'])==row['payloads']
 payloads={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
 for k,b in payloads.items():assert h(b)==k
 out={}
 for x in a['records']:
  assert x['path'] not in out
  b=payloads[x['sha256']];assert len(b)==x['bytes'];assert Path(x['path']).read_bytes()==b,x['path'];out[x['path']]=b
 return out
raw=archive(M['archive'])
freeze=json.loads(file(M['freeze']))
for x in freeze['files']:
 assert h((R/x['path']).read_bytes())==x['sha256'],x['path']
 assert h(git('show',M['source_head']+':'+x['path']))==x['sha256']
assert len(freeze['files'])==M['freeze']['files']
identity=json.loads(file(M['source_identity']))
for x in identity['files']:
 b=(R/x['path']).read_bytes();old=git('show',M['producer']+':'+x['path'])
 assert h(b)==x['sha256'] and h(old)==x['producer_sha256']
 if b!=old:
  assert x['path']=='crates/incant_physics_snapshot/src/lib.rs' and x['role']=='reviewed-current-error-order'
  def normalized(b):
   s=b.decode();a=s.index('pub enum Error {');e=s.index('\n}',a);ls=s[a:e].splitlines()
   return s[:a]+'\n'.join([ls[0],*sorted(ls[1:])])+s[e:]
  assert normalized(b)==normalized(old)
assert len(identity['files'])==M['source_identity']['files']
assert git('merge-base','--is-ancestor',M['base'],'HEAD')==b''
lock=tomllib.loads((R/'Cargo.lock').read_text());old=tomllib.loads(git('show',M['base']+':Cargo.lock').decode())
ids=lambda d:sorted((p['name'],p['version'],p.get('source'),p.get('checksum')) for p in d['package'])
assert ids(lock)==ids(old) and len(lock['package'])==968
prior={(p['name'],p['version']):p for p in old['package']};changes=[]
for p in lock['package']:
 q=prior[(p['name'],p['version'])]
 if p!=q:
  assert p['name'] in ['rapier2d','rapier3d'];assert sorted(p['dependencies'])==sorted(q['dependencies']+['incant_physics_snapshot'])
  fixed=dict(p);fixed['dependencies']=q['dependencies'];assert fixed==q;changes.append(p['name'])
assert sorted(changes)==['rapier2d','rapier3d']
for dim in [2,3]:
 manifest=tomllib.loads((R/f'third_party/rapier{dim}d/Cargo.toml').read_text())
 assert 'incant-checked-restore' not in manifest['features']['default']
 edge=manifest['dependencies']['incant_physics_snapshot'];assert edge['optional'] and edge['default-features'] is False
snapshot=tomllib.loads((R/'crates/incant_physics_snapshot/Cargo.toml').read_text());assert not snapshot.get('dependencies')
assert len(M['checks'])==15
for x in M['checks']:
 assert x['exit']==0 and x['reaped']
 for k in ['stdout','stderr']:assert h(raw[x[k]['path']])==x[k]['sha256']
def check(name):return next(x for x in M['checks'] if x['name']==name)
def output(name,key='stdout'):return raw[check(name)[key]['path']].decode()
def messages(s):
 for line in s.splitlines():
  if line.startswith('{'):
   try:yield json.loads(line)
   except json.JSONDecodeError:pass
for label,det in [('default',True),('fast8',False)]:
 text=output(label+'-tests');counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 assert len(counts)==24 and tuple(sum(int(x[i]) for x in counts) for i in range(3))==(214,0,0)
 for name in ['rapier2d','rapier3d']:
  rows=[x for x in messages(text) if x.get('reason')=='compiler-artifact' and x['target']['name']==name];assert rows
  for x in rows:
   f=set(x['features']);assert {'incant-checked-restore','incant-checkpoint','serde-serialize'}<=f
   assert ('enhanced-determinism' in f)==det and ('simd8' in f)==(not det)
 for dim in [2,3]:assert f'NATIVE_ADMISSION dimension={dim} primitive_steps=72 exact_derived_mass=true saved_bits_unchanged=true' in output(label+'-tests','stderr')
lines=output('normal-graph').splitlines();assert len(lines)==1 and lines[0].startswith('incant_physics_snapshot v0.0.1')
assert 'Ran 31 tests' in output('python-guards','stderr') and 'OK' in output('python-guards','stderr')
assert '"locked_package_identities_unchanged": 968' in output('probe-graph-audit')
rt=next(b for path,b in raw.items() if path.endswith('/runtime-default-tree.stdout'))
assert b'incant_physics_snapshot' not in rt and b'incant-checked-restore' not in rt
producer=json.loads(file(M['producer_manifest']));pr=archive(producer['archive'])
assert len(pr)==270
for x in producer['source']['files']:
 b=git('show',M['producer']+':'+x['path']);assert h(b)==x['sha256'] and len(b)==x['bytes']
for x in producer['commands']:
 assert x['exit_code']==0 and h(pr[x['log']])==x['sha256']
for x in producer['failed_attempts']:assert x['exit_code']!=0 and h(pr[x['log']])==x['sha256']
assert len(producer['commands'])==16 and len(producer['failed_attempts'])==5
for x in producer['binaries']+M['binaries']:
 p=Path(x['path']);b=p.read_bytes();assert h(b)==x['sha256'] and len(b)==x['bytes'] and p.stat().st_mode&0o111
assert h(file(M['producer_review']))=='fd0cba95b8ce65950916c7d5ddb0a8b64c46f67f7685f4ae8a1b1a26d4d9e66d'
print(json.dumps(dict(verified=True,current_records=M['archive']['records'],producer_records=270,frozen_inputs=len(freeze['files']),physics_inputs=len(identity['files']),checks=15,default=214,fast8=214,primitive_steps_each_mode=144,package_identities=968,normal_graph='std-only',native_construction=False,live_save_admission=False)))
