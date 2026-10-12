#!/usr/bin/env python3
"""Public mixed text/model/script/audio export; source-free CPU atlas observations.

This exercises the ordinary existing package/player route. It never creates a
GPU device or evaluates appearance, widget layout, IME or mobile performance.
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
import wave

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
        fonts = []
        for name in ('NotoSans-Regular.ttf', 'NotoSansArabic.ttf', 'IncantCJKFixture.ttf'):
            shutil.copyfile(ROOT / 'crates/incant_text/tests/fonts' / name, inputs / name)
            fonts.append(json.loads(execute([binary, 'import', project, name]).stdout)['asset']['id'])
        data = struct.pack('<9f', 0, 0, 0, 1, 0, 0, 0, 1, 0)
        model = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}],
            'nodes': [{'mesh': 0}], 'meshes': [{'primitives': [{'attributes': {'POSITION': 0}}]}],
            'buffers': [{'byteLength': len(data), 'uri': 'data:application/octet-stream;base64,' + base64.b64encode(data).decode('ascii')}],
            'bufferViews': [{'buffer': 0, 'byteLength': len(data)}],
            'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 3, 'type': 'VEC3',
                           'min': [0, 0, 0], 'max': [1, 1, 0]}]}
        write(inputs / 'triangle.gltf', model)
        mesh = json.loads(execute([binary, 'import', project, 'triangle.gltf']).stdout)['asset']['id']
        with wave.open(str(inputs / 'tone.wav'), 'wb') as audio:
            audio.setnchannels(1)
            audio.setsampwidth(2)
            audio.setframerate(48000)
            audio.writeframes(b''.join(struct.pack('<h', 6000 if i % 100 < 50 else -6000) for i in range(48000)))
        clip = json.loads(execute([binary, 'import', project, 'tone.wav']).stdout)['asset']['id']
        document = json.loads(project.read_text(encoding='utf-8'))
        scene = next(iter(document['scenes']))
        module = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))
        Rpc, source = module['Rpc'], module['source'](6)
        rpc = Rpc(binary, project, journal)
        try:
            revision = rpc.call('project.read')['revision']
            result = rpc.call('command.execute', {'expected_revision': revision,
                'description': 'Author mixed retained text game', 'commands': [
                    {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
                    {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Mover', 'path': 'move.ts'}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': ENTITY, 'name': 'Mixed', 'parent': None,
                        'provenance': None, 'components': {'Transform': transform([0, 0, 0]),
                            'Script': {'source': SCRIPT, 'props': {}},
                            'MeshRenderer': {'mesh': mesh, 'materials': [], 'cast_shadows': True},
                            'Text': {'content': 'ffi سلام 中文', 'fonts': fonts, 'font_size': 20,
                                     'line_height': 28, 'width': 500, 'wrap': 'word_or_glyph'},
                            'AudioSource': {'clip': clip, 'gain_db': 0, 'pan': 0, 'rate': 1,
                                'start_seconds': 0, 'playing': True, 'looping': True, 'streaming': False}}}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': CAMERA, 'name': 'Camera', 'parent': None,
                        'provenance': None, 'components': {'Transform': transform([0, 0, 8]),
                            'Camera': {'fov_degrees': 60, 'near': .1, 'far': 100, 'projection': {'kind': 'perspective'}}}}}]})
            require(result['revision'] == revision + 1, 'Nonatomic authoring')
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
            'scene': scene, 'tick_rate': document['settings']['tick_rate'], 'fonts': fonts,
            'revision': result['revision'], 'command_bus_only': True, 'journal_reopened': True})
        return

    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author'], timeout=180)
    authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
    exported = export_native.export(project, output / 'package', camera=CAMERA)
    require(exported['project_sha256'] == authored['project_sha256'], 'Different exported source')
    require(exported['build_storage_removed_before_startup'], 'Export build storage survived')
    shutil.rmtree(inputs)  # Only the newly created disposable authoring fixture.
    require(not inputs.exists(), 'Authoring survived')
    # Debug maps may embed original TypeScript; remove proof-owned sidecars before
    # claiming that the copied player ran without source text anywhere in its package.
    removed_maps = []
    for source_map in sorted((output / 'package/symbols').glob('*.map')):
        removed_maps.append({'path': str(source_map.relative_to(output / 'package')),
                             'sha256': digest(source_map)})
        source_map.unlink()
    require(not list((output / 'package').rglob('*.map')), 'Packaged source map survived')
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    data_root = player.parent / 'data'
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})
    expected = 6 * 20 / authored['tick_rate']
    plan = output / 'observe.json'
    write(plan, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
        'checks': [{'name': 'same-native-motion', 'tick': 20, 'expect': {'type': 'vector', 'entity': ENTITY,
            'property': 'translation', 'value': [expected, 0, 0], 'tolerance': 1e-9}}]})
    runs = []
    for index in range(3):
        pcm = output / f'run-{index}.pcm'
        value = json.loads(execute([player, '--package', data_root, '--ticks', '20', '--output', pcm,
                                   '--observe', plan], cwd=player.parent, env=env, timeout=60).stdout)
        text = value['render_input']
        require(value['observations']['passed'] and text['tick'] == 20, 'Wrong committed native tick')
        require(text['text_count'] == 1 and text['text_glyph_count'] > 5 and text['text_font_count'] == 3,
                'Missing actual multilingual shaping')
        require(text['text_missing_clusters'] == 0 and text['text_rtl_glyphs'] > 0, 'Missing/incorrect bidi shaping')
        require(text['text_atlas_nonzero'] > 0 and text['text_atlas_bytes'] <= 4 * 1024**2, 'Missing/unbounded atlas')
        require(not text['gpu_submitted'], 'Unexpected GPU submission')
        require(text['first_text_position'] == text['first_model_position'], 'Text/model pose divergence')
        require(abs(text['first_text_position'][0] - expected) < 1e-9, 'Wrong retained text pose')
        samples = struct.unpack('<' + 'f' * (pcm.stat().st_size // 4), pcm.read_bytes())
        require(any(sample != 0 for sample in samples), 'Silent mixed audio')
        write(output / f'run-{index}.json', value)
        runs.append({'report': value, 'pcm_sha256': digest(pcm)})
    require(len({run['report']['render_input']['text_atlas_sha256'] for run in runs}) == 1, 'Fresh atlas mismatch')
    require(len({run['pcm_sha256'] for run in runs}) == 1, 'Fresh PCM mismatch')
    blob = next((data_root / 'fonts').glob('*.ifont'))
    original = blob.read_bytes()
    rejected = []
    for mode in ('missing', 'tampered'):
        if mode == 'missing':
            blob.unlink()
        else:
            changed = bytearray(original)
            changed[-1] ^= 1
            blob.write_bytes(changed)
        pcm = output / f'refused-{mode}.pcm'
        result = native_process.run(list(map(str, [player, '--package', data_root, '--ticks', '1', '--output', pcm])),
                                    cwd=player.parent, env=env, timeout=60)
        require(result.returncode != 0 and not pcm.exists(), 'Bad font was accepted or output published')
        write(output / f'refused-{mode}.json', {'returncode': result.returncode, 'stderr': result.stderr})
        rejected.append(mode)
        blob.write_bytes(original)
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'fresh_exported_runs': runs,
        'refusals': rejected, 'authoring_deleted': True, 'build_storage_removed': True,
        'external_debug_source_maps_removed': removed_maps,
        'player_sha256': exported['player_sha256'], 'target': exported['target'],
        'limitations': ['CPU composed host proof only', 'No GPU text drawing, visual, IME, phone or performance gate']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
