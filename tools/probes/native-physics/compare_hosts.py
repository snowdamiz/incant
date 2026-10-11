#!/usr/bin/env python3
"""Verify uploaded native ECS streams across all three required desktop hosts."""
import json
from pathlib import Path
import sys
from run import compare, inspect

files=sorted(Path(sys.argv[1]).glob('*/report.json'))
if len(files)!=3:
    raise ValueError('expected three complete native ECS host reports')
reports=[json.loads(p.read_text()) for p in files]
if {r['host']['system'] for r in reports}!={'Darwin','Linux','Windows'}:
    raise ValueError('missing desktop host')
if len({json.dumps(r['source_sha256'],sort_keys=True) for r in reports})!=1:
    raise ValueError('hosts tested different source inputs')
references={}
for file,report in zip(files,reports):
    if report['status']!='passed' or report['trials']!=3:
        raise ValueError('missing completed trials')
    browser=report['host']['system']=='Linux'
    expected={(scenario,mode,workers,trial) for scenario in ('3d','2d') for mode,workers in [('native-parallel',1),('native-parallel',4)] for trial in range(3)}
    if browser:
        expected|={(scenario,'browser-wasm-serial',0,trial) for scenario in ('3d','2d') for trial in range(3)}
    actual={(r['scenario'],r['mode'],r['workers'],r['trial']) for r in report['records']}
    if report['browser_executed']!=browser or actual!=expected or len(report['records'])!=len(expected):
        raise ValueError('missing native/browser executions')
    for row in report['records']:
        path=file.parent/row['file']
        if path.parent!=file.parent or path.suffix!='.bin':
            raise ValueError('invalid stream path')
        data=path.read_bytes();checked=inspect(data,row['scenario'])
        if any(row[key]!=value for key,value in checked.items()):
            raise ValueError('stream/report identity mismatch')
        references.setdefault(row['scenario'],data)
        compare(references[row['scenario']],data,row['scenario'])
print('All full native ECS 3D/2D body-state streams match across three desktop hosts and serial browser WASM.')
