from pathlib import Path
import base64,gzip,hashlib,importlib.util,json,re,shutil,subprocess,sys,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/physics-artifact-vendor-identity-001';d=r/'docs/spikes/evidence/physics-artifact-vendor-identity'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
main='0a040a40e9cc2ac357e50b413852861657a44468';freeze=read(o/'freeze.json');assert freeze['head']==main
for p,s in freeze['files'].items():assert h(r/p)==s,p
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
assert read(o/'terminal.json')=={'exit':0,'commands':6,'all_frozen_inputs_unchanged':True}
checks=read(o/'results.json');assert len(checks)==6 and all(x['exit']==0 for x in checks)
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
assert read(o/'red.json')['exit']==1 and read(o/'green.json')['exit']==0
for name in ['red','green']:assert h(o/(name+'.log'))==read(o/(name+'.json'))['sha256']
assert 'Ran 22 tests' in (o/'green.log').read_text() and (o/'green.log').read_text().rstrip().endswith('OK')
assert 'Ran 369 tests' in (o/'tools.stderr').read_text() and (o/'tools.stderr').read_text().rstrip().endswith('OK (skipped=4)')
assert '# tests 22' in (o/'tools.stdout').read_text() and '# pass 22' in (o/'tools.stdout').read_text() and '# fail 0' in (o/'tools.stdout').read_text()
assert h(o/'repair.patch')=='d413eaec8a0353ad73801fe366f573f158af724a7cc54005f78ce7931453f58e'
sys.path.insert(0,str(r/'tools'));import physics_features
for mode in ['deterministic','fast']:
 audit=read(o/('compiler-'+mode+'-audit.json'));messages=physics_features.compiler_messages(o/('compiler-'+mode+'.stdout'));got=physics_features.validate_artifacts(messages,mode,target_dir=Path(audit['target_output']).parents[1],target=audit['target'],profile=audit['profile']);assert got=={k:v for k,v in audit.items() if k!='compiler_artifacts'}
 assert set(got['features'])==set(physics_features.SOLVERS)
 actual=[m for m in messages if m.get('reason')=='compiler-artifact' and m.get('target',{}).get('name') in physics_features.SOLVERS];assert actual==audit['compiler_artifacts']
 for name in ['parry2d','parry3d']:
  rows=[a for a in actual if a['target']['name']==name];assert rows
  assert all(a['package_id']=='path+'+(r/'third_party'/name).resolve().as_uri()+'#0.31.1' and Path(a['manifest_path']).resolve()==r/'third_party'/name/'Cargo.toml' for a in rows)
replay=read(o/'baseline-actual-replay.json');assert h(o/'baseline-physics-features.py')==replay['baseline_sha256']==sha(git('show',main+':tools/physics_features.py'))
assert len(replay['results'])==2
spec=importlib.util.spec_from_file_location('physics_vendor_baseline',o/'baseline-physics-features.py');baseline=importlib.util.module_from_spec(spec);sys.modules[spec.name]=baseline;spec.loader.exec_module(baseline)
for row in replay['results']:
 assert row['baseline_refused'] and h(row['compiler_messages'])==row['sha256']
 audit=read(row['corrected_audit'])
 try:baseline.validate_artifacts(baseline.compiler_messages(row['compiler_messages']),row['mode'],target_dir=Path(audit['target_output']).parents[1],target=audit['target'],profile=audit['profile'])
 except ValueError as error:assert str(error)==row['error']
 else:raise AssertionError('baseline must refuse actual vendored Cargo IDs')
 assert 'parry2d: unexpected compiler package/version/library identity' in row['error'] and 'parry3d: unexpected compiler package/version/library identity' in row['error']
failure=read(o/'hosted-failure.json');assert h(failure['log'])==failure['sha256'];assert failure['candidate']=='2fab0688fce68f4933ba4f14a7af074a088bfeb6'
for p,s in failure['source_identities'].items():assert sha(git('show',failure['candidate']+':'+p))==s
assert not git('diff',main,'--','crates','Cargo.lock','Cargo.toml','third_party','.github')
paths=git('diff','--name-only',main,'--','tools').decode().splitlines();assert set(paths)=={'tools/physics_features.py','tools/tests/test_physics_features.py'}
summary={'passed':True,'current_checks':6,'focused_tests':22,'python_tests':369,'python_platform_skips':4,'node_tests':22,'actual_compiler_modes':['deterministic','fast'],'actual_solver_targets':5,'frozen_inputs':len(freeze['files']),'source_paths':paths,'no_rust_dependency_or_workflow_changes':True,'new_shipping_execution':False};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
raws=[p for p in sorted(o.rglob('*')) if p.is_file() and '__pycache__' not in p.parts]+[Path(failure['log'])]
for path in raws:
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o.parent/'physics-artifact-vendor-run.py','run.py')]:shutil.copyfile(src,d/name)
x={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'main':main,'summary':summary,'source_freeze':{'original':str(o/'freeze.json'),'sha256':h(o/'freeze.json')},'sources':{p:h(r/p) for p in paths},'checks':checks,'hosted_failure':failure,'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'records':len(records),'payloads':len(payloads),'bytes':len(z)},'limits':['No Rust, solver math, feature requirement, workflow or timeout changes.','Actual compiler target checks use ci and the two native physics libraries, not a fresh shipping package.','The historical failed candidate is immutable; repaired ordinary shipping export and all required release checks remain pending.'],'review_state':'Independent final source/evidence review pending'}
p=r/'docs/spikes/evidence/physics-artifact-vendor-identity.json';p.write_text(json.dumps(x,indent=2)+'\n');print(p,h(p),flush=True)
