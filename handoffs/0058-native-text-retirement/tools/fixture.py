#!/usr/bin/env python3
"""Fresh 0058 native text-retirement fixture through public commands only.

Usage: python3 -I -B fixture.py OUTPUT --binary INCANT_HEADLESS

The 0057 public-command design with new 0058 identities: `init`, `import` and
one atomic incant_cmd RPC transaction author the project; Undo/Redo and a
journal reopen are checked. No project JSON is edited directly.

Layout and strings are kept identical to 0057 (title, body, four buttons and
the known Name, Arabic and CJK caption-field rows), so field regions of the
same editing state can be compared pixel for pixel with the 0057 captures.
A small review script makes UI outcomes visible in the world: Resume steps the
crate one slot right, the Arabic button one slot left, Options pauses its spin
and a submitted field lifts or lowers it. The strings, packaged test fonts
(Latin, Arabic, a CJK subset covering only 中文日本語一) and generated cubes are
review content, not game art. Refuses without a valid 0058 lease.
"""
import argparse
import base64
import datetime
import hashlib
import json
import math
from pathlib import Path
import runpy
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = Path.home() / '.codex/incant-evidence/0058'
PREFIX = '000000000000000000000058'
IDS = {name: PREFIX + suffix for name, suffix in {
    'script': '01', 'camera': '10', 'light': '11', 'floor': '12', 'crate': '13', 'pillar': '14',
    'root': '20', 'title': '21', 'body': '22', 'actions': '23', 'resume': '24', 'options': '25',
    'online': '26', 'continue': '27', 'fields': '28',
    'name_row': '29', 'name_caption': '2A', 'name': '2B',
    'arabic_row': '2C', 'arabic_caption': '2D', 'arabic': '2E',
    'cjk_row': '30', 'cjk_caption': '31', 'cjk': '32'}.items()}
SOURCE = """import type { NativeBehavior } from '@incant/runtime/native';
// 0058 review motion. The crate spins from accumulated simulation time, so the
// world keeps moving under the UI. Accepted UI outcomes change the world, so
// activation and submission are visible in pixels: Resume steps the crate one
// of four slots right, the Arabic button one slot left, Options pauses or
// resumes the spin, and any submitted field lifts or lowers the crate.
const RESUME = '%(resume)s';
const OPTIONS = '%(options)s';
const CONTINUE = '%(continue)s';
let slot = 1;
let spinning = true;
let lifted = false;
let angle = 0;
export default {
  update({ instances, chunks, dt, input }) {
    for (const event of input.ui.events) {
      if (event.kind === 'activated') {
        if (event.entity_id === RESUME) slot = (slot + 1) %% 4;
        else if (event.entity_id === CONTINUE) slot = (slot + 3) %% 4;
        else if (event.entity_id === OPTIONS) spinning = !spinning;
      } else if (event.kind === 'submitted') {
        lifted = !lifted;
      }
    }
    if (spinning) angle += 0.9 * dt;
    for (const instance of instances) {
      for (const chunk of chunks) {
        const i = chunk.ids.indexOf(instance.entity);
        if (i < 0) continue;
        const o = i * chunk.transformStride;
        chunk.transforms[o] = -1.35 + 0.9 * slot;
        chunk.transforms[o + 1] = lifted ? 0.15 : -0.55;
        chunk.transforms[o + 2] = 0.2;
        chunk.transforms[o + 3] = 0;
        chunk.transforms[o + 4] = Math.sin(angle / 2);
        chunk.transforms[o + 5] = 0;
        chunk.transforms[o + 6] = Math.cos(angle / 2);
      }
    }
    return undefined;
  },
} satisfies NativeBehavior;
""" % IDS


