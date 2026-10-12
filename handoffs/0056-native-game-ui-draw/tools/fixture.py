#!/usr/bin/env python3
"""Disposable 0056 game UI review fixture through public commands only.

Usage: python3 -I -B fixture.py OUTPUT --binary INCANT_HEADLESS

`init`, `import` and one atomic incant_cmd RPC transaction author the project;
Undo/Redo and a journal reopen are checked. No project JSON is edited directly.
The strings, packaged test fonts (Latin, Arabic, a small CJK subset covering
only 中文日本語一) and generated cubes are review content, not game art. There
is no Script: world motion is native Velocity, so a review harness needs no
project program. Widget appearance comes from the renderer, not from data.
"""
import argparse
import base64
import hashlib
import json
import math
from pathlib import Path
import runpy
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[3]
IDS = {name: '000000000000000000000000' + suffix for name, suffix in {
    'camera': '10', 'light': '11', 'floor': '12', 'cube': '13', 'pillar': '14',
    'root': '20', 'title': '21', 'body': '22', 'actions': '23', 'resume': '24', 'options': '25',
    'online': '26', 'continue': '27', 'fields': '28', 'name_label': '29', 'name': '2A',
    'arabic_label': '2B', 'arabic': '2C', 'cjk_label': '2D', 'cjk': '2E', 'name_group': '30',
    'arabic_group': '31', 'cjk_group': '32'}.items()}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cube(color):
    positions, normals, indices = [], [], []
    for axis in range(3):
        for sign in (-1, 1):
            normal = [0, 0, 0]
            normal[axis] = sign
            u, v = [(1, 2), (2, 0), (0, 1)][axis]
            base = len(positions)
            for a, b in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                p = [0.0, 0.0, 0.0]
                p[axis], p[u], p[v] = 0.5 * sign, 0.5 * a, 0.5 * b
                positions.append(p)
                normals.append(normal)
            quad = [0, 1, 2, 0, 2, 3] if sign > 0 else [0, 2, 1, 0, 3, 2]
            indices.extend(base + i for i in quad)
    data = b''.join(struct.pack('<3f', *p) for p in positions)
    data += b''.join(struct.pack('<3f', *n) for n in normals)
    data += b''.join(struct.pack('<H', i) for i in indices)
    count = len(positions)
    return {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
            'materials': [{'pbrMetallicRoughness': {'baseColorFactor': color,
                                                    'metallicFactor': 0, 'roughnessFactor': 0.6}}],
            'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'NORMAL': 1}, 'indices': 2, 'material': 0}]}],
            'buffers': [{'byteLength': len(data),
                         'uri': 'data:application/octet-stream;base64,' + base64.b64encode(data).decode('ascii')}],
            'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': count * 12},
                            {'buffer': 0, 'byteOffset': count * 12, 'byteLength': count * 12},
                            {'buffer': 0, 'byteOffset': count * 24, 'byteLength': len(indices) * 2}],
            'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': count, 'type': 'VEC3',
                           'min': [-0.5] * 3, 'max': [0.5] * 3},
                          {'bufferView': 1, 'componentType': 5126, 'count': count, 'type': 'VEC3'},
                          {'bufferView': 2, 'componentType': 5123, 'count': len(indices), 'type': 'SCALAR'}]}


