#!/usr/bin/env python3
"""Verify retained Android source/compile evidence; executes no tests or device work."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sha = lambda data: hashlib.sha256(data).hexdigest()


def verify(local_binaries=False):
    manifest = json.loads((HERE / 'manifest.json').read_text())
    payload = (HERE / 'artifacts.json.gz').read_bytes()
    assert sha(payload) == manifest['archive_sha256']
    rows = json.loads(gzip.decompress(payload))['files']
    assert set(rows) == {r['path'] for r in manifest['records']}
    decoded = {name: base64.b64decode(value, validate=True) for name, value in rows.items()}
    for row in manifest['records']:
        data = decoded[row['path']]
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
    for row in manifest['sources']:
        data = subprocess.check_output(['git', 'show', manifest['source_commit'] + ':' + row['path']], cwd=ROOT)
        assert sha(data) == row['sha256']
    for row in manifest['commands']:
        command = json.loads(decoded[row['receipt']])
        stem = row['receipt'].removesuffix('.json')
        assert command['exit_code'] == row['exit_code']
        for suffix in ('stdout', 'stderr'):
            assert sha(decoded[stem + '.' + suffix]) == command[suffix + '_sha256']
    if local_binaries:
        for row in manifest['binaries']:
            p = Path(row['copy'])
            assert p.stat().st_size == row['bytes'] and sha(p.read_bytes()) == row['sha256']
            assert p.stat().st_mode & 0o111
    print(json.dumps({'verified': True, 'records': len(rows), 'sources': len(manifest['sources']),
                      'commands': len(manifest['commands']), 'binary_copies':len(manifest['binaries']),
                      'local_binaries':local_binaries}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--local-binaries', action='store_true')
    verify(parser.parse_args().local_binaries)
