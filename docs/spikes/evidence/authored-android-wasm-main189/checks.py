from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/authored-android-wasm-main189-001';o.mkdir(exist_ok=False)
td=Path.home()/'.codex/incant-planar-integration-target';h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1');e['PATH']=str(Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin')+':/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
rows=[]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());cmd=list(map(str,cmd));print('starting '+name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as stdout,(o/(name+'.stderr')).open('wb') as stderr:rc=subprocess.run(cmd,cwd=r,env=e,stdout=stdout,stderr=stderr).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ('stdout','stderr'):row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name+' '+str(rc),flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
run('tools',['npm','run','test:tools'])
run('cook-tests',['./tools/cargo','test','--locked','--profile','ci','-j2','-p','incant_cook','--features','typescript','--all-targets'])
run('cook-clippy',['./tools/cargo','clippy','--locked','--profile','ci','-j2','-p','incant_cook','--features','typescript','--all-targets','--','-D','warnings'])
run('runtime-graph',['python3','tools/check_runtime_dependencies.py','incant_game_player','--features','incant_game_player/static-module'])
run('fmt',['./tools/cargo','fmt','--all','--','--check']);run('conventions',['python3','tools/generate_conventions.py','--check']);run('diff',['git','diff','--check','origin/main'])
for mode in ('deterministic','fast'):
 features='incant_headless/wasmtime-host,incant_cook/typescript'+(',incant_headless/physics-deterministic,incant_cook/physics-deterministic' if mode=='deterministic' else '')
 run(mode+'-host-build',['./tools/cargo','build','--locked','--profile','ci','-j2','--no-default-features','-p','incant_headless','-p','incant_cook','--bin','incant_headless','--example','audio_package_oracle','--features',features])
 b=o/(mode+'-binaries');b.mkdir();shutil.copy2(td/'ci/incant_headless',b/'incant_headless');shutil.copy2(td/'ci/examples/audio_package_oracle',b/'audio_package_oracle');(b/'identity.json').write_text(json.dumps({p.name:h(p) for p in b.iterdir() if p.is_file()},indent=2)+'\n')
 run(mode+'-preflight',['python3','tools/probes/authored-android-wasm/prepare.py',o/(mode+'-prepared'),'--binary',b/'incant_headless','--play-binary',b/'incant_headless','--oracle',b/'audio_package_oracle','--physics-mode',mode])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'files':len(freeze),'unchanged':True,'commands':len(rows),'scope':'Current composed CPU/compiler and public native preflight; no current Android target archive/caller execution yet'},indent=2)+'\n');print('all terminal',flush=True)
