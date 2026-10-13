"""Two frozen native motion-data tests. No build, screenshot, or appearance claim."""
from pathlib import Path
import json,hashlib,subprocess,importlib.util,fcntl,platform
R=Path(__file__).resolve().parents[3];E=Path.home()/'.codex/incant-evidence/0072'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();load=lambda p:json.loads(Path(p).read_text())
def main():
 grant=load(E/'review-ready.json');assert grant['packet']=='0072-motion-history-numeric' and grant['runner_sha256']==h(__file__)
 assert not (E/'gpu-released.json').exists(),'grant already consumed'
 manifest=load(R/'docs/spikes/evidence/retained-motion-history.json')
 assert h(R/'docs/spikes/evidence/retained-motion-history.json')=='1e6dd3f7962ff1cb0bbcd3ba6cb1efbfa17e18a8a48f59a63dfdda5d31ee3273'
 def source():
  for v in manifest['sources']+manifest['unchanged_baselines']:assert h(R/v['path'])==v['sha256'],v['path']
 source()
 binary=manifest['binaries'][0];assert h(binary['retained'])==binary['sha256']=='73b32363318cfcd028cccef5513994488a6d4bbbf277bee181523b7b68bc05f5'
 b={'path':binary['retained'],'sha256':binary['sha256'],'features':['default'],'source':'814717935462e4ee5dbdb1fdae2738d17477f932'}
 prior=Path(grant['predecessor']['path']);assert h(prior)==grant['predecessor']['sha256'];p=load(prior)
 assert not p['owned_processes'] and not p['ledger_pids_alive'];assert h(prior.parent/'gpu-ledger.jsonl')==p['ledger_sha256']
 module=Path(grant['lifecycle_runner']['path']);assert h(module)==grant['lifecycle_runner']['sha256']
 spec=importlib.util.spec_from_file_location('audited_lifecycle',module);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.R=R;m.E=E
 for v in map(json.loads,(prior.parent/'gpu-ledger.jsonl').read_text().splitlines()):assert not m.alive(v['pid']) and not m.group_alive(v['pid'])
 rows=[];owned=[]
 tests=['accepted_pose_buffers_and_history_swap_only_after_submission','skinned_and_ik_history_uses_exact_live_deformed_buffer_and_rejects_rebound_asset']
 with (E.parent/'native-gpu.lock').open('a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
  try:
   for name in tests:
    source();assert h(b['path'])==b['sha256'];m.TEST='motion_device_tests::'+name
    rc=m.execute(name,b,rows,owned,timeout=60)
    assert rc==0,(name,rc)
   source()
  finally:
   for child in owned:m.reap(child)
   live=[c.pid for c in owned if m.alive(c.pid) or m.group_alive(c.pid)]
   ledger=E/'gpu-ledger.jsonl';ledger.touch(exist_ok=True)
   ledger_live=[v['pid'] for v in map(json.loads,ledger.read_text().splitlines()) if m.alive(v['pid']) or m.group_alive(v['pid'])]
   m.new(E/'gpu-results.json',rows)
   m.new(E/'gpu-released.json',{'packet':grant['packet'],'at':m.now(),'children':len(owned),'owned_processes':live,'ledger_pids_alive':ledger_live,'ledger_sha256':h(ledger),'target':'none; precompiled frozen binaries only','system':{'system':platform.system(),'release':platform.release(),'machine':platform.machine(),'mac_ver':platform.mac_ver()[0]}})
   assert not live and not ledger_live
 assert len(rows)==2
 for row in rows:assert '1 passed; 0 failed' in Path(row['stdout']['path']).read_text()
 m.new(E/'summary.json',{'source':b['source'],'tests':tests,'passed':2,'new_frames_upper_bound':9,'output_sizes':[[65,33],[96,64]],'readback':'RGBA32Float motion/depth/validity numeric data only','appearance':'not examined','adapter_identity':'not printed by these existing tests; not a backend/device benchmark','binary_sha256':b['sha256']})
 print('Both exact retained motion-data tests passed; GPU released.')
if __name__=='__main__':main()
