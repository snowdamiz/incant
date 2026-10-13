from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/inert-graphs-main195-001';d=r/'docs/spikes/evidence/physics-inert-main196'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='c5e733aea806bb1e79c9b4514fe3ab6d87b0be00';main='3da8b0ebeab43f715b0531603e30a864e801be11'
f=read(o/'freeze.json');terminal=read(o/'terminal.json');assert terminal['exit']==0 and terminal['head']==f['head'] and terminal['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
checks=read(o/'results.json');assert len(checks)==8 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
text=Path(checks[0]['stdout']['path']).read_text()+Path(checks[0]['stderr']['path']).read_text();matches=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
counts={'passed':sum(int(x[0]) for x in matches),'failed':sum(int(x[1]) for x in matches),'ignored':sum(int(x[2]) for x in matches),'nonempty_groups':sum(int(x[0])>0 for x in matches)};assert counts=={'passed':40,'failed':0,'ignored':0,'nonempty_groups':7}
p=read(r/'docs/spikes/evidence/physics-inert-graphs.json');sources=[]
for row in p['sources']:
 assert sha(git('show',producer+':'+row['path']))==row['sha256'];now=h(r/row['path'])
 if row['path']!='Cargo.lock':assert now==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':now})
for row in p['records']:
 raw=Path(row['original']).read_bytes();z=(r/row['retained']).read_bytes();assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(z)==row['retained_sha256'] and gzip.decompress(z)==raw
for row in p['pinned_dependency_sources']:assert h(row['original'])==h(row['copy'])==row['sha256']
review=Path.home()/'.codex/incant-reviews/physics-inert-graphs-final-20261013.json';reviewhash=h(review);assert reviewhash.startswith('6acc');assert read(review)['head']==producer and read(review)['source_findings']==[]
prior='ad050c7642cc94a6fba53d6b269e4d5b108f46af';paths=git('diff','--name-only',prior,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml').decode().splitlines()
expected=['crates/incant_physics_snapshot/'+x for x in ['src/graph.rs','src/graph_codec.rs','src/lib.rs','tests/graphs.rs','tests/trusted_graphs.rs']];assert sorted(paths)==sorted(expected)
priorreview=Path.home()/'.codex/incant-reviews/physics-inert-main195-final-20261013.json';assert h(priorreview)=='de15aa9a7298eb7aa799c24b7bb632aa31db67a210d6ef3d118d803abea4ec38'
assert h(r/'Cargo.lock')==sha(git('show',prior+':Cargo.lock'))
assert len(Path(next(x for x in checks if x['name']=='normal-graph-002.json')['stdout']['path']).read_text().splitlines())==1
summary={'passed':True,'checks':8,'counts':counts,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_crate_inputs':len(sources)-1,'producer_records_rehashed':len(p['records']),'pinned_sources':len(p['pinned_dependency_sources']),'prior_accepted_inert_inputs_changed':expected,'lock_and_engine_unchanged_after_graph':True,'main_ancestor':main,'live_restore_or_save_admission':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'inert-graphs-run.py','run.py'),(review,'producer-final-review.json'),(priorreview,'prior-integration-review.json')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'main_ancestor':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'producer_review':{'path':str((d/'producer-final-review.json').relative_to(r)),'sha256':reviewhash},'prior_review':{'path':str((d/'prior-integration-review.json').relative_to(r)),'sha256':h(priorreview)},'limits':['All40 cases include prior30 snapshot/arena cases and10 graph cases; counts are not additive.','The separately accepted main195 snapshot+arena packet remains immutable; exactly5 graph source/test files change from that accepted union, with no lock/build/engine delta.','Actual main196 join changes only documentation/review evidence after frozen checks; source and build identities remain byte-exact.','No live constructor, contact-payload policy, full solver ownership composition, physics-save admission, GPU, shipping, device or performance claim.'],'review_state':'Final appended-graph integration acceptance pending'}
path=r/'docs/spikes/evidence/physics-inert-main196.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(path,h(path),flush=True)
