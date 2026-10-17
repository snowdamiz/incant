"""Recheck actual build logs as text; never run the recorded compiler or player."""
import json
import re

from oracle import FEATURES, GUEST, HEAD, HEADLESS, PLAYER, PREPARED, REGISTRY, equal, require, session, sha


def artifact(text, name, features, optimization):
    messages = [json.loads(line) for line in text.splitlines() if line.startswith('{')]
    equal(messages[-1], {'reason': 'build-finished', 'success': True}, 'Cargo terminal result')
    selected = [row for row in messages if row.get('reason') == 'compiler-artifact'
                and row['target']['name'] == name and row.get('executable')]
    require(len(selected) == 1, 'Unique actual executable artifact')
    row = selected[0]
    equal(sorted(row['features']), features, 'Exact actual artifact features')
    equal([row['profile']['opt_level'], row['profile']['test'],
           row['profile']['debug_assertions'], row['fresh']],
          [optimization, False, False, False], 'Fresh ordinary artifact profile')
    return row


def audit(text, data):
    exported = data('evidence/package/export.json')
    player = artifact(text('evidence/013.stdout.log'), 'incant_game_player', FEATURES, '3')
    headless = artifact(text('evidence/000.stdout.log'), 'incant_headless',
                        ['default', 'physics-deterministic', 'wasmtime-host'], '1')
    equal(headless, data('evidence/headless.json')['cargo_artifact'], 'Headless artifact receipt')
    equal(data('evidence/headless.json')['source_head'], HEAD, 'Headless producer')
    equal(data('evidence/headless.json')['copied']['sha256'], HEADLESS, 'Copied headless')
    require('/shipping/' in player['executable'], 'Shipping destination')
    lines = [line for line in text('evidence/013.stderr.log').splitlines()
             if 'Running `' in line and '--target aarch64-apple-darwin' in line]
    units = [('incant_runtime', 'lib', 'incant_runtime'), ('incant_script', 'lib', 'incant_script'),
             ('incant_game_player', 'lib', 'incant_game_player_library'),
             ('incant_game_player', 'bin', 'incant_game_player')]
    codegen = {}
    for name, kind, key in units:
        commands = [line for line in lines if re.search(r'--crate-name ' + name + r'\b', line)
                    and re.search(r'--crate-type ' + kind + r'(?:\s|$)', line)]
        require(len(commands) == 1, 'Unique fresh shipping invocation')
        command = commands[0]
        require('--test' not in command and 'feature="compiler"' not in command,
                'No test/source-compiler unit')
        flags = {k: v.strip('\"\'') for k, v in
                 re.findall(r'(?:^|\s)-C\s*([a-z-]+)=([^\s`]+)', command)}
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort',
                    'debuginfo': '2', 'split-debuginfo': 'packed'}
        if kind == 'bin':
            expected['lto'] = 'fat'
        equal({k: flags.get(k) for k in expected}, expected, 'Shipping codegen')
        require(flags.get('target-cpu') != 'native', 'Portable target codegen')
        codegen[key] = expected
    equal(exported['codegen'], codegen, 'Codegen receipt')
    graph = []
    forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook',
                 'incant_shader_cook', 'incant_source', 'incant_play', 'incant_play_legacy',
                 'incant_wasm_cook', 'incant_wasm_dev', 'loro', 'automerge'}
    for line in text('evidence/012.stdout.log').splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
        require(match is not None, 'Selected normal graph syntax')
        name = match[1]
        require(name not in forbidden and not name.startswith(('wasmtime', 'cranelift')),
                'Authoring/JIT normal dependency')
        require('compiler' not in match[2].split(','), 'Runtime compiler feature')
        graph.append(name)
    require(graph and graph[0] == 'incant_game_player', 'Selected player graph root')
    equal(sorted(set(graph)), exported['normal_dependencies'], 'Selected graph receipt')
    symbols = text('evidence/014.stdout.log')
    require(all(name in symbols for name in ('JS_ReadObject', 'JS_EvalFunction')), 'Bytecode runtime')
    require(not any(name in symbols for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal',
                'js_parse_program', 'js_parse_function_decl', 'incant_script::compiler'))
            and not re.search(r'incant_script[0-9]+compiler', symbols), 'Source compiler symbol')
    equal(exported['symbol_audit'], {'source': 'llvm-nm defined symbols',
                                   'sha256': sha(symbols.encode())}, 'Symbol audit binding')
    equal(exported['compiler'], text('evidence/006.stdout.log'), 'Exact compiler version')
    equal([exported['target'], exported['mode'], exported['startup_source_parsing'],
           exported['build_storage_removed_before_startup'], exported['signed']],
          ['aarch64-apple-darwin', 'headless', False, True, False], 'Ordinary source-free export')
    equal(exported['player_sha256'], PLAYER, 'Tested shipping player')
    startup = data('evidence/022.stdout.log')
    equal(startup, exported['startup'], 'Actual exporter startup output')
    equal([startup['session'], startup['tick'], startup['frames'], startup['pcm_sha256']],
          [session(1), 0, 800, sha(bytes(6400))], 'Exporter startup replacement')
    first = data('evidence/imports.json')['first']
    program = {'kind': 'wasm', 'module': first['asset']['id'],
               'fingerprint': first['asset']['sha256'], 'input_sha256': GUEST,
               'prepared_sha256': PREPARED, 'registry_sha256': REGISTRY, 'translator': 'wasm2c-1.0.42'}
    for record in ('evidence/package/export.json', 'evidence/package/project-build.json',
                   'evidence/result.json'):
        equal(data(record)['program'], program, 'Public imported program reaches shipping owner')
    equal(exported['project_sha256'], data('evidence/authored-identity.json')['project_sha256'],
          'Authored project reaches ordinary exporter')
    inventory = data('evidence/package-files.json')
    equal(sorted(inventory), ['data/clips.bin', 'data/empty-input.json', 'data/input-actions.json',
                             'data/package.json', 'data/world.scene', 'incant_game_player'],
          'Exact source-free runtime inventory')
    equal(inventory['incant_game_player'], {'bytes': 9750240, 'sha256': PLAYER, 'executable': True},
          'Copied executable identity')
    equal({name.removeprefix('data/'): row['sha256'] for name, row in inventory.items()
           if name.startswith('data/')}, exported['data']['files'], 'Runtime data hashes')
    equal(sum(row['bytes'] for name, row in inventory.items() if name.startswith('data/')),
          exported['data']['bytes'], 'Runtime data length')
    return player
