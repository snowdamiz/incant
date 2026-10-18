"""Pure data/report and harmless process controls; no native engine or GPU."""
import copy
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'tools/probes/core-locale-window.py'
PROBE = runpy.run_path(str(SCRIPT))


class CoreLocaleWindowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.base = Path(cls.temp.name)
        cls.author = cls.base / 'author'
        (cls.author / 'project').mkdir(parents=True)
        # Fixture identities are the authored sample's real stable IDs. No
        # project is submitted to an engine; generation is data-only in a fresh
        # interpreter to isolate the sample's existing art/hud absolute imports.
        fixture = r'''
import json, pathlib, runpy, sys
root, out = map(pathlib.Path, sys.argv[1:])
sys.path.insert(0, str(root/'samples/core'))
scene = runpy.run_path(str(root/'samples/core/scene.py'))
hud = scene['hud']; bindings = scene['bindings']
receipt = dict(format='incant-core-sample-presentation', version=1, locale='ar', state='playing',
 undo_redo_reopen=True, validation={'valid':True}, entities=scene['E'], enemies=scene['ENEMIES'],
 samples=scene['SAMPLES'], hud=bindings.roles(hud.prompt())['hud'],
 widgets={w['name']:hud.widget(w['index']) for w in hud.DOCUMENT['widgets']},
 localized_text={'ar':{key:key for key,_,_ in hud.requests(4,True)}})
project=out/'project/core.json'
project.write_text(json.dumps({'settings':{'localization':{'fallbacks':[],'locale':'ar','pseudo':False}}}))
(out/'project/gameplay.ts').write_bytes((root/'samples/core/gameplay.ts').read_bytes())
import hashlib
receipt.update(project_sha256=hashlib.sha256(project.read_bytes()).hexdigest(),
 gameplay_sha256=hashlib.sha256((root/'samples/core/gameplay.ts').read_bytes()).hexdigest())
(out/'receipt.json').write_text(json.dumps(receipt))
'''
        subprocess.run([sys.executable, '-B', '-c', fixture, str(ROOT), str(cls.author)],
                       check=True, capture_output=True, timeout=10)
        cls.recordings = cls.base / 'recordings'
        result = subprocess.run([sys.executable, '-B', str(SCRIPT), 'recordings', '--locale', 'ar',
                                 str(cls.author), str(cls.recordings)], capture_output=True, timeout=10)
        if result.returncode:
            raise AssertionError(result.stderr.decode())
        cls.manifest = json.loads((cls.recordings / 'recordings.json').read_text())
        cls.plans = {r['name']: json.loads((cls.recordings / (r['name'] + '-observe.json')).read_text())
                     for r in cls.manifest['cases']}

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def cpu(self, index=2):
        row = self.manifest['cases'][index]
        won = index != 0
        plan = self.plans[row['name']]
        hud = self.manifest['hud']
        entities = {owner: None for owner in plan['entities']}
        for role in ('won', 'lost', 'prompt'):
            shown = role == 'prompt' or (role == 'won') == won
            entities[hud[role]] = {'parent': hud['result' if shown else 'inactive']}
        logs = [{'tick': 664 if won else 616, 'message': 'core.damage health=' + ('2' if won else '0')}]
        if won:
            logs += [{'tick': tick, 'message': 'core.pickup ' + owner + ' count=' + str(i + 1)}
                     for i, (tick, owner) in enumerate(zip((127, 508, 1266, 1842), self.manifest['samples']))]
        logs += [{'tick': 2066 if won else 616, 'message': 'core.won result' if won else 'core.lost result'}]
        value = {'tick': row['simulation_tick'], 'frames': row['host_steps'] * 800,
                 'observations': {'passed': True, 'tick': row['simulation_tick'], 'tick_rate': 60,
                    'entities': entities, 'checks': [{**c, 'passed': True, 'actual': c['expect']['value']}
                                                     for c in sorted(plan['checks'], key=lambda c: (c['tick'], c['name']))]},
                 'logs': {'dropped': 0, 'entries': logs}, 'flipbooks': [], 'sources': {},
                 'render_input': {'mode': 'cpu-only', 'gpu_submitted': False, 'model_count': 6,
                    'camera': PROBE['ulid_hex'](PROBE['CAMERA']), 'ui': {'widgets': 13, 'glyphs': 90,
                        'viewport': [640, 480], 'focus': None, 'sources': row['labels']}}}
        if index == 2:
            value['session'] = {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156,
                'restarts': [{'host_step': 900, 'completed_generation': 1, 'simulation_tick': 900, 'output_frames': '720000'}]}
        return value

    def window(self):
        row = self.manifest['cases'][2]
        return {'mode': 'native-window', 'on_screen': True, 'backend': 'Metal', 'host_steps': 3056,
                'tick': 2156, 'accepted_frames': 2157, 'presentation_refreshes': 2,
                'session': {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156, 'accepted_frames': 3058},
                'input_recording': {'sha256': row['recording_sha256'], 'completed': True,
                    'requested_host_steps': 3056, 'accepted_host_steps': 3056, 'source': 'recorded-physical-input',
                    'physical_devices': False, 'native_focus_and_surface_required': True},
                'observations': copy.deepcopy(self.cpu()['observations']),
                'ui': {'widgets': 7, 'glyph_quads': 90, 'viewport': [640, 480], 'focus': None}}

    def test_unchanged_physical_routes_and_real_restart(self):
        manifest = PROBE['recordings'](self.recordings, 'ar')
        self.assertEqual([(r['host_steps'], r['simulation_tick']) for r in manifest['cases']],
                         [(900, 900), (2156, 2156), (3056, 2156)])
        composite = json.loads((self.recordings / 'lose-restart-win-input.json').read_text())
        self.assertIn({'tick': 900, 'events': [{'type': 'key', 'code': 'KeyR', 'down': True}]}, composite['frames'])
        self.assertTrue(all(set(frame) == {'tick', 'events'} for frame in composite['frames']))
        self.assertEqual(sum(r['host_steps'] * PROBE['TICK_BYTES'] for r in manifest['cases']), 39116800)

    def test_create_only_generation_preserves_existing_inputs(self):
        before = PROBE['inventory'](self.recordings)
        child = subprocess.run([sys.executable, '-B', str(SCRIPT), 'recordings', '--locale', 'ar',
                                str(self.author), str(self.recordings)], capture_output=True, timeout=10)
        self.assertNotEqual(child.returncode, 0)
        self.assertEqual(PROBE['inventory'](self.recordings), before)

    def test_authoring_and_recordings_refuse_other_locale_or_pseudo(self):
        with self.assertRaises(ValueError):
            PROBE['authoring'](self.author, 'zh-Hans')
        with self.assertRaises(ValueError):
            PROBE['recordings'](self.recordings, 'en-XA')
        with tempfile.TemporaryDirectory() as temporary:
            clone = Path(temporary) / 'author'
            import shutil
            shutil.copytree(self.author, clone)
            project = clone / 'project/core.json'
            for pseudo in (True, 0, None, 'false'):
                document = json.loads(project.read_text()); document['settings']['localization']['pseudo'] = pseudo
                project.write_text(json.dumps(document))
                receipt = json.loads((clone / 'receipt.json').read_text()); receipt['project_sha256'] = PROBE['digest'](project)
                (clone / 'receipt.json').write_text(json.dumps(receipt))
                with self.subTest(pseudo=pseudo), self.assertRaises(ValueError):
                    PROBE['authoring'](clone, 'ar')

    def test_cpu_checks_actual_state_text_and_all_observations(self):
        for index, row in enumerate(self.manifest['cases']):
            PROBE['check_cpu'](row, self.cpu(index), self.manifest, self.plans[row['name']])
        mutations = [lambda r: r.update(tick=True), lambda r: r.update(frames=1724800),
                     lambda r: r['observations']['checks'].pop(),
                     lambda r: r['observations']['checks'][0].update(actual=False),
                     lambda r: r['logs'].update(dropped=1), lambda r: r['logs']['entries'].pop(),
                     lambda r: r['render_input'].update(gpu_submitted=True),
                     lambda r: r['render_input']['ui'].update(widgets=7),
                     lambda r: r['render_input']['ui'].update(sources={}),
                     lambda r: r['session'].update(generation=1)]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                value = self.cpu(); mutation(value)
                with self.assertRaises(ValueError):
                    PROBE['check_cpu'](self.manifest['cases'][2], value, self.manifest, self.plans[PROBE['ORDER'][2]])

    def test_observations_follow_runtime_tick_name_order_without_losing_membership(self):
        # incant_observe::Plan::new sorts by (tick,name), not request insertion
        # order. Keep all exact names/ticks/expectations and actual bool results.
        a = {'name': 'a', 'tick': 2, 'expect': {'type': 'exists', 'entity': 'same', 'value': True}}
        z = {'name': 'z', 'tick': 2, 'expect': {'type': 'exists', 'entity': 'same', 'value': False}}
        first = {'name': 'first', 'tick': 1, 'expect': {'type': 'exists', 'entity': 'same', 'value': True}}
        plan = {'entities': ['same'], 'checks': [z, first, a]}
        value = {'observations': {'passed': True, 'tick': 2, 'tick_rate': 60, 'entities': {'same': None},
                 'checks': [{**c, 'passed': True, 'actual': c['expect']['value']} for c in (first, a, z)]}}
        PROBE['observations'](value, plan)
        mutations = [lambda r: r['checks'].pop(),
                     lambda r: r['checks'].__setitem__(1, copy.deepcopy(r['checks'][0])),
                     lambda r: r['checks'][1].update(name='different'),
                     lambda r: r['checks'][1].update(tick=3),
                     lambda r: r['checks'][1].update(expect={**r['checks'][1]['expect'], 'value': False}),
                     lambda r: r['checks'][1].update(actual=False),
                     lambda r: r['checks'][1].update(passed=False),
                     lambda r: r['checks'].reverse()]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                wrong = copy.deepcopy(value); mutation(wrong['observations'])
                with self.assertRaises(ValueError):
                    PROBE['observations'](wrong, plan)

    def test_restart_requires_fresh_world_and_exact_pcm_concatenation(self):
        reports = {name: self.cpu(i) for i, name in enumerate(PROBE['ORDER'])}
        # Small byte fixtures exercise the relation, not a native audio gate.
        pcm = {'lose-short': bytes(60 * PROBE['TICK_BYTES']), 'win': b'winning PCM'}
        pcm['lose-restart-win'] = pcm['lose-short'] + pcm['win']
        PROBE['check_restart'](reports, pcm)
        pcm['lose-restart-win'] = pcm['win'] + pcm['lose-short']
        with self.assertRaises(ValueError):
            PROBE['check_restart'](reports, pcm)
        pcm['lose-restart-win'] = pcm['lose-short'] + pcm['win']
        reports['lose-restart-win']['render_input']['ui']['glyphs'] += 1
        with self.assertRaises(ValueError):
            PROBE['check_restart'](reports, pcm)

    def test_window_current_generation_frame_count_is_not_cumulative(self):
        row = self.manifest['cases'][2]
        PROBE['check_window'](row, self.window(), self.cpu(), self.manifest, self.plans[row['name']])
        self.assertLess(self.window()['accepted_frames'], self.window()['host_steps'])
        for frames in (True, 2156.0, 2155, 3056):
            with self.subTest(frames=frames):
                value = self.window(); value['accepted_frames'] = frames
                with self.assertRaises(ValueError):
                    PROBE['check_window'](row, value, self.cpu(), self.manifest, self.plans[row['name']])

    def test_window_requires_complete_replay_and_seven_draw_inputs(self):
        row = self.manifest['cases'][2]
        mutations = [lambda r: r.update(backend='Vulkan'), lambda r: r.update(host_steps=3055),
                     lambda r: r['session'].update(generation=1), lambda r: r['session'].update(accepted_frames=2157),
                     lambda r: r['input_recording'].update(completed=False),
                     lambda r: r['input_recording'].update(native_focus_and_surface_required=False),
                     lambda r: r['input_recording'].update(sha256='0' * 64),
                     lambda r: r['observations']['checks'].pop()]
        mutations += [lambda r, n=n: r['ui'].update(widgets=n) for n in (0, True, 7.0, 13)]
        mutations += [lambda r, n=n: r['ui'].update(glyph_quads=n) for n in (0, True, None)]
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                value = self.window(); mutation(value)
                with self.assertRaises(ValueError):
                    PROBE['check_window'](row, value, self.cpu(), self.manifest, self.plans[row['name']])

    def test_package_refuses_cross_locale_project_data_and_selected_binary(self):
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary); (package / 'player/data').mkdir(parents=True)
            player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
            player.write_bytes(b'non-executable test input'); data = package / 'player/data/world.scene'; data.write_bytes(b'scene')
            receipt = {'format': 'incant-native-export', 'version': 1, 'mode': 'window',
                       'camera': PROBE['ulid_hex'](PROBE['CAMERA']), 'project_sha256': self.manifest['project_sha256'],
                       'startup_source_parsing': False, 'build_storage_removed_before_startup': True,
                       'player_sha256': PROBE['digest'](player), 'data': {'files': {'world.scene': PROBE['digest'](data)}}}
            PROBE['write'](package / 'export.json', receipt)
            PROBE['package'](package, self.manifest)
            for key, value in (('project_sha256', 'f' * 64), ('version', True), ('mode', 'headless'), ('profiling', {})):
                changed = dict(receipt); changed[key] = value; PROBE['write'](package / 'export.json', changed)
                with self.assertRaises(ValueError):
                    PROBE['package'](package, self.manifest)
            PROBE['write'](package / 'export.json', receipt); data.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                PROBE['package'](package, self.manifest)
            data.write_bytes(b'scene'); other = package / 'other'; other.write_bytes(b'wrong executable')
            with self.assertRaises(ValueError):
                PROBE['package'](package, self.manifest, other)

    @unittest.skipIf(os.name != 'posix', 'Harmless POSIX process-group cleanup control')
    def test_timeout_stops_descendant_and_retains_failure_and_streams(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary); pid = output / 'pid'; beat = output / 'beat'
            descendant = "import os,pathlib,time;pathlib.Path(%r).write_text(str(os.getpid()));f=open(%r,'ab',buffering=0)\nwhile True:f.write(b'x');time.sleep(.01)" % (str(pid), str(beat))
            parent = 'import subprocess,sys,time;subprocess.Popen([sys.executable,"-c",%r]);print("started",flush=True);time.sleep(30)' % descendant
            ledger = []
            wait = PROBE['native_process']._wait
            def wait_after_ready(tree, readers, command, timeout, deadline, cancel=None):
                # Own the real tree throughout bounded fixture startup. Only the
                # execution interval under test receives the original .4s bound.
                startup = time.monotonic() + 5
                while not (pid.exists() and beat.exists() and beat.stat().st_size
                           and b'started' in (output / 'harmless.stdout.raw.log').read_bytes()):
                    if cancel is not None:
                        cancel()
                    self.assertFalse(tree.exited(), 'Fixture exited before readiness')
                    self.assertLess(time.monotonic(), startup, 'Fixture readiness deadline exceeded')
                    time.sleep(.005)
                return wait(tree, readers, command, timeout, time.monotonic() + timeout, cancel)
            with self.assertRaises(subprocess.TimeoutExpired), \
                    patch.object(PROBE['native_process'], '_wait', wait_after_ready):
                PROBE['run_stage']([sys.executable, '-u', '-c', parent], output, 'harmless', .4, lambda: None, ledger)
            self.assertTrue(ledger[0]['managed_call_finished'])
            self.assertIn('TimeoutExpired', ledger[0]['error'])
            self.assertIn(b'started', (output / 'harmless.stdout.raw.log').read_bytes())
            child = int(pid.read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(child, 0)
            size = beat.stat().st_size; time.sleep(.05)
            self.assertEqual(beat.stat().st_size, size)
            self.assertTrue(json.loads((output / 'processes.json').read_text())[0]['managed_call_finished'])

    def test_prelaunch_cancellation_starts_no_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary); marker = output / 'must-not-exist'; ledger = []
            def refuse():
                raise InterruptedError('latched cancellation')
            with self.assertRaises(InterruptedError):
                PROBE['run_stage']([sys.executable, '-c', 'from pathlib import Path;Path(%r).touch()' % str(marker)],
                                   output, 'cancel', 5, refuse, ledger)
            self.assertFalse(marker.exists())
            self.assertTrue(ledger[0]['managed_call_finished'])

    @unittest.skipIf(os.name != 'posix', 'Harmless POSIX process control')
    def test_baseline_stops_first_failed_harmless_child_and_retains_result(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary); package = base / 'package'; (package / 'player/data').mkdir(parents=True)
            player = package / 'player/incant_game_player'
            calls = base / 'calls'
            player.write_text('#!' + sys.executable + '\nimport sys\nfrom pathlib import Path\n'
                              + 'with Path(%r).open("a") as f:f.write("one\\n")\n' % str(calls)
                              + 'print("intentional harmless refusal",flush=True)\nsys.exit(17)\n')
            player.chmod(0o700)
            data = package / 'player/data/world.scene'; data.write_bytes(b'not native scene data')
            PROBE['write'](package / 'export.json', {'format': 'incant-native-export', 'version': 1,
                'mode': 'window', 'camera': PROBE['ulid_hex'](PROBE['CAMERA']),
                'project_sha256': self.manifest['project_sha256'], 'startup_source_parsing': False,
                'build_storage_removed_before_startup': True, 'player_sha256': PROBE['digest'](player),
                'data': {'files': {'world.scene': PROBE['digest'](data)}}})
            output = base / 'output'
            # This control tests process refusal, not the machine's available
            # storage. Only this harmless fixture interpreter removes its floor.
            entry = 'import runpy,sys;p=runpy.run_path(sys.argv.pop(1));p["bounded"].__globals__["FLOOR"]=0;p["main"]()'
            child = subprocess.run([sys.executable, '-B', '-c', entry, str(SCRIPT), 'baseline', '--locale', 'ar',
                                    str(package), str(self.recordings), str(output)], capture_output=True, timeout=10)
            self.assertNotEqual(child.returncode, 0)
            self.assertEqual(calls.read_text(), 'one\n')
            result = json.loads((output / 'result.json').read_text())
            self.assertIs(result['passed'], False)
            self.assertIs(result['managed_calls_finished'], True)
            ledger = json.loads((output / 'processes.json').read_text())
            self.assertEqual(len(ledger), 1)
            self.assertEqual(ledger[0]['exit_code'], 17)
            self.assertEqual((output / 'lose-short.stdout.raw.log').read_text(), 'intentional harmless refusal\n')


if __name__ == '__main__':
    unittest.main()