def style(**values):
    """UiStyle with explicit defaults; lengths are logical pixels."""
    base = {'size': ['auto', 'auto'], 'min_size': [0, 0], 'max_size': [None, None], 'padding': [0, 0, 0, 0],
            'gap': [0, 0], 'direction': 'column', 'wrap': False, 'align': 'stretch', 'justify': 'start',
            'grow': 0, 'shrink': 0, 'basis': 'auto'}
    base.update(values)
    return base


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    module = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))
    Rpc, require = module['Rpc'], module['require']
    output.mkdir(parents=True, exist_ok=False)
    inputs = output / 'authoring'
    inputs.mkdir()
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'
    transcript = []

    def execute(*command):
        result = subprocess.run([str(binary), *map(str, command)], capture_output=True, text=True,
                                encoding='utf-8', timeout=120)
        transcript.append({'argv': ['incant_headless', *map(str, command)], 'exit_code': result.returncode})
        require(result.returncode == 0, result.stderr[-4000:])
        return result.stdout

    execute('init', project, '--entities', '0')
    fonts = []
    for name in ('NotoSans-Regular.ttf', 'NotoSansArabic.ttf', 'IncantCJKFixture.ttf'):
        shutil.copyfile(ROOT / 'crates/incant_text/tests/fonts' / name, inputs / name)
        fonts.append(json.loads(execute('import', project, name))['asset']['id'])
    meshes = {}
    for name, color in (('slate', [0.15, 0.17, 0.20, 1]), ('amber', [0.80, 0.52, 0.20, 1]),
                        ('teal', [0.18, 0.46, 0.50, 1])):
        (inputs / f'{name}.gltf').write_text(json.dumps(cube(color)), encoding='utf-8')
        meshes[name] = json.loads(execute('import', project, f'{name}.gltf'))['asset']['id']
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))

    def transform(at, rotation=(0, 0, 0, 1), scale=(1, 1, 1)):
        return {'translation': list(at), 'rotation': list(rotation), 'scale': list(scale)}

    def entity(key, name, components, parent=None):
        return {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': IDS[key], 'name': name, 'parent': parent and IDS[parent], 'provenance': None,
            'components': components}}

    def text(content, size, line, wrap='word'):
        return {'content': content, 'fonts': fonts, 'font_size': size, 'line_height': line, 'wrap': wrap}

    def ui(key, name, kind, node_style, order, parent=None, disabled=False):
        return entity(key, name, {'UiNode': {'kind': kind, 'style': node_style, 'order': order,
                                             'disabled': disabled}}, parent)

    def button(key, label, order, disabled=False):
        return ui(key, label, {'role': 'button', 'text': text(label, 16, 20)},
                  style(padding=[16, 9, 16, 9]), order, 'actions', disabled)

    def group(key, caption, value, order):
        # A caption over its field, 4 px apart; groups are 12 px apart.
        return [
            ui(f'{key}_group', f'{key} group', {'role': 'container'}, style(gap=[0, 4]), order, 'fields'),
            ui(f'{key}_label', f'{key} caption', {'role': 'label', 'text': text(caption, 13, 18)},
               style(), 1, f'{key}_group'),
            ui(key, f'{key} field', {'role': 'text_input', 'text': text(value, 16, 22, 'none')},
               style(size=[{'fraction': 1}, 'auto'], padding=[10, 7, 10, 7]), 2, f'{key}_group'),
        ]

    # Key light from the upper left and a little behind the camera.
    sy, cy = math.sin(math.radians(30) / 2), math.cos(math.radians(30) / 2)
    sx, cx = math.sin(math.radians(-40) / 2), math.cos(math.radians(-40) / 2)
    key = (cy * sx, sy * cx, -sy * sx, cy * cx)
    pitch = math.radians(-8)
    camera = (math.sin(pitch / 2), 0, 0, math.cos(pitch / 2))
    commands = [
        entity('camera', 'Camera', {'Transform': transform([0, 0.9, 7], camera),
               'Camera': {'fov_degrees': 45, 'near': 0.1, 'far': 100, 'projection': {'kind': 'perspective'}}}),
        entity('light', 'Key light', {'Transform': transform([0, 0, 0], key),
               'DirectionalLight': {'color': [1, 0.97, 0.92], 'intensity': 2.4, 'shadows': None}}),
        entity('floor', 'Floor', {'Transform': transform([0, -1.25, -2], scale=(16, 0.1, 12)),
               'MeshRenderer': {'mesh': meshes['slate'], 'materials': [], 'cast_shadows': False}}),
        entity('cube', 'Moving crate', {'Transform': transform([2.2, -0.6, 0.4]),
               'MeshRenderer': {'mesh': meshes['amber'], 'materials': [], 'cast_shadows': False},
               'Velocity': {'linear': [-0.4, 0, 0]}}),
        entity('pillar', 'Pillar', {'Transform': transform([-2.6, 0.2, -2.5], scale=(0.9, 3, 0.9)),
               'MeshRenderer': {'mesh': meshes['teal'], 'materials': [], 'cast_shadows': False}}),
        ui('root', 'UI root', {'role': 'root'},
           style(padding=[32, 28, 32, 28], gap=[0, 14], align='start', shrink=1), 0),
        ui('title', 'Title', {'role': 'label', 'text': text('Paused', 30, 36, 'none')}, style(), 1, 'root'),
        ui('body', 'Body', {'role': 'label', 'text': text(
            'Progress is saved at every checkpoint. يتم حفظ التقدم تلقائيا عند كل نقطة. 日本語 中文', 16, 23)},
           style(size=[{'fraction': 1}, 'auto'], max_size=[520, None]), 2, 'root'),
        ui('actions', 'Actions', {'role': 'container'},
           style(size=[{'fraction': 1}, 'auto'], max_size=[560, None], direction='row', wrap=True,
                 gap=[10, 10], align='start'), 3, 'root'),
        button('resume', 'Resume', 1),
        button('options', 'Options', 2),
        button('online', 'Online play', 3, disabled=True),
        button('continue', 'متابعة', 4),
        ui('fields', 'Fields', {'role': 'container'},
           style(size=[{'fraction': 1}, 'auto'], max_size=[360, None], gap=[0, 12]), 4, 'root'),
        *group('name', 'Player name', 'Ada Lovelace', 1),
        *group('arabic', 'الاسم', 'سلام عليكم', 2),
        *group('cjk', '日本語', '日本', 3),
    ]
    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        result = rpc.call('command.execute', {'expected_revision': revision,
                                              'description': 'Author 0056 game UI review scene',
                                              'commands': commands})
        require(result['revision'] == revision + 1, 'Nonatomic authoring')
        authored = rpc.call('project.read')
        rpc.call('history.undo')
        rpc.call('history.redo')
        require(rpc.call('project.read')['project'] == authored['project'], 'Undo/Redo changed content')
        require(rpc.call('project.save')['saved'], 'Project checkpoint failed')
        authored = rpc.call('project.read')
    finally:
        rpc.close()
    transcript.append({'rpc': ['project.read', 'command.execute', 'project.read', 'history.undo',
                               'history.redo', 'project.read', 'project.save', 'project.read'],
                       'commands': len(commands)})
    rpc = Rpc(binary, project, journal)
    try:
        require(rpc.call('project.read') == authored, 'Journal recovery mismatch')
    finally:
        rpc.close()
    record = {'binary_sha256': digest(binary), 'project_sha256': digest(project), 'scene': scene,
              'ids': IDS, 'fonts': fonts, 'meshes': meshes, 'revision': authored['revision'],
              'transcript': transcript, 'command_bus_only': True, 'undo_redo': True, 'journal_reopened': True,
              'commands': commands}
    (output / 'fixture.json').write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'project': str(project), 'camera': IDS['camera'], 'project_sha256': record['project_sha256']}))


if __name__ == '__main__':
    main()
