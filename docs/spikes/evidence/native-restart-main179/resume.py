from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/native-restart-main178-001';assert o.exists();td=Path.home()/'.codex/incant-planar-integration-target'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');assert git('rev-parse','HEAD')=='7f6e41c4a1a4d799dd115944f26cee51676fd36c'
assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
freeze_record=json.loads((o/'freeze.json').read_text());assert freeze_record['head']==git('rev-parse','HEAD');freeze=freeze_record['files'];assert all(h(r/p)==s for p,s in freeze.items())
e=dict(os.environ,CARGO_TARGET_DIR=str(td));e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
rows=json.loads((o/'results.json').read_text());assert rows[-1]['name']=='static-check' and rows[-1]['exit']==101
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.log')).open('wb') as out:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=subprocess.STDOUT).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'log':str(o/(name+'.log')),'sha256':h(o/(name+'.log'))};rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True)
 assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
def cargo(name,sub,args):run(name,['./tools/cargo',sub,'--locked','--profile','ci','-j2',*args])

cargo('static-fixture-cook','build',['-p','incant_wasm_cook'])
fixture=o/'static-fixture';fixture.mkdir();wabt=Path.home()/'.codex/incant-evidence/native-wasm-compound-conformance-007/inputs/wabt'
run('static-wabt-version',[str(wabt/'bin/wasm2c'),'--version'])
assert (o/'static-wabt-version.log').read_text().strip()=='1.0.42'
run('static-rust-guest',['python3','tools/rust_guest.py','crates/incant_wasm_dev/fixtures/bulk.rs',str(fixture/'bulk.wasm')])
run('static-fixture-prepare',['python3','tools/wasm2c.py',str(fixture/'bundle'),'--module','bulk='+str(fixture/'bulk.wasm'),'--wabt',str(wabt),'--cook',str(td/'ci/incant_wasm_cook')])
(o/'static-fixture-provenance.json').write_text(json.dumps({'reason':'Initial static compile lacked required trusted bundle environment; unchanged source, fresh trusted fixture for compile-only boundary.','wabt':{'path':str(wabt/'bin/wasm2c'),'sha256':h(wabt/'bin/wasm2c'),'version':'1.0.42'},'cook':{'path':str(td/'ci/incant_wasm_cook'),'sha256':h(td/'ci/incant_wasm_cook')},'source':{'path':'crates/incant_wasm_dev/fixtures/bulk.rs','sha256':h(r/'crates/incant_wasm_dev/fixtures/bulk.rs')},'bundle':{str(p.relative_to(fixture)):h(p) for p in fixture.rglob('*') if p.is_file()}},indent=2)+'\n')
e['INCANT_WASM_BUNDLE']=str(fixture/'bundle')
cargo('static-check-retry','check',['-p','incant_game_player','--no-default-features','--features','static-module,render-input'])
e.pop('INCANT_WASM_BUNDLE')
binary=o/'incant_headless';assert h(binary)==json.loads((o/'binary.json').read_text())['sha256']
cargo('compiler-free-check','check',['-p','incant_script','--no-default-features'])
for project in ['samples/core','samples/planar','sdk/ts']:run('ts-'+project.replace('/','-'),['node','node_modules/typescript/bin/tsc','-p',project+'/tsconfig.json'])
run('graphs',['python3','tools/check_runtime_dependencies.py','incant_game_runtime','incant_game_player','incant_script','--features','incant_game_player/static-module,incant_game_player/render-input'])
run('features',['python3','tools/physics_features.py','manifest']);run('fmt',['./tools/cargo','fmt','--all','--','--check']);run('conventions',['python3','tools/generate_conventions.py','--check']);run('diff',['git','diff','--check'])
for sample in ['core','planar']:
 prior=json.loads((r/f'docs/spikes/evidence/native-session-restart/native-session-restart-public-002/{sample}/prepared.json').read_text())
 run(sample+'-prepare',['python3','tools/probes/native-session-restart.py','prepare',sample,prior['original_authoring'],str(o/(sample+'-development')),'--binary',str(binary)])
 run(sample+'-ship',['python3','tools/probes/native-session-restart.py','ship',str(o/(sample+'-development')),str(o/(sample+'-shipping'))])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':git('rev-parse','HEAD'),'frozen_files':len(freeze),'unchanged':True,'commands':len(rows)},indent=2)+'\n');print('all terminal',flush=True)
