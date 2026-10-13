from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,tomllib,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/inert-arenas-main195-001';d=r/'docs/spikes/evidence/physics-inert-main195'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='cb96d821cce1b73db156ce1c01e5a1f69492c75d';parry='025d3a7e53a59d4ab1ff3e68221860d7b59be71a'
f=read(o/'freeze.json');terminal=read(o/'terminal.json');assert terminal['exit']==0 and terminal['all_frozen_inputs_unchanged']
for path,s in f['files'].items():assert h(r/path)==s,path
checks=read(o/'results.json');initial=read(o/'initial-results.json');assert len(checks)==8 and all(x['exit']==0 for x in checks) and checks[:7]==initial[:7] and initial[-1]['exit']==2
for row in checks+initial:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
row=checks[0];matches=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text())
counts={'passed':sum(int(x[0]) for x in matches),'failed':sum(int(x[1]) for x in matches),'ignored':sum(int(x[2]) for x in matches),'nonempty_groups':sum(int(x[0])>0 for x in matches)};assert counts=={'passed':30,'failed':0,'ignored':0,'nonempty_groups':5}
graph=next(x for x in checks if x['name']=='normal-graph-002.json');lines=Path(graph['stdout']['path']).read_text().splitlines();assert len(lines)==1 and lines[0].startswith('incant_physics_snapshot v0.0.1 ')
p=read(r/'docs/spikes/evidence/physics-inert-arenas.json');q=read(r/'docs/spikes/evidence/physics-inert-snapshot.json');sources=[]
for row in p['sources']:
 assert sha(git('show',producer+':'+row['path']))==row['sha256'];current=h(r/row['path'])
 if row['path']!='Cargo.lock':assert current==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':current})
for row in p['pinned_dependency_sources']:assert h(row['original'])==h(row['copy'])==row['sha256']
producer_records=0
for manifest in [p,q]:
 for row in manifest['records']:
  raw=Path(row['original']).read_bytes();stored=(r/row['retained']).read_bytes();assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(stored)==row['retained_sha256'];assert gzip.decompress(stored)==raw;producer_records+=1
 review=manifest['independent_review'];assert review['final_evidence']=='accepted';assert sha(gzip.decompress((r/review['final_receipt_retained']).read_bytes()))==review['final_receipt_sha256']
# Exact inherited engine inputs; the new crate and reviewed lock selection are the only implementation/build changes over the Parry union.
paths=git('diff','--name-only',parry,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml').decode().splitlines()
assert all(x=='Cargo.lock' or x.startswith('crates/incant_physics_snapshot/') for x in paths),paths
old=tomllib.loads(git('show',parry+':Cargo.lock').decode());new=tomllib.loads((r/'Cargo.lock').read_text());key=lambda x:(x['name'],x['version'],x.get('source'),x.get('checksum'));a={key(x):x for x in old['package']};b={key(x):x for x in new['package']};assert set(a)<=set(b)
added=set(b)-set(a);assert {(x[0],x[1]) for x in added}=={('incant_physics_snapshot','0.0.1'),('serde_arrays','0.2.0')}
# Current-base additive lock effects must equal the exact reviewed producer effects on its Parry parent.
pold=tomllib.loads(git('show','396f0602:Cargo.lock').decode());pnew=tomllib.loads(git('show',producer+':Cargo.lock').decode())
byname=lambda v:{(x['name'],x['version']):x for x in v['package']}
pa,pb=byname(pold),byname(pnew);ca,cb=byname(old),byname(new)
delta=lambda before,after:{k:{field:(before.get(k,{}).get(field),v) for field,v in value.items() if before.get(k,{}).get(field)!=v} for k,value in after.items() if before.get(k)!=value}
assert delta(pa,pb)==delta(ca,cb)
summary={'passed':True,'current_checks':8,'current_counts':counts,'frozen_inputs':len(f['files']),'producer_sources':len(sources),'exact_crate_sources':len(sources)-1,'pinned_sources':len(p['pinned_dependency_sources']),'producer_records_rehashed':producer_records,'existing_locked_identities':len(a),'new_locked_identities':[list(x) for x in sorted(added,key=str)],'normal_graph':'new crate only, no dependencies','engine_inputs_unchanged_except_lock':True,'live_solver_or_save_admission':False}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 raw=path.read_bytes();s=sha(raw);records.append({'original':str(path),'sha256':s,'bytes':len(raw)});payloads.setdefault(s,base64.b64encode(raw).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'inert-arenas-run.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'parry_dependency':parry,'producer':producer,'summary':summary,'checks':checks,'initial_failure':initial[-1],'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'producer_reviews':[q['independent_review'],p['independent_review']],'limits':['Inert validation only; no live solver construction, runtime consumer, world-format adoption or PhysicsUnsupported change.','Both producer source/final reviews are preserved. Current30 tests include the prior18 cases; counts are not additive.','All existing package identities are unchanged; pinned serde_arrays0.2.0 and the new crate are added with reviewed dev-only observation feature edges. Normal crate graph has no dependencies.','Only post-test packaging change was lossless gzip for a retained Parry diff. Initial whitespace failure and all unchanged passing checks are retained.','No shipping, GPU, window, phone, performance or complete physics-save claim.'],'review_state':'Independent combined integration review pending'}
path=r/'docs/spikes/evidence/physics-inert-main195.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(path,h(path),flush=True)
