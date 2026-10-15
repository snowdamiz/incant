#!/usr/bin/env python3
"""Offline cooked-boundary evidence replay; no compiler, player, GPU or capture."""
import argparse
import base64
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    source_root = args.source_root.resolve() if args.source_root else here.parents[3]
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(16 * 1024**2 + 1)
    require(len(payload) <= 16 * 1024**2, 'Archive exceeds bound')
    encoded = json.loads(payload)
    require(len(encoded) == len(manifest['records']) and set(encoded) == {r['key'] for r in manifest['records']}, 'Record membership differs')
    raw = {}
    for row in manifest['records']:
        data = base64.b64decode(encoded[row['key']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original differs: ' + row['key'])
        if row.get('code_input'):
            if args.source_root:
                require((source_root / row['source_path']).read_bytes() == data, 'Live source differs: ' + row['source_path'])
            if args.git_source:
                actual = subprocess.run(['git', 'show', args.git_source + ':' + row['source_path']], cwd=source_root, capture_output=True, check=True).stdout
                require(actual == data, 'Git source differs: ' + row['source_path'])
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['path'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'], 'Binary differs: ' + row['path'])

    def data(key):
        return json.loads(raw[key])

    freeze = data('source-003/freeze.json')
    require(freeze['base'] == manifest['native_source'] and len(freeze['files']) == 185 and not freeze['untracked_paths'], 'Native source scope differs')
    require(sha(raw['source-003/source.diff']) == freeze['review_patch_sha256'], 'Review patch differs')
    final_inputs = {r['path']: r for r in freeze['files']}
    for row in freeze['files']:
        require(len(raw['source/' + row['path']]) == row['bytes'] and sha(raw['source/' + row['path']]) == row['sha256'], 'Frozen source differs: ' + row['path'])
    for index, changed in [(1, {'crates/incant_render/src/metalfx/mod.rs', 'crates/incant_render/src/metalfx/system.rs'}), (2, {'crates/incant_render/src/metalfx/mod.rs'})]:
        old = data(f'source-00{index}/freeze.json')
        require({r['path'] for r in old['files']} == set(final_inputs), 'Predecessor source membership differs')
        require({r['path'] for r in old['files'] if r['sha256'] != final_inputs[r['path']]['sha256']} == changed, 'Predecessor delta differs')
        for row in old['files']:
            key = f'source-00{index}/files/' + row['path'] if row['path'] in changed else 'source/' + row['path']
            require(sha(raw[key]) == row['sha256'], 'Historical input differs')

    cpus = [data(f'cpu-00{i}/results.json') for i in range(1, 4)]
    for index, cpu in enumerate(cpus, 1):
        require(cpu['proposal_sha256'] == sha(raw[f'cpu-proposal-00{index}.json']) and cpu['supervisor_sha256'] == sha(raw[f'cpu-supervisor-00{index}.py']), 'CPU protocol differs')
        require(cpu['supervisor_terminal'] and cpu['native_engine_launches'] == 0 and not cpu['gpu_window_capture'], 'CPU scope differs')
        require(all(g['cleanup_confirmed'] for g in cpu['owned_groups']) and data(f'cpu00{index}-closeout.json')['independent_ps_all_absent'], 'CPU cleanup unconfirmed')
        for name, row in cpu['logs'].items():
            require(len(raw[f'cpu-00{index}/' + name]) == row['bytes'] and sha(raw[f'cpu-00{index}/' + name]) == row['sha256'], 'CPU log differs')
    require(cpus[0]['status'] == 'failed' and cpus[0]['results'][-1]['exit'] == 101 and b'cannot find type `Fallback`' in raw['cpu-001/1.stdout.log'], 'CPU001 compile failure lost')
    require(cpus[1]['status'] == 'failed' and cpus[1]['results'][-1]['exit'] == 101 and b'assertions_on_constants' in raw['cpu-002/5.stderr.log'], 'CPU002 lint failure lost')
    cpu = cpus[2]
    require(cpu['source'] == manifest['native_source'] and cpu['status'] == 'complete' and cpu['exit'] == 0 and len(cpu['results']) == 6, 'Final CPU result differs')
    require(all(r['exit'] == 0 and r['cleanup_confirmed'] and r['terminal_reaped'] for r in cpu['tool_probes'] + cpu['results'] if 'argv' in r), 'Final CPU child unfinished')
    require(cpu['results'][5]['kind'] == 'pure-data' and cpu['results'][5]['status'] == 'complete', 'Pure audit unfinished')

    def counts(key):
        return tuple(map(sum, zip(*[(int(a), int(b)) for a, b in re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw[key])], strict=True)))

    require(counts('cpu-001/0.stdout.log') == (54, 0), 'Cooker coverage differs')
    require(counts('cpu-002/0.stdout.log') == (191, 62) and counts('cpu-002/1.stdout.log') == (224, 62), 'Renderer coverage differs')
    require(all(counts(f'cpu-002/{i}.stdout.log') == (1, 0) for i in (2, 3, 4)) and all(counts(f'cpu-003/{i}.stdout.log') == (1, 0) for i in (0, 1)), 'Admission coverage differs')
    require(cpu['final_target_bytes'] - cpu['cumulative_target_baseline_bytes'] == 649192884 <= 4 * 1024**3, 'Cumulative target charge differs')
    require(b'Ran 39 tests' in raw['pure-controls-001.log'] and b'Ran 4 tests' in raw['compare-controls-001.log'] and b'Ran 3 tests' in raw['harmless-controls-001.log'], 'Pure controls differ')
    default, full, player = cpu['copied_artifacts']
    require('native-metalfx' in default['features'] and 'cooked-metalfx-boundary' not in default['features'], 'Default renderer feature route differs')
    require({'native-metalfx', 'cooked-metalfx-boundary', 'persistent-pipeline-cache'} <= set(full['features']), 'Cooked renderer feature route differs')
    require('native-window' in player['features'] and 'native-metalfx' not in player['features'], 'Compile-only player feature route differs')

    gpu = data('gpu-001/results.json')
    require(gpu['status'] == 'complete' and gpu['exit'] == 0 and gpu['supervisor_terminal'] and len(gpu['arms']) == 2, 'GPU result differs')
    require(gpu['plan_sha256'] == sha(raw['gpu-proposal-001.json']) and gpu['supervisor_sha256'] == sha(raw['gpu-supervisor-001.py']), 'GPU protocol differs')
    require(not gpu['capture'] and gpu['native_player_window_launches'] == 0, 'GPU scope differs')
    require(all(g['cleanup_confirmed'] for g in gpu['owned_groups']) and data('gpu001-closeout.json')['independent_ps_all_absent'], 'GPU cleanup unconfirmed')
    plan = data('gpu-proposal-001.json')
    for index, arm in enumerate(gpu['arms']):
        require(arm['status'] == 'complete' and arm['exit'] == 0 and arm['cleanup_confirmed'], 'GPU child failed')
        require(all(arm[key] == value for key, value in plan['arms'][index].items()), 'GPU arm binding differs')
        require(arm['binary_sha256'] == [default, full][index]['sha256'], 'GPU artifact differs')
        require(b'test result: ok. 1 passed; 0 failed; 0 ignored;' in raw[f'gpu-001/{index}.stdout.log'] and b'17 exact boundary cases; one submit/map' in raw[f'gpu-001/{index}.stdout.log'], 'Actual boundary control missing')
        require(b'Apple M5 Pro' in raw[f'gpu-001/{index}.stderr.log'] and b'backend: Metal' in raw[f'gpu-001/{index}.stderr.log'], 'Actual Metal identity missing')
    for name, row in gpu['retained_files'].items():
        require(len(raw['gpu-001/' + name]) == row['bytes'] and sha(raw['gpu-001/' + name]) == row['sha256'], 'Retained GPU output differs')

    # Reconstruct only saved inputs. These imported analyzers are pure data code.
    with tempfile.TemporaryDirectory(prefix='incant-boundary-evidence-') as temporary:
        root = Path(temporary).resolve()
        generated = root / 'target/aarch64-apple-darwin/ci/build/renderer/out'
        for row in manifest['records']:
            key = row['key']
            if key.startswith('source/'):
                path = root / key.removeprefix('source/')
            elif key.startswith('generated/'):
                path = generated / key.removeprefix('generated/')
            elif key.startswith('gpu-001/') or key == 'compare-boundary.py':
                path = root / key
            else:
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw[key])
        sys.path.insert(0, str(root / 'tools'))
        audit = module('saved_boundary_catalog_audit', root / 'tools/native_shaders.py')
        messages = [json.loads(line) for line in raw['cpu-003/4.stdout.log'].splitlines() if line.startswith(b'{')]
        target = data('cpu-proposal-003.json')['target']
        renderer = next(m['package_id'] for m in messages if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'incant_render')
        for message in messages:
            if message.get('reason') == 'compiler-artifact':
                message['filenames'] = [str(root / 'target' / Path(p).relative_to(target)) for p in message['filenames']]
            if message.get('reason') == 'build-script-executed' and message.get('package_id') == renderer:
                message['out_dir'] = str(generated)
        actual = audit.audit(messages, 'cooked', target_dir=root / 'target', target='aarch64-apple-darwin', profile='ci', physics_mode='fast', root=root)
        require(actual == data('cpu-003/actual-shaders.json'), 'Actual compiler-output audit differs')
        require(len(actual['families']) == 25 and sum(len(v['artifacts']) for v in actual['families'].values()) == 144 and sum(v['artifact_bytes'] for v in actual['families'].values()) == 632455, 'Catalog differs')
        compare = module('saved_boundary_compare', root / 'compare-boundary.py')
        result = compare.compare(root / 'gpu-001/default', root / 'gpu-001/aggregate')
        require(result == data('gpu-001/comparison.json') == gpu['comparison'], 'Boundary saved-byte comparison differs')
        require(result['cases'] == 17 and result['raw_bytes_each'] == 87040 and result['tolerance_codes'] == 0 and not result['vendor_scaler_execution'], 'Boundary scope differs')
    print(f'PASS {len(raw)} records;54cooker/191default/224full/3admission; both original CPU failures retained;25families/144artifacts;2Metal children/17cases/87040exact bytes; player compile-only; no vendor/window/capture/gate claim')


if __name__ == '__main__':
    main()
