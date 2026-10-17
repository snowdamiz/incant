"""Self-contained material-table fixtures for the actual native editor workflow.

The two models have identical authored geometry, including normals, UVs and
explicit tangents. Adding a normal map therefore cannot regenerate vertex data.
"""
import base64
import json
import struct


def gltf(png, replacement=False):
    positions = (-.7, -.5, 0, .7, -.5, 0, 0, .7, 0)
    columns = (positions, (0, 0, 1) * 3, (0, 0, 1, 0, .5, 1), (1, 0, 0, 1) * 3)
    views, accessors, data = [], [], bytearray()
    for values, width in zip(columns, (3, 3, 2, 4)):
        part = struct.pack('<' + 'f' * len(values), *values)
        views.append({'buffer': 0, 'byteOffset': len(data), 'byteLength': len(part)})
        accessors.append({'bufferView': len(views) - 1, 'componentType': 5126,
                          'count': 3, 'type': f'VEC{width}'})
        data.extend(part)
    accessors[0].update(min=[-.7, -.5, 0], max=[.7, .7, 0])
    # Image/table order and cardinality change, while geometry stays byte-exact.
    pixels = ([(128, 128, 255, 255), (0, 0, 255, 255), (255, 180, 0, 255)]
              if replacement else [(255, 0, 0, 255), (128, 128, 255, 255)])
    images = [{'uri': 'data:image/png;base64,' + base64.b64encode(png(pixel)).decode('ascii')}
              for pixel in pixels]
    samplers = [{'magFilter': 9728, 'minFilter': 9984, 'wrapS': 33071, 'wrapT': 33648},
                {'magFilter': 9729, 'minFilter': 9987, 'wrapS': 10497, 'wrapT': 10497}]
    textures = [{'source': 0, 'sampler': 0}, {'source': 1, 'sampler': 1}]
    materials = [
        {'name': 'red', 'pbrMetallicRoughness': {'baseColorTexture': {'index': 0},
         'metallicFactor': 0, 'roughnessFactor': .8}, 'emissiveFactor': [1, 0, 0]},
        {'name': 'unused', 'pbrMetallicRoughness': {'baseColorFactor': [.2, .6, .4, 1]}}]
    if replacement:
        samplers.append({'magFilter': 9729, 'minFilter': 9985,
                         'wrapS': 33071, 'wrapT': 33071})
        textures = [{'source': 0, 'sampler': 1}, {'source': 2, 'sampler': 0},
                    {'source': 1, 'sampler': 2}]
        materials.reverse()
        materials.append({'name': 'blue mapped', 'pbrMetallicRoughness': {
            'baseColorTexture': {'index': 2}, 'metallicRoughnessTexture': {'index': 1}},
            'normalTexture': {'index': 0}, 'occlusionTexture': {'index': 1},
            'emissiveTexture': {'index': 2}, 'emissiveFactor': [.1, .2, .3],
            'alphaMode': 'MASK', 'alphaCutoff': .25, 'doubleSided': True})
    document = {
        'asset': {'version': '2.0'},
        'buffers': [{'byteLength': len(data), 'uri': 'data:application/octet-stream;base64,'
                     + base64.b64encode(data).decode('ascii')}],
        'bufferViews': views, 'accessors': accessors, 'images': images,
        'samplers': samplers, 'textures': textures, 'materials': materials,
        'meshes': [{'primitives': [{'attributes': {
            'POSITION': 0, 'NORMAL': 1, 'TEXCOORD_0': 2, 'TANGENT': 3},
            'material': 2 if replacement else 0}]}],
        'nodes': [{'mesh': 0}], 'scenes': [{'nodes': [0]}], 'scene': 0,
    }
    return (json.dumps(document, separators=(',', ':')) + '\n').encode('utf-8')
