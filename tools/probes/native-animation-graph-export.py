#!/usr/bin/env python3
"""Public authored native graphs in fresh source/build-deleted QuickJS players.

CPU render input and accepted SDK observations only; no GPU, window, provider,
Wasm shipping selection, device performance or appearance acceptance.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
SCRIPT, FIRST, CAMERA, SECOND, COMPLETED = [f'{n:026}' for n in (1, 2, 3, 4, 5)]
ENTITIES = (FIRST, SECOND, COMPLETED)
TICKS = 90


def require(condition, message):
    if not condition:
        raise ValueError(message)


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fingerprints(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def remove_source_maps(package):
    """Remove only source-bearing debug maps from this probe's new package."""
    removed = {}
    for path in sorted((package / 'symbols').glob('*.js.map')):
        require(path.is_file() and not path.is_symlink(), 'Unexpected source-map artifact')
        removed[str(path.relative_to(package))] = digest(path)
        path.unlink()
    return removed


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


def graph(completed=False):
    p, a, b, c, x, y = [f'{n:026}' for n in (10, 20, 21, 22, 30, 31)]
    return {'initial': a, 'parameters': {p: {'initial': 0, 'min': -1, 'max': 2}},
        'states': {a: {'motion': {'kind': 'clip', 'clip': 0}, 'looping': not completed},
                   b: {'motion': {'kind': 'clip', 'clip': 1}, 'looping': True},
                   c: {'motion': {'kind': 'blend1d', 'parameter': p, 'cycle_seconds': 2,
                       'points': [{'threshold': 0, 'clip': 0}, {'threshold': 1, 'clip': 1}]}, 'looping': True}},
        'transitions': {x: {'from': a, 'to': b, 'priority': 0, 'fade_seconds': .5, 'after_seconds': None,
            'conditions': [{'parameter': p, 'comparison': 'greater_equal', 'value': .5}]},
            y: {'from': b, 'to': c, 'priority': 0, 'fade_seconds': 0, 'after_seconds': .5,
            'conditions': [{'parameter': p, 'comparison': 'greater_equal', 'value': .5}]}}}


def cases():
    """Independent fixed-step counts around transition, pause, blend and control boundaries."""
    def row(state, steps, elapsed, parameter=.5, flags=3, source=0, weight=0, clip=None):
        return [state, steps/60, elapsed/60, parameter, (0 if state == 0 else 1) if clip is None else clip,
                steps/60, flags, source/60, weight]
    r = row
    result = {
        1: [r(0,1,1,0),r(0,0,0,0,2)],
        2: [r(1,0,0,source=2),r(0,0,0,flags=2)],
        3: [r(1,1,1,source=3,weight=1/30),r(0,0,0,flags=2)],
        4: [r(1,2,2,source=4,weight=2/30),r(1,0,0,source=1)],
        6: [r(1,4,4,source=6,weight=4/30),r(1,2,2,source=3,weight=2/30)],
        7: [r(1,4,4,flags=2,source=6,weight=4/30),r(1,3,3,source=4,weight=3/30)],
        9: [r(1,4,4,flags=2,source=6,weight=4/30),r(1,5,5,source=6,weight=5/30)],
        10:[r(1,5,5,source=7,weight=5/30),r(1,6,6,source=7,weight=6/30)],
        34:[r(1,29,29,source=31,weight=29/30),r(1,30,30)],
        35:[r(1,30,30),r(2,0,0,weight=.5)],
        36:[r(2,0,0,weight=.5),r(2,1,1,source=1,weight=.5)],
        40:[r(2,4,4,source=4,weight=.5),r(2,5,5,source=5,weight=.5)],
        41:[r(2,5,5,.25,source=5,weight=.25),r(2,6,6,.75,source=6,weight=.75)],
        46:[r(2,8,10,.25,source=8,weight=.25),r(2,12,11,.75,source=12,weight=.75)],
        51:[r(2,4,14,.25,2,4,.25),r(2,22,16,.75,source=22,weight=.75)],
        54:[r(2,3,15,.25,source=3,weight=.25),r(2,28,19,.75,source=28,weight=.75)],
        61:[r(2,119,1,.25,source=119,weight=.25),r(2,2,1,.75,source=2,weight=.75)],
        71:[r(2,109,11,-1,clip=0),r(2,22,11,2)],
        76:[r(0,119,1,0),r(0,2,1,0)],
        90:[r(0,105,15,0),r(0,30,15,0)],
    }

    for tick, rows in result.items():
        if tick <= 7: third = r(0,16*tick,tick,0,1)
        elif tick <= 12: third = r(0,120,min(tick,9),0 if tick <= 10 else .5,4)
        elif tick <= 43:
            steps = tick-13
            third = r(1,steps,steps,source=120 if tick < 43 else 0,weight=steps/30 if tick < 43 else 0)
        elif tick <= 60:
            steps = tick-44
            p = .5 if tick <= 50 else .25
            third = r(2,steps,steps,p,source=steps,weight=p)
        else:
            steps = tick-60
            p = .25 if tick <= 70 else .75
            third = r(2,steps,steps,p,source=steps,weight=p)
        rows.append(third)
    return result


