from pathlib import Path
import hashlib,json,os,subprocess,time,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/physics-artifact-vendor-identity-001'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':git('rev-parse','HEAD'),'files':freeze},indent=2)+'\n')
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+e['PATH'];e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip();target='aarch64-apple-darwin';checks=[]
base=['./tools/cargo','check','--locked','--offline','--profile','ci','-j2','-p','incant_physics_runtime','-p','incant_physics_2d_runtime','--target',target,'--message-format=json']
commands=[('compiler-deterministic',base),('compiler-fast',base+['--no-default-features']),('tools',['npm','run','test:tools']),('fmt',['./tools/cargo','fmt','--all','--check']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check','origin/main'])]
sys.path.insert(0,str(r/'tools'));import physics_features
for name,cmd in commands:
 assert all(h(r/p)==s for p,s in freeze.items());print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'target':e['CARGO_TARGET_DIR']}
 for key in ['stdout','stderr']:row[key]={'path':str(o/(name+'.'+key)),'sha256':h(o/(name+'.'+key))}
 checks.append(row);(o/'results.json').write_text(json.dumps(checks,indent=2)+'\n');print(name,rc,flush=True)
 if rc:raise SystemExit(rc)
 if name.startswith('compiler-'):
  mode=name.removeprefix('compiler-');messages=physics_features.compiler_messages(o/(name+'.stdout'));receipt=physics_features.validate_artifacts(messages,mode,target_dir=e['CARGO_TARGET_DIR'],target=target,profile='ci');receipt['compiler_artifacts']=[m for m in messages if m.get('reason')=='compiler-artifact' and m.get('target',{}).get('name') in physics_features.SOLVERS];(o/(name+'-audit.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(name,'actual target audit PASS',flush=True)
 assert all(h(r/p)==s for p,s in freeze.items())
(o/'terminal.json').write_text(json.dumps({'exit':0,'commands':len(checks),'all_frozen_inputs_unchanged':True},indent=2)+'\n');print('terminal0',flush=True)
