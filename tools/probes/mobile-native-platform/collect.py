#!/usr/bin/env python3
"""Retain and verify native Apple adapter CPU evidence; never launches native code."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/spikes/evidence/mobile-native-platform'
SOURCE = 'ab738f41781689b008b0572ce6ca07f78a0788e3'
ABI_SOURCE = 'e65a512dc6202fc3c0b5da8d8541382f14cd565c'
INITIAL = '34d84332cb288b7542e2264def874c45d91fdb50'
sha = lambda b: hashlib.sha256(b).hexdigest()


def git_inputs(source, paths):
    proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=ROOT,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    result = {}
    for path in paths:
        proc.stdin.write(f'{source}:{path}\n'.encode())
        proc.stdin.flush()
        header = proc.stdout.readline().split()
        assert len(header) == 3 and header[1] == b'blob', path
        value = proc.stdout.read(int(header[2]))
        assert proc.stdout.read(1) == b'\n'
        result[path] = value
    proc.stdin.close()
    assert proc.wait() == 0
    return result


def collect():
    rows, payloads = [], {}
    for generation in range(1, 7):
        folder = Path.home() / f'.codex/incant-evidence/mobile-native-platform-{generation:03}'
        for path in sorted(folder.iterdir()):
            if path.suffix not in {'.json', '.log'}:
                continue
            value = path.read_bytes()
            digest = sha(value)
            rows.append({'path': f'{generation:03}/{path.name}', 'original': str(path),
                         'bytes': len(value), 'sha256': digest})
            payloads[digest] = base64.b64encode(value).decode()
    archive = gzip.compress(json.dumps({'records': rows, 'payloads': payloads},
                                      separators=(',', ':')).encode(), mtime=0)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / 'records.json.gz').write_bytes(archive)
    (OUT / 'manifest.json').write_text(json.dumps({
        'version': 1, 'source': SOURCE, 'initial_source': INITIAL,
        'archive_sha256': sha(archive), 'records': len(rows),
        'collector_sha256': sha(Path(__file__).read_bytes()),
        'claim': 'Mac CPU, compiler and Swift typechecking evidence only. Native Apple surface/device-audio source is implemented, but no GPU/audio-device/app execution or exported app is claimed. IME, mobile haptics and platform packaging remain follow-up.'
    }, indent=2) + '\n')


def verify(originals=False, binaries=False):
    m = json.loads((OUT / 'manifest.json').read_bytes())
    assert m['source'] == SOURCE and m['initial_source'] == INITIAL
    assert sha(Path(__file__).read_bytes()) == m['collector_sha256']
    raw = (OUT / 'records.json.gz').read_bytes()
    assert sha(raw) == m['archive_sha256']
    archive = json.loads(gzip.decompress(raw))
    data = {}
    for row in archive['records']:
        value = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
        assert len(value) == row['bytes'] and sha(value) == row['sha256']
        assert row['path'] not in data
        data[row['path']] = value
        if originals:
            assert Path(row['original']).read_bytes() == value
    assert len(data) == m['records']
    read = lambda key: json.loads(data[key])
    freezes = {n: read(f'{n:03}/inputs.json') for n in range(1, 7)}
    initial = git_inputs(INITIAL, freezes[2])
    abi = git_inputs(ABI_SOURCE, freezes[5])
    final = git_inputs(SOURCE, freezes[6])
    assert freezes[2] == freezes[3]
    assert all(sha(initial[p]) == v for p, v in freezes[2].items())
    assert all(sha(abi[p]) == v for p, v in freezes[5].items())
    assert all(sha(final[p]) == v for p, v in freezes[6].items())
    assert all(sha((ROOT / p).read_bytes()) == v for p, v in freezes[6].items())
    # Source004 differs only by the deprecated atomic method spelling. Preserve
    # its successful Swift/test runs and actual lint failure, not a repaired pass.
    assert freezes[4].keys() == freezes[5].keys()
    ffi = 'crates/incant_game_player/src/mobile_native/ffi.rs'
    assert [p for p in freezes[4] if freezes[4][p] != freezes[5][p]] == [ffi]
    assert sha(abi[ffi].replace(b'.try_update(', b'.fetch_update(')) == freezes[4][ffi]
    commands = copies = failures = 0
    for n in range(1, 7):
        prefix = f'{n:03}/'
        r = read(prefix + 'result.json')
        assert r['source_unchanged']
        if n in (1, 3):
            assert r['exit'] == (101 if n == 1 else 0) and r['reaped']
            name = 'check.log' if n == 1 else 'clippy.log'
            assert sha(data[prefix + name]) == r['log_sha256']
            commands += 1
            failures += int(r['exit'] != 0)
            continue
        assert sha(data[prefix + 'inputs.json']) == r['inputs_sha256']
        expected = {2: [0] * 5, 4: [0, 0, 0, 101], 5: [0] * 12, 6: [0] * 8}[n]
        assert [row['exit'] for row in r['commands']] == expected
        for row in r['commands']:
            assert row['reaped'] and sha(data[prefix + Path(row['log']).name]) == row['sha256']
            commands += 1
            failures += int(row['exit'] != 0)
        for binary in r['binaries']:
            a = binary['artifact']
            assert a['profile']['test'] and '/aarch64-apple-darwin/' in a['executable']
            logs = [data[prefix + Path(row['log']).name] for row in r['commands'] if row['name'].startswith('build-') and row['command'][0] == './tools/cargo']
            messages = []
            for log in logs:
                for line in log.splitlines():
                    try:
                        messages.append(json.loads(line))
                    except ValueError:
                        pass
            assert a in messages and a['reason'] == 'compiler-artifact'
            if binaries:
                assert sha(Path(binary['path']).read_bytes()) == binary['sha256']
            copies += 1
    for key, count in [('002/build-player-run.log', 36), ('002/build-hosting-run.log', 11),
                       ('004/player-run.log', 39), ('005/build-det-run.log', 39),
                       ('005/build-fast8-run.log', 39), ('006/build-det-run.log', 40),
                       ('006/build-fast8-run.log', 40)]:
        assert f'test result: ok. {count} passed; 0 failed; 0 ignored;'.encode() in data[key]
    for binary in read('006/result.json')['binaries']:
        expected = 'physics-fast-simd8' if 'fast8' in binary['path'] else 'physics-deterministic'
        assert {'mobile-apple', 'mobile-device', 'mobile-host', expected} <= set(binary['artifact']['features'])
    assert commands == 31 and failures == 2 and copies == 7
    print(json.dumps({'verified': True, 'source_inputs': len(final), 'records': len(data),
                      'commands': commands, 'retained_failures': failures, 'binary_copies': copies,
                      'final_player_tests_each_mode': 40, 'initial_hosting_tests': 11,
                      'swift_typecheck': True, 'device_execution': False}))


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
