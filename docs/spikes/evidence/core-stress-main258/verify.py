#!/usr/bin/env python3
"""Read-only current-main helper qualification and original evidence replay."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--verify-only', action='store_true', required=True)
    p.add_argument('--originals', action='store_true')
    p.add_argument('--external', action='store_true')
    args = p.parse_args()
    m = json.loads((HERE / 'manifest.json').read_text())
    raw = (HERE / 'records.json.gz').read_bytes()
    assert digest(raw) == m['archive_sha256']
    stored = json.loads(gzip.decompress(raw))
    payloads = {key: base64.b64decode(value, validate=True) for key, value in stored['payloads'].items()}
    assert all(digest(value) == key for key, value in payloads.items())
    records = {}
    for row in m['records']:
        value = payloads[row['sha256']]
        assert len(value) == row['bytes']
        if args.originals:
            assert (Path(m['raw_root']) / row['path']).read_bytes() == value
        records[row['path']] = value
    obj = lambda name: json.loads(records[name])
    source = m['source']
    inputs = obj('inputs.json')
    assert inputs['source'] == source
    for row in inputs['files']:
        assert digest(git('show', source + ':' + row['path'])) == row['sha256'], row['path']
        assert digest((ROOT / row['path']).read_bytes()) == row['sha256'], row['path']
    mapping = obj('source-map.json')
    assert mapping['source'] == source and mapping['base'] == m['base']
    changed = []
    for row in mapping['rows']:
        assert digest(git('show', 'e76a2a78f242b5cd04f4c3609f8297eb5ecef72d:' + row['path'])) == row['sha256']
        assert digest(git('show', source + ':' + row['path'])) == row['current_sha256']
        assert row['exact_producer'] == (row['sha256'] == row['current_sha256'])
        if not row['exact_producer']:
            changed.append(row['path'])
    assert changed == ['STATUS.md', 'sdk/ts/src/native.d.ts']
    assert git('diff', 'e76a2a78f242b5cd04f4c3609f8297eb5ecef72d', source, '--', 'sdk/ts/src/native.d.ts') == records['sdk-relevance.patch']
    assert digest(records['sdk-relevance.patch']) == mapping['sdk_patch_sha256']
    assert git('diff', m['base'], source, '--', 'samples', 'tools', 'crates', 'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json') == records['source.diff']
    changed_paths = git('diff', '--name-only', m['base'], source, '--', 'crates', 'tools', 'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json').decode().splitlines()
    assert changed_paths == []
    result = obj('result.json')
    names = ['node-dependency-setup', 'python10', 'typescript', 'node-bulk', 'producer-evidence', 'conventions', 'source-diff']
    assert result['passed'] and result['source'] == source
    assert [r['name'] for r in result['commands']] == names
    for row in result['commands']:
        assert row == obj(row['name'] + '.json')
        assert row['source'] == source and row['exit_code'] == 0 and row['owned_process_reaped']
        assert digest(records[row['name'] + '.log']) == row['log_sha256']
    assert b'Ran 10 tests' in records['python10.log'] and records['python10.log'].endswith(b'OK\n')
    assert b'all 9999 rows, four phases, five refusals, omitted-write mutation.' in records['node-bulk.log']
    assert obj('producer-evidence.log') == {'passed': True, 'records': 428, 'payloads': 70, 'sources': 16, 'external_checked': True}
    assert obj('initial-identity-refusal.json')['passed'] is False
    ancestry = obj('ancestry.json')
    assert git('rev-parse', ancestry['commit'] + '^{tree}') == git('rev-parse', source + '^{tree}')
    for parent in (m['base'], m['producer'], source):
        subprocess.run(['git', 'merge-base', '--is-ancestor', parent, 'HEAD'], cwd=ROOT, check=True)
    for row in m['reviews']:
        assert digest((HERE / row['file']).read_bytes()) == row['sha256']
    command = ['python3', '-B', 'docs/spikes/evidence/core-script-stress/verify.py']
    if args.external:
        command.append('--external')
    subprocess.run(command, cwd=ROOT, check=True)
    print(json.dumps({'passed': True, 'inputs': len(inputs['files']), 'producer_source_rows': len(mapping['rows']),
                      'current_commands': len(names), 'python_tests': 10, 'records': len(m['records']),
                      'payloads': len(payloads), 'native_or_performance_reexecution': False}))


if __name__ == '__main__':
    main()
