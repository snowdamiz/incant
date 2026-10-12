from pathlib import Path
import json,hashlib,subprocess,os,time
out=Path(__file__).parent;freeze=json.loads((out/'freeze.json').read_text());root=Path(freeze['root']);env=os.environ.copy();env['CARGO_TARGET_DIR']=str(Path.home()/'.codex/incant-physics-variant-target');rows=[]
commands=[['./tools/cargo','test','--locked','--profile','ci','-j2','-p','incant_shader_cook','-p','incant_render','--all-targets','--features','incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,incant_render/cooked-ui'],['./tools/cargo','clippy','--locked','--profile','ci','-j2','-p','incant_shader_cook','-p','incant_render','--all-targets','--features','incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,incant_render/cooked-ui','--','-D','warnings'],['./tools/cargo','check','--locked','--profile','ci','-j2','--target','wasm32-unknown-unknown','-p','incant_render','--features','cooked-output,cooked-geometry,cooked-sprites,cooked-ui'],['./tools/cargo','test','--locked','--profile','ci','-j2','-p','incant_render','--lib'],['./tools/cargo','fmt','--all','--','--check'],['python3','tools/generate_conventions.py','--check'],['git','diff','--check']]
for i,cmd in enumerate(commands):
 assert all(hashlib.sha256((root/f['path']).read_bytes()).hexdigest()==f['sha256'] for f in freeze['sources'])
 log=out/f'check-{i:02d}.log';start=time.monotonic()
 with log.open('w') as stream:r=subprocess.run(cmd,cwd=root,env=env,stdout=stream,stderr=subprocess.STDOUT)
 rows.append({'command':cmd,'exit':r.returncode,'seconds':time.monotonic()-start,'log':str(log),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()});(out/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(i,r.returncode,flush=True)
 if r.returncode:raise SystemExit(r.returncode)
(out/'terminal.json').write_text(json.dumps({'exit':0,'source_unchanged':all(hashlib.sha256((root/f['path']).read_bytes()).hexdigest()==f['sha256'] for f in freeze['sources'])})+'\n')
