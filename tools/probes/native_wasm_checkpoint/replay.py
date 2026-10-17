"""Replay retained data only. Never executes a recorded binary or command."""
import json
from pathlib import Path

import export_native
from . import fixture, oracle
from .route import inventory
from ..native_wasm_restart.verify import equal, require


def read(root, name, cap=1024 * 1024):
    require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'Relative data-only path')
    path = root / name
    require(not path.is_symlink() and path.is_file() and path.stat().st_size <= cap, 'Bounded data record: ' + name)
    return path.read_bytes()


def data(root, name):
    return json.loads(read(root, name))


def managed_logs(root, calls):
    # shipping_symbols.run intentionally merges stderr into stdout. Admit only
    # its complete macOS command sequence; every other call needs both streams.
    merged = [(index, call) for index, call in enumerate(calls) if 'stderr' not in call]
    require(len(merged) == 7, 'Seven merged symbol-tool streams')
    indices = [index for index, _ in merged]
    equal(indices, list(range(indices[0], indices[0] + 7)), 'Contiguous symbol packaging')
    commands = [call['argv'] for _, call in merged]
    require(all(isinstance(args, list) and len(args) >= 3 for args in commands), 'Symbol command shape')
    readobj, dwarf, symbols = commands[0][0], commands[0][-1], commands[1][-1]
    executable, player = commands[2][-1], commands[4][-1]
    require(Path(readobj).is_absolute() and Path(readobj).name == 'llvm-readobj'
            and Path(symbols).name == 'incant_game_player.dSYM'
            and Path(dwarf).parent == Path(symbols) / 'Contents/Resources/DWARF'
            and Path(dwarf).name.startswith('incant_game_player-')
            and Path(executable).name == Path(player).name == 'incant_game_player'
            and Path(player).parent == Path(symbols).parent.parent / 'player', 'Linked symbol paths')
    equal(commands, [[readobj, '--sections', dwarf], ['xcrun', 'dwarfdump', '--verify', symbols],
          ['xcrun', 'dwarfdump', '--uuid', executable], ['xcrun', 'dwarfdump', '--uuid', symbols],
          ['xcrun', 'strip', '-S', '-x', player], ['xcrun', 'dwarfdump', '--uuid', player],
          [readobj, '--sections', player]], 'Exact merged symbol command inventory')
    for index, call in enumerate(calls):
        require(call['finished'] is True and call['cleanup_confirmed'] is True, 'Every managed command closed')
        if index in indices:
            require(call['exit'] == 0, 'Successful merged symbol command')
        for stream in (('stdout',) if index in indices else ('stdout', 'stderr')):
            row = call[stream]
            require(Path(row['path']).name == row['path'], 'Simple managed log path')
            require(oracle.sha(read(root, row['path'], 128 * 1024**2)) == row['sha256'], 'Actual log identity')


def authoring(traces, authored):
    require(len(traces) == 2 and all(t['exit'] == 0 for t in traces), 'Two actual closed public RPC owners')
    first, second = [t['requests'] for t in traces]
    equal([r['request']['method'] for r in first], ['project.read', 'command.execute', 'project.read',
          'history.undo', 'project.read', 'history.redo', 'project.read', 'project.save', 'project.read'],
          'Exact command-bus lifecycle')
    equal([r['request']['method'] for r in second], ['project.read'], 'New journal reader')
    for rows in (first, second):
        for index, row in enumerate(rows, 1):
            require(row['request']['id'] == row['response']['id'] == index
                    and 'error' not in row['response'], 'RPC identity without refusal')
    response = lambda index: first[index]['response']['result']
    before, selected, saved = response(0), response(2), response(8)
    equal(response(4)['project'], before['project'], 'Undo restores all authored edits')
    equal(response(6)['project'], selected['project'], 'Redo restores exact selected program/scene')
    equal(saved['project'], selected['project'], 'Persisted scene unchanged')
    equal(second[0]['response']['result'], saved, 'Fresh reopened journal')
    equal(saved, authored, 'Actual public read receipt')
    command = first[1]['request']['params']
    require(command['expected_revision'] == before['revision']
            and response(1)['revision'] == before['revision'] + 1, 'Atomic revision')
    equal([c['op'] for c in command['commands']], ['create_entity', 'create_entity', 'set_scene_program'],
          'Public entity/program command inventory')


