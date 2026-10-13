from pathlib import Path
import os,sys,json,subprocess,hashlib,time
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/authored-ios-physics-main181-001';o.mkdir(exist_ok=False)
prepared=o.parent/'authored-ios-physics-preparation-main181';td=Path.home()/'.codex/incant-planar-integration-target'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head=='6b1724690f80b5fdd979dfc166b9dda0194958cc'
assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines()
freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1');rustbin=Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin'
e['PATH']=str(rustbin)+':/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
sys.path.insert(0,str(r/'tools'));import ios_archive,physics_export
from physics_features import validate_artifacts,compiler_messages
rows=[]
def run(name,cmd,env=None,mode=None):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=env or e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 if rc==0 and mode:row['compiler_features']=validate_artifacts(compiler_messages(o/(name+'.stdout')),mode,target_dir=td,target='aarch64-apple-ios',profile='ci')
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
run('ios-tools',['python3','-B','-m','unittest','discover','-s','tools/tests','-p','test_ios_export.py','-v'])
run('android-tools',['python3','-B','-m','unittest','discover','-s','tools/tests','-p','test_android_export.py','-v'])
run('diff',['git','diff','--check','origin/main'])
sdk=subprocess.check_output(['xcrun','--sdk','iphoneos','--show-sdk-path'],text=True).strip()
for mode in ['deterministic','fast']:
 project=prepared/(mode+'-prepared/authoring/game.json')
 env=ios_archive.environment(e,rustbin/'rustc',project,None);env['SDKROOT']=sdk
 features=','.join(physics_export.cargo_features(mode,ios_archive.CAPABILITIES))
 (o/(mode+'-target-environment.json')).write_text(json.dumps({'INCANT_MOBILE_PROJECT':str(project),'SDKROOT':sdk,'features':features},indent=2)+'\n')
 run(mode+'-device-check',['./tools/cargo','check','--locked','--profile','ci','-j2','--target','aarch64-apple-ios','-p','incant_game_player','--no-default-features','--features',features,'--message-format=json'],env=env,mode=mode)
for mode in ['deterministic','fast']:
 run(mode+'-simulator-export',['python3','tools/export_ios.py',prepared/(mode+'-prepared/authoring/game.json'),o/(mode+'-simulator-export'),'--target','aarch64-apple-ios-sim','--evidence',o/(mode+'-build-evidence')])
for mode in ['deterministic','fast']:
 exp=o/(mode+'-simulator-export');opposite='fast' if mode=='deterministic' else 'deterministic';foreign=o/(opposite+'-simulator-export')
 run(mode+'-simulator-callers',['python3','tools/probes/authored-ios-physics/verify.py','--prepared',prepared/(mode+'-prepared'),'--export',exp,'--export-sha256',h(exp/'export.json'),'--output',o/(mode+'-simulator-callers'),'--remove-authoring','--foreign-export',foreign,'--foreign-export-sha256',h(foreign/'export.json')])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'frozen_files':len(freeze),'unchanged':True,'commands':len(rows),'scope':'Fresh composed deterministic/fast simulator archives and callers; actual device target compile in both modes. Unsigned device shipping links remain producer749 evidence, not rebuilt current-main.'},indent=2)+'\n');print('all terminal',flush=True)
