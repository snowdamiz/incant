#!/usr/bin/env python3
"""Retain and verify hosted native-session CPU evidence; never rerun a builder."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/native-session-hosted-physics'
E = Path.home() / '.codex/incant-evidence/native-session-hosted-physics-001'
REVIEW = Path.home() / '.codex/incant-reviews/native-session-hosted-source003-root-20261013.json'
SOURCE = 'c8c1329036821735cc1c4203075e72383cb101cd'


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
    for name in [REVIEW.name, 'native-session-hosted-source002-root-20261013.json',
                 'native-game-physics-source001-independent-20261013.json',
                 'native-game-physics-source003-independent-20261013.json']:
        path = REVIEW.parent / name
        data = path.read_bytes()
        records.append({'original': str(path), 'path': 'reviews/' + name,
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
        'format': 'incant.native-session-hosted-physics.cpu-evidence', 'version': 1,
        'source_commit': subprocess.check_output(['git', 'rev-parse', SOURCE], cwd=ROOT, text=True).strip(),
        'source_freeze': 'source-003/freeze.json',
        'archive': {'path': 'records.json.gz', 'sha256': sha(archive), 'records': len(records), 'payloads': len(payloads)},
        'commands': commands, 'binaries': binaries,
        'test_summaries': {'all-det-002.log': 199, 'all-fast8-002.log': 199,
                           'all-det-003.log': 200, 'all-fast8-003.log': 200,
                           'feature-off-002.log': 14, 'feature-off-003.log': 14,
                           'focused-hosted-002.log': 13, 'focused-hosted-003.log': 14},
        'limits': [
            'Independent root source002 and source003 reviews retained. Final execution audit separate.',
            'Source001 failed missing-font fixture and inherited headless host-refusal precedence controls; raw failure/binary retained.',
            'Source002 broad199 cases per mode precede silent-owner preservation; final source003 broad200 cases per mode include that additional control.',
            'Library executable copies retained immediately. Integration/doc tests have exact logs, not copied executables.',
            'Focused counts overlap full suites. UI uses CPU measurement fixture; real offline mixer control is silent. Real mixed PCM game proof is separate.',
            'No arbitrary VM heap, Wasm state, device audio rewind, exported player, GPU or persisted physics admission claim.'
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
    assert historical == {'focused-hosted-001.json': 101}
    for b in m['binaries']:
        p = Path(b['original']); assert p.is_file() and p.stat().st_size == b['bytes'] and sha(p.read_bytes()) == b['sha256']
        assert oct(p.stat().st_mode & 0o777) == b['mode'], b['original']
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