def leased():
    if (SCRATCH / 'released').exists():
        raise SystemExit('0058 lease was released; refusing')
    try:
        lease = json.loads((SCRATCH / 'lease.json').read_text(encoding='utf-8'))
        expiry = datetime.datetime.fromisoformat(lease['expires_at'])
    except (OSError, ValueError, KeyError):
        raise SystemExit('no valid 0058 lease; refusing')
    if lease.get('packet') != '0058-native-text-retirement' or \
            datetime.datetime.now(datetime.timezone.utc) >= expiry:
        raise SystemExit('no valid 0058 lease (foreign or expired); refusing')


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
    leased()
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
    for name, color in (('floor', [0.30, 0.32, 0.36, 1]), ('crate', [0.82, 0.55, 0.22, 1]),
                        ('pillar', [0.20, 0.45, 0.52, 1])):
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
        return ui(key, label, {'role': 'button', 'text': text(label, 13, 16, 'none')},
                  style(padding=[10, 5, 10, 5]), order, 'actions', disabled)

    def row(key, caption, value, order):
        # Caption and field on one line; the field takes the remaining width and
        # may shrink to nothing (explicit zero minimum), so long values clip.
        return [
            ui(f'{key}_row', f'{key} row', {'role': 'container'},
               style(size=[{'fraction': 1}, 'auto'], direction='row', align='center', gap=[8, 0]),
               order, 'fields'),
            ui(f'{key}_caption', f'{key} caption', {'role': 'label', 'text': text(caption, 12, 16, 'none')},
               style(size=[{'pixels': 52}, 'auto']), 1, f'{key}_row'),
            ui(key, f'{key} field', {'role': 'text_input', 'text': text(value, 13, 16, 'none')},
               style(padding=[7, 4, 7, 4], grow=1, shrink=1, basis={'pixels': 0}), 2, f'{key}_row'),
        ]

    # Key light from the upper left, slightly behind the camera.
    sy, cy = math.sin(math.radians(30) / 2), math.cos(math.radians(30) / 2)
    sx, cx = math.sin(math.radians(-40) / 2), math.cos(math.radians(-40) / 2)
    key = (cy * sx, sy * cx, -sy * sx, cy * cx)
    pitch = math.radians(-8)
    camera = (math.sin(pitch / 2), 0, 0, math.cos(pitch / 2))
    commands = [
        {'op': 'write_script', 'script_id': IDS['script'], 'expected_source': {'state': 'missing'},
         'source': SOURCE},
        {'op': 'upsert_script', 'script': {'id': IDS['script'], 'name': 'Review outcomes', 'path': 'review_outcomes.ts'}},
        entity('camera', 'Camera', {'Transform': transform([0, 0.9, 7], camera),
               'Camera': {'fov_degrees': 45, 'near': 0.1, 'far': 100, 'projection': {'kind': 'perspective'}}}),
        entity('light', 'Key light', {'Transform': transform([0, 0, 0], key),
               'DirectionalLight': {'color': [1, 0.97, 0.92], 'intensity': 2.4, 'shadows': None}}),
        entity('floor', 'Floor', {'Transform': transform([0, -1.25, -2], scale=(16, 0.1, 12)),
               'MeshRenderer': {'mesh': meshes['floor'], 'materials': [], 'cast_shadows': False}}),
        entity('crate', 'Review crate', {'Transform': transform([-0.45, -0.55, 0.2]),
               'MeshRenderer': {'mesh': meshes['crate'], 'materials': [], 'cast_shadows': False},
               'Script': {'source': IDS['script'], 'props': {}}}),
        entity('pillar', 'Pillar', {'Transform': transform([2.4, 0.2, -2.5], scale=(0.9, 3, 0.9)),
               'MeshRenderer': {'mesh': meshes['pillar'], 'materials': [], 'cast_shadows': False}}),
        ui('root', 'UI root', {'role': 'root'},
           style(padding=[12, 10, 12, 10], gap=[0, 7], align='start', shrink=1), 0),
        ui('title', 'Title', {'role': 'label', 'text': text('Paused', 18, 22, 'none')}, style(), 1, 'root'),
        ui('body', 'Body', {'role': 'label', 'text': text(
            'Progress is saved. تم حفظ التقدم. 日本語 中文', 12, 16)},
           style(size=[{'fraction': 1}, 'auto'], max_size=[360, None]), 2, 'root'),
        ui('actions', 'Actions', {'role': 'container'},
           style(size=[{'fraction': 1}, 'auto'], max_size=[420, None], direction='row', wrap=True,
                 gap=[6, 6], align='start'), 3, 'root'),
        button('resume', 'Resume', 1),
        button('options', 'Options', 2),
        button('online', 'Online', 3, disabled=True),
        button('continue', 'متابعة', 4),
        ui('fields', 'Fields', {'role': 'container'},
           style(size=[{'fraction': 1}, 'auto'], max_size=[300, None], gap=[0, 5]), 4, 'root'),
        *row('name', 'Name', 'Ada Lovelace', 1),
        *row('arabic', 'الاسم', 'سلام', 2),
        *row('cjk', '日本語', '中文', 3),
    ]
    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        result = rpc.call('command.execute', {'expected_revision': revision,
                                              'description': 'Author 0058 native text retirement review scene',
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
    record = {'binary_sha256': digest(binary), 'project_sha256': digest(project),
              'script_sha256': digest(inputs / 'review_outcomes.ts'), 'scene': scene, 'ids': IDS,
              'fonts': fonts, 'font_sha256': {p.name: digest(p) for p in sorted(inputs.glob('*.ttf'))},
              'meshes': meshes, 'revision': authored['revision'], 'transcript': transcript,
              'command_bus_only': True, 'undo_redo': True, 'journal_reopened': True, 'commands': commands}
    (output / 'fixture.json').write_text(json.dumps(record, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'project': str(project), 'camera': IDS['camera'], 'project_sha256': record['project_sha256']}))


if __name__ == '__main__':
    main()
