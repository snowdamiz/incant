"""Receipt refusal and real harmless subprocess lifecycle controls, not a native gate."""
import copy
import base64
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import struct
import sys
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('native_editor_textures', ROOT/'tools/probes/native-editor-textures.py')
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


def proof(kind='texture'):
    stages = []
    for i, name in enumerate(probe.STAGES if kind == 'texture' else probe.MODEL_STAGES):
        stages.append({'stage':name, 'tick':4, 'frames':10+i, 'recipe_matches':True,
                       'accepted_fingerprint':('a' if i in (0,2) else 'b')*64,
                       'history':{'revision':3+min(i,3)}})
    return dict(passed=True, asset_kind=kind, platform='macos', adapter='AdapterInfo { backend: Metal }',
                play_generation=2, paused_tick=4, final_tick=5, stages=stages,
                vm_counter={'before_translation':20, 'observed_delta':10, 'expected_delta':10},
                malformed_source_refusal={'code':'asset.import'}, pixel_capture=False,
                appearance_review=False, performance_or_phase_gate=False,
                **{k:True for k in ('world_identity_unchanged', 'module_counter_continued',
                    'actual_window_presentations', 'manual_import_execute_history_handlers',
                    'real_source_watcher', 'undo_not_overwritten', 'source_recovery',
                    'stale_generation_refused', 'stop_releases_world')})


class ReceiptControls(unittest.TestCase):
    def test_accepts_exact_scoped_stage_receipt(self):
        value = proof()
        self.assertIs(probe.validate_proof(value), value)

    def test_model_proof_cannot_be_substituted_by_texture_or_unmarked_proof(self):
        value = proof('model')
        self.assertIs(probe.validate_proof(value, 'model'), value)
        for candidate in (proof(), dict(value, asset_kind='texture'),
                          dict(value, stages=proof()['stages'])):
            with self.assertRaises(ValueError):
                probe.validate_proof(candidate, 'model')
        del value['asset_kind']
        with self.assertRaises(ValueError):
            probe.validate_proof(value, 'model')
        # Existing archived texture receipts predate the explicit kind field.
        legacy = proof()
        del legacy['asset_kind']
        self.assertIs(probe.validate_proof(legacy), legacy)

    def test_model_fixture_changes_geometry_and_material_with_stable_bindings(self):
        source, replacement, before, after = probe.fixture('model')
        self.assertEqual((source, replacement), ('refresh.gltf', 'refresh-replacement.gltf'))
        before, after = json.loads(before), json.loads(after)
        for field in ('nodes', 'scenes', 'scene', 'meshes'):
            self.assertEqual(before[field], after[field])
        self.assertNotEqual(before['materials'], after['materials'])
        positions = []
        for document in (before, after):
            data = base64.b64decode(document['buffers'][0]['uri'].split(',', 1)[1], validate=True)
            self.assertEqual(len(data), document['buffers'][0]['byteLength'])
            points = list(zip(*[iter(struct.unpack('<9f', data))]*3))
            positions.append(points)
            for axis in range(3):
                self.assertAlmostEqual(min(p[axis] for p in points), document['accessors'][0]['min'][axis])
                self.assertAlmostEqual(max(p[axis] for p in points), document['accessors'][0]['max'][axis])
            self.assertNotIn('animations', document)
            self.assertNotIn('skins', document)
        self.assertNotEqual(*positions)

    def test_refuses_old_catalog_wrong_revision_tick_and_no_next_presentation(self):
        for edit in (
            lambda p:p['stages'][1].update(accepted_fingerprint='a'*64),
            lambda p:p['stages'][2]['history'].update(revision=4),
            lambda p:p['stages'][3].update(tick=5),
            lambda p:p['stages'][4].update(frames=13),
            lambda p:p.update(world_identity_unchanged=False),
            lambda p:p['vm_counter'].update(observed_delta=2),
        ):
            value = proof()
            edit(value)
            with self.assertRaises(ValueError):
                probe.validate_proof(value)

    def test_refuses_boolean_revision_and_unearned_pixel_or_gate_claim(self):
        for edit in (
            lambda p:p.update(play_generation=True),
            lambda p:p.update(final_tick=True),
            lambda p:p['stages'][0]['history'].update(revision=True),
            lambda p:p.update(pixel_capture=True),
            lambda p:p.update(performance_or_phase_gate=True),
            lambda p:p.update(malformed_source_refusal={'message':'failed'}),
        ):
            value = copy.deepcopy(proof())
            edit(value)
            with self.assertRaises(ValueError):
                probe.validate_proof(value)


