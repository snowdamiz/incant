#!/usr/bin/env python3
"""Offline source/CPU evidence replay. Never launches a compiler or engine."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / 'manifest.json').read_text())
    archive = (packet / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive']['sha256'], 'archive hash')
    require(len(archive) == manifest['archive']['bytes'], 'archive bytes')
    records = json.loads(gzip.decompress(archive))['records']
    require(len(records) == manifest['archive']['records'] == 52, 'record count')
    require(len({row['path'] for row in records}) == len(records), 'duplicate record')
    retained = {}
    for row, expected in zip(records, manifest['records'], strict=True):
        require({key: value for key, value in row.items() if key != 'data'} == expected,
                'record inventory')
        data = base64.b64decode(row['data'], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], row['path'])
        if args.originals:
            require(digest(Path(row['path']).read_bytes()) == row['sha256'], 'original ' + row['path'])
        retained[row['path']] = data

    def raw(suffix):
        matches = [data for path, data in retained.items() if path.endswith('/' + suffix)]
        require(len(matches) == 1, 'exact record ' + suffix)
        return matches[0].decode()

    def obj(suffix):
        return json.loads(raw(suffix))

    first, final = obj('cpu-001/results.json'), obj('cpu-002/results.json')
    require([row['exit_code'] for row in first['results']] == [0, 0, 101], 'first failure retained')
    require(not obj('cpu-001/terminal.json')['success'], 'first attempt cannot pass')
    require('E0659' in raw('cpu-001/2.raw.log'), 'original ambiguous import')
    require('test result:' not in raw('cpu-001/2.raw.log'), 'no original Rust test claim')
    for index, count in enumerate((22, 2)):
        require(re.search(rf'^Ran {count} tests in ', raw(f'cpu-001/{index}.raw.log'), re.M), 'Python count')
        require(raw(f'cpu-001/{index}.raw.log').endswith('\nOK\n'), 'Python outcome')
    rows = final['results']
    require(len(rows) == 7, 'remaining stages')
    require(all(row['exit_code'] == 0 and row['owned_cleanup_completed'] and 'error' not in row
                for row in rows), 'terminal outcomes/cleanup')
    require([row['original_stage_index'] for row in rows] == list(range(2, 9)), 'no repeated Python stages')
    for index, (passed, ignored) in enumerate(((13, 0), (18, 0), (6, 0), (156, 1), (151, 1))):
        require(f'test result: ok. {passed} passed; 0 failed; {ignored} ignored;' in raw(f'cpu-002/{index}.raw.log'),
                'exact feature-configuration result')
    for index in (0, 1):
        require('64,000 check/clock annotations in 1,000 complete frames allocate/reallocate nothing'
                in raw(f'cpu-002/{index}.raw.log'), 'diagnostic allocation control')
    for index in (0, 1, 2):
        require('allocation controls: hot scopes no allocation;' in raw(f'cpu-002/{index}.raw.log'),
                'dedicated allocation executable')
    require('budget_attribution_observes_real_vm_interrupt_and_explicit_checks_without_changing_writes ... ok'
            in raw('cpu-002/3.raw.log'), 'actual VM/rollback/detachment control')
    for index in (5, 6):
        require(rows[index]['argv'][-3:] == ['--', '-D', 'warnings'], 'strict lint')
        require('--all-targets' in rows[index]['argv'], 'all-target lint')
    terminal = obj('cpu-002/terminal.json')
    require(terminal['success'] and terminal['all_returned_children_cleanup_completed'], 'final cleanup')
    require(terminal['target_growth_bytes'] == 74090239 < 2 * 1024**3, 'cumulative target bound')
    require(final['copied_artifacts'] == manifest['binaries'] and len(manifest['binaries']) == 8,
            'copied executable inventory')
    for binary in manifest['binaries']:
        if args.binaries:
            data = Path(binary['copy']).read_bytes()
            require(len(data) == binary['bytes'] and digest(data) == binary['sha256'], 'binary copy')
    require(manifest['scope']['measurement_unrun'] and not manifest['scope']['eligible_for_performance_gate'],
            'no measurement/gate')
    if args.git_source or args.source_root:
        root = (args.source_root or packet.parents[3]).resolve()
        for row in manifest['source_inputs']:
            if not row['composition_check']:
                continue  # The prior source document is retained; final prose is docs-only.
            if args.git_source:
                data = subprocess.check_output(['git', 'show', args.git_source + ':' + row['path']], cwd=root)
                require(digest(data) == row['sha256'], 'Git source ' + row['path'])
            if args.source_root:
                require(digest((root / row['path']).read_bytes()) == row['sha256'], 'live source ' + row['path'])
    print('PASS: 52 records; 24 Python and 344 Rust feature-configuration test cases; '
          'three allocation executables; two strict lints; original compile refusal retained. '
          'Shipping diagnostic and performance remain unrun.')


if __name__ == '__main__':
    main()
