#!/usr/bin/env python3
"""Read-only replay of subtree source, CPU results and actual generated contracts."""
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
p.add_argument('--originals', action='store_true')
p.add_argument('--binaries', action='store_true')
p.add_argument('--git-source', action='store_true')
a = p.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((D / 'manifest.json').read_bytes())
assert h(Path(__file__).read_bytes()) == m['verifier_sha256']
compressed = (D / 'records.json.gz').read_bytes()
assert h(compressed) == m['archive_sha256']
with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as z:
    raw = z.read(64 * 1024 * 1024 + 1)
assert len(raw) == m['expanded_bytes'] <= 64 * 1024 * 1024
x = json.loads(raw)
assert len(x['records']) == m['records'] and len(x['payloads']) == m['payloads']
records = {r['path']: r for r in x['records']}
assert len(records) == m['records']
payloads = {k: base64.b64decode(v, validate=True) for k, v in x['payloads'].items()}
assert all(h(v) == k for k, v in payloads.items())

def data(name):
    r = records[name]
    b = payloads[r['sha256']]
    assert len(b) == r['bytes']
    return b

def obj(name):
    return json.loads(data(name))

for name in records:
    q = Path(name)
    assert not q.is_absolute() and '..' not in q.parts
    b = data(name)
    if a.originals:
        assert (Path(m['evidence_root']) / q).read_bytes() == b, name

def git_rows(rev, rows):
    assert re.fullmatch('[0-9a-f]{40}', rev)
    names = [rev + ':' + r['path'] for r in rows]
    proc = subprocess.run(['git', 'cat-file', '--batch'], cwd=R,
                          input=('\n'.join(names) + '\n').encode(),
                          capture_output=True, check=True, timeout=30)
    stream = io.BytesIO(proc.stdout)
    for r in rows:
        header = stream.readline().decode().split()
        assert len(header) == 3 and header[1] == 'blob'
        b = stream.read(int(header[2])); assert stream.read(1) == b'\n'
        assert h(b) == r['sha256'], (rev, r['path'])
    assert not stream.read()

for n in ('001', '002', '003', '004'):
    f = obj(f'source-{n}/freeze.json')
    assert f['source'] == m['sources'][n]
    for r in f['rows']:
        assert h(data(f'source-{n}/inputs/' + r['path'])) == r['sha256']
    if a.git_source:
        git_rows(f['source'], f['rows'])

expected = {'001': [('rust-version', 0), ('command-doc', 101)],
            '002': [('rust-version', 0), ('command_remaining', 0), ('document', 0), ('headless', 101)],
            '003': [('rust-version', 0), ('headless', 0), ('cook', 0)],
            '004': [('rust-version', 0), ('clippy', 0), ('fmt', 0), ('conventions', 0), ('diff', 0), ('sdk-fresh', 0), ('sdk-types', 0)]}
results = {}
for n, commands in expected.items():
    r = obj(f'cpu-{n}/result.json'); results[n] = r
    plan = obj(f'cpu-proposal-{n}.json')
    assert r['source'] == m['sources'][n] and not r['managed_call_active']
    assert r['outcome'] == ('failure' if n in ('001', '002') else 'success')
    assert r['plan_sha256'] == h(data(f'cpu-proposal-{n}.json'))
    assert r['runner_sha256'] == h(data(f'qualify-{n}.py'))
    assert plan['freeze_sha256'] == h(data(f'source-{n}/freeze.json'))
    assert r['native_process_sha256'] == h(data('native_process.py'))
    assert [(c['name'], c['exit_code']) for c in r['commands']] == commands
    assert not any(r[k] for k in ('gpu_execution', 'window_execution', 'listener_execution', 'native_player_execution'))
    assert r['minimum_sampled_free_bytes'] >= 6 * 1024**3
    assert r['target_last_sampled_bytes'] - r['target_initial_bytes'] <= 2 * 1024**3
    assert r['elapsed_seconds'] + plan.get('prior_elapsed_seconds', 0) <= 4000
    for c in r['commands']:
        assert c['managed_returned_after_cleanup']
        for channel in ('stdout', 'stderr'):
            assert h(data(f"cpu-{n}/{c['name']}.{channel}")) == c[channel + '_sha256']
        if a.binaries:
            for b in c.get('artifacts', []):
                digest = hashlib.sha256()
                with Path(b['path']).open('rb') as file:
                    while chunk := file.read(1024 * 1024): digest.update(chunk)
                assert digest.hexdigest() == b['sha256'], b['path']

assert b'physics entities currently require a scene-root Transform' in data('cpu-001/command-doc.stdout')
assert b'left: Null' in data('cpu-002/headless.stdout')
counts = lambda name: [tuple(map(int, r)) for r in re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored', data(name).decode())]
assert counts('cpu-001/command-doc.stdout')[-1] == (3, 1, 0)
assert sum(r[0] for r in counts('cpu-001/command-doc.stdout')[:-1]) == 85
assert counts('cpu-002/command_remaining.stdout') == [(4, 0, 0), (9, 0, 0), (1, 0, 0)]
assert sum(r[0] for r in counts('cpu-002/document.stdout')) == 28
assert counts('cpu-003/headless.stdout') == [(1, 0, 0)] * 4
assert counts('cpu-003/cook.stdout') == [(1, 0, 0)] * 2
assert m['unique_passing_tests'] == {'command': 99, 'document': 28, 'headless': 4, 'cook': 2}

g = obj('generation-001/result.json')
assert g['source'] == m['sources']['003']
assert g['cpu_result_sha256'] == h(data('cpu-003/result.json'))
assert len(g['schemas']) == 78
assert set(g['changed_schemas']) == {'Command.schema.json', 'EnginePinReview.schema.json', 'ProjectFileResponse.schema.json', 'ProjectPinReview.schema.json', 'agent-tools.json'}
for r in g['schemas']:
    assert h(data('generation-001/schemas/' + r['path'])) == r['generated_sha256']
    assert (r['before_sha256'] != r['generated_sha256']) == (r['path'] in g['changed_schemas'])
for c in g['rows']:
    assert c['exit_code'] == 0 and c['managed_returned_after_cleanup']
    for channel in ('stdout', 'stderr'):
        assert h(data(f"generation-001/{c['name']}.{channel}")) == c[channel + '_sha256']
for name in g['changed_schemas']:
    assert data('source-004/inputs/schemas/' + name) == data('generation-001/schemas/' + name)
assert h(data('source-004/inputs/sdk/ts/src/generated.d.ts')) == g['sdk_sha256']
assert h(data('source-004/inputs/sdk/ts/type-tests/subtree.ts')) == g['type_control_sha256']
assert not m['phase_gate_passed'] and not m['editor_ui_implemented']
print(f"PASS {m['records']} records; 133 unique passing Rust controls, strict lint and generated contracts; two prior failures retained. No UI, GPU or phase-gate claim.")
