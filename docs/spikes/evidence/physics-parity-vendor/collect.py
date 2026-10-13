#!/usr/bin/env python3
"""Retain and independently recheck the scoped checkpoint21 probe correction."""
from pathlib import Path
import gzip,hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[4]
HERE=Path(__file__).resolve().parent
RAW=Path('/Users/andreyyurlov/.codex/incant-evidence/physics-parity-vendor-001')
MANIFEST=HERE.parent/'physics-parity-vendor.json'
def digest(b):return hashlib.sha256(b).hexdigest()
def verify(m):
 for row in m['artifacts']:
  packed=(ROOT/row['retained']).read_bytes();assert digest(packed)==row['gzip_sha256'];b=gzip.decompress(packed);assert len(b)==row['bytes'] and digest(b)==row['sha256']
  if Path(row['original']).exists():assert Path(row['original']).read_bytes()==b
 for row in m['sources']:assert digest((ROOT/row['path']).read_bytes())==row['sha256']
 rows=json.loads((RAW/'actual-targets.json').read_text()) if RAW.exists() else json.loads(gzip.decompress((ROOT/m['targets_record']).read_bytes()))
 assert len(rows)==49 and len({(r['root'],r['solver'],r['target']) for r in rows})==49
 for r in rows:
  assert 'enhanced-determinism' in r['rapier_features'];assert ('parallel' in r['rapier_features'])==r['parallel']==(r['target']!='wasm32-unknown-unknown')
 assert m['checks'][0]['exit']==m['checks'][1]['exit']==m['checks'][2]['exit']==0
 assert m['hosted_failure']['head']=='51d8ab52a776e133c55acae180b4da464b4446db'
 assert subprocess.check_output(['git','diff','--name-only',m['base'],'--','crates','third_party','Cargo.toml','Cargo.lock','.github'],cwd=ROOT,text=True)==''
 return {'status':'passed','artifacts':len(m['artifacts']),'solver_target_records':len(rows),'source_files':len(m['sources'])}
if '--verify-only' in sys.argv:
 print(json.dumps(verify(json.loads(MANIFEST.read_text()))));raise SystemExit
paths=[*sorted(RAW.glob('*.log')),*sorted(RAW.glob('*.json')),RAW/'red-exit.txt',RAW/'repair.patch',RAW/'run.py',Path('/Users/andreyyurlov/.codex/incant-integration-evidence/checkpoint21/failures/portability.log'),HERE/'source-review.json']
artifacts=[]
for i,p in enumerate(paths):
 b=p.read_bytes();q=HERE/f'{i:02d}-{p.name}.gz';packed=gzip.compress(b,mtime=0);q.write_bytes(packed);artifacts.append({'original':str(p),'retained':str(q.relative_to(ROOT)),'sha256':digest(b),'bytes':len(b),'gzip_sha256':digest(packed)})
m={'base':'d96087e836c0b3ea28edfb2012b5dc116befc0d1','scope':'Probe package identity only; real hosted rerun still required','sources':[{'path':p,'sha256':h} for p,h in json.loads((RAW/'source.json').read_text()).items()],'artifacts':artifacts,'checks':json.loads((RAW/'checks.json').read_text()),'focused_tests':7,'tools_tests':{'python':373,'platform_skips':4,'node':22},'hosted_failure':{'head':'51d8ab52a776e133c55acae180b4da464b4446db','job':115226651500,'error':'missing pinned rapier3d dependency'},'old_source_reproduction_exit':int((RAW/'red-exit.txt').read_text()),'targets_record':next(r['retained'] for r in artifacts if r['original']==str(RAW/'actual-targets.json'))}
assert m['old_source_reproduction_exit']==1 and not (RAW/'red-result.json').exists()
MANIFEST.write_text(json.dumps(m,indent=2)+'\n');print(json.dumps(verify(m)))
