"""Fresh bounded temporal retry; original failed attempt remains immutable."""
from pathlib import Path
import json,hashlib,subprocess,importlib.util,fcntl,platform,os,signal,sys
R=Path(__file__).resolve().parents[3];E=Path.home()/'.codex/incant-evidence/0073-upscaling-numeric'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();load=lambda p:json.loads(Path(p).read_text())
def main():
 grant=load(E/'review-ready.json');assert grant['packet']=='0073-upscaling-numeric' and grant['runner_sha256']==h(__file__)
 assert not (E/'gpu-released.json').exists(),'grant already consumed'
 manifest=load(Path.home()/'.codex/incant-evidence/temporal-upscale-source-001/freeze.json')
 assert h(Path.home()/'.codex/incant-evidence/temporal-upscale-source-001/freeze.json')=='b7ee5ca277f5ca3b1574dd6640638347ca068cb11ba207cb4c1642bcf9f67411'
 assert h(R/'docs/spikes/evidence/temporal-upscaling.json')=='2d2f4ffa9c6c43a95ab733f8fa3fa92ac4f5a1743f0759597e31534e70f531ca'
 def source():
  for v in manifest['sources']+manifest['unchanged_baselines']:assert h(R/v['path'])==v['sha256'],v['path']
 source()
 binary=load(R/'docs/spikes/evidence/temporal-upscaling.json')['binaries'][0];binary['retained']=binary['copied_path'];assert h(binary['retained'])==binary['sha256']=='afa3d035e4cbf24f3166ee9c2feacbc09603c8f03f21b856e894526d50bb4298'
 binary['retained']=grant['executable_copy']['path'];assert h(binary['retained'])==grant['executable_copy']['sha256'] and os.access(binary['retained'],os.X_OK)
 b={'path':binary['retained'],'sha256':binary['sha256'],'features':['default'],'source':'0452c5f5afc11c99b7c90d111428cc23611c9590'}
 prior=Path(grant['predecessor']['path']);assert h(prior)==grant['predecessor']['sha256'];p=load(prior)
 assert not p['owned_processes'] and not p['ledger_pids_alive'];assert h(prior.parent/'gpu-ledger.jsonl')==p['ledger_sha256']
 module=Path(grant['lifecycle_runner']['path']);assert h(module)==grant['lifecycle_runner']['sha256']
 spec=importlib.util.spec_from_file_location('audited_lifecycle',module);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.R=R;m.E=E
 predecessor_rows=list(map(json.loads,(prior.parent/'gpu-ledger.jsonl').read_text().splitlines()))
 predecessor_children=[v for v in predecessor_rows if 'pid' in v]
 assert len(predecessor_children)==p['children']
 for v in predecessor_children:assert not m.alive(v['pid']) and not m.group_alive(v['pid'])
 rows=[];owned=[]
 tests=['upscaling::system::device_tests::actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay','models::retained::upscaling_tests::upscaling_color_motion_geometry_and_dynamic_source_extent_commit_together']
 binary_bytes=Path(b['path']).read_bytes()
 for name in tests:
  assert binary_bytes.count(name.encode())==1, name
 if '--preflight' in sys.argv:
  print('Source, binary execute mode, grant, exact tests and predecessor children verified; no GPU process launched.');return
 with (E.parent/'native-gpu.lock').open('a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  assert not (E/'gpu-released.json').exists()
  with (E/'attempt-started.json').open('x') as started:
   json.dump({'at':m.now(),'runner_sha256':h(__file__)},started);started.flush();os.fsync(started.fileno())
  (E/'attempt-debits.jsonl').touch(exist_ok=False)
  def interrupted(signum,frame): raise KeyboardInterrupt(f'signal {signum}')
  for sig in (signal.SIGTERM,signal.SIGHUP):signal.signal(sig,interrupted)
  try:
   for ordinal,name in enumerate(tests):
    with (E/'attempt-debits.jsonl').open('a') as debit:
     debit.write(json.dumps({'ordinal':ordinal,'test':name,'frames_max':[14,18][ordinal],'at':m.now()})+'\n');debit.flush();os.fsync(debit.fileno())
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
 m.new(E/'summary.json',{'source':b['source'],'tests':tests,'passed':2,'new_frames_upper_bound':32,'output_sizes':[[8,8],[65,33]],'readback':'RGBA16Float reconstructed/composite scalar rows plus retained DRS/ownership assertions','appearance':'not examined','adapter_identity':'printed by numerical upscaling case; not a benchmark','binary_sha256':b['sha256']})
 print('Both exact upscaling numeric tests passed; GPU released.')
if __name__=='__main__':main()
