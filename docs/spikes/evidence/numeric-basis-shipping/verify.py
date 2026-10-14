#!/usr/bin/env python3
"""Replay identities, functional scalars and raw intervals; never run native code."""
import argparse
import base64
import gzip
import hashlib
import json
import math
import subprocess
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def phase(row, ticks):
    assert row['samples'] == ticks
    assert all(row[k] == 0 for k in ('missing_frames', 'invalid_frames', 'overflow_samples'))
    counts = row['counts']
    assert len(counts) == 10001 and all(type(n) is int and 0 <= n < 2**64 for n in counts)
    assert sum(counts) == ticks
    rank = (95 * ticks + 99) // 100
    total = 0
    for index, count in enumerate(counts):
        total += count
        if total >= rank:
            lower, upper = (0, 0) if index == 0 else ((index - 1) * 10000 + 1, index * 10000)
            assert (row['p95_lower_ns'], row['p95_upper_ns']) == (lower, upper)
            return dict(lower=lower, upper=upper)
    raise AssertionError('No quantile')


def metrics(raw, ticks):
    assert raw['tick'] == ticks
    p = raw['profile']
    assert p['version'] == 2 and p['completed_frames'] == ticks and p['failed_frames'] == 0
    assert p['dropped_source_samples'] == 0 and p['tracy_process_transport'] is False
    f = p['frame_wall']
    assert (f['version'], f['unit'], f['bucket_width_ns'], f['max_ns']) == (1, 'ns', 10000, 100000000)
    assert f['attempted_frames'] == f['successful_frames'] == ticks and not f['saturated']
    assert all(f[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
    rows = {r['name']: r for r in f['phases']}
    assert len(rows) == len(f['phases'])
    result = {name + '.p95_wall_ns': phase(rows[name], ticks) for name in ('player.frame', 'script.tick')}
    a = p['rust_allocations']
    assert a['installed'] is True and a['saturated'] is False
    peak = a['process_peak_requested_bytes']
    assert type(peak) is int and 0 < peak < 2**64
    result['rust.process_peak_requested_bytes'] = dict(lower=peak, upper=peak)
    assert len(p['scripts']) == 2
    assert all(r['totals']['calls'] == ticks and r['totals']['unwound'] == 0
               and r['totals']['unavailable_cpu_samples'] == 0 for r in p['scripts'])
    return result


def vector(actual, expected):
    assert len(actual) == len(expected) == 3
    assert all(type(v) in (int, float) and math.isfinite(v) and abs(v-e) <= 1e-9
               for v, e in zip(actual, expected))


def position(n, tick):
    phase_value = (tick % 64 - 32) / 64
    return [n % 97 - 48 + phase_value, n // 97 % 97 - 48, n % 13 - 6 - phase_value]


def sentinel(tick):
    rows = [position(n, tick) for n in range(9999)]
    return [9999, sum(r[0] for r in rows) / 9999, sum(r[2] for r in rows) / 9999]


def engine(files):
    return {p: h for p, h in files.items() if p.startswith(('crates/', 'sdk/', 'tools/', '.cargo/', 'third_party/'))
            or p in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'package.json', 'package-lock.json')}


def delta(a, b):
    return {p: dict(baseline=a.get(p), candidate=b.get(p)) for p in sorted(set(a) | set(b)) if a.get(p) != b.get(p)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true', help='Rehash binary/PCM/assets and large external logs')
    parser.add_argument('--originals', action='store_true', help='Also compare portable records with original local files')
    parser.add_argument('--git-source', action='store_true', help='Rehash captured build-relevant Git objects in both revisions')
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    root = directory.parents[3]
    m = json.loads((directory / 'manifest.json').read_text())
    archive = (directory / 'records.json.gz').read_bytes()
    assert sha(archive) == m['archive_sha256']
    blobs = json.loads(gzip.decompress(archive))
    records, metadata = {}, {}
    for r in m['records']:
        assert r['name'] not in metadata
        metadata[r['name']] = r
        if r['external']:
            if not (args.external or args.originals):
                continue
            data = Path(r['original']).read_bytes()
        else:
            data = base64.b64decode(blobs[r['sha256']], validate=True)
        assert sha(data) == r['sha256'] and len(data) == r['bytes'], r['name']
        if args.originals and r['original']:
            assert Path(r['original']).read_bytes() == data, r['name']
        if r.get('git_commit') and args.git_source:
            assert subprocess.check_output(['git', 'show', r['git_commit'] + ':' + r['git_path']], cwd=root) == data
        records[r['name']] = data
    read = lambda name: json.loads(records[name])
    digest = lambda name: metadata[name]['sha256']
    plan = read('timing/plan.json')
    assert plan['order'] == ['baseline0', 'candidate0', 'candidate1', 'baseline1', 'baseline2', 'candidate2']
    assert plan['ticks'] == 2156 and plan['script_budget_ns'] == 2000000 and plan['regression_percent'] == 5
    assert plan['functional_result_sha256'] == digest('shipping/functional/result.json')
    assert plan['input_receipt_sha256'] == digest('input-receipt.json')
    assert read('input-receipt.json')['scripted_entities'] == 10000
    old_freeze, new_freeze = read('baseline/freeze.json'), read('shipping/freeze.json')
    assert delta(engine(old_freeze['repository_files']), engine(new_freeze['repository_files'])) == plan['expected_engine_delta']
    assert delta(old_freeze['authoring_files'], new_freeze['authoring_files']) == plan['expected_authoring_delta']
    assert plan['expected_authoring_delta'] == {}
    assert set(plan['expected_engine_delta']) == {
        'crates/incant_runtime/src/lib.rs', 'crates/incant_runtime/src/world.rs',
        'crates/incant_runtime/src/matrix_basis.rs', 'crates/incant_runtime/src/matrix_basis/tests.rs'}
    assert all(old_freeze[k] == new_freeze[k] for k in ('project', 'script', 'stress_script', 'camera', 'physics_mode'))
    for r in metadata.values():
        if r.get('git_commit'):
            f = old_freeze if r['git_commit'] == m['baseline_commit'] else new_freeze
            assert r['sha256'] == f['repository_files'][r['git_path']]
    for kind, label in [('baseline', 'baseline'), ('candidate', 'shipping')]:
        for name in ('freeze', 'export-result', 'post-build-source'):
            assert plan[kind + '_' + name.replace('-', '_') + '_sha256'] == digest(label + '/' + name + '.json')
        f = read(label + '/freeze.json')
        result = read(label + '/export-result.json')
        assert result['success'] and f['source'] == result['source'] == plan[kind + '_source']
        assert result['player_sha256'] == plan[kind + '_player_sha256'] == digest(label + '/package/player/incant_game_player')
        exported = read(label + '/package/export.json')
        assert exported['startup_source_parsing'] is False and exported['build_storage_removed_before_startup'] is True
        assert exported['physics_mode'] == 'fast'
        assert set(exported['codegen']) == {'incant_runtime', 'incant_script', 'incant_game_player_library', 'incant_game_player'}
        assert all(v['opt-level'] == '3' and v['codegen-units'] == '1' and v['panic'] == 'abort'
                   and v['split-debuginfo'] == 'packed' for v in exported['codegen'].values())
        assert exported['codegen']['incant_game_player']['lto'] == 'fat'
        assert exported['profiling']['mode'] == 'summary' and exported['profiling']['compiler']['profile'] == 'shipping'
        assert exported['player_sha256'] == result['player_sha256']
        for path, value in exported['data']['files'].items():
            assert digest(label + '/package/player/data/' + path) == value
        for path, value in f['authoring_files'].items():
            assert digest(label + '/authoring/' + path) == value
    assert len(new_freeze['authoring_files']) == len(old_freeze['authoring_files']) == 74
    commands = read('shipping/commands.json')
    assert all(r['exit'] == 0 for r in commands)
    for r in commands:
        for stream in ('stdout', 'stderr'):
            assert r[stream]['sha256'] == digest('shipping/' + r[stream]['file'])
        if Path(r['argv'][0]).name != 'incant_game_player':
            assert r['selected_env']['CARGO_BUILD_JOBS'] == r['selected_env']['RAYON_NUM_THREADS'] == '2'
            assert r['selected_env']['CARGO_NET_OFFLINE'] == 'true'
        else:
            assert all(r['selected_env'][k] is None for k in ('CARGO_BUILD_JOBS', 'RAYON_NUM_THREADS',
                                                           'CARGO_NET_OFFLINE', 'RUSTUP_TOOLCHAIN'))
    builds = [r for r in commands if 'build' in r['argv']]
    assert len(builds) == 1
    assert all(flag in builds[0]['argv'] for flag in ('--locked', 'shipping', 'aarch64-apple-darwin', '-j2'))
    control = read('baseline/functional-win/stdout.json')
    assert digest('baseline/functional-win/stdout.json') == plan['functional_report_sha256']
    assert read('shipping/functional/full-win/candidate/stdout.json') == control
    assert control['tick'] == 2156 and control['observations']['passed']
    assert control['pcm_sha256'] == plan['functional_pcm_sha256'] == digest('baseline/functional-win/output.pcm')
    fr = read('shipping/functional/result.json')
    assert fr['passed'] and fr['profiled'] is False and fr['processes'] == 41 and fr['reused_baseline_processes'] == 40 and fr['all_final_rows'] == 9999
    functional_commands = [n for n in metadata if n.startswith('shipping/functional/') and n.endswith('/command.json')]
    assert len(functional_commands) == 41
    for n in functional_commands:
        c = read(n); prefix = n.rsplit('/', 1)[0] + '/'
        assert c['exit'] == 0 and c['owned_process_reaped'] and c['profiled'] is False
        assert '--profile' not in c['argv'] and records[prefix + 'stderr.log'] == b''
        for name, value in c['outputs'].items():
            assert digest(prefix + name) == value
        assert c['identity']['player_sha256'] == (plan['baseline_player_sha256'] if '/baseline/' in n else plan['candidate_player_sha256'])
    rows_checked = []
    for batch in range(40):
        prefix = f'shipping/functional/rows-{batch:02}/'
        old_prefix = f'retained-baseline/rows-{batch:02}/'
        a, b = read(old_prefix + 'stdout.json'), read(prefix + 'candidate/stdout.json')
        old_command = read(old_prefix + 'command.json')
        assert old_command['exit'] == 0 and old_command['owned_process_reaped'] and old_command['profiled'] is False
        assert old_command['identity']['player_sha256'] == plan['baseline_player_sha256']
        for name, value in old_command['outputs'].items(): assert digest(old_prefix + name) == value
        assert read(old_prefix + 'observe.json') == read(prefix + 'candidate/observe.json')
        assert read(old_prefix + 'input.json') == read(prefix + 'candidate/input.json')
        assert a == b and b['tick'] == 65 and b['observations']['passed'] and b['logs']['dropped'] == 0
        assert b['pcm_sha256'] == digest(old_prefix + 'output.pcm') == digest(prefix + 'candidate/output.pcm')
        wanted = list(range(batch * 250, min(batch * 250 + 250, 9999)))
        entities = b['observations']['entities']
        assert set(entities) == {'01CSEN' + f'{100000+n:020}' for n in wanted}
        for n in wanted:
            vector(entities['01CSEN' + f'{100000+n:020}']['translation'], sentinel(65) if n == 0 else position(n, 65))
        checks = {r['name']: r for r in b['observations']['checks']}
        assert all(r['passed'] for r in checks.values())
        for tick in (1, 63, 64, 65):
            vector(checks['sentinel-' + str(tick)]['actual'], sentinel(tick))
        assert checks['population']['actual'] == 10045
        rows_checked.extend(wanted)
    assert rows_checked == list(range(9999))
    quiet = read('timing/quiet.json')
    pre = read('timing/pre-execution.json')
    assert quiet['root_explicit_grant'] and quiet['gpu_released'] and quiet['all_task_builders_terminal']
    assert quiet['plan_sha256'] == pre['plan_sha256'] == digest('timing/plan.json')
    assert quiet['runner_sha256'] == pre['runner_sha256'] == digest('timing/measure.py')
    assert pre['quiet_sha256'] == digest('timing/quiet.json') and pre['compiler_process_names'] == []
    raw_metrics, measured = {}, {}
    for label in plan['order']:
        prefix = 'timing/' + label + '/'
        report, measurement, command = read(prefix + 'stdout.json'), read(prefix + 'measurement.json'), read(prefix + 'command.json')
        assert command['exit'] == 0 and records[prefix + 'stderr.log'] == b''
        assert '--profile' in command['argv']
        assert command['stdout.json_sha256'] == measurement['profile_report_sha256'] == digest(prefix + 'stdout.json')
        assert command['stderr.log_sha256'] == digest(prefix + 'stderr.log')
        assert {k: v for k, v in report.items() if k != 'profile'} == control
        assert report['pcm_sha256'] == digest(prefix + 'output.pcm') == plan['functional_pcm_sha256']
        raw_metrics[label] = metrics(report, 2156)
        assert raw_metrics[label] == measurement['metrics']
        assert measurement['identities']['player_sha256'] == plan[('baseline' if label.startswith('baseline') else 'candidate') + '_player_sha256']
        source = 'baseline' if label.startswith('baseline') else 'shipping'
        exported, build = read(source + '/package/export.json'), read(source + '/package/project-build.json')
        assert measurement['identities'] == dict(export_sha256=digest(source + '/package/export.json'),
            build_log_sha256=digest(source + '/package/build.log'), player_sha256=exported['player_sha256'],
            data=exported['data']['files'], script_source_sha256=build['script']['source_sha256'], program=build['program'])
        assert measurement['match']['codegen'] == exported['codegen']
        assert measurement['match']['compiler'] == exported['compiler']
        assert measurement['match']['profiling'] == exported['profiling']['compiler']['features']
        assert measurement['match']['project_sha256'] == exported['project_sha256']
        measured[label] = measurement
    result = read('timing/results.json')
    assert result['plan'] == plan
    assert result['records'] == {label: digest('timing/' + label + '/measurement.json') for label in plan['order']}
    for i, pair in enumerate(result['pairs']):
        old, new = measured['baseline' + str(i)], measured['candidate' + str(i)]
        assert old['match'] == new['match']
        for name, value in new['metrics'].items():
            prior = old['metrics'][name]
            verdict = 'pass' if value['upper'] * 100 <= prior['lower'] * 105 else (
                'regression' if value['lower'] * 100 > prior['upper'] * 105 else 'inconclusive')
            assert pair['metrics'][name] == verdict
            assert pair['intervals'][name] == dict(baseline=prior, candidate=value)
        s = new['metrics']['script.tick.p95_wall_ns']
        assert pair['script_2ms_p95'] == ('pass' if s['upper'] <= 2000000 else 'fail' if s['lower'] > 2000000 else 'inconclusive')
        assert pair['script_2ms_p95'] == 'fail' and pair['passed'] is False
    assert result['pairs'][0]['metrics']['player.frame.p95_wall_ns'] == 'regression'
    assert result['phase_gate_passed'] is False
    terminal = read('timing/reaped.json')
    assert terminal['supervisor_exit'] == 0 and terminal['terminal_reaped'] and terminal['owned_players_absent'] and terminal['current_player_processes'] == []
    assert terminal['results_sha256'] == digest('timing/results.json')
    git_rows = 0
    if args.git_source:
        for label in ('baseline', 'shipping'):
            f = read(label + '/freeze.json')
            paths = [p for p in f['repository_files'] if p.startswith(('crates/', 'sdk/', 'tools/', '.cargo/', 'third_party/', 'samples/core/'))
                     or p in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'package.json', 'package-lock.json')]
            child = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
            try:
                for path in paths:
                    child.stdin.write((f['source'] + ':' + path + '\n').encode()); child.stdin.flush()
                    header = child.stdout.readline().split(); assert header[1] == b'blob', path
                    data = child.stdout.read(int(header[2])); assert child.stdout.read(1) == b'\n'
                    assert sha(data) == f['repository_files'][path], path
                    git_rows += 1
            finally:
                child.stdin.close(); child.stdout.close(); assert child.wait() == 0
    print(json.dumps(dict(passed=True, records=len(metadata), portable_payloads=len(blobs),
                          functional_processes=41, retained_baseline_row_processes=40, final_rows=9999, timed_processes=6, git_inputs=git_rows,
                          script_2ms_failures=3, frame_5percent_regressions=1, production_acceptance=False)))


if __name__ == '__main__':
    main()
