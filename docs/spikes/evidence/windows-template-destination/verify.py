#!/usr/bin/env python3
"""Offline template destination repair replay; no compiler, player or device."""
import argparse,base64,gzip,hashlib,json
from pathlib import Path
import subprocess

def require(ok,message):
    if not ok:
        raise ValueError(message)

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--originals',action='store_true')
    p.add_argument('--binaries',action='store_true')
    p.add_argument('--git-source')
    args=p.parse_args();here=Path(__file__).resolve().parent
    m=json.loads((here/'manifest.json').read_text());archive=(here/'records.json.gz').read_bytes()
    require(digest(archive)==m['archive_sha256'],'Archive differs')
    with gzip.open(here/'records.json.gz','rb') as f:
        payload=f.read(24*1024**2+1)
    require(len(payload)<=24*1024**2,'Archive exceeds bound')
    encoded=json.loads(payload);require(set(encoded)=={r['key'] for r in m['records']},'Record membership differs');raw={}
    for row in m['records']:
        b=base64.b64decode(encoded[row['key']],validate=True)
        require(len(b)==row['bytes'] and digest(b)==row['sha256'],'Record differs');raw[row['key']]=b
        if args.originals:
            require(Path(row['original']).read_bytes()==b,'Original differs: '+row['key'])
    def data(key):return json.loads(raw[key])
    r=data('cpu-001/results.json');plan=data('cpu-plan-001.json');close=data('closeout-001.json')
    require(r['source']==plan['source']==m['source'] and r['status']=='passed' and r['terminal'],'Final source/result differs')
    require(r['proposal_sha256']==digest(raw['cpu-plan-001.json']) and r['supervisor_sha256']==digest(raw['cpu-supervisor-001.py'])
            and plan['freeze_sha256']==digest(raw['freeze.json']),'Protocol differs')
    require([x['exit'] for x in r['commands']]==[0,0,0] and all(x['cleanup_confirmed'] for x in r['commands']),'CPU sequence differs')
    require(b'7 passed; 0 failed;' in raw['cpu-001/0.stdout.log'] and b'3 passed; 0 failed;' in raw['cpu-001/1.stdout.log'],'Actual coverage differs')
    require(len(r['cleanup_records'])==3 and not r['cleanup_errors'] and all(x['original_stop_returned'] for x in r['cleanup_records']),'Cleanup unconfirmed')
    require(close['supervisor_reaped'] and close['FONT_target_and_CPU_slot_released'] and close['matching_live_pids_or_groups']==[]
            and close['result_sha256']==digest(raw['cpu-001/results.json']),'Closeout differs')
    require(close['actual_tests']['Windows_only_regression_unrun_on_Mac'] is True,'Windows scope differs')
    require(close['sampled_max_target_growth_bytes']<=plan['bounds']['growth_bytes'] and close['minimum_sampled_free_bytes']>=plan['bounds']['free_floor_bytes'],'Resource cap differs')
    require(b'Incorrect function.' in raw['checkpoint42-windows-original.txt'],'Original Windows failure lost')
    if args.git_source:
        for row in data('freeze.json')['files']:
            value=subprocess.run(['git','show',args.git_source+':'+row['path']],cwd=here.parents[3],check=True,capture_output=True).stdout
            require(len(value)==row['bytes'] and digest(value)==row['sha256'],'Git source differs: '+row['path'])
    if args.binaries:
        for row in m['artifacts']:
            p=Path(row['path']);require(p.stat().st_size==row['bytes'] and digest(p.read_bytes())==row['sha256'],'External artifact differs')
    print(f'PASS {len(m["records"])} records;7 template+3headless tests;strict lint;Windows execution pending')

if __name__=='__main__':main()
