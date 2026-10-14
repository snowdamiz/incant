#!/usr/bin/env python3
"""Retain/replay fixed language observations; never execute a candidate compiler."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
PROBE = ROOT / 'tools/probes/typescript-aot'
PACKET = ROOT / 'docs/spikes/evidence/typescript-aot'
CASES = ['recursive-props', 'weak-metadata', 'closure', 'caught-error', 'for-of', 'bulk-kernel']
GATES = dict(sdk100percent='unmeasured', nativeMac2x='unmeasured', iosNoJit='unmeasured',
             mobileAddedBytesUnder5MB='unmeasured', maintainedSecurityRelease='unresolved', adopted=False)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def parse(data):
    return json.loads(data)


def validate_results(files):
    for run in ['run-001', 'run-002']:
        result = parse(files[f'{run}/result.json'])
        assert result['completed'] is True and 'failure' not in result
        assert result['decision18'] == GATES
        assert result['typescript'] == '5.9.3' and result['swc'] == '1.16.13'
        assert result['assemblyscript']['version'] == '0.28.20'
        assert result['node']['version'] == 'v22.23.3'
        assert len(result['node']['sha256']) == 64
        assert result['sdk_frontend']['passed'] is True
        sdk = parse(files[f'{run}/sdk-cook.json'])
        assert digest(files[f'{run}/sdk-behavior.cjs']) == sdk['bundleSha256'] == result['sdk_frontend']['bundle_sha256']
        assert digest(files[f'{run}/toolchain-files.json']) == result['assemblyscript']['installed_files_sha256']
        assert [c['name'] for c in result['commands']] == CASES
        assert [c['name'] for c in result['cases']] == CASES
        for source, expected in result['sources'].items():
            assert digest(files[f'{run}-sources/{source}']) == expected
        for command, case in zip(result['commands'], result['cases'], strict=True):
            name = case['name']
            assert command['error'] is None and command['signal'] is None
            assert command['ended'] >= command['started']
            assert command['exit'] == case['assembly_exit'] == (0 if name == 'bulk-kernel' else 1)
            assert command['argv'][0] == result['node']['executable']
            assert command['argv'][-8:] == ['--optimizeLevel', '3', '--shrinkLevel', '0', '--initialMemory', '8', '--maximumMemory', '16']
            assert case['strict_ts_passed'] is True
            assert case['assembly_compiled'] == (name == 'bulk-kernel')
            for stream in ['stdout', 'stderr']:
                assert digest(files[f'{run}/{name}.{stream}']) == command[f'{stream}_sha256']
            if name == 'bulk-kernel':
                data = files[f'{run}/{name}.wasm']
                assert data[:8] == b'\0asm\x01\0\0\0'
                assert digest(data) == case['wasm_sha256'] and len(data) == case['wasm_bytes'] == 4926
                assert case['wasm_execution'] == 'node_wasm_values_match'
                assert case['node_values'] == [[-367, -48.484375, -4.515625], [-367, -48, -5], [-367, -48.5, -4.5], [-367, -47.625, -5.375]]
            else:
                assert case['wasm_execution'] == 'not_run'
    markers = {'recursive-props': 'Not implemented: union types', 'weak-metadata': "Cannot find name 'WeakMap'", 'closure': 'Not implemented: Closures', 'caught-error': 'Not implemented: Exceptions', 'for-of': 'Not implemented: Iterators'}
    for name, text in markers.items():
        assert text in files[f'run-002/{name}.stderr'].decode()


def verify(manifest, files, external=False):
    assert manifest['decision18'] == GATES
    assert len(manifest['records']) == len(files)
    assert {r['path'] for r in manifest['records']} == set(files)
    for row in manifest['records']:
        data = files[row['path']]
        assert len(data) == row['bytes'] and digest(data) == row['sha256'], row['path']
        if external:
            assert Path(row['original']).read_bytes() == data, row['path']
    for name, expected in manifest['sources'].items():
        assert digest((ROOT / name).read_bytes()) == expected, name
    validate_results(files)
    checks = parse(files['checks/result.json'])
    assert checks['no_native_build'] is True and checks['shermes_on_path'] is None
    assert len(checks['commands']) == 5
    for command in checks['commands']:
        assert command['exit'] == 0
        assert digest(files[command['log']]) == command['sha256']
    upstream = parse((PROBE / 'upstream.json').read_bytes())
    assert parse(files['upstream/hermes-static-head.json'])['sha'] == upstream['static_hermes']['commit']
    assert parse(files['upstream/assembly-release-source.json'])['sha'] == upstream['assemblyscript']['commit']
    for row in upstream['documents']:
        assert digest(files[row['path']]) == row['sha256']
    sdk = (ROOT / 'sdk/ts/src/native.d.ts').read_text()
    block = re.search(r'export interface NativeContext \{(.*?)^\}', sdk, re.M | re.S).group(1)
    members = re.findall(r'^  readonly (\w+):', block, re.M)
    coverage = parse((PROBE / 'coverage.json').read_bytes())
    assert coverage['native_context_members'] == members
    assert coverage['decision18'] == GATES
    for requirement in coverage['requirements']:
        assert requirement['aot_host'] == 'pending'
        for name in requirement['existing']:
            assert (ROOT / name).is_file(), name


def load():
    manifest = parse((PACKET / 'manifest.json').read_bytes())
    archive = (PACKET / 'records.json.gz').read_bytes()
    assert digest(archive) == manifest['archive_sha256']
    encoded = parse(gzip.decompress(archive))
    assert len(encoded) == len(manifest['records'])
    files = {name: base64.b64decode(value, validate=True) for name, value in encoded.items()}
    return manifest, files


def record(evidence):
    files = {}
    for folder in ['upstream', 'run-001', 'run-002', 'run-001-sources', 'run-002-sources', 'checks']:
        for p in sorted((evidence / folder).rglob('*')):
            if p.is_file():
                files[p.relative_to(evidence).as_posix()] = p.read_bytes()
    for name in ['npm-lock.log', 'npm-ci.log']:
        files[name] = (evidence / name).read_bytes()
    sources = {}
    candidates = list(PROBE.rglob('*')) + [ROOT / name for name in [
        'sdk/ts/src/native.d.ts', 'sdk/ts/src/generated.d.ts', 'package-lock.json',
        'tools/native_script/cook.mjs', 'tools/native_script/graph.mjs', 'tools/native_script/typecheck_host.mjs']]
    for p in sorted(candidates):
        if p.is_file() and '__pycache__' not in p.parts:
            sources[p.relative_to(ROOT).as_posix()] = digest(p.read_bytes())
    manifest = dict(version=1, scope='Language observations only; native AOT and all five adoption conditions remain open.', decision18=GATES,
                    sources=sources, records=[dict(path=name, bytes=len(data), sha256=digest(data), original=str(evidence/name)) for name, data in files.items()])
    verify(manifest, files, True)
    PACKET.mkdir(parents=True, exist_ok=True)
    archive = gzip.compress(json.dumps({name: base64.b64encode(data).decode() for name, data in files.items()}, sort_keys=True).encode(), mtime=0)
    (PACKET / 'records.json.gz').write_bytes(archive)
    manifest['archive_sha256'] = digest(archive)
    (PACKET / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    return manifest, files


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--record', type=Path)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    assert bool(args.record) != args.verify_only, 'Choose --record E or --verify-only'
    manifest, files = record(args.record.resolve()) if args.record else load()
    verify(manifest, files, args.external)
    print(json.dumps(dict(verified=True, records=len(files), sources=len(manifest['sources']), compiler_cases=12,
                         final_strict_ts_cases=6, final_wasm_controls=1, adopted=False)))
