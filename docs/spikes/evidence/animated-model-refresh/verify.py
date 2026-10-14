#!/usr/bin/env python3
"""Offline receipt/source/pixel replay; never invokes a build, engine or GPU."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def counts(raw):
    rows = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed', raw)
    require(all(int(failed) == 0 for _, failed in rows), 'No passing-summary failures')
    return [int(passed) for passed, _ in rows]


def strict(pixel, channel):
    return pixel[channel] > 150 and pixel[3] > 200 and all(
        pixel[channel] - pixel[other] > 128 for other in range(3) if other != channel)


def replay(manifest, data):
    read = lambda name: json.loads(data[name])
    # Historical freezes remain historical. Only final code inputs are compared
    # to the current checkout; the documentation successor is intentionally separate.
    for version, source in manifest['source_by_freeze'].items():
        prefix = f'source-{version}/'
        freeze = read(prefix + 'freeze.json')
        require(freeze['source_sha'] == source and freeze['clean'] is True, 'Frozen source identity')
        for row in freeze['records']:
            raw = data[prefix + 'files/' + row['path']]
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Frozen copied input')
    expected_lengths = {'001': 1, '002': 4, '003': 3, '004': 2, '005': 2}
    for version, length in expected_lengths.items():
        receipt = read(f'cpu-source{version}-results.json')
        proposal = read(f'cpu-proposal-{version}.json')
        launch = read(f'cpu-launch-{version}.json')
        require(receipt['source_sha'] == proposal['source_sha'] == launch['source_sha']
                == manifest['source_by_freeze'][version], 'CPU producing source')
        require(len(receipt['results']) == length, 'Stop-first-failure command count')
        require(sha(data[f'cpu-proposal-{version}.json']) == launch['proposal_sha256']
                and sha(data[f'cpu-supervisor-{version}.py']) == launch['supervisor_sha256'],
                'Launched proposal/supervisor bytes')
        require(all(not value for value in launch['ambient_override_values'].values()), 'Ambient override refusal')
        require(launch['build_env'] == {'RUSTUP_TOOLCHAIN': '1.99.0',
                'CARGO_BUILD_JOBS': '2', 'RAYON_NUM_THREADS': '2'}, 'Pinned bounded build environment')
        require(all(identity['exit_code'] == 0 for identity in launch['identities'].values()), 'Host identity capture')
        for index, row in enumerate(receipt['results']):
            log = data[Path(row['log']).name]
            require(row['argv'] == proposal['commands'][index]['argv'], 'Actual command selection')
            require(row['reaped'] is True and sha(log) == row['log_sha256'], 'CPU reap/raw log identity')
            failed = version == '001' or (version == '003' and index == 2)
            require(row['exit_code'] == (101 if failed else 0), 'Distinct actual CPU outcome')
        if version == '001':
            raw = data['cpu-source001-0.raw.log']
            require(b'E0608' in raw and b'cannot index into a value' in raw and not counts(raw),
                    'Original iterator compile failure, no passed tests')
        if version == '002':
            actual = [counts(data[Path(row['log']).name]) for row in receipt['results']]
            require(actual == [[1, 2, 8, 11, 5, 4, 5, 5, 4, 1, 6, 2], [1, 1, 2, 5], [0], []],
                    'Native54 + Play9; library zero-test compilation; strict lint')
        if version == '003':
            require([counts(data[Path(r['log']).name]) for r in receipt['results'][:2]] == [[1], [1]],
                    'Two exact hosted controls')
            raw = data['cpu-source003-2.raw.log']
            require(b'left: ([11.5, 1.0, 0.0], [12.0, 1.0, 0.0])' in raw
                    and b'right: ([11.5, 0.0, 0.0], [12.0, 0.0, 0.0])' in raw
                    and b'test result: FAILED. 0 passed; 1 failed' in raw, 'Original wrong-y fixture failure')
        if version in ('004', '005'):
            require([counts(data[Path(r['log']).name]) for r in receipt['results']] == [[1], []],
                    'Narrow fixture repeat + strict lint only')
    # Final animated production was already exercised by native/Play source002.
    final_freeze = read('source-005/freeze.json')
    source002 = {r['path']: r['sha256'] for r in read('source-002/freeze.json')['records']}
    for row in final_freeze['records']:
        if '/src/' in row['path']:
            require(row['sha256'] == source002[row['path']], 'No production successor change')
    coverage = manifest['accepted_cpu_coverage']
    require([coverage[k] for k in ('native_source002', 'Play_source002', 'hosted_source003',
            'keys_only_fixture_source005', 'distinct_scoped_controls')] == [54, 9, 2, 1, 66],
            'Scoped66 coverage without counting fixture repetition')
    require(read('source-relevance-main297.json')['changed_paths'] == [], 'Recorded scoped main297 source closure')
    for run, prefix in [('001', 'gpu-source004-001/'), ('002', 'gpu-source005-002/')]:
        result = read(prefix + 'result.json')
        launch = read(prefix + 'launch.json')
        proposal = read(f'gpu-proposal-{run}.json')
        grant = read(f'gpu-grant-{run}.json')
        log = data[prefix + 'output.log']
        require(result['source_sha'] == launch['source_sha'] == proposal['source_sha']
                == manifest['gpu_sources'][run], 'Actual GPU source')
        require(result['argv'] == launch['argv'] == proposal['argv'], 'Exact ignored GPU control')
        require(result['binary']['sha256'] == grant['binary_sha256'] == proposal['binary']['sha256']
                and grant['source_sha'] == result['source_sha'] and grant['authorized'] is True,
                'Matching copied artifact/root grant')
        require(sha(data[f'gpu-proposal-{run}.json']) == launch['proposal_sha256']
                and sha(data[f'gpu-supervisor-{run}.py']) == launch['supervisor_sha256']
                and sha(data[f'gpu-grant-{run}.json']) == launch['grant_sha256'], 'GPU launch byte identities')
        require(proposal['child_timeout_seconds'] == 120 and proposal['execution_deadline_seconds'] == 180,
                'Original child/cooperative execution bounds')
        require(result['managed_child_reaped'] is True and sha(log) == result['output_log_sha256'],
                'Actual GPU terminal/raw identity')
        require(b'backend: Metal' in log and b'Apple M5 Pro' in log, 'Observed Mac Metal adapter')
        require(result['exit_code'] == (101 if run == '001' else 0), 'Failure/pass kept distinct')
        if run == '001':
            require(not result['outputs'] and b'Host(Consumer(Render(Disabled)))' in log,
                    'Unprepared motion startup failure; no pixel/refresh claim')
        else:
            for row in result['outputs']:
                require(sha(data[prefix + row['path']]) == row['sha256'], 'Retained raw output identity')
            require(counts(log) == [1], 'Actual numerical GPU control passed')
    prefix = 'gpu-source005-002/pixels/'
    names = ['initial', 'invalid_layout_retained', 'late_refusal_retained', 'replacement', 'continued']
    buffers = {name: data[prefix + name + '.rgba'] for name in names}
    require(all(len(raw) == 160*96*4 for raw in buffers.values()), 'Bounded complete RGBA buffers')
    for name, raw in buffers.items():
        rgba = list(zip(*[iter(raw)]*4))
        summary = read(prefix + name + '.json')
        require(summary['minimum_rgba'] == [min(p[c] for p in rgba) for c in range(4)]
                and summary['maximum_rgba'] == [max(p[c] for p in rgba) for c in range(4)], 'Raw extrema')
        require(summary['strict_red'] == sum(strict(p, 0) for p in rgba)
                and summary['strict_blue'] == sum(strict(p, 2) for p in rgba), 'Unchanged pixel classifier counts')
    initial, replacement = buffers['initial'], buffers['replacement']
    require(buffers['invalid_layout_retained'] == buffers['late_refusal_retained'] == initial,
            'Both complete refusal images retain original bytes')
    mask = lambda raw: [strict(p, 0) for p in zip(*[iter(raw)]*4)]
    before, after = mask(initial), mask(replacement)
    require(sum(before) == sum(after) == 276 and initial != replacement, 'Same-area actual deformation')
    require(all(after[y*160+x] == (x >= 12 and before[y*160+x-12])
                for y in range(96) for x in range(160)), 'Exact12px same-clock mask translation')
    proof = read(prefix + 'proof.json')
    require(all(proof[k] is True for k in ('world_identity_preserved', 'fresh_motion_owner',
            'invalid_layout_pixels_equal', 'late_refusal_pixels_equal')), 'Native state/refusal observations')
    require(proof['first_replacement_has_previous'] is False and proof['catalog_generation'] == 1,
            'Fresh deformation provenance/catalog')
    require((proof['tick'], proof['frames'], proof['presents'], proof['vm_counter_before'],
             proof['vm_counter_after'], proof['shift_pixels']) == (2, 7, 7, 1, 3, 12), 'World/VM/target continuation')
    require(proof['same_clock_seconds'] == 0.5
            and abs(proof['continued_clock_seconds'] - (0.5 + 1/60)) < 1e-12, 'Animation clock continuation')
    require(all(manifest['outcome_scope'][key] is False for key in ('actual_animated_editor_window',
            'GUI_click_coverage', 'appearance_acceptance', 'performance_or_phase_gate')), 'Unrun gates remain explicit')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    archive = (HERE / 'records.json.gz').read_bytes()
    require(len(archive) == manifest['archive_bytes'] and sha(archive) == manifest['archive_sha256'], 'Archive identity')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(manifest['archive_decoded_bytes'] + 1)
    require(len(raw) == manifest['archive_decoded_bytes'], 'Bounded decoded archive')
    payload = json.loads(raw)
    data = {}
    for row in payload['records']:
        name = PurePosixPath(row['name'])
        require(not name.is_absolute() and '..' not in name.parts and str(name) not in data, 'Unique relative record name')
        data[str(name)] = base64.b64decode(row['base64'], validate=True)
    require(set(data) == {r['name'] for r in manifest['records']}, 'Exact record set')
    for row in manifest['records']:
        raw = data[row['name']]
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Record identity:' + row['name'])
        if args.originals:
            require(Path(row['original']).read_bytes() == raw, 'Original byte identity:' + row['name'])
    if args.binaries:
        for row in manifest['binaries']:
            raw = Path(row['copy']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'External copied executable identity')
    for row in manifest['final_source_files']:
        path = PurePosixPath(row['path'])
        require(not path.is_absolute() and '..' not in path.parts, 'Relative source path')
        if args.source_root:
            require(sha((args.source_root / row['path']).read_bytes()) == row['sha256'], 'Live final source:' + row['path'])
        if args.git_source:
            require(re.fullmatch(r'[0-9a-f]{40}', args.git_source), 'Immutable full Git source SHA')
            raw = subprocess.check_output(['git', 'show', args.git_source + ':' + row['path']],
                                          cwd=args.source_root or HERE, timeout=10)
            require(sha(raw) == row['sha256'], 'Git final source:' + row['path'])
    replay(manifest, data)
    print(f'PASS {len(data)} records; CPU66 scoped controls; original CPU/GPU failures retained; '
          'Metal276->276 pixels shifted12; clock/world/VM continuation. Editor-window/GUI/performance unrun.')


if __name__ == '__main__':
    main()
