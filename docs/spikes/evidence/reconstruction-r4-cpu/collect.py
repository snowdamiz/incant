#!/usr/bin/env python3
"""Retain/replay R4 CPU evidence. Verification performs no build or device work."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import stat
import subprocess
import tarfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
EVIDENCE = Path('/Users/andreyyurlov/.codex/incant-evidence')
CPU = EVIDENCE / 'reconstruction-r4-cpu-001'
SOURCE = '72b1ba8ec351e86d66a90c401e0a75a530633616'
BASE = '76be9608e99bf089fcb1e0908827330f70c28ac1'
IMPORT = '0aed58027288c51736c5fec4c9895549b2b2d0a6'
CLAUDE = 'f7c26dcde284181b2949f2130f244243bc3163ff'
EXPECTED_FREEZE = '19f6b7c35f40802868d6617cf325bfaf9cfbf1fe842e549bf1926836e78477ee'
RELEVANCE = {
    'crates/incant_render/src/temporal_shader/cooked.rs',
    'crates/incant_shader_cook/src/temporal/contract.rs',
    'crates/incant_shader_cook/src/temporal/mod.rs',
    'crates/incant_shader_cook/tests/temporal.rs',
}
CHECKS = {
    'default': (101, [160, 2, 43]),
    'default-002': (0, [162, 0, 43]),
    'cooker-002': (101, [28, 1, 0]),
    'cooker-003': (0, [34, 0, 0]),
    'cooked-native-003': (101, [176, 1, 44]),
    'cooked-native-004': (0, [177, 0, 44]),
    'clippy-004': (0, None),
    'fmt-001': (1, None),
    'fmt-005': (0, None),
    'diff-005': (0, None),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def git_files(commit, paths):
    data = git('archive', '--format=tar', commit, *paths)
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        return {item.name: archive.extractfile(item).read()
                for item in archive if item.isfile()}


def capture():
    records, payloads = [], {}

    def add(name, data, origin):
        digest = sha(data)
        payloads[digest] = base64.b64encode(data).decode()
        records.append(dict(name=name, sha256=digest, bytes=len(data), origin=origin))

    roots = {
        'cpu': CPU,
        'supervision-r4': EVIDENCE / '0073-reconstruction-implementation-supervision-004',
        'supervision-rename': EVIDENCE / '0073-reconstruction-compile-fix-supervision-001',
    }
    for prefix, root in roots.items():
        for path in sorted(root.rglob('*')):
            if path.is_file() and not path.relative_to(root).parts[0].startswith('binaries-'):
                add(prefix + '/' + str(path.relative_to(root)), path.read_bytes(),
                    dict(kind='file', path=str(path)))
    packet = 'handoffs/0073-upscaling-review/'
    for folder in ('reconstruction_implementation_r4', 'reconstruction_r4_compile_fix'):
        for path, data in sorted(git_files(CLAUDE, [packet + folder]).items()):
            add('claude/' + path.removeprefix(packet), data,
                dict(kind='git', commit=CLAUDE, path=path))
    return dict(version=1, records=records, payloads=payloads)


def verify(store, originals=False, local_binaries=False):
    records = {row['name']: row for row in store['records']}
    assert len(records) == len(store['records']), 'duplicate record'
    blobs = {key: base64.b64decode(value, validate=True)
             for key, value in store['payloads'].items()}
    assert all(sha(value) == key for key, value in blobs.items())
    assert set(blobs) == {row['sha256'] for row in records.values()}
    for row in records.values():
        assert len(blobs[row['sha256']]) == row['bytes']
        if originals and row['origin']['kind'] == 'file':
            assert Path(row['origin']['path']).read_bytes() == blobs[row['sha256']], row['name']
    packet = git_files(CLAUDE, ['handoffs/0073-upscaling-review/reconstruction_implementation_r4',
                               'handoffs/0073-upscaling-review/reconstruction_r4_compile_fix'])
    for row in records.values():
        if row['origin']['kind'] == 'git':
            assert packet[row['origin']['path']] == blobs[row['sha256']]

    def raw(name):
        return blobs[records[name]['sha256']]

    def obj(name):
        return json.loads(raw(name))

    final = obj('cpu/source-005/freeze.json')
    assert sha(raw('cpu/source-005/freeze.json')) == EXPECTED_FREEZE
    assert final['head'] == SOURCE and final['base'] == BASE
    source = git_files(SOURCE, ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo',
                                'crates', 'third_party', 'docs/spikes/reconstruction-r4-oracle.md'])
    assert len(final['files']) == 2465 and len(final['source_copies']) == 35
    for row in final['files'] + final['source_copies']:
        data = source[row['path']]
        assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
    for generation in range(1, 6):
        freeze = obj(f'cpu/source-{generation:03}/freeze.json')
        for row in freeze['source_copies']:
            data = raw(f'cpu/source-{generation:03}/' + row['path'])
            assert len(data) == row['bytes'] and sha(data) == row['sha256']

    # Default was executed before cooked-only contract repairs; prove exact relevance.
    default_head = obj('cpu/source-003/freeze.json')['head']
    delta = set(git('diff', '--name-only', default_head, SOURCE).decode().splitlines())
    assert delta == RELEVANCE, delta
    module = source['crates/incant_render/src/temporal_shader.rs'].decode()
    assert '#[cfg(all(feature = "cooked-temporal", not(target_arch = "wasm32")))]\nmod cooked;' in module
    manifest = source['crates/incant_render/Cargo.toml'].decode()
    assert 'default = ["physics-deterministic"]' in manifest
    assert 'incant_shader_cook = { path = "../incant_shader_cook", optional = true }' in manifest
    assert '--features' not in obj('cpu/default-002.json')['argv']
    cooker_head = obj('cpu/source-004/freeze.json')['head']
    assert git('diff', '--name-only', cooker_head, SOURCE).decode().splitlines() == [
        'crates/incant_render/src/temporal_shader/cooked.rs']

    old = git_files(IMPORT, ['crates/incant_render'])
    shaders = [path for path in source if path.endswith('.wgsl')]
    assert len(shaders) == 21
    renamed = {'crates/incant_render/src/upscaling/resolve.wgsl',
               'crates/incant_render/src/upscaling/resolve_owner.wgsl'}
    for path in shaders:
        data = source[path]
        if path in renamed:
            assert data.count(b'motion_support_coherent') == 2
            data = data.replace(b'motion_support_coherent', b'coherent')
        assert data == old[path], path

    summaries = {}
    for name, (code, counts) in CHECKS.items():
        receipt, log = obj('cpu/' + name + '.json'), raw('cpu/' + name + '.log')
        assert receipt['exit'] == code and receipt['log_sha256'] == sha(log), name
        if 'pid' in receipt:
            assert receipt['reaped'] is True, name
        parsed = re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
        actual = [sum(int(row[i]) for row in parsed) for i in range(3)]
        if counts is not None:
            assert parsed and actual == counts, (name, actual)
        summaries[name] = dict(exit=code, tests=actual if parsed else None)
    for suffix in ('', '-002', '-003', '-004'):
        release = obj('cpu/release' + suffix + '.json')
        assert release['released'] and release['source_unchanged']
        assert release['live_owned_groups'] == []
        assert all(row['reaped'] for row in release['commands'])
    for prefix in ('supervision-r4', 'supervision-rename'):
        closure = obj(prefix + '/review-closure.json')
        terminal = closure.get('terminal', closure.get('final_terminal'))
        assert terminal['exit'] == 0 and terminal['reaped'] and terminal['group_empty_after']
        assert terminal['auto_memory_environment'] == {'CLAUDE_CODE_DISABLE_AUTO_MEMORY': '1'}

    binaries = [obj('cpu/' + name) for name in ('binary-default-002.json', 'binary-cooked-native-004.json')]
    for row in binaries:
        assert row['mode'] == '0o755'
        if local_binaries:
            path = Path(row['copy'])
            assert sha(path.read_bytes()) == row['sha256'] and path.stat().st_size == row['bytes']
            assert stat.S_IMODE(path.stat().st_mode) == int(row['mode'], 8)
    return dict(records=len(records), payloads=len(blobs), source_inputs=len(final['files']),
                source_copies=len(final['source_copies']), shaders=len(shaders),
                checks=summaries, binaries=binaries,
                default_relevance_delta=sorted(delta), source=SOURCE)


def main():
    if not __debug__:
        raise SystemExit('Run without Python optimization; assertions are required.')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    archive = HERE / 'records.json.gz'
    manifest_path = HERE / 'manifest.json'
    if args.verify_only:
        manifest = json.loads(manifest_path.read_text())
        assert sha(Path(__file__).read_bytes()) == manifest['collector_sha256']
        assert sha(archive.read_bytes()) == manifest['archive_sha256']
        store = json.loads(gzip.decompress(archive.read_bytes()))
        result = verify(store, args.originals, args.local_binaries)
        assert result == manifest['audit']
    else:
        store = capture()
        result = verify(store, True, True)
        archive.write_bytes(gzip.compress(json.dumps(store, sort_keys=True, separators=(',', ':')).encode(), mtime=0))
        manifest = dict(version=1, source=SOURCE, base=BASE, freeze_sha256=EXPECTED_FREEZE,
                        collector_sha256=sha(Path(__file__).read_bytes()), archive_sha256=sha(archive.read_bytes()),
                        audit=result, scope='CPU/source/cooker only; GPU and appearance unaccepted.',
                        limits=['Default binary executed at source003; only excluded cooked paths changed afterward.',
                                'Cooker suite executed at source004; cooker source unchanged at final005.',
                                'All GPU tests ignored; no shader execution, pixels, performance or release claim.',
                                'Original freeze scope sentence predates later ABI-contract repairs; this packet binds both.',
                                'Historical shader/ABI/format failures retained; old quality failures remain open.'])
        manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(dict(accepted=True, **result), indent=2))


if __name__ == '__main__':
    main()
