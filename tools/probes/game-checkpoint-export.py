#!/usr/bin/env python3
"""Fresh public export; ordinary checkpoints in source/build-deleted CPU players.

All project/script mutations use public incant_cmd routes. The mathematical mesh
is test data; this probe neither submits GPU work nor judges rendered pixels.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
ENTITY = '00000000000000000000000002'
SCRIPT = '00000000000000000000000001'
CAMERA = '00000000000000000000000003'


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def transform(at):
    return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}


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
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'

    def execute(command, **kwargs):
        result = native_process.run(list(map(str, command)), **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    if args.author:
        data = struct.pack('<9f', 0, 0, 0, 1, 0, 0, 0, 1, 0)
        model = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}],
            'nodes': [{'mesh': 0}], 'meshes': [{'primitives': [{'attributes': {'POSITION': 0}}]}],
            'buffers': [{'byteLength': len(data), 'uri': 'data:application/octet-stream;base64,' + base64.b64encode(data).decode('ascii')}],
            'bufferViews': [{'buffer': 0, 'byteLength': len(data)}],
            'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 3, 'type': 'VEC3',
                           'min': [0, 0, 0], 'max': [1, 1, 0]}]}
        write(inputs / 'triangle.gltf', model)
        imported = json.loads(execute([binary, 'import', project, 'triangle.gltf']).stdout)
        document = json.loads(project.read_text(encoding='utf-8'))
        scene = next(iter(document['scenes']))
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        rpc = Rpc(binary, project, journal)
        source = (ROOT / 'tools/probes/native-session-checkpoint/src/behavior.ts').read_text(encoding='utf-8')
        source = source.replace('if (chunk.ids[row] === instance.entity) chunk.transforms[row * 10] = count + events;',
            "if (chunk.ids[row] === instance.entity) { chunk.transforms[row * 10] = count + events; chunk.transforms[row * 10 + 1] += t.input.keyboard.pressed.includes('Space') ? 1 : 0; }")
        try:
            revision = rpc.call('project.read')['revision']
            value = rpc.call('command.execute', {'expected_revision': revision,
                'description': 'Author ordinary checkpoint export proof', 'commands': [
                    {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                    {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Persistent mover', 'path': 'move.ts'}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': ENTITY, 'name': 'Mover', 'parent': None,
                        'provenance': None, 'components': {'Transform': transform([0, 0, 0]),
                            'Script': {'source': SCRIPT, 'props': {'speed': 2}},
                            'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': True}}}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': CAMERA, 'name': 'Camera', 'parent': None,
                        'provenance': None, 'components': {'Transform': transform([0, 0, 8]),
                            'Camera': {'fov_degrees': 60, 'near': .1, 'far': 100, 'projection': {'kind': 'perspective'}}}}}]})
            require(value['revision'] == revision + 1, 'Nonatomic authoring')
            authored = rpc.call('project.read')
            require(rpc.call('project.save')['saved'], 'Project checkpoint failed')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(rpc.call('project.read') == authored, 'Journal recovery mismatch')
        finally:
            rpc.close()
        write(output / 'authoring.json', {'project_sha256': digest(project), 'source_sha256': digest(inputs / 'move.ts'),
            'scene': scene, 'tick_rate': document['settings']['tick_rate'], 'revision': value['revision'], 'command_bus_only': True, 'journal_reopened': True})
        return

    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author'], timeout=180)
    authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
    report = export_native.export(project, output / 'package', camera=CAMERA)
    require(report['project_sha256'] == authored['project_sha256'], 'Different exported source')
    require(report['build_storage_removed_before_startup'], 'Export build storage survived')
    shutil.rmtree(inputs)  # Only the private fixture created above.
    require(not inputs.exists(), 'Authoring survived')
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})
    data_root = player.parent / 'data'
    runs = []

    def expected(tick):
        return tick * 2 + max(0, (tick - 1) // 3)

    def plan(name, ticks, wrong=False, y=0):
        path = output / (name + '-observe.json')
        write(path, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
            'checks': [{'name': 'persistent-state-timer-' + str(tick), 'tick': tick,
                'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                           'value': [expected(tick) + int(wrong), y, 0], 'tolerance': 0}} for tick in ticks]})
        return path

    def launch(name, ticks, load=None, save=None, observe=None, extra=(), success=True, y=0):
        pcm = output / (name + '.pcm')
        command = [player, '--package', data_root, '--ticks', str(ticks), '--output', pcm]
        if load is not None:
            command += ['--load-checkpoint', load]
        if save is not None:
            command += ['--save-checkpoint', save]
        if observe is not None:
            command += ['--observe', observe]
        result = native_process.run(list(map(str, [*command, *extra])), cwd=player.parent, env=env, timeout=60)
        require((result.returncode == 0) == success, name + ': ' + result.stderr[-4000:])
        write(output / (name + '-receipt.json'), {'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        if not success:
            return result
        value = json.loads(result.stdout)
        require(value['observations']['passed'], 'Failed observations')
        require(value['render_input']['first_model_position'] == [expected(value['tick']), y, 0], 'Retained CPU pose differs')
        require(not value['render_input']['gpu_submitted'], 'Unexpected GPU')
        runs.append({'name': name, 'report': value, 'save_sha256': digest(save), 'pcm_bytes': pcm.stat().st_size})
        return value

    baseline, mid, resume, repeat = [output / name for name in ('baseline.save', 'mid.save', 'resume.save', 'repeat.save')]
    launch('baseline', 40, save=baseline, observe=plan('baseline', [0, 13, 40]))
    launch('first', 13, save=mid, observe=plan('first', [13]))
    for name, save in [('resume', resume), ('repeat', repeat)]:
        launch(name, 27, load=mid, save=save, observe=plan(name, [13, 40]))
    require(baseline.read_bytes() == resume.read_bytes() == repeat.read_bytes(), 'Checkpoint continuation differs')
    require((output / 'baseline.pcm').read_bytes() == (output / 'first.pcm').read_bytes() + (output / 'resume.pcm').read_bytes(), 'PCM continuation differs')
    fresh_input = output / 'fresh-input.json'
    write(fresh_input, {'format': 'incant-input', 'version': 1, 'tick_rate': authored['tick_rate'],
        'start_tick': 0, 'ticks': 27, 'frames': [
            {'tick': 1, 'events': [{'type': 'key', 'code': 'Space', 'down': True}]},
            {'tick': 2, 'events': [{'type': 'key', 'code': 'Space', 'down': False}]}]})
    launch('fresh-input', 27, load=mid, save=output / 'fresh-input.save',
        observe=plan('fresh-input', [14, 40], y=1), extra=['--input-recording', fresh_input], y=1)
    refused = []
    def refusal(name, load=mid, save=None, observe=None, extra=(), output_absent=True):
        result = launch(name, 1, load=load, save=save, observe=observe, extra=extra, success=False)
        if output_absent:
            require(not (output / (name + '.pcm')).exists(), name + ': PCM published')
        if save is not None and name != 'no-clobber':
            require(not save.exists(), name + ': checkpoint published')
        refused.append({'name': name, 'stderr': result.stderr})
    corrupt = output / 'corrupt.save'
    content = bytearray(mid.read_bytes()); content[-1] ^= 1; corrupt.write_bytes(content)
    refusal('corrupt', load=corrupt)
    oversized = output / 'oversized.save'
    with oversized.open('wb') as file:
        file.truncate(256 * 1024 * 1024 + 1)
    refusal('oversized', load=oversized)
    mismatch = output / 'mismatched.save'
    content = bytearray(mid.read_bytes()); content[16] ^= 1
    content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
    mismatch.write_bytes(content)
    refusal('mismatched-program', load=mismatch)
    refusal('assertion', save=output / 'assertion.save', observe=plan('assertion', [14], wrong=True))
    refusal('past-observation', observe=plan('past', [12]))
    sentinel = output / 'existing.save'; sentinel.write_bytes(b'preserve existing save')
    refusal('no-clobber', save=sentinel, output_absent=False)
    require(sentinel.read_bytes() == b'preserve existing save', 'Clobbered existing save')
    require(not list(output.glob('.incant-checkpoint-*')), 'Leaked save staging file')
    for mode in ('--window', '--gpu-offscreen', '--static-module'):
        refusal('unsupported-' + mode[2:], extra=[mode])
    # Actual bytecode/catalog identity is read before any restored host or VM is published.
    require(any(token in next(r['stderr'] for r in refused if r['name'] == 'mismatched-program').lower()
                for token in ('incompatible', 'programmismatch')), 'Mismatch refused for an unrelated reason')
    result = {'passed': True, 'authoring': authored, 'export': report, 'runs': runs, 'refusals': refused,
        'authoring_deleted': True, 'build_storage_removed': True, 'cpu_retained_host': True,
        'player_sha256': digest(player), 'final_save_sha256': digest(baseline),
        'limitations': ['Ordinary neutral-input load; ephemeral module globals/closures',
            'No physics/audio/static checkpoints', 'No GPU/window/device/performance or full-save completion claim',
            'PCM and checkpoint are not a multifile atomic transaction']}
    write(output / 'result.json', result)
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
