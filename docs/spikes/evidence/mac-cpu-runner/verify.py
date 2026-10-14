#!/usr/bin/env python3
"""Verify retained runner qualification without executing captured commands."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
import mac_performance

manifest = json.loads((HERE / 'manifest.json').read_text())
archive = (HERE / 'records.json.gz').read_bytes()
assert hashlib.sha256(archive).hexdigest() == manifest['archive_sha256']
records = {name: base64.b64decode(value, validate=True)
           for name, value in json.loads(gzip.decompress(archive)).items()}
assert records.keys() == manifest['records'].keys()
for name, data in records.items():
    expected = manifest['records'][name]
    assert len(data) == expected['bytes']
    assert hashlib.sha256(data).hexdigest() == expected['sha256']
    if name.startswith('source/'):
        assert subprocess.check_output(
            ['git', 'show', manifest['source'] + ':' + name.removeprefix('source/')], cwd=ROOT) == data
for index in range(3):
    receipt = json.loads(records[f'command-{index}.json'])
    assert receipt['exit'] == 0
    assert receipt['log_sha256'] == hashlib.sha256(records[f'command-{index}.log']).hexdigest()
assert b'Ran 8 tests' in records['command-0.log'] and records['command-0.log'].endswith(b'OK\n')
assert mac_performance.metrics(json.loads(records['player-report.json']), 3) == json.loads(records['derived-metrics.json'])
assert not manifest['shipping_measurement'] and not manifest['performance_gate']
print(f"Verified {len(records)} records, eight runner tests and actual CI-profile report parsing; no performance claim")
