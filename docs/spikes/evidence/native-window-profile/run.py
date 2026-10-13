from pathlib import Path
import subprocess,os,signal,json,hashlib,time,datetime,fcntl
R=Path('/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant');E=Path(__file__).parent
T=Path.home()/'.codex/incant-gpu-timestamps-target'
commands=[('default-player',['tools/cargo','test','--locked','--offline','--profile','ci','-j2','-p','incant_game_player']),('window-profile',['tools/cargo','test','--locked','--offline','--profile','ci','-j2','-p','incant_game_player','--lib','--features','native-window,profile-tracy']),('profile-cli',['tools/cargo','test','--locked','--offline','--profile','ci','-j2','-p','incant_game_player','--test','profile','--features','profile']),('summary',['tools/cargo','test','--locked','--offline','--profile','ci','-j2','-p','incant_profile','--features','summary']),('clippy',['tools/cargo','clippy','--locked','--offline','--profile','ci','-j2','-p','incant_game_player','-p','incant_profile','--all-targets','--features','incant_game_player/native-window,incant_game_player/profile-tracy','--','-D','warnings']),('web-profile',['tools/cargo','check','--locked','--offline','--profile','ci','-j2','-p','incant_profile','--features','summary','--target','wasm32-unknown-unknown']),('runtime-graph',['python3','tools/check_runtime_dependencies.py','incant_game_player','incant_profile']),('fmt',['tools/cargo','fmt','-p','incant_game_player','-p','incant_profile','--check']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check'])]
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
paths=subprocess.check_output(['git','ls-files','crates/incant_game_player','crates/incant_profile','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=R,text=True).splitlines()
f={p:sha(R/p) for p in paths};(E/'freeze.json').write_text(json.dumps({'files':f,'base':subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip()},indent=2)+'\n')
env={'HOME':str(Path.home()),'PATH':'/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin','TMPDIR':'/tmp','CARGO_TARGET_DIR':str(T),'CARGO_TERM_COLOR':'never','RUSTUP_AUTO_INSTALL':'0'}
rows=[]
with (T.parent/(T.name+'.lock')).open('a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
 for name,cmd in commands:
  assert all(sha(R/p)==s for p,s in f.items())
  now=datetime.datetime.now(datetime.timezone.utc).isoformat();t=time.monotonic();child=None
  with (E/(name+'.log')).open('xb') as log:
   try:
    child=subprocess.Popen(cmd,cwd=R,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    child.wait(timeout=1800)
   finally:
    if child:
     try:os.killpg(child.pid,signal.SIGKILL)
     except ProcessLookupError:pass
     child.wait()
     log.flush()
     row={'name':name,'argv':cmd,'start':now,'seconds':time.monotonic()-t,'pid':child.pid,'exit':child.returncode,'log':str(E/(name+'.log')),'sha256':sha(E/(name+'.log')),'reaped':True};rows.append(row);(E/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps(row),flush=True)
  if child.returncode:break
 assert all(sha(R/p)==s for p,s in f.items())
assert len(rows)==len(commands) and all(x['exit']==0 for x in rows)
