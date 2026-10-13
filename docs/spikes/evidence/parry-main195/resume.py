from pathlib import Path
import hashlib,json,os,re,subprocess,time
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/parry-main194-001';assert o.is_dir()
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain')
head=git('rev-parse','HEAD')
old=json.loads((o/'freeze.json').read_text());allowed={'.gitattributes','tools/probes/native-physics/Cargo.lock','tools/tests/test_credential_impact.py'}
freeze={p:h(r/p) for p in old['files']};changed={p for p in freeze if freeze[p]!=old['files'][p]};assert changed==allowed
(o/'continuation-freeze.json').write_text(json.dumps({'head':head,'files':freeze,'qualified_changes':[{'path':p,'old':old['files'][p],'new':freeze[p]} for p in sorted(changed)]},indent=2)+'\n')
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+e['PATH']
e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip()
initial=json.loads((o/'results.json').read_text());assert initial[-1]['name']=='diff' and initial[-1]['exit']==2 and all(x['exit']==0 for x in initial[:-1]);(o/'initial-results.json').write_text(json.dumps(initial,indent=2)+'\n');rows=initial[:-1]
def frozen(exception=None):
 for p,s in freeze.items():
  if p!=exception:assert h(r/p)==s,p
def run(name,cmd,exception=None):
 frozen(exception);print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'source_exception':exception}
 for stream in ['stdout','stderr']:row[stream]={'path':str(o/(name+'.'+stream)),'sha256':h(o/(name+'.'+stream))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);frozen(exception)
 if rc:raise SystemExit(rc)
p=json.loads((r/'docs/spikes/evidence/parry-parallel-ownership.json').read_text())
run('diff-corrected',['git','diff','--check','origin/main'])
for row in p['additional_checks']:
 if row['name']=='probe-lock-tools':continue
 run(row['name'],row['command'])
for row in p['probe_checks']:run(row['name'],row['command'])
script=next(row['command'] for row in p['checks'] if row['name']=='script-check')
def identity(label):
 outputs=[]
 for path in Path(e['CARGO_TARGET_DIR']).glob('ci/build/incant_script-*/output'):
  text=path.read_text();m=re.search(r'cargo:rustc-env=INCANT_CHECKPOINT_RUNTIME_ID=([0-9a-f]{64})',text)
  if m:outputs.append((path,m.group(1)))
 assert len(outputs)==1,outputs
 path,value=outputs[0];copy=o/(label+'-build-output.txt');copy.write_bytes(path.read_bytes())
 return {'id':value,'original':str(path),'retained':str(copy),'sha256':h(copy)}
baseline=identity('baseline');controls=[]
for label,path in [('profiler','crates/incant_profile/src/lib.rs'),('parry','third_party/parry3d/src/lib.rs')]:
 source=r/path;before=source.read_bytes()
 try:
  source.write_bytes(before+b'\n// Bounded integration compatibility identity control.\n')
  changed=h(source);run('identity-'+label+'-changed',script,path);mutation=identity(label+'-changed')
  assert mutation['id']!=baseline['id']
 finally:source.write_bytes(before)
 run('identity-'+label+'-restored',script);restored=identity(label+'-restored');assert restored['id']==baseline['id']
 controls.append({'path':path,'original_sha256':hashlib.sha256(before).hexdigest(),'mutated_sha256':changed,'baseline':baseline,'mutated':mutation,'restored':restored})
(o/'identity-controls.json').write_text(json.dumps({'passed':True,'controls':controls,'actual_cargo_build_script_outputs':True,'all_inputs_restored':True},indent=2)+'\n')
frozen();(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'commands':len(rows),'source_unchanged_after_controls':True},indent=2)+'\n');print('terminal0',flush=True)
