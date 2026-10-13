"""Verify retained checkpoint24 evidence without network, builds, or mutations."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
manifest = json.loads(HERE.with_suffix('.json').read_text())
sha = lambda data: hashlib.sha256(data).hexdigest()
for name, row in manifest['retained_artifacts'].items():
    data = (HERE / name).read_bytes()
    assert len(data) == row['bytes'] and sha(data) == row['sha256'], name
pre = json.loads((HERE / 'prepromotion.json').read_text())
assert sha((HERE / 'prepromotion.json').read_bytes()) == manifest['prepromotion_sha256']
for name, digest in pre['artifacts'].items():
    assert sha((HERE / name).read_bytes()) == digest, name
assert sha((HERE / 'independent-promotion-review.json').read_bytes()) == manifest['independent_review_sha256']
required = json.loads((HERE / 'required-final.json').read_text())
assert required['candidate'] == manifest['candidate']
checks = required['required']
assert len(checks) == len({c['name'] for c in checks}) == 23
assert all(c['conclusion'] == 'SUCCESS' and c['status'] == 'COMPLETED' for c in checks)
policy = json.loads((HERE / 'protection-final.json').read_text())['required_status_checks']['contexts']
assert sorted(policy) == sorted(c['name'] for c in checks)
runs = json.loads((HERE / 'runs-final.json').read_text())['workflow_runs']
assert len(runs) == len({r['workflow_id'] for r in runs}) == 8
assert all(r['head_sha'] == manifest['candidate'] and r['event'] == 'push' and r['status'] == 'completed' and r['conclusion'] == 'success' for r in runs)
jobs = [j for r in runs for j in json.loads((HERE / f"jobs-{r['id']}.json").read_text())['jobs']]
assert len(jobs) == 28 and sum(j['conclusion'] == 'success' for j in jobs) == 27
assert all(j['head_sha'] == manifest['candidate'] and j['status'] == 'completed' and (j['conclusion'] == 'success' or (j['name'] == 'deploy' and j['conclusion'] == 'skipped')) for j in jobs)
for stem, row in manifest['hosted_logs'].items():
    packed = (HERE / (stem + '.log.gz')).read_bytes()
    raw = gzip.decompress(packed)
    assert sha(packed) == row['gzip_sha256'] and len(packed) == row['gzip_bytes']
    assert sha(raw) == row['raw_sha256'] and len(raw) == row['raw_bytes']
def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
for ref in [manifest['release'], manifest['candidate'], manifest['selected_main']]:
    assert git('rev-parse', ref + '^{tree}') == manifest['tree']
assert git('show', '-s', '--format=%P', manifest['release']).split() == manifest['release_parents']
assert git('show', '-s', '--format=%B', manifest['release']).endswith('Built-by: astra')
print(json.dumps({'passed': True, 'records': len(manifest['retained_artifacts']), 'required_success': 23, 'workflow_success': 8, 'release': manifest['release']}))

first=json.loads((HERE/'prepare-attempt1.json').read_text())
assert first['exit']==1 and first['tool_reported_error']=='gh: Server Error (HTTP 502)'
for r in first['retained_successful_queries']:
 assert sha((HERE/'prepare-attempt1'/Path(r['path']).name).read_bytes())==r['sha256']
for stem,backend in [('macos-checks','Metal'),('linux-desktop','Vulkan'),('windows-desktop','Dx12')]:
 raw=gzip.decompress((HERE/(stem+'.log.gz')).read_bytes()).decode()
 assert 'Incant required GPU backend: '+backend in raw
 assert 'replay_refuses_nonuser_budget_commands_that_do_not_change_the_projection ... ok' in raw
 diagnostic=[json.loads(line.split('TIMESTAMP_PROBE ',1)[1]) for line in raw.splitlines() if 'TIMESTAMP_PROBE {' in line]
 assert len(diagnostic)==(4 if backend=='Metal' else 8)
 for d in diagnostic:
  assert d['values']==[1956956480,2237716543]
  if backend=='Metal':
   assert d['status']=='Unsupported' and d['ticks'] is None and d['positive_interval'] is None
  else:
   assert d['status']=='MeasuredDiagnostic' and d['positive_interval'] and d['resolve_changed']
linux=gzip.decompress((HERE/'linux-desktop.log.gz').read_bytes()).decode()
assert linux.count('Vulkan catalog: 31 render + 1 compute;')==4
assert 'read: Missing' in linux and 'read: Rejected' in linux and linux.count('read: Supplied')==2
print(json.dumps({'hosted_replay_repair':True,'actual_backend_checks':True,'optional_query_capability_paths':True,'vulkan_cache_lifecycle':True}))
