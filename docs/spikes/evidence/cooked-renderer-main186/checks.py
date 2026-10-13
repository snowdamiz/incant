from pathlib import Path
import subprocess,json,hashlib,os,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');out=Path.home()/'.codex/incant-integration-evidence/cooked-renderer-main186-001';freeze=json.loads((out/'freeze.json').read_text())
target=Path.home()/'.codex/incant-integration-targets/incant-dbf85c6c752803e4ffcf17b4ee9153f7e76e8c842fb98fb36cefafc89bf3e538'
assert json.loads((target/'incant-checkout-owner.json').read_text())['checkout']==str(r)
env=dict(os.environ,PATH='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+os.environ['PATH'],CARGO_TARGET_DIR=str(target));env.pop('INCANT_EXPECT_GPU_BACKEND',None)
features='incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,incant_render/cooked-ui,incant_render/cooked-text,incant_render/cooked-materials,incant_render/persistent-pipeline-cache'
pkgs=['-p','incant_render','-p','incant_render_gpu_host','-p','incant_shader_cook']
commands=[('default-cpu',['./tools/cargo','test','--profile','ci','--locked','-j2',*pkgs,'--all-targets']),('cooked-cpu',['./tools/cargo','test','--profile','ci','--locked','-j2',*pkgs,'--all-targets','--features',features]),('cooked-clippy',['./tools/cargo','clippy','--profile','ci','--locked','-j2',*pkgs,'--all-targets','--features',features,'--','-D','warnings']),('web',['./tools/cargo','check','-p','incant_render','--target','wasm32-unknown-unknown','--locked','-j2','--features',features]),('dependencies',['python3','tools/check_runtime_dependencies.py','incant_render','incant_render_gpu_host','--features',features]),('policy',['python3','-m','unittest','discover','-s','tools/tests','-p','test_release_policy.py','-v']),('lint',[str(Path.home()/'.codex/incant-tools/actionlint-1.7.12/actionlint'),'.github/workflows/ci.yml','.github/workflows/desktop.yml']),('fmt',['./tools/cargo','fmt','--all','--','--check']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check'])]
rows=[]
for name,cmd in commands:
 for p,h in freeze['files'].items():assert hashlib.sha256((r/p).read_bytes()).hexdigest()==h,p
 print('starting',name,flush=True)
 log=out/(name+'.log')
 with log.open('w') as stream:result=subprocess.run(cmd,cwd=r,env=env,stdout=stream,stderr=subprocess.STDOUT)
 rows.append({'name':name,'command':cmd,'exit':result.returncode,'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()});(out/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,result.returncode,flush=True)
 if result.returncode:raise SystemExit(result.returncode)
(out/'terminal.json').write_text(json.dumps({'exit':0,'at':datetime.datetime.now(datetime.UTC).isoformat(),'files':len(freeze['files']),'no_gpu_execution':True})+'\n')