class ManagedProtocolControls(unittest.TestCase):
    def test_malformed_bounds_refuse_before_spawning(self):
        for bound in (0, -1, float('nan'), float('inf'), True):
            with self.assertRaises(ValueError):
                probe.rpc_process(['must-not-spawn'], 'request\n', bound, lambda:None)

    def test_actual_bounded_stdin_stdout_and_stderr(self):
        code = "import sys; data=sys.stdin.read(); print(data.upper(),end=''); print('diagnostic',file=sys.stderr)"
        result = probe.rpc_process([sys.executable, '-c', code], 'request\n', 5, lambda:None)
        self.assertEqual((result.returncode,result.stdout,result.stderr), (0,'REQUEST\n','diagnostic\n'))

    @unittest.skipUnless(os.name == 'posix', 'POSIX cancellation lifecycle')
    def test_repeated_term_is_latched_and_owned_descendant_writer_stops(self):
        with tempfile.TemporaryDirectory() as folder:
            ready = Path(folder)/'pid'
            writes = Path(folder)/'writes'
            descendant = ("import os,pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); "
                          "f=open(sys.argv[2],'ab',buffering=0);\nwhile True: f.write(b'x'); time.sleep(.01)")
            parent = ("import subprocess,sys,time; "
                      "subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2],sys.argv[3]]); "
                      "sys.stdin.read(); time.sleep(60)")
            stop = threading.Event()
            def interrupt():
                end = time.monotonic() + 4
                while not stop.wait(0.01) and (not ready.exists() or not writes.exists()) and time.monotonic() < end:
                    pass
                if not stop.is_set():
                    os.kill(os.getpid(), signal.SIGTERM)
                    os.kill(os.getpid(), signal.SIGTERM)
            helper = threading.Thread(target=interrupt)
            helper.start()
            before = signal.getsignal(signal.SIGTERM)
            try:
                with probe.execution_deadline(5) as (check, _):
                    with self.assertRaisesRegex(RuntimeError, 'Cancelled by signal'):
                        probe.rpc_process([sys.executable,'-c',parent,descendant,str(ready),str(writes)], 'request\n', 5, check)
            finally:
                stop.set()
                helper.join()
            self.assertEqual(signal.getsignal(signal.SIGTERM), before)
            self.assertTrue(ready.exists())
            pid = int(ready.read_text())
            end = time.monotonic() + 5
            while True:
                try:
                    os.kill(pid,0)
                except ProcessLookupError:
                    break
                self.assertLess(time.monotonic(),end,'Owned descendant was not reaped')
                time.sleep(0.01)
            size = writes.stat().st_size
            threading.Event().wait(0.05)
            self.assertEqual(writes.stat().st_size,size)

    def test_protocol_timeout_cleans_child_and_retains_refusal(self):
        code = "import sys,time; sys.stdin.read(); print('before timeout',flush=True); print('diagnostic',file=sys.stderr,flush=True); time.sleep(60)"
        with self.assertRaises(subprocess.TimeoutExpired) as caught:
            probe.rpc_process([sys.executable,'-c',code],
                              'request\n', 0.5, lambda:None)
        record = probe.failure_record(caught.exception)
        self.assertTrue(record['reaped'])
        self.assertEqual(record['timeout_capture_scope'], 'rpc-received-prefix-before-cleanup')
        self.assertEqual(base64.b64decode(record['stdout_base64']), b'before timeout\n')
        self.assertEqual(base64.b64decode(record['stderr_base64']), b'diagnostic\n')

    def test_timeout_capture_is_lossless_and_missing_capture_explicit(self):
        error = subprocess.TimeoutExpired(['harmless'], 1, output=b'bad UTF-8: \xff\n')
        record = probe.failure_record(error)
        self.assertEqual(base64.b64decode(record['stdout_base64']), error.output)
        self.assertEqual(record['stdout_bytes'], len(error.output))
        self.assertTrue(record['stdout_capture_available'])
        self.assertFalse(record['stderr_capture_available'])
        self.assertNotIn('stderr', record)


if __name__ == '__main__':
    unittest.main()
