#!/usr/bin/env python3
"""Audit retained bytes and independently recompute measurements; never runs the game."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

from build import audit as build_audit
from metrics import compare, controls, metrics

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def digest(data):
    return hashlib.sha256(data).hexdigest()


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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    for path, sha in manifest['audit_files'].items():
        assert file_digest(HERE / path) == sha, path
    raw = (HERE / 'artifacts.json.gz').read_bytes()
    assert digest(raw) == manifest['archive_sha256']
    payloads = {k: base64.b64decode(v, validate=True) for k, v in json.loads(gzip.decompress(raw))['payloads'].items()}
    assert all(digest(value) == key for key, value in payloads.items())
    rows = {r['path']: r for r in manifest['records']}
    assert len(rows) == len(manifest['records'])
    for record in rows.values():
        if record['embedded']:
            assert len(payloads[record['sha256']]) == record['bytes']
        if args.external and 'original' in record:
            path = Path(record['original'])
            assert path.is_file() and path.stat().st_size == record['bytes'] and file_digest(path) == record['sha256'], path
    def read(path):
        return payloads[rows[path]['sha256']]
    def data(path):
        return json.loads(read(path))
    frozen = data('002/freeze.json')
    assert frozen == data('001/freeze.json') and frozen['source'] == manifest['executed_source']
    assert frozen['counts'] == {'authoring': 75, 'repository': 4724}
    assert frozen['project'] == rows['002/authoring/core.json']['sha256']
    assert frozen['script'] == rows['002/authoring/gameplay.ts']['sha256']
    for generation in ('001', '002'):
        for path, sha in frozen['authoring_files'].items():
            assert rows[generation + '/authoring/' + path]['sha256'] == sha
        commands = data(generation + '/commands.json')
        assert [c['exit'] for c in commands] == ([0] * 4 + [101] if generation == '001' else [0] * 7)
        for command in commands:
            assert command['started_ns'] <= command['ended_ns']
            for kind in ('stdout', 'stderr'):
                retained = rows[generation + '/' + command[kind]['file']]
                assert retained['sha256'] == command[kind]['sha256'] and retained['bytes'] == command[kind]['bytes']
    assert 'pinned Node dependencies' in data('001/export-result.json')['error']
    assert data('002/export-result.json')['success'] is True
    setup = data('002/setup.json')
    assert setup['setup_exit'] == 0 and setup['source_unchanged'] is True
    assert setup['compiler_dependencies']['package-lock.json'] == frozen['repository_files']['package-lock.json']
    assert setup['node']['sha256'] == rows['tools/node']['sha256']
    for path, row in rows.items():
        if path.startswith('source/'):
            assert row['sha256'] == frozen['repository_files'][path.removeprefix('source/')]
    project = data('002/authoring/core.json')
    entities = next(iter(project['scenes'].values()))['entities']
    assert len(project['scenes']) == 1 and len(entities) == 46
    assert sum('Script' in e['components'] for e in entities.values()) == 1
    assert project['settings'].get('physics_mode', 'deterministic') == 'deterministic'
    build_result = build_audit(read, data, rows)
    exported = data('002/package/export.json')
    functional = data('002/functional-win/stdout.json')
    previous = data('historical/win.json')
    command = data('002/functional-win/command.json')
    assert command['exit'] == 0 and command['source'] == manifest['executed_source']
    for name in ('stdout.json', 'stderr.log', 'output.pcm', 'win-input.json', 'win-observe.json'):
        assert command[name + '_sha256'] == rows['002/functional-win/' + name]['sha256']
    assert functional['pcm_sha256'] == rows['002/functional-win/output.pcm']['sha256']
    assert functional['observations']['passed'] is True
    assert rows['002/functional-win/output.pcm']['bytes'] == functional['frames'] * 8
    assert functional['tick'] == 2156 and functional['frames'] == 1724800
    assert functional['logs']['dropped'] == 0
    assert any(e['message'] == 'core.won tick=2066 health=2 pickups=4' for e in functional['logs']['entries'])
    for field in ('tick', 'frames', 'logs', 'observations', 'pcm_sha256'):
        assert functional[field] == previous[field], field
    assert functional['pcm_sha256'] == rows['historical/win.pcm']['sha256']
    measurements = []
    summaries = []
    for i in range(3):
        prefix = '002/measured-' + str(i) + '/'
        measurement = data(prefix + 'measurement.json')
        report = data(prefix + 'stdout.json')
        command = data(prefix + 'command.json')
        assert command['exit'] == 0 and command['ended_ns'] >= command['started_ns']
        assert '--profile' in command['argv']
        assert command['argv'][command['argv'].index('--ticks') + 1] == '2156'
        assert command['argv'][command['argv'].index('--ui-viewport') + 1] == '640x480'
        assert set(command['environment']) == {'PATH', 'TMPDIR'}
        assert all(len(command[k]) == 3 for k in ('load_average_before', 'load_average_after'))
        for name in ('stdout.json', 'stderr.log'):
            assert rows[prefix + name]['sha256'] == command[name + '_sha256']
        assert not read(prefix + 'stderr.log')
        assert measurement['profile_report_sha256'] == rows[prefix + 'stdout.json']['sha256']
        assert report['render_input']['gpu_submitted'] is False and report['render_input']['mode'] == 'cpu-only'
        assert report['pcm_sha256'] == rows[prefix + 'output.pcm']['sha256'] == functional['pcm_sha256']
        assert rows[prefix + 'output.pcm']['bytes'] == report['frames'] * 8
        assert {k: v for k, v in report.items() if k != 'profile'} == functional
        computed = metrics(report)
        assert computed == measurement['metrics']
        assert measurement['phase_gate_passed'] is False
        identities = measurement['identities']
        assert identities['player_sha256'] == exported['player_sha256']
        assert identities['export_sha256'] == rows['002/package/export.json']['sha256']
        assert identities['build_log_sha256'] == rows['002/package/build.log']['sha256']
        assert identities['data'] == exported['data']['files']
        match = measurement['match']
        assert match['codegen'] == exported['codegen'] and match['profiling'] == exported['profiling']['compiler']['features']
        assert match['compiler'] == exported['compiler'] and match['physics'] == 'deterministic'
        assert match['ticks'] == 2156 and match['tick_rate'] == 60
        assert match['project_sha256'] == frozen['project']
        assert match['script_source_sha256'] == data('002/package/project-build.json')['compiler']['bundleSha256']
        assert match['inputs'] == {k: rows[prefix + k + '.json']['sha256'] for k in ('input', 'observe')}
        assert match['inputs'] == {'input': rows['historical/win-input.json']['sha256'], 'observe': rows['historical/win-observe.json']['sha256']}
        if i:
            assert match == measurements[0]['match']
        comparison = None if i == 0 else compare(measurement, measurements[0])
        assert comparison == measurement['comparison']
        summaries.append({'run': i, 'metrics': computed, 'comparison': comparison})
        measurements.append(measurement)
    assert summaries == data('002/matched-runs.json')['runs']
    mutations = controls(data('002/measured-0/stdout.json'), measurements[0], measurements[2])
    git_rows = git_source(manifest['executed_source'], frozen['repository_files']) if args.git_source else 0
    print(json.dumps({'passed': True, 'records': len(rows), 'payloads': len(payloads), 'external_checked': args.external,
                      'git_source_rows': git_rows, 'mutation_controls': mutations, 'build': build_result,
                      'runs': summaries, 'machine': measurements[0]['match']['machine']}, indent=2))


if __name__ == '__main__':
    main()
