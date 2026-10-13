#!/usr/bin/env python3
"""Read-only combined lock/default repair qualification; no build or execution."""
import argparse
import base64
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def archive(path, digest):
    raw = path.read_bytes()
    assert sha(raw) == digest
    rows = {}
    for r in json.loads(gzip.decompress(raw)):
        b = base64.b64decode(r['data'], validate=True)
        assert len(b) == r['bytes'] and sha(b) == r['sha256']
        assert r['name'] not in rows
        rows[r['name']] = b
    return rows


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--verify-only', action='store_true')
    p.parse_args()
    c = json.loads((HERE / 'combined.json').read_bytes())
    for row in c['source'] + c['reviews']:
        assert sha((ROOT / row['path']).read_bytes()) == row['sha256'], row['path']
    roots = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo', 'crates', 'third_party', 'tools']
    changed = subprocess.check_output(['git', 'diff', '--name-only', c['base_main'], '--', *roots], cwd=ROOT).decode().splitlines()
    assert set(changed) == {r['path'] for r in c['source']}
    producer_roots = roots[:-1] + ['tools/cargo']
    assert not subprocess.check_output(['git', 'diff', c['default_producer'], '--', *producer_roots], cwd=ROOT)
    folder = HERE.parent / 'runtime-default-lint-main247'
    raw = (folder / 'manifest.json').read_bytes()
    assert sha(raw) == c['default_manifest_sha256']
    m = json.loads(raw)
    assert m['base_main'] == c['base_main']
    rows = archive(folder / 'records.json.gz', m['archive_sha256'])
    assert len(rows) == m['records'] == 10
    for name in m['checks']:
        r = json.loads(rows[name + '.json'])
        assert r['exit_code'] == 0 and r['source'] == m['source']
        assert sha(rows[name + '.log']) == r['log_sha256']
        assert b'Finished' in rows[name + '.log']
        argv = r['argv']
        assert argv[-3:] == ['--', '-D', 'warnings'] and '--all-targets' in argv
        assert argv.count('-p') == 2 and 'incant_physics_runtime' in argv and 'incant_physics_2d_runtime' in argv
        assert ('--no-default-features' in argv) == name.endswith('fast8')
        assert ('incant_physics_runtime/native-checkpoints' in ' '.join(argv)) == name.startswith('native')
    failures = archive(HERE / 'default-failures.json.gz', c['default_failures_sha256'])
    assert len(failures) == 4 and all(b'this `impl` can be derived' in b for b in failures.values())
    subprocess.run(['python3', str(HERE / 'verify.py'), '--verify-only'], cwd=ROOT, check=True)
    print('Verified six source paths, 17 lock checks, four Clippy configurations and both accepted disjoint packets; no new build.')


if __name__ == '__main__':
    main()
