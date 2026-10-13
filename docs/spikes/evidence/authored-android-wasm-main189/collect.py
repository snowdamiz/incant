from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil,tomllib
sys.dont_write_bytecode=True
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');p=Path('/Users/andreyyurlov/.codex/worktrees/agent-budget-settings/incant');base=Path.home()/'.codex/incant-integration-evidence';o=base/'authored-android-wasm-main189-001';target=base/'authored-android-wasm-main189-target-004';d=r/'docs/spikes/evidence/authored-android-wasm-main189'
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
def h(p):
 v=hashlib.sha256()
 with Path(p).open('rb') as f:
  for x in iter(lambda:f.read(1024*1024),b''):v.update(x)
 return v.hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip();f=read(o/'freeze.json');assert f['head']=='04c816cf7250cb66ccb109b4a66c42cff517d3d6';assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==read(target/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==11 and all(c['exit']==0 for c in checks)
for c in checks:
 for stream in ('stdout','stderr'):assert h(c[stream]['path'])==c[stream]['sha256']
tests=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',(o/'cook-tests.stdout').read_text());assert all(int(x[1])==int(x[2])==0 for x in tests)
original=Path.home()/'.codex/incant-evidence/authored-android-wasm-001/audit.py';source=original.read_text();prefix=source.split("(OUT/'producer-reaudit.json').write_text")[0];ns={'__file__':str(original)};exec(compile(prefix,str(original),'exec'),ns);producer_execution=ns['value'];assert producer_execution['fresh_android_processes']==6 and producer_execution['android_refusals']==26
preflight_prefix=source.split('records = []')[0];body=source.split('records = []')[1].split("    exported=OUT/")[0];body=body.replace("OUT.parent / 'authored-android-wasm-preflight-001' / mode","CURRENT / (mode+'-prepared')").replace("    assert not (prepared / 'authoring').exists()","    assert (prepared / 'authoring').is_dir()")
current_ns={'__file__':str(original),'CURRENT':o};exec(compile(preflight_prefix+'records = []'+body,str(original),'exec'),current_ns)
rows=read(target/'results.json');assert len(rows)==2 and {x['mode'] for x in rows}=={'deterministic','fast'}
sys.path.insert(0,str(r/'tools'));import android_archive,android_wasm,physics_features
for row in rows:
 mode=row['mode'];td=target/mode;audit=read(td/'audit/commands.json');assert len(audit)==12 and all(x['exit_code']==0 for x in audit)
 for c in audit:
  for stream in ('stdout','stderr'):assert h(td/'audit'/c[stream])==c[stream+'_sha256']
 for edges,gr in row['graphs'].items():
  a=(td/('producer-'+edges.replace(',','-')+'.txt')).read_text().replace(str(p),'<checkout>');b=(td/('current-'+edges.replace(',','-')+'.txt')).read_text().replace(str(r),'<checkout>');assert a==b and sha(a.encode())==gr['normalized_sha256'];assert not ({'incant_render','incant_shader_cook'}&set(gr['packages']))
 for item in row['source_rows']:
  assert h(r/item['path'])==item['current'];assert sha(subprocess.check_output(['git','show','a11bd4f4:'+item['path']],cwd=r))==item['producer']
 assert [i['path'] for i in row['source_rows'] if i['current']!=i['producer']]==['crates/incant_script/src/restart_request_tests.rs']
 command=audit[-1];assert command['argv'][1]=='check';stdout=(td/'audit'/command['stdout']).read_text();stderr=(td/'audit'/command['stderr']).read_text();messages=[json.loads(line) for line in stdout.splitlines() if line.startswith('{')];target_dir=Path(command['argv'][command['argv'].index('--target-dir')+1]);assert physics_features.validate_artifacts(messages,mode,target_dir=target_dir,target=android_archive.TARGET,profile='shipping')==row['physics'];assert android_wasm.audit_build(stdout,stderr,android_archive.TARGET,row['module'],r)==row['c_audit'];assert len(row['c_audit']['sources'])==5
 receipt=read(td/'project-export.json');assert h(td/'project-export.json')==row['project_receipt_sha256'];android_archive.producer_receipt(receipt,h(o/f'{mode}-prepared/authoring/game.json'),mode,row['module']['program'])
 assert row['module']['translation']==read(td/'wasm-translation.json')
 for name,digest in row['module']['translation']['files'].items():assert h(Path(row['module']['bundle'])/name)==digest
for name in ('guest-path-control','guest-current-path-control'):
 control=base/'authored-android-wasm-main189-target-002'/name;v=read(control/'result.json');assert v['original_wasm_sha256']==v['reconstructed_sha256']==h(control/'behavior.wasm');assert v['exact_sdk_provenance']
old_lock=tomllib.loads(subprocess.check_output(['git','show','a11bd4f4:Cargo.lock'],cwd=r,text=True));new_lock=tomllib.loads((r/'Cargo.lock').read_text());assert len(old_lock['package'])==len(new_lock['package']);lock_delta=[]
for old,new in zip(old_lock['package'],new_lock['package']):
 if old==new:continue
 assert old['name']==new['name']=='incant_render';assert {k:v for k,v in old.items() if k!='dependencies'}=={k:v for k,v in new.items() if k!='dependencies'};assert set(new['dependencies'])-set(old['dependencies'])=={'rustix'} and set(old['dependencies'])<=set(new['dependencies']);lock_delta.append({'package':'incant_render','added_edge':'rustix','selected_android_graph':False})
assert len(lock_delta)==1
changed=git('diff','--name-only','a11bd4f4',f['head'],'--','crates','tools','sdk','third_party','Cargo.toml','Cargo.lock','.cargo','rust-toolchain.toml').splitlines();exceptions={'Cargo.lock','crates/incant_script/src/restart_request_tests.rs','tools/check_workflow_jobs.py','tools/tests/test_workflow_jobs.py'};assert all(x in exceptions or x.startswith(('crates/incant_render/','crates/incant_render_gpu_host/tests/','crates/incant_shader_cook/')) for x in changed)
producer_path=r/'docs/spikes/evidence/authored-android-wasm/manifest.json';producer=read(producer_path);assert h(r/producer['archive']['path'])==producer['archive']['sha256'];pa=json.loads(gzip.decompress((r/producer['archive']['path']).read_bytes()))
for item in pa['records']:
 raw=base64.b64decode(pa['payloads'][item['sha256']]);assert sha(raw)==item['sha256'] and len(raw)==item['bytes'];assert h(item['path'])==item['sha256']
for item in producer['external_binaries']:assert h(item['path'])==item['sha256']
qualification={'current_source':f['head'],'producer_source':'a11bd4f4fe3b74402572e8c3bc0ca41b814f62df','selected_modes':['deterministic','fast'],'normal_and_build_graphs_equal':True,'selected_local_inputs_per_mode':len(rows[0]['source_rows']),'selected_source_exception':'Only cfg(test) restart quota test changed; shipping compilation excludes it.','lock_delta':lock_delta,'compiled_inputs_changed_outside_selected_graph':changed,'script_identity':'Cargo.lock is deliberately hashed conservatively by incant_script/build.rs. Current host preflight and Android build-time admission freshly cook matching identities; no old data is loaded into a new archive.','guest_identity':'SDK/ABI/guest source identical. Current and producer SDK rebuilds with original source-location mappings reproduce the exact old guest SHA; normal new guest/bundle embeds current source paths. Each current bundle is independently captured/audited and actually target-compiled.','compiler_identity':'All Rust/NDK tool hashes, target macro probes, wrappers and flags exact. Clang driver diagnostic differs only by checked checkout paths in debug/coverage compilation directories.','reused_execution':'Six original API37 Android processes and26 refusals remain exclusively a11 producer evidence. No new archive linking or Android execution is claimed.','fresh_execution':'Four current captured Wasmtime/native preflight processes, both-mode actual Android cargo check/admission, ten actual C source compilations, exact selected five-solver feature receipts.'}
summary={'passed':True,'current_commands':11,'target_commands':24,'cook_cases':sum(int(x[0]) for x in tests),'current_native_processes':4,'actual_android_target_checks':2,'producer_android_processes':6,'producer_refusals':26,'frozen_inputs':len(f['files']),'producer_records':len(pa['records'])};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);(o/'qualification.json').write_text(json.dumps(qualification,indent=2)+'\n');artifacts=[];external=[];payloads={}
roots=[o,*[base/f'authored-android-wasm-main189-target-{i:03}' for i in range(1,5)]]
for root in roots:
 for q in sorted(root.rglob('*')):
  if not q.is_file() or '__pycache__' in q.parts:continue
  if 'cook-target' in q.parts:
   if q.name=='incant_wasm_cook' and q.parent.name=='debug':external.append({'path':str(q),'sha256':h(q),'bytes':q.stat().st_size})
   continue
  if 'rust-sdk-libraries' in q.parts:continue
  if q.name in ('incant_headless','audio_package_oracle'):
   external.append({'path':str(q),'sha256':h(q),'bytes':q.stat().st_size});continue
  raw=q.read_bytes();digest=sha(raw);artifacts.append({'path':str(q),'sha256':digest,'bytes':len(raw)});payloads.setdefault(digest,base64.b64encode(raw).decode())
