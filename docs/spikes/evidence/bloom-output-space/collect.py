#!/usr/bin/env python3
"""Lossless CPU-only bloom correction evidence; never runs a GPU or builds."""
from pathlib import Path
import gzip,hashlib,io,json,re,subprocess,sys,tarfile
ROOT=Path(__file__).resolve().parents[4];HERE=Path(__file__).resolve().parent
RAW=Path('/Users/andreyyurlov/.codex/incant-evidence/bloom-output-space-001')
MANIFEST=HERE.parent/'bloom-output-space.json'
def digest(b):return hashlib.sha256(b).hexdigest()
def verify(m):
 for row in m['artifacts']:
  packed=(ROOT/row['retained']).read_bytes();assert digest(packed)==row['gzip_sha256'];b=gzip.decompress(packed);assert len(b)==row['bytes'] and digest(b)==row['sha256']
  if Path(row['original']).exists():assert Path(row['original']).read_bytes()==b
 archive=gzip.decompress((HERE/'source-inputs.tar.gz').read_bytes());assert digest(archive)==m['source_tar_sha256']
 with tarfile.open(fileobj=io.BytesIO(archive),mode='r:') as tar:
  members={t.name:t for t in tar.getmembers()};assert len(members)==len(m['inputs'])
  for row in m['inputs']:
   member=members[row['path']];assert member.isfile();b=tar.extractfile(member).read();assert digest(b)==row['sha256'] and len(b)==row['bytes'];assert (ROOT/row['path']).read_bytes()==b
 for row in m['binaries']:
  p=Path(row['copy'])
  if p.exists():assert digest(p.read_bytes())==row['sha256'] and p.stat().st_size==row['bytes']
 assert len(m['checks'])==8 and all(r['exit']==0 for r in m['checks'])
 logs={Path(r['original']).name:gzip.decompress((ROOT/r['retained']).read_bytes()).decode() for r in m['artifacts'] if r['original'].endswith('.log')}
 assert 'test result: ok. 93 passed; 0 failed; 37 ignored' in logs['01-render-cpu.log']
 assert 'test result: ok. 1 passed; 0 failed; 0 ignored' in logs['04-cooked-refusal.log']
 assert 'test result: ok. 1 passed; 0 failed; 0 ignored' in logs['isolated-moments.log']
 matches=re.findall(r'DRS (\d+)/16 fixed=\[([^]]+)\] old=\[([^]]+)\]',logs['isolated-moments.log']);assert len(matches)==4
 actual={int(n):[float(x) for x in a.split(',')] for n,a,_ in matches};base=actual[16]
 for values in actual.values():
  assert abs(values[0]/base[0]-1)<.01
  assert ((values[1]-base[1])**2+(values[2]-base[2])**2)**.5<.6
  assert abs(values[3]/base[3]-1)<.025
 assert m['gpu_execution'] is False and m['original_gpu_gate']=='failed; unchanged'
 return {'status':'passed','inputs':len(m['inputs']),'artifacts':len(m['artifacts']),'cpu_passed':93,'ignored_gpu':37,'gpu_execution':False}
if '--verify-only' in sys.argv:print(json.dumps(verify(json.loads(MANIFEST.read_text()))));raise SystemExit
freeze=json.loads((RAW/'source-001/freeze.json').read_text());buf=io.BytesIO()
with tarfile.open(fileobj=buf,mode='w') as tar:
 for row in freeze['files']:
  b=(RAW/'source-001'/row['path']).read_bytes();assert digest(b)==row['sha256'];info=tarfile.TarInfo(row['path']);info.size=len(b);info.mode=0o644;tar.addfile(info,io.BytesIO(b))
source_tar=buf.getvalue();(HERE/'source-inputs.tar.gz').write_bytes(gzip.compress(source_tar,mtime=0))
paths=[*sorted(RAW.glob('*.log')),*sorted(RAW.glob('*.json')),RAW/'run.py',RAW/'oracle.rs',RAW/'source-001/freeze.json',RAW/'source-001/source.patch',Path('/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-003/analysis.json'),Path('/Users/andreyyurlov/.codex/worktrees/native-audio-host/incant/.worktrees/0070-ambient-occlusion/handoffs/0070-ambient-occlusion/corrected-gpu-result.md')]
artifacts=[]
for i,p in enumerate(paths):
 b=p.read_bytes();q=HERE/f'{i:02d}-{p.name}.gz';packed=gzip.compress(b,mtime=0);q.write_bytes(packed);artifacts.append({'original':str(p),'retained':str(q.relative_to(ROOT)),'sha256':digest(b),'bytes':len(b),'gzip_sha256':digest(packed)})
m={'base':freeze['base'],'source_freeze_sha256':digest((RAW/'source-001/freeze.json').read_bytes()),'inputs':freeze['files'],'source_tar_sha256':digest(source_tar),'source_tar_gzip_sha256':digest((HERE/'source-inputs.tar.gz').read_bytes()),'artifacts':artifacts,'binaries':json.loads((RAW/'binaries.json').read_text()),'checks':json.loads((RAW/'results.json').read_text())['checks'],'gpu_execution':False,'original_gpu_gate':'failed; unchanged','limits':['private effects branch, no main integration','CPU analytic covered rectangle is not a GPU result','Claude appearance and original GPU gates must be rechecked on corrected source','SSAO unchanged']}
MANIFEST.write_text(json.dumps(m,indent=2)+'\n');print(json.dumps(verify(m)))
