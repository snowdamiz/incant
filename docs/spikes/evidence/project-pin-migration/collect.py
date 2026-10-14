#!/usr/bin/env python3
"""Retain/replay the host-only pin protocol qualification, without running tests."""
import argparse
import base64
import gzip
import hashlib
import json
import pathlib
import re
import subprocess

P = argparse.ArgumentParser()
P.add_argument('--root', type=pathlib.Path, required=True)
P.add_argument('--verify-only', action='store_true')
P.add_argument('--originals', action='store_true')
P.add_argument('--local-binaries', action='store_true')
a = P.parse_args()
R = a.root.resolve()
E = pathlib.Path('/Users/andreyyurlov/.codex/incant-evidence/project-pin-migration-001')
D = R / 'docs/spikes/evidence/project-pin-migration'
M, A = D / 'manifest.json', D / 'records.json.gz'

def sha(b):
    return hashlib.sha256(b).hexdigest()

def dump(v):
    return (json.dumps(v, indent=2, sort_keys=True) + '\n').encode()

attempts = {'lib-001': 101, 'lib-002': 0, 'all-001': 0, 'lint-001': 0}
if not a.verify_only:
    records, payloads, binaries = [], {}, []
    for p in sorted(E.rglob('*')):
        if not p.is_file() or 'binaries' in p.parts or p.name == 'diagnose':
            continue
        b = p.read_bytes()
        h = sha(b)
        records.append({'path': str(p.relative_to(E)), 'sha256': h, 'bytes': len(b)})
        payloads[h] = base64.b64encode(b).decode()
    for name in attempts:
        binaries.extend(json.loads((E/name/'receipt.json').read_text())['binaries'])
    p = E/'crdt-diagnosis-003/diagnose'
    binaries.append({'path': str(p), 'sha256': sha(p.read_bytes()), 'bytes': p.stat().st_size, 'kind': 'diagnostic-only'})
    paths = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=R, text=True).splitlines()
    inputs = [{'path': p, 'sha256': sha((R/p).read_bytes())} for p in paths if (R/p).is_file() and (p.endswith(('.rs', '.toml', '.lock', '.wgsl')) or p == 'tools/cargo')]
    A.write_bytes(gzip.compress(dump({'records': records, 'payloads': payloads}), mtime=0))
    manifest = {'version': 1, 'base': '730a7e4523b81de29fff2fe914b567a556dfc859', 'collector_sha256': sha(pathlib.Path(__file__).read_bytes()), 'archive_sha256': sha(A.read_bytes()), 'records': len(records), 'payloads': len(payloads), 'final_inputs': inputs, 'compiled_source_freeze': 'source-003/freeze.json', 'rust_attempts': attempts, 'binaries': binaries, 'scope': 'Qualified host-only exact-current legacy pin file protocol. 87 Mac native tests, strict lint, three lean checks. Original 19-pass/1-fail Abort encoding mismatch retained. No application endpoint, older-format conversion, GPU, timing or phase-gate acceptance.'}
    M.write_bytes(dump(manifest))
m = json.loads(M.read_bytes())
assert sha(pathlib.Path(__file__).read_bytes()) == m['collector_sha256']
assert sha(A.read_bytes()) == m['archive_sha256']
b = json.loads(gzip.decompress(A.read_bytes()))
assert len(b['records']) == m['records'] and len(b['payloads']) == m['payloads']
payloads = {h: base64.b64decode(v, validate=True) for h, v in b['payloads'].items()}
assert all(sha(v) == h for h, v in payloads.items())
rows = {r['path']: r for r in b['records']}
assert len(rows) == m['records']
for name, row in rows.items():
    assert len(payloads[row['sha256']]) == row['bytes']
    if a.originals:
        assert (E/name).read_bytes() == payloads[row['sha256']], name

def data(name):
    return payloads[rows[name]['sha256']]

def load(name):
    return json.loads(data(name))

for row in m['final_inputs']:
    assert sha((R/row['path']).read_bytes()) == row['sha256'], row['path']
for generation in ['source-001', 'source-002', 'source-003']:
    f = load(generation+'/freeze.json')
    for row in f['files']:
        assert rows[generation+'/inputs/'+row['path']]['sha256'] == row['sha256']
        if generation == 'source-003' and not row['path'].startswith('docs/'):
            assert sha((R/row['path']).read_bytes()) == row['sha256']
    assert rows[generation+'/tracked.patch']['sha256'] == f['tracked_patch_sha256']
for name, expected in attempts.items():
    r = load(name+'/receipt.json')
    assert r['exit_code'] == expected and r['source_unchanged']
    assert r.get('owned_process_group_cleaned_and_reaped') or (r['reaped'] and not r['remaining_group'])
    assert rows[name+'/output.log']['sha256'] == r['log_sha256']
    assert rows[name+'/freeze.json']['sha256'] == r['freeze_sha256']
    f = load(name+'/freeze.json')
    assert rows[f['source_freeze'].removeprefix(str(E)+'/')]['sha256'] == r['source_freeze_sha256']
for name, count in [('lib-002', 36), ('all-001', 87)]:
    summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', data(name+'/output.log').decode())
    assert [sum(int(x[i]) for x in summaries) for i in range(3)] == [count, 0, 0]
failed = data('lib-001/output.log')
assert b'19 passed; 1 failed' in failed and b'project abort projection mismatch' in failed
lean = load('lean-001/checks.json')
assert len(lean) == 3 and all(x['exit_code'] == 0 and x['owned_process_group_cleaned_and_reaped'] for x in lean)
for row in lean:
    assert rows['lean-001/'+row['name']+'.log']['sha256'] == row['log_sha256']
assert load('crdt-diagnosis-002/receipt.json')['build_exit'] == 1
assert load('crdt-diagnosis-003/receipt.json')['build_exit'] == load('crdt-diagnosis-003/receipt.json')['run_exit'] == 0
assert b'observed_mismatches=14/64' in data('crdt-diagnosis-003/run.log')
assert data('crdt-diagnosis-003/durable.loro') != data('crdt-diagnosis-003/imported.loro')
assert data('crdt-diagnosis-003/imported.loro') == data('crdt-diagnosis-003/after-stage.loro')
assert load('abort-diagnosis-001/receipt.json')['exact_equal'] is True
for row in m['binaries']:
    if a.local_binaries:
        p = pathlib.Path(row['path'])
        assert sha(p.read_bytes()) == row['sha256'] and p.stat().st_mode & 0o111
release = load('release.json')
assert release['builder_slot_released'] and not release['gpu_executed'] and not release['application_endpoint_added']
assert all(x['reaped'] and x['exit_code'] == attempts[x['name']] for x in release['rust_attempts'])
print(json.dumps({'verified': True, 'records': m['records'], 'payloads': m['payloads'], 'inputs': len(m['final_inputs']), 'binaries': len(m['binaries']), 'full_tests': 87, 'historical_failure_preserved': True, 'application_endpoint': False}))
