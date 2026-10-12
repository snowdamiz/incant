#!/usr/bin/env python3
"""Disposable 0054 rendered-review fixture through public commands only.

Usage: python3 -I -B fixture.py OUTPUT --binary INCANT_HEADLESS
`init`, `import` and one atomic incant_cmd RPC transaction author the project;
no project JSON is edited directly. The text strings, packaged test fonts and
the generated cube are numerical review content, not game art. The audio clip is
deliberately silent so the composed audio path runs without disturbing anyone.
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
import wave

ROOT = Path(__file__).resolve().parents[3]
IDS = {name: '000000000000000000000000' + suffix for name, suffix in {
    'script': '01', 'camera': '10', 'light': '11', 'cube': '12', 'latin': '20', 'arabic': '21',
    'cjk': '22', 'mixed': '23', 'label': '24', 'behind': '25'}.items()}
TEXT = {
    'latin': ('Office affinity fluffy waffle', 44, 56, 640, 'word', [-3.4, 2.55, 0]),
    'arabic': ('مرحبا بالعالم', 40, 56, 640, 'word', [-3.4, 1.75, 0]),
    'cjk': ('中文 日本語', 48, 64, 640, 'word', [-3.4, 1.0, 0]),
    'mixed': ('Wrapped ffi سلام 中文 ש and spaces', 28, 36, 300, 'word', [-3.4, 0.1, 0]),
    'label': ('ffi 中文', 24, 32, 300, 'none', [-0.42, 0.25, 0.51]),
    'behind': ('Behind the cube', 36, 48, 600, 'none', [0.6, -0.8, -1.6]),
}
SOURCE = """import type { NativeBehavior } from '@incant/runtime/native';
// Review motion only: props give each instance a base position, a horizontal
// sway amplitude and a yaw amplitude; poses derive from simulation time.
export default {
  update({ instances, chunks, clock }) {
    const t = clock.elapsed_seconds;
    for (const instance of instances) {
      const base = instance.props.base as readonly number[];
      const sway = Number(instance.props.sway);
      const yaw = Number(instance.props.spin) * Math.sin(0.7 * t);
      for (const chunk of chunks) {
        const i = chunk.ids.indexOf(instance.entity);
        if (i < 0) continue;
        const o = i * chunk.transformStride;
        chunk.transforms[o] = base[0] + sway * Math.sin(0.8 * t);
        chunk.transforms[o + 1] = base[1];
        chunk.transforms[o + 2] = base[2];
        chunk.transforms[o + 3] = 0;
        chunk.transforms[o + 4] = Math.sin(yaw / 2);
        chunk.transforms[o + 5] = 0;
        chunk.transforms[o + 6] = Math.cos(yaw / 2);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
"""


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cube():
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
    (inputs / 'cube.gltf').write_text(json.dumps(cube()), encoding='utf-8')
    mesh = json.loads(execute('import', project, 'cube.gltf'))['asset']['id']
    with wave.open(str(inputs / 'silence.wav'), 'wb') as audio:
        audio.setnchannels(1)
        audio.setsampwidth(2)
        audio.setframerate(48000)
        audio.writeframes(b'\0\0' * 48000)
    clip = json.loads(execute('import', project, 'silence.wav'))['asset']['id']
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))

    def transform(at, rotation=(0, 0, 0, 1)):
        return {'translation': at, 'rotation': list(rotation), 'scale': [1, 1, 1]}

    def entity(key, name, components, parent=None):
        return {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': IDS[key], 'name': name, 'parent': parent, 'provenance': None, 'components': components}}

    def text(key):
        content, size, line, width, wrap, _ = TEXT[key]
        return {'content': content, 'fonts': fonts, 'font_size': size, 'line_height': line,
                'width': width, 'wrap': wrap}

    # Unit quaternion: yaw 25 degrees after pitch -35 degrees; local -Z shines
    # down and away from the camera, lighting the cube's front and top faces.
    sy, cy = math.sin(math.radians(25) / 2), math.cos(math.radians(25) / 2)
    sx, cx = math.sin(math.radians(-35) / 2), math.cos(math.radians(-35) / 2)
    key = (cy * sx, sy * cx, -sy * sx, cy * cx)
    commands = [
        {'op': 'write_script', 'script_id': IDS['script'], 'expected_source': {'state': 'missing'}, 'source': SOURCE},
        {'op': 'upsert_script', 'script': {'id': IDS['script'], 'name': 'Review motion', 'path': 'review_motion.ts'}},
        entity('camera', 'Camera', {'Transform': transform([0, 0, 6]),
               'Camera': {'fov_degrees': 50, 'near': 0.1, 'far': 100, 'projection': {'kind': 'perspective'}}}),
        entity('light', 'Key light', {'Transform': transform([0, 0, 0], key),
               'DirectionalLight': {'color': [1, 1, 1], 'intensity': 2.5, 'shadows': None}}),
        entity('cube', 'Cube', {'Transform': transform([1.9, -1.2, -0.5]),
               'MeshRenderer': {'mesh': mesh, 'materials': [], 'cast_shadows': False},
               'Script': {'source': IDS['script'], 'props': {'base': [1.9, -1.2, -0.5], 'sway': 0, 'spin': 0.45}},
               'AudioSource': {'clip': clip, 'gain_db': 0, 'pan': 0, 'rate': 1, 'start_seconds': 0,
                               'playing': True, 'looping': True, 'streaming': False}}),
    ]
    for key, name in (('latin', 'Latin ligatures'), ('arabic', 'Arabic RTL'), ('cjk', 'CJK fallback'),
                      ('mixed', 'Mixed wrapped line'), ('behind', 'Text behind cube')):
        components = {'Transform': transform(TEXT[key][5]), 'Text': text(key)}
        if key == 'mixed':
            components['Script'] = {'source': IDS['script'], 'props': {'base': TEXT[key][5], 'sway': 0.25, 'spin': 0}}
        commands.append(entity(key, name, components))
    commands.append(entity('label', 'Cube label', {'Transform': transform(TEXT['label'][5]), 'Text': text('label')},
                           parent=IDS['cube']))
    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        result = rpc.call('command.execute', {'expected_revision': revision,
                                              'description': 'Author 0054 text draw review scene',
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
    record = {'binary_sha256': digest(binary), 'project_sha256': digest(project), 'script_sha256': digest(inputs / 'review_motion.ts'),
              'scene': scene, 'ids': IDS, 'fonts': fonts, 'mesh': mesh, 'clip': clip,
              'texts': {k: {'content': v[0], 'font_size': v[1], 'line_height': v[2], 'width': v[3], 'wrap': v[4],
                            'translation': v[5]} for k, v in TEXT.items()},
              'revision': result['revision'], 'transcript': transcript, 'command_bus_only': True,
              'journal_reopened': True}
    (output / 'fixture.json').write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'project': str(project), 'camera': IDS['camera'], 'project_sha256': record['project_sha256']}))


if __name__ == '__main__':
    main()
