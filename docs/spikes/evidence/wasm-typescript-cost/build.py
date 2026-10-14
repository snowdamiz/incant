"""Recompute actual target rustc codegen/features from retained verbose logs."""
import re
import shlex


def audit(read, data, rows, kind):
    prefix = 'packages-001/' + kind + '/'
    exported, build = data(prefix + 'export.json'), data(prefix + 'project-build.json')
    assert exported['target'] == 'aarch64-apple-darwin' and exported['mode'] == 'headless'
    assert exported['physics_mode'] == build['physics_mode'] == 'deterministic'
    assert exported['startup_source_parsing'] is build['startup_source_parsing'] is False
    assert exported['build_storage_removed_before_startup'] is True
    assert exported['player_sha256'] == rows[prefix + 'player/incant_game_player']['sha256']
    assert exported['project_sha256'] == build['project_sha256'] == rows['prepare-003/' + kind + '/project.json']['sha256']
    actual_data = {n.removeprefix(prefix + 'player/data/'): r['sha256'] for n, r in rows.items() if n.startswith(prefix + 'player/data/')}
    assert actual_data == exported['data']['files']
    assert not any(n.endswith(('.ts', '.js', '.map')) for n in actual_data)
    assert sum(rows[prefix + 'player/data/' + n]['bytes'] for n in actual_data) == exported['data']['bytes']
    wanted = exported['profiling']['compiler']['features'] | exported['physics']['features']
    codegen, features = {}, {}
    for line in read(prefix + 'build.log').decode().splitlines():
        if 'Running `' not in line or '--crate-name ' not in line:
            continue
        tokens = shlex.split(line.split('Running `', 1)[1].removesuffix('`'))
        option = lambda key: tokens[tokens.index(key) + 1] if key in tokens else None
        name = option('--crate-name')
        if name not in wanted or option('--target') != exported['target']:
            continue
        assert '--test' not in tokens
        key = name + ('_library' if name == 'incant_game_player' and option('--crate-type') == 'lib' else '')
        actual = sorted(re.fullmatch(r'feature="(.*)"', tokens[i + 1])[1]
                        for i, word in enumerate(tokens) if word == '--cfg' and tokens[i + 1].startswith('feature="'))
        assert actual == wanted[name], (key, actual, wanted[name])
        assert key not in features
        features[key] = actual
        if name in ('incant_runtime', 'incant_script', 'incant_game_player'):
            flags = dict(tokens[i + 1].split('=', 1) for i, word in enumerate(tokens) if word == '-C' and '=' in tokens[i + 1])
            required = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': 'packed'}
            if key == 'incant_game_player':
                required['lto'] = 'fat'
            assert all(flags.get(k) == v for k, v in required.items())
            codegen[key] = required
    assert codegen == exported['codegen']
    assert set(features) == set(wanted) | {'incant_game_player_library'}
    assert features['incant_game_player'] == features['incant_game_player_library']
    profile = exported['profiling']
    assert profile['mode'] == 'summary' and profile['transport'] == 'none'
    assert profile['compiler']['profile'] == 'shipping' and profile['runtime_opt_in'] == '--profile'
    graph = read(prefix + 'normal-dependencies.txt').decode()
    assert not any(name + ' ' in graph for name in ('incant_cmd', 'incant_doc', 'incant_agent', 'wasmtime', 'tracy-client'))
    if kind == 'wasm':
        assert 'incant_wasm_runtime ' in graph and build['program']['translator'] == 'wasm2c-1.0.42'
    else:
        assert 'incant_wasm_runtime ' not in graph and build['program'] is None
    for name in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
        assert 'enhanced-determinism' in features[name] and not any('simd' in f for f in features[name])
    return {'codegen_units': len(codegen), 'target_feature_units': len(features), 'runtime_data': actual_data}
