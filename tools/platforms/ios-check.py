#!/usr/bin/env python3
"""Run only this project's probe on an available simulator and check its report."""
import json
from pathlib import Path
import subprocess
import time

def sim(*args): return subprocess.check_output(['xcrun','simctl',*args],text=True).strip()
root=Path(__file__).resolve().parents[2]
devices=json.loads(sim('list','devices','available','--json'))['devices']
phones=[d for runtime,ds in devices.items() if 'iOS' in runtime for d in ds if 'iPhone' in d['name']]
if not phones: raise SystemExit('No iOS simulator installed; do not count the platform as tested')
device=next((d for d in phones if d['state']=='Booted'),phones[0]); uid=device['udid']; booted=device['state']!='Booted'
try:
    if booted: sim('boot',uid)
    sim('bootstatus',uid,'-b')
    sim('install',uid,str(root/'artifacts/ios-simulator/IncantSmoke.app'))
    container=Path(sim('get_app_container',uid,'dev.incant.smoke','data'))
    result=container/'Documents/smoke-result.json'
    result.unlink(missing_ok=True)
    sim('launch','--terminate-running-process',uid,'dev.incant.smoke')
    for _ in range(60):
        if result.exists(): break
        time.sleep(0.5)
    report=json.loads(result.read_text()); assert report['ok'] is True and report['status']==0,report
    (root/'artifacts/ios-simulator/result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))
finally:
    subprocess.run(['xcrun','simctl','terminate',uid,'dev.incant.smoke'],capture_output=True)
    if booted: sim('shutdown',uid)