def observations(start, end, wrong=False):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': list(ENTITIES),
        'checks': [{'name': f'{entity}-{tick}', 'tick': tick,
            'expect': {'type': 'vector', 'entity': entity, 'property': 'translation',
                       'value': [row[0] + row[3] + int(wrong), row[1], tick + row[8]], 'tolerance': 1e-8}}
            for tick, rows in cases().items() if start < tick <= end
            for entity, row in zip(ENTITIES, rows)]}


def check_logs(logs, start, end):
    entries = logs['entries']
    require(len(entries) == len(ENTITIES) * (end - start) and logs['dropped'] == 0, 'Committed log prefix/count')
    require(sum(len(e['message'].encode()) for e in entries) <= 256 * 1024, 'History byte quota')
    expected = cases()
    for offset in range(0, len(entries), len(ENTITIES)):
        tick = start + offset // len(ENTITIES) + 1
        seen = set()
        for entry in entries[offset:offset + len(ENTITIES)]:
            row = json.loads(entry['message'])
            require(entry['tick'] == tick == row[0] and entry['level'] == 'info', 'Accepted tick pairing')
            require(len(row) == 11 and row[1] in ENTITIES and row[1] not in seen, 'Copied row identity')
            seen.add(row[1])
            values = row[2:]
            require(all(type(v) in (int, float) and math.isfinite(v) for v in values), 'Nonfinite snapshot')
            if tick in expected:
                want = expected[tick][ENTITIES.index(row[1])]
                require(all(abs(a - b) < 1e-8 for a, b in zip(values, want)),
                        f'{row[1]}@{tick}: {values} != {want}')
        require(seen == set(ENTITIES), 'Instance rows missing')


