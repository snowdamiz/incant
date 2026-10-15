#!/usr/bin/env python3
"""Offline SSAO evidence replay; never launches a compiler, player or GPU."""
import argparse
import base64
import gzip
import hashlib
import importlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(ok, message):
    if not ok:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) <= 32 * 1024**2, 'Replay archive exceeds bound')
    records = json.loads(payload)
    require(len(records) == len(manifest['records']) and set(records) == {r['key'] for r in manifest['records']}, 'Record membership differs')
    raw = {}
    source_root = args.source_root.resolve() if args.source_root else here.parents[3]
    for row in manifest['records']:
        data = base64.b64decode(records[row['key']], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original differs: ' + row['key'])
        if row.get('code_input'):
            if args.source_root:
                require((source_root / row['source_path']).read_bytes() == data, 'Live source differs: ' + row['source_path'])
            if args.git_source:
                result = subprocess.run(['git', 'show', args.git_source + ':' + row['source_path']], cwd=source_root, check=True, capture_output=True)
                require(result.stdout == data, 'Git source differs: ' + row['source_path'])
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['path'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'], 'Retained binary differs: ' + row['path'])

    def data(key):
        return json.loads(raw[key])

    for index in (1, 2):
        freeze = data(f'source-00{index}/freeze.json')
        require(freeze['base'] == manifest['producing_base'] and len(freeze['files']) == 123, 'Producing source scope differs')
        require(digest(raw[f'source-00{index}/source.diff']) == freeze['tracked_patch_sha256'], 'Frozen patch differs')
        for row in freeze['files']:
            older = 'source001-only/' + row['path']
            key = older if index == 1 and older in raw else 'source/' + row['path']
            require(digest(raw[key]) == row['sha256'], 'Frozen source differs: ' + key)
    require(digest(raw['source/crates/incant_shader_cook/tests/fixtures/material_emit_main346.rs']) == '976f76dfdc879dcf2d72af59673e7b5faeb0a28994dcdecdb6ec27403e7c4c72', 'Historical ordinary emitter fixture differs')
    final_source = data('final-source.json')
    require(final_source['source_inputs_committed_as'] == manifest['source_inputs_committed_as'] and final_source['source_rows_byte_identical_to_commit'] == 123, 'Final byte-equivalent source differs')

    # Positive controls were added after the unchanged production commit. Keep
    # both pre-Clippy and final iterator-only source snapshots reproducible.
    for index in (3, 4):
        freeze = data(f'source-00{index}/freeze.json')
        require(freeze['base'] == manifest['source_inputs_committed_as'] and len(freeze['files']) == 125, 'Positive source scope differs')
        require(digest(raw[f'source-00{index}/source.diff']) == freeze['tracked_patch_sha256'], 'Positive tracked patch differs')
        for row in freeze['files']:
            legacy = 'positive-source003-only/' + row['path']
            positive = 'positive-source/' + row['path']
            key = legacy if index == 3 and legacy in raw else positive if positive in raw else 'source/' + row['path']
            require(digest(raw[key]) == row['sha256'], 'Positive source differs: ' + key)
    positive_source = data('positive-final-source.json')
    require(positive_source['source_inputs_committed_as'] == manifest['positive_source_inputs_committed_as'] and positive_source['source_rows_byte_identical_to_commit'] == 125, 'Positive final source differs')

    cpu = [data(f'cpu-00{i}/results.json') for i in (1, 2)]
    for index, receipt in enumerate(cpu, 1):
        prefix = f'cpu-00{index}/'
        require(receipt['proposal_sha256'] == digest(raw[f'cpu-proposal-00{index}.json']) and receipt['supervisor_sha256'] == digest(raw[f'cpu-supervisor-00{index}.py']), 'CPU protocol differs')
        require(receipt['native_engine_launches'] == 0 and not receipt['gpu_window_capture'] and receipt['supervisor_terminal'], 'Unexpected CPU execution/unfinished supervisor')
        require(all(g['cleanup_confirmed'] for g in receipt['owned_groups']) and all(r.get('terminal_reaped') for r in receipt['tool_probes'] + receipt['results'] if 'argv' in r), 'CPU ownership unfinished')
        for name, row in receipt['logs'].items():
            require(len(raw[prefix + name]) == row['bytes'] and digest(raw[prefix + name]) == row['sha256'], 'CPU raw log differs')
    require(cpu[0]['status'] == 'failed' and [r['exit'] for r in cpu[0]['results']] == [0, 0, 101] and b'E0433' in raw['cpu-001/2.stdout.log'] and b'E0609' in raw['cpu-001/2.stdout.log'], 'Original compile failure missing')
    require(cpu[1]['status'] == 'complete' and all(r.get('exit', 0) == 0 and r['status'] == 'complete' for r in cpu[1]['results']), 'Final CPU failed')
    def count(key):
        matches = re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw[key])
        return sum(int(a) for a, _ in matches), sum(int(b) for _, b in matches)
    require(count('cpu-001/0.stdout.log') == (47, 0) and b'ordinary_material_artifacts_are_byte_identical_to_main346_emitter ... ok' in raw['cpu-001/0.stdout.log'], 'Cooker/ordinary artifact regression differs')
    require(count('cpu-001/1.stdout.log') == (23, 3) and count('cpu-002/0.stdout.log') == (212, 55), 'Renderer CPU coverage differs')
    require(count('cpu-002/1.stdout.log') == (1, 0) and count('cpu-002/2.stdout.log') == (1, 0), 'Partial selection refusal differs')
    require(b'Ran 23 tests' in raw['source-check-0.log'] and b'Ran 4 tests' in raw['source-check-1.log'] and b'Ran 3 tests' in raw['harmless-controls-001.log'], 'Pure/lifecycle coverage differs')
    require(cpu[1]['final_target_bytes'] - cpu[0]['initial_target_bytes'] == 226198595 and cpu[1]['final_target_bytes'] - cpu[0]['initial_target_bytes'] <= 4 * 1024**3, 'Cumulative target bound differs')

    for index in (3, 4):
        receipt = data(f'cpu-00{index}/results.json')
        prefix = f'cpu-00{index}/'
        require(receipt['proposal_sha256'] == digest(raw[f'cpu-proposal-00{index}.json']) and receipt['supervisor_sha256'] == digest(raw[f'cpu-supervisor-00{index}.py']), 'Positive CPU protocol differs')
        require(receipt['native_engine_launches'] == 0 and not receipt['gpu_window_capture'] and receipt['supervisor_terminal'], 'Positive CPU execution unfinished')
        require(all(g['cleanup_confirmed'] for g in receipt['owned_groups']) and all(r.get('terminal_reaped') for r in receipt['tool_probes'] + receipt['results']), 'Positive CPU cleanup unfinished')
        for name, row in receipt['logs'].items():
            require(len(raw[prefix + name]) == row['bytes'] and digest(raw[prefix + name]) == row['sha256'], 'Positive CPU raw log differs')
        require(count(prefix + '0.stdout.log') == (2, 1) and count(prefix + '1.stdout.log') == (2, 1), 'Positive imported/metric CPU coverage differs')
        if index == 3:
            require(receipt['status'] == 'failed' and [r['exit'] for r in receipt['results']] == [0, 0, 101] and b'chunks_exact_to_as_chunks' in raw[prefix + '2.stderr.log'], 'Original strict iterator refusal missing')
        else:
            require(receipt['status'] == 'complete' and [r['exit'] for r in receipt['results']] == [0, 0, 0, 0], 'Final positive CPU/lint failed')
            require(receipt['final_target_bytes'] - cpu[0]['initial_target_bytes'] == 255667777 and receipt['final_target_bytes'] - cpu[0]['initial_target_bytes'] <= 4 * 1024**3, 'Final cumulative target bound differs')
    require(b'Ran 4 tests' in raw['compare-controls-002.log'] and b'\nOK\n' in raw['compare-controls-002.log'], 'Independent comparison refusal controls missing')

    # Re-run only the actual target-output data audit. Paths are relocated;
    # sources, generated bytes and compiler features stay exact.
    with tempfile.TemporaryDirectory(prefix='incant-ssao-evidence-') as temporary:
        root = Path(temporary).resolve()
        generated = root / 'target/aarch64-apple-darwin/ci/build/renderer/out'
        for row in manifest['records']:
            if row.get('source_path'):
                path = root / row['source_path']
            elif row['key'].startswith('generated/'):
                path = generated / row['key'].removeprefix('generated/')
            else:
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw[row['key']])
        sys.path.insert(0, str(root / 'tools'))
        audit = importlib.import_module('native_shaders')
        messages = [json.loads(line) for line in raw['cpu-002/4.stdout.log'].splitlines() if line.startswith(b'{')]
        target = data('cpu-proposal-002.json')['target']
        renderer = next(m['package_id'] for m in messages if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'incant_render')
        for row in messages:
            if row.get('reason') == 'compiler-artifact':
                row['filenames'] = [str(root / 'target' / Path(p).relative_to(target)) for p in row['filenames']]
            if row.get('reason') == 'build-script-executed' and row.get('package_id') == renderer:
                row['out_dir'] = str(generated)
        actual = audit.audit(messages, 'cooked', target_dir=root / 'target', target='aarch64-apple-darwin', profile='ci', physics_mode='fast', root=root)
        require(actual == data('cpu-002/actual-shaders.json'), 'Actual generated audit differs')
        require(len(actual['families']) == 17 and sum(len(v['artifacts']) for v in actual['families'].values()) == 99 and sum(v['artifact_bytes'] for v in actual['families'].values()) == 427087, 'Catalog coverage differs')
        require(sum(len(v['artifacts']) for k, v in actual['families'].items() if k.startswith('ssao-')) == 30, 'SSAO family incomplete')

    gpu = data('gpu-001/results.json')
    require(gpu['status'] == 'complete' and gpu['exit'] == 0 and gpu['supervisor_terminal'] and len(gpu['arms']) == 4 and not gpu['capture'] and gpu['native_engine_player_window_launches'] == 0, 'GPU scope/result differs')
    require(gpu['plan_sha256'] == digest(raw['gpu-proposal-001.json']) and gpu['supervisor_sha256'] == digest(raw['gpu-supervisor-001.py']), 'GPU protocol differs')
    require(all(g['cleanup_confirmed'] for g in gpu['owned_groups']) and data('gpu001-closeout.json')['independent_ps_all_absent'], 'GPU cleanup unfinished')
    for index, arm in enumerate(gpu['arms']):
        require(arm['status'] == 'complete' and arm['exit'] == 0 and arm['cleanup_confirmed'] and arm == dict(data('gpu-proposal-001.json')['arms'][index], **{k: arm[k] for k in ('status', 'start_unix', 'exit', 'cleanup_confirmed', 'end_unix')}), 'GPU arm/argv differs')
        out, err = raw[f'gpu-001/{index}.stdout.log'], raw[f'gpu-001/{index}.stderr.log']
        require(re.search(rb'test result: ok\. 1 passed; 0 failed; 0 ignored;', out) and b'Incant required GPU backend: Metal;' in err and b'Apple M5 Pro' in err and b'backend: Metal' in err, 'Actual Metal single-test proof missing')
    for name, row in gpu['logs'].items():
        require(len(raw['gpu-001/' + name]) == row['bytes'] and digest(raw['gpu-001/' + name]) == row['sha256'], 'GPU raw log differs')
    require(manifest['qualification']['native_player_window_capture_runs'] == 0, 'Unexpected player/window/capture claim')

    positive_gpu = data('gpu-002/results.json')
    positive_plan = data('gpu-proposal-002.json')
    require(positive_gpu['status'] == 'complete' and positive_gpu['exit'] == 0 and positive_gpu['supervisor_terminal'] and len(positive_gpu['arms']) == 2 and not positive_gpu['capture'] and positive_gpu['native_player_window_launches'] == 0, 'Positive GPU scope/result differs')
    require(positive_gpu['plan_sha256'] == digest(raw['gpu-proposal-002.json']) and positive_gpu['supervisor_sha256'] == digest(raw['gpu-supervisor-002.py']), 'Positive GPU protocol differs')
    require(all(g['cleanup_confirmed'] for g in positive_gpu['owned_groups']) and data('gpu002-closeout.json')['independent_ps_all_absent'], 'Positive GPU cleanup unfinished')
    for index, arm in enumerate(positive_gpu['arms']):
        require(arm['status'] == 'complete' and arm['exit'] == 0 and arm['cleanup_confirmed'] and arm == dict(positive_plan['arms'][index], **{k: arm[k] for k in ('status', 'start_unix', 'exit', 'cleanup_confirmed', 'end_unix')}), 'Positive GPU arm differs')
        out, err = raw[f'gpu-002/{index}.stdout.log'], raw[f'gpu-002/{index}.stderr.log']
        require(re.search(rb'test result: ok\. 1 passed; 0 failed; 0 ignored;', out) and b'Incant required GPU backend: Metal;' in err and b'Apple M5 Pro' in err and b'backend: Metal' in err, 'Actual positive Metal proof missing')
    for name, row in positive_gpu['retained_files'].items():
        require(len(raw['gpu-002/' + name]) == row['bytes'] and digest(raw['gpu-002/' + name]) == row['sha256'], 'Positive GPU retained bytes differ')
    with tempfile.TemporaryDirectory(prefix='incant-ssao-positive-replay-') as temporary:
        root = Path(temporary)
        for key, content in raw.items():
            if key.startswith('gpu-002/default/') or key.startswith('gpu-002/aggregate/') or key == 'compare-positive.py':
                path = root / key
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
        spec = importlib.util.spec_from_file_location('incant_retained_positive_compare', root / 'compare-positive.py')
        compare = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(compare)
        actual = compare.compare(root / 'gpu-002/default', root / 'gpu-002/aggregate')
        require(actual == positive_gpu['comparison'] == data('gpu-002/comparison.json'), 'Independent retained-byte comparison differs')
        require(actual['effect_pixels'] == [1231, 1231] and actual['chromatic_pixels'] == [972, 972] and actual['tolerance_codes'] == 0, 'Positive fixture observability differs')
    print(f'PASS {len(records)} records; cooker47/defaultAO23/full212/partial1+1; positiveCPU2+2/lints; retained compile+Clippy failures;17families/99artifacts;6Metal controls/exact3frame parity; no player/window/capture or gate claim')


if __name__ == '__main__':
    main()
