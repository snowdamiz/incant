#!/usr/bin/env python3
"""Read-only current/producer replay; no builds or executable test runs."""
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
import tempfile

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


def git_rows(source, rows, key='sha256', destination=None):
    result = subprocess.run(['git', 'cat-file', '--batch'], cwd=R, check=True, capture_output=True,
        input=''.join(source + ':' + row['path'] + '\n' for row in rows).encode())
    stream = io.BytesIO(result.stdout)
    for row in rows:
        header = stream.readline().split()
        assert len(header) == 3 and header[1] == b'blob', row['path']
        blob = stream.read(int(header[2]))
        assert stream.read(1) == b'\n' and h(blob) == row[key], (source, row['path'])
        if destination is not None:
            relative = Path(row['path'])
            assert not relative.is_absolute() and '..' not in relative.parts
            path = destination / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(blob)
    assert not stream.read()


paths = lambda left, right: set(subprocess.check_output(['git', 'diff', '--name-only', left, right], cwd=R).decode().splitlines())
f = obj('freeze-source-001.json')
assert (f['source'], f['base'], f['producer']) == (m['source'], m['base'], m['producer'])
assert len(f['rows']) == m['inputs'] == 4273
git_rows(m['source'], f['rows'])
subprocess.run(['git', 'merge-base', '--is-ancestor', m['base'], m['source']], cwd=R, check=True)
scope = obj('source-relevance.json')
assert (scope['source'], scope['base'], scope['producer']) == (m['source'], m['base'], m['producer'])
assert len(scope['owned']) == 43 and len(scope['producer_input_changes']) == 14
assert scope['producer_inputs'] == 3584
assert paths(m['base'], m['source']) == {q['path'] for q in scope['owned']}
assert paths(scope['producer_base'], m['base']) == set(scope['incoming_main_paths'])
git_rows(m['source'], scope['owned'], 'current_sha256')
git_rows(m['producer'], scope['owned'], 'producer_sha256')
assert {q['path'] for q in scope['owned'] if q['current_sha256'] != q['producer_sha256']} == {'STATUS.md'}
for source in [m['source'], m['base']]:
    git_rows(source, scope['producer_input_changes'], 'current_sha256')
git_rows(m['producer'], scope['producer_input_changes'], 'producer_sha256')
vendor = [row for row in f['rows'] if row['path'].startswith('third_party/rquickjs')]
assert len(vendor) == 391
git_rows(m['base'], vendor)

commands, checks = obj('commands.json'), obj('results.json')
assert len(checks) == len(commands) == 14
binaries = []
assets = obj('editor-assets.json')
assert len(assets) == 19
for row in assets:
    assert h(x['editor-assets/' + row['path']]) == row['sha256']
    assert len(x['editor-assets/' + row['path']]) == row['bytes']
for c, (name, argv, timeout) in zip(checks, commands):
    assert c == obj(name + '.json')
    assert (c['name'], c['argv'], c['timeout_seconds']) == (name, argv, timeout)
    assert c['exit_code'] == 0 and c['source_unchanged'] and c['managed_child_tree_reaped']
    assert c['runner_sha256'] == h(x['run.py'] if name in {v[0] for v in commands[:7]} else x['continue-static-002.py'])
    assert c['freeze_sha256'] == h(x['freeze-source-001.json'])
    assert c['command_plan_sha256'] == h(x['commands.json'])
    assert c['stdout_sha256'] == h(x[name + '.stdout']) and c['stderr_sha256'] == h(x[name + '.stderr'])
    assert c['environment']['CARGO_BUILD_JOBS'] == c['environment']['RAYON_NUM_THREADS'] == '2'
    assert c['environment']['RUSTUP_TOOLCHAIN'] == '1.99.0'
    assert c['environment']['CARGO_TARGET_DIR'] == '/tmp/incant-agent-font-target'
    if name != 'bundle':
        assert c['editor_assets_sha256'] == h(x['editor-assets.json'])
    binaries += c['binaries']
