from pathlib import Path
import base64,gzip,hashlib,json,subprocess,sys,shutil,datetime,re
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/editor-renderer-import-main192-001';d=r/'docs/spikes/evidence/editor-renderer-import';cp=o.parent/'checkpoint18'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==6 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ('stdout','stderr'):assert h(c[s]['path'])==c[s]['sha256']
assert '1 passed; 0 failed; 1 ignored' in (o/'editor-cpu.stdout').read_text()
assert '1 passed; 0 failed; 0 ignored' in (o/'renderer-backend-contract.stdout').read_text()
source='editor/app/tests/model_gpu.rs';base='7fc413d23cbb65e244ca515ca5d684aec21cfb90'
assert git('diff','--name-only',base,'--','crates','editor','Cargo.toml','Cargo.lock','tools','.github')==source
patch=subprocess.check_output(['git','diff',base,'--',source],cwd=r);assert sha(patch)=='a7b9542b70171d1aca5ef17ce4b3be6130f34ce2104a92686240b6be39d543d3'
assert patch==(cp/'editor-renderer-import.patch').read_bytes()
assert subprocess.check_output(['git','show','dbace214b5bef7d6708a871b8059125673939732:'+source],cwd=r)==subprocess.check_output(['git','show',base+':'+source],cwd=r)
logs=[]
for name,job,wrapper_exit in [('linux-desktop-115113538804.log',115113538804,101),('windows-desktop-115113538960.log',115113538960,1)]:
 p=cp/name;s=p.read_text();assert 'error[E0425]: cannot find type `Renderer` in the crate root' in s;assert 'crate::Renderer' in s;assert f'Process completed with exit code {wrapper_exit}.' in s
 logs.append({'job':job,'wrapper_exit':wrapper_exit,'path':str(p),'sha256':h(p),'observed':'E0425 shared test helper missing Renderer root alias'})
summary={'passed':True,'frozen_inputs':len(f['files']),'checks':6,'editor_cpu':{'passed':1,'ignored_gpu':1},'renderer_cpu':{'passed':1,'filtered_out':95},'failed_hosted_jobs':2,'production_unchanged':True,'gpu_executed':False}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
paths=[p for p in sorted(o.rglob('*')) if p.is_file()]+[cp/x for x in ('linux-desktop-115113538804.log','windows-desktop-115113538960.log','editor-renderer-import.patch')]
for p in paths:
 b=p.read_bytes();s=sha(b);records.append({'original':str(p),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
encoded=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);p=d/'artifacts.json.gz';p.write_bytes(encoded);assert len(encoded)<16*1024*1024
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(o.parent/'editor-renderer-import-main192-run.py','run.py'),(cp/'editor-renderer-import.patch','repair.patch')]:shutil.copyfile(src,d/name)
review={'kind':'parent-message source acceptance','reviewer':'root','patch_sha256':sha(patch),'acceptance':'Read editor-renderer-import.patch a7b9542b: source accepted, two-line test-only correction imports root alias needed by sharedsupport and routes its existing actualheadless renderer through unchangedchecked backend assertion. Await scoped compile/Clippy/regression evidence before finalrepair acceptance; noGPU resultclaim.'};(d/'source-review.json').write_text(json.dumps(review,indent=2)+'\n')
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'base':base,'tested_head':f['head'],'summary':summary,'source':{'path':source,'sha256':h(r/source)},'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'patch_sha256':sha(patch),'checks':checks,'hosted_failures':logs,'archive':{'path':str(p.relative_to(r)),'sha256':h(p),'bytes':len(encoded),'records':len(records),'payloads':len(payloads)},'limits':['Test-only current-main repair; backend assertion unchanged and now also applied to the editor GPU renderer','No local GPU execution or actual hosted repair success claimed; checkpoint18 remains frozen and unpromoted','Local macOS compilation and CPU tests do not substitute for Linux/Windows full release checks'],'review_state':'Final independent execution review pending'}
p=r/'docs/spikes/evidence/editor-renderer-import.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(str(p),h(p),flush=True)
