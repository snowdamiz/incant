#!/usr/bin/env python3
"""Read-only input/graph qualification; preserve producer execution generations."""
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


def tree(ref, roots):
    raw = subprocess.check_output(['git', 'ls-tree', '-rz', ref, '--', *roots], cwd=ROOT)
    return {p.decode(): m.decode().split()[2] for row in raw.split(b'\0') if row
            for m, p in [row.split(b'\t', 1)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.parse_args()
    m = json.loads((HERE / 'manifest.json').read_bytes())
    raw = (HERE / 'source-composition.json').read_bytes()
    assert sha(raw) == m['source_composition_sha256']
    s = json.loads(raw)
    expected = {f['path']: f['current_blob'] for f in s['files']}
    prior = {f['path']: f['producer_blob'] for f in s['files']}
    assert len(expected) == len(prior) == m['source_files'] == 3663
    assert tree(s['source_head'], s['roots']) == expected
    assert tree(s['producer'], s['roots']) == prior
    assert subprocess.check_output(['git', 'ls-files', '--', *s['roots']], cwd=ROOT).decode().splitlines() == sorted(expected)
    changed = sorted(p for p in expected if expected[p] != prior[p])
    assert changed == s['exceptions'] == [
        'crates/incant_physics_2d_runtime/src/native_owner.rs',
        'crates/incant_physics_runtime/src/native_owner.rs',
        'tools/probes/mobile-runtime-library/fixture/Cargo.lock',
        'tools/probes/mobile-runtime-library/fixture/Cargo.toml',
        'tools/probes/native-physics/Cargo.lock',
        'tools/tests/test_probe_locks.py']
    for f in s['files']:
        assert sha((ROOT / f['path']).read_bytes()) == f['sha256'], f['path']
        raw = subprocess.check_output(['git', 'cat-file', 'blob', f['producer_blob']], cwd=ROOT)
        assert sha(raw) == f['producer_sha256'], f['path']
    for p in changed:
        assert (ROOT / p).read_bytes() == subprocess.check_output(['git', 'show', s['base_main'] + ':' + p], cwd=ROOT)
    # These two branches retain the same checked owner constructor and all
    # subsequent methods. Ordinary initialization is covered by the accepted
    # independent main248 Default repair, not claimed as byte-identical here.
    for p in changed[:2]:
        current = (ROOT / p).read_text()
        old = subprocess.check_output(['git', 'show', s['producer'] + ':' + p], cwd=ROOT).decode()
        assert current.split('impl NativeOwner {', 1)[1] == old.split('impl NativeOwner {', 1)[1]
        assert 'inner: CheckpointOwner::for_simulation(),' in current and 'inner: CheckpointOwner::for_simulation(),' in old
        assert '#[cfg(feature = "native-checkpoints")]\nimpl Default for NativeOwner' in current
        assert '#[cfg_attr(not(feature = "native-checkpoints"), derive(Default))]' in current
    for r in s['reviews']:
        assert sha((HERE / r['path']).read_bytes()) == r['sha256']
    raw = (HERE / 'checks.json.gz').read_bytes()
    assert sha(raw) == m['archive_sha256']
    rows = {}
    for r in json.loads(gzip.decompress(raw)):
        b = base64.b64decode(r['data'], validate=True)
        assert len(b) == r['bytes'] and sha(b) == r['sha256'] and r['name'] not in rows
        if Path(r['original']).exists():
            assert Path(r['original']).read_bytes() == b
        rows[r['name']] = b
    assert len(rows) == m['records'] == 28 and len(m['checks']) == 14
    for n in m['checks']:
        r = json.loads(rows[n + '.json'])
        assert r['name'] == n and r['exit_code'] == 0
        assert sha(rows[n + '.log']) == r['log_sha256']
    for mode in ['det', 'fast8', 'ordinary']:
        prior = json.loads(rows[f'producer-graph-{mode}.json'])
        current = json.loads(rows[f'graph-{mode}.json'])
        assert prior['argv'] == current['argv']
        assert rows[f'producer-graph-{mode}.log'].decode().replace(prior['cwd'], '<workspace>') == rows[f'graph-{mode}.log'].decode().replace(current['cwd'], '<workspace>')
    assert b'incant_physics_snapshot' not in rows['graph-ordinary.log']
    assert b'incant-checked-restore' not in rows['graph-ordinary.log']
    producer = HERE.parent / 'native-session-hosted-physics'
    assert sha((producer / 'manifest.json').read_bytes()) == m['producer_manifest_sha256']
    subprocess.run(['python3', str(ROOT / 'tools/probes/native-session-hosted-physics/collect.py'), '--verify-only'], cwd=ROOT, check=True)
    print(json.dumps({'current_inputs': len(expected), 'reviewed_exceptions': len(changed),
                      'read_only_checks': len(m['checks']), 'current_records': len(rows),
                      'normal_graphs_equal': 3, 'producer_execution_unchanged': True,
                      'new_build_or_execution': False}, indent=2))


if __name__ == '__main__':
    main()