counts = lambda name: [sum(int(row[i]) for row in re.findall(
    r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', x[name + '.stdout'].decode())) for i in range(3)]
assert counts('source-cmd') == [107, 0, 0]
assert counts('headless') == [39, 0, 0]
assert counts('editor') == [4, 0, 0]
assert b'189 passed' in x['bridge-tests.stdout']
schemas = obj('schema-comparison.json')
assert schemas['all_equal'] and len(schemas['rows']) == 78
assert schemas['checker_sha256'] == h(x['compare-schemas-002.py'])
assert obj('supervisor-failure-001.json')['exit_code'] == 1
assert not obj('supervisor-failure-001.json')['source_changed']
assert obj('initial-completion.json')['all_14_command_exits_zero']
for row in schemas['rows']:
    assert h(x['generated-schemas/' + Path(row['path']).name]) == row['sha256']
git_rows(m['source'], schemas['rows'])
headless = next(b for b in binaries if Path(b['copy']).name == 'headless-incant_headless')
assert schemas['binary_copy_sha256'] == headless['sha256']
if a.local_binaries:
    for binary in binaries:
        path = Path(binary['copy'])
        assert path.stat().st_size == binary['size'] and path.stat().st_mode & 0o111
        with path.open('rb') as stream:
            assert hashlib.file_digest(stream, 'sha256').hexdigest() == binary['sha256']
# A separate test-only successor leaves the preceding production executions intact.
S = 'test-isolation-002/'
sf = obj(S + 'freeze.json')
assert sf['base'] == m['source'] and sf['source'] == m['successor'] and len(sf['rows']) == 4274
git_rows(sf['source'], sf['rows'])
if a.current:
    for row in sf['rows']:
        assert h((R / row['path']).read_bytes()) == row['sha256'], row['path']
changed = paths(m['source'], sf['source'])
expected_changed = {'crates/incant_cmd/src/' + n for n in [
    'lib.rs', 'test_isolation.rs', 'agent_budget_replay_tests.rs',
    'project_file_tests.rs', 'project_checkpoint_tests.rs', 'source_protocol_tests.rs']} | {'crates/incant_cmd/tests/prefabs.rs'}
assert changed == expected_changed
patch = subprocess.check_output(['git', 'diff', m['source'], sf['source'], '--', 'crates/incant_cmd'], cwd=R)
assert patch == x[S + 'repair.patch']
for module in ['project_file_tests', 'project_checkpoint_tests', 'source_protocol_tests']:
    path = 'crates/incant_cmd/src/' + module + '.rs'
    old = subprocess.check_output(['git', 'show', m['source'] + ':' + path], cwd=R).decode()
    new = subprocess.check_output(['git', 'show', sf['source'] + ':' + path], cwd=R).decode()
    restored = re.sub(r'\n    if crate::test_isolation::isolated\(\s*"[^"]+",?\s*\) \{\n        return;\n    \}', '', new)
    assert restored == old, path
inventory = obj(S + 'test-inventory.json')
assert len(inventory['all_unit_cases']) == 29 and inventory['parent_count'] == 26
assert inventory['intentional_crash_entrypoints'] == 3 and not inventory['production_change'] and not inventory['retry']
sc = obj(S + 'commands.json')
sr = obj(S + 'results.json')
assert len(sc) == len(sr) == 5
for c, (name, argv, timeout) in zip(sr, sc):
    assert c == obj(S + name + '.json')
    assert (c['name'], c['argv'], c['timeout_seconds']) == (name, argv, timeout)
    assert c['exit_code'] == 0 and c['source_unchanged'] and c['managed_child_tree_reaped']
    assert c['runner_sha256'] == h(x[S + 'run.py']) and c['freeze_sha256'] == h(x[S + 'freeze.json'])
    assert c['command_plan_sha256'] == h(x[S + 'commands.json'])
    assert c['editor_assets_sha256'] == h(x['editor-assets.json'])
    assert c['stdout_sha256'] == h(x[S + name + '.stdout']) and c['stderr_sha256'] == h(x[S + name + '.stderr'])
    assert c['environment']['CARGO_BUILD_JOBS'] == c['environment']['RAYON_NUM_THREADS'] == '2'
    binaries += c['binaries']
assert counts(S + 'cmd-lib-prefabs-parallel') == [39, 0, 0]
assert not any('--test-threads' in arg for arg in sc[0][1])
if a.local_binaries:
    for c in sr:
        for binary in c['binaries']:
            path = Path(binary['copy'])
            assert path.stat().st_size == binary['size'] and path.stat().st_mode & 0o111
            with path.open('rb') as stream:
                assert hashlib.file_digest(stream, 'sha256').hexdigest() == binary['sha256']
failure = obj('checkpoint34-linux-failure/diagnosis-001.json')
assert failure['job_id'] == 115743846285 and failure['failed'] == 1
assert failure['log_sha256'] == h(x['checkpoint34-linux-failure/log-raw.stdout'])
assert b'20 passed; 1 failed' in x['checkpoint34-linux-failure/log-raw.stdout']
assert b'WouldBlock' in x['checkpoint34-linux-failure/log-raw.stdout']
assert b'9 passed; 1 failed' in x['checkpoint34-windows-failure/log-raw.stdout']
assert b'code: 33' in x['checkpoint34-windows-failure/log-raw.stdout']
assert h(x[S + 'isolation-review.json']) == '151acda9cf58ad2ce1562281bf51358d8ac0669304a3d5ad72633cee1cf8200f'

release = obj('release.json')
assert release['exit_code'] == 0 and release['supervisors_absent'] and release['target_released']
assert release['managed_calls_returned_after_cleanup'] and not release['gpu_used']
assert h(x['producer-review.json']) == '42d1f532271cb3ee166faac890b610291e0ce2ef1d2033fbaf1efd5011c13489'
assert h(x['source-review.json']) == 'e0a14062be32a92fe25a446d443a877f83711a8890b449163826f2e8c55bc9d7'
assert obj('source-review.json')['source'] == m['source']
assert obj('source-review.json')['freeze_sha256'] == h(x['freeze-source-001.json'])
prior = obj('producer-replay.json')
assert prior['exit_code'] == 0 and prior['terminal_reaped']
assert prior['stdout_sha256'] == h(x['producer-replay.log'])
assert obj('producer-replay.log')['verified']
producer = D.parent / 'project-file-application'
for name, digest in m['producer_files'].items():
    assert h((producer / name).read_bytes()) == digest, name
material = obj('producer-materialization.json')
assert material['git_source'] == m['producer'] and len(material['rows']) == 3662
ancestry = obj('producer-ancestry.json')
assert ancestry['producer'] == m['producer'] and ancestry['before'] == m['source']
assert ancestry['parents'] == [m['source'], m['producer']]
actual = subprocess.check_output(['git', 'show', '-s', '--format=%T %P', ancestry['merge']], cwd=R).decode().split()
assert actual == [ancestry['tree']] + ancestry['parents']
assert subprocess.check_output(['git', 'rev-parse', m['source'] + '^{tree}'], cwd=R).decode().strip() == ancestry['tree']
assert subprocess.check_output(['git', 'show', '-s', '--format=%B', ancestry['merge']], cwd=R).decode().rstrip().endswith('Built-by: astra')
if a.current:
    subprocess.run(['git', 'merge-base', '--is-ancestor', ancestry['merge'], 'HEAD'], cwd=R, check=True)
with tempfile.TemporaryDirectory(prefix='incant-application-producer-') as directory:
    root = Path(directory)
    git_rows(m['producer'], material['rows'], destination=root)
    command = [sys.executable, str(root / 'docs/spikes/evidence/project-file-application/collect.py'),
               '--root', str(root), '--verify-only']
    if a.originals:
        command.append('--originals')
    if a.local_binaries:
        command.append('--local-binaries')
    result = subprocess.run(command, cwd=root, check=True, capture_output=True)
    verified = json.loads(result.stdout)
    assert verified['verified'] and verified['records'] == 517 and verified['binary_copies'] == 80
print(json.dumps({'verified': True, 'current_records': len(x), 'current_inputs': len(f['rows']),
    'current_checks': len(checks) + len(sr), 'successor_parallel_cases': 39, 'current_rust_cases': 150, 'bridge_cases': 189,
    'current_copies': len(binaries), 'schemas': 78, 'editor_assets': len(assets),
    'producer_records': verified['records'], 'producer_copies': verified['binary_copies'],
    'performance_claim': False}))
