#!/usr/bin/env python3
"""Retain or read-only verify persisted-game CPU evidence; never rebuild tests."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/persisted-game-checkpoints'
E = Path.home() / '.codex/incant-evidence/physics-persisted-session-001'
SOURCE = '2352679e52ecbd544a43400e212715000612fa3d'
REVIEW_NAMES = [
    'physics-persisted-session-source003-ordinary002-independent-20261013.json',
    'physics-persisted-outer-followup-source-20261013.json',
    'physics-persisted-ccd-source-independent-20261013.json',
]
EXPECTED = {
    'ordinary-001': (237, 1, 0), 'ordinary-002': (238, 0, 0),
    'native-focused-001': (1, 4, 0), 'native-existing-001': (260, 0, 0),
    'native-focused-002': (7, 0, 0), 'native-all-001': (267, 0, 0),
    'native-all-fast8-001': (267, 0, 0),
}
SUMMARY = re.compile(rb'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def git_blobs(commit, paths):
    """Read exactly the named committed inputs without depending on live files."""
    names = [f'{commit}:{p}' for p in paths]
    raw = subprocess.check_output(['git', 'cat-file', '--batch'], cwd=ROOT,
                                  input=('\n'.join(names) + '\n').encode())
    cursor = 0
    for path in paths:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        assert len(header) == 3 and header[1] == b'blob', (commit, path, header)
        size = int(header[2]); cursor = end + 1
        yield path, raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)


def retain():
    records, payloads, binaries, generations = [], {}, [], {}

    def add(path, data, **metadata):
        row = {'path': path, 'sha256': sha(data), 'bytes': len(data), **metadata}
        records.append(row)
        payloads.setdefault(row['sha256'], base64.b64encode(data).decode())

    for path in sorted(E.rglob('*')):
        if not path.is_file() or '__pycache__' in path.parts:
            continue
        data = path.read_bytes()
        rel = path.relative_to(E).as_posix()
        if ((path.name.startswith('bin-') and not path.name.endswith('.log'))
                or path.name == 'incant_game_player'):
            binaries.append({'path': rel, 'original': str(path), 'sha256': sha(data),
                             'bytes': len(data), 'mode': oct(path.stat().st_mode & 0o777)})
        else:
            add(rel, data, original=str(path))
    for name in REVIEW_NAMES:
        p = Path.home() / '.codex/incant-reviews' / name
        add('reviews/' + name, p.read_bytes(), original=str(p))
    for freeze_path in sorted(E.glob('*/inputs.json')):
        stage = freeze_path.parent.name
        result = read(freeze_path.parent / 'result.json')
        commit = result['head']
        frozen = read(freeze_path)
        if commit not in generations:
            generations[commit] = frozen
            for path, data in git_blobs(commit, sorted(frozen)):
                assert sha(data) == frozen[path], (stage, path)
                add('sources/' + commit + '/' + path, data, commit=commit, source_path=path)
        else:
            assert generations[commit] == frozen
    add('collector.py', Path(__file__).read_bytes())
    archive = gzip.compress(json.dumps({'records': records, 'payloads': payloads},
                                      sort_keys=True, separators=(',', ':')).encode(), mtime=0)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / 'records.json.gz').write_bytes(archive)
    manifest = {
        'format': 'incant.persisted-game-checkpoints.cpu-evidence', 'version': 1,
        'source_commit': SOURCE,
        'archive': {'path': 'records.json.gz', 'sha256': sha(archive),
                    'records': len(records), 'payloads': len(payloads)},
        'source_generations': {c: len(rows) for c, rows in generations.items()},
        'test_counts': EXPECTED, 'binaries': binaries,
        'limits': [
            'Default-off native-checkpoints; CPU tests, not a phase or phone/performance gate.',
            'Ordinary source001/002 precede the joined implementation. Their147-input freezes and raw binaries/logs are retained; prior uncommitted source bytes are not reconstructed.',
            'Native-focused001 default CCD failures remain. Existing001 excludes new persistence cases.',
            'Final focused002 and both complete native suites use the identical committed source2352679e.',
            'Four strict Clippy checks passed in checks001. Static-module setup failed without its mandatory trusted bundle, then passed with the hash-bound reused fixture in checks002.',
            'Static-module and Wasmtime results are compilation only. No shipped Wasm persistence, real device audio, GPU save, arbitrary guest heap or backend identity portability claim.',
            'Exact executable copies are local hash-bound evidence, not embedded in this portable archive.',
            'Tests overlap between stages; counts must not be summed as unique cases.',
        ],
    }
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def verify(originals=False, local_binaries=False):
    manifest = read(OUT / 'manifest.json')
    raw = (OUT / manifest['archive']['path']).read_bytes()
    assert sha(raw) == manifest['archive']['sha256']
    archive = json.loads(gzip.decompress(raw))
    assert len(archive['records']) == manifest['archive']['records']
    assert len(archive['payloads']) == manifest['archive']['payloads']
    records = {}
    for row in archive['records']:
        data = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
        assert row['path'] not in records
        records[row['path']] = data
        if originals and 'original' in row:
            assert Path(row['original']).read_bytes() == data, row['original']
    assert records['collector.py'] == Path(__file__).read_bytes()
    binaries = {b['original']: b for b in manifest['binaries']}
    assert len(binaries) == len(manifest['binaries'])
    for name, expected in EXPECTED.items():
        result = json.loads(records[name + '/result.json'])
        assert result['source_unchanged'] and not result.get('collector_error')
        if name != 'native-existing-001':
            assert result['exit'] == 0 and result['reaped']
        counts = [0, 0, 0]
        for test in result['tests']:
            binary = binaries[test['binary']]
            assert test['reaped'] and binary['sha256'] == test['sha256']
            log_name = Path(test['binary']).name
            if name.startswith('ordinary-'):
                log_name = log_name.removeprefix('bin-')
            log = records[name + '/' + log_name + '.log']
            if 'log_sha256' in test:
                assert sha(log) == test['log_sha256']
            found = [tuple(map(int, row)) for row in SUMMARY.findall(log)]
            assert found and (test['exit'] != 0) == any(row[1] != 0 for row in found)
            if 'counts' in test:
                assert found == [tuple(map(int, row)) for row in test['counts']]
            for row in found:
                counts = [a + b for a, b in zip(counts, row)]
        assert tuple(counts) == expected, (name, counts, expected)
        if 'player' in result:
            assert binaries[result['player']['copy']]['sha256'] == result['player']['sha256']
    checks = {}
    for stage in ['checks-001', 'checks-002']:
        result = json.loads(records[stage + '/result.json'])
        assert result['head'] == SOURCE and result['source_unchanged']
        assert sha(records[stage + '/inputs.json']) == result['inputs_sha256']
        for check in result['checks']:
            assert check['reaped']
            assert sha(records[stage + '/' + check['name'] + '.log']) == check['log_sha256']
            assert check['exit'] == (101 if stage == 'checks-001' and check['name'] == 'static-module' else 0)
            checks[check['name']] = check['exit']
    assert len(checks) == 13 and not any(checks.values())
    for path, data in records.items():
        if not path.endswith('/inputs.json'):
            continue
        result = json.loads(records[path.removesuffix('inputs.json') + 'result.json'])
        assert sha(data) == result['inputs_sha256']
        frozen = json.loads(data)
        for source, digest in frozen.items():
            assert sha(records['sources/' + result['head'] + '/' + source]) == digest
    if local_binaries:
        for row in binaries.values():
            path = Path(row['original'])
            assert path.stat().st_size == row['bytes'] and sha(path.read_bytes()) == row['sha256']
            assert oct(path.stat().st_mode & 0o777) == row['mode']
    print(json.dumps({'records': len(records), 'payloads': len(archive['payloads']),
                      'source_generations': manifest['source_generations'],
                      'binaries': len(binaries), 'final_checks': len(checks),
                      'test_counts': EXPECTED, 'originals': originals,
                      'local_binaries': local_binaries}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    if not args.verify_only:
        retain()
    verify(args.originals, args.local_binaries)
