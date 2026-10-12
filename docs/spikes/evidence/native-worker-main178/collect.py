from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,subprocess,sys,shutil
root=Path('/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant');base=Path.home()/'.codex/incant-integration-evidence';src=base/'native-worker-main178-002';dest=root/'docs/spikes/evidence/native-worker-main178'
def h(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text())
def git(*a):return subprocess.check_output(['git',*a],cwd=root).decode().strip()
freeze=read(src/'freeze.json');assert all(h(root/p)==s for p,s in freeze['files'].items());assert read(src/'terminal.json')['exit']==0
checks=read(src/'results.json');assert len(checks)==14
sys.path.insert(0,str(root/'tools'));from physics_features import validate_artifacts,compiler_messages
for c in checks:
 assert c['exit']==0
 for stream in ['stdout','stderr']:assert h(c[stream]['path'])==c[stream]['sha256']
 if 'compiler_features' in c:
  assert validate_artifacts(compiler_messages(c['stdout']['path']),c['name'].split('-')[0],target_dir=Path.home()/'.codex/incant-physics-foundation-integration-target',target='aarch64-apple-darwin',profile='ci')==c['compiler_features']
counts={}
for mode in ['deterministic','fast']:
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (src/(mode+'-test.stdout')).read_text());assert rows and all(int(f)==0 for _,f,_ in rows);counts[mode]={'passed':sum(int(p) for p,_,_ in rows),'ignored':sum(int(i) for _,_,i in rows),'groups':len(rows),'failed':0}
py=(src/'python.stderr').read_text();assert re.search(r'Ran \d+ tests',py) and '\nOK' in py
binaries=read(src/'binaries.json');assert all(h(p['path'])==p['sha256'] for p in binaries.values())
protocol=read(src/'protocol-proof/result.json');assert protocol['passed'] and len(protocol['cases'])==12;assert protocol['binary_sha256']=={m:p['sha256'] for m,p in binaries.items()}
commands=read(src/'protocol-proof/commands.json');assert len(commands)==8
for mode in ['deterministic','fast']:
 folder=src/'protocol-proof'/mode;assert not (folder/'authoring').exists()
 baseline=json.loads(next(c['stdout'] for c in commands if c['argv'][0]==binaries[mode]['path'] and c['argv'][1]=='native-play'));assert baseline['passed']
 for name,status in [('eof',0),('truncated',1),('oversized',1),('retained',0)]:
  process=read(folder/name/'process.json');assert process['reaped'] and process['exit_code']==status
  replies=[json.loads(x) for x in (folder/name/'stdout').read_text().splitlines()];assert replies[0]['event']=='ready' and replies[0]['physics_mode']==mode and replies[0]['state']['tick']==0
  if name=='retained':
   assert replies[-1]['result']=={'playing':False};state=read(folder/'state.json');assert state['tick']==12 and state['logs']==baseline['logs'] and state['frames']==baseline['frames']
   for eid,e in state['entities'].items():
    expected=baseline['observations']['entities'][eid];assert e['transform']=={k:expected[k] for k in ['translation','rotation','scale']} and e['world_transform']==expected['world_transform']
   assert [x['error']['code'] for x in replies if 'error' in x]==['version','invalid_request','invalid_request','invalid_request','native','sequence']
 for c in commands:
  if c['argv'][0]==binaries[mode]['path'] and c['exit_code']:
   assert c['exit_code']==1 and not c['stdout'] and any(t in c['stderr'] for t in ['project changed before worker startup','physics mode'])
owner=read(src/'owner-proof/result.json');assert owner['passed'] and owner['original_unchanged'];assert not (src/'owner-proof/authoring').exists()
for name,selection in owner['selections'].items():
 assert read(src/'owner-proof'/name/'selection.json')==selection
 assert selection['build_exit_code']==0 and selection['passed']==(name!='caller-failure')
 command=selection.get('worker_command',selection.get('play_command'));assert not Path(command[0]).exists()
 assert validate_artifacts(compiler_messages(src/'owner-proof'/name/'cargo.jsonl'),selection['physics_mode'],target_dir=Path.home()/'.codex/incant-physics-foundation-integration-target',target=selection['target'],profile='ci')==selection['physics']
for state in owner['states']:
 assert state['tick']==12 and state['logs']==owner['baseline']['logs'] and state['frames']==owner['baseline']['frames']
 for eid,e in state['entities'].items():
  expected=owner['baseline']['observations']['entities'][eid];assert e['transform']=={k:expected[k] for k in ['translation','rotation','scale']} and e['world_transform']==expected['world_transform']
source=[]
for name in ['native-play-worker','native-worker-owner']:
 m=read(root/f'docs/spikes/evidence/{name}.json')
 for p,s in m['source_sha256'].items():
  assert h(root/p)==s;source.append({'path':p,'sha256':s,'producer':name,'exact':True})
