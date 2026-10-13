"""Fresh bounded temporal retry; original failed attempt remains immutable."""
from pathlib import Path
import json,hashlib,subprocess,importlib.util,fcntl,platform,os,signal
R=Path(__file__).resolve().parents[3];E=Path.home()/'.codex/incant-evidence/0072-temporal-r2'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();load=lambda p:json.loads(Path(p).read_text())
def main():
 grant=load(E/'review-ready.json');assert grant['packet']=='0072-motion-history-numeric' and grant['runner_sha256']==h(__file__)
 assert not (E/'gpu-released.json').exists(),'grant already consumed'
 manifest=load(Path.home()/'.codex/incant-evidence/temporal-color-source-001/freeze.json')
 assert h(Path.home()/'.codex/incant-evidence/temporal-color-source-001/freeze.json')=='06a242fd0d56fa2afc05c9931567795a12919e6b82f0dd90ee8dcd94adeeb39f'
 assert h(R/'docs/spikes/evidence/temporal-color-accumulation.json')=='2040592a60773886f605f396121224659a4a86334f636c406672e6e4ff6d084a'
 def source():
  for v in manifest['sources']+manifest['unchanged_baselines']:assert h(R/v['path'])==v['sha256'],v['path']
 source()
 binary=load(R/'docs/spikes/evidence/temporal-color-accumulation.json')['binaries'][0];binary['retained']=binary['copied_path'];assert h(binary['retained'])==binary['sha256']=='665e9de2750dca6ed6dcca7bdb5e25561d75a1e7e6d2592ee86fe5c2b51fed94'
 binary['retained']=grant['executable_copy']['path'];assert h(binary['retained'])==grant['executable_copy']['sha256'] and os.access(binary['retained'],os.X_OK)
 b={'path':binary['retained'],'sha256':binary['sha256'],'features':['default'],'source':'e73afe43ffaf78e73b725876411128d092791f5a'}
 prior=Path(grant['predecessor']['path']);assert h(prior)==grant['predecessor']['sha256'];p=load(prior)
 assert not p['owned_processes'] and not p['ledger_pids_alive'];assert h(prior.parent/'gpu-ledger.jsonl')==p['ledger_sha256']
 module=Path(grant['lifecycle_runner']['path']);assert h(module)==grant['lifecycle_runner']['sha256']
 spec=importlib.util.spec_from_file_location('audited_lifecycle',module);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.R=R;m.E=E
 for v in map(json.loads,(prior.parent/'gpu-ledger.jsonl').read_text().splitlines()):assert not m.alive(v['pid']) and not m.group_alive(v['pid'])
 rows=[];owned=[]
 tests=['temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion','models::retained::temporal_tests::temporal_color_motion_and_geometry_publish_once_and_failures_consume_no_phase']
 binary_bytes=Path(b['path']).read_bytes()
 for name in tests:
  assert binary_bytes.count(name.encode())==1, name
 with (E.parent/'native-gpu.lock').open('a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  assert not (E/'gpu-released.json').exists()
  with (E/'attempt-started.json').open('x') as started:
   json.dump({'at':m.now(),'runner_sha256':h(__file__)},started);started.flush();os.fsync(started.fileno())
  def interrupted(signum,frame): raise KeyboardInterrupt(f'signal {signum}')
  for sig in (signal.SIGTERM,signal.SIGHUP):signal.signal(sig,interrupted)
  try:
   for ordinal,name in enumerate(tests):
    with (E/'attempt-debits.jsonl').open('a') as debit:
     debit.write(json.dumps({'ordinal':ordinal,'test':name,'frames_max':[8,5][ordinal],'at':m.now()})+'\n');debit.flush();os.fsync(debit.fileno())
    source();assert h(b['path'])==b['sha256'];m.TEST=name
    rc=m.execute(name,b,rows,owned,timeout=60)
    assert rc==0,(name,rc)
    assert '1 passed; 0 failed' in Path(rows[-1]['stdout']['path']).read_text(), 'Exact test did not run once'
   source()
  finally:
   for child in owned:m.reap(child)
   live=[c.pid for c in owned if m.alive(c.pid) or m.group_alive(c.pid)]
   ledger=E/'gpu-ledger.jsonl';ledger.touch(exist_ok=True)
   ledger_live=[v['pid'] for v in map(json.loads,ledger.read_text().splitlines()) if m.alive(v['pid']) or m.group_alive(v['pid'])]
   m.new(E/'gpu-results.json',rows)
   m.new(E/'gpu-released.json',{'packet':grant['packet'],'at':m.now(),'children':len(owned),'owned_processes':live,'ledger_pids_alive':ledger_live,'ledger_sha256':h(ledger),'debits_sha256':h(E/'attempt-debits.jsonl'),'target':'none; precompiled frozen binaries only','system':{'system':platform.system(),'release':platform.release(),'machine':platform.machine(),'mac_ver':platform.mac_ver()[0]}})
   assert not live and not ledger_live
 assert len(rows)==2
 for row in rows:assert '1 passed; 0 failed' in Path(row['stdout']['path']).read_text()
 m.new(E/'summary.json',{'source':b['source'],'tests':tests,'passed':2,'new_frames_upper_bound':13,'output_sizes':[[8,8],[65,33]],'readback':'RGBA16Float temporal resolve/copy numeric data plus retained ownership assertions','appearance':'not examined','adapter_identity':'not printed by these existing tests; not a backend/device benchmark','binary_sha256':b['sha256']})
 print('Both exact temporal numeric tests passed; GPU released.')
if __name__=='__main__':main()
