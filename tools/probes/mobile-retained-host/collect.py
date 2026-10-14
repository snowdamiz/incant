#!/usr/bin/env python3
"""Retain and verify CPU/mobile-compilation evidence; never runs native code."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/mobile-retained-host'
SOURCE = 'd3a7b3ed'
sha = lambda data: hashlib.sha256(data).hexdigest()


def source_bytes(paths):
    proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=ROOT,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    result = {}
    for path in paths:
        proc.stdin.write(f'{SOURCE}:{path}\n'.encode())
        proc.stdin.flush()
        header = proc.stdout.readline().split()
        assert len(header) == 3 and header[1] == b'blob', path
        data = proc.stdout.read(int(header[2]))
        assert proc.stdout.read(1) == b'\n'
        result[path] = data
    proc.stdin.close()
    assert proc.wait() == 0
    return result


def collect():
    records, payloads = [], {}
    for generation in [1, 2]:
        evidence = Path.home() / f'.codex/incant-evidence/mobile-retained-host-{generation:03}'
        for path in sorted(evidence.iterdir()):
            if not path.is_file() or path.name.startswith('incant_game_player-'):
                continue
            data = path.read_bytes()
            digest = sha(data)
            records.append({'path': f'{generation:03}/{path.name}', 'original': str(path),
                            'bytes': len(data), 'sha256': digest})
            payloads[digest] = base64.b64encode(data).decode()
    archive = gzip.compress(json.dumps({'records': records, 'payloads': payloads},
                                      separators=(',', ':')).encode(), mtime=0)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / 'records.json.gz').write_bytes(archive)
    manifest = {'version': 1, 'source': subprocess.check_output(
        ['git', 'rev-parse', SOURCE], cwd=ROOT, text=True).strip(),
        'archive_sha256': sha(archive), 'records': len(records),
        'collector_sha256': sha(Path(__file__).read_bytes()),
        'claim': 'Retained native owner CPU tests and iOS library compilation only; no platform surface, device audio, shipping measurement or phase gate.'}
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


def verify(originals=False, binaries=False):
    manifest = json.loads((OUT / 'manifest.json').read_bytes())
    assert sha(Path(__file__).read_bytes()) == manifest['collector_sha256']
    archive = (OUT / 'records.json.gz').read_bytes()
    assert sha(archive) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(archive))
    data = {}
    for row in archive['records']:
        value = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
        assert len(value) == row['bytes'] and sha(value) == row['sha256']
        assert row['path'] not in data
        data[row['path']] = value
        if originals:
            assert Path(row['original']).read_bytes() == value
    assert len(data) == manifest['records']
    read = lambda path: json.loads(data[path])
    old, new = read('001/inputs.json'), read('002/inputs.json')
    assert old.keys() == new.keys()
    tests = 'crates/incant_game_player/src/mobile_host/tests.rs'
    assert [path for path in old if old[path] != new[path]] == [tests]
    assert sha(data['001/initial-tests.rs']) == old[tests]
    current = source_bytes(new)
    assert all(sha(current[path]) == digest for path, digest in new.items())
    expected = {1: (5, 26), 2: (7, 28)}
    for generation in [1, 2]:
        prefix = f'{generation:03}/'
        result = read(prefix + 'result.json')
        assert result['exit'] == 0 and result['reaped'] and result['source_unchanged']
        assert len(result['tests']) == 2
        for test, count in zip(result['tests'], expected[generation]):
            assert test['exit'] == 0 and test['reaped']
            log = data[prefix + test['name'] + '.log']
            assert sha(log) == test['log_sha256']
            assert f'test result: ok. {count} passed; 0 failed; 0 ignored;'.encode() in log
        messages = [json.loads(line) for line in data[prefix + 'cargo.jsonl'].splitlines()]
        binary = result['binary']
        assert binary['artifact'] in messages and binary['artifact']['profile']['test']
        assert 'mobile-host' in binary['artifact']['features']
        if binaries:
            assert sha(Path(binary['copy']).read_bytes()) == binary['sha256']
    first = read('002/checks.json')
    followup = read('002/checks-followup.json')
    assert first['source_unchanged'] and followup['source_unchanged']
    assert [row['exit'] for row in first['commands']] == [0, 0, 0, 0, 2]
    assert len(followup['commands']) == 5
    assert all(row['exit'] == 0 for row in followup['commands'])
    for row in first['commands'] + followup['commands']:
        assert row['reaped'] and sha(data['002/' + row['name'] + '.log']) == row['log_sha256']
    assert followup['commands'][-1]['command'][1] == 'check'
    assert 'aarch64-apple-ios' in followup['commands'][-1]['command']
    print(json.dumps({'verified': True, 'source_inputs': len(new), 'records': len(data),
                      'focused_tests': 7, 'player_library_tests': 28,
                      'passing_checks': 9, 'retained_driver_argument_failures': 1,
                      'originals': originals, 'binaries': binaries}))


if __name__ == '__main__':
    if not __debug__:
        raise RuntimeError('verification requires assertions')
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    if not args.verify_only:
        collect()
    verify(args.originals, args.local_binaries)
