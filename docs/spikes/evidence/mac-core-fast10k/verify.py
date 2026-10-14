#!/usr/bin/env python3
"""Independently audit retained shipping evidence. Never executes a player/build."""
import argparse
import base64
from datetime import datetime
import gzip
import hashlib
import json
import math
from pathlib import Path
import subprocess

from build import audit as build_audit
from metrics import compare, controls, metrics

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def file_digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while block := f.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def git_source(source, frozen):
    process = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for path, sha in frozen.items():
            process.stdin.write((source + ':' + path + '\n').encode())
            process.stdin.flush()
            header = process.stdout.readline().split()
            assert len(header) == 3 and header[1] == b'blob', path
            raw = process.stdout.read(int(header[2]))
            assert process.stdout.read(1) == b'\n' and digest(raw) == sha, path
    finally:
        process.stdin.close()
        process.stdout.close()
        assert process.wait(timeout=10) == 0
    return len(frozen)


def functional(report, observed, reference):
    assert report['tick'] == 2156 and report['frames'] == 1724800
    assert report['observations']['tick'] == 2156 and report['observations']['tick_rate'] == 60
    assert report['observations']['passed'] is True and report['logs']['dropped'] == 0
    assert report['render_input']['gpu_submitted'] is False and report['render_input']['mode'] == 'cpu-only'
    checks = report['observations']['checks']
    assert len(checks) == len(observed['checks']) == 8
    for actual, expected in zip(sorted(checks, key=lambda r: r['name']), sorted(observed['checks'], key=lambda r: r['name'])):
        assert {k: actual[k] for k in ('name', 'tick', 'expect')} == expected
        assert actual['passed'] is True
        value, want = actual['actual'], expected['expect']
        if want['type'] == 'vector':
            assert len(value) == len(want['value']) == 3 and want['tolerance'] == 1e-9
            assert all(math.isfinite(a) and abs(a - b) <= want['tolerance'] for a, b in zip(value, want['value']))
        elif want['type'] == 'entity_count':
            assert value == want['value'] == 10041
        else:
            assert want['type'] == 'exists' and value is want['value'] is True
    assert {c['name'] for c in checks} == {'core-stress-' + str(i) for i in (0, 1, 4999, 96, 97, 9998)} | {'core-stress-population', 'runs-to-end'}
    logs = report['logs']['entries']
    pickups = [r for r in logs if r['message'].startswith('core.pickup ')]
    assert [r['tick'] for r in pickups] == [127, 508, 1266, 1842]
    assert [r['message'].split()[1] for r in pickups] == reference['samples']
    assert [(r['tick'], r['message']) for r in logs if r['message'].startswith('core.damage ')] == [(664, 'core.damage health=2')]
    assert [(r['tick'], r['message']) for r in logs if r['message'].startswith('core.won ')] == [(2066, 'core.won tick=2066 health=2 pickups=4')]
    assert not any(r['message'].startswith('core.lost ') for r in logs)
    hud = reference['hud']
    for role in ('won', 'lost', 'prompt'):
        assert report['observations']['entities'][hud[role]]['parent'] == hud['inactive' if role == 'lost' else 'result']
    assert all(report['observations']['entities'][owner] is None for owner in reference['samples'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify-only', action='store_true', help='Compatibility flag; all operations are read-only.')
    parser.add_argument('--external', action='store_true', help='Rehash every hash-bound original, including player/PCM/assets.')
    parser.add_argument('--git-source', action='store_true', help='Rehash both successful complete source inventories from immutable Git.')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    for name, sha in manifest['audit_files'].items():
        assert file_digest(HERE / name) == sha, name
    raw = (HERE / 'artifacts.json.gz').read_bytes()
    assert digest(raw) == manifest['archive_sha256']
    payloads = {key: base64.b64decode(value, validate=True) for key, value in json.loads(gzip.decompress(raw))['payloads'].items()}
    assert all(digest(value) == key for key, value in payloads.items())
    rows = {r['path']: r for r in manifest['records']}
    assert len(rows) == len(manifest['records'])
    assert set(payloads) == {r['sha256'] for r in rows.values() if r['embedded']}
    for r in rows.values():
        if r['embedded']:
            assert len(payloads[r['sha256']]) == r['bytes']
        if args.external and 'original' in r:
            p = Path(r['original'])
            assert p.is_file() and p.stat().st_size == r['bytes'] and file_digest(p) == r['sha256'], p
    def read(path):
        return payloads[rows[path]['sha256']]
    def data(path):
        return json.loads(read(path))
    plan, pre = data('pair/plan.json'), data('pair/pre-execution.json')
    assert plan['order'] == ['baseline0', 'candidate0', 'baseline1', 'candidate1', 'baseline2', 'candidate2']
    assert plan['ticks'] == 2156
    assert pre['known_build_sessions_terminal'] is True and pre['compiler_process_name_scan'] == []
    # This is a recorded task-process check, not proof that user applications were idle.
    pre_ns = int(datetime.fromisoformat(pre['at']).timestamp() * 1_000_000_000)
    freezes, exports, functionals, build_results, source_counts = {}, {}, {}, {}, {}
    for label in ('original', 'baseline', 'candidate'):
        prefix = label + '/'
        f = data(prefix + 'freeze.json')
        assert f['source'] == manifest['sources'][label] and f['physics_mode'] == 'fast' and f['cpu_jobs'] == 2
        assert f['counts'] == {'scripted_entities': 10000, 'source_groups': 2}
        assert len(f['authoring_files']) == 62
        assert f['input_receipt_sha256'] == rows['reference/stress-prepared.json']['sha256']
        for path, sha in f['authoring_files'].items():
            assert rows[prefix + 'authoring/' + path]['sha256'] == sha, (label, path)
        assert f['project'] == rows[prefix + 'authoring/core.json']['sha256']
        assert f['script'] == rows[prefix + 'authoring/gameplay.ts']['sha256']
        assert f['stress_script'] == rows[prefix + 'authoring/stress.ts']['sha256']
        freezes[label] = f
        commands = data(prefix + 'commands.json')
        assert [c['exit'] for c in commands] == ([0] * 4 + [None] if label == 'original' else [0] * 7)
        for command in commands:
            assert command['started_ns'] <= command['ended_ns']
            for stream in ('stdout', 'stderr'):
                retained = rows[prefix + command[stream]['file']]
                assert (retained['sha256'], retained['bytes']) == (command[stream]['sha256'], command[stream]['bytes'])
        result = data(prefix + 'export-result.json')
        assert result['ended_ns'] >= result['started_ns']
        if label == 'original':
            assert result['success'] is False and 'timed out after 2400 seconds' in result['error']
            assert commands[-1]['ended_ns'] - commands[-1]['started_ns'] >= 2400 * 1_000_000_000
            size = data(prefix + 'generated-binding-size.json')
            assert size['source'] == f['source'] and size['script_instances'] == 10000
            assert size['sha256'] == rows[prefix + 'generated-behavior.rs']['sha256']
            assert size['bytes'] == rows[prefix + 'generated-behavior.rs']['bytes'] == 3520096
            continue
        assert result['success'] is True and result['source'] == plan[label + '_source'] == f['source']
        post = data(prefix + 'post-build-source.json')
        assert post['passed'] is True and post['source'] == f['source']
        assert post['freeze_sha256'] == rows[prefix + 'freeze.json']['sha256']
        assert post['export_result_sha256'] == rows[prefix + 'export-result.json']['sha256']
        assert post['repository_paths_verified'] == len(f['repository_files'])
        assert post['authoring_paths_verified'] == 62
        recorded = next(r for r in pre['packages'] if r['path'] == plan[label])
        assert recorded['functional_source'] == f['source'] and recorded['terminal_exit'] == recorded['functional_exit'] == 0
        assert recorded['export_sha256'] == rows[prefix + 'export-result.json']['sha256']
        setup = data(prefix + 'setup.json')
        assert setup['exit'] == 0 and setup['package_lock_sha256'] == f['repository_files']['package-lock.json']
        build_results[label] = build_audit(read, data, rows, prefix)
        export = data(prefix + 'package/export.json')
        assert export['player_sha256'] == result['player_sha256']
        exports[label] = export
        control = data(prefix + 'functional-win/stdout.json')
        command = data(prefix + 'functional-win/command.json')
        assert command['exit'] == 0 and command['source'] == f['source']
        assert command['core_win_verified'] is command['stress_checks_passed'] is True
        assert command['scripted_entities'] == 10000 and command['source_groups'] == 2
        assert result['ended_ns'] <= command['started_ns'] < command['ended_ns'] < pre_ns
        for name, sha in command['outputs'].items():
            assert rows[prefix + 'functional-win/' + name]['sha256'] == sha
        assert command['identity']['player_sha256'] == export['player_sha256']
        assert command['identity']['data'] == export['data']['files']
        assert command['identity']['export_sha256'] == rows[prefix + 'package/export.json']['sha256']
        assert command['identity']['build_log_sha256'] == rows[prefix + 'package/build.log']['sha256']
        assert '--profile' not in command['argv'] and not read(prefix + 'functional-win/stderr.log')
        functional(control, data(prefix + 'functional-win/observe.json'), data('reference/core-prepared.json')['receipt'])
        assert control['pcm_sha256'] == rows[prefix + 'functional-win/output.pcm']['sha256']
        assert rows[prefix + 'functional-win/output.pcm']['bytes'] == control['frames'] * 8
        functionals[label] = control
        source_counts[label] = git_source(f['source'], f['repository_files']) if args.git_source else 0
    assert freezes['original']['authoring_files'] == freezes['baseline']['authoring_files'] == freezes['candidate']['authoring_files']
    inputs = data('reference/shipping-inputs.json')
    prepared = data('reference/stress-prepared.json')
    assert inputs['project_sha256'] == prepared['project_sha256'] == freezes['baseline']['project']
    assert prepared['authoring_files'] == freezes['baseline']['authoring_files']
    assert inputs['ticks'] == 2156 and inputs['scripted_entities'] == prepared['scripted_entities'] == 10000
    assert inputs['ui_viewport'] == '640x480' and inputs['sample_rate'] == 48000
    for key in ('input', 'observe'):
        assert inputs[key + '_sha256'] == rows['baseline/functional-win/' + key + '.json']['sha256']
    assert functionals['baseline'] == functionals['candidate']
    assert exports['baseline']['data'] == exports['candidate']['data']
    for path, row in rows.items():
        if path.startswith('source/'):
            name = path.removeprefix('source/')
            assert row['sha256'] == freezes['candidate']['repository_files'][name] == freezes['baseline']['repository_files'][name]
    project = data('baseline/authoring/core.json')
    assert len(project['scenes']) == 1 and project['settings']['physics_mode'] == 'fast'
    entities = next(iter(project['scenes'].values()))['entities']
    assert len(entities) == 10045 and sum('Script' in e['components'] for e in entities.values()) == 10000
    measures, summaries = {}, []
    previous_end = pre_ns
    for name in plan['order']:
        label, i = name[:-1], int(name[-1])
        prefix = label + '/paired-' + str(i) + '/'
        measurement, report, command = data(prefix + 'measurement.json'), data(prefix + 'stdout.json'), data(prefix + 'command.json')
        assert command['exit'] == 0 and previous_end <= command['started_ns'] <= command['ended_ns']
        previous_end = command['ended_ns']
        argv = command['argv']
        assert '--profile' in argv and argv[argv.index('--ticks') + 1] == '2156'
        assert argv[argv.index('--ui-viewport') + 1] == '640x480'
        assert set(command['environment']) == {'PATH', 'TMPDIR'}
        assert all(len(command[key]) == 3 for key in ('load_average_before', 'load_average_after'))
        for file in ('stdout.json', 'stderr.log'):
            assert rows[prefix + file]['sha256'] == command[file + '_sha256']
        assert not read(prefix + 'stderr.log')
        assert measurement['profile_report_sha256'] == rows[prefix + 'stdout.json']['sha256']
        assert report['pcm_sha256'] == rows[prefix + 'output.pcm']['sha256'] == functionals[label]['pcm_sha256']
        assert rows[prefix + 'output.pcm']['bytes'] == report['frames'] * 8
        assert {k: v for k, v in report.items() if k != 'profile'} == functionals[label]
        scripts = report['profile']['scripts']
        assert len(scripts) == 2 and [r['slot'] for r in scripts] == [0, 1]
        assert [r['source'] for r in scripts] == [[1, 102, 114, 192] + [0] * 11 + [i] for i in (1, 2)]
        for row in scripts:
            assert row['totals']['calls'] == 2156 and row['totals']['unwound'] == row['totals']['unavailable_cpu_samples'] == 0
        computed = metrics(report)
        assert computed == measurement['metrics'] and measurement['phase_gate_passed'] is False
        assert measurement['identities'] == data(label + '/functional-win/command.json')['identity']
        match, exported = measurement['match'], exports[label]
        assert match['codegen'] == exported['codegen'] and match['profiling'] == exported['profiling']['compiler']['features']
        assert match['compiler'] == exported['compiler'] and match['physics'] == 'fast'
        assert match['ticks'] == 2156 and match['tick_rate'] == 60
        assert match['project_sha256'] == freezes[label]['project']
        assert match['script_source_sha256'] == data(label + '/package/project-build.json')['compiler']['bundleSha256']
        assert match['data'] == exported['data']['files']
        assert match['inputs'] == {k: rows[prefix + k + '.json']['sha256'] for k in ('input', 'observe')}
        assert match['inputs'] == {k: rows[label + '/functional-win/' + k + '.json']['sha256'] for k in ('input', 'observe')}
        if measures:
            assert match == measures['baseline0']['match']
        prior = measures['baseline' + str(i)] if label == 'candidate' else measures.get('baseline0')
        comparison = None if prior is None else compare(measurement, prior)
        assert comparison == measurement['comparison']
        if comparison:
            assert comparison['script_2ms_p95'] == 'fail' and comparison['passed'] is False
            assert all(v == 'pass' for v in comparison['metrics'].values())
        summaries.append({'label': label, 'run': i, 'metrics': computed, 'comparison': comparison,
                          'functional_control_equal': {k: True for k in ('tick', 'frames', 'logs', 'observations', 'pcm_sha256')},
                          'measurement_sha256': rows[prefix + 'measurement.json']['sha256']})
        measures[name] = measurement
    assert data('pair/results.json') == {'plan': plan, 'runs': summaries}
    mutations = controls(data('baseline/paired-0/stdout.json'), measures['baseline0'], measures['candidate0'])
    print(json.dumps({'passed': True, 'records': len(rows), 'payloads': len(payloads), 'external_checked': args.external,
                      'git_source_rows': source_counts, 'mutation_controls': mutations, 'builds': build_results,
                      'runs': summaries, 'machine': measures['baseline0']['match']['machine'], 'phase_gate_passed': False}, indent=2))


if __name__ == '__main__':
    main()
