"""Receipt refusal and real harmless subprocess lifecycle controls, not a native gate."""
import copy
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('native_editor_textures', ROOT/'tools/probes/native-editor-textures.py')
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


def proof():
    stages = []
    for i, name in enumerate(probe.STAGES):
        stages.append({'stage':name, 'tick':4, 'frames':10+i, 'recipe_matches':True,
                       'accepted_fingerprint':('a' if i in (0,2) else 'b')*64,
                       'history':{'revision':3+min(i,3)}})
    return dict(passed=True, platform='macos', adapter='AdapterInfo { backend: Metal }',
                play_generation=2, paused_tick=4, final_tick=5, stages=stages,
                malformed_source_refusal={'code':'asset.import'}, pixel_capture=False,
                appearance_review=False, performance_or_phase_gate=False,
                **{k:True for k in ('world_identity_unchanged', 'same_vm_continued',
                    'actual_window_presentations', 'manual_import_execute_history_handlers',
                    'real_source_watcher', 'undo_not_overwritten', 'source_recovery',
                    'stale_generation_refused', 'stop_releases_world')})


class ReceiptControls(unittest.TestCase):
    def test_accepts_exact_scoped_stage_receipt(self):
        value = proof()
        self.assertIs(probe.validate_proof(value), value)

    def test_refuses_old_catalog_wrong_revision_tick_and_no_next_presentation(self):
        for edit in (
            lambda p:p['stages'][1].update(accepted_fingerprint='a'*64),
            lambda p:p['stages'][2]['history'].update(revision=4),
            lambda p:p['stages'][3].update(tick=5),
            lambda p:p['stages'][4].update(frames=13),
            lambda p:p.update(world_identity_unchanged=False),
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
        with self.assertRaises(subprocess.TimeoutExpired):
            probe.rpc_process([sys.executable,'-c','import sys,time; sys.stdin.read(); time.sleep(60)'],
                              'request\n', 0.1, lambda:None)


if __name__ == '__main__':
    unittest.main()
