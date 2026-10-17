"""Fixture isolation and real-receipt rejection controls; no native execution."""
import base64
import copy
import json
import struct
import unittest

from test_native_editor_textures import animated_proof, probe


def decode(raw):
    document = json.loads(raw)
    data = base64.b64decode(document['buffers'][0]['uri'].split(',', 1)[1], validate=True)
    if len(data) != document['buffers'][0]['byteLength']:
        raise ValueError('buffer length')
    def accessor(index):
        item = document['accessors'][index]
        view = document['bufferViews'][item['bufferView']]
        code = {5126: 'f', 5123: 'H'}[item['componentType']]
        width = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[item['type']]
        return struct.unpack_from('<' + code * item['count'] * width, data, view['byteOffset'])
    return document, [accessor(i) for i in range(len(document['accessors']))]


class AnimatedVertexControls(unittest.TestCase):
    def test_replacement_changes_only_position_and_refusal_changes_only_indices(self):
        fixture = probe.native_editor_animated_vertices
        documents, columns = zip(*(decode(fixture.gltf(*args)) for args in ((), (True,), (True, True))))
        primitive = documents[0]['meshes'][0]['primitives'][0]
        position = primitive['attributes']['POSITION']
        indices = primitive['indices']
        normal = primitive['attributes']['NORMAL']
        for field in ('nodes', 'skins', 'animations', 'meshes', 'materials', 'scenes', 'scene', 'bufferViews'):
            self.assertEqual([d[field] for d in documents], [documents[0][field]] * 3)
        self.assertEqual([i for i, (a, b) in enumerate(zip(columns[0], columns[1])) if a != b], [position])
        self.assertEqual(columns[1][position], (-.25,) + columns[0][position][1:])
        self.assertEqual([i for i, (a, b) in enumerate(zip(columns[1], columns[2])) if a != b], [indices])
        self.assertEqual(columns[1][indices], (0, 1, 2))
        self.assertEqual(columns[2][indices], (0, 2, 1))
        self.assertEqual(columns[0][normal], (0, 0, 1) * 3)
        for document, values in zip(documents, columns):
            points = values[position]
            bounds = document['accessors'][position]
            self.assertEqual(bounds['min'], [min(points[i::3]) for i in range(3)])
            self.assertEqual(bounds['max'], [max(points[i::3]) for i in range(3)])
        with self.assertRaises(ValueError):
            fixture.gltf(False, True)

    def test_receipt_requires_vertex_stages_and_actual_topology_refusal(self):
        value = animated_proof()
        value['asset_kind'] = 'animated-vertices'
        fixture = probe.native_editor_animated_vertices
        for stage, name in zip(value['stages'], fixture.STAGES):
            stage['stage'] = name
        value['incompatible_refresh_refusal']['event']['message'] = 'Native Play asset refresh: ' + fixture.REFUSAL
        self.assertIs(probe.validate_proof(value, 'animated-vertices'), value)
        for edit in (
            lambda p: p['stages'][1].update(stage='watched_keys'),
            lambda p: p['incompatible_refresh_refusal']['event'].update(message='Native Play asset refresh: animation clip bindings or duration table changed'),
            lambda p: p['stages'][2]['animation']['playback'].update(seconds=.5),
            lambda p: p['stages'][4].update(accepted_fingerprint='c' * 64),
        ):
            candidate = copy.deepcopy(value)
            edit(candidate)
            with self.assertRaises(ValueError):
                probe.validate_proof(candidate, 'animated-vertices')
        with self.assertRaises(ValueError):
            probe.validate_proof(value, 'animated-model')

    def test_runner_selects_only_the_requested_fixture(self):
        names = probe.fixture('animated-vertices')
        self.assertEqual(names[:2], ('refresh.gltf', 'refresh-replacement.gltf'))
        self.assertEqual(names[2:], (probe.native_editor_animated_vertices.gltf(),
                                    probe.native_editor_animated_vertices.gltf(True)))
        with self.assertRaises(ValueError):
            probe.fixture('animated-vertices', 'material-tables')


if __name__ == '__main__':
    unittest.main()
