from pathlib import Path
import os,sys,json,subprocess,hashlib,time
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/core-live-hud-main190-001';o.mkdir(exist_ok=False)
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD')
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines();freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'files':freeze},indent=2)+'\n')
prior=Path.home()/'.codex/incant-integration-evidence/authored-android-wasm-main189-001/deterministic-binaries'
b={'path':str(prior/'incant_headless'),'sha256':json.loads((prior/'identity.json').read_text())['incant_headless']};assert h(b['path'])==b['sha256']
subprocess.run(['git','diff','--exit-code','04c816cf7250cb66ccb109b4a66c42cff517d3d6',head,'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml','tools'],cwd=r,check=True)
assert subprocess.run(['git','merge-base','--is-ancestor','f517265a568724a5aa09ff1abc939c9368d35afb',head],cwd=r).returncode==1
(o/'headless-relevance.json').write_text(json.dumps({'binary':b,'tested_head':'04c816cf7250cb66ccb109b4a66c42cff517d3d6','all_crates_manifests_lock_sdk_schemas_vendor_toolchain_tools_equal':True,'note':'Current immutable deterministic Wasmtime-capable headless reused; all implementation/build source exact. No new build. Original private f517 history is not an ancestor.'},indent=2)+'\n')
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH'];rows=[]
def run(name,cmd):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
run('units',['python3','-B','-m','unittest','discover','-s','samples/core','-p','test_*.py','-v'])
run('strict-types',['node','node_modules/typescript/bin/tsc','--noEmit','-p','samples/core/tsconfig.json'])
run('public-scene',['python3','-B','samples/core/scene.py',o/'scene','--binary',b['path'],'--art',Path.home()/'.codex/incant-evidence/0060/builds/f5-en-playing/project/art'])
run('authored-bindings',['python3','-B','samples/core/verify_bindings.py','prepare',o/'scene',o/'native','--binary',b['path']])
run('all-locale-font-coverage',['python3','-B',Path.home()/'.codex/incant-integration-evidence/core-live-hud-fonts.py',o])
run('conventions',['python3','tools/generate_conventions.py','--check'])
run('diff',['git','diff','--check','origin/main'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'unchanged':True,'commands':len(rows)},indent=2)+'\n')
