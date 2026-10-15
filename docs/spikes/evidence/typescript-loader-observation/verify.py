#!/usr/bin/env python3
"""Replay fixed identities and observed test counts; never run native payloads."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--external', action='store_true')
args = parser.parse_args()
directory = Path(__file__).resolve().parent
root = directory.parents[3]
manifest = json.loads((directory / 'manifest.json').read_bytes())
digest = lambda data: hashlib.sha256(data).hexdigest()
archive = (directory / 'records.json.gz').read_bytes()
assert digest(archive) == manifest['archive_sha256']
assert len(archive) == manifest['archive_bytes']
packet = json.loads(gzip.decompress(archive))
records = {row['path']: row for row in packet['records']}
assert len(records) == len(packet['records']) == manifest['records']
payloads = {key: base64.b64decode(data, validate=True) for key, data in packet['payloads'].items()}
assert all(digest(data) == key for key, data in payloads.items())

def raw(name):
    row = records[name]
    data = payloads[row['sha256']]
    assert len(data) == row['bytes']
    return data

def obj(name):
    return json.loads(raw(name))

for name in records:
    raw(name)
for path in manifest['source_paths']:
    actual = subprocess.check_output(['git', 'show', manifest['source'] + ':' + path], cwd=root)
    assert actual == raw('source/' + path), path
assert b'const DEADLINE: Duration = Duration::from_secs(8);' in raw('source/crates/incant_cook/src/typescript.rs')
assert 'native TypeScript cook exceeded its 8 second deadline' in raw('checkpoint46-failed-log.txt').decode()
assert '"parent_elapsed_us":10955531' in raw('checkpoint46-failed-log.txt').decode()
assert '"elapsed_us":311,"phase":"driver_entry"' in raw('checkpoint46-failed-log.txt').decode()
assert '# pass 23\n' in raw('node-tests.log').decode()
assert obj('node-result.json')['exit'] == 0
counts = lambda name: re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw(name).decode())
assert counts('0.stdout.log') == [('0', '0', '0')]  # Retained feature-selection miss.
assert counts('1.stdout.log') == [('3', '0', '0'), ('3', '0', '0')]
assert counts('cpu-successor-002/0.stdout.log') == [('4', '0', '0')]
assert obj('feature-selection-correction.json')['source_unchanged']
rows = [json.loads(row) for row in re.findall(r'INCANT_NATIVE_COOK_PHASES (\{[^\r\n]+\})', raw('1.stderr.log').decode())]
names = ['driver_entry', 'typescript_ready', 'swc_ready', 'compiler_ready', 'input_ready', 'snapshot_ready', 'typecheck_done', 'transform_done', 'artifact_ready', 'publish_done']
complete = [row for row in rows if row['outcome'] == 'exited_success']
assert len(complete) == 5
for row in rows:
    assert row['version'] == 2 and row['state'] == 'valid_prefix'
    assert [p['phase'] for p in row['phases']] == names[:len(row['phases'])]
    times = [p['elapsed_us'] for p in row['phases']]
    assert times == sorted(times) and all(type(t) is int and 0 <= t <= 86400000000 for t in times)
assert all(len(row['phases']) == 10 for row in complete)
for name, commands in [('qualification.json', 3), ('cpu-successor-002/qualification.json', 2)]:
    observed = obj(name)
    assert observed['status'] == 'passed' and observed['cleanup_confirmed']
    assert len(observed['commands']) == commands
    assert all(row['exit'] == 0 and row['reaped'] for row in observed['commands'])
    assert all(row['cleanup_confirmed'] for row in observed['owned_groups'])
    if args.external:
        for row in observed['artifacts']:
            data = Path(row['path']).read_bytes()
            assert len(data) == row['bytes'] and digest(data) == row['sha256']
print(json.dumps({'verified': True, 'source': manifest['source'], 'records': len(records),
                  'Node_tests': 23, 'Rust_parser_deadline_tests': 4, 'integration_tests': 6,
                  'complete_cooks': 5, 'Windows': 'pending; failure not repaired'}))
