from pathlib import Path
import os,sys,json,subprocess,hashlib,time
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/native-pcm-main180-001';o.mkdir(exist_ok=False)
td=Path.home()/'.codex/incant-planar-integration-target';b=Path.home()/'.codex/incant-integration-evidence'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head==sys.argv[1];assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
rows=[]
def run(name,cmd,extra=None):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd));env=dict(e,**(extra or {}))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=env,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'extra_env':extra or {}}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
c=['--locked','--profile','ci','-j2']
run('player-tests',['./tools/cargo','test',*c,'-p','incant_game_player','--features','native-window,mobile-ffi','--lib'])
run('checkpoint-restart-tests',['./tools/cargo','test',*c,'-p','incant_game_runtime','--features','ui','--test','checkpoints','--test','restart'])
run('clippy',['./tools/cargo','clippy',*c,'-p','incant_game_player','--features','native-window,mobile-ffi','--all-targets','--','-D','warnings'])
fixture=b/'native-restart-main178-001/static-fixture';provenance=json.loads((b/'native-restart-main178-001/static-fixture-provenance.json').read_text());assert all(h(fixture/p)==s for p,s in provenance['bundle'].items());assert h(r/provenance['source']['path'])==provenance['source']['sha256'];(o/'reused-trusted-static-fixture.json').write_text(json.dumps({'provenance':provenance,'scope':'Compile-only static boundary; no module execution or new guest shipping claim.'},indent=2)+'\n')
run('static-check',['./tools/cargo','check',*c,'-p','incant_game_player','--no-default-features','--features','static-module,render-input'],{'INCANT_WASM_BUNDLE':str(fixture/'bundle')})
run('mobile-check',['./tools/cargo','check',*c,'-p','incant_game_player','--features','mobile-ffi'])
run('graphs',['python3','tools/check_runtime_dependencies.py','incant_game_runtime','incant_game_player','incant_script','--features','incant_game_player/render-input'])
run('physics-features',['python3','tools/physics_features.py','manifest'])
run('fmt',['./tools/cargo','fmt','--all','--','--check'])
run('conventions',['python3','tools/generate_conventions.py','--check'])
run('diff',['git','diff','--check'])
run('shipping-proof',['python3','tools/probes/native-streaming-pcm.py','--fixture',Path.home()/'.codex/incant-evidence/native-streaming-pcm-fixture-001','--old-package',Path.home()/'.codex/incant-evidence/0060/packages/f5-en-playing','--output',o/'shipping'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'frozen_files':len(freeze),'unchanged':True,'commands':len(rows)},indent=2)+'\n');print('all terminal',flush=True)
