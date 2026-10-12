#!/usr/bin/env python3
"""Disposable 0055 mixed-direction wrap review fixture, through public commands only.

Usage: python3 -I -B fixture.py OUTPUT --binary INCANT_HEADLESS --layout LAYOUT_JSON
Adapted from the 0054 fixture. `init`, `import` (the three packaged 0054 test
fonts and a generated unit cube) and one atomic incant_cmd RPC transaction
author the project; no project JSON is edited directly. Texts and placement come
from cases.json. Two orthographic cameras frame two 640x480 pages at 100 layout
px per unit for the startup window. Thin cube "guides" mark each declared width
box; their heights come from the CPU layout probe (layout.json). There is no
script and no audio: this review needs static layout only.
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

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CASES = json.loads((HERE / 'cases.json').read_text(encoding='utf-8'))
PREFIX = '0' * 22  # 26-character Crockford ULIDs: prefix + four decimal digits


def ident(n):
    return f'{PREFIX}{n:04d}'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cube():
    """Unit cube (+-0.5) with one material; the same generator as 0054."""
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
            'materials': [{'pbrMetallicRoughness': {'baseColorFactor': [0.32, 0.42, 0.58, 1],
                                                    'metallicFactor': 0, 'roughnessFactor': 0.55}}],
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


def placement(case):
    """World translation of the layout box's top-left corner for the startup frame."""
    width, height = CASES['frame']
    unit = height / CASES['vertical_size']  # layout px per world unit at startup
    cx, cy, _ = CASES['pages'][case['page']]['camera']
    left, top = case['at']
    return [cx + (left - width / 2) / unit, cy + (height / 2 - top) / unit, 0.0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--layout', type=Path, required=True)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    layout = {t['key']: t for t in json.loads(args.layout.read_text(encoding='utf-8'))['texts']}
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
    fonts, font_files = [], {}
    for name in CASES['fonts']:
        shutil.copyfile(ROOT / 'crates/incant_text/tests/fonts' / name, inputs / name)
        font_files[name] = digest(inputs / name)
        fonts.append(json.loads(execute('import', project, name))['asset']['id'])
    (inputs / 'cube.gltf').write_text(json.dumps(cube()), encoding='utf-8')
    mesh = json.loads(execute('import', project, 'cube.gltf'))['asset']['id']
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))

    def transform(at, rotation=(0, 0, 0, 1), scale=(1, 1, 1)):
        return {'translation': list(at), 'rotation': list(rotation), 'scale': list(scale)}

    def entity(key, name, components, parent=None):
        return {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': key, 'name': name, 'parent': parent, 'provenance': None, 'components': components}}

    cameras = {}
    commands = []
    for i, (page, spec) in enumerate(CASES['pages'].items()):
        cameras[page] = ident(10 + i)
        commands.append(entity(cameras[page], f'Camera {page}', {
            'Transform': transform(spec['camera']),
            'Camera': {'fov_degrees': 50, 'near': 0.1, 'far': 100,
                       'projection': {'kind': 'orthographic', 'vertical_size': CASES['vertical_size']}}}))
    # Same key light as 0054: lights the cube faces that look at the cameras.
    sy, cy = math.sin(math.radians(25) / 2), math.cos(math.radians(25) / 2)
    sx, cx = math.sin(math.radians(-35) / 2), math.cos(math.radians(-35) / 2)
    commands.append(entity(ident(20), 'Key light', {
        'Transform': transform([0, 0, 0], (cy * sx, sy * cx, -sy * sx, cy * cx)),
        'DirectionalLight': {'color': [1, 1, 1], 'intensity': 2.5, 'shadows': None}}))
    nodes = {}
    for i, case in enumerate(CASES['texts']):
        group, text = ident(100 + i), ident(200 + i)
        half = math.radians(case.get('rotate_degrees', 0)) / 2
        s = case.get('scale', 1)
        commands.append(entity(group, f'{case["key"]} box', {
            'Transform': transform(placement(case), (0, 0, math.sin(half), math.cos(half)), (s, s, s))}))
        commands.append(entity(text, f'{case["key"]} text', {
            'Transform': transform([0, 0, 0]),
            'Text': {'content': case['content'], 'fonts': fonts, 'font_size': case['font_size'],
                     'line_height': case['line_height'], 'width': case['width'], 'wrap': case['wrap']}},
            parent=group))
        nodes[case['key']] = {'group': group, 'text': text, 'translation': placement(case)}
        if case['guides']:
            height = layout[case['key']]['size'][1]
            for j, x in enumerate((-2, case['width'] + 2)):
                # 2 px x full layout height, 0.05 behind the text plane; text is depth-tested
                # LessEqual without writes, so it draws over a guide it overlaps.
                guide = ident(300 + 2 * i + j)
                commands.append(entity(guide, f'{case["key"]} {"left" if j == 0 else "right"} guide', {
                    'Transform': transform([x / 100, -height / 200, -0.05], scale=(0.02, height / 100, 0.01)),
                    'MeshRenderer': {'mesh': mesh, 'materials': [], 'cast_shadows': False}}, parent=group))
                nodes[case['key']].setdefault('guides', []).append(guide)
    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        result = rpc.call('command.execute', {'expected_revision': revision,
                                              'description': 'Author 0055 bidi wrap review scene',
                                              'commands': commands})
        require(result['revision'] == revision + 1, 'Nonatomic authoring')
        authored = rpc.call('project.read')
        require(rpc.call('project.save')['saved'], 'Project checkpoint failed')
    finally:
        rpc.close()
    transcript.append({'rpc': ['project.read', 'command.execute', 'project.read', 'project.save'],
                       'commands': len(commands)})
    rpc = Rpc(binary, project, journal)
    try:
        require(rpc.call('project.read') == authored, 'Journal recovery mismatch')
    finally:
        rpc.close()
    record = {'binary_sha256': digest(binary), 'project_sha256': digest(project),
              'cases_sha256': digest(HERE / 'cases.json'), 'layout_sha256': digest(args.layout),
              'scene': scene, 'cameras': cameras, 'fonts': fonts, 'font_files': font_files, 'mesh': mesh,
              'nodes': nodes, 'revision': result['revision'], 'transcript': transcript,
              'command_bus_only': True, 'journal_reopened': True}
    (output / 'fixture.json').write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'project': str(project), 'cameras': cameras, 'project_sha256': record['project_sha256']}))


if __name__ == '__main__':
    main()
