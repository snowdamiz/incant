"""Read-only replay of retained checkpoint30 policy, CI and merge identities."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sha = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((HERE / 'manifest.json').read_text())
a = (HERE / 'artifacts.json.gz').read_bytes()
assert len(a) == m['archive_bytes'] and sha(a) == m['archive_sha256']
rows = json.loads(gzip.decompress(a))['records']
assert len(rows) == m['records'] == len({r['name'] for r in rows})
raw = {}
for r in rows:
    b = base64.b64decode(r['base64'], validate=True)
    assert len(b) == r['bytes'] and sha(b) == r['sha256'], r['name']
    raw[r['name']] = b
read = lambda n: json.loads(raw[n])
pre = read('prepromotion.json')
assert sha(raw['prepromotion.json']) == m['prepromotion_sha256']
for n, digest in pre['artifacts'].items():
    assert sha(raw[n]) == digest, n
review = read('root-review.json')
assert review['accepted'] and sha(raw['root-review.json']) == m['root_review_sha256']
assert review['candidate'] == m['candidate'] and review['tested_tree'] == m['tree']
policy = read('protection-final.json')['required_status_checks']['contexts']
assert read('protection.json') == read('protection-final.json')
checks = read('required-final.json')['required']
assert len(checks) == len({c['name'] for c in checks}) == 23
assert sorted(policy) == sorted(c['name'] for c in checks) == sorted(review['required_actual_success'])
assert all(c['conclusion'] == 'SUCCESS' and c['status'] == 'COMPLETED' for c in checks)
pr = read('pr-final.json')
assert pr['headRefOid'] == m['candidate'] and pr['baseRefOid'] == m['release_parents'][0]
assert pr['state'] == 'OPEN' and pr['mergeable'] == 'MERGEABLE' and pr['mergeStateStatus'] == 'CLEAN'
rollup = {c['name']: c for c in pr['statusCheckRollup']}
assert all(rollup[c['name']] == c for c in checks)
runs = read('runs-final.json')['workflow_runs']
assert len(runs) == len({r['workflow_id'] for r in runs}) == 8
assert all(r['head_sha'] == m['candidate'] and r['event'] == 'push' and r['head_branch'] == 'codex/release-candidate' and r['status'] == 'completed' and r['conclusion'] == 'success' and r['run_attempt'] == 1 for r in runs)
jobs = []
for r in runs:
    group = read(f"jobs-{r['id']}.json")
    assert group['total_count'] == len(group['jobs'])
    jobs.extend(group['jobs'])
assert len(jobs) == 28 and sum(j['conclusion'] == 'success' for j in jobs) == 27
assert all(j['head_sha'] == m['candidate'] and j['status'] == 'completed' and (j['conclusion'] == 'success' or (j['name'] == 'deploy' and j['conclusion'] == 'skipped')) for j in jobs)
def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
for ref in (m['release'], m['candidate'], m['selected_main']):
    assert git('rev-parse', ref + '^{tree}') == m['tree']
for ref in (m['release_parents'][0], m['selected_main'], read('manifest.json')['previous_candidate']):
    git('merge-base', '--is-ancestor', ref, m['candidate'])
assert git('show', '-s', '--format=%P', m['release']).split() == m['release_parents']
assert git('show', '-s', '--format=%B', m['release']).endswith('Built-by: astra')
for ref in (m['candidate'], m['release_parents'][0]):
    assert sorted(json.loads(git('show', ref + ':.github/release-checks.json'))['required_contexts']) == sorted(policy)
merged = read('pr-merged.json')
assert merged['state'] == 'MERGED' and merged['headRefOid'] == m['candidate'] and merged['mergeCommit']['oid'] == m['release']
print(json.dumps({'passed': True, 'records': len(rows), 'required_success': 23, 'workflow_success': 8, 'release': m['release'], 'tree': m['tree']}))
