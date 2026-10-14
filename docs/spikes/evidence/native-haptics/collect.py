#!/usr/bin/env python3
"""Lossless CPU haptics evidence; never launches a build, host or device."""
from pathlib import Path
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[4]
PACKET = Path(__file__).resolve().parent
EVIDENCE = Path('/Users/andreyyurlov/.codex/incant-evidence/native-haptics-001')
SOURCE = '44b9aead71fe83252445264adb597f51b984843b'
MAIN = '31519f8be9d09ce53eb93dc68b69ec275915492e'
FAILURES = {'desktop-002': 101, 'clippy-003': 101, 'script-regression-003': 101}
COUNTS = {
    'desktop-002': (5, 1, 0), 'script-regression-003': (150, 1, 0),
    'script-002': (9, 0, 0), 'desktop-003': (6, 0, 0), 'desktop-004': (6, 0, 0),
    'script-regression-005': (152, 0, 0), 'game-regression-005': (44, 0, 0),
    'script-fast-005': (10, 0, 0), 'game-fast-005': (1, 0, 0),
    'desktop-input-regression-005': (53, 0, 0), 'input-integration-005': (29, 0, 0),
    'script-hosted-regression-005': (20, 0, 0),
}
REQUIRED = set(COUNTS) | {'clippy-005', 'wasmtime-005', 'sdk-005', 'manifests-005',
    'lean-005', 'conventions-005', 'format-005', 'diff-final-005', 'fixture-wat-001',
    'fixture-bundle-002', 'cook-tool-001'}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, indent=2) + '\n').encode()


