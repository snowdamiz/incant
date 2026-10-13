from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,shutil,subprocess,sys,tomllib
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/parry-main194-001'
d=r/'docs/spikes/evidence/parry-main195'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='f5e2c04c3aeefb3f1ebfa49c217d100bf53f9d8a';main='da6db80dcba750649fb7323f80680dc9417cd884'
f=read(o/'continuation-freeze.json');old=read(o/'freeze.json');t=read(o/'terminal.json')
assert t['exit']==0 and t['head']==f['head'] and t['commands']==19 and t['source_unchanged_after_controls']
for p,s in f['files'].items():assert h(r/p)==s,p
assert set(f['files'])==set(old['files'])
assert {p for p in f['files'] if f['files'][p]!=old['files'][p]}=={'.gitattributes','tools/probes/native-physics/Cargo.lock','tools/tests/test_credential_impact.py'}
for row in f['qualified_changes']:assert old['files'][row['path']]==row['old'] and f['files'][row['path']]==row['new']
assert git('rev-parse',f['head']+'^{tree}')==git('rev-parse','aec0d1640688364a6e5401d7c533c8b36b19812f^{tree}')
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
checks=read(o/'results.json');assert len(checks)==19 and all(x['exit']==0 for x in checks)
initial=read(o/'initial-results.json');assert initial[-1]['name']=='diff' and initial[-1]['exit']==2 and checks[:7]==initial[:7]
for row in checks+initial:
 for key in ['stdout','stderr']:assert h(row[key]['path'])==row[key]['sha256']
counts={}
for name in ['default-physics','fast-physics']:
 row=next(x for x in checks if x['name']==name);text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text()
 matches=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 counts[name]={'passed':sum(int(x[0]) for x in matches),'failed':sum(int(x[1]) for x in matches),'ignored':sum(int(x[2]) for x in matches),'groups':len(matches)}
 assert counts[name]=={'passed':18,'failed':0,'ignored':0,'groups':6}
p=read(r/'docs/spikes/evidence/parry-parallel-ownership.json');sources=[];changed=[]
for row in p['sources']+p['probe_sources']:
 b=git('show',producer+':'+row['path']);assert sha(b)==row['sha256']
 current=h(r/row['path']);sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':current})
 if current!=row['sha256']:changed.append(row['path'])
assert set(changed)=={'Cargo.toml','Cargo.lock','crates/incant_script/build.rs','tools/probes/native-physics/Cargo.lock'}
assert (r/'crates/incant_script/build.rs').read_bytes()==git('show',producer+':crates/incant_script/build.rs').replace(b'        "incant_runtime",\n',b'        "incant_runtime",\n        "incant_profile",\n')
for path in ['Cargo.lock','tools/probes/native-physics/Cargo.lock']:
 before=tomllib.loads(git('show',main+':'+path).decode());after=tomllib.loads((r/path).read_text())
 for pkg in before['package']:
  if pkg['name'] in ['parry2d','parry3d']:assert pkg['version']=='0.31.1';del pkg['source'];del pkg['checksum']
 assert before==after,path
before=tomllib.loads(git('show',main+':Cargo.toml').decode());after=tomllib.loads((r/'Cargo.toml').read_text())
before['workspace']['exclude']+=['third_party/parry2d','third_party/parry3d']
for name in ['parry2d','parry3d']:before['patch']['crates-io'][name]={'path':'third_party/'+name}
assert before==after
for row in p['artifacts']:
 raw=Path(row['original']).read_bytes();stored=(r/row['retained']['path']).read_bytes()
 assert sha(raw)==row['sha256'] and len(raw)==row['bytes']
 assert sha(stored)==row['retained']['sha256'] and len(stored)==row['retained']['bytes'] and gzip.decompress(stored)==raw
assert h(r/p['final_review']['path'])==p['final_review']['sha256']=='e2b3879789c86bac796d6d5f9ee10bdd091f43045852c4b6ac456ac4d12504aa'
metadata=[]
for row in checks:
 if not row['name'].endswith('-metadata'):continue
 m=read(row['stdout']['path']);local=[]
 for package in m['packages']:
  if package['name'] in ['parry2d','parry3d']:
   assert package['version']=='0.31.1' and package['source'] is None
   assert Path(package['manifest_path'])==r/'third_party'/package['name']/'Cargo.toml'
   local.append(package['name'])
 assert local
 metadata.append({'name':row['name'],'patched_parry':local,'stdout_sha256':row['stdout']['sha256']})
assert len(metadata)==4
controls=read(o/'identity-controls.json');assert controls['passed'] and controls['all_inputs_restored']
assert len(controls['controls'])==2
for row in controls['controls']:
 assert row['baseline']['id']==row['restored']['id']!=row['mutated']['id']
 assert h(r/row['path'])==row['original_sha256']!=row['mutated_sha256']
 for key in ['baseline','mutated','restored']:
  v=row[key];raw=Path(v['retained']).read_bytes();assert sha(raw)==v['sha256']
  assert b'cargo:rustc-env=INCANT_CHECKPOINT_RUNTIME_ID='+v['id'].encode() in raw
summary={'passed':True,'checks':19,'counts':counts,'frozen_inputs':len(f['files']),'producer_inputs':len(sources),'exact_producer_inputs':len(sources)-len(changed),'producer_artifacts_rehashed':len(p['artifacts']),'probe_metadata_checks':len(metadata),'identity_controls':2,'actual_main_ancestor':main,'new_shipping_or_gpu_execution':False}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
encoded=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);parts=[]
for i,offset in enumerate(range(0,len(encoded),16*1024*1024)):
 path=d/f'artifacts.json.gz.part-{i:03}';path.write_bytes(encoded[offset:offset+16*1024*1024]);parts.append({'path':str(path.relative_to(r)),'sha256':h(path),'bytes':path.stat().st_size})
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','initial-freeze.json'),(o/'continuation-freeze.json','freeze.json'),(o/'identity-controls.json','identity-controls.json'),(o.parent/'parry-main194-run.py','run.py'),(o.parent/'parry-main194-resume.py','resume.py')]:shutil.copyfile(src,d/name)
patch=git('diff',producer,'HEAD','--',*changed);(d/'integration-seams.patch').write_bytes(patch)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_initial_head':old['head'],'tested_final_head':f['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'main_ancestor':main,'producer':producer,'summary':summary,'sources':sources,'checks':checks,'initial_failure':initial[-1],'continuation_changes':f['qualified_changes'],'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'parts':parts,'gzip_sha256':sha(encoded),'gzip_bytes':len(encoded),'records':len(records),'payloads':len(payloads)},'metadata':metadata,'identity_controls':controls,'producer_review':p['final_review'],'limits':['18 CPU tests execute in each physics mode; repeated cases overlap and are not 36 distinct regressions.','Only compatibility identity composition adds production text beyond accepted producer: incant_profile is included in the conservative runtime source hash. Actual baseline/comment mutation/restore builds verify invalidation for profiler and Parry, with all bytes restored.','The first full-base diff failed on an archived patch whitespace line; exact-path metadata correction and already-reviewed main195 tooling/isolated-lock repair were composed before the remaining checks. Passing unchanged Rust checks were preserved.','Producer Miri experiments remain failed and qualified; this is no clean Miri gate, complete dependency soundness audit, shipping/performance/device claim, or physics-save admission.','Read-only clone reused only external build dependencies after local workspace products were removed. No graphics, native window, or shipping package was executed.'],'review_state':'Independent current-main acceptance pending'}
path=r/'docs/spikes/evidence/parry-main195.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(path,h(path),flush=True)
