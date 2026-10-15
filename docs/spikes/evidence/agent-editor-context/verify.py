"""Read-only evidence replay; no provider, build, window or screen capture."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--git-source', action='store_true')
args = p.parse_args()
here = Path(__file__).resolve().parent
root = here.parents[3]
digest = lambda data: hashlib.sha256(data).hexdigest()
manifest = json.loads((here / 'manifest.json').read_text())
archive = (here / 'records.json.gz').read_bytes()
assert digest(archive) == manifest['archive_sha256']
records = json.loads(gzip.decompress(archive))
assert set(records) == set(manifest['records'])
for name, text in records.items():
    assert digest(text.encode()) == manifest['records'][name], name
source = json.loads(records[manifest['source_record']])
results = json.loads(records[manifest['results']])
review = json.loads(records[manifest['independent_review']])
assert results['source'] == source['source'] == review['reviewed_commit'] == manifest['source']
assert results['complete'] and not results['screen_capture'] and not results['native_window']
assert results['source_manifest_sha256'] == digest(records['source.json'].encode())
assert results['runner_sha256'] == digest(records['run.py'].encode())
assert review['findings'] == []
expected = {'agent-tests', 'editor-agent-tests', 'strict-lint', 'ui-all-tests',
            'ui-typecheck', 'ui-build', 'generated-bridge', 'fmt', 'conventions'}
steps = {row['name']: row for row in results['steps']}
assert set(steps) == expected and len(steps) == len(results['steps'])
for name, row in steps.items():
    assert row['exit_code'] == 0 and row['terminal_reaped'] and 'error' not in row, name
    log = records[name + '.log']
    assert digest(log.encode()) == row['log_sha256'] and len(log.encode()) == row['log_bytes'], name
for name, count in [('agent-tests', 115), ('editor-agent-tests', 23)]:
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', records[name + '.log'])
    assert sum(int(n) for n, _ in rows) == count == steps[name]['passed_tests']
    assert all(int(failed) == 0 for _, failed in rows)
assert re.search(r'Tests\s+977 passed \(977\)', records['ui-all-tests.log'])
assert re.search(r'Test Files\s+48 passed \(48\)', records['ui-all-tests.log'])
if args.git_source:
    for row in source['files']:
        data = subprocess.check_output(['git', 'show', source['source'] + ':' + row['path']], cwd=root)
        assert digest(data) == row['sha256'], row['path']
print(json.dumps({'source': source['source'], 'records': len(records), 'tests': manifest['counts'],
                  'git_source_verified': args.git_source, 'native_execution': False, 'capture': False}))
