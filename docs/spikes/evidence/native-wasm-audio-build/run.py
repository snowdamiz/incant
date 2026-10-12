from pathlib import Path
import hashlib,json,os,subprocess,time
root=Path('/Users/andreyyurlov/.codex/worktrees/agent-budget-settings/incant')
out=Path(__file__).resolve().parent
freeze=json.loads((out/'engine-freeze.json').read_text())
env=dict(os.environ)
env['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin'
env['CARGO_TARGET_DIR']='/tmp/incant-agent-font-target'
commands=[
 ['headless','./tools/cargo','build','--locked','--profile','ci','-j2','-p','incant_headless','--features','wasmtime-host'],
 ['oracle','./tools/cargo','build','--locked','--profile','ci','-j2','-p','incant_cook','--example','audio_package_oracle'],
 ['oracle-clippy','./tools/cargo','clippy','--locked','--profile','ci','-j2','-p','incant_cook','--example','audio_package_oracle','--','-D','warnings'],
 ['sdk-wasm32','./tools/cargo','check','--locked','--profile','ci','-j2','-p','incant-module','--target','wasm32-unknown-unknown'],
 ['fmt','./tools/cargo','fmt','--all','--check'],
 ['diff','git','diff','--check'],
]
receipts=[]
for name,*argv in commands:
 assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in freeze['files'].items())
 start=time.monotonic()
 with (out/(name+'.log')).open('w') as log:
  result=subprocess.run(argv,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
 receipts.append({'name':name,'argv':argv,'exit_code':result.returncode,'seconds':time.monotonic()-start,'log':name+'.log','sha256':hashlib.sha256((out/(name+'.log')).read_bytes()).hexdigest()})
 (out/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n')
 print(name,result.returncode,flush=True)
 if result.returncode:raise SystemExit(result.returncode)
print('all commands passed',flush=True)
