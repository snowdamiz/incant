from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,shutil,subprocess,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/clusters-drs-main196-001';d=r/'docs/spikes/evidence/clusters-drs-main197'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
cluster='41fa35bff795f988642c2d4f2b3cbf2e5fe032d9';drs='92a3dfecfcf1cbc841b83f93fb0f705775abf254';main='e67c200d3a83263b4db1e57d153762f78ff78473'
scope=['crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github']
f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['commands']==14 and t['head']==f['head'] and t['all_frozen_inputs_unchanged'] and not t['gpu_execution']
for p,s in f['files'].items():assert h(r/p)==s,p
assert not git('diff','--name-only',f['head'],'HEAD','--',*scope).strip()
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
checks=read(o/'results.json');assert len(checks)==14 and all(x['exit']==0 for x in checks)
counts=[]
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
 if row['name'] in ['shader-cook','default-cpu','cooked-cpu','cluster-only']:
  text=''.join(Path(row[k]['path']).read_text() for k in ['stdout','stderr']);m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
  counts.append({'name':row['name'],'passed':sum(int(x[0]) for x in m),'failed':sum(int(x[1]) for x in m),'ignored':sum(int(x[2]) for x in m),'nonempty_groups':sum(int(x[0])+int(x[2])>0 for x in m)})
assert [(x['passed'],x['failed'],x['ignored']) for x in counts]==[(25,0,0),(82,0,90),(109,0,90),(80,0,33)]
seams=['.github/workflows/ci.yml','.github/workflows/desktop.yml','crates/incant_render/Cargo.toml','crates/incant_render/src/lib.rs','crates/incant_render/src/lighting/mod.rs','crates/incant_render/tests/lighting_gpu/extended.rs','crates/incant_render/tests/lighting_gpu/mod.rs']
sources=[]
for p in git('diff','--name-only',main,'HEAD','--',*scope).decode().splitlines():
 b=(r/p).read_bytes();refs=[]
 for ref in [cluster,drs]:
  z=subprocess.run(['git','show',ref+':'+p],cwd=r,capture_output=True)
  if z.returncode==0 and z.stdout==b:refs.append(ref)
 assert refs or p in seams,p
 sources.append({'path':p,'sha256':sha(b),'exact_producer_commits':refs,'reviewed_seam':not bool(refs)})
assert len(sources)==28 and sum(bool(x['exact_producer_commits']) for x in sources)==21
assert (r/'Cargo.lock').read_bytes()==git('show',main+':Cargo.lock')
# Only additive cluster feature beyond current main's manifest; cache and existing flags retained.
manifest=(r/'crates/incant_render/Cargo.toml').read_text();old=git('show',main+':crates/incant_render/Cargo.toml').decode()
assert manifest.replace('cooked-clusters = ["cooked-output"]\n','')==old
for p,n in [('.github/workflows/ci.yml',1),('.github/workflows/desktop.yml',2)]:
 now=(r/p).read_text();before=git('show',main+':'+p).decode();assert now.replace(',incant_render/cooked-clusters','')==before;assert now.count(',incant_render/cooked-clusters')==n
for p,n in [('crates/incant_render/tests/lighting_gpu/mod.rs',3),('crates/incant_render/tests/lighting_gpu/extended.rs',1)]:
 b=(r/p).read_text();needle='crate::test_support::checked(Renderer::headless().unwrap())';assert b.count(needle)==n;assert b.replace(needle,'Renderer::headless().unwrap()')==git('show',cluster+':'+p).decode()
assert h(o/'integration-seams.patch')=='22a2410e9adfeb5fc103d3ae25fc21fa212b50735f0aac5709c572f8c4f8858f'
producer_artifacts=[]
def raw_copy(row,base):
 raw=Path(row['original']).read_bytes();z=(base/row['retained']).read_bytes();assert sha(raw)==row['raw_sha256'] and len(raw)==row['bytes'];assert sha(z)==row['sha256'];assert (gzip.decompress(z) if row.get('compression')=='gzip' else z)==raw
 producer_artifacts.append({'original':row['original'],'sha256':sha(raw),'retained':str((base/row['retained']).relative_to(r)),'retained_sha256':sha(z)})
