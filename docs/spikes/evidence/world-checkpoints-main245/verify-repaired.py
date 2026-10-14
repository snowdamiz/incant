#!/usr/bin/env python3
"""Verify the bounded three-manifest repair and retained execution generations."""
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def records(path, expected):
    raw = path.read_bytes()
    assert sha(raw) == expected['sha256']
    rows = json.loads(gzip.decompress(raw))
    assert len(rows) == expected['records']
    result = {}
    for row in rows:
        name = row['path']
        assert not Path(name).is_absolute() and '..' not in Path(name).parts and name not in result
        data = base64.b64decode(row['data_base64'], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256'], name
        result[name] = data
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    meta = json.loads((HERE / 'feature-repair.json').read_bytes())
    r = records(HERE / meta['archive']['path'], meta['archive'])
    freeze = json.loads(r['freeze.json'])
    assert sha(r['freeze.json']) == meta['freeze_sha256']
    before = json.loads((HERE / 'source-composition.json').read_bytes())
    paths = subprocess.check_output(['git', 'ls-files', '--', *before['roots']], cwd=ROOT).decode().splitlines()
    assert paths == [row['path'] for row in before['files']] == [row['path'] for row in freeze['sources']]
    old = {row['path']: row['sha256'] for row in before['files']}
    new = {row['path']: row['sha256'] for row in freeze['sources']}
    changed = {p for p in paths if old[p] != new[p]}
    assert changed == {row['path'] for row in meta['manifest_changes']}
    assert changed == {'crates/incant_physics_runtime/Cargo.toml',
                       'crates/incant_physics_2d_runtime/Cargo.toml',
                       'crates/incant_runtime/Cargo.toml'}
    for row in meta['manifest_changes']:
        assert old[row['path']] == row['before_sha256'] and new[row['path']] == row['after_sha256']
        assert sha(r['sources/' + row['path']]) == row['after_sha256']
    for path in paths:
        assert sha((ROOT / path).read_bytes()) == new[path], path
    assert sha(r['feature-repair.patch']) == meta['patch_sha256']
    red = json.loads(r['manifest-current-red.json'])
    assert red['exit_code'] == 1 and red['diagnostics'] == 12
    assert sha(r['manifest-current-red.log']) == red['log_sha256']
    for check in meta['checks']:
        name = check['name']
        receipt_bytes = r[name + '.json']; receipt = json.loads(receipt_bytes); log = r[name + '.log']
        assert sha(receipt_bytes) == check['receipt_sha256']
        assert receipt['exit_code'] == 0 and receipt['reaped'] is True
        assert sha(log) == receipt['log_sha256']
        groups = re.findall(rb'test result: ok\. (\d+) passed;', log)
        assert sum(map(int, groups)) == check['passed'] and len(groups) == check['groups']
    for mode in ['ordinary', 'det', 'fast8']:
        text = r['graph-' + mode + '.log'].decode()
        expected_mode = 'fast-simd8' if mode == 'fast8' else 'deterministic'
        command = json.loads(r['graph-audit-' + mode + '.json'])
        assert command['argv'][-3:] == [expected_mode, '--target', 'aarch64-apple-darwin']
        selected = {}
        for line in text.splitlines():
            match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
            assert match, line
            flags = set(filter(None, match[2].split(',')))
            if match[1] not in {'rapier2d', 'rapier3d', 'parry2d', 'parry3d', 'simba', 'incant_physics_snapshot'}:
                continue
            if match[1] in selected:
                assert flags == selected[match[1]]
            selected[match[1]] = flags
        assert ('incant_physics_snapshot' in selected) == (mode != 'ordinary')
        if mode != 'ordinary':
            assert selected['incant_physics_snapshot'] == {'construction', 'physics-' + expected_mode}
        for solver in ['rapier2d', 'rapier3d', 'parry2d', 'parry3d', 'simba']:
            flags = selected[solver]
            if solver == 'simba':
                assert ('libm_force' in flags) == (mode != 'fast8')
            else:
                assert ('enhanced-determinism' in flags) == (mode != 'fast8')
                assert ('simd8' in flags) == (mode == 'fast8')
                assert 'parallel' in flags
                if solver.startswith('parry'):
                    assert 'rayon' in flags
    for name in ['runtime-det', 'runtime-fast8']:
        binaries = json.loads(r['executables-' + name + '.json'])
        executed = re.findall(r'Running .* \(([^)]+)\)', r[name + '.log'].decode())
        assert executed == [b['source'] for b in binaries]
        assert len(binaries) == meta['binary_counts'][name]
        if args.local_binaries:
            for b in binaries:
                p = Path(b['copy'])
                assert sha(p.read_bytes()) == b['sha256'] and p.stat().st_mode & 0o111
    terminal = json.loads(r['terminal.json'])
    assert terminal == {'success': True, 'source_files': len(paths), 'all_children_reaped': True}
    # Preserve the original verifier and historical source004 receipt unchanged.
    # Execute it on only its retained eight source copies, never relabeling them
    # as the repaired source. No compiler or test binary is run.
    producer = HERE.parent / 'native-world-checkpoints'
    manifest_bytes = (producer / 'manifest.json').read_bytes()
    assert sha(manifest_bytes) == before['producer_manifest_sha256']
    original = json.loads(manifest_bytes)
    history = records(producer / original['archive']['path'], original['archive'])
    with tempfile.TemporaryDirectory(prefix='incant-world-history-') as directory:
        root = Path(directory)
        packet = root / 'docs/spikes/evidence/native-world-checkpoints'; packet.mkdir(parents=True)
        for name in ['manifest.json', 'records.json.gz', 'verify.py']:
            data = (producer / name).read_bytes()
            assert sha(data) == meta['producer_files'][name]
            (packet / name).write_bytes(data)
        for row in original['sources']:
            path = root / row['path']; path.parent.mkdir(parents=True, exist_ok=True)
            data = history['source-004/' + row['path']]
            assert sha(data) == row['sha256']
            path.write_bytes(data)
        subprocess.run(['python3', str(packet / 'verify.py')], check=True)
    print(f'Verified {len(paths)} current frozen inputs, exactly three manifest changes, '
          f'{len(meta["checks"])} actual passing checks and retained source004 history. '
          'Persisted physics and whole-session admission remain closed.')


if __name__ == '__main__':
    main()
