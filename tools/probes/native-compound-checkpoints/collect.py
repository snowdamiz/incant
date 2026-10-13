#!/usr/bin/env python3
"""Retain the compound/tile checkpoint union's source and CPU qualification."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/native-compound-checkpoints'
EVIDENCE = Path.home() / '.codex/incant-evidence/native-compound-checkpoints-001'
BASE = '646aeb4f66dc36b4ae8749e0741b88ce90d2bac0'
SOURCE = '97bd21af81cf9444a6563d6c54a6c32d77d8c583'
if not __debug__:
    raise RuntimeError('verification requires assertions')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def collect():
    OUT.mkdir(parents=True, exist_ok=True)
    receipts = sorted(EVIDENCE.glob('*/receipt.json'))
    binaries = {}
    for path in receipts:
        for row in json.loads(path.read_bytes())['executable_copies']:
            p = Path(row['copy'])
            assert sha(p.read_bytes()) == row['sha256']
            binaries[str(p)] = {**row, 'bytes': p.stat().st_size,
                                'mode': p.stat().st_mode & 0o777}
    records = []
    for p in sorted(EVIDENCE.rglob('*')):
        if not p.is_file() or str(p) in binaries:
            continue
        assert not p.is_symlink() and p.suffix in ('.json', '.py', '.log')
        raw = p.read_bytes()
        records.append({'path': str(p.relative_to(EVIDENCE)), 'original': str(p),
                        'sha256': sha(raw), 'bytes': len(raw),
                        'data': base64.b64encode(raw).decode()})
    archive = gzip.compress(json.dumps(records, separators=(',', ':')).encode(), mtime=0)
    (OUT / 'records.json.gz').write_bytes(archive)
    paths = subprocess.check_output(['git', 'diff', '--name-only', BASE, SOURCE,
                                     '--', 'crates', 'third_party'], cwd=ROOT).decode().splitlines()
    sources = []
    for path in paths:
        raw = subprocess.check_output(['git', 'show', SOURCE + ':' + path], cwd=ROOT)
        sources.append({'path': path, 'sha256': sha(raw), 'bytes': len(raw),
                        'data': base64.b64encode(raw).decode()})
    manifest = {'format': 'incant-compound-checkpoint-union', 'version': 1,
                'base_main': BASE, 'source': SOURCE, 'archive_sha256': sha(archive),
                'records': len(records), 'sources': sources,
                'binaries': list(binaries.values()),
                'scope': 'Default-off opaque native/outer/ECS compound and tile checkpoints. '
                '538 four-package tests per mode before main250 composition; unchanged '
                'physics/runtime inputs plus 238 current script/game tests per mode. '
                'Native Parry and Rapier private unit qualifications are separate linked packets. '
                'No persisted physics byte admission, GPU, phone or performance claim.'}
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def verify(originals, local_binaries):
    m = json.loads((OUT / 'manifest.json').read_bytes())
    archive = (OUT / 'records.json.gz').read_bytes()
    assert sha(archive) == m['archive_sha256']
    records = {}
    for row in json.loads(gzip.decompress(archive)):
        raw = base64.b64decode(row['data'], validate=True)
        assert len(raw) == row['bytes'] and sha(raw) == row['sha256']
        assert row['path'] not in records
        records[row['path']] = raw
        if originals:
            assert Path(row['original']).read_bytes() == raw
    assert len(records) == m['records']
    for row in m['sources']:
        raw = base64.b64decode(row['data'], validate=True)
        assert len(raw) == row['bytes'] and sha(raw) == row['sha256']
        assert subprocess.check_output(['git', 'show', m['source'] + ':' + row['path']], cwd=ROOT) == raw
        assert (ROOT / row['path']).read_bytes() == raw
    binary_by_path = {row['copy']: row for row in m['binaries']}
    for path, raw in records.items():
        if not path.endswith('.json'):
            continue
        r = json.loads(raw)
        if not isinstance(r, dict) or 'exit_code' not in r or 'log_sha256' not in r:
            continue
        log = str(Path(path).with_suffix('.log'))
        if path.endswith('/receipt.json'):
            log = str(Path(path).parent / 'run.log')
            assert sha(records[str(Path(path).parent / 'freeze.json')]) == r['source_sha256']
            for binary in r['executable_copies']:
                assert binary_by_path[binary['copy']]['sha256'] == binary['sha256']
        assert sha(records[log]) == r['log_sha256']
    totals = {}
    for generation, expected, copies in [('union-002', 538, 67), ('consumers-001', 238, 17)]:
        for mode in ('det', 'fast8'):
            label = generation + '-' + mode
            r = json.loads(records[label + '/receipt.json'])
            log = records[label + '/run.log'].decode()
            rows = [tuple(map(int, row)) for row in re.findall(
                r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', log)]
            assert r['exit_code'] == 0 and sum(row[0] for row in rows) == expected
            assert all(row[1:] == (0, 0) for row in rows)
            assert len(r['executable_copies']) == copies
            totals[label] = expected
            units = [json.loads(line) for line in log.splitlines() if line.startswith('{')]
            for name in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
                found = [u for u in units if u.get('reason') == 'compiler-artifact'
                         and u['target']['name'] == name and 'lib' in u['target']['kind']]
                assert found, name
                wanted, absent = ('enhanced-determinism', 'simd8') if mode == 'det' else ('simd8', 'enhanced-determinism')
                assert all(wanted in u['features'] and absent not in u['features']
                           and 'parallel' in u['features'] for u in found)
    # Current-main changes are confined to already reviewed script/game work;
    # affected physics/schema/ECS production and test inputs remain exact.
    prior = json.loads(records['union-002-det/freeze.json'])['source']
    current = json.loads(records['consumers-001-fast8/freeze.json'])['source']
    changed = [p for p in prior if prior[p] != current.get(p)]
    assert len(changed) == 11
    assert all(p.startswith(('crates/incant_script/', 'crates/incant_game_runtime/')) for p in changed)
    for path, digest in current.items():
        assert sha((ROOT / path).read_bytes()) == digest, path
    qualification = [json.loads(raw) for path, raw in records.items()
                     if path.startswith('qualification-001/') and path.endswith('.json')]
    checks = [r for r in qualification if isinstance(r, dict) and 'exit_code' in r]
    assert len(checks) == 17
    assert all(r['exit_code'] == (1 if r['name'] == 'vendor-fmt' else 0) for r in checks)
    for vendor, edition in [('parry', '2021'), ('rapier', '2024')]:
        r = json.loads(records['qualification-002/' + vendor + '-fmt.json'])
        assert r['exit_code'] == 0 and r['argv'][r['argv'].index('--edition') + 1] == edition
    ordinary = records['qualification-001/graph-ordinary.log']
    assert b'incant_physics_snapshot' not in ordinary and b'incant-checked-restore' not in ordinary
    assert json.loads(records['source-001-det/receipt.json'])['exit_code'] == 101
    if local_binaries:
        for row in m['binaries']:
            p = Path(row['copy'])
            assert p.stat().st_size == row['bytes'] and p.stat().st_mode & 0o777 == row['mode']
            assert sha(p.read_bytes()) == row['sha256']
    print(json.dumps({'verified': True, 'records': len(records), 'owned_sources': len(m['sources']),
                      'immediate_binaries': len(m['binaries']), 'tests': totals,
                      'current_checks': 18, 'originals_checked': originals,
                      'binary_bytes_checked': local_binaries}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--collect', action='store_true')
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    if args.collect:
        collect()
    verify(args.originals, args.local_binaries)
