import os,subprocess,json,hashlib
from pathlib import Path
root=Path('/Users/andreyyurlov/.codex/worktrees/cooked-visual-assets/incant');out=Path(__file__).parent
env=dict(os.environ,PATH='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:'+os.environ.get('PATH',''),CARGO_TARGET_DIR='/tmp/incant-cpu-profiler-target',CARGO_BUILD_JOBS='2')
checks=[
 ('clippy',['clippy','--locked','-p','incant_profile','-p','incant_runtime','-p','incant_script','-p','incant_game_runtime','-p','incant_game_player','--features','incant_game_player/profile-tracy','--all-targets','--','-D','warnings']),
 ('default-player',['test','--locked','-p','incant_game_player','--lib']),
 ('wasm-default',['check','--locked','-p','incant_profile','-p','incant_runtime','--target','wasm32-unknown-unknown']),
 ('wasm-summary',['check','--locked','-p','incant_profile','--features','summary','--target','wasm32-unknown-unknown'])
]
rows=[]
for name,args in checks:
 path=out/(name+'-002.log');command=['tools/cargo']+args
 with path.open('wb') as f:result=subprocess.run(command,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 rows.append({'name':name,'command':command,'exit':result.returncode,'log':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()});(out/'broad-checks-002.json').write_text(json.dumps(rows,indent=2)+'\n')
 print(name,result.returncode,flush=True)
 if result.returncode:raise SystemExit(result.returncode)
