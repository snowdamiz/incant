from pathlib import Path
import os,subprocess,json,time,hashlib,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/authored-ios-main176-001');o.mkdir(exist_ok=False)
target=Path('/Users/andreyyurlov/.codex/incant-integration-targets/incant-dbf85c6c752803e4ffcf17b4ee9153f7e76e8c842fb98fb36cefafc89bf3e538');h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head=='de3ed2c9d4f5178310b45f968de25ccc2621e198';assert json.loads((target/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(target),PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_')):del e[k]
results=[]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();log=o/(name+'.log')
 with log.open('w') as f:rc=subprocess.run(cmd,cwd=r,env=e,stdout=f,stderr=subprocess.STDOUT).returncode
 rec={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'log':str(log),'sha256':h(log)};results.append(rec);(o/'results.json').write_text(json.dumps(results,indent=2)+'\n');print(name,rc,flush=True)
 assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
 return rec
# Remove only this checkout's old local package products; immutable prior archives stay outside target.
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--no-deps'],cwd=r,env=e));packages=[p['name'] for p in metadata['packages'] if p['id'] in metadata['workspace_members']];run('clean-local-packages',['cargo','clean','--target-dir',str(target)]+sum((['-p',p] for p in packages),[]))
c=['--locked','--profile','ci','-j2']
for name,cmd in [
 ('script',['./tools/cargo','test']+c+['-p','incant_script']),
 ('cook',['./tools/cargo','test']+c+['-p','incant_cook','--features','typescript','--test','scripts','--test','typescript','--test','project_scripts']),
 ('player',['./tools/cargo','test']+c+['-p','incant_game_player','--features','mobile-ffi,render-input']),
 ('clippy',['./tools/cargo','clippy']+c+['-p','incant_script','-p','incant_cook','-p','incant_game_player','--features','incant_cook/typescript,incant_game_player/mobile-ffi','--all-targets','--','-D','warnings']),
 ('graph',['python3','tools/check_runtime_dependencies.py','incant_script','incant_game_player','--features','incant_game_player/mobile-ffi']),
 ('physics-features',['python3','tools/physics_features.py','manifest']),
 ('ios-tests',['python3','-B','-m','unittest','discover','-s','tools/tests','-p','test_ios_export.py']),
 ('fmt',['./tools/cargo','fmt','--all','--','--check']),
 ('conventions',['python3','tools/generate_conventions.py','--check']),
 ('diff',['git','diff','--check']),
 ('headless',['./tools/cargo','build']+c+['-p','incant_headless'])]:run(name,cmd)
b=target/'ci/incant_headless';shutil.copy2(b,o/'incant_headless');(o/'incant_headless').chmod(0o555);(o/'headless.json').write_text(json.dumps({'source':head,'profile':'ci','sha256':h(o/'incant_headless'),'bytes':b.stat().st_size},indent=2)+'\n')
run('prepare',['python3','tools/probes/authored-ios-project/prepare.py',str(o/'prepared'),'--binary',str(o/'incant_headless')])
for name,triple in [('simulator','aarch64-apple-ios-sim'),('device','aarch64-apple-ios')]:
 run('export-'+name,['python3','tools/export_ios.py',str(o/'prepared/authoring/game.json'),str(o/name),'--target',triple,'--evidence',str(o/(name+'-build-evidence'))])
for name in ['simulator','device']:
 cmd=['python3','tools/probes/authored-ios-project/verify.py','--prepared',str(o/'prepared'),'--export',str(o/name),'--export-sha256',h(o/name/'export.json'),'--output',str(o/(name+'-callers'))]
 if name=='simulator':cmd+=['--remove-authoring']
 run('callers-'+name,cmd)
(o/'terminal.json').write_text(json.dumps({'head':head,'exit':0,'unchanged':True,'frozen_inputs':len(freeze),'commands':len(results),'limits':['CPU archive and simulator callers only; no physical device execution, GPU, window, signing or performance result.','Unsigned device C/Swift link-only.']},indent=2)+'\n');print('all terminal',flush=True)
