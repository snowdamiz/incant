#!/usr/bin/env python3
"""Authored nested trees through public commands, shipping bytecode and save/resume.

Every sample is observed independently. CPU render-input evidence only; numerical
skinning, rendered appearance and reference-device performance have separate tests.
"""
import argparse
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
BEHAVIOR = Path(__file__).with_suffix('') / 'behavior.ts'
BASE = runpy.run_path(str(ROOT / 'tools/probes/native-animation-graph-export.py'))
require, write, digest, fingerprints = [BASE[k] for k in ('require', 'write', 'digest', 'fingerprints')]
id = lambda n: f'{n:026}'
SCRIPT, FIRST, CAMERA, REST = map(id, (1, 2, 3, 4))
PARAMETERS = list(map(id, (10, 11, 12)))
STATES = list(map(id, (20, 21)))
OBSERVED = list(map(id, range(200, 208))) + list(map(id, (220, 221, 222, 230)))
TICKS = 56
CHECK_TICKS = (1, 4, 5, 8, 9, 11, 12, 13, 16, 17, 20, 21, 25, 29, 33, 37, 41, 45, 56)


def fixture(root):
    BASE['fixture'](root)
    model = json.loads((root / 'rig.gltf').read_text())
    data = bytearray((root / 'rig.bin').read_bytes())
    originals = model['animations']
    model['animations'] = []
    for n in range(7):
        duration = n + 1
        view, accessor = len(model['bufferViews']), len(model['accessors'])
        model['bufferViews'].append({'buffer': 0, 'byteOffset': len(data), 'byteLength': 8})
        data.extend(struct.pack('<2f', 0, duration))
        model['accessors'].append({'bufferView': view, 'componentType': 5126, 'type': 'SCALAR',
                                  'count': 2, 'min': [0], 'max': [duration]})
        animation = json.loads(json.dumps(originals[n % 2]))
        animation['name'] = f'clip-{n}'
        animation['samplers'][0]['input'] = accessor
        model['animations'].append(animation)
    model['buffers'][0]['byteLength'] = len(data)
    (root / 'rig.bin').write_bytes(data)
    write(root / 'rig.gltf', model)


def tree(offset):
    leaves = list(map(id, range(offset, offset + 8)))
    grids = list(map(id, (offset + 8, offset + 9)))
    top = id(offset + 10)
    nodes = {node: {'kind': 'clip', 'clip': n if n < 7 else None} for n, node in enumerate(leaves)}
    for n, grid in enumerate(grids):
        nodes[grid] = {'kind': 'blend2d', 'parameters': PARAMETERS[:2], 'x': [0, 1], 'y': [0, 1],
                       'children': leaves[n * 4:n * 4 + 4]}
    nodes[top] = {'kind': 'blend1d', 'parameter': PARAMETERS[2], 'points': [
        {'threshold': 0, 'node': grids[0]}, {'threshold': 1, 'node': grids[1]}]}
    return {'root': top, 'nodes': nodes}


def graph(rest=False):
    if rest:
        return {'initial': id(30), 'parameters': {}, 'states': {id(30): {'looping': False,
            'motion': {'kind': 'tree', 'cycle_seconds': .25,
                       'tree': {'root': id(31), 'nodes': {id(31): {'kind': 'clip', 'clip': None}}}}}}, 'transitions': {}}
    return {'initial': STATES[0], 'parameters': {p: {'initial': v, 'min': 0, 'max': 1}
            for p, v in zip(PARAMETERS, (.25, .5, .75))}, 'states': {
                state: {'looping': True, 'motion': {'kind': 'tree', 'cycle_seconds': cycle, 'tree': tree(offset)}}
                for state, cycle, offset in zip(STATES, (2, 1), (100, 120))}, 'transitions': {}}


