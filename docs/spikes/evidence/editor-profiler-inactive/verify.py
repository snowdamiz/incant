"""Bounded data-only replay of the inactive-profiler CPU proxy; executes no engine."""
import argparse
import base64
import copy
import gzip
import hashlib
import io
import json
import math
from pathlib import Path, PurePosixPath
import re
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def parse(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate JSON key: ' + key)
            result[key] = value
        return result
    def invalid(value):
        raise ValueError('nonfinite JSON: ' + value)
    return json.loads(data, object_pairs_hook=unique, parse_constant=invalid)


def inflate(data, bound):
    with gzip.GzipFile(fileobj=io.BytesIO(data)) as stream:
        result = stream.read(bound + 1)
    assert len(result) <= bound, 'decompression bound'
    return result


def safe(path):
    parts = PurePosixPath(path)
    assert parts.parts and not parts.is_absolute() and '..' not in parts.parts


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    for flag in ('originals', 'binaries', 'external-inputs', 'git-source'):
        ap.add_argument('--' + flag, action='store_true')
    args = ap.parse_args()
    here = Path(__file__).resolve().parent
    manifest = parse((here / 'manifest.json').read_bytes())
    assert manifest['format'] == 'incant-inactive-profiler-cpu-proxy' and manifest['version'] == 1
    meta = manifest['archive']
    archive = (here / 'records.json.gz').read_bytes()
    assert len(archive) == meta['bytes'] <= 4 * 1024**2 and sha(archive) == meta['sha256']
    raw = inflate(archive, 16 * 1024**2)
    assert len(raw) == meta['inflated_bytes']
    rows = parse(raw)
    assert len(rows) == meta['records'] <= 400
    records = {}
    for row in rows:
        safe(row['path'])
        assert row['path'] not in records
        assert 0 <= row['bytes'] <= 32 * 1024**2
        records[row['path']] = row
    assert sum(row['bytes'] for row in rows) <= 192 * 1024**2

    def data(name):
        row = records[name]
        encoded = base64.b64decode(row['data'], validate=True)
        assert row['encoding'] in ('base64', 'gzip-base64')
        value = inflate(encoded, 32 * 1024**2) if row['encoding'] == 'gzip-base64' else encoded
        assert len(value) == row['bytes'] and sha(value) == row['sha256'], name
        return value

    def obj(name):
        return parse(data(name))

    # Decode and hash one record at a time; never write or execute archived scripts.
    for name, row in records.items():
        data(name)
        if args.originals:
            assert file_sha(row['original']) == row['sha256'], name
    for name, digest in manifest['bindings'].items():
        assert records[name]['sha256'] == digest
    for generation in ('001', '002'):
        freeze = obj(f'source-{generation}/freeze.json')
        assert len(freeze['rows']) == 88
        assert re.fullmatch('[0-9a-f]{40}', freeze['source'])
        for row in freeze['rows']:
            safe(row['path'])
            original = data(f"source-{generation}/inputs/{row['path']}")
            assert len(original) == row['size'] and sha(original) == row['sha256']
            if args.git_source:
                actual = subprocess.check_output(
                    ['git', 'show', freeze['source'] + ':' + row['path']],
                    cwd=here.parents[3], timeout=15)
                assert sha(actual) == row['sha256']
    assert obj('source-002/freeze.json')['source'] == manifest['source']
    cpu, cp = obj('cpu-003/result.json'), obj('cpu-plan-003.json')
    assert cpu['source'] == manifest['source'] and cpu['outcome'] == 'success' and cpu['error'] is None
    assert cpu['plan_sha256'] == sha(data('cpu-plan-003.json'))
    assert cpu['runner_sha256'] == sha(data('qualify-003.py'))
    assert cp['freeze_sha256'] == sha(data('source-002/freeze.json'))
    assert cpu['native_process_sha256'] == sha(data('native_process.py'))
    cpu_grant = obj('root-cpu-grant-003.json')
    assert cpu_grant['accepted_for_cpu'] and cpu_grant['source'] == cpu['source']
    assert cpu_grant['plan_sha256'] == cpu['plan_sha256'] and cpu_grant['runner_sha256'] == cpu['runner_sha256']
    assert cpu['minimum_sampled_free_bytes'] >= cp['minimum_free_bytes']
    assert cpu['target_last_sampled_bytes'] - cpu['target_initial_bytes'] <= cp['maximum_target_growth_bytes']
    assert cpu['elapsed_seconds'] <= cp['overall_seconds']
    assert not any(cpu[k] for k in ('managed_call_active', 'gpu_execution', 'window_execution', 'listener_execution', 'native_player_execution'))
    assert len(cpu['commands']) == 8 and cpu['commands'][0]['name'] == 'rust-version'
    for command in cpu['commands']:
        assert command['exit_code'] == 0 and command['managed_returned_after_cleanup']
        for stream in ('stdout', 'stderr'):
            assert sha(data(f"cpu-003/{command['name']}.{stream}")) == command[stream + '_sha256']
    for step, command in zip(cp['steps'], cpu['commands'][1:]):
        assert step['name'] == command['name'] and step['argv'] == command['argv']
        if step.get('tests'):
            assert re.search(rb'test result: ok\. 2 passed; 0 failed; 1 ignored;', data(f"cpu-003/{step['name']}.stdout"))

    # Compare actual Cargo unit/profile sets; never union multiple unit feature rows.
    units = []
    for mode in ('disabled', 'enabled'):
        found = {}
        for line in data(f'cpu-003/fixture-{mode}-controls.stdout').splitlines():
            try:
                event = parse(line)
            except (ValueError, UnicodeDecodeError):
                continue
            if event.get('reason') == 'compiler-artifact':
                key = (event['package_id'], tuple(event['target']['kind']), event['target']['name'])
                found.setdefault(key, set()).add((tuple(event['features']), json.dumps(event['profile'], sort_keys=True)))
        units.append(found)
    assert units[0].keys() == units[1].keys() and len(units[0]) == 546
    assert all(sum(map(len, u.values())) == 568 for u in units)
    expected = {'incant_agent_native': {'agent-profiler'}, 'incant_game_runtime': {'profile'},
                'incant_profile': {'session', 'summary'}, 'incant_runtime': {'profile'}, 'incant_script': {'profile'}}
    changed = []
    for key, off in units[0].items():
        on = units[1][key]
        if off == on:
            continue
        assert len(off) == len(on) == 1
        (a, profile), = off
        (b, other), = on
        package = key[0].split('/')[-1].split('#')[0]
        assert profile == other and set(b) - set(a) == expected[package] and not set(a) - set(b)
        changed.append(package)
    assert len(changed) == 7 and set(changed) == expected.keys()

    # Preserve each failed attempt rather than reclassifying its later successor.
    assert obj('cpu-002/result.json')['outcome'] != 'success'
    failed = obj('functional-001/result.json')
    assert failed['state'] == 'failure' and 'derivative changed other authoring' in failed['error']
    assert len(failed['runs']) == 1 and failed['runs'][0]['mode'] == 'off' and failed['runs'][0]['exit_code'] == 0
    functional = obj('functional-002/result.json')
    assert functional['state'] == 'success' and len(functional['runs']) == 1 and functional['runs'][0]['mode'] == 'on'
    assert functional['reused_off']['sha256'] == sha(data('functional-001/off/report.json'))

    plan, result = obj('timing-packet-001/plan.json'), obj('timing-001/result.json')
    assert plan['source'] == result['source'] == manifest['source']
    assert result['plan_sha256'] == sha(data('timing-packet-001/plan.json'))
    assert result['runner_sha256'] == sha(data('timing-packet-001/run.py'))
    assert result['grant_sha256'] == sha(data('timing-packet-001/root-quiet-grant.json'))
    grant = obj('timing-packet-001/root-quiet-grant.json')
    assert grant['accepted_for_timing'] and grant['quiet_lanes_acknowledged']
    assert grant['plan_sha256'] == result['plan_sha256'] and grant['runner_sha256'] == result['runner_sha256']
    assert result['state'] == 'success' and result['error'] is None and not result['managed_call_exception']
    assert result['elapsed_seconds'] <= plan['stage_seconds'] and result['minimum_sampled_free_bytes'] >= plan['minimum_free_bytes']
    assert plan['sequence'] == [r['label'] for r in result['runs']] == ['B0', 'C0', 'C1', 'B1', 'B2', 'C2']
    assert plan['pairs'] == [['B0', 'C0'], ['B1', 'C1'], ['B2', 'C2']]
    assert len(result['metadata']) == 6
    for row in result['metadata']:
        assert row['exit_code'] == 0 and row['managed_returned_after_cleanup']
        for stream in ('stdout', 'stderr'):
            assert sha(data(f"timing-001/{row['phase']}-{row['name']}.{stream}")) == row[stream + '_sha256']
    observations = obj('reference/observations')
    receipt = obj('reference/core_receipt')['receipt']
    for name, bound in plan['inputs'].items():
        assert sha(data('reference/' + name)) == bound['sha256']

    def report(name, mode):
        r = obj(name)
        assert r['format'] == 'incant-inactive-core-proxy' and r['version'] == 1
        assert r['agent_profiler_compiled'] is (mode == 'on')
        assert not any(r[k] for k in ('capture_activated', 'gpu_submitted', 'window_launched', 'eligible_for_shipping_gate'))
        assert r['terminal_error'] is None and r['physics_mode'] == 'deterministic'
        assert r['ticks'] == 2156 and r['tick_rate'] == 60 and r['audio_frames'] == 1724800
        assert r['entity_count'] == 10041 and r['scene'] == plan['scene']
        for key, bound in [('project_sha256', 'project'), ('recording_sha256', 'recording'), ('observations_sha256', 'observations')]:
            assert r[key] == plan['inputs'][bound]['sha256']
        checks = r['observations']['checks']
        assert r['observations']['passed'] and len(checks) == len(observations['checks']) == 8
        assert set(r['observations']['entities']) == set(observations['entities']) and len(observations['entities']) == 17
        by_name = {c['name']: c for c in observations['checks']}
        assert {c['name'] for c in checks} == set(by_name)
        for c in checks:
            assert {k: c[k] for k in ('name', 'tick', 'expect')} == by_name[c['name']] and c['passed']
            expected, actual = c['expect'], c['actual']
            if expected['type'] == 'vector':
                assert len(actual) == 3 and all(math.isfinite(v) for v in actual)
                assert all(abs(x-y) <= expected['tolerance'] for x, y in zip(actual, expected['value']))
            else:
                assert expected['type'] in ('exists', 'entity_count') and actual == expected['value']
        logs = [v for v in r['logs']['entries'] if not v['message'].startswith('proof.hud ')]
        assert r['logs']['dropped'] == 0
        pickups = [v for v in logs if v['message'].startswith('core.pickup ')]
        assert [v['message'].split()[1] for v in pickups] == receipt['samples']
        assert [v['tick'] for v in pickups] == [127, 508, 1266, 1842]
        assert [v['tick'] for v in logs if v['message'].startswith('core.won ')] == [2066]
        assert not any(v['message'].startswith('core.lost ') for v in logs)
        damage = [v for v in logs if v['message'].startswith('core.damage ')]
        assert damage and int(damage[-1]['message'].rsplit('=', 1)[1]) == 2
        hud = receipt['hud']
        for role in ('won', 'lost', 'prompt'):
            if role in hud:
                parent = hud['result' if role in ('won', 'prompt') else 'inactive']
                assert r['observations']['entities'][hud[role]]['parent'] == parent
        d = r['derivative']
        for side in ('before', 'after'):
            assert sha(d[side + '_canonical_text'].encode()) == d[side + '_sha256']
        before, after = parse(d['before_canonical_text']), parse(d['after_canonical_text'])
        expected = copy.deepcopy(before)
        assert expected['settings']['physics_mode'] == 'fast'
        expected['settings']['physics_mode'] = 'deterministic'
        after['settings'].setdefault('physics_mode', 'deterministic')
        assert expected == after
        assert d['commands'] == [{'op': 'set_physics_mode', 'mode': 'deterministic'}]
        assert d['actor'] == {'origin': 'user', 'actor': 'inactive-profiler-fixture', 'model': None, 'conversation_id': None}
        assert re.fullmatch(r'[0-7][0-9A-HJKMNP-TV-Z]{25}', d['id'])
        identity = {k: r[k] for k in ('ticks', 'tick_rate', 'audio_frames', 'pcm_sha256', 'script_source_sha256',
                    'scene', 'entity_count', 'logs', 'observations', 'project_sha256', 'recording_sha256', 'observations_sha256', 'physics_mode')}
        identity['derivative'] = {k: d[k] for k in ('before_sha256', 'after_sha256', 'commands', 'actor')}
        assert sha(json.dumps(identity, sort_keys=True, separators=(',', ':')).encode()) == plan['functional_identity_sha256']
        t = r['timing']
        raw = t['raw_wall_ns']
        assert len(raw) == t['expected_frames'] == t['recorded_frames'] == 2156 and t['complete']
        assert t['clock_invalid_frames'] == 0 and all(type(v) is int and 0 <= v < 2**64 for v in raw)
        p95, total = sorted(raw)[(2156 * 95 + 99) // 100 - 1], sum(raw)
        assert p95 == t['p95_wall_ns'] and total == t['total_wall_ns'] < 2**64
        return {'raw_nearest_rank_p95_ns': p95, 'total_ns': total}

    report('functional-001/off/report.json', 'off')
    report('functional-002/on/report.json', 'on')
    metrics = {}
    for row in result['runs']:
        assert row['exit_code'] == 0 and row['managed_returned_after_cleanup']
        for a in row['artifacts']:
            key = 'timing-001/' + row['label'] + '/' + Path(a['path']).name
            assert records[key]['sha256'] == a['sha256'] and records[key]['bytes'] == a['bytes']
        assert row['mode'] == ('off' if row['label'].startswith('B') else 'on')
        metrics[row['label']] = report('timing-001/' + row['label'] + '/report.json', row['mode'])
    comparison = obj('timing-001/comparison.json')
    assert [[p['baseline_run'], p['candidate_run']] for p in comparison['pairs']] == plan['pairs']
    for pair in comparison['pairs']:
        assert set(pair['metrics']) == {'raw_nearest_rank_p95_ns', 'total_ns'}
        for key, m in pair['metrics'].items():
            a, b = metrics[pair['baseline_run']][key], metrics[pair['candidate_run']][key]
            assert m['baseline'] == a and m['candidate'] == b and m['difference_ns'] == b-a
            assert m['candidate_over_baseline'] == b/a and m['within_fixed_5_percent'] == (b*100 <= a*105)
    assert comparison['all_pairs_within_fixed_local_5_percent']
    assert comparison['excluded_frames'] == comparison['retries'] == 0 and not comparison['quantiles_averaged']
    assert result['performance_or_shipping_gate'] is None and not result['gpu_window_capture']
    if args.binaries:
        for row in manifest['copied_binaries'].values():
            assert Path(row['path']).stat().st_size == row['bytes'] and file_sha(row['path']) == row['sha256']
    if args.external_inputs:
        for row in manifest['external_inputs']:
            assert Path(row['path']).stat().st_size == row['bytes'] and file_sha(row['path']) == row['sha256']
    print(json.dumps({'records': len(records), 'source_generations': 2, 'source_rows_each': 88,
                      'cpu_controls': 4, 'functional_routes': 2, 'timing_routes': 6, 'frames_each': 2156,
                      'all_fixed_pairs_within_local_5_percent': True, 'recorded_logical_pcm_parity': True,
                      'originals_verified': args.originals, 'external_binaries_verified': args.binaries,
                      'external_inputs_verified': args.external_inputs, 'historical_git_source_verified': args.git_source,
                      'shipping_gui_gate_or_zero_overhead_claim': False}))


if __name__ == '__main__':
    main()
