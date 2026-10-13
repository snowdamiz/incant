"""Two exact numerical GPU tests, only after root's explicit exclusive lease.
Never maps images or captures a screen. Each test emits 84 raw buffer digests.
"""
from pathlib import Path
import datetime,fcntl,hashlib,json,os,platform,re,signal,subprocess,time
R=Path('/Users/andreyyurlov/.codex/worktrees/native-audio-host/incant')
E=Path(__file__).parent
TEST='lighting::gpu_tests::extended::extended_cluster_readback_cameras_viewports_and_retained_motion'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
load=lambda p:json.loads(Path(p).read_text())
now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat()
def new(path,value):
 with Path(path).open('x') as f:json.dump(value,f,indent=2);f.write('\n')
def source():
 f=load(E/'freeze.json')
 assert all(sha(R/p)==v for p,v in f['files'].items())
 assert sha(R/'docs/spikes/evidence/cluster-grid-readback.json')=='8c057aeaf7dc40bd413ed8336244b5ab393eb7ee735ce963fbea0bccc042146f'
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip()=='4fffb383786971169caea949ededf3ac64f96d2e'
def alive(pid):
 try:os.kill(pid,0);return True
 except ProcessLookupError:return False

def group_alive(pid):
 try:os.killpg(pid,0);return True
 except ProcessLookupError:return False

def reap(p):
 try:os.killpg(p.pid,signal.SIGKILL)
 except ProcessLookupError:pass
 return p.wait()

def execute(name,b,rows,owned,timeout=300):
 out=E/(name+'-gpu.stdout');err=E/(name+'-gpu.stderr')
 argv=[b['path'],TEST,'--ignored','--exact','--nocapture','--test-threads=1']
 env={'PATH':'/usr/bin:/bin:/usr/sbin:/sbin','HOME':str(Path.home()),'TMPDIR':'/tmp','RUST_BACKTRACE':'1'}
 started=now();t=time.monotonic();timed_out=False;error=None;p=None
 with out.open('xb') as stdout,err.open('xb') as stderr:
  try:
   p=subprocess.Popen(argv,cwd=R,env=env,stdout=stdout,stderr=stderr,start_new_session=True)
   owned.append(p)
   try:p.wait(timeout=timeout)
   except subprocess.TimeoutExpired:timed_out=True
  except BaseException as failure:
   error=type(failure).__name__;raise
  finally:
   if p is not None:
    # Reap after *every* wait outcome, including interrupts and failed waits.
    code=reap(p)
    stdout.flush();stderr.flush()
    row={'name':name,'argv':argv,'binary_sha256':b['sha256'],'features':b['features'],'pid':p.pid,'start':started,'end':now(),'seconds':time.monotonic()-t,'exit':code,'timeout':timed_out,'wait_error':error,'stdout':{'path':str(out),'sha256':sha(out)},'stderr':{'path':str(err),'sha256':sha(err)},'environment':{'PATH':env['PATH'],'HOME':'set','TMPDIR':'/tmp','RUST_BACKTRACE':'1'}}
    rows.append(row)
    with (E/'gpu-ledger.jsonl').open('a') as f:f.write(json.dumps(row)+'\n')
    print(json.dumps({'variant':name,'exit':code,'seconds':row['seconds']}),flush=True)
    assert not alive(p.pid) and not group_alive(p.pid)
 return code

def release(rows,owned):
 for p in owned:reap(p)
 live=[p.pid for p in owned if alive(p.pid) or group_alive(p.pid)]
 ledger=E/'gpu-ledger.jsonl'
 ledger.touch(exist_ok=True)
 ledger_live=[json.loads(line)['pid'] for line in ledger.read_text().splitlines() if line.strip() and alive(json.loads(line)['pid'])]
 new(E/'gpu-results.json',rows)
 new(E/'gpu-released.json',{'packet':'cluster-grid-readback-002','at':now(),'children':len(owned),'owned_processes':live,'ledger_pids_alive':ledger_live,'ledger_sha256':sha(ledger),'system':{'system':platform.system(),'release':platform.release(),'machine':platform.machine(),'mac_ver':platform.mac_ver()[0]}})
 assert not live and not ledger_live

def run():
 # A missing grant or unfinished predecessor refuses BEFORE any device work.
 grant=load(E/'gpu-grant.json');assert grant['packet']=='cluster-grid-readback-002'
 assert grant['runner_sha256']==sha(__file__)
 predecessor=Path(grant['predecessor_release']['path'])
 assert sha(predecessor)==grant['predecessor_release']['sha256']
 released=load(predecessor);assert not released['owned_processes'] and not released['ledger_pids_alive']
 ledger=predecessor.parent/'gpu-ledger.jsonl';assert sha(ledger)==released['ledger_sha256']
 assert all(not alive(json.loads(line)['pid']) for line in ledger.read_text().splitlines() if line.strip())
 source();assert not (E/'gpu-results.json').exists()
 builds={c['name'].removesuffix('-cpu'):c['binary'] for c in load(E/'results.json') if 'binary' in c}
 rows=[];owned=[]
 with (E/'gpu.lock').open('a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  try:
   for name in ['baseline','cooked']:
    source();b=builds[name];assert sha(b['path'])==b['sha256']
    code=execute(name,b,rows,owned)
    assert sha(b['path'])==b['sha256']
    if code:break
   source()
  finally:
   release(rows,owned)
 assert len(rows)==2 and all(r['exit']==0 for r in rows)
 pattern=re.compile(r'^cluster-readback (\S+) grid=(\d+)x(\d+)x24 words=(\d+) local=(\d+) counts_sha256=([0-9a-f]{64}) masks_sha256=([0-9a-f]{64})$',re.M)
 expected={f'{camera}-n{n}-v{v}' for camera in ['preview','perspective','orthographic'] for n in [1,31,32,33,4096] for v in range(5)}|{f'retained-{camera}-step{step}' for camera in ['perspective','orthographic'] for step in range(4)}|{'directional-only'}
 variants={};adapters={}
 for name in ['baseline','cooked']:
  raw=(E/(name+'-gpu.stdout')).read_text();assert '1 passed; 0 failed' in raw
  matches=pattern.findall(raw);assert len(matches)==84
  parsed={v[0]:v[1:] for v in matches};assert set(parsed)==expected
  actual=re.findall(r'cluster-readback-adapter (.+)',raw);assert len(actual)==1 and 'backend: Metal' in actual[0]
  adapters[name]=actual[0];variants[name]=parsed
 assert adapters['baseline']==adapters['cooked']
 assert variants['baseline']==variants['cooked']
 new(E/'gpu-parity.json',{'passed':True,'named_records_per_variant':84,'adapters':adapters,'exact_raw_count_and_mask_digests_equal':True,'variants':variants,'limits':['Metal on recorded adapter only','No image/pixel/appearance or performance evidence','Default versus cooked-clusters features as recorded']})
 print('Exact numerical parity: 84 records in each variant',flush=True)
if __name__=='__main__':run()
