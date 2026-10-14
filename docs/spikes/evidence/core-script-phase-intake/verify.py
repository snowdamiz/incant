#!/usr/bin/env python3
"""Read-only verification of diagnostic source composition and scoped lint."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
PRODUCER = HERE.parent / 'core-script-phase'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def blobs(ref, paths):
    queries = ''.join(f'{ref}:{p}\n' for p in paths).encode()
    stream = io.BytesIO(subprocess.check_output(
        ['git', 'cat-file', '--batch'], input=queries, cwd=ROOT))
    for path in paths:
        header = stream.readline().split()
        assert len(header) == 3 and header[1] == b'blob', path
        size = int(header[2])
        data = stream.read(size)
        assert len(data) == size and stream.read(1) == b'\n'
        yield path, data
    assert not stream.read()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--current', action='store_true')
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    for name in ['review', 'composition']:
        assert sha((HERE / f'{name}.json').read_bytes()) == manifest[f'{name}_sha256']
    composition = json.loads((HERE / 'composition.json').read_text())
    packed = (HERE / 'records.json.gz').read_bytes()
    assert sha(packed) == manifest['archive_sha256']
    records = json.loads(gzip.decompress(packed))
    assert len(records) == manifest['records'] == 6
    retained = {}
    for row in records:
        data = base64.b64decode(row['payload'], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
        assert row['path'] not in retained
        retained[row['path']] = data
    lint = json.loads(retained['lint.json'])
    assert lint['exit_code'] == 0 and lint['source_unchanged']
    assert lint['managed_child_tree_reaped']
    assert lint['source'] == manifest['composition_source'] == composition['source']
    assert lint['runner_sha256'] == sha(retained['lint.py'])
    assert lint['stdout_sha256'] == sha(retained['lint.stdout'])
    assert lint['stderr_sha256'] == sha(retained['lint.stderr'])
    assert lint['argv'] == ['./tools/cargo', 'clippy', '--offline', '--locked',
        '--profile', 'ci', '-j2', '--target', 'aarch64-apple-darwin', '-p',
        'incant_script', '--no-default-features', '--features',
        'compiler,profile,physics-fast-simd8', '--lib', '--tests', '--', '-D', 'warnings']
    old = json.loads(gzip.decompress((PRODUCER / 'records.json.gz').read_bytes()))
    payloads = {r['path']: base64.b64decode(r['payload'], validate=True)
                for r in old if 'payload' in r}
    inputs = json.loads(payloads['build-inputs-001.json'])
    expected = {r['path']: r['sha256'] for r in inputs}
    assert len(expected) == composition['recorded_inputs'] == 3876
    exceptions = {r['path']: r for r in composition['exceptions']}
    assert set(exceptions) == {'crates/incant_physics/tests/character.rs',
        'crates/incant_physics/tests/compound.rs', 'crates/incant_physics/tests/runtime.rs',
        'tools/mac_performance.py', 'tools/tests/test_mac_performance.py'}
    for path, data in blobs(manifest['producer_source'], list(expected)):
        assert sha(data) == expected[path], path
    for path, row in exceptions.items():
        assert row['producer_sha256'] == expected[path]
        expected[path] = row['current_sha256']
    assert expected == lint['inputs']
    for path, data in blobs(manifest['composition_source'], list(expected)):
        assert sha(data) == expected[path], path
        if args.current:
            assert sha((ROOT / path).read_bytes()) == expected[path], path
    for path, data in blobs(manifest['main'], list(exceptions)):
        assert sha(data) == exceptions[path]['current_sha256'], path
    assert composition['equal_inputs'] == 3871
    for ref in [manifest['producer_packet'], manifest['main']]:
        subprocess.run(['git', 'merge-base', '--is-ancestor', ref,
                        manifest['retained_ancestry']], cwd=ROOT, check=True)
    subprocess.run(['git', 'diff', '--exit-code', manifest['composition_source'],
                    manifest['retained_ancestry']], cwd=ROOT, check=True)
    command = [sys.executable, str(PRODUCER / 'verify.py')]
    if args.external:
        command.append('--external')
    subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    print(json.dumps({'verified': True, 'inputs': 3876, 'exact': 3871,
                      'reviewed_exceptions': 5, 'lint_commands': 1,
                      'native_test_rerun': False, 'shipping_gate_claim': False}))


if __name__ == '__main__':
    main()