def assertions(files, binaries, local_binaries):
    receipts = {name[:-5]: json.loads(data) for name, data in files.items()
                if '/' not in name and name.endswith('.json')
                and 'argv' in json.loads(data)}
    assert REQUIRED <= receipts.keys(), REQUIRED - receipts.keys()
    for name, receipt in receipts.items():
        log = files[name + '.log']
        assert sha(log) == receipt['log_sha256'], name
        assert receipt['exit'] == FAILURES.get(name, 0), (name, receipt['exit'])
        if name in COUNTS:
            rows = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.decode())
            assert tuple(sum(int(row[i]) for row in rows) for i in range(3)) == COUNTS[name], name
        for binary in receipt.get('binaries', []):
            assert any(row['original'] == binary['path'] and row['sha256'] == binary['sha256'] for row in binaries)
    for generation in range(1, 6):
        prefix = f'source-{generation:03}'
        freeze = json.loads(files[prefix + '/freeze.json'])
        for row in freeze['files']:
            # Earlier captures used source/ rather than files/ only if explicitly retained.
            path = next((p for p in [prefix + '/files/' + row['path'], prefix + '/source/' + row['path']]
                         if p in files), None)
            assert path is not None and sha(files[path]) == row['sha256'], (prefix, row['path'])
    context = json.loads(files['final-source-context.json'])
    assert context['head'] == SOURCE and context['actual_main'] == MAIN
    tree = subprocess.check_output(['git', 'ls-tree', '-r', SOURCE], cwd=ROOT, text=True)
    objects = {line.split('\t', 1)[1]: line.split()[2] for line in tree.splitlines()}
    process = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for row in context['source_files']:
            process.stdin.write((objects[row['path']] + '\n').encode()); process.stdin.flush()
            header = process.stdout.readline().decode().split()
            data = process.stdout.read(int(header[2])); assert process.stdout.read(1) == b'\n'
            assert sha(data) == row['sha256'], row['path']
    finally:
        process.stdin.close(); process.stdout.close(); assert process.wait() == 0
    final = json.loads(files['source-005/freeze.json'])
    context_hashes = {row['path']: row['sha256'] for row in context['source_files']}
    for row in final['files']:
        if row['path'].startswith(('crates/', 'sdk/')) or row['path'] in ('Cargo.lock', 'Cargo.toml'):
            assert context_hashes[row['path']] == row['sha256'], row['path']
    changed = subprocess.check_output(['git', 'diff', '--name-only', MAIN, SOURCE], cwd=ROOT, text=True).splitlines()
    assert len(changed) == 27 and all(path.endswith(('.rs', '.ts', '.md')) for path in changed)
    assert subprocess.check_output(['git', 'diff', MAIN, SOURCE, '--', 'Cargo.lock', 'Cargo.toml', ':(glob)**/Cargo.toml'], cwd=ROOT) == b''
    for directory in ('static-fixture', 'static-fixture-002'):
        manifest = json.loads(files[directory + '/manifest.json'])
        for path, digest in manifest['files'].items():
            assert sha(files[directory + '/' + path]) == digest
    for row in json.loads(files['pinned-gilrs.json']):
        assert sha(files[row['copy']]) == row['sha256']
    if local_binaries:
        for row in binaries:
            p = Path(row['original']); assert sha(p.read_bytes()) == row['sha256'] and p.stat().st_mode & 0o111
    return {'commands': len(receipts), 'successful_commands': len(receipts) - len(FAILURES),
            'historical_failures': FAILURES, 'source_inputs': len(context['source_files']),
            'final_frozen_inputs': len(final['files']), 'immediate_binary_copies': len(binaries),
            'counts': COUNTS, 'manifest_and_lock_unchanged': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    if args.verify_only:
        manifest = json.loads((PACKET / 'manifest.json').read_text())
        raw = (PACKET / 'artifacts.json.gz').read_bytes()
        assert sha(raw) == manifest['archive']['sha256']
        archive = json.loads(gzip.decompress(raw)); files = {}
        for row in archive['records']:
            data = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
            assert len(data) == row['bytes'] and sha(data) == row['sha256']
            files[row['relative']] = data
        binaries = manifest['binaries']
        result = assertions(files, binaries, args.local_binaries)
        assert json.loads(json.dumps(result)) == manifest['summary']
        print(json.dumps(result, sort_keys=True)); return
    files = {str(p.relative_to(EVIDENCE)): p.read_bytes() for p in sorted(EVIDENCE.rglob('*'))
             if p.is_file() and 'binaries' not in p.relative_to(EVIDENCE).parts}
    binaries = [{'original': str(p), 'sha256': sha(p.read_bytes()), 'bytes': p.stat().st_size,
                 'mode': oct(p.stat().st_mode & 0o777)}
                for p in sorted((EVIDENCE / 'binaries').rglob('*')) if p.is_file()]
    summary = assertions(files, binaries, True)
    records = [{'relative': name, 'original': str(EVIDENCE / name), 'sha256': sha(data), 'bytes': len(data)}
               for name, data in sorted(files.items())]
    payloads = {sha(data): base64.b64encode(data).decode() for data in files.values()}
    raw = gzip.compress(json.dumps({'records': records, 'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
    assert len(raw) < 8 * 1024 * 1024
    (PACKET / 'artifacts.json.gz').write_bytes(raw)
    manifest = {'source_head': SOURCE, 'actual_main': MAIN, 'summary': summary,
                'archive': {'sha256': sha(raw), 'bytes': len(raw), 'records': len(records), 'payloads': len(payloads)},
                'binaries': binaries,
                'limits': ['CPU/mock desired-output and adapter invocation evidence only; no hardware/GPU/window run.',
                           'Gilrs process-lifetime worker is not joined on drop; asynchronous channel and physical delivery are not bounded or acknowledged by these proofs.',
                           'Wasm haptic capability enabling explicitly refuses; mobile actuator and physical-device parity remain open.',
                           'Counts overlap across focused/regression/mode generations; no summed unique-case claim.',
                           'Source001 predates accepted persisted union. Source002 adds receiver guard and persistent cleanup error. Source003 is mechanical Clippy correction. Source004 uncompiled test spelling was corrected in005. Source005 preserves historical structural overflow diagnostics.',
                           'Immediate copied executables are local evidence, hash-bound but not embedded in this compact portable archive.']}
    (PACKET / 'manifest.json').write_bytes(encoded(manifest))
    print(json.dumps(summary, sort_keys=True)); print('manifest', sha((PACKET / 'manifest.json').read_bytes()))


if __name__ == '__main__':
    main()