def oracle():
    """Integer quarter-cycle clock; independent tensor-product pose enumeration.

    Commands commit after this tick's observation. Elapsed time is counted as
    whole ticks; it advances at zero speed but stops on explicit master pause.
    """
    rows = {}
    quarter = elapsed = state = 0
    speed, active = 1, True
    params = [.25, .5, .75]
    for tick in range(1, TICKS + 1):
        if active:
            quarter = (quarter + speed) % (8 if state == 0 else 4)
            elapsed += 1
        phase = quarter / (8 if state == 0 else 4)
        poses = []
        x, y, z = params
        # Canonical cooked nodes are child-before-parent. The evaluator emits
        # leaves in reverse topological order; weights still come independently
        # from this fixture's tensor product, without calling the runtime.
        for n in reversed(range(8)):
            weight = (x if n & 1 else 1 - x) * (y if n & 2 else 1 - y) * (z if n & 4 else 1 - z)
            if weight:
                poses.append([n if n < 7 else -1, (n + 1) * phase if n < 7 else 0, weight])
        values = {id(200 + n): pose for n, pose in enumerate(poses + [[-2, 0, 0]] * (8 - len(poses)))}
        values[id(220)] = [len(poses), quarter / 4, 3 if active else 2]
        values[id(221)] = list(params)
        values[id(222)] = [elapsed / 60, state, int(not active)]
        # Rest begins paused, completes in one 15x-speed tick, then is paused and resumed.
        values[id(230)] = [0 if tick <= 5 else .25, 0 if tick <= 5 else 4, int(tick <= 5 or 9 <= tick <= 11)]
        rows[tick] = values
        if tick == 4: params[0] = 1
        if tick in (8, 12): active = tick == 12
        if tick == 10: params[1:] = [.25, 0]
        if tick == 16: speed = -1
        if tick == 20: params[0], params[2] = .5, .5
        if tick in (24, 28): state, quarter, elapsed = int(tick == 24), 0, 0
        if tick in (32, 36): speed = int(tick == 36)
        if tick == 40: params = [0, 0, 1]
        if tick == 44: params = [.25, .5, .75]
    return rows


EXPECTED = oracle()


def observations(start, end, wrong=False):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': OBSERVED,
            'checks': [{'name': f'{tick}-{entity}', 'tick': tick, 'expect': {'type': 'vector',
                       'entity': entity, 'property': 'translation',
                       'value': [value[0] + int(wrong), *value[1:]], 'tolerance': 1e-9}}
                       for tick in CHECK_TICKS if start < tick <= end for entity, value in EXPECTED[tick].items()]}


