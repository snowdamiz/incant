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
from shipping import cargo_output, codegen
from shipping_symbols import backend, package, require, run


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--physics', action='store_true', help='Also cook/run native rigid-body and angular motion fixture')
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
        physics_scene = None
        character_scene = None
        if args.physics:
            physics_entities = []
            for n, position in [(101, [0, -0.5, 0]), (102, [0, 3, 0]), (103, [5, 4, 0])]:
                spatial = dict(transform, translation=position)
                collider = {'shape': {'type': 'box', 'half_extents': [20, 0.5, 20] if n == 101 else [0.5, 0.5, 0.5]},
                            'density': 1000, 'friction': 0.5, 'restitution': 0, 'sensor': False,
                            'memberships': 4294967295, 'filter': 4294967295}
                components = {'Transform': spatial, 'Collider': collider}
                if n != 101:
                    components['RigidBody'] = {'motion': 'dynamic' if n == 102 else 'kinematic',
                        'gravity_scale': 1, 'linear_damping': 0, 'angular_damping': 0, 'can_sleep': True, 'ccd': True}
                if n == 103:
                    components['Velocity'] = {'linear': [2, 0, 0]}
                    components['AngularVelocity'] = {'angular': [0, 1, 0]}
                physics_entities.append({'id': f'{n:026d}', 'name': f'Body {n}', 'parent': None,
                                         'provenance': None, 'components': components})
            # Authoring remains an atomic command-bus transaction. The normal
            # cook command receives the saved document, not handcrafted bytes.
            commands = [{'op': 'delete_entity', 'scene_id': scene_id, 'entity_id': entity['id']}
                        for entity in entities]
            commands += [{'op': 'create_entity', 'scene_id': scene_id, 'entity': entity} for entity in physics_entities]
            request = [{'id': 3, 'method': 'command.execute', 'params': {
                'expected_revision': 1, 'description': 'Native physics fixture', 'commands': commands}},
                {'id': 4, 'method': 'project.save'}]
            reply = subprocess.check_output([headless, 'rpc', project, '--journal', journal],
                input=''.join(json.dumps(r) + '\n' for r in request), text=True)
            require(all('error' not in json.loads(line) for line in reply.splitlines()), 'Physics authoring failed')
            physics_scene = inputs / 'physics.scene'
            run([cook, project, scene_id, physics_scene])
            character_entities = []
            for n, position, half, sensor in [(201,[0,-0.5,0],[20,0.5,20],False),
                    (202,[0,0.6,0],[0.5,0.5,0.5],False), (203,[2.5,1,0],[0.5,2,4],False),
                    (204,[0.75,0.6,0],[0.1,1,0.5],True)]:
                components = {'Transform':dict(transform,translation=position),
                    'Collider':{'shape':{'type':'box','half_extents':half},'density':1000,
                    'friction':0.5,'restitution':0,'sensor':sensor,'memberships':4294967295,'filter':4294967295}}
                if n == 202:
                    components['RigidBody'] = {'motion':'kinematic','gravity_scale':1,'linear_damping':0,
                        'angular_damping':0,'can_sleep':True,'ccd':True}
                    components['Velocity'] = {'linear':[0,0,0]}
                character_entities.append({'id':f'{n:026d}','name':f'Character fixture {n}',
                    'parent':None,'provenance':None,'components':components})
            commands = [{'op':'delete_entity','scene_id':scene_id,'entity_id':entity['id']} for entity in physics_entities]
            commands += [{'op':'create_entity','scene_id':scene_id,'entity':entity} for entity in character_entities]
            requests = [{'id':5,'method':'command.execute','params':{'expected_revision':2,
                'description':'Native character and trigger fixture','commands':commands}}, {'id':6,'method':'project.save'}]
            reply = subprocess.check_output([headless,'rpc',project,'--journal',journal],
                input=''.join(json.dumps(r)+'\n' for r in requests),text=True)
            require(all('error' not in json.loads(line) for line in reply.splitlines()),'Character authoring failed')
            character_scene = inputs/'character.scene'
            run([cook,project,scene_id,character_scene])

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
        controls = (ROOT / 'sdk/templates/native-input/move.ts').read_text()
        controls = '\n'.join(line for line in controls.splitlines() if not line.startswith('import type'))
        combined = template.replace('export default {', 'const structural = {')
        combined += '\n' + controls.replace('export default {', 'const controls = {')
        queries = (ROOT / 'sdk/templates/native-physics/raycast.ts').read_text()
        queries = '\n'.join(line for line in queries.splitlines() if not line.startswith('import type'))
        combined += '\n' + queries.replace('export function', 'function')
        character = (ROOT/'sdk/templates/native-physics/character.ts').read_text()
        character = '\n'.join(line for line in character.splitlines() if not line.startswith('import type'))
        combined += '\n' + character.replace('export function','function')

        combined += '''
import type { NativeContext } from '../../ts/src/native';
let inputTick = 0;
let characterTick = 0, enters = 0, exits = 0;
let previousCharacter: NativeContext['physics']['computeCharacterMotion'] | undefined;
let previousEvents: NativeContext['physics']['triggerEvents'] | undefined;
let previousRay: NativeContext['physics']['raycast'] | undefined;
export default {update(context: NativeContext) {
    const query = { origin: [-10,10,0], direction: [0,-1,0], max_distance: 100,
        include_sensors: false, memberships: 0xffffffff, filter: 0xffffffff };
    if (previousRay) {
        let denied = false; try { previousRay(query); } catch { denied = true; }
        if (!denied) throw Error('retained native query reactivated');
    }
    previousRay = context.physics.raycast;
    if (previousEvents) {
        let denied=false; try { previousEvents(); } catch { denied=true; }
        if (!denied) throw Error('retained event capability reactivated');
    }
    if (previousCharacter) {
        let denied=false; try { previousCharacter({entity_id:'00000000000000000000000202',translation:[0,0,0]}); } catch { denied=true; }
        if (!denied) throw Error('retained character capability reactivated');
    }
    previousCharacter=context.physics.computeCharacterMotion;
    previousEvents=context.physics.triggerEvents;
    if (context.input.keyboard.held.includes('KeyC')) {
        try { context.physics.computeCharacterMotion({entity_id:'00000000000000000000000000',translation:[0,0,0]}); } catch {}
        return undefined;
    }
    if (context.input.keyboard.held.includes('KeyT')) {
        try { (context.physics.triggerEvents as any)(1); } catch {}
        return undefined;
    }
    const character='00000000000000000000000202', sensor='00000000000000000000000204';
    const hasCharacter=context.chunks.some(c=>c.ids.includes(character));
    if (hasCharacter) {
        const query={entity_id:character,translation:[5/60,-1/60,0] as [number,number,number]};
        if (context.input.keyboard.held.includes('KeyL')) {
            for(let i=0;i<129;i++){try{context.physics.computeCharacterMotion(query)}catch{}}
            return undefined;
        }
        ++characterTick;
        const events=context.physics.triggerEvents();
        for(const event of events) {
            if(event.first===character && event.second===sensor) {
                if(event.entered) ++enters; else ++exits;
            }
        }
        if(context.physics.triggerEvents().length!==events.length)throw Error('events drained');
        driveCharacter(context,character,query.translation);
        if(characterTick>=60 && (enters!==1 || exits!==1))throw Error('missing character sensor lifecycle');
        return undefined;
    }
    if(context.physics.triggerEvents().length)throw Error('unexpected trigger events');

    if (context.input.keyboard.held.includes('KeyQ')) {
        try { context.physics.raycast({...query, direction:[0,0,0]}); } catch {}
        return undefined;
    }
    if (context.input.keyboard.held.includes('KeyB')) {
        for (let i=0;i<1025;i++) { try { context.physics.raycast(query); } catch {} }
        return undefined;
    }
    const hasPhysics = context.chunks.some(c => c.ids.includes('00000000000000000000000101'));
    const ground = groundBelow(context.physics, [-10,10,0], null);
    if (hasPhysics) {
        if (!ground || ground.entity_id !== '00000000000000000000000101' ||
            ground.distance !== 10 || ground.point[1] !== 0 || ground.normal[1] !== 1)
            throw Error('native committed ground query');
        if (groundBelow(context.physics, [-10,10,0], ground.entity_id) !== null)
            throw Error('native query exclusion');
        return undefined;
    }
    if (ground !== null) throw Error('nonphysical scene raycast');
    ++inputTick;
    const input = context.input;
    if ('move' in input.actions) {
        if (inputTick === 1) {
            if (!input.actions.jump.pressed || !input.actions.jump.released || input.actions.jump.active ||
                input.keyboard.pressed[0] !== 'Space' || input.keyboard.released[0] !== 'Space' ||
                !input.actions.use.pressed || input.gestures[0]?.type !== 'tap' ||
                input.touch_changes.length !== 2 || input.connected[0] !== 7 ||
                input.gamepads[7]?.left_stick[0] !== 0.75) throw Error('recorded devices/actions/gesture');
        } else if (inputTick === 2 && (input.actions.jump.pressed || !input.actions.use.released)) {
            throw Error('stale input edges');
        } else if (inputTick === 3 && (input.disconnected[0] !== 7 || input.actions.move.active)) {
            throw Error('recorded disconnect');
        } else if (inputTick === 4 && input.focused) { throw Error('recorded focus'); }
    }
    structural.update(context);
    controls.update(context);
    return undefined;
}} satisfies NativeBehavior;
'''
        source.write_text(f'// {sentinel}\n' + checks + combined)
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
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping script build failed; see {out / "build.log"}')
            flags = {}
            checked_crates = ['incant_input', 'incant_runtime', 'incant_script', 'incant_script_player']
            if args.physics:
                checked_crates += ['incant_physics_runtime', 'rapier3d']
            for crate in checked_crates:
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
        if physics_scene is not None:
            shutil.copy2(physics_scene, out / 'player/physics.scene')
        if character_scene is not None:
            shutil.copy2(character_scene, out / 'player/character.scene')
        # These are bounded startup exchange inputs, not cooked binary assets or
        # session saves. Typed packets are retained after one-time decoding.
        clip = {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
                'ticks': 60, 'frames': [
            {'tick': 1, 'events': [
                {'type': 'key', 'code': 'Space', 'down': True},
                {'type': 'key', 'code': 'Space', 'down': False},
                {'type': 'gamepad_connected', 'id': 7},
                {'type': 'gamepad_axis', 'id': 7, 'axis': 'left_x', 'value': 0.75},
                {'type': 'touch', 'id': 1, 'phase': 'down', 'position': [3, 4]},
                {'type': 'touch', 'id': 1, 'phase': 'up', 'position': [3, 4]}]},
            {'tick': 2, 'events': [{'type': 'gamepad_axis', 'id': 7, 'axis': 'left_x', 'value': 0.25}]},
            {'tick': 3, 'events': [{'type': 'gamepad_disconnected', 'id': 7}]},
            {'tick': 4, 'events': [{'type': 'focus', 'focused': False}]},
            {'tick': 5, 'events': [{'type': 'focus', 'focused': True}]}]}
        actions = {
            'move': {'kind': 'axis1', 'dead_zone': 0, 'bindings': [{'type': 'gamepad_axis', 'id': 7, 'axis': 'left_x'}]},
            'jump': {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'Space'}]},
            'use': {'kind': 'button', 'bindings': [{'type': 'gesture', 'gesture': 'tap'}]}}
        (out / 'player/input.json').write_text(json.dumps(clip) + '\n')
        (out / 'player/actions.json').write_text(json.dumps(actions) + '\n')
    # No authoring inputs or build storage exist at this point. Only the player,
    # checked scene and bounded input-exchange files enter the runtime directory; maps are separate.
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
    recorded = []
    for ticks in (0, 1, 2, 3, 4, 60):
        command = [player, 'world.scene', str(ticks), '--input-recording', 'input.json', '--input-actions', 'actions.json']
        first = subprocess.check_output(command, cwd=out / 'player', text=True)
        second = subprocess.check_output(command, cwd=out / 'player', text=True)
        expected_count = 3 if 1 <= ticks < 4 else 2
        require(first == second and first.startswith(f'tick={ticks} entities={expected_count}\n'), 'Nonrepeatable input result')
        positions = [ast.literal_eval(re.search(r'(\[[^\[\]]*\])$', line)[1]) for line in first.splitlines()[1:]]
        expected_x = 0 if ticks == 0 else 0.75 / 60 if ticks == 1 else 1 / 60
        require(all(abs(p[0] - expected_x) < 1e-12 and p[1:] == [ticks, 3 if ticks else 0]
                    for p in positions[:2]), f'Wrong recorded input movement: {positions}')
        recorded.append({'ticks': ticks, 'entities': expected_count, 'output': first})
    require((out / 'player/world.scene').read_bytes() == original_scene, 'Recorded input changed cooked scene')
    physics_observations = []
    if args.physics:
        physics_bytes = (out / 'player/physics.scene').read_bytes()
        for ticks in (0, 1, 60, 600):
            command = [player, 'physics.scene', str(ticks)]
            first = subprocess.check_output(command, cwd=out / 'player', text=True)
            second = subprocess.check_output(command, cwd=out / 'player', text=True)
            require(first == second and first.startswith(f'tick={ticks} entities=3\n'), 'Nonrepeatable native physics')
            rows = [line for line in first.splitlines() if line.startswith('physics ')]
            require(len(rows) == 3, 'Missing full typed body-state observations')
            arrays = [[ast.literal_eval(value) for value in re.findall(r'\[[^\[\]]*\]', line)[1:]] for line in rows]
            require(arrays[0][0] == [0, -0.5, 0], 'Fixed floor moved')
            require(abs(arrays[2][0][0] - (5 + ticks * 2 / 60)) < 0.001, 'Kinematic body double-integrated')
            require(arrays[2][2:] == [[2, 0, 0], [0, 1, 0]], 'Angular/linear state missing')
            if ticks:
                require(arrays[1][0][1] < 3, 'Dynamic body did not step')
                require(arrays[2][1] != [0, 0, 0, 1], 'Angular motion did not step')
            if ticks == 600:
                require(abs(arrays[1][0][1] - 0.5) < 0.02 and rows[1].endswith(' true'), 'Collision/sleep missing')
            physics_observations.append({'ticks': ticks, 'output': first})
        require((out / 'player/physics.scene').read_bytes() == physics_bytes, 'Runtime changed cooked physics')
    character_observations = []
    if args.physics:
        character_bytes = (out/'player/character.scene').read_bytes()
        for ticks in (0,1,60,120):
            command = [player,'character.scene',str(ticks)]
            first = subprocess.check_output(command,cwd=out/'player',text=True)
            second = subprocess.check_output(command,cwd=out/'player',text=True)
            require(first == second and first.startswith(f'tick={ticks} entities=4\n'),'Nonrepeatable character result')
            rows = [line for line in first.splitlines() if line.startswith('physics ')]
            require(len(rows)==4,'Missing character body observations')
            arrays = [[ast.literal_eval(value) for value in re.findall(r'\[[^\[\]]*\]',line)[1:]] for line in rows]
            position = arrays[1][0]
            if ticks >= 60:
                require(1.4<position[0]<1.51 and 0.49<position[1]<0.53,'Character did not stop at floor and wall')
            character_observations.append({'ticks':ticks,'output':first})
        require((out/'player/character.scene').read_bytes()==character_bytes,'Character mutated cooked input')
    query_refusals = []
    for key in ('KeyQ', 'KeyB', 'KeyC', 'KeyT', *(['KeyL'] if args.physics else [])):
        rejected = dict(clip, frames=[{'tick': 1, 'events': [{'type':'key', 'code':key, 'down':True}]}])
        rejection_clip = out/'player'/f'{key}.json'
        rejection_clip.write_text(json.dumps(rejected)+'\n')
        result = subprocess.run([player, 'character.scene' if key == 'KeyL' else 'world.scene', '1', '--input-recording', rejection_clip.name,
                                 '--input-actions', 'actions.json'], cwd=out/'player', capture_output=True, text=True)
        # A swallowed invalid/over-budget query must never publish world output.
        require(result.returncode != 0 and not result.stdout, f'{key}: rejected query published output')
        expected = 'PhysicsQuery' if key in ('KeyQ','KeyC') else 'QueryInput' if key == 'KeyT' else 'InputLimit'
        require(expected in result.stderr,
                f'{key}: expected query refusal, got {result.stderr}')
        query_refusals.append({'key':key, 'stderr':result.stderr.strip()})
    result = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags,
              'player_sha256': sha(player), 'player_bytes': player.stat().st_size,
              'symbols': identity, 'script_metadata': metadata, 'authoring_preserved': before,
              'inputs_and_build_storage_removed': True,
              'removed_input_scope': 'authoring/compile inputs and temporary Cargo storage; runtime exchange files retained',
              'runtime_exchange_files': ['input.json', 'actions.json', *[f'{entry["key"]}.json' for entry in query_refusals]],
              'runtime_source_compiler_symbols_absent': True,
              'dynamic_source_routes_rejected': True, 'observations': observations,
              'recorded_input_observations': recorded, 'native_physics_observations': physics_observations,
              'native_character_observations': character_observations, 'character_and_trigger_capabilities': True,
              'committed_raycast_and_expired_capabilities': True, 'query_refusals': query_refusals, 'input_exchange_only_at_startup': True,
              'scope': 'Native host==target embedded script; no portable ABI, authenticated file loader, full export, mobile execution or phone gate'}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': True, 'target': target, 'player_bytes': result['player_bytes'], 'evidence': str(out / 'result.json')}))


if __name__ == '__main__':
    main()
