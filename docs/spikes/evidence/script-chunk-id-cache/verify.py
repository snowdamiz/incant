#!/usr/bin/env python3
"""Verify retained sources, command results and explicit local binary copies."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    assert sha(Path(__file__).read_bytes()) == manifest['verifier_sha256']
    archive = (HERE / 'artifacts.json.gz').read_bytes()
    assert sha(archive) == manifest['archive_sha256']
    payloads = {key: base64.b64decode(value, validate=True)
                for key, value in json.loads(gzip.decompress(archive))['payloads'].items()}
    assert all(sha(value) == key for key, value in payloads.items())
    records = {row['path']: row for row in manifest['records']}
    assert len(records) == len(manifest['records'])
    for path, row in records.items():
        if row['embedded']:
            assert len(payloads[row['sha256']]) == row['bytes']
        if args.external:
            original = Path(manifest['raw_root']) / path
            assert original.stat().st_size == row['bytes'] and sha(original.read_bytes()) == row['sha256']
            assert oct(original.stat().st_mode & 0o777) == row['mode']

    def read(path):
        return payloads[records[path]['sha256']]

    frozen = json.loads(read('source-001/freeze.json'))
    assert frozen['source_base'] == manifest['base_commit']
    assert frozen['source_paths'] == {row['path']: row['sha256'] for row in manifest['sources']}
    for path, digest in frozen['source_paths'].items():
        assert records['source-001/' + path]['sha256'] == digest
    counts = {}
    binaries = 0
    for name in manifest['passing_commands']:
        receipt = json.loads(read(name + '.json'))
        assert receipt['exit'] == 0 and receipt['error'] is None
        assert receipt['ended_ns'] >= receipt['started_ns']
        assert receipt['source_freeze_sha256'] == records['source-001/freeze.json']['sha256']
        argv = receipt['argv']
        assert argv[0] == './tools/cargo'
        assert all(option in argv for option in ('--offline', '--locked', '-j2', '--no-default-features'))
        assert argv[argv.index('--profile') + 1] == 'ci'
        assert argv[argv.index('--target') + 1] == 'aarch64-apple-darwin'
        mode = 'physics-fast-simd8' if name.startswith('fast-') else 'physics-deterministic'
        assert argv[argv.index('--features') + 1] == 'compiler,' + mode
        assert receipt['environment']['RAYON_NUM_THREADS'] == '2'
        if name.endswith('clippy'):
            assert argv[-3:] == ['--', '-D', 'warnings']
        for log in receipt['logs'].values():
            assert records[log['path']]['sha256'] == log['sha256']
            assert records[log['path']]['bytes'] == log['bytes']
        for binary in receipt['binaries']:
            retained = records[binary['copy']]
            assert retained['sha256'] == binary['sha256'] and retained['bytes'] == binary['bytes']
            assert int(retained['mode'], 8) & 0o111
            assert binary['original'] in read(name + '.stderr').decode()
            binaries += 1
        match = re.search(r'test result: ok\. (\d+) passed; 0 failed;', read(name + '.stdout').decode())
        if match:
            counts[name] = int(match.group(1))
    assert counts == {'deterministic-lib': 138, 'deterministic-project': 12, 'fast-lib': 138, 'fast-project': 12}
    assert binaries == 4
    probe = subprocess.run(['git', 'cat-file', '-e', manifest['source_commit']], cwd=ROOT, capture_output=True)
    if probe.returncode == 0:
        for row in manifest['sources']:
            content = subprocess.check_output(['git', 'show', manifest['source_commit'] + ':' + row['path']], cwd=ROOT)
            assert sha(content) == row['sha256']
    print(json.dumps({'passed': True, 'records': len(records), 'payloads': len(payloads),
                      'cases_per_mode': 150, 'binary_copies': binaries,
                      'external_checked': args.external, 'producer_git_checked': probe.returncode == 0}))


if __name__ == '__main__':
    main()
