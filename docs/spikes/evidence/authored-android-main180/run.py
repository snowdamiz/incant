from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil,socket
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/authored-android-main179-003';o.mkdir(exist_ok=False)
td=Path.home()/'.codex/incant-integration-targets/incant-dbf85c6c752803e4ffcf17b4ee9153f7e76e8c842fb98fb36cefafc89bf3e538'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head=='79557b893a13e13ac5eacf7356b3a1591a12bde8';assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
sys.path.insert(0,str(r/'tools'));from physics_features import validate_artifacts,compiler_messages
prior=json.loads((o.parent/'authored-android-main179-002/results.json').read_text())[0];assert prior['name']=='script-tests' and prior['exit']==0
for stream in ['stdout','stderr']:assert h(prior[stream]['path'])==prior[stream]['sha256']
(o/'reused-script-tests.json').write_text(json.dumps({'check':prior,'basis':'Only cooker destination test changed; incant_script and all production sources are byte-identical to prior freeze.'},indent=2)+'\n')
rows=[]
def run(name,cmd,mode=None):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 if rc==0 and mode:row['compiler_features']=validate_artifacts(compiler_messages(o/(name+'.stdout')),mode,target_dir=td,target='aarch64-apple-darwin',profile='ci')
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
c=['--locked','--profile','ci','-j2']
for name,cmd in [
 ('cook-tests',['./tools/cargo','test',*c,'-p','incant_cook','--features','typescript','--test','mobile_project','--test','scripts','--test','typescript','--test','project_scripts']),
 ('candidate-cook-tests',['./tools/cargo','test',*c,'-p','incant_cook','--features','probe-android-bytecode','--test','android_probe']),
 ('candidate-script-tests',['./tools/cargo','test',*c,'-p','incant_script','--features','probe-android-bytecode','artifact::mobile_tests']),
 ('player-tests',['./tools/cargo','test',*c,'-p','incant_game_player','--features','mobile-ffi,render-input','--lib']),
 ('clippy',['./tools/cargo','clippy',*c,'-p','incant_script','-p','incant_cook','-p','incant_game_player','--features','incant_cook/typescript,incant_game_player/mobile-ffi','--all-targets','--','-D','warnings']),
 ('python',['python3','-B','-m','unittest','discover','-s','tools/tests','-v']),
 ('graphs',['python3','tools/check_runtime_dependencies.py','incant_script','incant_game_player','--features','incant_game_player/mobile-ffi']),
 ('physics-features',['python3','tools/physics_features.py','manifest']),
 ('fmt',['./tools/cargo','fmt','--all','--','--check']),
 ('conventions',['python3','tools/generate_conventions.py','--check']),
 ('diff',['git','diff','--check','origin/main'])]:run(name,cmd)
binaries={}
for mode in ['deterministic','fast']:
 flags=['--no-default-features']+(['--features','physics-deterministic'] if mode=='deterministic' else [])
 run(mode+'-headless',['./tools/cargo','build',*c,'--target','aarch64-apple-darwin','-p','incant_headless','--message-format=json',*flags],mode)
 b=o/('incant_headless-'+mode);shutil.copy2(td/'aarch64-apple-darwin/ci/incant_headless',b);b.chmod(0o555);binaries[mode]={'path':str(b),'sha256':h(b),'bytes':b.stat().st_size};(o/'binaries.json').write_text(json.dumps(binaries,indent=2)+'\n')
for mode in ['deterministic','fast']:
 run(mode+'-prepare',['python3','tools/probes/authored-android-project/prepare.py',o/(mode+'-prepared'),'--binary',binaries['deterministic']['path'],'--play-binary',binaries[mode]['path'],'--physics-mode',mode])
sdk=Path('/opt/homebrew/share/android-commandlinetools');ndk=sdk/'ndk/28.2.13676358';java=Path('/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home');assert (java/'bin/java').is_file()
for mode,port in [('deterministic',5568),('fast',5570)]:
 exp=o/(mode+'-export');run(mode+'-export',['python3','tools/export_android.py',o/(mode+'-prepared/authoring/game.json'),exp,'--target','aarch64-linux-android','--ndk',ndk,'--evidence',o/(mode+'-build-evidence')])
 for p in [port,port+1]:
  with socket.socket() as s:s.bind(('127.0.0.1',p))
 run(mode+'-callers',['python3','tools/probes/authored-android-project/verify.py','--prepared',o/(mode+'-prepared'),'--export',exp,'--export-sha256',h(exp/'export.json'),'--sdk',sdk,'--java',java,'--ndk',ndk,'--output',o/(mode+'-callers'),'--remove-authoring','--port',str(port)])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'frozen_files':len(freeze),'unchanged':True,'commands':len(rows)},indent=2)+'\n');print('all terminal',flush=True)
