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
    prior = {f['path']: f['producer_blob'] for f in s['files'] if f['producer_blob'] is not None}
    assert len(expected) == m['source_files'] == 3669 and len(prior) == 3667
    assert tree(s['source_head'], s['roots']) == expected
    assert tree(s['producer'], s['roots']) == prior
    assert subprocess.check_output(['git', 'ls-files', '--', *s['roots']], cwd=ROOT).decode().splitlines() == sorted(expected)
    added = sorted(set(expected) - set(prior))
    assert added == s['additional_evidence_tools'] == ['tools/probes/native-session-hosted-physics/collect.py', 'tools/probes/native-session-physics/collect.py']
    changed = sorted(p for p in prior if expected[p] != prior[p])
    assert changed == s['exceptions'] == [
        'crates/incant_physics_2d_runtime/src/native_owner.rs',
        'crates/incant_physics_runtime/src/native_owner.rs',
        'tools/probes/mobile-runtime-library/fixture/Cargo.lock',
        'tools/probes/mobile-runtime-library/fixture/Cargo.toml',
        'tools/probes/native-physics/Cargo.lock',
        'tools/tests/test_probe_locks.py']
    for f in s['files']:
        assert sha((ROOT / f['path']).read_bytes()) == f['sha256'], f['path']
        if f['producer_blob'] is not None:
            raw = subprocess.check_output(['git', 'cat-file', 'blob', f['producer_blob']], cwd=ROOT)
            assert sha(raw) == f['producer_sha256'], f['path']
        else:
            assert f['path'] in added and f['producer_sha256'] is None
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
    producer = HERE.parent / 'native-game-physics-checkpoints'
    assert sha((producer / 'manifest.json').read_bytes()) == m['producer_manifest_sha256']
    packet = json.loads((producer / 'manifest.json').read_bytes())
    archive = (producer / packet['archive']['path']).read_bytes()
    assert sha(archive) == packet['archive']['sha256']
    historical = {}
    for r in json.loads(gzip.decompress(archive)):
        b = base64.b64decode(r['data_base64'], validate=True)
        assert len(b) == r['bytes'] and sha(b) == r['sha256']
        assert r['path'] not in historical
        historical[r['path']] = b
    freeze = json.loads(historical['source-003/freeze.json'])
    assert len(freeze['files']) == 3667
    frozen = {r['path']: r['sha256'] for r in freeze['files']}
    assert frozen == {r['path']: r['producer_sha256'] for r in s['files'] if r['producer_blob'] is not None}
    assert (ROOT / added[0]).read_bytes() == subprocess.check_output(['git', 'show', 'eb7fc7d4:' + added[0]], cwd=ROOT)
    assert (ROOT / added[1]).read_bytes() == subprocess.check_output(['git', 'show', '1d604dfa:' + added[1]], cwd=ROOT)
    subprocess.run(['python3', str(producer / 'verify.py'), '--local-binaries'], cwd=ROOT, check=True)
    print(json.dumps({'current_inputs': len(expected), 'reviewed_exceptions': len(changed), 'added_evidence_collectors': len(added),
                      'read_only_checks': len(m['checks']), 'current_records': len(rows),
                      'normal_graphs_equal': 3, 'producer_execution_unchanged': True,
                      'new_build_or_execution': False}, indent=2))


if __name__ == '__main__':
    main()
