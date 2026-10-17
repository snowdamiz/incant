"""Fixed animated vertex refresh fixtures; no engine execution or pixel claim."""
import base64
import json
import struct

import native_editor_animation

STAGES = ['manual_import_original', 'watched_vertices', 'undo_original', 'redo_vertices',
          'incompatible_topology_retained_replacement', 'source_recovered_vertices']
REFUSAL = 'animated model topology, rest or skin bindings changed'


def gltf(replacement=False, incompatible=False):
    """Keep animation/skin bindings fixed; reject only an index-order change."""
    if incompatible and not replacement:
        raise ValueError('The incompatible control starts from the replacement')
    document = json.loads(native_editor_animation.gltf())
    raw = bytearray(base64.b64decode(document['buffers'][0]['uri'].split(',', 1)[1], validate=True))
    primitive = document['meshes'][0]['primitives'][0]
    position = document['accessors'][primitive['attributes']['POSITION']]
    view = document['bufferViews'][position['bufferView']]
    if replacement:
        struct.pack_into('<f', raw, view['byteOffset'], -.25)
        position['min'][0] = -.25
    if incompatible:
        indices = document['accessors'][primitive['indices']]
        view = document['bufferViews'][indices['bufferView']]
        struct.pack_into('<3H', raw, view['byteOffset'], 0, 2, 1)
    # Authored normals prevent flat-normal expansion from hiding index order in
    # a changed vertex/influence table. Both controls have identical attributes.
    while len(raw) % 4:
        raw.append(0)
    normals = struct.pack('<9f', *([0, 0, 1] * 3))
    primitive['attributes']['NORMAL'] = len(document['accessors'])
    document['accessors'].append({'bufferView': len(document['bufferViews']),
                                 'componentType': 5126, 'type': 'VEC3', 'count': 3})
    document['bufferViews'].append({'buffer': 0, 'byteOffset': len(raw), 'byteLength': len(normals)})
    raw.extend(normals)
    document['buffers'][0] = {'byteLength': len(raw), 'uri':
        'data:application/octet-stream;base64,' + base64.b64encode(raw).decode()}
    return (json.dumps(document, separators=(',', ':')) + '\n').encode()


def validate(proof, require):
    native_editor_animation.validate(proof, require, refusal_text=REFUSAL)
