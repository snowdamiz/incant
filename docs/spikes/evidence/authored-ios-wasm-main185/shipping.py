from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/authored-ios-wasm-main185-001';assert o.is_dir()
td=Path.home()/'.codex/incant-planar-integration-target';h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head.startswith('4b277d57');assert json.loads((o/'terminal.json').read_text())['exit']==0;assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};assert json.loads((o/'freeze.json').read_text())=={'head':head,'root':str(r),'files':freeze}
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1');e['PATH']=str(Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin')+':/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
rows=json.loads((o/'results.json').read_text())
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());cmd=list(map(str,cmd));print('starting '+name,flush=True);start=time.monotonic()
 with (o/(name+'.stdout')).open('wb') as stdout,(o/(name+'.stderr')).open('wb') as stderr:rc=subprocess.run(cmd,cwd=r,env=e,stdout=stdout,stderr=stderr).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ('stdout','stderr'):row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name+' '+str(rc),flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)

trusted=o/'trusted-tools/wabt'
assert trusted.is_dir()
(o/'wabt-inputs.json').write_text(json.dumps({str(p.relative_to(trusted)):h(p) for p in trusted.rglob('*') if p.is_file()},indent=2)+'\n')
for mode in ('deterministic','fast'):
 run(mode+'-simulator-export',['python3','tools/export_ios.py',o/(mode+'-prepared/authoring/game.json'),o/(mode+'-simulator-export'),'--target','aarch64-apple-ios-sim','--camera','0000000000000000000000009C','--evidence',o/(mode+'-build-evidence'),'--wabt',trusted])
for mode in ('deterministic','fast'):
 exp=o/(mode+'-simulator-export');other='fast' if mode=='deterministic' else 'deterministic';foreign=o/(other+'-simulator-export')
 run(mode+'-simulator-callers',['python3','tools/probes/authored-ios-wasm/verify.py','--prepared',o/(mode+'-prepared'),'--export',exp,'--export-sha256',h(exp/'export.json'),'--output',o/(mode+'-simulator-callers'),'--remove-authoring','--foreign-export',foreign,'--foreign-export-sha256',h(foreign/'export.json')])
(o/'shipping-terminal.json').write_text(json.dumps({'exit':0,'head':head,'files':len(freeze),'unchanged':True,'commands':len(rows),'scope':'Two fresh composed static Wasm simulator archives and C/Swift caller suites; original unsigned device links remain scoped to a765 producer'},indent=2)+'\n');print('shipping terminal',flush=True)
