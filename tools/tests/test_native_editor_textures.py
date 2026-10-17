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


def animated_proof():
    value = proof('model')
    value['asset_kind'] = 'animated-model'
    before = {'playback':{'clip':0,'seconds':.5+value['paused_tick']/60,'speed':1,'playing':True,'looping':True},
              'completed':False,'transition':None}
    after = copy.deepcopy(before)
    after['playback']['seconds'] += 1/60
    event = {'id':'owned-event','level':'error',
             'message':'Native Play asset refresh: animation clip bindings or duration table changed'}
    for i, (stage, name) in enumerate(zip(value['stages'],probe.native_editor_animation.STAGES)):
        stage.update(stage=name,animation=copy.deepcopy(before),history={'revision':3+i})
    value['stages'][4].update(authored_fingerprint='c'*64,refresh_error=event)
    value.update(animation_continued=True,committed_incompatible_refused=True,
        animation_step={'before':before,'after':after},incompatible_refresh_refusal={
            'committed_revision':7,'authored_fingerprint':'c'*64,'accepted_fingerprint':'b'*64,
            'event':event,'native_code_available':False})
    del value['malformed_source_refusal']
    return value


class ReceiptControls(unittest.TestCase):
    def test_animated_fixture_keeps_skin_geometry_and_bindings_while_changing_only_clip_keys(self):
        documents = [json.loads(probe.native_editor_animation.gltf(*args))
                     for args in ((),(True,),(True,True))]
        for field in ('nodes','scenes','scene','skins','meshes','materials'):
            self.assertTrue(all(d[field]==documents[0][field] for d in documents))
        times=[]
        geometry=[]
        for d in documents:
            raw=base64.b64decode(d['buffers'][0]['uri'].split(',',1)[1],validate=True)
            self.assertEqual(len(raw),d['buffers'][0]['byteLength'])
            def accessor(index,code,width):
                a=d['accessors'][index];v=d['bufferViews'][a['bufferView']]
                return struct.unpack_from('<'+code*(a['count']*width),raw,v['byteOffset'])
            primitive=d['meshes'][0]['primitives'][0]
            geometry.append(accessor(primitive['attributes']['POSITION'],'f',3))
            self.assertEqual(accessor(primitive['attributes']['JOINTS_0'],'H',4),(0,)*12)
            self.assertEqual(accessor(primitive['attributes']['WEIGHTS_0'],'f',4),(1,0,0,0)*3)
            sampler=d['animations'][0]['samplers'][0]
            times.append(accessor(sampler['input'],'f',1))
            self.assertEqual(d['animations'][0]['channels'],documents[0]['animations'][0]['channels'])
        self.assertEqual(geometry,[geometry[0]]*3)
        self.assertEqual(times,[(0,2),(0,.5,2),(0,.5,3)])

    def test_animated_receipt_requires_real_commit_refusal_recovery_and_exact_paused_state(self):
        value=animated_proof()
        self.assertIs(probe.validate_proof(value,'animated-model'),value)
        for edit in (
            lambda p:p['stages'][1]['animation']['playback'].update(seconds=0),
            lambda p:p['stages'][4]['history'].update(revision=6),
            lambda p:p['incompatible_refresh_refusal'].update(authored_fingerprint='b'*64),
            lambda p:p['incompatible_refresh_refusal'].update(code='invented'),
            lambda p:p['incompatible_refresh_refusal']['event'].update(level='info'),
            lambda p:p['animation_step']['after']['playback'].update(seconds=.5),
            lambda p:p['animation_step']['before']['playback'].update(clip=True),
            lambda p:p.update(committed_incompatible_refused=False),
        ):
            candidate=copy.deepcopy(value);edit(candidate)
            with self.assertRaises(ValueError):
                probe.validate_proof(candidate,'animated-model')

    def test_animated_mode_cannot_accept_static_or_texture_receipts(self):
        for kind in ('model','texture'):
            with self.assertRaises(ValueError):
                probe.validate_proof(proof(kind),'animated-model')

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

    def test_material_tables_change_without_implicit_geometry_regeneration(self):
        _, _, before, after = probe.fixture('model', 'material-tables')
        documents = [json.loads(value) for value in (before, after)]
        self.assertLess(max(len(before), len(after)), 65536)
        for field in ('buffers', 'bufferViews', 'accessors', 'nodes', 'scenes', 'scene'):
            self.assertEqual(documents[0][field], documents[1][field])
        for document, expected_count, material in zip(documents, (2, 3), (0, 2)):
            primitive = document['meshes'][0]['primitives'][0]
            self.assertEqual(primitive['material'], material)
            self.assertEqual(primitive['attributes'],
                             {'POSITION': 0, 'NORMAL': 1, 'TEXCOORD_0': 2, 'TANGENT': 3})
            data = base64.b64decode(document['buffers'][0]['uri'].split(',', 1)[1], validate=True)
            self.assertEqual(len(data), document['buffers'][0]['byteLength'])
            for index, values in ((1, (0, 0, 1) * 3), (3, (1, 0, 0, 1) * 3)):
                view = document['bufferViews'][index]
                self.assertEqual(struct.unpack_from('<' + 'f' * len(values), data, view['byteOffset']), values)
            for table in ('materials', 'images', 'textures', 'samplers'):
                self.assertEqual(len(document[table]), expected_count)
            for texture in document['textures']:
                self.assertLess(texture['source'], len(document['images']))
                self.assertLess(texture['sampler'], len(document['samplers']))
            for image in document['images']:
                raw = base64.b64decode(image['uri'].split(',', 1)[1], validate=True)
                self.assertTrue(raw.startswith(b'\x89PNG\r\n\x1a\n'))
        active = documents[1]['materials'][2]
        roles = [active['pbrMetallicRoughness'][name] for name in
                 ('baseColorTexture', 'metallicRoughnessTexture')]
        roles += [active[name] for name in ('normalTexture', 'occlusionTexture', 'emissiveTexture')]
        self.assertEqual([role['index'] for role in roles], [2, 1, 0, 1, 2])
        self.assertEqual(active['alphaMode'], 'MASK')
        self.assertNotEqual(documents[0]['materials'], documents[1]['materials'])
        self.assertNotEqual(documents[0]['images'], documents[1]['images'])

    def test_material_variant_refuses_unrelated_workflows(self):
        for kind, variant in (('texture', 'material-tables'), ('animated-model', 'material-tables'),
                              ('model', 'unknown')):
            with self.assertRaises(ValueError):
                probe.fixture(kind, variant)

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
