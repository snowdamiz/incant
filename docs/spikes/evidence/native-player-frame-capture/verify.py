#!/usr/bin/env python3
"""Replay retained frame-capture evidence without a compiler, player or GPU."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = args.source_root.resolve() if args.source_root else here.parents[3]
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
        value = base64.b64decode(encoded[row['key']], validate=True)
        require(len(value) == row['bytes'] and sha(value) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = value
        if args.originals:
            require(Path(row['original']).read_bytes() == value, 'Original differs: ' + row['key'])
    if args.binaries:
        for row in manifest['binaries']:
            path = Path(row['copy'])
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == row['bytes'] and actual == row['sha256'], 'Executable differs')

    def data(key):
        return json.loads(raw[key])

    old = data('source-002/freeze.json')
    frozen = data('source-003/freeze.json')
    require(old['source'] == manifest['production_source'] and frozen['source'] == manifest['final_native_source'], 'Source identity differs')
    original = {row['path']: row for row in old['inputs']}
    final = {row['path']: row for row in frozen['inputs']}
    require(len(final) == 4047 and set(final) == set(original), 'Input inventory differs')
    metal = 'crates/incant_game_player/src/capture/metal.rs'
    require({p for p in final if final[p]['sha256'] != original[p]['sha256']} == {metal}, 'Successor source delta differs')
    require(sha(raw['historical/metal.rs']) == original[metal]['sha256'], 'Failed fixture source differs')
    for row in manifest['records']:
        if row.get('code_input'):
            require(sha(raw[row['key']]) == final[row['source_path']]['sha256'], 'Code record differs from final inventory')
    require(b'"version":1' in raw['historical/metal.rs'] and b'"version":2' in raw['source/' + metal], 'Version correction differs')
    require(b'if recording.version == 1' in raw['source/crates/incant_input/src/recording.rs'], 'Runtime version admission missing')
    if args.originals:
        for row in frozen['inputs']:
            require(sha(Path(row['copy']).read_bytes()) == row['sha256'], 'Frozen original differs: ' + row['path'])
    if args.source_root:
        for row in frozen['inputs']:
            require(sha((root / row['path']).read_bytes()) == row['sha256'], 'Live producing input differs: ' + row['path'])
    if args.git_source:
        # One bounded batch reads the exact declared inputs; no checkout or build.
        request = ''.join(args.git_source + ':' + path + '\n' for path in final)
        result = subprocess.run(['git', 'cat-file', '--batch'], input=request.encode(), cwd=root, capture_output=True, check=True)
        offset = 0
        for path, row in final.items():
            end = result.stdout.index(b'\n', offset)
            header = result.stdout[offset:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'Git input unavailable: ' + path)
            length = int(header[2])
            start = end + 1
            require(length == row['bytes'] and sha(result.stdout[start:start + length]) == row['sha256'], 'Git input differs: ' + path)
            require(result.stdout[start + length:start + length + 1] == b'\n', 'Git framing differs')
            offset = start + length + 1
        require(offset == len(result.stdout), 'Unexpected Git input output')

    def counts(key):
        return [(int(a), int(b)) for a, b in re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw[key])]

    cpus = [data(f'cpu-00{i}/results.json') for i in (1, 2)]
    for index, cpu in enumerate(cpus, 1):
        require(cpu['status'] == 'complete' and cpu['exit'] == 0 and cpu['supervisor_terminal'], 'CPU qualification incomplete')
        require(cpu['proposal_sha256'] == sha(raw[f'cpu-proposal-00{index}.json']) and cpu['supervisor_sha256'] == sha(raw[f'cpu-supervisor-00{index}.py']), 'CPU protocol differs')
        require(cpu['GPU_window_capture_or_player_executions'] == 0, 'CPU execution scope differs')
        require(all(stage['exit'] == 0 and stage['terminal_reaped'] for stage in cpu['stages']), 'CPU child unfinished')
        require(all(group['cleanup_confirmed'] for group in cpu['owned_groups']), 'CPU cleanup unconfirmed')
        for name, row in cpu['logs'].items():
            require(len(raw[f'cpu-00{index}/' + name]) == row['bytes'] and sha(raw[f'cpu-00{index}/' + name]) == row['sha256'], 'CPU log differs')
        require(cpu['final_target_bytes'] <= cpu['capability_target_ceiling_bytes'] and cpu['minimum_free_bytes'] >= 6 * 1024**3, 'CPU resource bound exceeded')
    require(cpus[0]['source'] == manifest['production_source'] and len(cpus[0]['stages']) == 6, 'Original CPU scope differs')
    require(cpus[1]['source'] == manifest['final_native_source'] and len(cpus[1]['stages']) == 2, 'Successor CPU scope differs')
    require([counts(f'cpu-001/{n}.stdout.log') for n in (0, 1, 2)] == [[(5, 0)], [(30, 1)], [(1, 0)]], 'Original CPU coverage differs')
    require(counts('cpu-002/0.stdout.log') == [(1, 0)], 'Actual CPU baseline regression missing')
    require(data('cpu001-closeout.json')['independent_PID_and_group_absence'] and data('cpu002-closeout.json')['independently_absent_pid_and_groups'], 'CPU absence receipt missing')
    require(data('cpu002-closeout.json')['cumulative_copy_bytes'] == 66030864 <= 128 * 1024**2, 'Copy account differs')
    require(cpus[1]['final_target_bytes'] - cpus[0]['initial_target_bytes'] == 458017095 <= 2 * 1024**3, 'Cumulative target charge differs')
    require(len(data('protocol-controls-001.json')['passed']) == 3 and all(r['passed'] for r in data('protocol-controls-001.json')['passed']), 'Lifecycle controls differ')
    require(b'No such file or directory' in raw['metadata-001.stderr.log'], 'Initial metadata PATH refusal lost')

    wanted = {'default', 'frame-capture', 'physics-deterministic', 'render-gpu', 'render-input'}
    for index, cpu in enumerate(cpus):
        artifact, = cpu['copied_artifacts']
        require(set(artifact['features']) == wanted and artifact == manifest['binaries'][index], 'Actual artifact feature route differs')
    for index in (1, 2):
        gpu = data(f'gpu-00{index}/results.json')
        require(gpu['proposal_sha256'] == sha(raw[f'gpu-proposal-00{index}.json']) and gpu['supervisor_sha256'] == sha(raw[f'gpu-supervisor-00{index}.py']), 'GPU protocol differs')
        require(gpu['supervisor_terminal'] and gpu['child_terminal_reaped'] and gpu['GPU_test_children'] == 1, 'GPU child unfinished')
        require(gpu['compilation_window_capture_helper_executions'] == 0 and gpu['independent_owned_PID_and_group_absence'] and all(g['cleanup_confirmed'] for g in gpu['owned_groups']), 'GPU cleanup/scope differs')
        require(gpu['binary'] == cpus[index - 1]['copied_artifacts'][0], 'GPU artifact differs')
        for name, row in gpu['logs'].items():
            require(len(raw[f'gpu-00{index}/' + name]) == row['bytes'] and sha(raw[f'gpu-00{index}/' + name]) == row['sha256'], 'GPU log differs')
    failed = data('gpu-001/results.json')
    require(failed['status'] == 'failed' and failed['child_exit'] == 101 and b'Err` value: Version' in raw['gpu-001/stderr.log'], 'Original actual test failure lost')
    require(not data('gpu001-closeout.json')['renderer_or_capture_assertions_reached'], 'Original failure overclaimed')
    gpu = data('gpu-002/results.json')
    require(gpu['status'] == 'complete' and gpu['exit'] == gpu['child_exit'] == 0, 'Actual Metal control failed')
    require(counts('gpu-002/stdout.log') == [(1, 0)] and data('gpu002-closeout.json')['actual_GPU_tests_passed'] == 1, 'Actual Metal control missing')
    require(data('gpu-proposal-002.json')['bounds']['child_seconds'] == 119 and data('gpu-proposal-002.json')['bounds']['sequence_seconds'] == 179, 'Failed attempt time charge lost')
    require(gpu['prior_accounting']['prior_cumulative_log_bytes'] == 1265131 and gpu['prior_GPU_output_bytes'] == 2785, 'Prior GPU resource charge lost')
    code = raw['source/' + metal]
    require(b'incant_render::wgpu::Backend::Metal' in code and b'"screen UI must change real readback bytes"' in code, 'Actual backend/UI assertions missing')
    require(b'[(0, 1, 0), (2, 1, 2), (3, 2, 1), (4, 2, 2)]' in code and b'VM continuity' in code, 'Restart/VM assertions missing')
    print(f'PASS {len(raw)} records; 4047 source inputs; 36 original CPU executions + 1 baseline regression; strict lint/native-window compile-only; original GPU version failure retained; 1 actual Metal sampled PNG/UI/PCM/restart control; no retained-image/window/performance claim')


if __name__ == '__main__':
    main()
