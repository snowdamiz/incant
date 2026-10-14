"""Independently bind shipping receipts to actual retained Cargo output."""
import json
import shlex


def audit(read, data, rows, prefix):
    original_read, original_data = read, data
    read = lambda path: original_read(prefix + path.removeprefix("002/"))
    data = lambda path: original_data(prefix + path.removeprefix("002/"))
    rows = {"002/" + p.removeprefix(prefix): r for p, r in rows.items() if p.startswith(prefix)}
    exported = data('002/package/export.json')
    build = data('002/package/project-build.json')
    assert exported['mode'] == 'headless' and exported['target'] == 'aarch64-apple-darwin'
    assert exported['physics_mode'] == exported['physics_variant'] == 'fast'
    assert build['physics_mode'] == build['physics_variant'] == 'fast'
    assert build['startup_source_parsing'] is exported['startup_source_parsing'] is False
    assert exported['build_storage_removed_before_startup'] is True
    assert build['script']['embedded_source'] is False and build['script']['format'] == 3
    assert len(build['compiler']['sources']) == len(build['compiler']['modules']) == 2
    assert {m['sha256'] for m in build['compiler']['modules']} == {rows['002/authoring/' + p]['sha256'] for p in ('gameplay.ts', 'stress.ts')}
    assert build['project_sha256'] == exported['project_sha256'] == rows['002/authoring/core.json']['sha256']
    assert exported['player_sha256'] == rows['002/package/player/incant_game_player']['sha256']
    actual_data = {p.removeprefix('002/package/player/data/'): r['sha256'] for p, r in rows.items() if p.startswith('002/package/player/data/')}
    assert actual_data == exported['data']['files']
    assert not any(p.endswith(('.ts', '.js', '.map')) for p in actual_data)
    assert sum(rows['002/package/player/data/' + p]['bytes'] for p in actual_data) == exported['data']['bytes']
    assert rows['002/package/build.log']['sha256'] == rows['002/command-04-stderr.log']['sha256']
    assert rows['002/package/normal-dependencies.txt']['sha256'] == rows['002/command-03-stdout.log']['sha256']
    assert exported['compiler'].strip() == read('002/command-01-stdout.log').decode().strip()
    assert exported['symbol_audit']['sha256'] == rows['002/command-05-stdout.log']['sha256']
    seen = {}
    for line in read('002/command-04-stderr.log').decode().splitlines():
        if 'Running `' not in line or '--crate-name ' not in line:
            continue
        tokens = shlex.split(line.split('Running `', 1)[1].removesuffix('`'))
        def option(name):
            return tokens[tokens.index(name) + 1] if name in tokens else None
        name = option('--crate-name')
        if name not in ('incant_runtime', 'incant_script', 'incant_game_player') or option('--target') != exported['target']:
            continue
        assert '--test' not in tokens
        key = name + ('_library' if name == 'incant_game_player' and option('--crate-type') == 'lib' else '')
        flags = dict(tokens[i + 1].split('=', 1) for i, v in enumerate(tokens) if v == '-C' and '=' in tokens[i + 1])
        required = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': 'packed'}
        if key == 'incant_game_player':
            required['lto'] = 'fat'
        assert all(flags.get(k) == v for k, v in required.items()), (key, flags)
        assert key not in seen
        seen[key] = required
    assert seen == exported['codegen']
    wanted = exported['profiling']['compiler']['features'] | exported['physics']['features']
    artifacts = {}
    for line in read('002/command-04-stdout.log').splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        target = message.get('target', {})
        name = target.get('name')
        if message.get('reason') != 'compiler-artifact' or name not in wanted:
            continue
        if not any('/aarch64-apple-darwin/shipping/' in f for f in message['filenames']):
            continue
        kind = tuple(target['kind'])
        artifacts[(name, kind)] = message['features']
        assert message['features'] == wanted[name], (name, message['features'])
    assert {name for name, _ in artifacts} == set(wanted)
    assert ('incant_game_player', ('lib',)) in artifacts and ('incant_game_player', ('bin',)) in artifacts
    assert exported['profiling']['compiler']['profile'] == 'shipping'
    assert exported['profiling']['mode'] == 'summary' and exported['profiling']['transport'] == 'none'
    graph = read('002/command-03-stdout.log').decode()
    assert not any(name in graph for name in ('incant_doc ', 'incant_cmd ', 'incant_agent ', 'tracy-client '))
    for name in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d'):
        features = wanted[name]
        assert 'enhanced-determinism' not in features
        # Preserve actual resolved features; 'fast' is not a portable SIMD claim.
    return {'codegen_units_verified': len(seen), 'target_feature_artifacts': len(artifacts), 'runtime_data_files': len(actual_data)}
