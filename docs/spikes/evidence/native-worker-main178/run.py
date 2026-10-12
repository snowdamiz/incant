from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant');o=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/native-worker-main178-002');o.mkdir(exist_ok=False);td=Path('/Users/andreyyurlov/.codex/incant-physics-foundation-integration-target')
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');assert git('rev-parse','HEAD')=='cbfcbf29991c6c640006ad49aea67aedebc67f27';assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':git('rev-parse','HEAD'),'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td));e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_')):e.pop(k)
sys.path.insert(0,str(r/'tools'));from physics_features import validate_artifacts,compiler_messages
rows=[]
def run(name,cmd,mode=None):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 if rc==0 and mode:row['compiler_features']=validate_artifacts(compiler_messages(o/(name+'.stdout')),mode,target_dir=td,target='aarch64-apple-darwin',profile='ci')
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
binaries={}
for mode in ['deterministic','fast']:
 flags=['--no-default-features']+(['--features','physics-deterministic'] if mode=='deterministic' else [])
 for sub in ['test','clippy','build']:
  cmd=['./tools/cargo',sub,'--locked','--profile','ci','--target','aarch64-apple-darwin','-j2','-p','incant_headless','--message-format=json']+flags
  cmd+=['--all-targets'] if sub!='build' else ['--bin','incant_headless']
  if sub=='clippy':cmd+=['--','-D','warnings']
  run(mode+'-'+sub,cmd,mode)
 copied=o/('incant_headless-'+mode);shutil.copy2(td/'aarch64-apple-darwin/ci/incant_headless',copied);binaries[mode]={'path':str(copied),'sha256':h(copied),'bytes':copied.stat().st_size}
(o/'binaries.json').write_text(json.dumps(binaries,indent=2)+'\n')
run('python',['python3','-B','-m','unittest','discover','-s','tools/tests','-v'])
run('node',['node','--test','tools/tests/native_script.test.mjs'])
run('protocol-process',['python3','tools/probes/native-play-worker.py','--deterministic',binaries['deterministic']['path'],'--fast',binaries['fast']['path'],'--output',str(o/'protocol-proof')])
run('owner-process',['python3','tools/probes/native-worker-owner.py','--binary',binaries['deterministic']['path'],'--authoring','/Users/andreyyurlov/.codex/incant-evidence/physics-project-play-002/project','--target-dir',str(td),'--output',str(o/'owner-proof')])
run('features',['python3','tools/physics_features.py','manifest'])
run('fmt',['./tools/cargo','fmt','--all','--','--check'])
run('conventions',['python3','tools/generate_conventions.py','--check'])
run('diff',['git','diff','--check'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':git('rev-parse','HEAD'),'frozen_files':len(freeze),'unchanged':True,'commands':len(rows)},indent=2)+'\n');print('all terminal',flush=True)
