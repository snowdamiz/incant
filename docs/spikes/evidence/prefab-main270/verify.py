#!/usr/bin/env python3
"""Read-only replay of the reviewed prefab producer and main270 qualification."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

P = argparse.ArgumentParser()
P.add_argument('--originals', action='store_true')
P.add_argument('--local-binaries', action='store_true')
P.add_argument('--current', action='store_true')
a = P.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((D / 'manifest.json').read_bytes())
assert h(Path(__file__).read_bytes()) == m['verifier_sha256']
raw = (D / 'records.json.gz').read_bytes()
assert h(raw) == m['archive_sha256']
x = json.loads(gzip.decompress(raw))
assert len(x['records']) == m['records'] and len(x['payloads']) == m['payloads']
for digest, data in x['payloads'].items():
    assert h(base64.b64decode(data, validate=True)) == digest
records = {v['path']: v for v in x['records']}
assert len(records) == len(x['records'])

def data(name):
    q = records[name]
    b = base64.b64decode(x['payloads'][q['sha256']], validate=True)
    assert len(b) == q['bytes']
    return b

def obj(name):
    return json.loads(data(name))

for q in x['records']:
    b = data(q['path'])
    if a.originals:
        assert (Path(m['evidence_root']) / q['path']).read_bytes() == b, q['path']

def git_rows(rev, rows, key='sha256'):
    names = [f"{rev}:{row['path']}" for row in rows]
    p = subprocess.run(['git', 'cat-file', '--batch'], cwd=R,
                       input=('\n'.join(names) + '\n').encode(), capture_output=True, check=True)
    stream = io.BytesIO(p.stdout)
    for row in rows:
        header = stream.readline().decode().split()
        assert len(header) == 3 and header[1] == 'blob', row['path']
        content = stream.read(int(header[2])); assert stream.read(1) == b'\n'
        assert h(content) == row[key], (rev, row['path'])
    assert stream.read() == b''

freeze = obj('freeze-001.json')
assert freeze['source'] == m['source']
git_rows(m['source'], freeze['rows'])
if a.current:
    for row in freeze['rows']:
        assert h((R / row['path']).read_bytes()) == row['sha256'], row['path']
relevance = obj('source-relevance.json')
assert relevance['source'] == m['source'] and relevance['producer'] == m['producer']
assert len(relevance['owned_exact']) == 29
assert {r['path'] for r in relevance['changed_inputs']} == {
    'crates/incant_cook/src/typescript.rs', 'crates/incant_script/src/lib.rs'}
git_rows(m['producer'], relevance['owned_exact'])
git_rows(m['source'], relevance['owned_exact'])
for r in relevance['changed_inputs']:
    git_rows(m['producer'], [r]); git_rows(m['source'], [r], 'current_sha256')
    git_rows(m['base'], [r], 'current_sha256')
assert relevance['exact_inputs'] == 3615 and relevance['producer_inputs'] == 3617
for ref in (m['producer'], m['base']):
    subprocess.run(['git', 'merge-base', '--is-ancestor', ref, m['source']], cwd=R, check=True)

# Preserve the producer's archive and its historical failures independently.
PD = R / 'docs/spikes/evidence/prefab-authoring'
pm_bytes = (PD / 'manifest.json').read_bytes()
assert h(pm_bytes) == m['producer_manifest_sha256']
pm = json.loads(pm_bytes)
assert h((PD / 'collect.py').read_bytes()) == pm['collector_sha256']
pa = (PD / pm['archive']).read_bytes(); assert h(pa) == pm['archive_sha256']
px = json.loads(gzip.decompress(pa)); assert len(px['records']) == pm['records']
assert len(px['payloads']) == pm['payloads']
for digest, value in px['payloads'].items():
    assert h(base64.b64decode(value, validate=True)) == digest
pr = {q['path']: q for q in px['records']}
def pbytes(name):
    q = pr[name]; b = base64.b64decode(px['payloads'][q['sha256']], validate=True)
    assert len(b) == q['bytes']; return b
for q in px['records']:
    b = pbytes(q['path'])
    if a.originals:
        assert (Path(m['producer_evidence_root']) / q['path']).read_bytes() == b
for c in pm['checks']:
    receipt = json.loads(base64.b64decode(px['payloads'][c['receipt_sha256']]))
    assert receipt['exit'] == c['exit'] and receipt['reaped'] and receipt['source_unchanged']
assert {c['name']: c['exit'] for c in pm['checks'] if c['exit']} == {
    'focused-001': 101, 'deletion-negative-001': 101, 'lint-001': 101}
for name in ('lean-001/checks.json', 'lean-002/checks.json'):
    rows = json.loads(pbytes(name)); assert len(rows) == 5
    assert all(c['exit'] == 0 and c['reaped'] for c in rows)
for q in px['records']:
    if q['path'].endswith('/freeze.json'):
        for row in json.loads(pbytes(q['path'])).get('source', []):
            assert pr[q['path'].removesuffix('freeze.json') + 'source/' + row['path']]['sha256'] == row['sha256']
git_rows(m['producer'], pm['final_inputs'])
assert len(pm['final_inputs']) == 3617
# The producer's broader extension-based inventory also covers root .cargo
# configuration. These identities were compared before this qualification.
exact = obj('exact-inputs.json')
assert len(exact) == 3615
compared = {q['path']: q for q in exact + relevance['changed_inputs']}
assert set(compared) == {q['path'] for q in pm['final_inputs']}
for q in pm['final_inputs']:
    row = compared[q['path']]
    assert row['sha256'] == q['sha256']
    if a.current:
        assert h((R / q['path']).read_bytes()) == row['current_sha256'], q['path']

pattern = r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored'
def counts(b):
    rows = re.findall(pattern, b.decode(errors='replace'))
    return [sum(int(row[i]) for row in rows) for i in range(3)]
assert counts(pbytes('doc-cmd-all-001/output.log')) == [87, 0, 0]
assert counts(pbytes('headless-001/output.log')) == [1, 0, 0]
assert counts(pbytes('cook-001/output.log')) == [1, 0, 0]

checks = obj('results.json')
assert [c['name'] for c in checks] == ['doc-cmd', 'headless', 'cook', 'clippy',
    'sdk-fresh', 'sdk-types', 'fmt', 'conventions', 'diff']
binaries = []
for c in checks:
    name = c['name']; assert obj(name + '.json') == c
    assert c['exit_code'] == 0 and c['source_unchanged'] and c['managed_child_tree_reaped']
    assert c['freeze_sha256'] == h(data('freeze-001.json')) and c['runner_sha256'] == h(data('run.py'))
    assert h(data(name + '.stdout')) == c['stdout_sha256']
    assert h(data(name + '.stderr')) == c['stderr_sha256']
    binaries += c['binaries']
for name, total in [('doc-cmd', 87), ('headless', 1), ('cook', 1)]:
    assert counts(data(name + '.stdout')) == [total, 0, 0]
if a.local_binaries:
    for q in binaries:
        p = Path(q['copy']); assert h(p.read_bytes()) == q['sha256']
        assert p.stat().st_size == q['size'] and p.stat().st_mode & 0o111
    for q in pm['binaries']:
        p = Path(q['path']); assert h(p.read_bytes()) == q['sha256'] and p.stat().st_mode & 0o111
assert h(data('producer-review.json')) == m['producer_review_sha256']
assert obj('producer-review.json')['accepted'] is True
print(json.dumps({'verified': True, 'current_records': m['records'], 'current_checks': len(checks),
    'current_inputs': len(freeze['rows']), 'owned_exact': 29, 'producer_records': pm['records'],
    'producer_inputs': len(pm['final_inputs']), 'current_test_cases': 89,
    'current_binary_copies': len(binaries), 'producer_binary_copies': len(pm['binaries'])}))
