from pathlib import Path
import os,sys,json,subprocess,hashlib,time,shutil,socket
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o=Path.home()/'.codex/incant-integration-evidence/authored-android-pcm-main181-001';o.mkdir(exist_ok=False)
td=Path.home()/'.codex/incant-integration-targets/incant-dbf85c6c752803e4ffcf17b4ee9153f7e76e8c842fb98fb36cefafc89bf3e538'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
assert not git('status','--porcelain');head=git('rev-parse','HEAD');assert head=='489168fc49a23a4f4779907cd85793971dfe3cca'
assert json.loads((td/'incant-checkout-owner.json').read_text())['checkout']==str(r)
paths=git('ls-files','crates','editor/app','tools','sdk','schemas','third_party','samples/core','samples/planar','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github').splitlines()
freeze={p:h(r/p) for p in paths if (r/p).is_file()};(o/'freeze.json').write_text(json.dumps({'head':head,'root':str(r),'files':freeze},indent=2)+'\n')
e=dict(os.environ,CARGO_TARGET_DIR=str(td),PYTHONDONTWRITEBYTECODE='1')
rustbin=Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin'
e['PATH']=str(rustbin)+':/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
sys.path.insert(0,str(r/'tools'));from physics_features import validate_artifacts,compiler_messages
import android_archive
rows=[]
def run(name,cmd,mode=None,env=None,target='aarch64-apple-darwin'):
 assert all(h(r/p)==v for p,v in freeze.items());print('starting',name,flush=True);start=time.monotonic();cmd=list(map(str,cmd))
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:rc=subprocess.run(cmd,cwd=r,env=env or e,stdout=out,stderr=err).returncode
 row={'name':name,'command':cmd,'exit':rc,'seconds':time.monotonic()-start}
 for s in ['stdout','stderr']:row[s]={'path':str(o/(name+'.'+s)),'sha256':h(o/(name+'.'+s))}
 if rc==0 and mode:row['compiler_features']=validate_artifacts(compiler_messages(o/(name+'.stdout')),mode,target_dir=td,target=target,profile='ci')
 rows.append(row);(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,rc,flush=True);assert all(h(r/p)==v for p,v in freeze.items())
 if rc:raise SystemExit(rc)
c=['--locked','--profile','ci','-j2'];binaries={}
for mode in ['deterministic','fast']:
 features='mobile-ffi,render-input'+(',physics-deterministic' if mode=='deterministic' else '')
 flags=['--no-default-features','--features',features]
 run(mode+'-player-tests',['./tools/cargo','test',*c,'-p','incant_game_player',*flags,'--lib'])
 run(mode+'-player-clippy',['./tools/cargo','clippy',*c,'-p','incant_game_player',*flags,'--all-targets','--','-D','warnings'])
 hf=['--no-default-features']+(['--features','physics-deterministic'] if mode=='deterministic' else [])
 run(mode+'-headless',['./tools/cargo','build',*c,'--target','aarch64-apple-darwin','-p','incant_headless','--message-format=json',*hf],mode)
 b=o/('incant_headless-'+mode);shutil.copy2(td/'aarch64-apple-darwin/ci/incant_headless',b);b.chmod(0o555);binaries[mode]={'path':str(b),'sha256':h(b),'bytes':b.stat().st_size};(o/'binaries.json').write_text(json.dumps(binaries,indent=2)+'\n')
run('cook-admission',['./tools/cargo','test',*c,'-p','incant_cook','--features','typescript','--test','mobile_project','--test','scripts'])
run('android-tools',['python3','-B','-m','unittest','discover','-s','tools/tests','-p','test_android*.py','-v'])
run('graphs',['python3','tools/check_runtime_dependencies.py','incant_game_player','--features','incant_game_player/mobile-ffi'])
run('physics-features',['python3','tools/physics_features.py','manifest'])
run('fmt',['./tools/cargo','fmt','--all','--','--check'])
run('conventions',['python3','tools/generate_conventions.py','--check'])
for mode in ['deterministic','fast']:
 run(mode+'-prepare',['python3','tools/probes/authored-android-project/prepare.py',o/(mode+'-prepared'),'--binary',binaries['deterministic']['path'],'--play-binary',binaries[mode]['path'],'--physics-mode',mode])
sdk=Path('/opt/homebrew/share/android-commandlinetools');ndk=sdk/'ndk/28.2.13676358';java=Path('/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home')
cross,selected=android_archive.environment(e,rustbin/'rustc',o/'fast-prepared/authoring/game.json',None,android_archive.toolchain(ndk))
(o/'fast-target-environment.json').write_text(json.dumps(selected,indent=2)+'\n')
run('fast-android-target',['./tools/cargo','check',*c,'--target','aarch64-linux-android','-p','incant_game_player','--no-default-features','--features','mobile-ffi,project,render-input','--message-format=json'],env=cross)
exp=o/'deterministic-export'
run('deterministic-export',['python3','tools/export_android.py',o/'deterministic-prepared/authoring/game.json',exp,'--target','aarch64-linux-android','--ndk',ndk,'--evidence',o/'deterministic-build-evidence'])
for p in [5568,5569]:
 with socket.socket() as s:s.bind(('127.0.0.1',p))
run('deterministic-callers',['python3','tools/probes/authored-android-project/verify.py','--prepared',o/'deterministic-prepared','--export',exp,'--export-sha256',h(exp/'export.json'),'--sdk',sdk,'--java',java,'--ndk',ndk,'--output',o/'deterministic-callers','--remove-authoring','--port','5568'])
(o/'terminal.json').write_text(json.dumps({'exit':0,'head':head,'frozen_files':len(freeze),'unchanged':True,'commands':len(rows),'scope':'Composed PCM deterministic archive and actual Android callers; fast target compile and native mobile tests. Full fast shipping execution remains the preserved pre-PCM proof.'},indent=2)+'\n');print('all terminal',flush=True)
