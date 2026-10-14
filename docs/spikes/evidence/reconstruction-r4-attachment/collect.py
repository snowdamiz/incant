#!/usr/bin/env python3
"""Replay the test-only attachment correction and bounded numerical run, no GPU."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
E = Path('/Users/andreyyurlov/.codex/incant-evidence')
CPU = E / 'reconstruction-r4-attachment-reference-001'
GPU = E / 'reconstruction-r4-numeric-002'
SOURCE = 'cae2bffcdef061d3e1505b7f78a2d9888b582e61'
PRODUCTION = '72b1ba8ec351e86d66a90c401e0a75a530633616'
FREEZE = 'aae2ad5281edfea33454312eabdf5eae5c3ccc069b5b2c7f8d2ee35e3f27f1c7'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git_files(commit):
    data = subprocess.check_output(['git', 'archive', commit, 'Cargo.toml', 'Cargo.lock',
                                   'rust-toolchain.toml', '.cargo', 'crates', 'third_party'], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        return {item.name: archive.extractfile(item).read() for item in archive if item.isfile()}


def capture():
    records, payloads = [], {}
    for prefix, root in [('cpu', CPU), ('gpu', GPU)]:
        for path in sorted(root.rglob('*')):
            relative = path.relative_to(root)
            if not path.is_file() or relative.parts[0] in ('binaries', 'binaries-002', 'tmp') or '__pycache__' in relative.parts:
                continue
            data = path.read_bytes()
            digest = sha(data)
            payloads[digest] = base64.b64encode(data).decode()
            records.append(dict(name=prefix + '/' + str(relative), path=str(path), sha256=digest, bytes=len(data)))
    return dict(version=1, records=records, payloads=payloads)


def verify(store, originals=False, binaries=False):
    records = {row['name']: row for row in store['records']}
    assert len(records) == len(store['records'])
    payloads = {key: base64.b64decode(value, validate=True) for key, value in store['payloads'].items()}
    assert set(payloads) == {row['sha256'] for row in records.values()}
    for row in records.values():
        data = payloads[row['sha256']]
        assert sha(data) == row['sha256'] and len(data) == row['bytes']
        if originals:
            assert Path(row['path']).read_bytes() == data, row['name']

    def raw(name):
        return payloads[records[name]['sha256']]

    def obj(name):
        return json.loads(raw(name))

    source, baseline = git_files(SOURCE), git_files(PRODUCTION)
    final = obj('cpu/source-003/freeze.json')
    assert sha(raw('cpu/source-003/freeze.json')) == FREEZE
    changed = {row['path'] for row in final['files']}
    assert len(changed) == 4
    assert {path for path in source if source[path] != baseline.get(path)} == changed
    assert not set(baseline) - set(source)
    for generation in range(1, 4):
        freeze = obj(f'cpu/source-{generation:03}/freeze.json')
        assert freeze['production_source'] == PRODUCTION
        for row in freeze['files']:
            data = raw(f'cpu/source-{generation:03}/' + row['path'])
            assert sha(data) == row['sha256'] and len(data) == row['bytes']
            if generation == 3:
                assert source[row['path']] == data
    for manifest, generation in [('build-inputs.json', 2), ('build-inputs-002.json', 3)]:
        inputs = obj('cpu/' + manifest)
        assert inputs['production'] == PRODUCTION and len(inputs['files']) == 2467
        for row in inputs['files']:
            data = raw(f'cpu/source-{generation:03}/' + row['path']) if row['path'] in changed else source[row['path']]
            assert sha(data) == row['sha256'] and len(data) == row['bytes'], row['path']
    system = source['crates/incant_render/src/upscaling/system.rs']
    for name in ['evidence_reference', 'device_tests']:
        assert f'#[cfg(test)]\n#[path = "{name}.rs"]\nmod {name};'.encode() in system
    shaders = [path for path in source if path.endswith('.wgsl')]
    assert len(shaders) == 21 and all(source[path] == baseline[path] for path in shaders)

    counts = {}
    for generation in ('001', '002'):
        for name in ('default', 'cooked-native', 'clippy'):
            key = name + '-' + generation
            receipt, log = obj('cpu/' + key + '.json'), raw('cpu/' + key + '.log')
            assert receipt['exit'] == 0 and receipt['reaped'] and receipt['log_sha256'] == sha(log)
            if name != 'clippy':
                actual = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
                expected = (166, 0, 43) if name == 'default' else (181, 0, 44)
                if generation == '002':
                    expected = (5, 0, 0)
                assert len(actual) == 1 and tuple(map(int, actual[0])) == expected
                counts[key] = list(expected)
        release = obj('cpu/release-' + generation + '.json')
        assert release['released'] and release['source_unchanged'] and release['live_owned_groups'] == []
    for key in ('fmt-003', 'diff-003', 'default-list-003', 'cooked-native-list-003'):
        receipt, log = obj('cpu/' + key + '.json'), raw('cpu/' + key + '.log')
        assert receipt['exit'] == 0 and receipt['reaped'] and receipt['log_sha256'] == sha(log)
    executable_rows = []
    for suffix in ('', '-002'):
        for name in ('default', 'cooked-native'):
            row = obj('cpu/' + name + '-binary' + suffix + '.json')
            executed = name + ('-001' if not suffix else '-002')
            assert row['log_sha256'] == sha(raw('cpu/' + executed + '.log'))
            if binaries:
                path = Path(row['copy'])
                assert sha(path.read_bytes()) == row['sha256'] and path.stat().st_size == row['bytes']
                assert path.stat().st_mode & 0o111
            executable_rows.append(row)

    release, plan, grant = obj('gpu/release.json'), obj('gpu/preparation/plan.json'), obj('gpu/grant.json')
    assert release['accepted'] and release['released'] and release['exception'] is None
    assert release['images'] == release['owned_processes'] == release['process_groups_alive'] == []
    for field, path in [('plan_sha256', 'preparation/plan.json'), ('runner_sha256', 'preparation/run.py')]:
        assert release[field] == grant[field] == sha(raw('gpu/' + path))
    assert release['grant_sha256'] == sha(raw('gpu/grant.json'))
    assert release['lifecycle_sha256'] == sha(raw('gpu/preparation/lifecycle.py'))
    assert plan['source'] == SOURCE and plan['submissions'] == plan['maps'] == 62
    assert plan['attempts'] == 4 and plan['images'] == plan['builds'] == plan['retries'] == 0
    assert [row['sha256'] for row in plan['binaries']] == [row['sha256'] for row in executable_rows[2:]]
    jobs = ['ordinary-default', 'owner-default', 'ordinary-cooked', 'owner-cooked']
    debits = [json.loads(line) for line in raw('gpu/debits.jsonl').splitlines()]
    children = release['children']
    assert children == [json.loads(line) for line in raw('gpu/ledger.jsonl').splitlines()]
    assert len(children) == len(debits) == 4 and [row['job'] for row in children] == jobs
    assert [row['job'] for row in debits] == jobs and [row['submits'] for row in debits] == [19, 12, 19, 12]
    assert all(row['attempts'] == 1 and row['maps'] == row['submits'] and not row['refundable'] for row in debits)
    for index, row in enumerate(children):
        assert row['accepted'] and row['exit'] == 0 and row['reaped'] and row['group_empty_after']
        assert all(row['checks'].values()) and row['adapter'] == children[0]['adapter'] and 'backend: Metal' in row['adapter']
        assert row['argv'][0] == plan['binaries'][index // 2]['path']
        assert row['argv'][1:3] == ['--ignored', '--exact']
        assert row['argv'][4:] == ['--nocapture', '--test-threads=1']
        assert row['environment']['keys'] == ['HOME', 'INCANT_EXPECT_GPU_BACKEND', 'PATH', 'RUST_BACKTRACE', 'TMPDIR']
        out, err = raw('gpu/' + row['job'] + '/stdout.txt'), raw('gpu/' + row['job'] + '/stderr.txt')
        assert sha(out) == row['stdout_sha256'] and sha(err) == row['stderr_sha256']
        assert b'test result: ok. 1 passed; 0 failed; 0 ignored;' in out and b'panicked at' not in err
        if row['job'].startswith('ordinary'):
            assert [(int(a), int(b)) for a, b in re.findall(rb'^upscaling mode(\d+) attachment(\d+):', out, re.M)] == [(m, a) for m in range(19) for a in range(2)]
        else:
            assert list(map(int, re.findall(rb'^OWNER_HISTORY case=(\d+)', out, re.M))) == list(range(1, 13))
            assert out.count(b'queue_submits=12 readback_maps=12 max_extent=16x16 no_images=true') == 1
    terminal = obj('gpu/supervisor-terminal.json')
    assert terminal['exit'] == 0 and terminal['reaped'] and terminal['actual_children'] == 4
    return dict(records=len(records), payloads=len(payloads), build_inputs=2467, test_only_paths=4,
                production_wgsl_unchanged=21, cpu=counts, executable_copies=4,
                gpu_children=4, submissions=62, maps=62, images=0,
                adapter=children[0]['adapter'], appearance='not assessed', timing='not assessed')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    archive = HERE / 'records.json.gz'
    if args.verify_only:
        encoded = archive.read_bytes()
        manifest = json.loads((HERE / 'manifest.json').read_text())
        assert sha(encoded) == manifest['archive_sha256']
        result = verify(json.loads(gzip.decompress(encoded)), args.originals, args.local_binaries)
        assert result == manifest['result']
    else:
        store = capture()
        result = verify(store, True, True)
        encoded = gzip.compress(json.dumps(store, sort_keys=True, separators=(',', ':')).encode(), mtime=0)
        archive.write_bytes(encoded)
        (HERE / 'manifest.json').write_text(json.dumps(dict(source=SOURCE, production=PRODUCTION,
            archive_sha256=sha(encoded), result=result), indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
