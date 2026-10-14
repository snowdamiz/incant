#!/usr/bin/env python3
"""Replay the final CPU join receipt; no executable or GPU is started."""
import base64
import gzip
import hashlib
import json
from pathlib import Path


def sha(value):
    return hashlib.sha256(value).hexdigest()


here = Path(__file__).resolve().parent
manifest = json.loads((here / 'manifest.json').read_text())
archive = (here / 'records.json.gz').read_bytes()
assert sha(archive) == manifest['archive_sha256']
with gzip.open(here / 'records.json.gz', 'rb') as stream:
    payload = stream.read(16 * 1024 * 1024 + 1)
assert len(payload) <= 16 * 1024 * 1024
rows = json.loads(payload)
assert len(rows) == len(manifest['records'])
data = {}
for row, expected in zip(rows, manifest['records']):
    assert {k: row[k] for k in ('path', 'bytes', 'sha256')} == expected
    value = base64.b64decode(row['base64'], validate=True)
    assert len(value) == row['bytes'] and sha(value) == row['sha256']
    assert row['path'] not in data
    data[row['path']] = value
plan = json.loads(data['plan-001.json'])
freeze = json.loads(data['freeze.json'])
result = json.loads(data['qualify-001/result.json'])
assert freeze['head'] == plan['source'] == manifest['source']
assert plan['freeze_sha256'] == result['source_freeze_sha256'] == sha(data['freeze.json'])
assert plan['source_inventory_sha256'] == sha(data['source-inventory.json'])
assert result['plan_sha256'] == sha(data['plan-001.json'])
assert result['runner_sha256'] == sha(data['qualify-001.py'])
assert result['native_process_sha256'] == sha(data['native_process.py'])
assert result['outcome'] == 'passed' and result['error'] is None
assert result['processes_returned_after_cleanup'] and not result['gpu_execution']
assert result['minimum_sampled_free_bytes'] >= 6 * 1024 ** 3
assert [(c['name'], c['exit_code']) for c in result['commands']] == [
    ('profile-window-lib', 0), ('profile-window-clippy', 0), ('unprofiled-window-check', 0)]
for actual, planned in zip(result['commands'], plan['commands']):
    assert actual['argv'] == planned['argv']
    for stream in ('stdout', 'stderr'):
        assert sha(data[f"qualify-001/{actual['name']}.{stream}"]) == actual[stream + '_sha256']
for expected in (b'80 passed; 0 failed; 0 ignored',):
    assert expected in data['qualify-001/profile-window-lib.stdout']
assert b'release: 1.99.0' in data['qualify-001/rustc.txt']
assert b'host: aarch64-apple-darwin' in data['qualify-001/rustc.txt']
for artifact in result['commands'][0]['artifacts']:
    assert 'profile' in artifact['features'] and 'native-window' in artifact['features']
for name, original in [('cadence.rs', 'cadence.rs'), ('window.rs', 'mod.rs')]:
    record = next(row for row in freeze['files'] if row['path'] == 'crates/incant_game_player/src/window/' + original)
    assert sha(data[name]) == record['sha256']
print('PASS native host timing: 80 CPU tests, strict Clippy and profile-off compilation; no FPS acceptance')
