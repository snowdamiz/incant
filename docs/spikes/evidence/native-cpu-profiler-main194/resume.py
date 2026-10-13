from pathlib import Path
import os,sys,json,subprocess,hashlib,time
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/native-cpu-profiler-integration-001';assert o.is_dir()
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD')
f=json.loads((o/'freeze.json').read_text());assert head==f['head'];freeze=f['files'];assert all(h(r/p)==v for p,v in freeze.items())
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH'];e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:'+e['PATH'];e['CARGO_TARGET_DIR']=subprocess.check_output(['python3','tools/local_cargo_target.py','--root',str(Path.home()/'.codex/incant-integration-targets')],cwd=r,text=True).strip();rows=[x for x in json.loads((o/'initial-results.json').read_text()) if x['exit']==0]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
run('dependency-guard-complete',['python3','tools/check_runtime_dependencies.py','incant_profile','incant_runtime','incant_game_runtime','incant_game_player','--features','incant_game_player/profile-tracy'])
run('diff',['git','diff','--check','origin/main'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'unchanged':True,'commands':len(rows)},indent=2)+'\n')
