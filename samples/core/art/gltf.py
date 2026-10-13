"""Minimal glTF 2.0 writer (.gltf JSON + one .bin) for the engine's importer.

Only core glTF is emitted: POSITION/NORMAL/TEXCOORD_0, uint32 indices, metallic-
roughness materials, external PNG images, decomposed TRS nodes and LINEAR TRS
animation. No extensions, cameras, vertex colours or morph targets, because the
Incant importer refuses them explicitly.
"""
import json
import struct

REPEAT, CLAMP = 10497, 33071
LINEAR, LINEAR_MIPMAP_LINEAR = 9729, 9987


class Material:
    def __init__(self, name, base=(1, 1, 1, 1), metallic=0.0, roughness=0.8, base_texture=None,
                 normal_texture=None, normal_scale=1.0, emissive=(0, 0, 0), emissive_texture=None,
                 occlusion_texture=None, occlusion_strength=1.0, double_sided=False, sampler='repeat'):
        self.name, self.base, self.metallic, self.roughness = name, base, metallic, roughness
        self.base_texture, self.normal_texture, self.normal_scale = base_texture, normal_texture, normal_scale
        self.emissive, self.emissive_texture = emissive, emissive_texture
        self.occlusion_texture, self.occlusion_strength = occlusion_texture, occlusion_strength
        self.double_sided, self.sampler = double_sided, sampler


class Node:
    def __init__(self, name, primitives=(), translation=(0, 0, 0), rotation=(0, 0, 0, 1), scale=(1, 1, 1),
                 children=()):
        """primitives: [(Mesh, Material)]."""
        self.name, self.primitives = name, list(primitives)
        self.translation, self.rotation, self.scale = translation, rotation, scale
        self.children = list(children)


