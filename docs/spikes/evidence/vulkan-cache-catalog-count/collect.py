from pathlib import Path
import base64,gzip,hashlib,json,subprocess,sys,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/vulkan-cache-catalog-repair';d=r/'docs/spikes/evidence/vulkan-cache-catalog-count'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
main='7f0e61f245ad1c4a49e6f2dec908f6274f8f02dc';candidate='2fab0688fce68f4933ba4f14a7af074a088bfeb6';p='crates/incant_render/tests/persistent_cache.rs'
f=read(o/'freeze.json');assert all(h(r/k)==v for k,v in f.items());checks=read(o/'results.json');assert len(checks)==5 and all(x['exit']==0 for x in checks);assert read(o/'terminal.json')['exit']==0
for row in checks:
 for k in ['stdout','stderr']:assert h(o/(row['name']+'.'+k))==row[k+'_sha256']
old=git('show',main+':'+p).decode();current=(r/p).read_text();assert current==old.replace('text + 3 UI.','text + 4 UI.').replace('num_allocated, 30','num_allocated, 31').replace('Vulkan catalog: 30 render + 1 compute','Vulkan catalog: 31 render + 1 compute');assert old==git('show',candidate+':'+p).decode()
# Independently bind all production pipeline constructor bytes to failed candidate.
constructors=['crates/incant_render/src/'+x for x in ['materials.rs','shadows/mod.rs','lib.rs','output.rs','sprites/mod.rs','text/mod.rs','ui/mod.rs']]
for x in constructors:
 assert (r/x).read_bytes()==git('show',main+':'+x)
 if x.endswith('/lib.rs'):
  now=(r/x).read_text();before=git('show',candidate+':'+x).decode()
  section=lambda text:text[text.index('    pub async fn new('):text.index('    pub fn headless(')]
  assert section(now)==section(before).replace('lighting::LightingSystem::new(&device, cache);','lighting::LightingSystem::new(&device, adapter.get_info().backend, cache)?;')
 else:assert (r/x).read_bytes()==git('show',candidate+':'+x)
job=read(o/'hosted-job.json');run=read(o/'hosted-run.json');assert job['id']==115136684605 and job['conclusion']=='failure' and job['head_sha']==candidate and run['head_sha']==candidate
log=o.parent/'checkpoint19/linux-desktop-115136684605.log';text=log.read_text();assert 'left: 31' in text and 'right: 30' in text and 'persistent_cache.rs:107:5' in text and 'backend: Vulkan' in text
review=Path.home()/'.codex/incant-reviews/vulkan-cache-count-source-20261013.json';assert h(review)=='ccf6495fea05e957d641f746c6de98af17eccb8741a35313be3d8148558e2410';assert h(o/'repair.patch')=='a1b5f2dfcce99faceb8682d553351abce74a6c809649dbd6605a8f49f270e972'
summary={'checks':5,'source_changes':1,'changed_lines':3,'production_constructor_inputs_exact_main':len(constructors),'candidate_catalog_sources_exact':6,'candidate_constructor_seam':'existing cooked-cluster backend/cache call only','expected_render_pipelines':31,'expected_compute_pipelines':1,'hosted_red':{'candidate':candidate,'run':run['id'],'job':job['id'],'exit':101,'actual':31,'expected':30},'gpu_execution':False,'actual_linux_rerun':'pending next frozen checkpoint'};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted([p for p in o.iterdir() if p.is_file()]+[log,review]):
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o.parent/'vulkan-cache-catalog-run.py','run.py'),(o/'freeze.json','freeze.json'),(review,'source-review.json')]:shutil.copyfile(src,d/name)
m={'main':main,'source':{'path':p,'sha256':h(r/p)},'summary':summary,'checks':checks,'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'source_review':{'path':str((d/'source-review.json').relative_to(r)),'sha256':h(review)},'patch_sha256':h(o/'repair.patch'),'limits':['Test-only correction. All persistence lifecycle assertions, exact counts, backend assertion, checksum corruption and resource stability checks remain.','Full harness compiled with cooked features and Clippy on macOS. No Vulkan/Metal/window execution by publisher.','Original Linux failure remains immutable; four fresh Vulkan subprocess stages must pass in a successor checkpoint.']};p=r/'docs/spikes/evidence/vulkan-cache-catalog-count.json';p.write_text(json.dumps(m,indent=2)+'\n');print(p,h(p))