for name in ['cooked-cluster-shaders.json','dynamic-resolution.json']:
 p=read(r/'docs/spikes/evidence'/name)
 for row in p['artifacts']:raw_copy(row,r)
for row in read(r/'handoffs/0067-dynamic-resolution/evidence/manifest.json'):raw_copy(row,r/'handoffs/0067-dynamic-resolution/evidence')
archives=[]
for name in ['cluster-grid-readback-002/artifacts.json.gz','cluster-grid-readback-gpu002/artifacts.json.gz']:
 p=r/'docs/spikes/evidence'/name;a=json.loads(gzip.decompress(p.read_bytes()))
 for s,v in a['payloads'].items():assert sha(base64.b64decode(v))==s
 for row in a['records']:
  raw=Path(row['original']).read_bytes();assert len(raw)==row['bytes'] and sha(raw)==row['sha256'] and raw==base64.b64decode(a['payloads'][row['sha256']])
 archives.append({'path':str(p.relative_to(r)),'sha256':h(p),'records':len(a['records']),'payloads':len(a['payloads'])})
reviews=[]
for name in ['dynamic-resolution-0067-review.json','cluster-grid-readback-gpu002/final-review.json']:
 p=r/'docs/spikes/evidence'/name;reviews.append({'path':str(p.relative_to(r)),'sha256':h(p)})
assert reviews[0]['sha256']=='7f61de512ea3365e43e998e887c87a61605e0092a9dfb1ed634fde814cc4a5d5'
cp=read(r/'docs/spikes/evidence/cluster-grid-readback.json');assert cp['gpu']['records_per_variant']==84 if 'records_per_variant' in cp['gpu'] else cp['verification']['gpu_records_per_variant']==84
for row in cp['gpu']['checks']:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
 assert row['exit']==0 and not row['timeout']
# Historical GPU proof remains historical: no execution/appearance inference from this collector.
summary={'passed':True,'checks':len(checks),'frozen_inputs':len(f['files']),'counts':counts,'counts_overlap':True,'changed_sources':len(sources),'exact_producer_files':21,'reviewed_seams':7,'producer_raw_retained_pairs':len(producer_artifacts),'producer_archive_records':sum(x['records'] for x in archives),'actual_main_ancestor':main,'compiled_inputs_unchanged_after_test':True,'lock_unchanged_from_main':True,'current_gpu_execution':False}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for p in sorted(o.rglob('*')):
 if p.is_file():
  b=p.read_bytes();s=sha(b);records.append({'original':str(p),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'clusters-drs-run.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'main_ancestor':main,'producers':{'cluster':cluster,'dynamic_resolution':drs},'summary':summary,'checks':checks,'sources':sources,'producer_artifacts':producer_artifacts,'producer_archives':archives,'producer_reviews':reviews,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'review_state':'Final integration review pending','limits':['Current union runs CPU tests and native/window/WebAssembly compilation only; 90 GPU tests remain ignored in each all-target variant.','Test counts overlap across default/cooked/cluster-only feature configurations.','Original cooked-cluster Metal numerical parity and default-feature DRS appearance/latency evidence remain producer-scoped, not new combined GPU acceptance.','CI adds cooked-clusters to existing backend-asserted release checks, preserving defaults; actual Linux/Vulkan and Windows/DX12 success remains a frozen checkpoint requirement.','DRS is opt-in nearest expansion; no temporal reconstruction, MetalFX, default enablement, exact GPU timing, phone/performance or phase gate claim.']}
p=r/'docs/spikes/evidence/clusters-drs-main197.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(p,h(p),flush=True)
