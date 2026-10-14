#!/usr/bin/env python3
"""Read-only replay of engine compatibility/prefab composition and producer evidence."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--current', action='store_true')
p.add_argument('--originals', action='store_true')
p.add_argument('--local-binaries', action='store_true')
a = p.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((D / 'manifest.json').read_bytes())
assert h(Path(__file__).read_bytes()) == m['verifier_sha256']

def archive(directory, manifest, originals):
    raw = (directory / 'records.json.gz').read_bytes()
    assert h(raw) == manifest['archive_sha256']
    x = json.loads(gzip.decompress(raw))
    assert len(x['records']) == manifest['records']
    assert len(x['payloads']) == manifest['payloads']
    for digest, value in x['payloads'].items():
        assert h(base64.b64decode(value, validate=True)) == digest
    records = {}
    for row in x['records']:
        assert row['path'] not in records
        b = base64.b64decode(x['payloads'][row['sha256']], validate=True)
        assert len(b) == row['bytes']
        if a.originals:
            assert (Path(originals) / row['path']).read_bytes() == b, row['path']
        records[row['path']] = b
    return records

def git_rows(rev, rows, key='sha256'):
    command = ['git', 'cat-file', '--batch']
    result = subprocess.run(command, cwd=R, check=True, capture_output=True,
        input=''.join(rev + ':' + q['path'] + '\n' for q in rows).encode())
    stream = io.BytesIO(result.stdout)
    for row in rows:
        header = stream.readline().split()
        assert len(header) == 3 and header[1] == b'blob', row['path']
        data = stream.read(int(header[2])); assert stream.read(1) == b'\n'
        assert h(data) == row[key], (rev, row['path'])
    assert stream.read() == b''

x = archive(D, m, m['evidence_root'])
obj = lambda name: json.loads(x[name])
f = obj('freeze-001.json')
assert f['source'] == m['source']
assert len(f['rows']) == 4004
git_rows(m['source'], f['rows'])
if a.current:
    for row in f['rows']:
        assert h((R / row['path']).read_bytes()) == row['sha256'], row['path']
for ref in [m['base'], m['producer']]:
    subprocess.run(['git', 'merge-base', '--is-ancestor', ref, m['source']], cwd=R, check=True)
relevance = obj('source-relevance.json')
assert relevance['source_head'] == m['source'] and relevance['producer'] == m['producer']
assert len(relevance['owned']) == 24 and sum(q['producer_exact'] for q in relevance['owned']) == 16
git_rows(m['producer'], relevance['owned'], 'producer_sha256')
git_rows(m['source'], relevance['owned'], 'current_sha256')
git_rows(m['source'], relevance['extra'])
git_rows(m['producer'], relevance['producer_inputs'])
git_rows(m['source'], relevance['producer_inputs'], 'current_sha256')
assert len(relevance['producer_inputs']) == 3615
changed = {q['path'] for q in relevance['producer_inputs'] if not q['producer_exact']}
assert changed == {
    'schemas/EnginePinReview.schema.json', 'crates/incant_cmd/src/lib.rs',
    'crates/incant_cook/src/typescript.rs', 'crates/incant_cook/tests/physics_2d.rs',
    'crates/incant_cook/tests/runtime.rs', 'crates/incant_doc/src/audio.rs',
    'crates/incant_doc/src/lib.rs', 'crates/incant_doc/src/navigation.rs',
    'schemas/Command.schema.json', 'schemas/GameSave.schema.json',
    'schemas/Project.schema.json', 'schemas/agent-tools.json', 'sdk/ts/src/generated.d.ts'}
# Unmodified main correctness changes remain exact; other differences are the
# explicitly reviewed command/Project/schema joins, not producer execution claims.
unchanged_main = {'crates/incant_cook/src/typescript.rs', 'crates/incant_cook/tests/physics_2d.rs',
    'crates/incant_cook/tests/runtime.rs', 'crates/incant_doc/src/audio.rs', 'crates/incant_doc/src/navigation.rs'}
git_rows(m['base'], [q for q in relevance['producer_inputs'] if q['path'] in unchanged_main], 'current_sha256')

PD = R / 'docs/spikes/evidence/project-engine-compatibility'
pm_raw = (PD / 'manifest.json').read_bytes()
assert h(pm_raw) == m['producer_manifest_sha256']
pm = json.loads(pm_raw)
assert h((PD / 'collect.py').read_bytes()) == pm['collector_sha256']
y = archive(PD, pm, m['producer_evidence_root'])
assert len(pm['final_inputs']) == 3615
git_rows(m['producer'], pm['final_inputs'])
assert {q['name']: q['exit'] for q in pm['checks'] if q['exit']} == {'focused-001': 101, 'headless-001': 101}
for c in pm['checks']:
    r = json.loads(y[c['name'] + '/receipt.json'])
    assert h(y[c['name'] + '/receipt.json']) == c['receipt_sha256']
    assert r['exit'] == c['exit'] and r['reaped'] and r['source_unchanged']
    assert r['log_sha256'] == h(y[c['name'] + '/output.log'])
for name, data in y.items():
    if name.endswith('/freeze.json'):
        for row in json.loads(data).get('source', []):
            key = name.removesuffix('freeze.json') + ('copies/' if name.startswith('source-final/') else 'source/') + row['path']
            assert h(y[key]) == row['sha256'], key
lean = json.loads(y['lean-001/checks.json'])
assert len(lean) == 5 and all(q['exit'] == 0 and q['reaped'] for q in lean)
release = json.loads(y['release.json'])
assert release['builder_slot_released'] and not release['gpu_executed']
assert len(release['rust_attempts']) == 7 and all(q['reaped'] and q['group_absent'] for q in release['rust_attempts'])
original_schema = json.loads(y['schema-generation.json'])
assert original_schema['exit'] == 0 and len(original_schema['schemas']) == 71
git_rows(m['producer'], [{'path': 'schemas/' + name, 'sha256': digest} for name, digest in original_schema['schemas'].items()])

def counts(b):
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', b.decode(errors='replace'))
    return [sum(int(q[i]) for q in rows) for i in range(3)]
assert counts(y['doc-cmd-all-001/output.log']) == [80, 0, 0]
assert counts(y['headless-002/output.log']) == [1, 0, 0]
assert counts(y['cook-001/output.log']) == [1, 0, 0]

# The schema generator executable was copied before any subsequent compiler.
# Its Rust/build closure is exact to final tests; only generated data changed.
schema_build = obj('schema-build.json')
assert schema_build == obj('schema-build-results.json')[0]
assert schema_build['exit_code'] == 0 and schema_build['source_unchanged'] and schema_build['managed_child_tree_reaped']
assert schema_build['runner_sha256'] == h(x['schema-build.py'])
assert schema_build['freeze_sha256'] == h(x['freeze-source.json'])
assert schema_build['stdout_sha256'] == h(x['schema-build.stdout'])
assert schema_build['stderr_sha256'] == h(x['schema-build.stderr'])
sf = obj('freeze-source.json'); git_rows(sf['source'], sf['rows'])
final_rows = {q['path']: q['sha256'] for q in f['rows']}
assert {q['path'] for q in sf['rows'] if q['sha256'] != final_rows[q['path']]} == {
    'schemas/Command.schema.json', 'schemas/EnginePinReview.schema.json',
    'schemas/agent-tools.json', 'sdk/ts/src/generated.d.ts'}
sg = obj('schema-generate.json')
assert sg['exit_code'] == 0 and sg['reaped'] and len(sg['rows']) == 74
assert sg['binary_sha256'] == schema_build['binaries'][0]['sha256']
assert sg['stdout_sha256'] == h(x['schema-generate.stdout']) and sg['stderr_sha256'] == h(x['schema-generate.stderr'])
git_rows(m['source'], sg['rows'])
if a.current:
    for q in sg['rows']: assert h((R / q['path']).read_bytes()) == q['sha256']

checks = obj('results.json')
assert [q['name'] for q in checks] == ['doc-cmd', 'headless', 'cook', 'clippy', 'sdk-fresh', 'sdk-types', 'fmt', 'conventions', 'diff']
binaries = schema_build['binaries'][:]
for c in checks:
    name = c['name']; assert obj(name + '.json') == c
    assert c['exit_code'] == 0 and c['source_unchanged'] and c['managed_child_tree_reaped']
    assert c['runner_sha256'] == h(x['run.py']) and c['freeze_sha256'] == h(x['freeze-001.json'])
    assert c['stdout_sha256'] == h(x[name + '.stdout']) and c['stderr_sha256'] == h(x[name + '.stderr'])
    binaries += c['binaries']
assert m['current_test_counts'] == {'doc-cmd': 96, 'headless': 2, 'cook': 2}
for name, total in m['current_test_counts'].items():
    assert counts(x[name + '.stdout']) == [total, 0, 0]
assert h(x['producer-review.json']) == m['producer_review_sha256']
assert obj('producer-review.json')['status'] == 'accepted-scoped-source-and-producer-evidence'
assert obj('producer-verify.json')['exit_code'] == 0
if a.local_binaries:
    for q in binaries:
        p = Path(q['copy']); assert h(p.read_bytes()) == q['sha256']
        assert p.stat().st_size == q['size'] and p.stat().st_mode & 0o111
    for q in pm['binaries']:
        p = Path(q['path']); assert h(p.read_bytes()) == q['sha256'] and p.stat().st_mode & 0o111
print(json.dumps({'verified': True, 'current_records': m['records'], 'current_inputs': len(f['rows']),
    'qualification_checks': len(checks), 'current_tests': m['current_test_counts'], 'schemas': len(sg['rows']),
    'current_binary_copies': len(binaries), 'producer_records': pm['records'], 'producer_copies': len(pm['binaries'])}))
