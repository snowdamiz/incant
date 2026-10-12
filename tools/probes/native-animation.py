#!/usr/bin/env python3
"""Public skeletal animation authoring and source/build-free CPU shipping replay.

An optional separately authored review fixture retains its own source. The export
proof deletes only its private authoring input; no GPU/device/appearance claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
SCRIPT, ENTITY, CAMERA, TABLE = [f'{i:026}' for i in (1, 2, 3, 5)]
TICKS = 160
# Independent numerical checkpoints: (clip, committed seconds, speed, flags)
# observed after the native fixed step, before that callback's queued controls.
CASES = {
    1: (0, 1 / 60, 1, 1), 2: (0, 1 / 60, 1, 0), 3: (1, .5, 2, 2),
    4: (1, .5 + 1 / 30, 2, 3), 7: (1, 1.75 + 1 / 30, 2, 3),
    13: (1, 1.75 + 6 / 30 - 1 / 60, -1, 3),
    19: (1, .02 - 1 / 60, -1, 1), 20: (1, 0, -1, 4),
    23: (-1, 0, -1, 0), 25: (-1, 0, -1, 1),
    27: (0, .05 + 1 / 60, 1, 1), 32: (0, 2, 1, 4),
    36: (0, 1 / 60, 1, 3), 41: (0, .25 - .75 / 60, -.75, 3),
    66: (1, 1, -.75, 2), 71: (1, 1 + 1 / 60, 1, 1),
    133: (1, 2, 1, 4), 136: (-1, 0, 1, 0),
    138: (0, 1.9 + 1 / 60, 1, 1), 145: (0, 2, 1, 4),
    150: (0, .5 - .5 / 60, -.5, 3),
    158: (0, .5 - 8 * .5 / 60, -.5, 2),
    160: (0, .5 - 8 * .5 / 60, -.5, 2),
}


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def transform(at):
    return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


def fixture(root):
    """One joint, a weighted triangle and real two-second move/rotate clips."""
    data, views, accessors = bytearray(), [], []

    def add(values, code, component, kind, count):
        while len(data) % 4:
            data.append(0)
        payload = struct.pack('<' + code * len(values), *values)
        index = len(accessors)
        views.append({'buffer': 0, 'byteOffset': len(data), 'byteLength': len(payload)})
        data.extend(payload)
        accessors.append({'bufferView': index, 'componentType': component, 'type': kind, 'count': count})
        return index

    position = add([0, 0, 0, 1, 0, 0, 0, 1, 0], 'f', 5126, 'VEC3', 3)
    normals = add([0, 0, 1] * 3, 'f', 5126, 'VEC3', 3)
    joints = add([0, 0, 0, 0] * 3, 'H', 5123, 'VEC4', 3)
    weights = add([1, 0, 0, 0] * 3, 'f', 5126, 'VEC4', 3)
    bind = add([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1], 'f', 5126, 'MAT4', 1)
    time = add([0, 2], 'f', 5126, 'SCALAR', 2)
    move = add([0, 0, 0, 2, 0, 0], 'f', 5126, 'VEC3', 2)
    rotate = add([0, 0, 0, 1, 0, 0, 1, 0], 'f', 5126, 'VEC4', 2)
    accessors[position].update(min=[0, 0, 0], max=[1, 1, 0])
    accessors[time].update(min=[0], max=[2])
    clips = [{'name': name, 'samplers': [{'input': time, 'output': output, 'interpolation': 'LINEAR'}],
              'channels': [{'sampler': 0, 'target': {'node': 0, 'path': path}}]}
             for name, output, path in [('move', move, 'translation'), ('rotate', rotate, 'rotation')]]
    model = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0, 1]}],
             'nodes': [{}, {'mesh': 0, 'skin': 0}],
             'meshes': [{'primitives': [{'attributes': {'POSITION': position, 'NORMAL': normals,
                                                       'JOINTS_0': joints, 'WEIGHTS_0': weights}}]}],
             'skins': [{'joints': [0], 'skeleton': 0, 'inverseBindMatrices': bind}], 'animations': clips,
             'buffers': [{'uri': 'rig.bin', 'byteLength': len(data)}],
             'bufferViews': views, 'accessors': accessors}
    (root / 'rig.bin').write_bytes(data)
    write(root / 'rig.gltf', model)


def check_logs(logs, start, end):
    entries = logs['entries']
    require(len(entries) == end - start + 1 and logs['dropped'] == 0, 'Unexpected committed log history')
    for entry, tick in zip(entries, range(start, end + 1)):
        fields = entry['message'].split(':')
        require(len(fields) == 5 and entry['level'] == 'info', 'Malformed committed animation log')
        require(int(fields[0]) == entry['tick'] == tick, 'Wrong script/log clock')
        if tick in CASES:
            actual, expected = list(map(float, fields[1:])), CASES[tick]
            require(all(abs(a - b) <= 1e-8 for a, b in zip(actual, expected))
                    and len(actual) == 4, f'Animation state at {tick}: {actual} != {expected}')


def observation(ticks, wrong=False):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
            'checks': [{'name': 'animation-' + str(tick), 'tick': tick,
                        'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                                   'value': [CASES[tick][1] + int(wrong), CASES[tick][3],
                                             tick + (tick - 1) // 3], 'tolerance': 1e-8}}
                       for tick in ticks]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--review-fixture', action='store_true',
                        help='Independently author a retained restarting move/rotate project; do not export/delete it')
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import export_native
    import native_process
    inputs = output if args.review_fixture else output / 'authoring'
    project = inputs / 'project.json'
    # Import, RPC, and the real editor must replay the same default journal.
    journal = project.with_suffix('.journal.jsonl')

    def execute(command, **kwargs):
        result = native_process.run(list(map(str, command)), **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    if args.author:
        fixture(inputs)
        imported = json.loads(execute([binary, 'import', project, 'rig.gltf']).stdout)
        document = json.loads(project.read_text(encoding='utf-8'))
        scene, rate = next(iter(document['scenes'])), document['settings']['tick_rate']
        require(rate == 60, 'Numerical fixture requires the public default 60Hz')
        filename = 'review.ts' if args.review_fixture else 'behavior.ts'
        source = (ROOT / 'tools/probes/native-animation' / filename).read_text(encoding='utf-8')
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        rpc = Rpc(binary, project, journal)
        try:
            revision = rpc.call('project.read')['revision']
            created = rpc.call('command.execute', {'expected_revision': revision,
                'description': 'Author native skeletal animation proof', 'commands': [
                    {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                    {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Animation controls', 'path': 'animation.ts'}},
                    {'op': 'upsert_string_table', 'table': {'id': TABLE, 'name': 'Probe', 'source_locale': 'en',
                        'messages': {'ready': {'en': 'Ready', 'fr': 'Prêt'}}}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': ENTITY, 'name': 'Numerical skeletal rig',
                        'parent': None, 'provenance': None, 'components': {'Transform': transform([0, 0, 0]),
                            'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': False},
                            'Animation': {'clip': 0, 'seconds': 0, 'speed': 1, 'playing': True, 'looping': False},
                            'Script': {'source': SCRIPT, 'props': {}}}}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': CAMERA, 'name': 'Stationary camera',
                        'parent': None, 'provenance': None, 'components': {'Transform': transform([.5, .5, 8]),
                            'Camera': {'fov_degrees': 60, 'near': .1, 'far': 200, 'projection': {'kind': 'perspective'}}}}}]})
            require(created['revision'] == revision + 1, 'Creation was not one transaction')
            rpc.call('history.undo')
            require(rpc.call('project.read')['project'] == document, 'Undo did not restore imported project')
            require(not (inputs / 'animation.ts').exists(), 'Undo retained created source')
            rpc.call('history.redo')
            authored = rpc.call('project.read')
            request = {'seconds': (180 if args.review_fixture else TICKS) / rate, 'camera': CAMERA}
            if not args.review_fixture:
                request['observe'] = observation(CASES)
            played = rpc.call('play.run', request)
            write(output / 'native-play-result.json', played)
            require(played['passed'] and not played['state']['gpu_submitted']
                    and played['state']['cpu_render_input'], 'Native CPU animation Play failed')
            if not args.review_fixture:
                check_logs(played['state']['logs'], 1, TICKS)
            value = rpc.call('play.state_query', {'path': f'/entities/{ENTITY}/transform/translation'})
            require(rpc.call('project.read') == authored, 'Play modified authoring')
            require(rpc.call('project.save')['saved'], 'Project save failed')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(rpc.call('project.read') == authored, 'Saved journal recovery differs')
            require(rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source,
                    'Recovered source differs')
        finally:
            rpc.close()
        write(output / 'authoring-result.json', {'command_bus_only': True, 'undo_redo_and_reopen': True,
            'authoring_unchanged_by_play': True, 'review_fixture_retained': args.review_fixture,
            'ids': {'script': SCRIPT, 'entity': ENTITY, 'camera': CAMERA, 'table': TABLE,
                    'scene': scene, 'asset': imported['asset']['id']}, 'tick_rate': rate,
            'project_sha256': digest(project), 'journal_sha256': digest(journal), 'source_sha256': digest(inputs / 'animation.ts'),
            'gltf_sha256': digest(inputs / 'rig.gltf'), 'buffer_sha256': digest(inputs / 'rig.bin'),
            'binary_sha256': digest(binary), 'native_play': played, 'queried_translation': value['untrusted_runtime_data']})
        return

    output.mkdir(parents=True, exist_ok=False)
    if not args.review_fixture:
        inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    author_command = [sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author']
    if args.review_fixture:
        author_command.append('--review-fixture')
    execute(author_command, timeout=180)
    authored = json.loads((output / 'authoring-result.json').read_text(encoding='utf-8'))
    if args.review_fixture:
        print(json.dumps({'project': str(project), 'project_sha256': authored['project_sha256'],
                          'retained_for_native_review': True}))
        return
    exported = export_native.export(project, output / 'package', camera=CAMERA)
    require(exported['project_sha256'] == authored['project_sha256']
            and exported['build_storage_removed_before_startup'], 'Export source/build receipt mismatch')
    shutil.rmtree(inputs)  # Only this new private proof fixture, after packaging.
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})

    def launch(name, ticks, start=0, load=None, save=None, expected_success=True, wrong=False):
        plan = output / (name + '-observe.json')
        write(plan, observation([n for n in CASES if start < n <= start + ticks], wrong))
        command = [player, '--package', player.parent / 'data', '--ticks', str(ticks),
                   '--output', output / (name + '.pcm'), '--observe', plan]
        if load:
            command += ['--load-checkpoint', load]
        if save:
            command += ['--save-checkpoint', save]
        result = native_process.run(list(map(str, command)), cwd=player.parent, env=env, timeout=60)
        write(output / (name + '-receipt.json'), {'returncode': result.returncode,
                                                'stdout': result.stdout, 'stderr': result.stderr})
        require((result.returncode == 0) == expected_success, name + ': ' + result.stderr[-4000:])
        if not expected_success:
            require(not (output / (name + '.pcm')).exists() and not save.exists(), 'Rejected output published')
            return None
        value = json.loads(result.stdout)
        require(value['tick'] == start + ticks and value['observations']['passed'], name + ': clock/observations')
        require(value['render_input']['model_count'] == 1 and not value['render_input']['gpu_submitted'],
                'No actual CPU model input or unexpected GPU work')
        check_logs(value['logs'], start + 1, start + ticks)
        write(output / (name + '.json'), value)
        return value

    baseline = output / 'baseline.save'
    runs = [launch('baseline', TICKS, save=baseline)]
    for n in range(2):
        save = output / f'repeat-{n}.save'
        runs.append(launch(f'repeat-{n}', TICKS, save=save))
        require(save.read_bytes() == baseline.read_bytes(), 'Fresh checkpoint bytes differ')
    require(runs[0] == runs[1] == runs[2], 'Fresh source-free reports differ')
    mid = output / 'mid.save'
    first = launch('first', 41, save=mid)
    continued = output / 'continued.save'
    resumed = launch('continued', TICKS - 41, start=41, load=mid, save=continued)
    require(continued.read_bytes() == baseline.read_bytes(), 'Animation/session continuation differs')
    require((output / 'baseline.pcm').read_bytes() == (output / 'first.pcm').read_bytes()
            + (output / 'continued.pcm').read_bytes(), 'PCM continuation differs')
    require(first['logs']['entries'] + resumed['logs']['entries'] == runs[0]['logs']['entries'],
            'Committed log continuation differs')
    require(resumed['render_input'] == runs[0]['render_input'], 'Retained CPU model continuation differs')
    corrupt = output / 'corrupt.save'
    data = bytearray(mid.read_bytes())
    data[-1] ^= 1
    corrupt.write_bytes(data)
    launch('corrupt', 1, start=41, load=corrupt, save=output / 'corrupt-output.save', expected_success=False)
    launch('wrong-observation', 2, save=output / 'wrong-output.save', expected_success=False, wrong=True)
    write(output / 'result.json', {'passed': True, 'public_authoring': authored,
        'fresh_source_free_runs': len(runs), 'ticks': TICKS, 'numeric_cases': len(CASES),
        'checkpoint_continuation': True, 'checkpoint_sha256': digest(baseline), 'pcm_sha256': runs[0]['pcm_sha256'],
        'source_deleted': not inputs.exists(), 'private_build_deleted': exported['build_storage_removed_before_startup'],
        'player_sha256': exported['player_sha256'], 'target': exported['target'],
        'refusal_controls': ['corrupt checkpoint', 'wrong observation with withheld save/PCM'],
        'limitations': ['Local CPU functional proof; no GPU/window/device/performance/visual or phase gate claim']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
