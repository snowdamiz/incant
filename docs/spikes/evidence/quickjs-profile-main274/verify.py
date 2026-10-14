#!/usr/bin/env python3
"""Read-only current/producer evidence replay; no builds or binary execution."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--verify-only', action='store_true')
p.add_argument('--current', action='store_true')
p.add_argument('--originals', action='store_true')
p.add_argument('--local-binaries', action='store_true')
a = p.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((D / 'manifest.json').read_bytes())
assert h(Path(__file__).read_bytes()) == m['verifier_sha256']
assert h((D / 'collect.py').read_bytes()) == m['collector_sha256']
raw = (D / 'records.json.gz').read_bytes()
assert h(raw) == m['archive_sha256']
archive = json.loads(gzip.decompress(raw))
payloads = {k: base64.b64decode(v, validate=True) for k, v in archive['payloads'].items()}
assert len(payloads) == m['payloads'] and all(h(v) == k for k, v in payloads.items())
x = {}
for row in archive['records']:
    assert row['path'] not in x
    data = payloads[row['sha256']]
    assert len(data) == row['bytes']
    if a.originals:
        assert (Path(m['evidence_root']) / row['path']).read_bytes() == data, row['path']
    x[row['path']] = data
assert len(x) == m['records']
obj = lambda name: json.loads(x[name])


def git_rows(source, rows, key='sha256'):
    result = subprocess.run(['git', 'cat-file', '--batch'], cwd=R, check=True, capture_output=True,
        input=''.join(source + ':' + row['path'] + '\n' for row in rows).encode())
    stream = io.BytesIO(result.stdout)
    for row in rows:
        header = stream.readline().split()
        assert len(header) == 3 and header[1] == b'blob', row['path']
        blob = stream.read(int(header[2]))
        assert stream.read(1) == b'\n' and h(blob) == row[key], (source, row['path'])
    assert not stream.read()


f = obj('freeze-001.json')
assert f['source'] == m['source'] and f['base'] == m['base'] and f['producer'] == m['producer']
assert len(f['rows']) == m['inputs'] == 3984
git_rows(m['source'], f['rows'])
if a.current:
    for row in f['rows']:
        assert h((R / row['path']).read_bytes()) == row['sha256'], row['path']
subprocess.run(['git', 'merge-base', '--is-ancestor', m['base'], m['source']], cwd=R, check=True)
scope = obj('source-relevance.json')
assert (scope['source'], scope['base'], scope['producer']) == (m['source'], m['base'], m['producer'])
assert len(scope['owned']) == 418
git_rows(m['source'], scope['owned'], 'current_sha256')
git_rows(m['producer'], scope['owned'], 'producer_sha256')
assert {q['path'] for q in scope['owned'] if q['current_sha256'] != q['producer_sha256']} == {'.gitattributes'}
assert all(q['producer_exact'] == (q['current_sha256'] == q['producer_sha256']) for q in scope['owned'])
paths = lambda left, right: set(subprocess.check_output(['git', 'diff', '--name-only', left, right], cwd=R).decode().splitlines())
assert paths(m['base'], m['source']) == {q['path'] for q in scope['owned']}
assert paths(scope['producer_base'], m['base']) == set(scope['incoming_main_paths'])
assert len(scope['incoming_main_paths']) == 44
for source in [m['base'], m['producer']]:
    old = subprocess.check_output(['git', 'show', source + ':.gitattributes'], cwd=R).splitlines()
    joined = subprocess.check_output(['git', 'show', m['source'] + ':.gitattributes'], cwd=R).splitlines()
    assert set(old) <= set(joined)

plan = obj('qualification-plan.json')
checks = obj('results.json')
assert len(checks) == len(plan['commands']) == 13
binaries = []
for c, (name, argv, timeout) in zip(checks, plan['commands']):
    assert c == obj(name + '.json')
    assert (c['name'], c['argv'], c['timeout_seconds']) == (name, argv, timeout)
    assert c['exit_code'] == 0 and c['source_unchanged'] and c['managed_child_tree_reaped']
    assert c['runner_sha256'] == h(x['run.py']) and c['freeze_sha256'] == h(x['freeze-001.json'])
    assert c['stdout_sha256'] == h(x[name + '.stdout']) and c['stderr_sha256'] == h(x[name + '.stderr'])
    assert c['environment']['CARGO_BUILD_JOBS'] == c['environment']['RAYON_NUM_THREADS'] == '2'
    assert c['environment']['CARGO_TARGET_DIR'] == '/tmp/incant-agent-font-target'
    binaries += c['binaries']
counts = lambda name: [sum(int(row[i]) for row in re.findall(
    r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', x[name + '.stdout'].decode())) for i in range(3)]
assert counts('profile-off') == [2, 0, 0]
assert counts('profile-summary') == [11, 0, 0]
for mode in ['physics-deterministic', 'physics-fast-simd8']:
    assert counts('fixtures-' + mode) == [2, 0, 0]
assert b'Ran 21 tests' in x['python-consumers.stderr'] and b'\nOK\n' in x['python-consumers.stderr']
assert b'Ran 10 tests' in x['probe-locks.stderr'] and b'\nOK\n' in x['probe-locks.stderr']
assert obj('vendor-audit.stdout') == {'bundled_abis': 20, 'execution': False, 'patched_files': 26, 'performance_claim': False, 'upstream_files': 383}
assert len(binaries) == 13
if a.local_binaries:
    for binary in binaries:
        path = Path(binary['copy'])
        assert path.stat().st_size == binary['size'] and path.stat().st_mode & 0o111
        with path.open('rb') as stream:
            assert hashlib.file_digest(stream, 'sha256').hexdigest() == binary['sha256']
release = obj('release.json')
assert release['exit_code'] == 0 and release['supervisors_absent'] and release['target_released']
assert release['managed_calls_returned_after_cleanup'] and not release['gpu_used']
assert h(x['producer-review.json']) == m['producer_review_sha256'] == '0637a7747f030ec4f4bc0f58a4853cf62c3f2115ba82eba2e392d6bb16d9f9dc'
assert obj('source-approval.json')['source'] == m['source']
prior = obj('producer-replay.json')
assert prior['exit_code'] == 0 and prior['terminal_reaped']
assert prior['stdout_sha256'] == h(x['producer-replay.log'])
assert obj('producer-replay.log')['pass']
producer = D.parent / 'quickjs-maintained-profile'
for name, digest in m['producer_files'].items():
    assert h((producer / name).read_bytes()) == digest, name
command = [sys.executable, str(producer / 'verify.py'), '--git-source']
if a.originals or a.local_binaries:
    command.append('--external')
result = subprocess.run(command, cwd=R, check=True, capture_output=True)
verified = json.loads(result.stdout)
assert verified['pass'] and verified['records'] == 3325 and verified['executables'] == 54
print(json.dumps({'verified': True, 'current_records': len(x), 'current_inputs': len(f['rows']),
    'current_checks': len(checks), 'current_rust_cases': 17, 'python_controls': 31,
    'current_copies': len(binaries), 'producer_records': verified['records'],
    'producer_copies': verified['executables'], 'performance_claim': False}))
