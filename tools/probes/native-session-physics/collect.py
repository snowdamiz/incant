#!/usr/bin/env python3
"""Retain and verify scoped native-session CPU evidence; never rerun a builder."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/native-session-physics-checkpoints'
E = Path.home() / '.codex/incant-evidence/native-session-physics-checkpoints-001'
REVIEW = Path.home() / '.codex/incant-reviews/native-session-physics-source001-root-20261013.json'
SOURCE = '515df482'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(path.read_text())


def retain():
    OUT.mkdir(parents=True, exist_ok=True)
    records, payloads, binaries = [], {}, []
    for path in sorted(E.rglob('*')):
        if not path.is_file():
            continue
        data = path.read_bytes()
        row = {'original': str(path), 'path': path.relative_to(E).as_posix(),
               'sha256': sha(data), 'bytes': len(data)}
        if any(part.startswith('binaries-') for part in path.parts):
            row['mode'] = oct(path.stat().st_mode & 0o777)
            binaries.append(row)
        else:
            records.append(row)
            payloads.setdefault(row['sha256'], base64.b64encode(data).decode())
    data = REVIEW.read_bytes()
    records.append({'original': str(REVIEW), 'path': 'source-review.json',
                    'sha256': sha(data), 'bytes': len(data)})
    payloads.setdefault(sha(data), base64.b64encode(data).decode())
    archive = gzip.compress(json.dumps({'records': records, 'payloads': payloads},
                                      sort_keys=True, separators=(',', ':')).encode(), mtime=0)
    (OUT / 'records.json.gz').write_bytes(archive)
    commands = []
    for path in sorted(E.glob('*.json')):
        value = read(path)
        if isinstance(value, dict) and 'argv' in value and 'exit_code' in value:
            commands.append({'receipt': path.name, 'exit_code': value['exit_code']})
    manifest = {
        'format': 'incant.native-session-physics.cpu-evidence', 'version': 1,
        'source_commit': subprocess.check_output(['git', 'rev-parse', SOURCE], cwd=ROOT, text=True).strip(),
        'source_freeze': 'source-004/freeze.json',
        'archive': {'path': 'records.json.gz', 'sha256': sha(archive), 'records': len(records), 'payloads': len(payloads)},
        'commands': commands, 'binaries': binaries,
        'test_summaries': {'all-det-004.log': 195, 'all-fast8-004.log': 195,
                           'feature-off-004.log': 186, 'focused-det-003.log': 9},
        'limits': [
            'Source001 independent review; source002/003 fixture correction and source004 identity-list addition retained for final review.',
            'Conservative manifest guard failed: 12 baseline diagnostics and one transitive script diagnostic; separate root-owned repair pending.',
            'Both actual scoped normal mode graphs pass all five solver feature assertions.',
            'Test totals overlap focused cases. Optional host features are compile-only; this slice is headless.',
            'Only immediately copied library test executables retained; integration and doc tests have logs, not retained executables.',
            'No arbitrary VM heap, Wasm checkpoint, hosted UI/audio continuation, exported player, GPU/device or persisted physics admission claim.'
        ]}
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def verify():
    m = read(OUT / 'manifest.json')
    data = (OUT / m['archive']['path']).read_bytes()
    assert sha(data) == m['archive']['sha256']
    a = json.loads(gzip.decompress(data))
    assert len(a['records']) == m['archive']['records']
    assert len(a['payloads']) == m['archive']['payloads']
    rows = {}
    for row in a['records']:
        raw = base64.b64decode(a['payloads'][row['sha256']], validate=True)
        assert len(raw) == row['bytes'] and sha(raw) == row['sha256'], row['path']
        original = Path(row['original'])
        if original.exists():
            assert sha(original.read_bytes()) == row['sha256'], row['original']
        assert row['path'] not in rows
        rows[row['path']] = raw
    freeze = json.loads(rows[m['source_freeze']])
    for source in freeze['sources']:
        raw = subprocess.check_output(['git', 'show', m['source_commit'] + ':' + source['path']], cwd=ROOT)
        assert sha(raw) == source['sha256'], source['path']
    for cmd in m['commands']:
        receipt = json.loads(rows[cmd['receipt']])
        assert receipt['exit_code'] == cmd['exit_code']
        if cmd['receipt'] != 'focused-det-001.json':
            assert not receipt['changed_inputs'] and receipt['error'] is None
        log = cmd['receipt'].removesuffix('.json') + '.log'
        assert sha(rows[log]) == receipt['log_sha256'], log
        for binary in receipt.get('binaries', []):
            matches = [b for b in m['binaries'] if b['original'] == binary['copy']]
            assert len(matches) == 1 and matches[0]['sha256'] == binary['sha256']
    for name, count in m['test_summaries'].items():
        matches = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', rows[name])
        assert sum(int(p) for p, _, _ in matches) == count, (name, matches)
        assert all(int(f) == int(i) == 0 for _, f, i in matches), name
    historical = {r['receipt']: r['exit_code'] for r in m['commands'] if r['exit_code']}
    assert historical == {'focused-det-001.json': 101, 'focused-det-002.json': 101, 'manifest-004.json': 1}
    for b in m['binaries']:
        p = Path(b['original']); assert p.is_file() and p.stat().st_size == b['bytes'] and sha(p.read_bytes()) == b['sha256']
    print(json.dumps({'verified_records': len(rows), 'sources': len(freeze['sources']),
                      'commands': len(m['commands']), 'binaries': len(m['binaries']),
                      'historical_failures': historical}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    if not args.verify_only:
        retain()
    verify()
