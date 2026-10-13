#!/usr/bin/env python3
"""Retain/review lossless Rapier compound regression CPU evidence without rerunning native code."""
import argparse
import gzip
import hashlib
import io
import json
import re
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/rapier-compound-regressions'
EVIDENCE = Path.home() / '.codex/incant-evidence/rapier-compound-regressions-001'
SOURCE = '29bb8d9db9a579616d20e4c64e8aa8f926dfe9fd'
if not __debug__:
    raise RuntimeError('verification requires Python assertions')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def collect():
    OUT.mkdir(parents=True, exist_ok=True)
    records, payloads, binaries = [], {}, []
    for path in sorted(EVIDENCE.rglob('*')):
        if path.is_symlink():
            raise ValueError('evidence symlink: ' + str(path))
        if not path.is_file():
            continue
        data = path.read_bytes()
        row = {'path': str(path.relative_to(EVIDENCE)), 'original': str(path),
               'sha256': digest(data), 'bytes': len(data)}
        if any(part.startswith('binaries-') for part in path.relative_to(EVIDENCE).parts[:-1]):
            row['mode'] = oct(path.stat().st_mode & 0o777)
            binaries.append(row)
        else:
            records.append(row)
            payloads.setdefault(row['sha256'], data)
    archive = OUT / 'evidence.tar.gz'
    with archive.open('wb') as raw, gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode='w') as tar:
            for sha, data in sorted(payloads.items()):
                info = tarfile.TarInfo('payload/' + sha)
                info.size, info.mtime, info.mode = len(data), 0, 0o644
                tar.addfile(info, io.BytesIO(data))
    sources = json.loads((EVIDENCE / 'source-001/freeze.json').read_text())['sources']
    commands = []
    for path in sorted(EVIDENCE.glob('*.json')):
        r = json.loads(path.read_text())
        if isinstance(r, dict) and 'argv' in r and 'exit_code' in r:
            commands.append({'receipt': path.name, 'exit_code': r['exit_code'],
                             'log_sha256': r['log_sha256']})
    manifest = {'format': 'incant-rapier-compound-regression-evidence', 'version': 1,
                'source_commit': SOURCE, 'archive': archive.name,
                'archive_sha256': digest(archive.read_bytes()), 'records': records,
                'payloads': len(payloads), 'binaries': binaries,
                'sources': sources, 'commands': commands,
                'qualification': 'Unfiltered private Rapier native CPU library regressions for both dimensions and modes. Prepared manifests only relocate the snapshot dependency; inherited fast-mode fixtures are separately bound. All executed binaries are copied immediately before further builds. No GPU, file-save, device or performance claim.'}
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def verify(local_binaries=False, originals=False):
    manifest = json.loads((OUT / 'manifest.json').read_text())
    archive = OUT / manifest['archive']
    assert digest(archive.read_bytes()) == manifest['archive_sha256']
    with tarfile.open(archive, 'r:gz') as tar:
        payloads = {}
        for member in tar.getmembers():
            assert member.isfile() and member.name.startswith('payload/')
            assert member.name not in payloads
            payloads[member.name] = tar.extractfile(member).read()
    assert len(payloads) == manifest['payloads']
    records = {}
    for row in manifest['records']:
        data = payloads['payload/' + row['sha256']]
        assert len(data) == row['bytes'] and digest(data) == row['sha256']
        assert row['path'] not in records
        records[row['path']] = data
        if originals:
            assert Path(row['original']).read_bytes() == data, row['original']
    for row in manifest['sources']:
        git = subprocess.check_output(['git', 'show', manifest['source_commit'] + ':' + row['path']], cwd=ROOT)
        assert digest(git) == row['sha256'], row['path']
        assert records['source-001/sources/' + row['path']] == git
    for command in manifest['commands']:
        r = json.loads(records[command['receipt']])
        assert r['exit_code'] == command['exit_code']
        assert digest(records[Path(r['log']).name]) == command['log_sha256'] == r['log_sha256']
        for binary in r.get('binaries', []):
            match = next(b for b in manifest['binaries'] if b['original'] == binary['copy'])
            assert match['sha256'] == binary['sha256'] and match['bytes'] == binary['bytes']
    for path, data in records.items():
        if path.startswith('binaries-') and path.endswith('.json'):
            for binary in json.loads(data):
                match = next(b for b in manifest['binaries'] if b['original'] == binary['path'])
                assert match['sha256'] == binary['sha256'] and match['bytes'] == binary['bytes']
                assert match['mode'] == binary['mode']
                assert digest(records[Path(binary['executed_log']).name]) == binary['log_sha256']
                assert digest(records['source-001/freeze.json']) == binary['source_freeze_sha256']
    for mode in ('det', 'fast8'):
        log = records[f'native-all-{mode}-001.log'].decode()
        summaries = [tuple(map(int, row)) for row in re.findall(
            r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out', log)]
        assert summaries == [(159, 0, 0, 0, 0), (177, 0, 0, 0, 0)], summaries
        receipt = json.loads(records[f'native-all-{mode}-001.json'])
        assert receipt['exit_code'] == 0 and '--lib' in receipt['argv']
        assert receipt['argv'][receipt['argv'].index('--features') + 2] == '--'
        graph = json.loads(records[f'graph-audit-{mode}-001.json'])
        wanted, absent = ('enhanced-determinism', 'simd8') if mode == 'det' else ('simd8', 'enhanced-determinism')
        for name in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
            matches = [line for line in graph['selected'] if line.startswith(name + ' v')]
            assert matches
            for line in matches:
                features = set(line.split('|', 1)[1].split(','))
                assert wanted in features and absent not in features
        assert len(json.loads(records[f'binaries-{mode}-001.json'])) == 2
    if local_binaries:
        for row in manifest['binaries']:
            p = Path(row['original']); data = p.read_bytes()
            assert len(data) == row['bytes'] and digest(data) == row['sha256']
            assert oct(p.stat().st_mode & 0o777) == row['mode']
    print(json.dumps({'verified': True, 'records': len(records), 'payloads': len(payloads),
                      'sources': len(manifest['sources']), 'commands': len(manifest['commands']),
                      'binaries': len(manifest['binaries']), 'originals_checked': originals,
                      'binary_bytes_checked': local_binaries}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--collect', action='store_true')
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    parser.add_argument('--originals', action='store_true')
    args = parser.parse_args()
    if args.collect:
        collect()
    verify(args.local_binaries, args.originals)
