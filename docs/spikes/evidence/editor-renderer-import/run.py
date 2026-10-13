from pathlib import Path
import os,sys,json,subprocess,hashlib,time
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/editor-renderer-import-main192-001';o.mkdir(exist_ok=False)
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert git('diff','--name-only')=='editor/app/tests/model_gpu.rs';head=git('rev-parse','HEAD')
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'files':freeze},indent=2)+'\n')
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH'];e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:'+e['PATH'];e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip();rows=[]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
run('editor-cpu',['cargo','test','--locked','--profile','ci','-j','2','-p','incant_editor','--test','model_gpu','--','--nocapture'])
run('renderer-backend-contract',['cargo','test','--locked','--profile','ci','-j','2','-p','incant_render','--lib','required_backend_rejects_unknown_and_mismatched_adapters','--','--nocapture'])
run('editor-clippy',['cargo','clippy','--locked','--profile','ci','-j','2','-p','incant_editor','--test','model_gpu','--','-D','warnings'])
run('fmt',['cargo','fmt','--all','--','--check'])
run('conventions',['python3','tools/generate_conventions.py','--check'])
run('diff',['git','diff','--check'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'unchanged':True,'commands':len(rows)},indent=2)+'\n')
