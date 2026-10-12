"""Pearl Cove presentation invariants: identities, art bounds, layout and routes.

CPU-only checks of the generator and builder data; the engine import, command
transactions and native-play scenarios run through samples/planar/scene.py.
"""
import json
import math
import re
from pathlib import Path
import runpy
import struct
import sys
import tempfile
import unittest
import zlib

PLANAR = Path(__file__).resolve().parents[2] / 'samples/planar'
sys.path.insert(0, str(PLANAR))
from art import build, cove, palette  # noqa: E402
import hud  # noqa: E402

CROCKFORD = set('0123456789ABCDEFGHJKMNPQRSTVWXYZ')


def canonical(ulid):
    return len(ulid) == 26 and set(ulid) <= CROCKFORD and ulid[0] in '01234567'


def raw_pixels(path):
    """Decoded scanlines of the generator's filter-0 RGBA PNGs."""
    data, at, idat, size = path.read_bytes(), 8, b'', None
    while at < len(data):
        length, kind = struct.unpack('>I4s', data[at:at + 8])
        if kind == b'IHDR':
            size = struct.unpack('>II', data[at + 8:at + 16])
        elif kind == b'IDAT':
            idat += data[at + 8:at + 8 + length]
        at += 12 + length
    return size, zlib.decompress(idat)


class PearlCoveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        root = Path(cls.temp.name)
        cls.first, cls.second = root / 'a', root / 'b'
        cls.manifest = build.build(cls.first)
        build.build(cls.second)
        cls.descriptor = json.loads((cls.first / 'cove.atlas.json').read_text())

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def test_identities_are_canonical_unique_ulids(self):
        ids = list(self.manifest['frames'].values()) + [cove.ATLAS, cove.SCRIPT, cove.PLAYER, cove.TERRAIN,
                                                         cove.CAMERA, cove.CONTROLLER, cove.BOAT]
        ids += cove.PEARLS + cove.URCHINS + cove.GLINTS + list(cove.LAYERS.values())
        ids += [hud.ROOT, hud.STATUS, hud.PROMPT, hud.TABLE]
        self.assertTrue(all(canonical(i) for i in ids))
        self.assertEqual(len(ids), len(set(ids)))
        self.assertEqual(set(self.descriptor['frames']), set(self.manifest['frames'].values()))

    def test_sheets_are_pixel_deterministic_bounded_and_in_tone_map_identity(self):
        self.assertEqual(self.descriptor, json.loads((self.second / 'cove.atlas.json').read_text()))
        self.assertLessEqual(len(self.descriptor['frames']), 256)
        for name in ('cove-tiles.png', 'cove-sprites.png'):
            (width, height), raw = raw_pixels(self.first / name)
            self.assertEqual(raw_pixels(self.second / name), ((width, height), raw))
            for frame in self.descriptor['frames'].values():
                if frame['path'] == name:
                    x, y, w, h = frame['region']
                    self.assertTrue(x + w <= width and y + h <= height, frame['name'])
                    if frame['name'].split('/')[0] in ('sand', 'shade', 'sea', 'pool', 'rock', 'pier'):
                        self.assertEqual((w, h), (16, 16))
            stride = width * 4 + 1
            for row in range(height):
                line = raw[row * stride:(row + 1) * stride]
                self.assertEqual(line[0], 0)
                for i in range(1, stride, 4):
                    rgb, alpha = tuple(line[i:i + 3]), line[i + 3]
                    if alpha == 255 and rgb != palette.SPARKLE:
                        self.assertLessEqual(max(rgb), 231, (name, row, rgb))

    def test_roles_and_routes_fit_the_layout(self):
        def walkable(x, y):
            return cove.kind(math.floor(x), math.floor(y)) in cove.SAND + '='

        for x, y in [cove.START, *cove.PEARL_AT, *cove.URCHIN_AT]:
            self.assertIn(cove.kind(int(x), int(y)), cove.SAND)
        self.assertEqual(cove.kind(int(cove.EXIT_AT[0]), int(cove.EXIT_AT[1])), '=')
        self.assertIn(cove.kind(int(cove.EXIT_AT[0]), int(cove.EXIT_AT[1]) - 1), cove.WATER)
        play = runpy.run_path(str(PLANAR / 'playtest.py'))
        x, y = cove.START
        reached = []
        for direction, metres in play['WIN']:
            dx, dy = play['VECTORS'][direction]
            for _ in range(round(metres / 0.05)):
                if not all(walkable(x + dx * 0.05 + ox, y + dy * 0.05 + oy)
                           for ox in (-0.3, 0.3) for oy in (-0.3, 0.3)):
                    break  # the solver stops the body at the wall
                x, y = x + dx * 0.05, y + dy * 0.05
                for hx, hy in cove.URCHIN_AT:
                    self.assertGreater(math.hypot(x - hx, y - hy), 0.75, (x, y))
                for index, (px, py) in enumerate(cove.PEARL_AT):
                    if index not in reached and math.hypot(x - px, y - py) < 0.65:
                        reached.append(index)
        self.assertEqual(reached, [0, 1, 2])
        self.assertLess(math.hypot(x - cove.EXIT_AT[0], y - cove.EXIT_AT[1]), 0.7)

    def test_authored_entities_honour_the_controller_contract(self):
        scene = runpy.run_path(str(PLANAR / 'scene.py'))
        commands = scene['terrain_commands']('S', 'T') + scene['actor_commands']('S', 'T')
        entities = {c['entity']['id']: c['entity'] for c in commands}
        props = entities[cove.CONTROLLER]['components']['Script']['props']
        self.assertEqual(set(props), {'player', 'exit', 'pickups', 'hazards', 'speed'})
        roles = [props['player'], props['exit'], *props['pickups'], *props['hazards']]
        self.assertEqual(len(roles), len(set(roles)))
        self.assertTrue(1 <= len(props['pickups']) <= 16 and len(props['hazards']) <= 16)
        self.assertTrue(0 < props['speed'] <= 12)
        player = entities[props['player']]['components']
        self.assertEqual((player['RigidBody2D']['motion'], player['RigidBody2D']['gravity_scale']), ('dynamic', 0))
        self.assertFalse(player['Collider2D']['sensor'])
        self.assertIn('Velocity', player)
        for role in roles[1:]:
            self.assertTrue(entities[role]['components']['Collider2D']['sensor'])
        for role in roles:
            entity = entities[role]
            self.assertIsNone(entity['parent'])
            self.assertEqual(entity['components']['Transform']['scale'], [1, 1, 1])
        self.assertFalse(any(e['parent'] in props['pickups'] for e in entities.values()))
        self.assertNotIn('Transform', entities[cove.CONTROLLER]['components'])

    def test_hud_binds_authored_widgets_and_complete_strings(self):
        scene = runpy.run_path(str(PLANAR / 'scene.py'))
        commands = hud.commands('S', 'FONT', scene['controller']())
        table = next(c['table'] for c in commands if c['op'] == 'upsert_string_table')
        self.assertEqual(table['id'], hud.TABLE)
        self.assertEqual(set(table['messages']), {'hud.collecting', 'hud.ready', 'hud.won', 'hud.lost'})
        for key, translations in table['messages'].items():
            self.assertEqual(set(translations), set(hud.LOCALES), key)
            for text in translations.values():
                names = set(re.findall(r'{(\w+)\s*[,}]', text))
                self.assertLessEqual(names, {'collected', 'total'}, (key, text))
                if 'plural' in text:
                    self.assertRegex(text, r'^{total, plural, one {[^{}]*#[^{}]*} other {[^{}]*#[^{}]*}}$')
        nodes = {c['entity']['id']: c['entity'] for c in commands if c['op'] == 'create_entity'}
        self.assertEqual(nodes[hud.ROOT]['components']['UiNode']['kind'], {'role': 'root'})
        self.assertIsNone(nodes[hud.ROOT]['parent'])
        for widget in (hud.STATUS, hud.PROMPT):
            kind = nodes[widget]['components']['UiNode']['kind']
            self.assertEqual((nodes[widget]['parent'], kind['role'], kind['text']['content']),
                             (hud.ROOT, 'label', ''))
            self.assertEqual(kind['text']['fonts'], ['FONT'])
        binding = next(c for c in commands if c['op'] == 'set_component')
        props = binding['value']['props']
        self.assertEqual(props['flipbooks'], {'player': cove.PLAYER, 'boat': cove.BOAT})
        self.assertEqual(props['hud'], {'table': hud.TABLE, 'status': hud.STATUS, 'prompt': hud.PROMPT})
        self.assertNotIn('results', props)  # no static terminal text that could miss the locale
        self.assertEqual({k: v for k, v in props.items() if k not in ('flipbooks', 'hud')},
                         scene['controller']()['props'])


if __name__ == '__main__':
    unittest.main()
