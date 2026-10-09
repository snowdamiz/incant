#!/usr/bin/env python3
"""Run the real headless renderer and validate PNG transport, not appearance."""
import hashlib
import argparse
import json
from pathlib import Path
import struct
import subprocess
import sys
import zlib

ROOT = Path(__file__).resolve().parents[2]


def validate_png(data, width, height):
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('Renderer did not produce a PNG')
    offset, kinds = 8, []
    while offset < len(data):
        length = struct.unpack('>I', data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        crc = struct.unpack('>I', data[offset + 8 + length:offset + 12 + length])[0]
        if zlib.crc32(kind + payload) != crc:
            raise ValueError('Incomplete or corrupt renderer PNG')
        if kind == b'IHDR' and struct.unpack('>II', payload[:8]) != (width, height):
            raise ValueError('Renderer returned incorrect dimensions')
        kinds.append(kind)
        offset += length + 12
    if not kinds or kinds[0] != b'IHDR' or kinds[-1] != b'IEND' or b'IDAT' not in kinds or offset != len(data):
        raise ValueError('Renderer PNG has missing chunks')


def edit(executable, project, commands, revision=0, journal=None):
    """All probe edits use the same validated command bus as the editor."""
    requests = [
        {'id': 1, 'method': 'command.execute', 'params': {
            'commands': commands, 'expected_revision': revision, 'description': 'Renderer geometry probe'}},
        {'id': 2, 'method': 'project.save', 'params': {}},
    ]
    command = [str(executable), 'rpc', str(project)]
    if journal is not None:
        command += ['--journal', str(journal)]
    result = subprocess.run(command,
                            input=''.join(json.dumps(item) + '\n' for item in requests),
                            text=True, capture_output=True, check=True, timeout=30)
    responses = [json.loads(line) for line in result.stdout.splitlines()]
    if len(responses) != 2 or any('error' in response for response in responses):
        raise ValueError('Renderer probe command transaction failed')


def model_probe(executable, output):
    """Public CLI import, command binding and cache-only GPU rendering."""
    root = output / 'models'
    root.mkdir()
    project = root / 'models.incant.json'
    subprocess.run([str(executable), 'init', str(project), '--name', 'Imported geometry review',
                    '--entities', '2'], check=True, capture_output=True, timeout=30)
    def source(offset):
        (root / 'triangle.bin').write_bytes(struct.pack('<9f', offset-1, -1, 0, offset+1, -1, 0, offset, 1, 0))
        (root / 'triangle.gltf').write_text(json.dumps({
            'asset': {'version': '2.0'}, 'buffers': [{'uri': 'triangle.bin', 'byteLength': 36}],
            'bufferViews': [{'buffer': 0, 'byteLength': 36}],
            'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 3, 'type': 'VEC3',
                           'min': [offset-1, -1, 0], 'max': [offset+1, 1, 0]}],
            'meshes': [{'primitives': [{'attributes': {'POSITION': 0}}]}],
            'nodes': [{'translation': [1, 0, 0], 'children': [1]},
                      {'mesh': 0, 'translation': [0, 2, 0]}, {'mesh': 0, 'translation': [-7, 0, 0]}],
            'scenes': [{'nodes': [2]}, {'nodes': [0]}], 'scene': 1,
        }))
    def import_model():
        return json.loads(subprocess.check_output([str(executable), 'import', str(project), 'triangle.gltf'],
                                                 text=True, timeout=60))
    def capture(name):
        path = root / name
        response = json.loads(subprocess.check_output([str(executable), 'screenshot', str(project), str(path),
                                                      '--width', '640', '--height', '360'], text=True, timeout=120))
        pixels = path.read_bytes()
        validate_png(pixels, 640, 360)
        geometry = response['geometry']
        if geometry != {'diagnostic_entities': 0, 'model_entities': 2, 'primitive_instances': 2,
                        'model_triangles': 2, 'model_draw_calls': 1}:
            raise ValueError('Unexpected imported geometry projection')
        return pixels, response
    source(0)
    imported = import_model()
    document = json.loads(project.read_text())
    asset_id = next(iter(document['assets']))
    scene = next(iter(document['scenes'].values()))
    commands = []
    for entity, x in zip(scene['entities'], [-3, 0]):
        for component, value in {
            'Transform': {'translation': [x, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
            'MeshRenderer': {'mesh': asset_id, 'materials': [], 'cast_shadows': True},
        }.items():
            commands.append({'op': 'set_component', 'scene_id': scene['id'], 'entity_id': entity,
                             'component': component, 'value': value})
    edit(executable, project, commands, imported['revision'], project.with_suffix('.journal.jsonl'))
    first, engine = capture('original.png')
    (root / 'triangle.gltf').unlink()
    (root / 'triangle.bin').unlink()
    retained, _ = capture('without-sources.png')
    if first != retained:
        raise ValueError('Rendering changed after removing model sources')
    source(2)
    import_model()
    changed, _ = capture('reimported.png')
    if first == changed:
        raise ValueError('Reimported geometry did not change the GPU output')
    return {'source_independent': True, 'reimport_changes_pixels': True,
            'original_sha256': hashlib.sha256(first).hexdigest(),
            'reimported_sha256': hashlib.sha256(changed).hexdigest(), 'engine': engine}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'artifacts/render-smoke')
    args = parser.parse_args()
    executable = ROOT / 'target/release' / ('incant_headless.exe' if sys.platform == 'win32' else 'incant_headless')
    output = args.output
    output.mkdir(parents=True, exist_ok=True)
    project, image = output / 'sample.incant.json', output / 'viewport.png'
    subprocess.run([str(executable), 'init', str(project), '--name', 'CI native renderer probe', '--entities', '3'], check=True, timeout=30)
    response = subprocess.check_output([str(executable), 'screenshot', str(project), str(image), '--width', '320', '--height', '180'], text=True, timeout=120)
    engine = json.loads(response)
    data = image.read_bytes()
    validate_png(data, 320, 180)
    scene = next(iter(json.loads(project.read_text())['scenes'].values()))
    parent, child = list(scene['entities'])[:2]
    def transform(entity, translation, rotation, scale):
        return {'op': 'set_component', 'scene_id': scene['id'], 'entity_id': entity,
                'component': 'Transform', 'value': {'translation': translation, 'rotation': rotation, 'scale': scale}}
    def reparent(parent_id):
        return {'op': 'reparent_entity', 'scene_id': scene['id'], 'entity_id': child, 'parent': parent_id}
    edit(executable, project, [
        transform(parent, [0.5, 0, 0], [0, 0, 1, 0], [2, 1, 1]),
        transform(child, [-1, 0, 0], [0, 0, 0, 1], [1, 1, 1]), reparent(parent)])
    def capture(name):
        image = output / name
        subprocess.run([str(executable), 'screenshot', str(project), str(image), '--width', '320', '--height', '180'],
                       capture_output=True, text=True, check=True, timeout=120)
        pixels = image.read_bytes()
        validate_png(pixels, 320, 180)
        return pixels
    hierarchy = capture('hierarchy.png')
    edit(executable, project, [
        reparent(None), transform(child, [2.5, 0, 0], [0, 0, 1, 0], [2, 1, 1])])
    flattened = capture('flattened.png')
    if hierarchy != flattened or hierarchy == data:
        raise ValueError('Parented geometry did not match equivalent world-space geometry')
    report = {'platform': sys.platform, 'native_renderer_executed': True, 'png_transport_validated': True,
              'hierarchy_matches_flattened': True, 'hierarchy_differs_from_initial': True,
              'width': 320, 'height': 180, 'sha256': hashlib.sha256(data).hexdigest(),
              'hierarchy_sha256': hashlib.sha256(hierarchy).hexdigest(),
              'engine': engine, 'imported_models': model_probe(executable, output),
              'visual_review': False, 'native_window_composition_review': False}
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
