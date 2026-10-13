from pathlib import Path
import hashlib,json,os,subprocess,time
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/physics-body-main205-001';o.mkdir(exist_ok=False)
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD')
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();f={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'files':f},indent=2)+'\n')
e={'HOME':str(Path.home()),'PATH':'/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin','PYTHONDONTWRITEBYTECODE':'1','CARGO_TERM_COLOR':'never','TMPDIR':'/tmp','RUSTUP_AUTO_INSTALL':'0'};e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip()
p=json.loads((r/'docs/spikes/evidence/physics-inert-body-state.json').read_text());rows=[]
for item in p['commands'][-8:]:
 name=item['label'];cmd=item['argv'];
 if name=='diff-001':cmd=['git','diff','--check','origin/main']
 for path,s in f.items():assert h(r/path)==s,path
 start=time.monotonic();print('starting',name,flush=True)
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for k in ['stdout','stderr']:row[k]={'path':str(o/(name+'.'+k)),'sha256':h(o/(name+'.'+k))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True)
 for path,s in f.items():assert h(r/path)==s,path
 if rc:raise SystemExit(rc)
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'commands':len(rows),'all_frozen_inputs_unchanged':True},indent=2)+'\n');print('terminal0',flush=True)
