#!/usr/bin/env python3
"""Build an embedded TS behavior, remove inputs/build storage, execute its player.

This verifies the first native host==target embedding path. It does not certify
mobile exports, authenticated file loaders, phone performance or the legacy SDK.
"""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from shipping import codegen
from shipping_symbols import backend, package, require, run


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / f'target/release/examples/cook_scene{suffix}')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV'])
    target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target)
    require(kind != 'pdb', 'This symbol audit currently supports Mach-O/ELF; MSVC PDB inspection is not implemented')
    split = 'packed' if kind == 'dsym' else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for the declared shipping build')
    headless, cook = args.headless_binary.resolve(), args.cook_binary.resolve()
    require(headless.is_file() and cook.is_file(), 'Build the headless and cook_scene tools first')
    with tempfile.TemporaryDirectory(prefix='script-inputs-', dir=out) as inputs:
        inputs = Path(inputs)
        project, journal = inputs / 'game.incant.json', inputs / 'history.jsonl'
        run([headless, 'init', project, '--name', 'Embedded script', '--entities', '0'])
        scene_id = next(iter(json.loads(project.read_text())['scenes']))
        transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
        entities = [{'id': f'{n:026d}', 'name': f'Entity {n}', 'parent': None, 'provenance': None,
                     'components': {'Transform': transform}} for n in (1, 2)]
        requests = [
            {'id': 1, 'method': 'command.execute', 'params': {'expected_revision': 0,
             'description': 'Create embedded script fixture', 'commands': [
                 {'op': 'create_entity', 'scene_id': scene_id, 'entity': entity} for entity in entities]}},
            {'id': 2, 'method': 'project.save'},
        ]
        reply = subprocess.check_output([headless, 'rpc', project, '--journal', journal],
                                       input=''.join(json.dumps(r) + '\n' for r in requests), text=True)
        require(all('error' not in json.loads(line) for line in reply.splitlines()), 'Authoring command failed')
        before = {path.name: sha(path) for path in (project, journal)}
        scene = inputs / 'world.scene'
        run([cook, project, scene_id, scene])
        require(before == {path.name: sha(path) for path in (project, journal)}, 'Cook changed authoring')
        template = (ROOT / 'sdk/templates/native-structural/spawn.ts').read_text()
        sentinel = 'INCANT_SOURCE_ONLY_COMMENT_fbd20c41'
        # This runs as already compiled guest bytecode. Each dynamic source route
        # must fail in the actual compiler-disabled shipping player.
        checks = '''
for (const compile of [() => eval('1'), () => (0,eval)('1'),
    () => Function('return 1'), () => (function(){}).constructor('return 1'),
    () => (async function(){}).constructor('return 1'),
    () => (function*(){}).constructor('yield 1')]) {
    let denied = false;
    try { compile(); } catch (_) { denied = true; }
    if (!denied) throw Error('shipping source compiler active');
}
'''
        source = inputs / 'behavior.ts'
        source.write_text(f'// {sentinel}\n' + checks + template)
        commonjs = inputs / 'behavior.js'
        run(['node', ROOT / 'tools/build_script.mjs', source, commonjs])
        environment = dict(os.environ,
                           PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''),
                           INCANT_NATIVE_SCRIPT_SOURCE=str(commonjs),
                           INCANT_NATIVE_SCRIPT_MAP=str(commonjs) + '.map',
                           INCANT_NATIVE_SCRIPT_NAME='embedded-structural.js')
        cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
        # Deleted before the first player launch; a source/cache lookup cannot
        # recover an executable artifact from this build's OUT_DIR.
        with tempfile.TemporaryDirectory(prefix='script-shipping-', dir=out) as storage:
            command = [*cargo, 'build', '-j2', '-p', 'incant_script_player', '--profile', 'shipping',
                       '--locked', '--target', target, '--target-dir', storage,
                       '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            build = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
            (out / 'build.log').write_text(build.stderr)
            require(build.returncode == 0, f'Shipping script build failed; see {out / "build.log"}')
            flags = {}
            for crate in ('incant_runtime', 'incant_script', 'incant_script_player'):
                lines = [line for line in build.stderr.splitlines() if 'Running `' in line
                         and re.search(r'--crate-name ' + crate + r'\b', line) and f'--target {target}' in line]
                require(len(lines) == 1, f'Missing fresh target compile for {crate}')
                require('feature="compiler"' not in lines[0], f'Runtime compiler feature leaked into {crate}')
                values = codegen(lines[0])
                expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort'}
                if crate == 'incant_script_player':
                    expected['lto'] = 'fat'
                if target.endswith('-windows-msvc'):
                    expected['target-feature'] = '+crt-static'
                require(all(values.get(k) == v for k, v in expected.items()), f'Wrong codegen: {values}')
                require(values.get('target-cpu') != 'native', 'Nonportable CPU target')
                flags[crate] = {k: values[k] for k in expected}
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            executables = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
                           and a['target']['name'] == 'incant_script_player' and a.get('executable')]
            require(len(executables) == 1, 'Missing native player executable')
            executable = Path(executables[0])
            symbols = run([llvm_bin / f'llvm-nm{suffix}', '--defined-only', '--demangle', executable])
            require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Bytecode runtime symbols absent')
            for forbidden in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program', 'js_parse_function_decl', 'incant_script::compiler'):
                require(forbidden not in symbols, f'Runtime source compiler symbol linked: {forbidden}')
            # JS_EvalInternal (one C underscore) is the optional-eval dispatcher
            # and must remain to reject dynamic source. __JS_EvalInternal is the
            # actual parser and is forbidden above. Save audit evidence.
            (out / 'source-symbol-audit.json').write_text(json.dumps({
                'bytecode_reader_present': 'JS_ReadObject' in symbols,
                'bytecode_evaluator_present': 'JS_EvalFunction' in symbols,
                'optional_eval_dispatcher_present': 'JS_EvalInternal' in symbols,
                'source_parser_symbols_absent': True,
            }, indent=2) + '\n')
            require(sentinel.encode() not in executable.read_bytes(), 'Source text embedded in player')
            player, identity = package(executable, out, kind, llvm_bin)
            metadata_files = list(Path(storage).glob('*/shipping/build/incant_script_player-*/out/script-symbols.json'))
            require(len(metadata_files) == 1, 'Missing script symbol manifest')
            metadata = json.loads(metadata_files[0].read_text())
            shutil.copy2(metadata_files[0], out / 'symbols/script-symbols.json')
            shutil.copy2(metadata_files[0].with_name('behavior.js.map'), out / 'symbols/behavior.js.map')
        shutil.copy2(scene, out / 'player/world.scene')
    # No authoring inputs or build storage exist at this point. Only the player
    # and checked scene enter the runtime working directory; maps are separate.
    original_scene = (out / 'player/world.scene').read_bytes()
    observations = []
    for ticks in (0, 1, 2, 3, 4, 60):
        command = [player, 'world.scene', str(ticks)]
        first = subprocess.check_output(command, cwd=out / 'player', text=True)
        second = subprocess.check_output(command, cwd=out / 'player', text=True)
        expected = 3 if 1 <= ticks < 4 else 2
        require(first == second and first.startswith(f'tick={ticks} entities={expected}\n'), 'Nonrepeatable script/structural result')
        positions = [ast.literal_eval(re.search(r'(\[[^\[\]]*\])$', line)[1]) for line in first.splitlines()[1:]]
        require(all(p == [0, ticks, 0] for p in positions[:2]), f'Wrong direct component writes: {positions}')
        observations.append({'ticks': ticks, 'entities': expected, 'output': first})
    require((out / 'player/world.scene').read_bytes() == original_scene, 'Script changed cooked scene')
    result = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags,
              'player_sha256': sha(player), 'player_bytes': player.stat().st_size,
              'symbols': identity, 'script_metadata': metadata, 'authoring_preserved': before,
              'inputs_and_build_storage_removed': True, 'runtime_source_compiler_symbols_absent': True,
              'dynamic_source_routes_rejected': True, 'observations': observations,
              'scope': 'Native host==target embedded script; no portable ABI, authenticated file loader, full export, mobile execution or phone gate'}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': True, 'target': target, 'player_bytes': result['player_bytes'], 'evidence': str(out / 'result.json')}))


if __name__ == '__main__':
    main()