def audit_build(root, owner, exported):
    calls = owner['calls']
    selected = {}
    for package, profile in [('incant_headless', 'ci'), ('incant_wasm_cook', None), ('incant_game_player', 'shipping')]:
        matches = [c for c in calls if 'build' in c['argv'] and package in c['argv']]
        require(len(matches) == 1, 'One fresh build of ' + package)
        call = matches[0]
        require(call['exit'] == 0 and call['finished'] is True and call['cleanup_confirmed'] is True,
                'Actual completed build')
        args = call['argv']
        require('-j2' in args and '--locked' in args and '--target-dir' in args, 'Bounded fresh Cargo route')
        if profile is None:
            require('--profile' not in args and 'cook-target' in args[args.index('--target-dir') + 1], 'Default fresh cooker')
        else:
            require(args[args.index('--profile') + 1] == profile, 'Actual build profile')
            text = read(root, call['stdout']['path'], 128 * 1024**2).decode()
            messages = [json.loads(line) for line in text.splitlines() if line.startswith('{')]
            equal(messages[-1], {'reason': 'build-finished', 'success': True}, 'Cargo terminal')
            artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                         and m['target']['name'] == package and m.get('executable')]
            require(len(artifacts) == 1, 'Unique real executable')
            artifact = artifacts[0]
            features = ['default', 'physics-deterministic'] if package == 'incant_headless' else [
                'physics-deterministic', 'project', 'render-input', 'static-module']
            equal(sorted(artifact['features']), features, 'Actual feature closure selection')
            equal([artifact['profile']['test'], artifact['fresh'], artifact['profile']['opt_level']],
                  [False, False, '1' if profile == 'ci' else '3'], 'Fresh ordinary artifact')
            selected[package] = artifact
        if package == 'incant_game_player':
            text = read(root, call['stderr']['path'], 128 * 1024**2).decode()
            equal(export_native.verify_codegen(text, 'aarch64-apple-darwin', 'packed'), exported['codegen'],
                  'Actual shipping unit codegen')
    equal(selected['incant_headless'], data(root, 'headless.json')['cargo_artifact'], 'Copied headless producer')
    graph = read(root, 'package/normal-dependencies.txt', 1024 * 1024).decode()
    trees = [c for c in calls if len(c['argv']) > 1 and c['argv'][1] == 'tree']
    require(len(trees) == 1, 'Unique actual selected dependency graph')
    equal(read(root, trees[0]['stdout']['path']).decode(), graph, 'Graph file bound to actual Cargo stdout')
    equal(export_native.normal_dependencies(graph), exported['normal_dependencies'], 'Selected normal graph')
    require('compiler' not in [x.strip() for line in graph.splitlines() for x in line.split('|')[-1].split(',')],
            'No runtime source compiler feature')
    require(not any('shipping_session' in key for key in exported), 'Ordinary fresh exporter, no session shortcut')
    symbols = [c for c in calls if Path(c['argv'][0]).name == 'llvm-nm']
    require(len(symbols) == 1, 'Actual symbol audit invocation')
    text = read(root, symbols[0]['stdout']['path'], 128 * 1024**2).decode()
    require(all(name in text for name in ('JS_ReadObject', 'JS_EvalFunction'))
            and not any(name in text for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program',
                                                   'js_parse_function_decl', 'incant_script::compiler')),
            'Actual native source-compiler symbol boundary')
    equal(exported['symbol_audit'], {'source': 'llvm-nm defined symbols', 'sha256': oracle.sha(text.encode())},
          'Symbol receipt bound to actual output')


