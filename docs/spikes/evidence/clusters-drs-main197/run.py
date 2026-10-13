from pathlib import Path
import hashlib,json,os,subprocess,time
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/clusters-drs-main196-001'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');assert not (o/'freeze.json').exists();head=git('rev-parse','HEAD')
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();f={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'files':f},indent=2)+'\n')
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+e['PATH'];e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip()
cargo=['./tools/cargo'];args=['--locked','--profile','ci','-j2'];pkgs=['-p','incant_render','-p','incant_render_gpu_host'];features=','.join('incant_render/'+x for x in ['cooked-output','cooked-geometry','cooked-sprites','cooked-ui','cooked-text','cooked-materials','cooked-clusters','persistent-pipeline-cache'])
commands=[('shader-cook',cargo+['test']+args+['-p','incant_shader_cook']),('default-cpu',cargo+['test']+args+pkgs+['--all-targets']),('cooked-cpu',cargo+['test']+args+pkgs+['--all-targets','--features',features]),('cluster-only',cargo+['test']+args+['-p','incant_render','--lib','--features','cooked-clusters']),('clippy',cargo+['clippy']+args+pkgs+['-p','incant_shader_cook','--all-targets','--features',features,'--','-D','warnings']),('native-window-compile',cargo+['check']+args+['-p','incant_game_player','--all-targets','--features','native-window,'+features]),('web-default',cargo+['check']+args+['-p','incant_render','--lib','--target','wasm32-unknown-unknown']),('web-cooked',cargo+['check']+args+['-p','incant_render','--lib','--target','wasm32-unknown-unknown','--features',features]),('runtime-graph',['python3','tools/check_runtime_dependencies.py','incant_render','incant_render_gpu_host','--features',features]),('release-policy',['python3','-m','unittest','discover','-s','tools/tests','-p','test_release_policy.py']),('actionlint',[str(Path.home()/'.codex/incant-tools/actionlint-1.7.12/actionlint'),'.github/workflows/ci.yml','.github/workflows/desktop.yml']),('fmt',cargo+['fmt','--all','--','--check']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check','origin/main'])]
rows=[]
for name,cmd in commands:
 for path,s in f.items():assert h(r/path)==s,path
 start=time.monotonic();print('starting',name,flush=True)
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for key in ['stdout','stderr']:row[key]={'path':str(o/(name+'.'+key)),'sha256':h(o/(name+'.'+key))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True)
 for path,s in f.items():assert h(r/path)==s,path
 if rc:raise SystemExit(rc)
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'commands':len(rows),'all_frozen_inputs_unchanged':True,'gpu_execution':False},indent=2)+'\n');print('terminal0',flush=True)
