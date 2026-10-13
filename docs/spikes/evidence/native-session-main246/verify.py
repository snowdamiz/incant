#!/usr/bin/env python3
"""Verify current inputs, read-only graph checks and retained execution generations."""
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
PRODUCER = HERE.parent / 'native-session-physics-checkpoints'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def tree(ref, roots):
    raw = subprocess.check_output(['git', 'ls-tree', '-rz', ref, '--', *roots], cwd=ROOT)
    return {p.decode(): m.decode().split()[2] for row in raw.split(b'\0') if row
            for m, p in [row.split(b'\t', 1)]}


def unpack(path, expected):
    raw = path.read_bytes()
    assert sha(raw) == expected, path
    rows = json.loads(gzip.decompress(raw))
    result = {}
    for row in rows:
        name = row.get('name', row.get('path'))
        assert name not in result, name
        data = base64.b64decode(row['data'], validate=True)
        assert sha(data) == row['sha256'], name
        if 'bytes' in row:
            assert len(data) == row['bytes'], name
        original = Path(row['original'])
        if original.exists():
            assert original.read_bytes() == data, original
        result[name] = data
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true', help='Only read-only verification exists')
    parser.add_argument('--local-binaries', action='store_true', help='Also hash repaired producer copies')
    args = parser.parse_args()
    manifest = read(HERE / 'manifest.json')
    raw = (HERE / 'source-composition.json').read_bytes()
    assert sha(raw) == manifest['source_composition_sha256']
    scope = json.loads(raw)
    paths = subprocess.check_output(['git', 'ls-files', '--', *scope['roots']], cwd=ROOT).decode().splitlines()
    assert paths == [f['path'] for f in scope['files']]
    expected = {f['path']: f['git_blob'] for f in scope['files']}
    assert tree(scope['producer_repaired'], scope['roots']) == expected
    assert tree(scope['source_head'], scope['roots']) == expected
    for row in scope['files']:
        assert sha((ROOT / row['path']).read_bytes()) == row['sha256'], row['path']
    assert len(paths) == manifest['source_files']
    for review in scope['reviews']:
        assert sha((HERE / review['path']).read_bytes()) == review['sha256']
    for name, digest in scope['producer_manifests'].items():
        assert sha((PRODUCER / name).read_bytes()) == digest
    repair = read(PRODUCER / 'manifest-repair.json')
    assert repair['source_commit'] == scope['producer_repaired']
    rows = unpack(PRODUCER / 'manifest-repair-records.json.gz', repair['archive_sha256'])
    assert len(rows) == repair['records'] == 11
    receipt_names = sorted(n for n in rows if n.endswith('.json') and 'argv' in json.loads(rows[n]))
    assert receipt_names == ['focused-det-005.json', 'focused-fast8-005.json', 'manifest-005.json',
                             'selected-det-005.json', 'selected-fast8-005.json']
    binaries = []
    for name in receipt_names:
        receipt = json.loads(rows[name])
        assert receipt['exit_code'] == 0 and receipt['error'] is None
        assert not receipt['changed_inputs']
        assert sha(rows[name.removesuffix('.json') + '.log']) == receipt['log_sha256']
        binaries.extend(receipt['binaries'])
        for source in receipt['inputs']:
            if any(source['path'] == r or source['path'].startswith(r + '/') for r in scope['roots']):
                assert sha((ROOT / source['path']).read_bytes()) == source['sha256'], source['path']
    assert binaries == repair['binaries']
    if args.local_binaries:
        for binary in binaries:
            assert sha(Path(binary['copy']).read_bytes()) == binary['sha256']
    for mode in ['det', 'fast8']:
        receipt = json.loads(rows[f'focused-{mode}-005.json'])
        feature = 'physics-deterministic' if mode == 'det' else 'physics-fast-simd8'
        assert receipt['argv'] == ['./tools/cargo', 'test', '--offline', '--locked', '--profile', 'ci',
                                   '-j2', '-p', 'incant_script', '--no-default-features', '--features',
                                   f'compiler,native-checkpoints,{feature}', '--lib',
                                   'checkpoint::native::', '--', '--test-threads=2']
        results = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',
                             rows[f'focused-{mode}-005.log'])
        assert results == [(b'9', b'0', b'0')], results
    checks = unpack(HERE / 'checks.json.gz', manifest['archive_sha256'])
    assert len(checks) == manifest['records'] == 2 * len(manifest['checks'])
    for name in manifest['checks']:
        receipt = json.loads(checks[name + '.json'])
        assert receipt['name'] == name and receipt['exit_code'] == 0
        assert sha(checks[name + '.log']) == receipt['log_sha256']
    for mode in ['det', 'fast8']:
        old = json.loads(rows[f'selected-{mode}-005.json'])
        current = json.loads(checks[f'graph-{mode}.json'])
        assert old['argv'] == current['argv']
        assert rows[f'selected-{mode}-005.log'].decode().replace(old['cwd'], '<workspace>') == \
            checks[f'graph-{mode}.log'].decode().replace(current['cwd'], '<workspace>')
    ordinary = checks['graph-ordinary.log']
    assert b'incant_physics_snapshot' not in ordinary and b'incant-checked-restore' not in ordinary
    # Original producer independently binds source004, 101 records, 18 receipts,
    # five immediate library copies, 195/195/186 totals and the three retained reds.
    subprocess.run(['python3', str(ROOT / 'tools/probes/native-session-physics/collect.py'),
                    '--verify-only'], cwd=ROOT, check=True)
    print(json.dumps({'current_inputs': len(paths), 'current_read_only_checks': len(manifest['checks']),
                      'repair_records': len(rows), 'repair_focused_tests_per_mode': 9,
                      'current_records': len(checks), 'normal_graphs_equal': 2,
                      'ordinary_snapshot_excluded': True,
                      'new_compilation_or_execution': False}, indent=2))


if __name__ == '__main__':
    main()
