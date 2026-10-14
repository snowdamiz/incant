"""Replay eight unprofiled and six profiled reports without running a player."""
from oracle import functional, metrics, controls, silent_hash, PCM_BYTES
from build import audit as build_audit


def audit(read, data, rows):
    builds = {k: build_audit(read, data, rows, k) for k in ('typescript', 'wasm')}
    assert builds['typescript']['runtime_data'] == builds['wasm']['runtime_data']
    exports = {k: data('packages-001/' + k + '/export.json') for k in builds}
    for key in ('target', 'compiler', 'codegen', 'physics_mode', 'scene_id', 'camera'):
        assert exports['typescript'][key] == exports['wasm'][key]
    features = [exports[k]['profiling']['compiler']['features'] for k in builds]
    assert set(features[0]) == set(features[1])
    for name in features[0]:
        a, b = set(features[0][name]), set(features[1][name])
        assert not a - b and b - a == ({'static-module'} if name in ('incant_script', 'incant_game_player') else set())
    expected_pcm = silent_hash()
    for page in range(4):
        reports = []
        for kind in builds:
            prefix = f'functional-001/functional-{page}-{kind}'
            c, raw = data(prefix + '.json'), data(prefix + '.stdout')
            assert c['exit'] == 0 and '--profile' not in c['argv'] and 'profile' not in raw
            for suffix in ('.stdout', '.stderr'):
                assert c[suffix + '_sha256'] == rows[prefix + suffix]['sha256']
            assert not read(prefix + '.stderr')
            assert c['argv'][c['argv'].index('--ticks') + 1] == '3600'
            assert raw['pcm_sha256'] == rows[prefix + '.pcm']['sha256'] == expected_pcm
            assert rows[prefix + '.pcm']['bytes'] == PCM_BYTES
            reports.append(functional(raw, range(page * 256, (page + 1) * 256)))
        assert reports[0] == reports[1]
    removal = data('functional-001/source-removal-result.json')
    assert removal['private_export_builds_removed'] is True
    freeze = data('export-001/freeze.json')
    for name, row in removal['removed'].items():
        assert row['sha256'] == freeze['input_files'][name]['sha256'] == rows['prepare-003/' + name]['sha256']
    result, plan = data('timed-001/result.json'), data('timing-plan-001/plan.json')
    assert result['plan_sha256'] == rows['timing-plan-001/plan.json']['sha256']
    assert plan['order'] == ['typescript0', 'wasm0', 'wasm1', 'typescript1', 'typescript2', 'wasm2']
    pre = data('timed-001/pre-execution.json')
    assert pre['runner_sha256'] == rows['timing-plan-001/measure.py']['sha256']
    assert pre['quiet_sha256'] == rows['timing-plan-001/quiet.json']['sha256']
    assert pre['compiler_process_names'] == []
    assert data('timing-plan-001/quiet.json')['all_task_builders_terminal'] is True
    assert data('timing-plan-001/quiet.json')['gpu_released'] is True
    assert data('timing-plan-001/supervisor.json')['exit'] == 0
    assert not read('timing-plan-001/supervisor.stderr')
    records, reports, commands = {}, {}, []
    for label in plan['order']:
        kind = label[:-1]; prefix = 'timed-001/' + label + '/'
        m, r, c = data(prefix + 'measurement.json'), data(prefix + 'stdout.json'), data(prefix + 'command.json')
        assert result['records'][label] == rows[prefix + 'measurement.json']['sha256']
        assert c['exit'] == 0 and c['started_ns'] <= c['ended_ns']
        if commands:
            assert commands[-1]['ended_ns'] <= c['started_ns']
        commands.append(c)
        assert '--profile' in c['argv'] and c['argv'][c['argv'].index('--ticks') + 1] == '3600'
        assert set(c['environment']) == {'PATH', 'TMPDIR'}
        for filename in ('stdout.json', 'stderr.log'):
            assert c[filename + '_sha256'] == rows[prefix + filename]['sha256']
        assert not read(prefix + 'stderr.log')
        assert m['profile_report_sha256'] == rows[prefix + 'stdout.json']['sha256']
        phase = 'script.tick' if kind == 'typescript' else 'script.static_module'
        assert m['script_phase'] == phase and metrics(r, phase) == m['metrics']
        assert m['comparison'] is None and m['phase_gate_passed'] is False
        assert r['pcm_sha256'] == rows[prefix + 'output.pcm']['sha256'] == expected_pcm
        assert rows[prefix + 'output.pcm']['bytes'] == PCM_BYTES
        reports[label] = functional(r, [0, 512, 1023])
        identity = m['identities']; exported = exports[kind]
        assert identity['player_sha256'] == exported['player_sha256']
        assert identity['export_sha256'] == rows['packages-001/' + kind + '/export.json']['sha256']
        assert identity['build_log_sha256'] == rows['packages-001/' + kind + '/build.log']['sha256']
        assert identity['data'] == exported['data']['files']
        build = data('packages-001/' + kind + '/project-build.json')
        assert identity['program'] == build['program'] and identity['script_source_sha256'] == build['script']['source_sha256']
        match = m['match']
        assert match['inputs'] == {n: rows['prepare-003/' + n + '.json']['sha256'] for n in ('input', 'observe')}
        assert match['ticks'] == 3600 and match['tick_rate'] == 60
        assert match.get('script_phase', 'script.tick') == phase
        assert match['program'] == identity['program'] and match['script_source_sha256'] == identity['script_source_sha256']
        assert match['project_sha256'] == exported['project_sha256']
        assert match['profiling'] == exported['profiling']['compiler']['features']
        assert match['codegen'] == exported['codegen'] and match['compiler'] == exported['compiler']
        records[label] = m
    first = records['typescript0']['match']
    for label, record in records.items():
        assert reports[label] == reports['typescript0']
        assert record['match'] == records[label[:-1] + '0']['match']
        for key in ('machine', 'target', 'codegen', 'physics', 'compiler', 'ui_options', 'data', 'ticks', 'tick_rate', 'inputs', 'protocol'):
            assert record['match'][key] == first[key]
    computed = []
    for repeat in range(3):
        a, b = records[f'typescript{repeat}']['metrics'], records[f'wasm{repeat}']['metrics']
        ts, wasm = a['script.tick.p95_wall_ns'], b['script.static_module.p95_wall_ns']
        assert ts['lower'] > 0
        ratio = {'defined': True, 'lower': [wasm['lower'], ts['upper']], 'upper': [wasm['upper'], ts['lower']]}
        computed.append({'repeat': repeat, 'metrics': {'typescript': a, 'wasm': b}, 'wasm_over_typescript_p95': ratio})
    assert computed == result['pairs'] and result['phase_gate_passed'] is False
    return {'builds': builds, 'functional_runs': 8, 'timed_runs': 6, 'pairs': computed,
            'machine': first['machine'], 'mutation_controls': controls(data('timed-001/typescript0/stdout.json'))}