def check_logs(logs, start, end):
    require(logs['dropped'] == 0 and len(logs['entries']) == 2 * (end - start), 'Accepted log count')
    seen = set()
    for entry in logs['entries']:
        row = json.loads(entry['message'])
        tick, entity, seconds, elapsed, flags, paused, poses = row
        require(entry['tick'] == tick and entry['level'] == 'info' and start < tick <= end, 'Log tick/level')
        require((tick, entity) not in seen and entity in (FIRST, REST), 'Log identity')
        seen.add((tick, entity))
        expected = EXPECTED[tick]
        if entity == FIRST:
            n, want_seconds, want_flags = expected[id(220)]
            want_elapsed, _, want_paused = expected[id(222)]
            want_poses = [expected[id(200 + i)] for i in range(n)]
        else:
            want_seconds, want_flags, want_paused = expected[id(230)]
            want_elapsed = max(0, tick - 5 - (min(tick, 11) - 8 if tick >= 9 else 0)) / 60
            want_poses = [[-1, 0, 1]]
        require(flags == want_flags and paused == want_paused and abs(seconds - want_seconds) < 1e-9
                and abs(elapsed - want_elapsed) < 1e-9, f'Graph clock/flags {entity}@{tick}: {row}')
        require(len(poses) == len(want_poses) and all(len(p) == 3 for p in poses), 'Pose count/layout')
        require(all(math.isfinite(a) and abs(a - b) < 1e-9 for p, q in zip(poses, want_poses) for a, b in zip(p, q)),
                f'Independent full pose mismatch {entity}@{tick}: {poses} != {want_poses}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--stage', choices=('author', 'export', 'all'), default='all')
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    import export_native
    run = native_process.run
    commands = []
    def recorded(command, **kwargs):
        index = len(commands)
        record = {'argv': list(map(str, command)), 'at': datetime.datetime.now(datetime.timezone.utc).isoformat()}
        commands.append(record)
        start = time.monotonic()
        result = run(list(map(str, command)), **kwargs)
        record.update(returncode=result.returncode, seconds=time.monotonic()-start)
        for stream in ('stdout', 'stderr'):
            value = getattr(result, stream)
            if isinstance(value, (str, bytes)):
                path = output / f'command-{index:03}-{stream}.txt'
                path.write_bytes(value.encode() if isinstance(value, str) else value)
                record[stream] = {'file': path.name, 'sha256': digest(path)}
        write(output / 'commands.json', commands)
        return result
    native_process.run = recorded
    def execute(command, **kwargs):
        result = recorded(command, **kwargs)
        require(result.returncode == 0, str(result.stderr)[-8000:])
        return result
    inputs, project = output / 'authoring', output / 'authoring/project.json'
    if args.stage != 'export':
        output.mkdir(parents=True, exist_ok=False)
        inputs.mkdir()
        execute([binary, 'init', project, '--entities', '0'])
        fixture(inputs)
        imported = json.loads(execute([binary, 'import', project, 'rig.gltf']).stdout)
        document = json.loads(project.read_text())
        scene = next(iter(document['scenes']))
        require(document['settings']['tick_rate'] == 60, 'Default tick rate')
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        journal = project.with_suffix('.journal.jsonl')
        rpc = Rpc(binary, project, journal)
        calls = []
        def call(method, params=None):
            result = rpc.call(method, params)
            calls.append({'method': method, 'params': params, 'result': result})
            write(output / 'public-commands.json', calls)
            return result
        try:
            source = BEHAVIOR.read_text()
            revision = call('project.read')['revision']
            batch = [{'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                     {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Tree observations', 'path': 'tree.ts'}}]
            for entity in (FIRST, REST):
                rest = entity == REST
                batch.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity,
                    'name': 'Completed rest tree' if rest else 'Nested eight-pose tree', 'parent': None, 'provenance': None,
                    'components': {'Transform': BASE['transform']([0, 0, 0]),
                        'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': False},
                        'Animation': {'clip': None, 'seconds': 0, 'speed': 15, 'playing': not rest, 'looping': not rest},
                        'AnimationGraph': graph(rest), 'Script': {'source': SCRIPT, 'props': {'rest': rest}}}}})
            for entity in OBSERVED + [CAMERA]:
                components = {'Transform': BASE['transform']([0, 0, 8] if entity == CAMERA else [0, 0, 0])}
                if entity == CAMERA:
                    components['Camera'] = {'fov_degrees': 60, 'near': .1, 'far': 200, 'projection': {'kind': 'perspective'}}
                batch.append({'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity,
                    'name': 'Pose observation', 'parent': None, 'provenance': None, 'components': components}})
            created = call('command.execute', {'expected_revision': revision, 'description': 'Author nested animation tree export proof', 'commands': batch})
            require(created['revision'] == revision + 1, 'One atomic transaction')
            call('history.undo')
            require(call('project.read')['project'] == document and not (inputs / 'tree.ts').exists(), 'Atomic Undo')
            call('history.redo')
            authored = call('project.read')
            played = call('play.run', {'seconds': TICKS / 60, 'camera': CAMERA, 'observe': observations(0, TICKS)})
            require(played['passed'] and played['state']['cpu_render_input'] and not played['state']['gpu_submitted'], 'Public CPU Play')
            check_logs(played['state']['logs'], 0, TICKS)
            require(call('project.read') == authored, 'Play changed authoring')
            require(call('project.save')['saved'], 'Save')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call('project.read') == authored, 'Fresh recovery')
            require(call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source, 'Script recovery')
        finally:
            rpc.close()
        write(output / 'authoring.json', {'project_sha256': digest(project), 'binary_sha256': digest(binary),
            'files': fingerprints(inputs), 'logs': played['state']['logs'], 'source_sha256': digest(BEHAVIOR),
            'command_bus_only': True, 'undo_redo_reopen': True, 'authoring_unchanged_by_play': True})
        write(output / 'independent-oracle.json', EXPECTED)
        if args.stage == 'author':
            print(json.dumps({'authoring_passed': True, 'output': str(output)}))
            return
    authored = json.loads((output / 'authoring.json').read_text())
    require(digest(project) == authored['project_sha256'] and fingerprints(inputs) == authored['files']
            and digest(binary) == authored['binary_sha256'] and digest(BEHAVIOR) == authored['source_sha256'], 'Preflight input changed')
    if args.stage == 'export':
        commands.extend(json.loads((output / 'commands.json').read_text()))
    exported = export_native.export(project, output / 'package', camera=CAMERA)
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export identity')
    removed_maps = BASE['remove_source_maps'](output / 'package')
    shutil.rmtree(inputs)
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    environment = {'PATH': str(player.parent), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    environment.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    runs = []
    def launch(name, ticks, start=0, load=None, wrong=False, diagnostic=None):
        observe, pcm, save = [output / (name + ext) for ext in ('-observe.json', '.pcm', '.save')]
        write(observe, observations(start, start + ticks, wrong))
        command = [player, '--package', player.parent / 'data', '--ticks', ticks, '--output', pcm,
                   '--observe', observe, '--save-checkpoint', save]
        if load is not None: command.extend(['--load-checkpoint', load])
        result = recorded(command, cwd=player.parent, env=environment, timeout=60)
        if diagnostic is not None:
            require(result.returncode == 1 and diagnostic == result.stderr.strip() and not pcm.exists() and not save.exists(),
                    name + ': expected typed refusal: ' + result.stderr)
            return None
        require(result.returncode == 0, name + ': ' + result.stderr)
        report = json.loads(result.stdout)
        require(report['tick'] == start + ticks and report['observations']['passed'], 'Public observation result')
        require(report['render_input']['model_count'] == 2 and not report['render_input']['gpu_submitted'], 'Retained CPU models')
        check_logs(report['logs'], start, start + ticks)
        require(BASE['checkpoint_version'](save) == 6, 'Graph save version')
        write(output / (name + '.json'), report)
        runs.append({'name': name, 'start': start, 'ticks': ticks, 'report_sha256': digest(output / (name + '.json')),
                     'pcm_sha256': digest(pcm), 'save_sha256': digest(save)})
        return report
    baseline = launch('baseline', TICKS)
    require(baseline['logs'] == authored['logs'], 'Public Play/export observations differ')
    for n in range(2):
        repeat = launch(f'repeat-{n}', TICKS)
        require(repeat == baseline and (output / f'repeat-{n}.save').read_bytes() == (output / 'baseline.save').read_bytes(), 'Repeated process differs')
    for split, label in ((4, 'eight-to-four'), (9, 'paused'), (11, 'rest-completed-paused'), (21, 'reverse'), (25, 'second-state'), (34, 'zero-speed')):
        first = launch(label, split)
        resumed = launch(label + '-continued', TICKS - split, split, output / (label + '.save'))
        require((output / (label + '-continued.save')).read_bytes() == (output / 'baseline.save').read_bytes(), 'Resumed final save differs')
        require((output / (label + '.pcm')).read_bytes() + (output / (label + '-continued.pcm')).read_bytes() == (output / 'baseline.pcm').read_bytes(), 'Resumed PCM differs')
        require(first['logs']['entries'] + resumed['logs']['entries'] == baseline['logs']['entries'], 'Resumed observations differ')
        require(resumed['render_input'] == baseline['render_input'], 'Resumed render input differs')
    corrupt = output / 'corrupt-input.save'
    data = bytearray((output / 'paused.save').read_bytes()); data[-1] ^= 1; corrupt.write_bytes(data)
    launch('corrupt', 1, 9, corrupt, diagnostic='Error: Checkpoint(Integrity)')
    launch('wrong-observation', 1, wrong=True, diagnostic='Error: "checkpoint withheld: native observation assertions failed"')
    require(not list(output.glob('.incant-checkpoint-*')), 'Leaked checkpoint staging')
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'export': exported, 'runs': runs,
        'vector_assertions_per_full_run': len(OBSERVED) * len(CHECK_TICKS), 'full_pose_log_ticks': TICKS, 'fresh_full_processes': 3, 'continuation_pairs': 6,
        'authoring_deleted': not inputs.exists(), 'removed_source_maps': removed_maps,
        'package_files': fingerprints(output / 'package/player'), 'probe_sha256': digest(Path(__file__)),
        'behavior_sha256': digest(BEHAVIOR), 'commands_sha256': digest(output / 'commands.json'),
        'limits': ['CPU render-input and complete sample observations, not GPU pixels or device/performance gate',
                   'Tree-to-tree timed fades, IK, root motion and module checkpoint state remain separate unfinished work']} )
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
