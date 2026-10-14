#!/usr/bin/env python3
"""Replay source/check receipts only; never build or execute native programs."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='rehash retained test copies')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'artifacts.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'archive hash')
    payloads = {
        key: base64.b64decode(value, validate=True)
        for key, value in json.loads(gzip.decompress(archive)).items()
    }
    for key, data in payloads.items():
        require(digest(data) == key, 'payload hash')
    for row in manifest['records']:
        data = payloads[row['sha256']]
        require(len(data) == row['bytes'], 'record size')
        if args.originals:
            require(Path(row['original']).read_bytes() == data, row['original'])
    require(len(manifest['source_rows']) == 5, 'five reviewed source inputs')
    for row in manifest['source_rows']:
        require(digest(payloads[row['sha256']]) == row['sha256'], 'reviewed source')
        if args.source_root:
            require(digest((args.source_root / row['path']).read_bytes()) == row['sha256'], row['path'])
    final_tests = []
    for command in manifest['commands']:
        log = payloads[command['log_sha256']].decode()
        if 'test_totals' in command:
            rows = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
            totals = {key: sum(int(row[i]) for row in rows) for i, key in enumerate(('passed', 'failed', 'ignored'))}
            totals['harnesses'] = len(rows)
            require(totals == command['test_totals'], command['id'])
            if command['phase'] == 'source001':
                require(command['exit_code'] == 0 and totals == {'passed': 233, 'failed': 0, 'ignored': 1, 'harnesses': 10}, 'final test scope')
                controls = re.findall(r'test loan::tests::\w+ \.\.\. ok', log)
                require(len(controls) == 7, 'all seven native loan controls')
                final_tests.append(command['id'])
        elif command['argv'][1] == 'clippy':
            require(command['exit_code'] == 0 and 'Finished `ci` profile' in log, 'strict lint result')
        else:
            require(command['exit_code'] == 0 and not log, 'format/diff result')
    require(final_tests == ['default-all-003', 'fast-all-001'], 'two exact final physics runs')
    first = manifest['commands'][0]
    require(first['id'] == 'default-lib-001' and first['exit_code'] == 101 and first['test_totals']['failed'] == 1, 'retained initial failure')
    require('TypeError: ArrayBuffer is detached or resized' in payloads[first['log_sha256']].decode(), 'initial DataView failure')
    identity_outputs = [payloads[row['sha256']].decode() for row in manifest['records'] if row['role'] == 'actual-build-identity-output']
    require(len(identity_outputs) == 6, 'actual compiler identity outputs')
    for text in identity_outputs:
        for name in manifest['build_identity']['loan_inputs']:
            require('cargo:rerun-if-changed=' + name + '\n' in text, 'loan input identity')
        require('INCANT_SCRIPT_BUILD_ID=' + manifest['build_identity']['build_id'] in text, 'actual conservative build ID')
    require(manifest['no_shipping_export_or_measurement'] is True and manifest['phase_gate_passed'] is False, 'scope')
    require(len(manifest['test_executables']) == 18, 'retained native test executables')
    if args.binaries:
        for row in manifest['test_executables']:
            data = Path(row['copy']).read_bytes()
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], row['copy'])
    print(json.dumps({'verified': True, 'reviewed_source_files': 5, 'final_native_passes': 466, 'ignored_diagnostics': 2, 'retained_initial_failures': 1, 'shipping_measurement': False, 'phase_gate_passed': False, 'external_binary_rehashed': args.binaries}))


if __name__ == '__main__':
    main()
