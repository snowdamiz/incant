from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime,tomllib
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o=Path.home()/'.codex/incant-integration-evidence/retained-motion-main202-001'
d=r/'docs/spikes/evidence/retained-motion-main203'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
main='78e18433d23bbaf4ae09b8e651d841660fac171c';source='814717935462e4ee5dbdb1fdae2738d17477f932';producer='f40452012c96b9bca9904edc95402d0e4a724d8a';ancestry='95e223e0dfa26313b7f38e7988e214a3fa4a4386'
f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['head']==f['head'] and t['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
assert git('rev-parse',f['head']+'^{tree}')==git('rev-parse',ancestry+'^{tree}')
checks=read(o/'results.json');assert len(checks)==10 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
counts={}
for label,expected in [('cpu-all',[91,0,92]),('cooked-refusal',[12,0,9])]:
 row=next(x for x in checks if x['name']==label);text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text();m=[list(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)];s=list(map(sum,zip(*m)));assert s==expected
 counts[label]={'passed':s[0],'failed':s[1],'ignored':s[2],'result_groups':len(m),'nonempty_pass_groups':sum(x[0]>0 for x in m)}
p=read(r/'docs/spikes/evidence/retained-motion-history.json');assert h(r/'docs/spikes/evidence/retained-motion-history.json')=='1e6dd3f7962ff1cb0bbcd3ba6cb1efbfa17e18a8a48f59a63dfdda5d31ee3273'
for row in p['sources']+p['unchanged_baselines']:
 assert h(r/row['path'])==sha(git('show',source+':'+row['path']))==row['sha256'],row['path']
for row in p['artifacts']:
 z=(r/row['retained']).read_bytes();raw=gzip.decompress(z);assert sha(z)==row['retained_sha256'] and sha(raw)==row['raw_sha256'] and len(raw)==row['raw_bytes'];assert Path(row['original']).read_bytes()==raw
for row in p['binaries']:assert h(row['retained'])==row['sha256'] and Path(row['retained']).stat().st_size==row['bytes']
assert h(r/p['source_review']['path'])==p['source_review']['sha256']
expected=sorted(git('diff','--name-only',source+'^',source,'--','crates','Cargo.lock','Cargo.toml').decode().splitlines())
actual=git('diff','--name-only',main,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml','editor/app').decode().splitlines();assert sorted(actual)==expected,(actual,expected)
old=tomllib.loads(git('show',main+':Cargo.lock').decode())['package'];new=tomllib.loads((r/'Cargo.lock').read_text())['package'];assert len(old)==len(new)
lock_delta=[]
for a,b in zip(old,new):
 if a!=b:
  assert a['name']==b['name']=='incant_render';aa=dict(a);bb=dict(b);ad=aa.pop('dependencies');bd=bb.pop('dependencies');assert aa==bb and sorted(bd)==sorted(ad+['naga']);lock_delta.append('incant_render dev-only naga edge')
assert len(lock_delta)==1
n=read(r/'docs/spikes/evidence/retained-motion-numeric.json');assert n['source']==source and n['numeric']['passed']==2
arc=r/n['archive']['path'];assert h(arc)==n['archive']['sha256'];z=read(r/n['review']['path']);assert h(r/n['review']['path'])==n['review']['sha256']
a=json.loads(gzip.decompress(arc.read_bytes()));assert len(a['records'])==15 and len(a['payloads'])==13
raw_by_path={}
for row in a['records']:
 b=base64.b64decode(a['payloads'][row['sha256']]);assert sha(b)==row['sha256'] and len(b)==row['bytes'];assert Path(row['original']).read_bytes()==b;raw_by_path[row['original']]=b
results=json.loads(next(b for path,b in raw_by_path.items() if path.endswith('/gpu-results.json')));assert len(results)==2
for row in results:
 assert row['exit']==0 and row['timeout'] is False and row['wait_error'] is None and row['features']==['default'];assert row['name'] in n['numeric']['tests'];assert row['binary_sha256']==n['numeric']['binary_sha256']==h(row['argv'][0]);assert '--ignored' in row['argv'] and '--exact' in row['argv']
 for k in ['stdout','stderr']:assert sha(raw_by_path[row[k]['path']])==row[k]['sha256']
 assert b'1 passed; 0 failed; 0 ignored' in raw_by_path[row['stdout']['path']]
reviews=[('retained-motion-history-cpu-final-20261013.json','3b2bd8f37250971f96efc4a1f6c4f2f8cf06df4bd37a62f25d3347f15ab73337'),('retained-motion-numeric-final-20261013.json','39060ae23d23dd2ee330963014198be7f80bb334e50d09853566991763b03f7a')]
for name,s in reviews:assert h(Path.home()/'.codex/incant-reviews'/name)==s
summary={'passed':True,'checks':10,'unique_command_labels':9,'counts':counts,'frozen_inputs':len(f['files']),'producer_sources':len(p['sources']),'unchanged_baselines':len(p['unchanged_baselines']),'producer_artifacts_rehashed':len(p['artifacts']),'producer_binaries_rehashed':len(p['binaries']),'numeric_records':len(a['records']),'numeric_payloads':len(a['payloads']),'prior_numeric_processes':2,'changed_source_paths':expected,'lock_delta':lock_delta,'current_gpu_execution':False,'appearance_acceptance':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
archive=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(archive)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(archive)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'retained-motion-main202-run.py','run.py')]:shutil.copyfile(src,d/name)
review_rows=[]
for name,s in reviews:
 dest=d/name;shutil.copyfile(Path.home()/'.codex/incant-reviews'/name,dest);review_rows.append({'path':str(dest.relative_to(r)),'sha256':s})
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'ancestry_head':ancestry,'main_ancestor':main,'source':source,'producer':producer,'summary':summary,'checks':checks,'sources':p['sources'],'unchanged_baselines':p['unchanged_baselines'],'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(archive),'bytes':len(archive),'records':len(records),'payloads':len(payloads)},'producer_cpu_manifest':{'path':'docs/spikes/evidence/retained-motion-history.json','sha256':h(r/'docs/spikes/evidence/retained-motion-history.json')},'producer_numeric_manifest':{'path':'docs/spikes/evidence/retained-motion-numeric.json','sha256':h(r/'docs/spikes/evidence/retained-motion-numeric.json')},'reviews':review_rows,'limits':['Current composed CPU/compile checks only; two accepted prior default-feature numerical GPU assertion processes remain producer-scoped. No current composed GPU or appearance claim.','The conventions command ran twice; its identical empty stdout/stderr share paths overwritten by the second invocation. Ten executions, nine unique labels; no distinct log bytes are claimed.','91 ordinary CPU and12 cooked-refusal cases overlap; ignored92/9 GPU cases were not executed by this lane.','No temporal color resolve, TAA, temporal upscaling, MetalFX, native cooked motion path, phone or performance gate.','Timestamp composition is not in this source. The separate motion render pass must receive its own timestamp span or an explicit exclusion when composing the timestamp candidate.','Cargo.lock adds only incant_render dev-only naga; no package identity changes.'],'review_state':'Current-main integration final independent review pending'}
path=r/'docs/spikes/evidence/retained-motion-main203.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(path,h(path),flush=True)
