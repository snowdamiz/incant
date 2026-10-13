"""Read-only assertion audit; no build, GPU or process launch."""
from pathlib import Path
import json,gzip,hashlib,re,stat,sys
r=Path(sys.argv[1] if len(sys.argv)>1 else '/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant')
p=r/'docs/spikes/evidence/temporal-edge-correctness.json';m=json.loads(p.read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
for row in m['source_freeze']['sources']+m['source_freeze']['baselines']:
 assert sha((r/row['path']).read_bytes())==row['sha256'],row['path']
 assert sha(Path(row['copy']).read_bytes())==row['sha256'],row['copy']
raws={}
for row in m['artifacts']:
 retained=(r/row['retained']).read_bytes();assert sha(retained)==row['retained_sha256']
 raw=gzip.decompress(retained);assert sha(raw)==row['sha256'] and len(raw)==row['bytes']
 assert Path(row['original']).read_bytes()==raw,row['original']
 raws[row['original']]=raw
assert len(m['checks'])==7 and all(row['exit']==0 and row['reaped'] for row in m['checks'])
for row in m['checks']:assert sha(raws[row['log']])==row['sha256']
for label,needle in [('native','checks-003/native.log'),('cooked_features','checks-003/cooked-refusal.log')]:
 raw=next(v for k,v in raws.items() if k.endswith(needle)).decode()
 rows=[list(map(int,x)) for x in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw)]
 assert m['counts'][label]==dict(passed=sum(x[0] for x in rows),failed=sum(x[1] for x in rows),ignored=sum(x[2] for x in rows),groups=len(rows))
for row in m['binaries']:
 b=Path(row['copy']);assert sha(b.read_bytes())==row['sha256'] and b.stat().st_size==row['bytes']
 assert oct(stat.S_IMODE(b.stat().st_mode))==row['mode'] and stat.S_IMODE(b.stat().st_mode)&0o111
review=m['independent_source_review'];assert sha((r/review['retained']).read_bytes())==review['sha256']
assert Path(review['original']).read_bytes()==(r/review['retained']).read_bytes()
assert not m['gpu_review']['executed']
print(json.dumps(dict(sources=len(m['source_freeze']['sources']),baselines=len(m['source_freeze']['baselines']),artifacts=len(m['artifacts']),binaries=len(m['binaries']),checks=len(m['checks']),counts=m['counts'],manifest_sha256=sha(p.read_bytes())),indent=2))
