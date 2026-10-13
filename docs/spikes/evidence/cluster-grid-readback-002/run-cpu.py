from pathlib import Path
import subprocess,os,json,hashlib,time,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-audio-host/incant');o=Path(__file__).parent
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();f=json.loads((o/'freeze.json').read_text());e=os.environ.copy();e['PATH']='/opt/homebrew/opt/rustup/bin:'+e['PATH'];e['CARGO_TARGET_DIR']='/tmp/incant-agent-font-target';e['CARGO_BUILD_JOBS']='2'
results=[]
commands=[]
for name,features in [('baseline',[]),('cooked',['--features','cooked-clusters'])]:
 commands.append((name+'-cpu',['./tools/cargo','test','--locked','--profile','ci','-j2','-p','incant_render',*features,'--lib','--message-format=json','lighting::gpu_tests']))
 commands.append((name+'-clippy',['./tools/cargo','clippy','--locked','--profile','ci','-j2','-p','incant_render',*features,'--all-targets','--','-D','warnings']))
commands += [('fmt',['/opt/homebrew/opt/rustup/bin/rustfmt','--edition','2024','--check','crates/incant_render/tests/lighting_gpu/extended.rs']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check'])]
try:
 for name,cmd in commands:
  assert all(h(r/p)==v for p,v in f['files'].items())
  start=time.monotonic()
  with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:
   proc=subprocess.Popen(cmd,cwd=r,env=e,stdout=out,stderr=err);code=proc.wait()
  row={'name':name,'argv':cmd,'exit':code,'seconds':time.monotonic()-start,'pid':proc.pid,'stdout':h(o/(name+'.stdout')),'stderr':h(o/(name+'.stderr'))}
  if code==0 and name.endswith('-cpu'):
   bins=[]
   for line in (o/(name+'.stdout')).read_text().splitlines():
    try:x=json.loads(line)
    except ValueError:continue
    if x.get('reason')=='compiler-artifact' and x.get('target',{}).get('name')=='incant_render' and x.get('profile',{}).get('test') and x.get('executable'):bins.append(x)
   assert len(bins)==1,bins
   x=bins[0];dest=o/(name.removesuffix('-cpu')+'-binary');shutil.copy2(x['executable'],dest);row['binary']={'path':str(dest),'sha256':h(dest),'features':x['features'],'original':x['executable']}
  results.append(row);(o/'results.json').write_text(json.dumps(results,indent=2)+'\n');print(name,code,round(row['seconds'],2),flush=True)
  assert code==0,name
 assert all(h(r/p)==v for p,v in f['files'].items())
 (o/'terminal.json').write_text(json.dumps({'exit':0,'checks':len(results)})+'\n')
except BaseException as ex:
 (o/'terminal.json').write_text(json.dumps({'exit':1,'checks':len(results),'error':repr(ex)})+'\n');raise
