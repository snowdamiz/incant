"""Read-only current-main relevance and retained observable checks; no build/GPU."""
from pathlib import Path
import gzip, hashlib, json, re, subprocess, tarfile
R=Path(__file__).resolve().parents[4];D=Path(__file__).resolve().parent
M=json.loads((D.parent/'physics-collider-main216.json').read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
def file(row):
 b=(R/row['path']).read_bytes();assert sha(b)==row['sha256'],row['path'];return b
for ref in [M['main'],M['producer'],M['integration_source']]:subprocess.run(['git','merge-base','--is-ancestor',ref,'HEAD'],cwd=R,check=True)
raw={}
for row in M['records']:
 b=gzip.decompress(file(row));assert len(b)==row['raw_bytes'] and sha(b)==row['raw_sha256'];assert Path(row['raw_path']).read_bytes()==b;raw[Path(row['raw_path']).name]=b
identity=json.loads(raw['source-identity.json']);names=git('ls-tree','-r','--name-only',M['producer'],'--',*identity['roots']).decode().splitlines();assert names==[x['path'] for x in identity['files']]
assert not git('diff','--name-only',M['producer'],'HEAD','--',*identity['roots']).strip()
for row in identity['files']:file(row)
assert len(identity['files'])==M['source_files']
delta=json.loads(raw['unrelated-main-delta.json']);scope=['crates','third_party','Cargo.toml','Cargo.lock','.cargo','rust-toolchain.toml','tools'];changed=git('diff','--name-only',M['producer'],'HEAD','--',*scope).decode().splitlines();assert changed==[x['path'] for x in delta]
for row in delta:
 p=row['path'];assert p.startswith(('crates/incant_render/','crates/incant_render_data/','crates/incant_render_game/','crates/incant_text/','crates/incant_ui/'))
 prior=git('ls-tree',M['producer'],'--',p).strip()
 assert (sha(git('show',M['producer']+':'+p)) if prior else None)==row['producer_sha256'];assert sha(git('show',M['main']+':'+p))==sha((R/p).read_bytes())==row['main_sha256']
for row in M['producer_reviews']:file(row)
P=json.loads(file(M['producer_manifest']));E=Path(P['evidence_root'])
with tarfile.open(R/P['archive']['path'],'r:gz') as archive:
 assert sha((R/P['archive']['path']).read_bytes())==P['archive']['sha256']
 for row in P['records']:
  b=archive.extractfile(row['payload']).read();assert len(b)==row['bytes'] and sha(b)==row['sha256'];assert (E/row['path']).read_bytes()==b
for group in ['sources','baselines','native_pins']:
 for row in P[group]:
  frozen=Path(row['copy']).read_bytes();assert len(frozen)==row['bytes'] and sha(frozen)==row['sha256']
  if row['path'] not in ['STATUS.md','docs/spikes/physics-collider-state.md']:
   assert (R/row['path']).read_bytes()==frozen,row['path']
  else:
   assert row['path'] in ['STATUS.md','docs/spikes/physics-collider-state.md']
for row in P['binaries']:
 p=Path(row['path']);assert p.stat().st_size==row['bytes'] and p.stat().st_mode&0o100;assert sha(p.read_bytes())==row['sha256']
for c in P['checks']['checks']:assert c['exit']==0 and c['reaped'] and sha(Path(c['log']).read_bytes())==c['sha256']
assert len(P['checks']['checks'])==7
checks=json.loads(raw['checks.json']);assert len(checks)==8
for c in checks:assert c['exit']==0 and c['reaped'] and sha(raw[Path(c['path']).name])==c['sha256']
a=raw['current-selected-graph.log'].decode().replace(checks[0]['cwd'],'<ROOT>');b=raw['producer-selected-graph.log'].decode().replace(checks[1]['cwd'],'<ROOT>');assert a==b==raw['normalized-selected-graph.txt'].decode()
def counts(text):
 rows=[tuple(map(int,x))for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)]
 return {'passed':sum(x[0]for x in rows),'failed':sum(x[1]for x in rows),'ignored':sum(x[2]for x in rows),'groups':len(rows)}
assert counts((E/'final-tests-001.log').read_text())==P['counts']=={'passed':135,'failed':0,'ignored':0,'groups':20}
s='\n'.join(raw['replay-'+Path(x['path']).name+'.log'].decode() for x in P['binaries']);assert counts(s)=={'passed':34,'failed':0,'ignored':0,'groups':2}
for d in [2,3]:
 for label in ['mass_modes=3 groups=and+or flags=all user_data=u128 joined=true','enabled=all3 standalone=true','enabled_with_disabled_parent=true detached_disabled_by_parent=true','group_intersections=true+false+true']:assert f'COLLIDER_NATIVE dimension={d} '+label in s
 assert f'trusted_continuation dimension={d} checkpoints=3 steps=300' in s
 assert ('planar'if d==2 else 'spatial')+'::actual_sleep_stamp_wrap_requires_retained_scratch_not_zero_reconstruction ... ok' in s
red=(E/'enabled-admission-red.log').read_text();assert red.count('an `Err` value: Ownership')==2 and '0 passed; 2 failed; 0 ignored' in red
assert not P['gpu'] and not P['live_restore']
print(json.dumps({'passed':True,'source_files':len(names),'unrelated_main_files':len(delta),'integration_records':len(M['records']),'producer_records':len(P['records']),'checks':8,'replayed_cases':34,'retained_producer_cases':135,'new_compile':False,'live_save_admission':False},indent=2))
