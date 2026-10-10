#!/usr/bin/env python3
"""Report Phase 0 readiness without manufacturing evidence or director approval."""
import argparse
import json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--strict',action='store_true');args=p.parse_args()
root=Path(__file__).resolve().parents[1]
gate=json.loads((root/'docs/gates/phase0.json').read_text())
missing=[f'spike:{name}' for name,value in gate['spikes'].items() if not value['passed']]
missing += [f'nightly:{name}' for name,passed in gate['nightly_runnable_targets'].items() if not passed]
for name in ('year_one_staffing_confirmed','director_approval'):
    if not gate[name]: missing.append(name)
print(json.dumps({'phase':0,'passed':not missing,'remaining':missing},indent=2))
if args.strict and missing: raise SystemExit(1)
