from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/native-restart-main178-001';o.mkdir(exist_ok=False);td=Path.home()/'.codex/incant-planar-integration-target'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');assert git('rev-parse','HEAD')=='7f6e41c4a1a4d799dd115944f26cee51676fd36c'
assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':git('rev-parse','HEAD'),'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td));e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
rows=[]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.log')).open('wb') as out:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=subprocess.STDOUT).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'log':str(o/(name+'.log')),'sha256':h(o/(name+'.log'))};rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True)
 assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
def cargo(name,sub,args):run(name,['./tools/cargo',sub,'--locked','--profile','ci','-j2',*args])
cargo('window-tests','test',['-p','incant_game_player','--features','native-window','--lib'])
cargo('runtime-tests','test',['-p','incant_script','-p','incant_game_runtime','-p','incant_render_game','-p','incant_observe','-p','incant_audio_runtime'])
cargo('mobile-tests','test',['-p','incant_game_player','--features','mobile-ffi','--lib'])
cargo('clippy','clippy',['-p','incant_script','-p','incant_game_runtime','-p','incant_game_player','-p','incant_render_game','-p','incant_observe','-p','incant_audio_runtime','--all-targets','--features','incant_game_player/native-window,incant_game_player/mobile-ffi','--','-D','warnings'])
cargo('headless','build',['-p','incant_headless'])
binary=o/'incant_headless';shutil.copy2(td/'ci/incant_headless',binary);(o/'binary.json').write_text(json.dumps({'path':str(binary),'sha256':h(binary),'bytes':binary.stat().st_size},indent=2)+'\n')
cargo('static-check','check',['-p','incant_game_player','--no-default-features','--features','static-module,render-input'])
cargo('compiler-free-check','check',['-p','incant_script','--no-default-features'])
for project in ['samples/core','samples/planar','sdk/ts']:run('ts-'+project.replace('/','-'),['node','node_modules/typescript/bin/tsc','-p',project+'/tsconfig.json'])
run('graphs',['python3','tools/check_runtime_dependencies.py','incant_game_runtime','incant_game_player','incant_script','--features','incant_game_player/static-module,incant_game_player/render-input'])
run('features',['python3','tools/physics_features.py','manifest']);run('fmt',['./tools/cargo','fmt','--all','--','--check']);run('conventions',['python3','tools/generate_conventions.py','--check']);run('diff',['git','diff','--check'])
for sample in ['core','planar']:
 prior=json.loads((r/f'docs/spikes/evidence/native-session-restart/native-session-restart-public-002/{sample}/prepared.json').read_text())
 run(sample+'-prepare',['python3','tools/probes/native-session-restart.py','prepare',sample,prior['original_authoring'],str(o/(sample+'-development')),'--binary',str(binary)])
 run(sample+'-ship',['python3','tools/probes/native-session-restart.py','ship',str(o/(sample+'-development')),str(o/(sample+'-shipping'))])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':git('rev-parse','HEAD'),'frozen_files':len(freeze),'unchanged':True,'commands':len(rows)},indent=2)+'\n');print('all terminal',flush=True)
