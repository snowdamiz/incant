#!/usr/bin/env python3
"""Bounded retained-data replay; never executes archived code or native artifacts."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
TEST = 'core_pose::actual_core_2156_route_preserves_gameplay_and_complete_pcm_with_hud_and_rigid_animation'


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path, cap):
    with Path(path).open('rb') as stream:
        data = stream.read(cap + 1)
    require(len(data) <= cap, 'Input exceeds bound: ' + str(path))
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='Also hash external complete PCM files')
    parser.add_argument('--git-source', type=Path, help='Check changed feature files at all producer commits')
    parser.add_argument('--source-root', type=Path, help='Check all inputs at final producer, including its docs')
    args = parser.parse_args()
    m = json.loads(read(HERE / 'manifest.json', 256 * 1024))
    require(m['format'] == 'native-core-pose-evidence-v1', 'Manifest version')
    a = m['archive']
    require(a['file'] == 'records.json.gz', 'Archive path')
    packed = read(HERE / a['file'], 8 * 1024**2)
    require(len(packed) == a['bytes'] and sha(packed) == a['sha256'], 'Packed identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:
        data = stream.read(32 * 1024**2 + 1)
    require(len(data) <= 32 * 1024**2 and len(data) == a['uncompressed_bytes']
            and sha(data) == a['uncompressed_sha256'], 'Expanded identity')
    encoded = json.loads(data)
    require(len(m['records']) == len({r['name'] for r in m['records']})
            and set(encoded) == {r['name'] for r in m['records']}, 'Exact record set')
    records = {}
    for row in m['records']:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record hash')
        if args.originals:
            require(read(row['original'], row['bytes']) == data, 'Original: ' + row['name'])
        records[row['name']] = data

    def value(name):
        return json.loads(records[name])

    copies = []
    counts = {}
    expected = [('cpu-002', 'cpu-packet-002', '001', False, [0, 0, 0, 0, 101]),
                ('cpu-003', 'cpu-packet-003', '002', True, [0, 0]),
                ('core-replay-001', 'core-replay-packet-001', '002', False, [0, 0, 0, 0]),
                ('core-replay-002', 'core-replay-packet-002', '003', True, [0, 0, 0])]
    for run, packet, generation, passed, exits in expected:
        p = value(packet + '/plan.json')
        r, c, o = [value(run + '/' + f + '.json') for f in ['result', 'closeout', 'owner']]
        require(p['source'] == r['source'] == c['source'] == m['sources'][generation], 'Producer identity')
        require(r['plan_sha256'] == sha(records[packet + '/plan.json'])
                and r['runner_sha256'] == sha(records[packet + '/run.py'])
                and c['result_sha256'] == sha(records[run + '/result.json'])
                and c['owner_sha256'] == sha(records[run + '/owner.json']), 'Plan/runner/result chain')
        require(r['passed'] is passed and c['passed'] is passed
                and c['actual_terminal_exit'] == (0 if passed else 1)
                and [s['exit'] for s in r['stages']] == exits, 'Preserved actual outcomes')
        require(c['observed_absence'] and all(x['pid_absent'] and x['group_absent']
                for x in c['observed_absence']), 'Fresh process closure')
        require(all(call['finished'] and call['cleanup_confirmed'] for call in o['calls']), 'Owned children reaped')
        all_names = []
        for call in o['calls']:
            for stream in ['stdout', 'stderr']:
                require(sha(records[run + '/' + call[stream]['path']]) == call[stream]['sha256'], 'Raw stream binding')
            text = records[run + '/' + call['stdout']['path']].decode()
            names = re.findall(r'^test (\S+) \.\.\. ok$', text, re.M)
            summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
            if summaries:
                require(sum(int(s[0]) for s in summaries) == len(names)
                        and all(s[1] == '0' for s in summaries), 'Raw membership/summary')
                all_names.extend(names)
        counts[run] = len(all_names)
        if run.startswith('core-replay'):
            require(all_names == [TEST], 'Exact copied Core fixture')
        else:
            required = [name for s in r['stages'] if 'tests' in s for name in s['tests']['passed']]
            require(all_names == required, 'Recorded membership')
        require(r['peaks']['target'] <= p['target_cap_bytes'] == 18 * 1024**3
                and r['seconds'] <= p['bounds']['aggregate_seconds']
                and r['global_postcleanup_seconds'] == p['prior_postcleanup_seconds'] + r['seconds'], 'Resources/costs retained')
        copies.extend({k:x[k] for k in ['path', 'bytes', 'sha256']} for x in r['copies'])
    require(counts == {'cpu-002': 461, 'cpu-003': 0, 'core-replay-001': 1, 'core-replay-002': 1}, 'Expected executions')
    require(copies == m['binaries'] and len(copies) == 8, 'Copied test identities')
    require('useless conversion' in records['cpu-002/006.stderr.log'].decode(), 'Original lint failure retained')
    first = value('core-replay-001/result.json')
    require(first['error'] == "ValueError('Render report mismatch')", 'Original aggregate failure retained')
    require(records['core-replay-packet-001/analyze.py'] == records['core-replay-packet-002/analyze.py'], 'Analyzer not relaxed')
    baseline = value(m['baseline'])
    for generation in ['001', '002']:
        route = value('core-replay-' + generation + '/route/result.json')
        require(route['passed'] and route['steps'] == 2156 and route['pcm_frames'] == 1_724_800
                and route['pose_displays'] == route['glyph_displays'] == 6468
                and route['sampled_rigid_poses'] == 49460, 'Complete route')
        require(route['ordinary'] == route['pose'] == baseline['observations']
                and len(route['ordinary']['checks']) == 8 and len(route['ordinary']['entities']) == 17
                and route['logs'] == baseline['logs'], 'Exact historical gameplay/logs')
        for key in ['full_world_snapshot_equal', 'animation_states_equal', 'audio_states_equal', 'ui_checkpoint_and_draw_input_equal']:
            require(route[key] is True, 'Actual in-test equality')
        require(route['complete_pcm_sha256'] == baseline['pcm_sha256'], 'Complete PCM report')
        if generation == '001':
            expected_render = dict(baseline['render_input'])
            expected_render.pop('ui')
            require(route['render_input'] == expected_render, 'Exact missing UI defect')
        else:
            require(route['render_input'] == baseline['render_input'], 'Full historical render and HUD report')
            require(value('core-replay-002/result.json')['analysis']['passed'], 'Final strict analysis')
        producer = value('core-replay-' + generation + '/producer-binding.json')
        require(producer['compiler_bearing_CPU_test_not_shipping'] is True
                and producer['producer']['startup_source_parsing'] is False
                and producer['producer']['script']['embedded_source'] is False
                and producer['producer']['physics_variant'] == 'fast', 'Cook/runtime boundaries')
    require(len(m['PCM']) == 4 and all(x['bytes'] == 13_798_400
            and x['sha256'] == baseline['pcm_sha256'] for x in m['PCM']), 'Complete PCM file identities')
    f = value('source-003/freeze.json')
    for generation, commit in m['sources'].items():
        freeze = value('source-' + generation + '/freeze.json')
        require(freeze['source'] == commit and freeze['source_manifest_sha256'] == sha(records['source-' + generation + '/source.json'])
                and freeze['patch_sha256'] == sha(records['source-' + generation + '/source.patch']), 'Source/patch chain')
        if args.git_source:
            require(re.fullmatch('[0-9a-f]{40}', commit), 'Commit syntax')
            for name, digest in freeze['changed'].items():
                path = Path(name)
                require(not path.is_absolute() and '..' not in path.parts, 'Git source path')
                data = subprocess.check_output(['git', 'show', commit + ':' + name], cwd=args.git_source, timeout=10)
                require(sha(data) == digest, 'Changed feature source: ' + name)
    if args.source_root:
        root = args.source_root.resolve()
        for name, digest in value('source-003/source.json')['files'].items():
            rel = Path(name)
            require(not rel.is_absolute() and '..' not in rel.parts, 'Source path')
            path = (root / rel).resolve()
            require(path.is_relative_to(root) and sha(read(path, 64 * 1024**2)) == digest, 'Producing source: ' + name)
    if args.binaries:
        for row in copies + m['PCM']:
            require(sha(read(row['path'], row['bytes'])) == row['sha256'], 'External binary/PCM identity')
    if 'fast-002/result.json' in records:
        prior = value('fast-001/result.json')
        require(prior['passed'] is False and 'ENOTCACHED' in prior['error'], 'Offline setup refusal retained')
        fast = value('fast-002/result.json')
        owner = value('fast-002/output/owner.json')
        close = value('fast-002/closeout.json')
        require(fast['passed'] and len(fast['stages']) == len(owner['calls']) == 11
                and close['actual_exit'] == 0 and close['all_absent']
                and all(x['pid_absent'] and x['group_absent'] for x in close['observed_absence']), 'Final development checks')
        require(sha(records['fast-002/result.json']) == close['result_sha256']
                and sha(records['fast-002/output/owner.json']) == close['owner_sha256'], 'Development receipt binding')
        for stage, call in zip(fast['stages'], owner['calls'], strict=True):
            require(stage['command'] == call['argv'] and stage['exit'] == call['exit'] == 0
                    and call['finished'] and call['cleanup_confirmed'], 'Development actual child')
            for stream in ['stdout', 'stderr']:
                require(sha(records['fast-002/output/' + call[stream]['path']]) == call[stream]['sha256'], 'Development raw stream')
    print(f'PASS: {len(records)} records; 461 focused controls; final on/off strict lint; both Core tests retained, original analyzer failed and corrected full report passed; 8 executable and 4 complete PCM identities. No GPU/window/cadence/shipping-performance claim.')


if __name__ == '__main__':
    main()
