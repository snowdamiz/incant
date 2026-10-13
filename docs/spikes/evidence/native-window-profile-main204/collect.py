from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/native-window-profile-main203-001';d=r/'docs/spikes/evidence/native-window-profile-main204'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='94a97c6a9ad310bd3ab219016629abb0b5df47c0';merge=read(o.parent/'pr204/merge.json');main=merge['main'];f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['head']==f['head'] and t['all_frozen_inputs_unchanged']
for p,s in f['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
ancestry=read(o/'ancestry.json');assert ancestry['tested_head']==f['head'] and ancestry['main']==main;assert git('rev-parse',f['head']+'^{tree}')==git('rev-parse',ancestry['head']+'^{tree}')
checks=read(o/'results.json');assert len(checks)==10 and len({x['name'] for x in checks})==10 and all(x['exit']==0 for x in checks)
counts={}
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
 text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text();matches=[list(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)]
 if matches:counts[row['name']]={'passed':sum(x[0] for x in matches),'failed':sum(x[1] for x in matches),'ignored':sum(x[2] for x in matches),'groups':len(matches)}
for name,num in {'default-player':20,'window-profile':69,'profile-cli':1,'summary':4}.items():assert counts[name]['passed']==num and counts[name]['failed']==counts[name]['ignored']==0
p=read(r/'docs/spikes/evidence/native-window-profile.json');assert h(r/'docs/spikes/evidence/native-window-profile.json')=='8b18740b34743ae3863b9b6a59b13b45d44222f1652ce31ee1ffe8e13da26945'
sources=[]
for path,s in p['source_files'].items():
 assert sha(git('show',producer+':'+path))==s;current=h(r/path)
 if path!='Cargo.lock':assert current==s,path
 sources.append({'path':path,'producer_sha256':s,'current_sha256':current,'exact':current==s})
assert h(r/'Cargo.lock')==sha(git('show',main+':Cargo.lock'))
archive=git('show',producer+':'+p['archive']['path']);assert sha(archive)==p['archive']['sha256'];a=json.loads(gzip.decompress(archive));assert len(a['records'])==67 and len(a['payloads'])==65
for row in a['records']:
 b=base64.b64decode(a['payloads'][row['sha256']]);assert sha(b)==row['sha256'] and len(b)==row['bytes']
 if row['path'].startswith('source/'):assert git('show',producer+':'+row['path'][7:])==b
 else:assert Path(row['path']).read_bytes()==b
sourcepaths=git('diff','--name-only',producer+'^',producer,'--','crates').decode().splitlines();actual=git('diff','--name-only',main,'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','.gitattributes','.github','rust-toolchain.toml','editor/app').decode().splitlines();assert sorted(actual)==sorted(sourcepaths)
review=Path.home()/'.codex/incant-reviews/native-window-profile-final-20261013.json';assert h(review)=='b341a9c5ef6a1d1ca14652133d18c0b2f24908ad2d39a690f5344f30855889c6'
summary={'passed':True,'commands':10,'counts':counts,'counts_overlap':True,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_producer_inputs':sum(x['exact'] for x in sources),'producer_records_rehashed':67,'producer_payloads_rehashed':65,'changed_source_paths':sourcepaths,'lock_unchanged_from_main':True,'actual_window_execution':False,'actual_tracy_capture':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'native-window-profile-main203-run.py','run.py'),(review,'producer-final-review.json')]:shutil.copyfile(src,d/name)
m={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'ancestry_head':ancestry['head'],'main_ancestor':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'records':len(records),'payloads':len(payloads)},'producer_review':{'path':str((d/'producer-final-review.json').relative_to(r)),'sha256':h(review)},'limits':['Counts overlap across features. Current actual offline CLI proof passes; native-window feature tests only compile and exercise pre-window errors/pure helpers. No native-window launch or live Tracy-client execution.','Five production files match independently reviewed producer bytes. Only producer Cargo.lock differs by already merged renderer dev-only naga; current lock unchanged from main204.','No GPU, rendered-pixel, phone, shipping performance or profiler-completion claim.','Original producer archive retains thirteen actual raw files and54 Git source payloads; relative source/ paths are synthetic retained entries, not missing absolute original files.'],'review_state':'Current-main final independent integration review pending'}
path=r/'docs/spikes/evidence/native-window-profile-main204.json';path.write_text(json.dumps(m,indent=2)+'\n');print(path,h(path),flush=True)
