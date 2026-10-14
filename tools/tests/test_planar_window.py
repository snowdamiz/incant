"""Data/oracle controls only; these never launch a native engine or GPU."""
import copy
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'tools/probes/planar-window.py'
PROBE = runpy.run_path(str(SCRIPT))


class PlanarWindowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.recordings = Path(cls.temp.name) / 'recordings'
        # A fresh Python interpreter isolates the sample's existing absolute
        # art/hud imports from unrelated sample modules in the tools suite.
        subprocess.run([sys.executable, str(SCRIPT), 'recordings', str(cls.recordings)],
                       check=True, capture_output=True, timeout=10)
        cls.manifest = json.loads((cls.recordings / 'recordings.json').read_text())
        cls.row = cls.manifest['cases'][0]

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def cpu(self):
        m = self.manifest
        return {'tick': 781, 'observations': {'passed': True, 'checks': [{'passed': True}],
                    'entities': {m['player']: {'translation': [10.5, 2.5, 0]}}},
                'logs': {'dropped': 0, 'entries': [{'message': 'planar.pickup ' + p + ' 1/3'} for p in m['pearls']]
                         + [{'message': 'planar.won tick=740 pickups=3'}]},
                'flipbooks': [{'id': list(bytes.fromhex(PROBE['ulid_hex'](owner))), 'frame': frame, 'playing': False}
                              for owner, frame in ((m['player'], 2), (m['boat'], 1))],
                'render_input': {'mode': 'cpu-only', 'gpu_submitted': False, 'sprite_count': 9,
                    'ui': {'viewport': [640, 360], 'widgets': 3, 'glyphs': 28,
                           'sources': {m['hud_status']: 'Sailing home with 3 pearls', m['hud_prompt']: ''}}}}

    def window(self):
        return {'mode': 'native-window', 'on_screen': True, 'backend': 'Metal', 'tick': 781,
                'host_steps': 781, 'accepted_frames': 782,
                'input_recording': {'sha256': self.row['recording_sha256'], 'completed': True,
                    'requested_host_steps': 781, 'accepted_host_steps': 781, 'physical_devices': False,
                    'native_focus_and_surface_required': True},
                'observations': copy.deepcopy(self.cpu()['observations']),
                'ui': {'widgets': 3, 'glyph_quads': 28}}

    def test_exact_existing_routes_and_no_ui_or_viewport_injection(self):
        self.assertEqual([(r['name'], r['ticks']) for r in self.manifest['cases']],
                         [('keyboard-win', 781), ('urchin-loss', 201), ('shore-and-cliff', 221)])
        self.assertIs(self.manifest['route_changes'], False)
        for row in self.manifest['cases']:
            raw = (self.recordings / (row['name'] + '-input.json')).read_bytes()
            recording = json.loads(raw)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), row['recording_sha256'])
            self.assertEqual((recording['tick_rate'], recording['start_tick'], recording['ticks']), (60, 0, row['ticks']))
            self.assertTrue(all(set(f) == {'tick', 'events'} and all(e['type'] == 'key' for e in f['events'])
                                for f in recording['frames']))
            self.assertTrue(all(e['down'] is False for e in recording['frames'][-1]['events']))

    def test_existing_output_is_refused_without_replacing_recordings(self):
        before = (self.recordings / 'recordings.json').read_bytes()
        result = subprocess.run([sys.executable, str(SCRIPT), 'recordings', str(self.recordings)],
                                capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.recordings / 'recordings.json').read_bytes(), before)

    def test_cpu_oracle_requires_gameplay_hud_and_no_gpu(self):
        PROBE['check_cpu'](self.row, self.cpu(), self.manifest)
        mutations = [lambda r: r['logs']['entries'].pop(0),
                     lambda r: r['render_input']['ui']['sources'].update({self.manifest['hud_status']: 'Pending'}),
                     lambda r: r['render_input'].update(gpu_submitted=True),
                     lambda r: r['flipbooks'][0].update(frame=0),
                     lambda r: r['observations']['entities'][self.manifest['player']].update(translation=[10.5, 2.5, .1]),
                     lambda r: r.update(tick=True)]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                report = self.cpu(); mutation(report)
                with self.assertRaises(ValueError):
                    PROBE['check_cpu'](self.row, report, self.manifest)

    def test_window_requires_exact_cpu_parity_and_native_eligibility(self):
        PROBE['check_window'](self.row, self.window(), self.cpu(), self.manifest)
        mutations = [lambda r: r.update(backend='Vulkan'), lambda r: r.update(host_steps=780),
                     lambda r: r['input_recording'].update(sha256='0' * 64),
                     lambda r: r['input_recording'].update(native_focus_and_surface_required=False),
                     lambda r: r['observations']['entities'][self.manifest['player']].update(translation=[10.4, 2.5, 0]),
                     lambda r: r['ui'].update(glyph_quads=0)]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                report = self.window(); mutation(report)
                with self.assertRaises(ValueError):
                    PROBE['check_window'](self.row, report, self.cpu(), self.manifest)

    def test_package_requires_actual_window_export_and_all_data_hashes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); (root / 'player/data').mkdir(parents=True)
            player = root / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
            player.write_bytes(b'not an executable; data-only control')
            data = root / 'player/data/world.scene'; data.write_bytes(b'fixture')
            receipt = {'format': 'incant-native-export', 'version': 1, 'mode': 'window',
                       'camera': PROBE['ulid_hex'](self.manifest['camera']), 'startup_source_parsing': False,
                       'build_storage_removed_before_startup': True, 'player_sha256': PROBE['digest'](player),
                       'data': {'files': {'world.scene': PROBE['digest'](data)}}}
            (root / 'export.json').write_text(json.dumps(receipt))
            PROBE['package'](root, self.manifest)
            receipt['camera'] = self.manifest['camera']
            (root / 'export.json').write_text(json.dumps(receipt))
            with self.assertRaises(ValueError):
                PROBE['package'](root, self.manifest)
            receipt['camera'] = PROBE['ulid_hex'](self.manifest['camera'])
            (root / 'export.json').write_text(json.dumps(receipt))
            data.write_bytes(b'corrupted')
            with self.assertRaises(ValueError):
                PROBE['package'](root, self.manifest)
            data.write_bytes(b'fixture'); receipt.update(mode='window', version=True)
            (root / 'export.json').write_text(json.dumps(receipt))
            with self.assertRaises(ValueError):
                PROBE['package'](root, self.manifest)
            receipt.update(mode='headless', version=1)
            (root / 'export.json').write_text(json.dumps(receipt))
            with self.assertRaises(ValueError):
                PROBE['package'](root, self.manifest)


if __name__ == '__main__':
    unittest.main()
