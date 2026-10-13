from pathlib import Path
import subprocess,hashlib,json,os,time
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/vulkan-cache-catalog-repair');h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
paths=['crates/incant_render/tests/persistent_cache.rs','crates/incant_render/src/ui/mod.rs','crates/incant_render/src/materials.rs','crates/incant_render/src/shadows/mod.rs','crates/incant_render/src/output.rs','crates/incant_render/src/lib.rs','Cargo.lock','crates/incant_render/Cargo.toml']
f={p:h(r/p) for p in paths};(o/'freeze.json').write_text(json.dumps(f,indent=2)+'\n');e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+e['PATH'];e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip()
features='cooked-output,cooked-geometry,cooked-sprites,cooked-ui,cooked-text,cooked-materials,cooked-clusters,persistent-pipeline-cache';cargo=['./tools/cargo'];args=['--locked','--offline','--profile','ci','-j2','-p','incant_render','--test','persistent_cache','--features',features]
commands=[('compile',cargo+['test']+args+['--no-run']),('clippy',cargo+['clippy']+args+['--','-D','warnings']),('fmt',cargo+['fmt','--all','--','--check']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check','origin/main'])]
rows=[]
for name,cmd in commands:
 print('starting',name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start,'stdout_sha256':h(o/(name+'.stdout')),'stderr_sha256':h(o/(name+'.stderr'))};rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True)
 assert all(h(r/p)==s for p,s in f.items())
 if rc:raise SystemExit(rc)
(o/'terminal.json').write_text(json.dumps({'exit':0,'commands':len(rows),'GPU_execution':False})+'\n')
