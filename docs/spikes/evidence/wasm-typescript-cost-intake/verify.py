#!/usr/bin/env python3
"""Check curated helper/source identity without running tools or measurements."""
from pathlib import Path
import argparse,hashlib,json,re,subprocess,sys
P=Path(__file__).resolve().parent;R=P.parents[3];H=lambda b:hashlib.sha256(b).hexdigest()
a=argparse.ArgumentParser();a.add_argument('--external',action='store_true');a.add_argument('--git-source',action='store_true');x=a.parse_args()
m=json.loads((P/'qualification.json').read_text());assert H((P/'review.json').read_bytes())==m['review_sha256']
for row in m['owned_paths']:
 raw=(R/row['path']).read_bytes()
 if row['path']=='docs/spikes/wasm-typescript-cost.md':
  raw=raw.split(b'\n## Current-main intake\n')[0] if b'\n## Current-main intake\n' in raw else raw
 assert H(raw)==row['sha256'],row['path']
for c in m['commands']:
 assert c['exit']==0 and H((P/c['log']).read_bytes())==c['sha256']
assert b'Ran 21 tests' in (P/'python.log').read_bytes() and (P/'python.log').read_bytes().strip().endswith(b'OK')
cmd=[sys.executable,str(R/'docs/spikes/evidence/wasm-typescript-cost/verify.py')]
if x.external:cmd.append('--external')
if x.git_source:cmd.append('--git-source')
subprocess.run(cmd,cwd=R,check=True,stdout=subprocess.DEVNULL)
print(json.dumps({'passed':True,'curated_paths':len(m['owned_paths']),'focused_python_cases':21,'strict_typescript':True,'producer_replay':True,'historical_execution_source':m['executed_source']}))
