#!/usr/bin/env python3
"""Read-only metadata/lock and retained failure qualification; never compile."""
import argparse
import base64
import gzip
import hashlib
import json
import subprocess
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_bytes())
    raw = (HERE / 'records.json.gz').read_bytes()
    assert sha(raw) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(raw))
    assert len(archive) == manifest['records']
    rows = {}
    for row in archive:
        data = base64.b64decode(row['data'], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
        assert row['name'] not in rows
        rows[row['name']] = data
        p = Path(row['original'])
        if p.exists():
            assert p.read_bytes() == data, p
    for source in manifest['source'] + manifest['build_metadata_inputs']:
        assert sha((ROOT / source['path']).read_bytes()) == source['sha256'], source['path']
    assert len(manifest['checks']) == 17
    for name in manifest['checks']:
        r = json.loads(rows[name + '.json'])
        assert r['name'] == name and r['exit_code'] == 0
        assert sha(rows[name + '.log']) == r['log_sha256']
        if name.startswith(('native-physics-', 'mobile-runtime-library-fixture-')):
            assert '--locked' in r['argv'] and '--offline' in r['argv']
            text = rows[name + '.log'].decode()
            assert 'incant_physics_snapshot' not in text
            for package in ['rapier2d', 'rapier3d', 'parry2d', 'parry3d']:
                selected = [s for s in text.splitlines() if s.startswith(package + ' ')]
                assert selected and all(r['cwd'] + '/third_party/' + package in s for s in selected)
    assert b'Ran 10 tests' in rows['guard.log'] and rows['guard.log'].rstrip().endswith(b'OK')
    diagnosis = json.loads(rows['diagnosis.json'])
    assert diagnosis['frozen_candidate'] == manifest['failed_candidate']
    failed = {r['path'] for r in diagnosis['locks'] if r['locked_exit']}
    assert len(diagnosis['locks']) == 6
    assert failed == {'tools/probes/native-physics/Cargo.lock',
                      'tools/probes/mobile-runtime-library/fixture/Cargo.lock'}
    for identity in manifest['identity_changes']:
        path = identity['path']
        stem = str(Path(path).parent).replace('/', '-')
        before = rows[stem + '.before.lock']
        assert before == subprocess.check_output(['git', 'show', manifest['failed_candidate'] + ':' + path], cwd=ROOT)
        old = tomllib.loads(before.decode())['package']
        new = tomllib.loads((ROOT / path).read_text())['package']
        def key(p):
            return (p['name'], p['version'], p.get('source'), p.get('checksum'))
        oi, ni = set(map(key, old)), set(map(key, new))
        assert sorted(oi - ni) == [tuple(x) for x in identity['removed_identities']]
        assert sorted(ni - oi) == [tuple(x) for x in identity['added_identities']]
        assert len(oi & ni) == identity['preserved_identities']
        for name, version, _, _ in oi - ni:
            assert name in ['rapier2d', 'rapier3d', 'parry2d', 'parry3d']
            assert (name, version, None, None) in ni - oi
    for name in ['115527785722-linux.log', '115527786015-windows.log', '115527786309.log']:
        assert b'cannot update the lock file' in rows[name] and b'--locked' in rows[name]
    controls = json.loads(rows['weak-resolution-controls.json'])
    assert [r['locked'] for r in controls] == [['engine', 'probe'], ['engine', 'probe'], ['engine', 'leaf', 'probe']]
    assert all(r['exit_code'] == 0 for r in controls)
    print(json.dumps({'records': len(rows), 'build_metadata_inputs': len(manifest['build_metadata_inputs']),
                      'checks': 17, 'nested_locks': 6, 'new_build_or_execution': False}, indent=2))


if __name__ == '__main__':
    main()
