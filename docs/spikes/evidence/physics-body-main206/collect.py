from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime,tomllib
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/physics-body-main205-001';d=r/'docs/spikes/evidence/physics-body-main206'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='c250c23eb2bed7acd667b2e97bcb10789217463c';main=read(o.parent/'pr206/merge.json')['main'];f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['head']==f['head'] and t['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
ancestry=read(o/'ancestry.json');assert ancestry['tested_head']==f['head'] and ancestry['main']==main;assert git('rev-parse',f['head']+'^{tree}')==git('rev-parse',ancestry['head']+'^{tree}')
checks=read(o/'results.json');assert len(checks)==8 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
text=Path(checks[0]['stdout']['path']).read_text()+Path(checks[0]['stderr']['path']).read_text();rows=[list(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)]
counts={'passed':sum(x[0] for x in rows),'failed':sum(x[1] for x in rows),'ignored':sum(x[2] for x in rows),'nonempty_groups':sum(x[0]>0 for x in rows),'result_groups':len(rows)};assert counts=={'passed':101,'failed':0,'ignored':0,'nonempty_groups':16,'result_groups':18}
p=read(r/'docs/spikes/evidence/physics-inert-body-state.json');assert h(r/'docs/spikes/evidence/physics-inert-body-state.json')=='121429371ff5c6c8a3843e28a57d35447ec1e82fc0672cbfba75fa84600f8828';sources=[]
for row in p['sources']:
 assert sha(git('show',producer+':'+row['path']))==row['sha256'];now=h(r/row['path'])
 if row['path']!='Cargo.lock':assert now==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':now})
for row in p['records']:
 raw=Path(row['original']).read_bytes();z=(r/row['retained']).read_bytes();assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(z)==row['retained_sha256'] and gzip.decompress(z)==raw
for row in p['pinned']:assert h(row['source'])==h(row['copy'])==row['sha256']
review=Path.home()/'.codex/incant-reviews/physics-inert-body-state-final-20261013.json';assert h(review)=='dd28c87d1df24b51b563d826ba47880784af08d3ab2cb8755b5b8615d55f7b54'
actual=git('diff','--name-only',main,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml').decode().splitlines();expected=git('diff','--name-only',producer+'^',producer,'--','crates','Cargo.lock').decode().splitlines();assert sorted(actual)==sorted(expected)
key=lambda x:(x['name'],x['version'],x.get('source',''))
old={key(x):x for x in tomllib.loads(git('show',main+':Cargo.lock').decode())['package']};new={key(x):x for x in tomllib.loads((r/'Cargo.lock').read_text())['package']};assert old.keys()==new.keys();lock_delta=[]
assert old==new
normal=Path(next(x for x in checks if x['name']=='normal-graph-001')['stdout']['path']).read_text();assert len(normal.splitlines())==1 and normal.startswith('incant_physics_snapshot ')
summary={'passed':True,'checks':8,'counts':counts,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_crate_inputs':len(sources)-1,'producer_records_rehashed':len(p['records']),'pinned_sources':len(p['pinned']),'changed_source_paths':expected,'lock_delta':lock_delta,'package_identities_unchanged':len(new),'main_ancestor':main,'live_restore_or_save_admission':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'physics-body-main205-run.py','run.py'),(review,'producer-final-review.json')]:shutil.copyfile(src,d/name)
m={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'ancestry_head':ancestry['head'],'main_ancestor':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'producer_final_review':{'path':str((d/'producer-final-review.json').relative_to(r)),'sha256':h(review)},'limits':['101 cases include the prior88 pair-state cases; counts are not additive.','All44 crate source/test/manifest files equal acceptedc250. Older producer lock predates merged profiling packages/edges and renderer dev naga; current lock equals actualmain206.','No live constructor, complete owner authentication, reciprocal solver/island ownership, previous-cluster scratch reconstruction, physics-save admission, GPU, shipping, device or performance claim.'],'review_state':'Current-main final body-state integration review pending'}
path=r/'docs/spikes/evidence/physics-body-main206.json';path.write_text(json.dumps(m,indent=2)+'\n');print(path,h(path),flush=True)
