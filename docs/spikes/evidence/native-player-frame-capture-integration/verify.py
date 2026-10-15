#!/usr/bin/env python3
"""Replay composition evidence as saved bytes; no compiler, player or GPU."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def sha(value):
    return hashlib.sha256(value).hexdigest()


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
    legacy = here.parent / 'native-player-frame-capture'
    for name, expected in manifest['historical_packet'].items():
        require(sha((legacy / name).read_bytes()) == expected, 'Historical packet changed')
    # Execute only the checked-in, hash-bound data verifier, never archived code.
    command = [sys.executable, str(legacy / 'verify.py')]
    if args.originals:
        command.append('--originals')
    if args.binaries:
        command.append('--binaries')
    subprocess.run(command, check=True, cwd=root)
    require(sha((here / 'records.json.gz').read_bytes()) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(16 * 1024**2 + 1)
    require(len(payload) <= 16 * 1024**2, 'Archive exceeds bound')
    encoded = json.loads(payload)
    require(set(encoded) == {r['key'] for r in manifest['records']} and len(encoded) == len(manifest['records']), 'Record inventory differs')
    raw = {}
    for row in manifest['records']:
        value = base64.b64decode(encoded[row['key']], validate=True)
        require(len(value) == row['bytes'] and sha(value) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = value
        if args.originals:
            require(Path(row['original']).read_bytes() == value, 'Original differs: ' + row['key'])

    def data(key):
        return json.loads(raw[key])

    source = data('source-005/freeze.json')
    require(source['source'] == manifest['native_source'] == '7328884bd0b05dbce687c85e41f78ecd1373aedb', 'Native identity differs')
    require(source['joined_main'] == manifest['joined_main'] and len(source['inputs']) == 4081, 'Composition inventory differs')
    require(source['inputs'] == data('source-004/freeze.json')['inputs'] and source['history_only_predecessor']['source_tree_unchanged'], 'History-only join differs')
    inventory = {row['path']: row for row in source['inputs']}
    for row in manifest['records']:
        if 'source_path' in row:
            require(sha(raw[row['key']]) == inventory[row['source_path']]['sha256'], 'Source record differs')
    if args.originals or args.source_root:
        for row in source['inputs']:
            if args.originals:
                require(sha(Path(row['copy']).read_bytes()) == row['sha256'], 'Frozen source differs: ' + row['path'])
            if args.source_root:
                require(sha((root / row['path']).read_bytes()) == row['sha256'], 'Live producing input differs: ' + row['path'])
    if args.git_source:
        request = ''.join(args.git_source + ':' + path + '\n' for path in inventory)
        result = subprocess.run(['git', 'cat-file', '--batch'], cwd=root, input=request.encode(), capture_output=True, check=True)
        offset = 0
        for path, row in inventory.items():
            end = result.stdout.index(b'\n', offset)
            header = result.stdout[offset:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'Missing Git input: ' + path)
            length = int(header[2])
            start = end + 1
            require(length == row['bytes'] and sha(result.stdout[start:start + length]) == row['sha256'], 'Git input differs: ' + path)
            require(result.stdout[start + length:start + length + 1] == b'\n', 'Git framing differs')
            offset = start + length + 1
        require(offset == len(result.stdout), 'Extra Git output')
    binary = manifest['binary']
    if args.binaries:
        path = Path(binary['copy'])
        with path.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        require(path.stat().st_size == binary['bytes'] and actual == binary['sha256'], 'Copied executable differs')

    cpu, gpu = data('cpu-004/results.json'), data('gpu-003/results.json')
    for label, result in [('cpu-004', cpu), ('gpu-003', gpu)]:
        number = label[-3:]
        kind = label.split('-')[0]
        require(result['source'] == manifest['native_source'] and result['status'] == 'complete' and result['exit'] == 0 and result['supervisor_terminal'], 'Native result differs')
        require(result['proposal_sha256'] == sha(raw[f'{kind}-proposal-{number}.json']) and result['supervisor_sha256'] == sha(raw[f'{kind}-supervisor-{number}.py']), 'Protocol differs')
        require(all(g['cleanup_confirmed'] for g in result['owned_groups']), 'Cleanup unconfirmed')
        for name, row in result['logs'].items():
            require(len(raw[label + '/' + name]) == row['bytes'] and sha(raw[label + '/' + name]) == row['sha256'], 'Native log differs')
    require(len(cpu['stages']) == 6 and all(s['exit'] == 0 and s['terminal_reaped'] for s in cpu['stages']), 'CPU stage scope differs')
    counts = [re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw[f'cpu-004/{n}.stdout.log']) for n in range(3)]
    require(counts == [[(b'6', b'1')], [(b'2', b'1')], [(b'5', b'0')]], 'CPU coverage differs')
    require(cpu['copied_artifacts'] == [binary] and gpu['binary'] == binary, 'Actual GPU artifact differs')
    require(set(binary['features']) == {'default', 'frame-capture', 'physics-deterministic', 'render-gpu', 'render-input'}, 'Actual feature route differs')
    close = data('cpu004-closeout.json')
    require(close['independently_absent_pid_and_groups'] and close['wrapper_terminal_reaped'] and close['minor_supervisor_wording_limit'], 'CPU closeout missing')
    require(close['actual_checks']['CPU_source_free_CLI_children_executed_by_passing_material_controls'] == cpu['CPU_player_CLI_children_if_integration_tests_pass'] == 5 and cpu['GPU_window_capture_helper_executions'] == 0, 'CPU process scope differs')
    require(close['cumulative_copy_bytes'] == 99724528 <= 128 * 1024**2 and close['cumulative_log_bytes'] == 1753328 <= 128 * 1024**2, 'Cumulative resource charges differ')
    require(close['prior_CPU_seconds'] == 361 and close['prior_GPU_seconds'] == 3 and close['cumulative_CPU_GPU_seconds'] == 531, 'Prior elapsed charges lost')
    require(cpu['capability_baseline_bytes'] == 61993234651 and cpu['capability_target_ceiling_bytes'] == 64140718299 and cpu['final_target_bytes'] == 62738066922, 'Target account differs')
    require(gpu['child_exit'] == 0 and gpu['child_terminal_reaped'] and gpu['independent_owned_PID_and_group_absence'] and gpu['GPU_test_children'] == 1 and gpu['compilation_window_capture_helper_executions'] == 0, 'GPU process scope differs')
    require(re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', raw['gpu-003/stdout.log']) == [(b'1', b'0')], 'Actual Metal control missing')
    require(data('gpu003-closeout.json')['independently_absent_pid_and_groups'] and data('gpu003-closeout.json')['cumulative_GPU_conservative_seconds'] == 5, 'GPU absence/account missing')
    plan = data('gpu-proposal-003.json')
    require(plan['bounds']['child_seconds'] == 117 and plan['bounds']['sequence_seconds'] == 177 and gpu['prior_GPU_output_bytes'] == 5629, 'Prior GPU charges lost')
    code = raw['source/crates/incant_game_player/src/lib.rs']
    require(code.index(b'capture::Options::split') < code.index(b'materials::Options::split') < code.index(b'checkpoint::Options::split') < code.index(b'package::expand'), 'Parser composition order differs')
    require(b'actual_player_rejects_material_modes_before_splitters_can_hide_them' in raw['source/crates/incant_game_player/src/capture/tests.rs'], 'Entry-point regression missing')
    print(f'PASS {len(raw)} additive records; 4081 joined inputs; 13CPU/5CPU CLI children/2ignored/2strict lints/native-window compile-only; 1same-source Metal control; historical failure retained; no window/appearance/performance claim')


if __name__ == '__main__':
    main()