class Writer:
    def __init__(self):
        self.bin = bytearray()
        self.doc = {'asset': {'version': '2.0', 'generator': 'Incant Core Sample art toolkit'},
                    'scene': 0, 'scenes': [{'nodes': []}], 'nodes': [], 'meshes': [], 'materials': [],
                    'buffers': [], 'bufferViews': [], 'accessors': []}
        self.material_index, self.image_index, self.texture_index, self.sampler_index = {}, {}, {}, {}
        self.node_index = {}

    def _view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        view = {'buffer': 0, 'byteOffset': len(self.bin), 'byteLength': len(data)}
        if target:
            view['target'] = target
        self.bin.extend(data)
        self.doc['bufferViews'].append(view)
        return len(self.doc['bufferViews']) - 1

    def _accessor(self, values, kind, component=5126, target=None, bounds=False):
        width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[kind]
        if component == 5126:
            data = struct.pack('<%df' % (len(values) * width), *[c for v in values for c in (v if width > 1 else (v,))])
        else:
            data = struct.pack('<%dI' % len(values), *values)
        accessor = {'bufferView': self._view(data, target), 'componentType': component,
                    'count': len(values), 'type': kind}
        if bounds:
            columns = list(zip(*values)) if width > 1 else [values]
            accessor['min'] = [float(min(c)) for c in columns]
            accessor['max'] = [float(max(c)) for c in columns]
        self.doc['accessors'].append(accessor)
        return len(self.doc['accessors']) - 1

    def _sampler(self, mode):
        if mode not in self.sampler_index:
            wrap = REPEAT if mode == 'repeat' else CLAMP
            self.doc.setdefault('samplers', []).append(
                {'magFilter': LINEAR, 'minFilter': LINEAR_MIPMAP_LINEAR, 'wrapS': wrap, 'wrapT': wrap})
            self.sampler_index[mode] = len(self.doc['samplers']) - 1
        return self.sampler_index[mode]

    def _texture(self, uri, mode):
        key = (uri, mode)
        if key not in self.texture_index:
            if uri not in self.image_index:
                self.doc.setdefault('images', []).append({'uri': uri})
                self.image_index[uri] = len(self.doc['images']) - 1
            self.doc.setdefault('textures', []).append({'source': self.image_index[uri], 'sampler': self._sampler(mode)})
            self.texture_index[key] = len(self.doc['textures']) - 1
        return {'index': self.texture_index[key]}

    def _material(self, m):
        if id(m) in self.material_index:
            return self.material_index[id(m)]
        pbr = {'baseColorFactor': [float(c) for c in m.base], 'metallicFactor': float(m.metallic),
               'roughnessFactor': float(m.roughness)}
        if m.base_texture:
            pbr['baseColorTexture'] = self._texture(m.base_texture, m.sampler)
        out = {'name': m.name, 'pbrMetallicRoughness': pbr}
        if m.normal_texture:
            out['normalTexture'] = dict(self._texture(m.normal_texture, m.sampler), scale=float(m.normal_scale))
        if m.occlusion_texture:
            out['occlusionTexture'] = dict(self._texture(m.occlusion_texture, m.sampler),
                                           strength=float(m.occlusion_strength))
        if any(m.emissive) or m.emissive_texture:
            out['emissiveFactor'] = [float(c) for c in m.emissive]
        if m.emissive_texture:
            out['emissiveTexture'] = self._texture(m.emissive_texture, m.sampler)
        if m.double_sided:
            out['doubleSided'] = True
        self.doc['materials'].append(out)
        self.material_index[id(m)] = len(self.doc['materials']) - 1
        return self.material_index[id(m)]

    def _node(self, node):
        primitives = []
        for mesh, material in node.primitives:
            if not mesh.indices:
                continue
            attributes = {'POSITION': self._accessor(mesh.positions, 'VEC3', target=34962, bounds=True),
                          'NORMAL': self._accessor(mesh.normals, 'VEC3', target=34962)}
            if material.base_texture or material.normal_texture or material.emissive_texture or material.occlusion_texture:
                attributes['TEXCOORD_0'] = self._accessor(mesh.uvs, 'VEC2', target=34962)
            primitives.append({'attributes': attributes, 'material': self._material(material),
                               'indices': self._accessor(mesh.indices, 'SCALAR', 5125, target=34963)})
        out = {'name': node.name}
        if any(node.translation):
            out['translation'] = [float(v) for v in node.translation]
        if tuple(node.rotation) != (0, 0, 0, 1):
            out['rotation'] = [float(v) for v in node.rotation]
        if tuple(node.scale) != (1, 1, 1):
            out['scale'] = [float(v) for v in node.scale]
        if primitives:
            self.doc['meshes'].append({'name': node.name, 'primitives': primitives})
            out['mesh'] = len(self.doc['meshes']) - 1
        self.doc['nodes'].append(out)
        index = len(self.doc['nodes']) - 1
        self.node_index[node.name] = index
        children = [self._node(child) for child in node.children]
        if children:
            out['children'] = children
        return index

    def add_root(self, node):
        self.doc['scenes'][0]['nodes'].append(self._node(node))

    def add_animation(self, name, tracks):
        """tracks: [(node name, 'translation'|'rotation'|'scale', times, values)]; LINEAR."""
        samplers, channels = [], []
        for node, path, times, values in tracks:
            kind = 'VEC4' if path == 'rotation' else 'VEC3'
            samplers.append({'input': self._accessor(times, 'SCALAR', bounds=True),
                             'output': self._accessor(values, kind), 'interpolation': 'LINEAR'})
            channels.append({'sampler': len(samplers) - 1, 'target': {'node': self.node_index[node], 'path': path}})
        self.doc.setdefault('animations', []).append({'name': name, 'samplers': samplers, 'channels': channels})

    def save(self, directory, name):
        """Write NAME.gltf and NAME.bin into directory; returns their paths."""
        self.doc['buffers'] = [{'uri': name + '.bin', 'byteLength': len(self.bin)}]
        for key in ('meshes', 'materials'):
            if not self.doc[key]:
                del self.doc[key]
        gltf_path, bin_path = directory / (name + '.gltf'), directory / (name + '.bin')
        bin_path.write_bytes(bytes(self.bin))
        gltf_path.write_text(json.dumps(self.doc, indent=1, sort_keys=True) + '\n', encoding='utf-8')
        return gltf_path, bin_path


def stats(mesh_material_pairs):
    return {'primitives': len(mesh_material_pairs),
            'vertices': sum(len(m) for m, _ in mesh_material_pairs),
            'triangles': sum(m.triangles() for m, _ in mesh_material_pairs)}
