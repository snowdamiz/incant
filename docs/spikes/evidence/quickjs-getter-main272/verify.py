#!/usr/bin/env python3
"""Replay the diagnostic-only main272 intake; never compile or run native code."""
import argparse
import base64
import gzip
import hashlib
import json
import subprocess
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
PRODUCER = ROOT / 'docs/spikes/evidence/quickjs-malloc-state'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(ref, path):
    return subprocess.check_output(['git', 'show', ref + ':' + path], cwd=ROOT)


def identities(rows):
    return {(r['name'], r['version'], r.get('source'), r.get('checksum')) for r in rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--current', action='store_true')
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    assert sha(Path(__file__).read_bytes()) == manifest['verifier_sha256']
    raw = (HERE / 'records.json.gz').read_bytes()
    assert sha(raw) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(raw))
    payloads = {h: base64.b64decode(v, validate=True) for h, v in archive['payloads'].items()}
    assert all(sha(v) == h for h, v in payloads.items())
    rows = {r['name']: r for r in archive['records']}
    assert len(rows) == len(archive['records']) == manifest['records']
    assert len(payloads) == manifest['payloads']
    for row in rows.values():
        assert len(payloads[row['sha256']]) == row['bytes']
        if args.external:
            assert Path(row['original']).read_bytes() == payloads[row['sha256']], row['name']

    def data(name):
        return payloads[rows[name]['sha256']]

    def load(name):
        return json.loads(data(name))

    scope = load('source-relevance.json')
    assert scope['source'] == manifest['source'] and scope['base'] == manifest['base']
    owned = scope['owned']
    assert len(owned) == 12 and len(scope['current_inputs']) == 6
    changed = subprocess.check_output(
        ['git', 'diff', '--name-only', scope['base'], scope['source']], cwd=ROOT, text=True
    ).splitlines()
    assert set(changed) == {r['path'] for r in owned}
    for row in owned + scope['current_inputs']:
        assert sha(git(scope['source'], row['path'])) == row['sha256'], row['path']
        if args.current or row in owned:
            assert sha((ROOT / row['path']).read_bytes()) == row['sha256'], row['path']
    for row in scope['current_inputs']:
        assert git(scope['source'], row['path']) == git(scope['base'], row['path'])
        assert sha(data('inputs/' + row['path'])) == row['sha256']
    assert sha(data('producer-review.json')) == manifest['producer_review_sha256']

    original_manifest = json.loads((PRODUCER / 'manifest.json').read_text())
    original_archive = json.loads(gzip.decompress((PRODUCER / 'records.json.gz').read_bytes()))
    original_rows = {r['name']: r for r in original_archive['records']}

    def original(name):
        digest = original_rows[name]['sha256']
        result = base64.b64decode(original_archive['payloads'][digest], validate=True)
        assert sha(result) == digest
        return result

    assert original_manifest['source'] == scope['source_commit']
    for row in owned:
        if row['path'].startswith('tools/probes/'):
            assert sha(original('source-001/files/owned/' + Path(row['path']).name)) == row['sha256']
    assert data('baseline.lock') == original('source-001/files/baseline/Cargo.lock')
    for path in ['rust-toolchain.toml', 'tools/cargo', 'tools/native_process.py']:
        assert data('inputs/' + path) == original('source-001/files/baseline/' + path)
    before = tomllib.loads(data('baseline.lock').decode())['package']
    after = tomllib.loads(data('inputs/Cargo.lock').decode())['package']
    assert identities(before) == identities(after) and len(after) == 968
    old = {(r['name'], r['version']): r for r in before}
    new = {(r['name'], r['version']): r for r in after}
    changed_rows = [key for key in old if old[key] != new[key]]
    assert len(changed_rows) == 1 and changed_rows[0][0] == 'incant_doc'
    key = changed_rows[0]
    expected = dict(old[key])
    expected['dependencies'] = sorted(expected['dependencies'] + ['semver'])
    assert expected == new[key]
    workspace = tomllib.loads(data('inputs/Cargo.toml').decode())
    assert workspace['workspace']['dependencies']['rquickjs'] == '=0.14.0'
    for variant in ['getter', 'wrong-count']:
        assert data(variant + '.lock') == original(f'prepared-001/{variant}/Cargo.lock')
        for row in tomllib.loads(data(variant + '.lock').decode())['package']:
            if row['name'] in ['incant-quickjs-malloc-state-probe', 'rquickjs-core', 'rquickjs-sys']:
                assert 'source' not in row and 'checksum' not in row
            else:
                assert identities([row]) <= identities(after)
    checks = load('checks.json')
    assert len(checks) == 7
    for check in checks:
        assert sha(data(check['log'])) == check['log_sha256']
        expected_exit = 2 if check['name'] == 'diff' else (1 if check['name'] == 'invalid-registry-refused' else 0)
        assert check['exit_code'] == expected_exit
    assert data('diff.log').count(b'trailing whitespace.') == 6
    packaged = load('diff-packaged.json')
    assert packaged['exit_code'] == 0 and sha(data(packaged['log'])) == packaged['log_sha256']
    attrs = git(scope['base'], '.gitattributes') + data('attributes-append.txt')
    assert sha(attrs) == packaged['packaging_delta']['sha256']
    if args.current:
        assert (ROOT / '.gitattributes').read_bytes() == attrs
    command = [sys.executable, str(PRODUCER / 'verify.py')]
    if args.external:
        command.append('--external')
    # Portable replay deliberately does not require excluded historical producer ancestry.
    subprocess.run(command, cwd=ROOT, check=True)
    print(json.dumps({'pass': True, 'records': len(rows), 'owned_paths': 12,
                      'current_inputs': 6, 'lock_identities': 968,
                      'current_checks': 8, 'historical_packaging_failure_retained': True,
                      'current_native_execution': False, 'current': args.current,
                      'external': args.external}))


if __name__ == '__main__':
    main()
