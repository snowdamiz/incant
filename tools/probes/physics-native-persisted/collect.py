#!/usr/bin/env python3
"""Retain and verify native persisted CPU proof; never execute native code."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/physics-native-persisted'
E = Path.home() / '.codex/incant-evidence/physics-native-persisted-001'
SOURCE = 'b78ff58d4de2a2ee7e1744ca1bd4a290d9d3f0f5'
BASE = '6c25a3d3365bbac62f16e7113ce9fb2383e39384'
DEV_ONLY = {
    ('bincode', '1.3.3', 'registry+https://github.com/rust-lang/crates.io-index',
     'b1f45e9417d87227c7a56d22e471c6206462cba514c7590c09aff4cf6d1ddcad'),
    ('oorandom', '11.1.5', 'registry+https://github.com/rust-lang/crates.io-index',
     'd6790f58c7ff633d8771f42965289203411a5e5c68388703c06e14f24770b41e'),
}
FAILURES = {'focused-persisted-001.json': 101, 'focused-persisted-003.json': 101,
            'focused-ccd-001.json': 101, 'focused-ccd-002.json': 101}
SUMMARIES = {
    'focused-structure-001.log': [4, 4], 'focused-structure-002.log': [6, 6],
    'focused-persisted-002.log': [3, 3], 'focused-persisted-004.log': [15, 15],
    'focused-revision-001.log': [1, 1], 'focused-ccd-003.log': [14, 14],
    'focused-ccd-004.log': [23, 23],
    'qualified-native-all-det-001.log': [182, 200],
    'qualified-native-all-fast8-001.log': [182, 200],
}
if not __debug__:
    raise RuntimeError('evidence verification requires assertions')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(path):
    return json.loads(path.read_text())


def retain():
    OUT.mkdir(parents=True, exist_ok=True)
    records, payloads, binaries = [], {}, []
    for p in sorted(E.rglob('*')):
        if p.is_symlink():
            raise ValueError('evidence symlink: ' + str(p))
        if not p.is_file() or '__pycache__' in p.parts:
            continue
        data = p.read_bytes()
        row = {'path': p.relative_to(E).as_posix(), 'original': str(p),
               'sha256': sha(data), 'bytes': len(data)}
        if any(part.startswith('binaries-') for part in p.relative_to(E).parts[:-1]):
            row['mode'] = oct(p.stat().st_mode & 0o777)
            binaries.append(row)
        else:
            records.append(row)
            payloads.setdefault(row['sha256'], data)
    archive = OUT / 'evidence.tar.gz'
    with archive.open('wb') as raw, gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode='w') as tar:
            for digest, data in sorted(payloads.items()):
                info = tarfile.TarInfo('payload/' + digest)
                info.size, info.mtime, info.mode = len(data), 0, 0o644
                tar.addfile(info, io.BytesIO(data))
    commands = []
    for p in sorted(E.glob('*.json')):
        value = load(p)
        if isinstance(value, dict) and 'argv' in value and 'exit_code' in value:
            commands.append({'receipt': p.name, 'exit_code': value['exit_code']})
    manifest = {
        'format': 'incant.native-persisted.cpu-evidence', 'version': 1,
        'source_commit': SOURCE, 'base_commit': BASE,
        'source_freeze': 'source-qualified-001/freeze.json',
        'archive': archive.name, 'archive_sha256': sha(archive.read_bytes()),
        'records': records, 'payloads': len(payloads), 'binaries': binaries,
        'commands': commands, 'test_summaries': SUMMARIES,
        'historical_failures': FAILURES,
        'scope': [
            'Default-off native persisted rigid subset only; outer/world/session/player integration is separate.',
            'Both modes execute 182 planar + 200 spatial native unit cases, with zero ignored. Focused runs overlap those suites.',
            'Final focused CCD004 has 23 cases/dimension, including 162 paired steps and 48 moving import/recapture boundaries per dimension. Full suites repeat these in both modes.',
            'Historical 42dee source/receipts are superseded on CCD activation; final b78 uses max_ccd_substeps<=1 and exercises actual post-solve clamps. All failures retained.',
            'Private native-test lock adds only pinned bincode1.3.3 and oorandom11.1.5 dev dependencies, matching the accepted preceding native harness; root lock is unchanged.',
            'Four final mode binaries and all earlier immediate copies remain external with exact hashes/modes; no overwritten Cargo path is relabeled as historical execution.',
            'Logical work/allocation budgets are not CPU time, host allocator success, or a proof of every future floating-point operation or collision precision.',
            'No GPU, device performance, arbitrary joint/soft/custom/cluster/multi-pass CCD, full-feature save, or public outer file-admission claim.',
        ],
    }
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def git_sources(paths):
    command = ['git', 'cat-file', '--batch']
    requests = ''.join(SOURCE + ':' + p + '\n' for p in paths).encode()
    data = subprocess.check_output(command, input=requests, cwd=ROOT)
    result, cursor = {}, 0
    for path in paths:
        end = data.index(b'\n', cursor)
        header = data[cursor:end].split()
        assert len(header) == 3 and header[1] == b'blob', (path, header)
        size = int(header[2]); cursor = end + 1
        result[path] = data[cursor:cursor + size]
        cursor += size
        assert data[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(data)
    return result


def verify(originals=False, local_binaries=False):
    manifest = load(OUT / 'manifest.json')
    assert manifest['source_commit'] == SOURCE and manifest['historical_failures'] == FAILURES
    archive = (OUT / manifest['archive']).read_bytes()
    assert sha(archive) == manifest['archive_sha256']
    payloads = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz') as tar:
        for member in tar:
            assert member.isfile() and re.fullmatch(r'payload/[0-9a-f]{64}', member.name)
            assert member.name not in payloads
            payloads[member.name] = tar.extractfile(member).read()
    assert len(payloads) == manifest['payloads']
    records = {}
    for row in manifest['records']:
        data = payloads['payload/' + row['sha256']]
        assert sha(data) == row['sha256'] and len(data) == row['bytes'], row['path']
        assert row['path'] not in records
        records[row['path']] = data
        if originals:
            assert Path(row['original']).read_bytes() == data, row['original']
    def read(name):
        return json.loads(records[name])
    def relative(absolute):
        return str(Path(absolute).relative_to(E))
    freezes = [p for p in records if p.endswith('/freeze.json')]
    for name in freezes:
        for row in read(name)['sources']:
            copy = str(Path(name).parent / 'sources' / row['path'])
            assert sha(records[copy]) == row['sha256'], copy
    sources = read(manifest['source_freeze'])['sources']
    git = git_sources([r['path'] for r in sources])
    for row in sources:
        assert sha(git[row['path']]) == row['sha256'], row['path']
    assert subprocess.check_output(['git', 'show', BASE + ':Cargo.lock'], cwd=ROOT) == git['Cargo.lock']
    for dim in ('2d', '3d'):
        prefix = 'third_party/rapier' + dim + '/'
        for row in json.loads(git[prefix + 'INCANT_PERSISTED_NATIVE.json'])['files']:
            assert sha(git[prefix + row['path']]) == row['sha256'], row['path']
    for command in manifest['commands']:
        receipt = read(command['receipt'])
        assert receipt['exit_code'] == command['exit_code']
        assert sha(records[relative(receipt['log'])]) == receipt['log_sha256']
        if 'freeze' in receipt:
            assert sha(records[relative(receipt['freeze'])]) == receipt['freeze_sha256']
        else:
            assert receipt['source_freeze_sha256'] == sha(records[manifest['source_freeze']])
    assert {r['receipt']: r['exit_code'] for r in manifest['commands'] if r['exit_code']} == FAILURES
    assert manifest['test_summaries'] == SUMMARIES
    for name, expected in SUMMARIES.items():
        matches = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', records[name])
        assert [int(p) for p, _, _ in matches] == expected, (name, matches)
        assert all(int(f) == int(i) == 0 for _, f, i in matches)
    assert b'Ran 32 tests' in records['supplemental-guards-001.log']
    assert b'\nOK\n' in records['supplemental-guards-001.log']
    binary_rows = {r['original']: r for r in manifest['binaries']}
    for name in records:
        if name.startswith('binaries-') and name.endswith('.json'):
            for row in read(name):
                retained = binary_rows[row['path']]
                assert retained['sha256'] == row['sha256'] and retained['bytes'] == row['bytes']
                assert retained['mode'] == row['mode'] == '0o755'
                log = records[relative(row['executed_log'])]
                assert sha(log) == row['log_sha256']
                assert ('(' + row['original'] + ')').encode() in log
    if local_binaries:
        for row in manifest['binaries']:
            p = Path(row['original']); data = p.read_bytes()
            assert sha(data) == row['sha256'] and len(data) == row['bytes']
            assert oct(p.stat().st_mode & 0o777) == row['mode']
    verify_final_preparations(records, read, relative, git)
    print(json.dumps({'verified': True, 'records': len(records), 'payloads': len(payloads),
                      'sources': len(sources), 'source_generations': len(freezes),
                      'binaries': len(binary_rows), 'commands': len(manifest['commands']),
                      'historical_failures': FAILURES, 'tests_per_mode': 382,
                      'local_binaries': local_binaries, 'originals': originals}, indent=2))


def verify_final_preparations(records, read, relative, git):
    replacements = {row['copy']: row for row in read('qualified-fast8-fixture-binding-001.json')}
    assert len(replacements) == 16
    root_lock = tomllib.loads(git['Cargo.lock'].decode())
    registry = lambda lock: {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                             for p in lock['package'] if p.get('source')}
    prior = tomllib.loads(records['previous-native-test.lock'].decode())
    assert DEV_ONLY <= registry(prior)
    provenance = read('native-test-lock-provenance.json')
    assert sha(records['previous-native-test.lock']) == provenance['sha256']
    for mode, flavor in [('det', 'enhanced-determinism'), ('fast8', 'simd8')]:
        prefix = 'prepared-qualified-' + mode + '-001/'
        for row in read(prefix + 'source-copies.json')['sources']:
            data = records[relative(row['copy'])]
            expected = replacements[row['copy']]['sha256'] if row['copy'] in replacements else row['copy_sha256']
            assert sha(data) == expected, row['copy']
            source = str(Path(row['source']).relative_to(ROOT))
            assert sha(git[source]) == row['source_sha256'], source
        lock = tomllib.loads(records[prefix + 'Cargo.lock'].decode())
        assert registry(lock) - registry(root_lock) == DEV_ONLY
        graph = records['qualified-graph-' + mode + '-001.log'].decode()
        assert 'bincode v' not in graph and 'oorandom v' not in graph
        for package in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
            matches = [line for line in graph.splitlines() if line.startswith(package + ' v')]
            assert matches
            for line in matches:
                features = set(line.split('|', 1)[1].split(','))
                assert flavor in features and 'incant-checked-restore' in features
                assert ('simd8' if mode == 'det' else 'enhanced-determinism') not in features
        assert len(read('binaries-qualified-' + mode + '-001.json')) == 2
    ordinary = records['supplemental-ordinary-graph-001.log']
    assert b'incant_physics_snapshot ' not in ordinary and b'incant-checked-restore' not in ordinary
    diagnostic = read('broad-flag-diagnostic.json')
    assert diagnostic['frames'][0]['rejected_by_old_false_enablement']
    assert not any(r['actual_containment_failures'] for r in diagnostic['frames'])
    for row in read('outer-frame-retention-001.json') + read('review-retention-001.json'):
        assert sha(records[relative(row['copy'])]) == row['sha256']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    if not args.verify_only:
        retain()
    verify(args.originals, args.local_binaries)