compressed=gzip.compress(json.dumps({'format':'incant-lossless-evidence','version':1,'records':artifacts,'payloads':payloads},separators=(',',':')).encode(),mtime=0);parts=[]
for i,pos in enumerate(range(0,len(compressed),16*1024*1024)):
 q=d/f'artifacts.json.gz.part-{i:03}';raw=compressed[pos:pos+16*1024*1024];q.write_bytes(raw);parts.append({'path':str(q.relative_to(r)),'sha256':sha(raw),'bytes':len(raw)})
for src,name in [(o/'freeze.json','freeze.json'),(o/'qualification.json','qualification.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(base/'authored-android-wasm-main189-checks.py','checks.py'),(base/'android-main189-target-checks.py','target-checks.py'),(base/'qualify-android-guest-paths.py','guest-producer-path-control.py'),(base/'qualify-android-current-guest-paths.py','guest-current-path-control.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.UTC).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD'),'base_main':'2e0b2fe4ab8d0b2a1e96723df90bd15db11dcf20','summary':summary,'qualification':qualification,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'checks':checks,'target_checks':rows,'archive':{'parts':parts,'sha256':sha(compressed),'bytes':len(compressed),'records':len(artifacts),'payloads':len(payloads)},'external_artifacts':external,'producer_evidence':{'path':str(producer_path.relative_to(r)),'sha256':h(producer_path),'review_path':'docs/spikes/evidence/authored-android-wasm/final-review.json','review_sha256':h(r/'docs/spikes/evidence/authored-android-wasm/final-review.json'),'records':len(pa['records']),'external_binaries':len(producer['external_binaries']),'execution':producer_execution},'review_state':'Current-main independent final review pending','limits':['Current target work is compile/build-time admission only, no new archive/caller execution. Original accepted six Android API37 CPU processes remain producer-scoped.','Three prior runner-only assertion/setup failures are retained: WABT root/bin mismatch, guest source-location hash difference and compiler diagnostic cwd hash difference. No production source changed or passing runtime suite rerun for them.','Both selected runtime/build dependency graphs and source closure audited; conservative script identity freshly recooked with current lock.','No physical-device, minimum-OS, GPU/audio-device, performance, signing or phase approval.']}
q=r/'docs/spikes/evidence/authored-android-wasm-main189.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q),len(compressed),flush=True)