def checkpoint_version(path):
    """Inspect documented session framing; gameplay still loads through its real parser."""
    data = path.read_bytes()
    require(data[:8] == b'INCSES01' and len(data) >= 128, 'Session checkpoint framing')
    lengths = struct.unpack_from('<4I', data, 80)
    require(len(data) == 128 + sum(lengths), 'Session section lengths')
    require(data[96:128] == hashlib.sha256(data[:96] + data[128:]).digest(), 'Session integrity')
    world = data[128:128 + lengths[0]]
    require(len(world) >= 12, 'World section missing')
    return struct.unpack_from('<I', world, 8)[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    import export_native
    inputs = output / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'project.journal.jsonl'

    def execute(command, **kwargs):
        result = native_process.run(list(map(str, command)), **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    if args.author:
        fixture(inputs)
        imported = json.loads(execute([binary, 'import', project, 'rig.gltf']).stdout)
        document = json.loads(project.read_text())
        scene = next(iter(document['scenes']))
        require(document['settings']['tick_rate'] == 60, 'Public default tick rate')
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        source = (ROOT / 'tools/probes/native-animation-graph-export/behavior.ts').read_text()
        rpc = Rpc(binary, project, journal)
        calls = []

        def call(method, params=None):
            calls.append({'method': method, 'params': params})
            return rpc.call(method, params) if params is not None else rpc.call(method)

        try:
            revision = call('project.read')['revision']
            commands = [
                {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Native animation graph', 'path': 'graph.ts'}}]
            for entity in ENTITIES:
                commands.append({'op': 'create_entity', 'scene_id': scene,
                    'entity': {'id': entity, 'name': 'Running source' if entity == FIRST else 'Paused source',
                        'parent': None, 'provenance': None, 'components': {'Transform': transform([0, 0, 0]),
                            'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': False},
                            'Animation': {'clip': 0, 'seconds': 0, 'speed': 16 if entity == COMPLETED else 1, 'playing': entity != SECOND, 'looping': entity != COMPLETED},
                            'AnimationGraph': graph(entity == COMPLETED),
                            'Script': {'source': SCRIPT, 'props': {'first': entity == FIRST, 'completed': entity == COMPLETED}}}}})
            commands.append({'op': 'create_entity', 'scene_id': scene,
                'entity': {'id': CAMERA, 'name': 'Stationary camera', 'parent': None, 'provenance': None,
                    'components': {'Transform': transform([0, 0, 8]), 'Camera': {'fov_degrees': 60,
                        'near': .1, 'far': 200, 'projection': {'kind': 'perspective'}}}}})
            created = call('command.execute', {'expected_revision': revision,
                'description': 'Author public animation graph export proof', 'commands': commands})
            require(created['revision'] == revision + 1, 'Atomic creation')
            call('history.undo')
            require(call('project.read')['project'] == document and not (inputs / 'graph.ts').exists(), 'Undo')
            call('history.redo')
            authored = call('project.read')
            played = call('play.run', {'seconds': TICKS / 60, 'camera': CAMERA, 'observe': observations(0, TICKS)})
            write(output / 'native-play.json', played)
            require(played['passed'] and not played['state']['gpu_submitted']
                    and played['state']['cpu_render_input'], 'CPU NativePlay')
            check_logs(played['state']['logs'], 0, TICKS)
            require(call('project.read') == authored, 'Play mutated authoring')
            require(call('project.save')['saved'], 'Save failed')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call('project.read') == authored, 'Editor-default journal path reopened through public RPC with --journal')
            require(call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source, 'Source reopen')
        finally:
            rpc.close()
        write(output / 'public-commands.json', calls)
        write(output / 'authoring.json', {'command_bus_only': True, 'undo_redo_save_default_journal_reopen': True,
            'ids': {'script': SCRIPT, 'entities': ENTITIES, 'camera': CAMERA, 'scene': scene, 'asset': imported['asset']['id']},
            'imported_asset': imported['asset'], 'project_sha256': digest(project), 'binary_sha256': digest(binary),
            'files': fingerprints(inputs), 'native_play_logs': played['state']['logs']})
        return

    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author'], timeout=180)
    authored = json.loads((output / 'authoring.json').read_text())
    exported = export_native.export(project, output / 'package', camera=CAMERA)
    require(exported['project_sha256'] == authored['project_sha256']
            and exported['build_storage_removed_before_startup'], 'Export source/build identity')
    # The ordinary exporter retains developer source maps outside player/. This
    # newly-created probe package needs no source-bearing crash artifacts during
    # its subsequent source-deleted runs. Preserve their identities, then remove
    # only those artifacts; never change the exporter or an earlier proof output.
    removed_maps = remove_source_maps(output / 'package')
    shutil.rmtree(inputs)  # Only this probe's newly created private authoring fixture.
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    environment = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
                   'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    environment.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    runs = []

    def launch(name, ticks, start=0, load=None, save=None, success=True, wrong=False):
        plan, pcm = output / (name + '-observe.json'), output / (name + '.pcm')
        write(plan, observations(start, start + ticks, wrong))
        command = [player, '--package', player.parent / 'data', '--ticks', ticks, '--output', pcm, '--observe', plan]
        if load is not None:
            command.extend(['--load-checkpoint', load])
        if save is not None:
            command.extend(['--save-checkpoint', save])
        result = native_process.run(list(map(str, command)), cwd=player.parent, env=environment, timeout=60)
        write(output / (name + '-receipt.json'), {'argv': list(map(str, command)), 'returncode': result.returncode,
            'stdout': result.stdout, 'stderr': result.stderr})
        require((result.returncode == 0) == success, name + ': ' + result.stderr[-4000:])
        if not success:
            require(not pcm.exists() and (save is None or not save.exists()), 'Rejected output published')
            return None
        value = json.loads(result.stdout)
        require(value['tick'] == start + ticks and value['observations']['passed'], 'Tick/observations')
        require(value['render_input']['model_count'] == len(ENTITIES) and not value['render_input']['gpu_submitted'], 'CPU model host')
        check_logs(value['logs'], start, start + ticks)
        write(output / (name + '.json'), value)
        runs.append({'name': name, 'ticks': ticks, 'start': start, 'report_sha256': digest(output / (name + '.json')),
            'pcm_sha256': digest(pcm), 'save_sha256': digest(save), 'world_save_version': checkpoint_version(save)})
        return value

    baseline = output / 'baseline.save'
    full = [launch('baseline', TICKS, save=baseline)]
    for n in range(2):
        save = output / f'repeat-{n}.save'
        full.append(launch(f'repeat-{n}', TICKS, save=save))
        require(save.read_bytes() == baseline.read_bytes(), 'Fresh save bytes differ')
    require(full[0] == full[1] == full[2], 'Fresh reports differ')
    require(full[0]['logs'] == authored['native_play_logs'], 'NativePlay and exported SDK observations differ')
    for split, label in [(10, 'active'), (7, 'paused'), (11, 'completed-paused'), (46, 'blended')]:
        mid, final = output / (label + '.save'), output / (label + '-continued.save')
        first = launch(label + '-first', split, save=mid)
        require(checkpoint_version(mid) == 6, label + ': graph state not saved as v6')
        resumed = launch(label + '-continued', TICKS - split, start=split, load=mid, save=final)
        require(final.read_bytes() == baseline.read_bytes(), label + ': continuation save differs')
        require((output / 'baseline.pcm').read_bytes() == (output / (label + '-first.pcm')).read_bytes()
                + (output / (label + '-continued.pcm')).read_bytes(), label + ': PCM continuation differs')
        require(first['logs']['entries'] + resumed['logs']['entries'] == full[0]['logs']['entries'], 'Log continuation differs')
        require(resumed['render_input'] == full[0]['render_input'], 'Final retained CPU input differs')
    corrupt = output / 'corrupt.save'
    data = bytearray((output / 'active.save').read_bytes()); data[-1] ^= 1; corrupt.write_bytes(data)
    launch('corrupt', 1, start=10, load=corrupt, save=output / 'corrupt-output.save', success=False)
    launch('wrong-observation', 1, save=output / 'wrong-output.save', success=False, wrong=True)
    require(not list(output.glob('.incant-checkpoint-*')), 'Leaked save staging')
    write(output / 'result.json', {'passed': True, 'implementation': 'QuickJS compiled project bytecode',
        'public_authoring': authored, 'export': exported, 'fresh_full_runs': 3, 'ticks_each': TICKS,
        'independent_snapshot_cases': len(cases()) * len(ENTITIES), 'runs': runs, 'checkpoint_continuations': ['active fade v6', 'paused fade v6', 'completed paused v6', 'blend state v6'],
        'authoring_deleted': not inputs.exists(), 'private_export_build_deleted': exported['build_storage_removed_before_startup'],
        'removed_external_source_maps': removed_maps,
        'package_files': fingerprints(output / 'package/player'), 'probe_source_sha256': digest(Path(__file__)),
        'behavior_source_sha256': digest(ROOT / 'tools/probes/native-animation-graph-export/behavior.ts'),
        'refusal_controls': ['corrupt checkpoint: no PCM/save', 'wrong observation: no PCM/save'],
        'limitations': ['Local CPU render input and SDK observations, not deformed-vertex readback/GPU/window/appearance/device/performance acceptance',
            'Graph controls currently use the public TypeScript path; graph Rust/Wasm ABI controls are not claimed',
            'Existing combined SDK tests cover Wasmtime/static wasm2c separately; full static module-state checkpoints remain Unsupported',
            'Ordinary checkpoints retain explicit script state; ephemeral VM globals/closures are not persisted',
            'No phase gate; flat 1D adjacent-pair blend only, blended-state fades/2D/nested trees/IK/root motion remain unsupported']})
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