def replay(root):
    root = Path(root)
    owner = data(root, 'owner.json')
    require(owner['passed'] is True and owner['error'] is None, 'Successful bounded owner')
    require(owner['seconds'] <= 5400, 'Original aggregate deadline')
    for name, cap in [('target', 12 * 1024**3), ('scratch', 8 * 1024**3),
                      ('output', 2 * 1024**3), ('rss', 8 * 1024**3)]:
        require(type(owner['peaks'][name]) is int and 0 < owner['peaks'][name] <= cap, 'Resource peak: ' + name)
    managed_logs(root, owner['calls'])
    outputs = {}
    for name, (start, end) in oracle.RUNS.items():
        receipt = data(root, name + '-process.json')
        equal([receipt['returncode'], receipt['stderr']], [0, ''], 'Actual successful player')
        equal(json.loads(receipt['stdout']), data(root, name + '.json'), 'Unmodified real stdout')
        equal(data(root, name + '-input.json'), fixture.recording(start, end - start), 'Exact rebased public input')
        equal(data(root, name + '-observe.json'), fixture.observation(start, end), 'Exact mathematical observation plan')
        outputs[name] = {'report': data(root, name + '.json'), 'pcm': read(root, name + '.pcm', 256000),
                         'saved': read(root, name + '.save')}
    oracle.joined(outputs)
    names = [*oracle.RUNS, *oracle.REFUSALS, 'pcm-exists', 'save-exists']
    player_pids = set()
    leaders = {row['pid'] for row in owner['owned']}
    for name in names:
        receipt = data(root, name + ('-process.json' if name in oracle.RUNS else '-refusal.json'))
        calls = [c for c in owner['calls'] if c['argv'] == receipt['argv']]
        require(len(calls) == 1 and calls[0]['exit'] == receipt['returncode'], 'Distinct actual owned player call')
        pid = calls[0]['pid']
        require(type(pid) is int and pid > 0 and pid in leaders and pid not in player_pids,
                'Distinct observed native process owner')
        player_pids.add(pid)
        for stream in ('stdout', 'stderr'):
            equal(read(root, calls[0][stream]['path'], 1024 * 1024).decode(), receipt[stream], 'Receipt bound to managed output')
        require('--package' in receipt['argv'] and '--static-module' not in receipt['argv'], 'Public package selection')
        if name not in oracle.RUNS:
            oracle.refusal(receipt, name)
            if name in oracle.REFUSALS:
                require(read(root, name + '.save') == oracle.mutate_checkpoint(outputs['prefix']['saved'], name),
                        'Exact coherent negative input')
    package = root / 'package/player'
    files = data(root, 'package-files.json')
    equal(inventory(package), files, 'Exact immutable runtime inventory and modes')
    require(all(not name.endswith(('.wasm', '.rs', '.c', '.rlib', '.js', '.map')) for name in files), 'Source-free package')
    for name in ('authoring', 'trusted-wabt', 'guest-build'):
        require(not (root / name).exists() and not (root / name).is_symlink(), 'Owned source deleted')
    require(not list((root / 'private-builds').iterdir()), 'Fresh build scratch removed')
    exported = data(root, 'package/export.json')
    rpc_paths = sorted(root.glob('rpc-*.stderr.json'), key=lambda p: int(p.name.split('-')[1].split('.')[0]))
    authoring([data(root, p.name) for p in rpc_paths], data(root, 'authored-read.json'))
    authored = data(root, 'authored.json')
    program = {'kind': 'wasm', 'module': authored['module'], 'fingerprint': authored['fingerprint'],
               'input_sha256': authored['guest_sha256'], 'prepared_sha256': authored['prepared_sha256'],
               'registry_sha256': authored['registry_sha256'], 'translator': 'wasm2c-1.0.42'}
    equal(exported['program'], program, 'Public source identity')
    equal(data(root, 'package/project-build.json')['program'], program, 'Actual generated program identity')
    equal(exported['player_sha256'], files['incant_game_player']['sha256'], 'Exact tested native executable')
    equal(data(root, 'guest.json')['artifact']['sha256'], authored['guest_sha256'], 'Real SDK guest')
    imported = data(root, 'imports.json')
    require(imported['first']['changed'] is True and imported['first']['asset']['kind'] == 'wasm'
            and imported['audio']['changed'] is True and imported['audio']['asset']['kind'] == 'audio'
            and imported['repeated']['changed'] is False and imported['repeated']['cache_hit'] is True
            and imported['first']['revision'] == imported['repeated']['revision'], 'Public importer results')
    equal([imported['first']['asset']['id'], imported['first']['asset']['sha256']],
          [authored['module'], authored['fingerprint']], 'Public module selection identity')
    imports = [c for c in owner['calls'] if len(c['argv']) > 1 and c['argv'][1] == 'import']
    require(len(imports) == 3, 'Three actual public import invocations')
    for call, name in zip(imports, ('first', 'repeated', 'audio')):
        equal(json.loads(read(root, call['stdout']['path']).decode()), imported[name], 'Actual import stdout')
    require(exported['build_storage_removed_before_startup'] is True and exported['startup_source_parsing'] is False,
            'Ordinary source-free startup')
    audit_build(root, owner, exported)
    return {'passed': True, 'four_fresh_positive_processes': True, 'six_exact_refusals': True,
            'exact_checkpoint_and_pcm_continuation': True, 'player_sha256': exported['player_sha256'],
            'cleanup_scope': 'owner children only; wrapper absence requires post-exit closeout and exact inventory'}


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    print(json.dumps(replay(parser.parse_args().output), indent=2))
