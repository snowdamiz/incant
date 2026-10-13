from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/inert-manifolds-main201-001';d=r/'docs/spikes/evidence/physics-manifolds-main201'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='7ca4dd872c7d97c66829c60a360218071d187bb1';main='ebccdfda1f41a0409eb3a566f06bb174d23cfd34'
f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['head']==f['head'] and t['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
checks=read(o/'results.json');assert len(checks)==8 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
text=Path(checks[0]['stdout']['path']).read_text()+Path(checks[0]['stderr']['path']).read_text();m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
counts={'passed':sum(int(x[0]) for x in m),'failed':sum(int(x[1]) for x in m),'ignored':sum(int(x[2]) for x in m),'nonempty_groups':sum(int(x[0])>0 for x in m)};assert counts=={'passed':79,'failed':0,'ignored':0,'nonempty_groups':13}
p=read(r/'docs/spikes/evidence/physics-inert-manifolds.json');sources=[]
for row in p['sources']:
 assert sha(git('show',producer+':'+row['path']))==row['sha256'];now=h(r/row['path'])
 if row['path']!='Cargo.lock':assert now==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':now})
for row in p['records']:
 raw=Path(row['original']).read_bytes();z=(r/row['retained']).read_bytes();assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(z)==row['retained_sha256'] and gzip.decompress(z)==raw
for row in p['pinned']:assert h(row['source'])==h(row['copy'])==row['sha256']
review=Path.home()/'.codex/incant-reviews/physics-inert-manifolds-source-002-second-20261013.json';assert h(review)=='3bdf838cb5e6b36d2b8ec870dcc23d69a1e21357dd4c920bfe7593e318923ca3'
final_review=Path.home()/'.codex/incant-reviews/physics-inert-manifolds-final-20261013.json';assert h(final_review)=='6a5052fd7a824e7e7b1a37b9e626c784c4dfc6b5969fcc783e0c29f1eff59faa'
paths=git('diff','--name-only',main,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml').decode().splitlines();expected=['crates/incant_physics_snapshot/'+x for x in ['src/attachment.rs','src/contact_manifolds/codec.rs','src/contact_manifolds/mod.rs','src/contact_manifolds/validate.rs','src/contact_points.rs','src/contact_points_codec.rs','src/lib.rs','tests/manifolds.rs','tests/trusted_manifolds.rs']];assert sorted(paths)==sorted(expected)
assert h(r/'Cargo.lock')==sha(git('show',main+':Cargo.lock'))
normal=Path(next(x for x in checks if x['name']=='normal-graph-001')['stdout']['path']).read_text();assert len(normal.splitlines())==1 and normal.startswith('incant_physics_snapshot ')
summary={'passed':True,'checks':8,'counts':counts,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_crate_inputs':len(sources)-1,'producer_records_rehashed':len(p['records']),'pinned_sources':len(p['pinned']),'changed_source_paths':expected,'lock_unchanged':True,'main_ancestor':main,'live_restore_or_save_admission':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'inert-manifolds-run.py','run.py'),(review,'independent-source-review.json'),(final_review,'producer-final-review.json')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'main_ancestor':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'independent_source_review':{'path':str((d/'independent-source-review.json').relative_to(r)),'sha256':h(review)},'producer_final_review':{'path':str((d/'producer-final-review.json').relative_to(r)),'sha256':h(final_review)},'limits':['79 cases include prior65 inert cases plus14 manifold cases; counts are not additive.','All33 crate source/test/manifest files equal the reviewed producer; the older producer lock differs by already-merged profiling packages/edges; lock is unchanged from currentmain201.','No live constructor, whole-owner authentication, reciprocal solver/island ownership, physics-save admission, GPU, shipping, device or performance claim.'],'review_state':'Final manifold integration acceptance pending'}
p=r/'docs/spikes/evidence/physics-manifolds-main201.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(p,h(p),flush=True)
