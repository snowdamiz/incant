from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime,tomllib
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/physics-pair-main204-001';d=r/'docs/spikes/evidence/physics-pair-main205'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='77910a658f7badd679d4cddbd8f89cc6dab5aeb6';main=read(o.parent/'pr205/merge.json')['main'];f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['head']==f['head'] and t['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
ancestry=read(o/'ancestry.json');assert ancestry['tested_head']==f['head'] and ancestry['main']==main;assert git('rev-parse',f['head']+'^{tree}')==git('rev-parse',ancestry['head']+'^{tree}')
checks=read(o/'results.json');assert len(checks)==8 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
text=Path(checks[0]['stdout']['path']).read_text()+Path(checks[0]['stderr']['path']).read_text();rows=[list(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)]
counts={'passed':sum(x[0] for x in rows),'failed':sum(x[1] for x in rows),'ignored':sum(x[2] for x in rows),'nonempty_groups':sum(x[0]>0 for x in rows),'result_groups':len(rows)};assert counts=={'passed':88,'failed':0,'ignored':0,'nonempty_groups':14,'result_groups':16}
p=read(r/'docs/spikes/evidence/physics-inert-pair-state.json');assert h(r/'docs/spikes/evidence/physics-inert-pair-state.json')=='193ca3fbe9a7e9f1bfd9f6ec7b32d239f3eb847eda87eac9cbf3df9a12b4f13d';sources=[]
for row in p['sources']:
 assert sha(git('show',producer+':'+row['path']))==row['sha256'];now=h(r/row['path'])
 if row['path']!='Cargo.lock':assert now==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':now})
for row in p['records']:
 raw=Path(row['original']).read_bytes();z=(r/row['retained']).read_bytes();assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(z)==row['retained_sha256'] and gzip.decompress(z)==raw
for row in p['pinned']:assert h(row['source'])==h(row['copy'])==row['sha256']
review=Path.home()/'.codex/incant-reviews/physics-inert-pair-state-final-20261013.json';assert h(review)=='9ae2d7c298e6f97e77804b0b6888a96a2f0d8f0782876405d0c51b89a844e4e0'
actual=git('diff','--name-only',main,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml').decode().splitlines();expected=git('diff','--name-only',producer+'^',producer,'--','crates','Cargo.lock').decode().splitlines();assert sorted(actual)==sorted(expected)
key=lambda x:(x['name'],x['version'],x.get('source',''))
old={key(x):x for x in tomllib.loads(git('show',main+':Cargo.lock').decode())['package']};new={key(x):x for x in tomllib.loads((r/'Cargo.lock').read_text())['package']};assert old.keys()==new.keys();lock_delta=[]
for k in old:
 if old[k]!=new[k]:
  assert k[0]=='incant_physics_snapshot';a=dict(old[k]);b=dict(new[k]);ad=a.pop('dependencies');bd=b.pop('dependencies');assert a==b and sorted(bd)==sorted(ad+['serde']);lock_delta.append('incant_physics_snapshot dev-only serde edge')
assert len(lock_delta)==1
normal=Path(next(x for x in checks if x['name']=='normal-graph-001')['stdout']['path']).read_text();assert len(normal.splitlines())==1 and normal.startswith('incant_physics_snapshot ')
summary={'passed':True,'checks':8,'counts':counts,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_crate_inputs':len(sources)-1,'producer_records_rehashed':len(p['records']),'pinned_sources':len(p['pinned']),'changed_source_paths':expected,'lock_delta':lock_delta,'package_identities_unchanged':len(new),'main_ancestor':main,'live_restore_or_save_admission':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'physics-pair-main204-run.py','run.py'),(review,'producer-final-review.json')]:shutil.copyfile(src,d/name)
m={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'ancestry_head':ancestry['head'],'main_ancestor':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'producer_final_review':{'path':str((d/'producer-final-review.json').relative_to(r)),'sha256':h(review)},'limits':['88 cases include the prior79 manifold cases; counts are not additive.','All39 crate source/test/manifest files equal accepted77910. Older producer lock predates merged profiling packages/edges and renderer dev naga; current main lock adds only an already pinned dev-only serde edge.','No live constructor, complete owner authentication, reciprocal solver/island ownership, previous-cluster scratch reconstruction, physics-save admission, GPU, shipping, device or performance claim.'],'review_state':'Current-main final pair-state integration review pending'}
path=r/'docs/spikes/evidence/physics-pair-main205.json';path.write_text(json.dumps(m,indent=2)+'\n');print(path,h(path),flush=True)
