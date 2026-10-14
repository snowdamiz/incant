#!/usr/bin/env python3
"""Retain/review lossless Parry compound CPU evidence without rerunning native code."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/parry-compound-checkpoints'
EVIDENCE = Path.home() / '.codex/incant-evidence/parry-compound-checkpoints-001'
SOURCE = '4a668973077024fa29628a4ebfc519b017697157'
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
        if 'binaries' in path.relative_to(EVIDENCE).parts:
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
    sources = json.loads((EVIDENCE / 'source-001/freeze.json').read_text())['rows']
    commands = []
    for path in sorted(EVIDENCE.glob('*.json')):
        r = json.loads(path.read_text())
        if isinstance(r, dict) and 'argv' in r and 'exit_code' in r:
            commands.append({'receipt': path.name, 'exit_code': r['exit_code'],
                             'log_sha256': r['sha256']})
    manifest = {'format': 'incant-parry-compound-cpu-evidence', 'version': 1,
                'source_commit': SOURCE, 'archive': archive.name,
                'archive_sha256': digest(archive.read_bytes()), 'records': records,
                'payloads': len(payloads), 'binaries': binaries,
                'sources': sources, 'commands': commands,
                'qualification': 'Native CPU only. Prepared Parry unit-test manifests differ only in relocatable snapshot path and test-only development dependencies; separate test lock preserves all overlapping root package identities. Earlier failures retained. No full world/file-save/device/performance claim.'}
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
        assert records['source-001/' + row['path']] == git
    for command in manifest['commands']:
        r = json.loads(records[command['receipt']])
        assert r['exit_code'] == command['exit_code']
        assert digest(records[Path(r['log']).name]) == command['log_sha256'] == r['sha256']
        for binary in r.get('binaries', []):
            match = next(b for b in manifest['binaries'] if b['original'] == binary['copy'])
            assert match['sha256'] == binary['sha256'] and match['bytes'] == binary['bytes']
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