fixture_paths=['crates/incant_headless/tests/native_play.rs','crates/incant_headless/tests/native_rpc.rs','crates/incant_headless/tests/native_ui.rs']
assert read(base/'native-worker-fixture-mode-red/result.json')['exit']==101 and read(base/'native-worker-fixture-mode-green/result.json')['exit']==0
assert read(base/'native-worker-main177-001/results.json')[-1]['exit']==101
dest.mkdir(parents=True,exist_ok=True);reviews=[]
for name in ['native-play-worker-independent-20261012.json','native-worker-owner-independent-20261012.json','native-worker-fixture-mode-source-20261012.json']:
 p=Path.home()/'.codex/incant-reviews'/name;shutil.copyfile(p,dest/name);reviews.append({'path':str((dest/name).relative_to(root)),'sha256':h(p)})
paths=list(src.rglob('*'))+list((base/'native-worker-fixture-mode-red').rglob('*'))+list((base/'native-worker-fixture-mode-green').rglob('*'))
paths += [p for p in (base/'native-worker-main177-001').iterdir() if p.is_file() and not p.name.startswith('incant_headless')]
paths += [base/'native-worker-fixture-mode.patch',base/'native-worker-fixture-mode-review.json']
artifacts=[];payloads={}
for p in sorted(set(paths)):
 if not p.is_file() or str(p) in [x['path'] for x in binaries.values()]:continue
 raw=p.read_bytes();s=sha(raw);artifacts.append({'original':str(p),'sha256':s,'bytes':len(raw)})
 if s not in payloads:
  try:payloads[s]={'encoding':'utf8','data':raw.decode('utf8')}
  except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
a=dest/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'artifacts':artifacts,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
shutil.copyfile(src/'freeze.json',dest/'freeze.json');shutil.copyfile(base/'native-worker-main178-run.py',dest/'run.py');shutil.copyfile(Path(__file__),dest/'collect.py')
patch=subprocess.check_output(['git','diff','f1f8768a95f048320b1c20155cf513a81684ea2d',freeze['head'],'--',*fixture_paths],cwd=root);(dest/'fixture-mode.patch').write_bytes(patch)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':freeze['head'],'integration_head':git('rev-parse','HEAD'),'actual_main':'15f31d0375983ac3eac82086cd6a2379ad6adb46','source_freeze':{'path':str((dest/'freeze.json').relative_to(root)),'sha256':h(dest/'freeze.json'),'count':len(freeze['files'])},'source_comparison':source,'fixture_correction':{'paths':{p:h(root/p) for p in fixture_paths},'patch_sha256':sha(patch),'original_red':str(base/'native-worker-main177-001/results.json'),'additional_red':str(base/'native-worker-fixture-mode-red/result.json'),'targeted_green':str(base/'native-worker-fixture-mode-green/result.json'),'qualification':'Test-only public SetPhysicsMode in existing authoring transactions; explicit opposite-mode refusal. Targeted green preceded rustfmt-only assertion layout; both final full suites use the frozen formatted source.'},'checks':checks,'rust_counts':counts,'python_summary':re.findall(r'Ran \d+ tests[^\n]*|OK[^\n]*',py)[-2:],'node_summary':(src/'node.stdout').read_text().splitlines()[-9:],'binaries':binaries,'archive':{'path':str(a.relative_to(root)),'sha256':h(a),'artifacts':len(artifacts),'unique_payloads':len(payloads),'bytes':a.stat().st_size},'protocol':{'cases':protocol['cases'],'commands':len(commands),'owned_children':8,'source_deleted':True,'full_retained_state_matches_native':True},'owner':{'selections':list(owner['selections']),'retained_ticks':[s['tick'] for s in owner['states']],'source_deleted':True,'original_unchanged':True,'private_executables_removed':True,'full_selected_state_matches_native':True},'reviews':reviews,'limits':['Developer ci-profile CPU execution only; no shipping, GPU, editor viewport, native-window, physical-device or performance acceptance.','Windows process-tree branch compiles/tests through existing fixtures; actual Windows execution remains pending.','Movement fixtures have no active solver bodies; this is process/kernel selection and protocol evidence, not numerical backend equivalence or speed.','Worker drops authoring state after startup but startup compiles native TypeScript. PCM is not exposed by the worker protocol.','Request/read/write deadlines and encoded response cap do not establish bounded allocator use or an untrusted process sandbox.'],'review_state':'Independent final current-main execution review pending.'}
out=root/'docs/spikes/evidence/native-worker-main178.json';out.write_text(json.dumps(manifest,indent=2)+'\n');print(str(out),h(out));print(json.dumps({'counts':counts,'artifacts':len(artifacts),'sources':len(source),'freeze':len(freeze['files'])}))
