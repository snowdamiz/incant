#!/usr/bin/env python3
"""Verify retained qualification without rebuilding or executing any native test."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    raw = (HERE / 'records.json.gz').read_bytes()
    assert sha(raw) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(raw))
    payloads = {digest: base64.b64decode(value, validate=True)
                for digest, value in archive['payloads'].items()}
    assert all(sha(raw) == digest for digest, raw in payloads.items())
    rows = {row['name']: row for row in archive['records']}
    originals = {row['original']: row for row in archive['records']}
    assert len(rows) == len(archive['records']) == manifest['records']
    assert len(payloads) == manifest['payloads']
    for row in rows.values():
        if row['embedded']:
            assert len(payloads[row['sha256']]) == row['bytes']
        if args.external:
            actual = Path(row['original']).read_bytes()
            assert sha(actual) == row['sha256'] and len(actual) == row['bytes'], row['name']
    def data(name):
        return payloads[rows[name]['sha256']]
    def load(name):
        return json.loads(data(name))
    def linked(path, digest):
        assert originals[path]['sha256'] == digest, path
    freezes = []
    for generation, source in enumerate(manifest['source_generations'], 1):
        freeze = load(f'source-{generation:03d}/freeze.json')
        assert freeze['source_commit'] == source
        for record in freeze['records']:
            linked(record['copy'], record['sha256'])
            if args.git_source and record['role'] in ('owned', 'baseline'):
                blob = subprocess.check_output(['git', 'show', source + ':' + record['path']], cwd=ROOT)
                assert sha(blob) == record['sha256'], record['path']
        freezes.append({r['path']: r['sha256'] for r in freeze['records'] if r['role'] == 'owned'})
    assert {p for p in freezes[0] if freezes[0][p] != freezes[1][p]} == {
        'crates/incant_profile/src/model.rs', 'tools/mac_performance.py',
        'tools/tests/test_mac_performance.py'}
    assert {p for p in freezes[1] if freezes[1][p] != freezes[2][p]} == {
        'crates/incant_game_player/tests/profile.rs'}
    commands, binaries = [], []
    test_summaries = {}
    for name in sorted(rows):
        if re.fullmatch(r'execution-\d+/checks.json', name):
            prefix = name.split('/')[0]
            checks = load(name)
            for command in checks:
                commands.append(dict(command, generation=prefix))
                assert command['owned_process_group_cleaned']
                for stream in ('stdout', 'stderr'):
                    linked(command[stream], command[stream + '_sha256'])
                stdout = data(prefix + '/' + Path(command['stdout']).name).decode(errors='replace')
                summaries = [tuple(map(int, match)) for match in re.findall(
                    r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', stdout)]
                if summaries:
                    assert all(failed == 0 for _, failed, _ in summaries)
                    test_summaries[prefix + '/' + command['name']] = summaries
            assert load(prefix + '/terminal.json')['all_native_calls_returned_after_cleanup']
            assert load(prefix + '/reaped.json')['supervisor_absent']
        if re.fullmatch(r'execution-\d+/binaries.json', name):
            for binary in load(name):
                linked(binary['path'], binary['sha256'])
                assert originals[binary['path']]['bytes'] == binary['bytes'] and int(binary['mode'], 8) & 0o111
                binaries.append(binary)
        if re.fullmatch(r'execution-\d+/compiled-native.json', name):
            for compiled in load(name):
                for file in compiled['files']:
                    linked(file['path'], file['sha256'])
                    if Path(file['path']).name == 'quickjs.c':
                        assert file['sha256'] == freezes[2]['third_party/rquickjs-sys/quickjs/quickjs.c']
    assert len(commands) == manifest['commands']
    assert sum(c['exit_code'] == 0 for c in commands) == manifest['success_commands']
    failures = [{'generation': c['generation'], 'name': c['name'], 'exit_code': c['exit_code']}
                for c in commands if c['exit_code'] != 0]
    assert failures == manifest['retained_failed_commands']
    assert len(binaries) == manifest['copied_executables']
    assert load('execution-004/result.json')['success']
    assert load('execution-006/result.json')['success']
    for mode, generation in [('physics-deterministic', 'execution-005'),
                             ('physics-fast-simd8', 'execution-006')]:
        assert sum(row[0] for row in test_summaries[generation + '/script-all-' + mode]) == 214
        assert sum(row[2] for row in test_summaries[generation + '/script-all-' + mode]) == 1
        assert test_summaries[generation + '/player-lib-' + mode] == [(21, 0, 0)]
        assert sum(row[0] for row in test_summaries[generation + '/cook-identity-' + mode]) == 6
        assert test_summaries['execution-006/player-profile-' + mode] == [(1, 0, 0)]
    assert sum(row[0] for row in test_summaries['execution-006/script-feature-off']) == 213
    assert test_summaries['execution-006/player-tracy-parsing'] == [(4, 0, 0)]
    final_commands = {c['name']: c for c in commands if c['generation'] == 'execution-006'}
    for name in ['strict-physics-deterministic', 'strict-physics-fast-simd8', 'strict-tracy',
                 'browser-compile', 'lean-graph', 'vendor-audit', 'format', 'conventions', 'diff']:
        assert final_commands[name]['exit_code'] == 0
    assert b'cannot specify features for packages outside of workspace' in data('execution-002/core-malloc-parity.stderr')
    assert b'no matching package named `trybuild`' in data('execution-003/prepared-lock-reconcile.stderr')
    assert b'unnecessary_unwrap' in data('execution-005/strict-physics-deterministic.stdout')
    assert test_summaries['execution-004/core-malloc-parity'] == [(1, 0, 0)]
    assert b'Ran 17 tests' in data('execution-004/python-consumers.stderr')
    assert b'\nOK\n' in data('execution-004/python-consumers.stderr')
    # Test-only isolated resolution: exact source bytes, explicit manifest/lock overlay.
    prepared = load('prepared-native-002/sources.json')
    for record in prepared['exact_copies']:
        if record['copy'].endswith(('rquickjs-core/Cargo.toml', 'rquickjs-core/Cargo.lock')):
            continue
        linked(record['copy'], record['sha256'])
    lock = load('execution-004/prepared-lock-comparison.json')
    assert lock['root_unchanged'] and lock['root_packages'] == 968
    assert {p['name'] for p in lock['additional_test_resolutions']} == {
        'dlopen2', 'dlopen2_derive', 'phf', 'phf_shared', 'tokio-macros'}
    root_lock = tomllib.loads(data('source-003/files/Cargo.lock').decode())['package']
    identity = lambda p: (p['name'], p['version'], p.get('source'), p.get('checksum'))
    expected = {identity(p) for p in root_lock} | {identity(p) for p in lock['additional_test_resolutions']}
    for package in tomllib.loads(data('prepared-native-002/rquickjs-core/Cargo.lock').decode())['package']:
        assert identity(package) in expected
    assert data('source-001/files/Cargo.lock') == data('source-003/files/Cargo.lock')
    if args.git_source:
        old = tomllib.loads(subprocess.check_output(['git', 'show', manifest['base'] + ':Cargo.lock'], cwd=ROOT).decode())['package']
        assert len(old) == len(root_lock) == 968
        for before, after in zip(old, root_lock):
            expected_package = dict(before)
            if before['name'] in ('rquickjs-core', 'rquickjs-sys'):
                expected_package.pop('source'); expected_package.pop('checksum')
            assert expected_package == after, before['name']
    replay = load('protocol-replay/result.json')
    assert replay['success'] and len(replay['reports']) == 4
    for report in replay['reports']:
        linked(report['source'], report['source_sha256']); linked(report['report'], report['sha256'])
        value = json.loads(payloads[report['sha256']])
        heap = value['profile']['quickjs_heap']
        assert value['profile']['version'] == 3 and heap['version'] == 1
        assert heap['mode'] == report['heap_mode']
        assert heap['samples'] == 3 and heap['invalid_samples'] == 0
        assert heap['latest']['limit_bytes'] == 32 * 1024 * 1024
        assert (heap['latest']['estimated_used_bytes'] is None) == (heap['mode'] == 'maintained')
    if args.external:
        for tool in load('tool-identities.json')['tools']:
            assert sha(Path(tool['resolved']).read_bytes()) == tool['sha256']
    print(json.dumps({'pass': True, 'records': len(rows), 'payloads': len(payloads),
                      'commands': len(commands), 'executables': len(binaries),
                      'test_summaries': test_summaries, 'failures_preserved': failures,
                      'external': args.external, 'git_source': args.git_source,
                      'performance_claim': False}, sort_keys=True))


if __name__ == '__main__':
    main()
