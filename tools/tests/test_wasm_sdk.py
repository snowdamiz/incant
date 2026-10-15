"""Registry, generated artifacts and fixed trusted guest build contract."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('generator',ROOT/'tools/generate_wasm_sdk.py')
generator=importlib.util.module_from_spec(spec);spec.loader.exec_module(generator)
class RegistryTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(generator.REGISTRY.read_bytes())
    def test_light_controls_append_without_changing_any_published_import(self):
        import hashlib
        self.assertEqual(hashlib.sha256(json.dumps(self.data['functions'][:62],sort_keys=True).encode()).hexdigest(), '5486081da93b52f8f837c50745ca87bd0adb55a8f882e6d1f9174d647b1541f1')
        self.assertEqual([(f['namespace'],f['name'],f['opcode']) for f in self.data['functions'][62:]], [('incant_light_v1','command',62)])
        self.assertEqual(self.data['records']['LightCommand'], [['entity','u8',16],['kind','u32'],['reserved','u32'],['values','f64',3]])
        self.assertEqual(self.data['constants']['LIGHT_MAX_COMMANDS'], self.data['constants']['COMMANDS'])

    def test_all_committed_host_guest_and_c_artifacts_are_exact(self):
        for name,text in generator.outputs().items():self.assertEqual((ROOT/name).read_text(),text,name)
    def test_unknown_versions_namespaces_signatures_and_layouts_fail_closed(self):
        changes=[lambda d:d.update(abi_version=2),lambda d:d.update(endianness='big'),lambda d:d.update(unknown=1),lambda d:d['functions'][0].update(namespace='incant_v9'),lambda d:d['functions'][0].update(params=[['epoch','i32']]),lambda d:d['functions'][0].update(opcode=100),lambda d:d['records']['TransformRow'][0].__setitem__(1,'ptr'),lambda d:d['records']['TransformRow'][0].__setitem__(2,0)]
        for change in changes:
            data=copy.deepcopy(self.data);change(data)
            with self.assertRaises(ValueError):generator.generate(data,'test')
    def test_v1_compatibility_numbers_and_existing_wire_sizes(self):
        funcs=self.data['functions'];self.assertEqual([(f['namespace'],f['name'],f['opcode']) for f in funcs[:3]],[('incant_v1','chunk_count',0),('incant_v1','chunk_len',1),('incant_v1','chunk_flags',2)])
        def size(name):return sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('InputHeader','Keyboard','Mouse','Gamepad','Touch','TouchChange','FlipbookHeader','Flipbook','TransformRow','VelocityRow')],[24,48,68,188,36,24,16,24,80,24])
        self.assertEqual(self.data['enums']['Key'][-1],['F12',103])

    def test_additive_logging_namespace_has_explicit_signature_tags_and_bounds(self):
        log=self.data['functions'][17]
        self.assertEqual((log['namespace'],log['name'],log['opcode']),('incant_log_v1','write',17))
        self.assertEqual(log['params'],[['epoch','i64'],['level','i32'],['ptr','i32'],['len','i32']])
        self.assertIsNone(log['result'])
        self.assertEqual(self.data['enums']['LogLevel'],[['Debug',0],['Info',1],['Warn',2],['Error',3]])
        self.assertEqual([self.data['constants'][n] for n in ('LOG_MAX_MESSAGE_BYTES','LOG_MAX_TICK_ENTRIES','LOG_MAX_TICK_BYTES','LOG_MAX_RETAINED_ENTRIES','LOG_MAX_RETAINED_BYTES')],[1024,64,16384,1024,262144])

    def test_localization_has_four_versioned_copied_imports_and_separate_wire_bounds(self):
        self.assertEqual([(f['namespace'], f['name'], f['opcode']) for f in self.data['functions'][18:22]], [
            ('incant_localization_v1', 'locale', 18), ('incant_localization_v1', 'localize', 19),
            ('incant_localization_v1', 'read_result', 20), ('incant_localization_v1', 'set_locale', 21)])
        self.assertEqual([self.data['constants'][n] for n in ('LOCALIZATION_MAX_QUERIES', 'LOCALIZATION_MAX_INPUT_BYTES', 'LOCALIZATION_MAX_OUTPUT_BYTES', 'LOCALIZATION_MAX_COMMANDS')], [64, 65536, 262144, 64])
        self.assertGreater(self.data['constants']['LOCALIZATION_MAX_REQUEST_BYTES'], self.data['constants']['LOCALIZATION_MAX_INPUT_BYTES'])
        self.assertIn('pub mod localization;', generator.outputs()['crates/incant_wasm_abi/src/lib.rs'])

    def test_additive_formatting_preserves_prior_imports_and_versioned_scalar_frames(self):
        # The published 0–24 import definitions, including signatures and dispatch.
        self.assertEqual(__import__('hashlib').sha256(json.dumps(self.data['functions'][:25],sort_keys=True).encode()).hexdigest(),'903c7c0779e48a38bb08aa28adedc2af37f5900f59935aa26b27fef692c7f59f')
        self.assertEqual(self.data['functions'][25], {'namespace':'incant_localization_v1','name':'format','params':[['epoch','i64'],['ptr','i32'],['len','i32']],'result':'i32','opcode':25,'call':'Call::Format { ptr, len }'})
        self.assertEqual(self.data['magic']['LOCALIZATION_FORMAT_MAGIC'],'INCLFM01')
        self.assertEqual(self.data['constants']['LOCALIZATION_FORMAT_BYTES'],24)
        self.assertEqual(self.data['enums']['DateLength'],[['Short',0],['Medium',1],['Long',2]])

    def test_crossfade_v2_is_additive_and_legacy_layout_stays_exact(self):
        import hashlib
        self.assertEqual(hashlib.sha256(json.dumps(self.data['functions'][:25],sort_keys=True).encode()).hexdigest(),'903c7c0779e48a38bb08aa28adedc2af37f5900f59935aa26b27fef692c7f59f')
        self.assertEqual([(f['namespace'],f['name'],f['opcode']) for f in self.data['functions'][26:29]], [('incant_animation_v2','snapshot_len',26),('incant_animation_v2','read_snapshot',27),('incant_animation_v2','control',28)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual((size('AnimationHeader'),size('AnimationRow'),size('AnimationCommand'),size('AnimationRowV2')),(16,40,24,80))
        self.assertEqual(self.data['records']['AnimationRowV2'][:5],self.data['records']['AnimationRow'])
        self.assertEqual(self.data['magic']['ANIMATION_MAGIC'],'INCAN001')
        self.assertEqual(self.data['magic']['ANIMATION_V2_MAGIC'],'INCAN002')
        self.assertEqual(self.data['constants']['ANIMATION_MAX_TRANSITION_SECONDS'],60)

    def test_steering_is_additive_with_bounded_copied_request_and_result(self):
        import hashlib
        self.assertEqual(hashlib.sha256(json.dumps(self.data['functions'][:29],sort_keys=True).encode()).hexdigest(),'e22b4844821168327448003914bcfc81b0ce45ba1335d855e4cee5617f68b61a')
        self.assertEqual([(f['namespace'],f['name'],f['opcode']) for f in self.data['functions'][29:31]],[('incant_navigation_v1','steer',29),('incant_navigation_v1','read_steering',30)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('SteeringHeader','SteeringAgent','SteeringObstacleHeader','SteeringResultHeader','SteeringVelocity')],[32,104,40,8,40])
        self.assertEqual([self.data['constants'][n] for n in ('STEERING_MAX_AGENTS','STEERING_MAX_OBSTACLES','STEERING_MAX_EDGES','NAVIGATION_MAX_QUERIES')],[128,32,128,32])

    def test_graph_and_actual_pose_snapshots_preserve_all_prior_imports(self):
        import hashlib
        self.assertEqual(hashlib.sha256(json.dumps(self.data['functions'][:31],sort_keys=True).encode()).hexdigest(), '488d58e397c8f8bc65ce4c81370cdfb42ad7a0d81d3cf5f94f5c83fa075b6811')
        self.assertEqual([(f['namespace'],f['name'],f['opcode']) for f in self.data['functions'][31:36]], [
            ('incant_animation_v3','snapshot_len',31), ('incant_animation_v3','read_snapshot',32),
            ('incant_animation_graph_v1','snapshot_len',33), ('incant_animation_graph_v1','read_snapshot',34),
            ('incant_animation_graph_v1','control',35)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('AnimationRowV3','AnimationGraphHeader','AnimationGraphRow','AnimationGraphParameter','AnimationGraphCommand')], [104,24,64,24,32])
        self.assertEqual(self.data['records']['AnimationRowV3'][:-4], self.data['records']['AnimationRowV2'])
        self.assertEqual((self.data['magic']['ANIMATION_V3_MAGIC'],self.data['magic']['ANIMATION_GRAPH_MAGIC']), ('INCAN003','INCAGR01'))
        self.assertEqual([self.data['constants'][n] for n in ('ANIMATION_GRAPH_MAX_ROWS','ANIMATION_GRAPH_MAX_PARAMETERS','ANIMATION_GRAPH_MAX_PARAMETER_ROWS')], [1024,16,16384])

    def test_navigation_path_imports_append_without_changing_prior_contract(self):
        import hashlib
        self.assertEqual(hashlib.sha256(json.dumps(self.data['functions'][:36],sort_keys=True).encode()).hexdigest(), 'fa2e3ec7dde134d5458c49a49fbbd98ad9e2964690a9e7f69e343bd12676e5b1')
        self.assertEqual([(f['name'],f['opcode']) for f in self.data['functions'][36:39]], [('find_path',36),('find_grid_path',37),('read_path',38)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('PathRequest','GridPathRequest','PathResultHeader','PathPoint','PathTraversal','GridPathResultHeader','GridPathCell')], [80,40,32,24,32,24,8])

    def test_physics_preserves_every_published_operation_and_record(self):
        import hashlib
        prior = {'functions': (39, 'e0d4a885e49771e5daa50a01df376190db803ea0ec6d1d1c9227f906cd38b827'), 'constants': (66, '9e783e73163b4113bbb31feffe5e184660f7e32c1c9224060c7c0d9c22efccef'), 'magic': (16, '8e27ae4ba830e36b9a78910acb43e108b8945cfeb7fa3cf46bba42e804e84aa1'), 'enums': (9, '8af90a2e8f963fee749b2c02742ef375ee6c1c522f823534b44d79f86560dcae'), 'records': (36, 'bc13e394043f8a4c6ae4e0f96445f2bc93c1543978c55fc4c1c0a38bb5e6b3f4'), 'namespaces': (10, '642193b37901f7992fabdf53d9d6badb7e68482955ce0b42222f4522d25a7c00')}
        for name, (length, digest) in prior.items():
            value=self.data[name]
            prefix=value[:length] if isinstance(value,list) else dict(list(value.items())[:length])
            self.assertEqual(hashlib.sha256(json.dumps(prefix,sort_keys=True).encode()).hexdigest(),digest,name)
        self.assertEqual([(f['name'],f['opcode']) for f in self.data['functions'][39:42]], [('query_batch',39),('trigger_events',40),('read_result',41)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('PhysicsHeader','PhysicsQuery','PhysicsRow','PhysicsEvent','PhysicsEventHeader')],[16,112,96,40,32])

    def test_weighted_pose_v4_preserves_all_prior_contracts(self):
        import hashlib
        prior = {'functions': (42, '6e4b7168e0fa1d68539dcd81135698c2a1d0928a8401fbc264565ef0a7e3394f'), 'constants': (73, '004d9eda78ce436e0e829a38ddc1731cf3e923591f4c26dcf6105abbec105a19'), 'magic': (19, '5865239dc8d99fc89e9b7ca7c4f5cee7fd7c4bbbfeddd0cc67d038960aa0e8e9'), 'enums': (9, '8af90a2e8f963fee749b2c02742ef375ee6c1c522f823534b44d79f86560dcae'), 'records': (41, 'f6e19fbae8390a07534db3626163d84f4e529e44bd5de231f0573e77ae622964'), 'namespaces': (11, '5257fdbf89a4558c57169c73a59520e26f5ccd408be8941ec2201512f52cef0f')}
        for name, (length, digest) in prior.items():
            value=self.data[name]
            prefix=value[:length] if isinstance(value,list) else dict(list(value.items())[:length])
            self.assertEqual(hashlib.sha256(json.dumps(prefix,sort_keys=True).encode()).hexdigest(),digest,name)
        self.assertEqual([(f['name'],f['opcode']) for f in self.data['functions'][42:44]], [('snapshot_len',42),('read_snapshot',43)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('AnimationHeaderV4','AnimationRowV4','AnimationPoseSample')], [40,96,24])
        self.assertEqual(self.data['records']['AnimationRowV4'][:-4], self.data['records']['AnimationRowV2'])
        self.assertLess(40+100000*96+214336*24,self.data['constants']['MEMORY'])

    def test_audio_preserves_all_prior_entries_and_bounds_complete_batches(self):
        import hashlib
        prior = {'functions': (59, 'ff94f98874d021ee6f475e464025e7e7d0365ef4d73ad043b91ffd075e72355b'), 'constants': (91, 'dbe7519e2f573722b4cc510f264bad5171d72a781af30184393ea81847db0449'), 'magic': (26, '623cd993b428e1129a8668527669aeb5de41727edf4770ec9b30532ba7e33269'), 'enums': (16, '79bb31b08db7c9114769a28cb51186284518aa40a961d970ddd22bdd16705682'), 'records': (59, 'da104457d876b74abce25b6ca1e6cf75c5b9514a77c7d963bc69ed2ea714baa9'), 'namespaces': (18, '4f02d3fe8c61a0510cde409b52c822b55daee6aeb960a475bdb28d68e69e3650')}
        for name, (count, digest) in prior.items():
            value = self.data[name]
            prefix = value[:count] if isinstance(value, list) else dict(list(value.items())[:count])
            self.assertEqual(hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest(), digest, name)
        self.assertEqual([(f['name'], f['opcode']) for f in self.data['functions'][59:62]], [('snapshot_len',59),('read_snapshot',60),('command',61)])
        size = lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('AudioHeader','AudioRow','AudioCommandHeader','AudioCommand')], [40,200,16,32])
        self.assertEqual([self.data['constants'][n] for n in ('AUDIO_MAX_NODES','AUDIO_MAX_BUSES','AUDIO_MAX_SOURCES','AUDIO_MAX_COMMANDS')], [161,32,128,1024])
        self.assertLess(40+161*200, self.data['constants']['MEMORY'])

    def test_hud_is_additive_and_keeps_every_existing_definition(self):
        import hashlib
        prior = {'functions': (56, '7bc086cca969c85182c9ac7cc76aeeb78f525101e1ee09a9fb4703255606497a'), 'constants': (83, 'dc0641095face8dcc5f4272324eb29baee6a59519e319a2436ae0acc61f1f7f9'), 'magic': (25, 'fdf193751733a8c34dd6d592bd1127680bf6d820d3dd037a88e7b01fb38f246e'), 'enums': (14, '61087efd5ded951d7c33c64e3f2aca42da14b2ed57bfbbbbcf5b423b6a356f6d'), 'records': (54, 'a98d1a75f7a4531cbe38cc9baf6c1a656b174d1113f39d853dc41356d9f26461'), 'namespaces': (17, '94e7812e8ff0469646a83623dd46e3a6626f2be6268b8774ee88d575139c4f0d')}
        for name, (count, digest) in prior.items():
            value = self.data[name]
            prefix = value[:count] if isinstance(value,list) else dict(list(value.items())[:count])
            self.assertEqual(hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest(), digest, name)
        self.assertEqual([(f['name'], f['opcode']) for f in self.data['functions'][56:59]], [('snapshot_len',56),('read_snapshot',57),('control',58)])
        self.assertEqual(self.data['constants']['HUD_MAX_WIDGETS'], 1024)
        self.assertEqual(self.data['constants']['HUD_MAX_EVENT_BYTES'], 65536)

    def test_root_motion_is_additive_and_composes_bounded_complete_pose(self):
        import hashlib
        prior = {'functions': (53, 'd27012c3cbb5d955fc95e016a6b81aea45aa2c2723370cf3d26add863b642844'), 'constants': (80, '7bf1f9c9967a0a4b08d2f0c9f4cefa77f54598f08a6bd3a7231d739bb8d21eec'), 'magic': (24, '30df319d481b60150961bac7dc576a196ab8c0a9dd7309c26b99865b89a028ae'), 'enums': (12, 'b4b4ead339d515212605318268308bcf4628cc3b172efabf02997638b20d1629'), 'records': (51, '170b465881f84eee24becf739d8a69162f41bbc7bb46b276ae51dd6cb41e0715'), 'namespaces': (16, '7c6db52d1444153c7b74e3de9ee4d9b13297611693068ab46d6afb8a76f02292')}
        for name, (length, digest) in prior.items():
            value = self.data[name]
            prefix = value[:length] if isinstance(value, list) else dict(list(value.items())[:length])
            self.assertEqual(hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest(), digest, name)
        self.assertEqual([(f['name'], f['opcode']) for f in self.data['functions'][53:56]], [('snapshot_len',53),('read_snapshot',54),('control',55)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('RootMotionHeader','RootMotionRow','RootMotionCommand')], [40,376,24])
        self.assertLess(40+24+48+100000*96+214336*24+1024*40+4096*104+1024*376,self.data['constants']['MEMORY'])

    def test_compound_v5_preserves_all_ik_v1_and_older_contracts(self):
        import hashlib
        prior = {'functions': (47, '24a4f20a4824173fd34ccfe146ffdd4bc02d5a223254e9b8e9d2c22532ac59eb'), 'constants': (79, 'b3dd8bc360f94f28cf8f201757612eb4db3ab78e8802cda823fdcb086591b0b4'), 'magic': (21, '9fda2c34d213c5f6e47d7f19894f5c50c35980082dce3940211807837e41fcc5'), 'enums': (10, 'ee8f597e5d8ac45fbb2156428d197fb24e7c00d1df7891ad529a7a5062024ed2'), 'records': (47, '03e7330e3a13a870f3ef1ca2a0bf7dbd55a1625b2db309c6a9e54bb4f2827858'), 'namespaces': (13, '63726804aa625d91daaa4292808840540e23f7512c0886dc656e30dfc0eadf82')}
        for name, (length, digest) in prior.items():
            value = self.data[name]
            prefix = value[:length] if isinstance(value, list) else dict(list(value.items())[:length])
            self.assertEqual(hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest(), digest, name)
        self.assertEqual([(f['namespace'], f['name'], f['opcode']) for f in self.data['functions'][47:53]], [
            ('incant_animation_v5','snapshot_len',47),('incant_animation_v5','read_snapshot',48),
            ('incant_animation_ik_v2','snapshot_len',49),('incant_animation_ik_v2','read_snapshot',50),
            ('incant_animation_graph_v2','snapshot_len',51),('incant_animation_graph_v2','read_snapshot',52)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('AnimationHeaderV5','AnimationPoseGroups','AnimationGraphHeaderV2','AnimationGraphFade')],[48,40,24,72])
        self.assertLess(24+48+100000*96+214336*24+1024*40+4096*104,self.data['constants']['MEMORY'])

    def test_ik_appends_complete_pose_and_controls_without_changing_prior_contracts(self):
        import hashlib
        prior = {'functions': (44, 'e01ce550578a75dd34b10b66e72ca60a41b121abcf939d2192119a29240ba3fe'), 'constants': (75, '0836adeb1054b65ecd57a43fd98830d4467f9143ab70ee6b7e83eaa49d3c0ccd'), 'magic': (20, '619885d7e753e6c210d83b5a1038fb98131d21711161e031cdb17ee7992048c2'), 'enums': (10, 'ee8f597e5d8ac45fbb2156428d197fb24e7c00d1df7891ad529a7a5062024ed2'), 'records': (44, '0a7b76658ed18036af674986ec5b4ba538f606d03157fff185b27bb578e25fdf'), 'namespaces': (12, '17f9963c5a4fc8bcd41d02cd5c1e9dff8106cd08dc4e0599936f476e55a79d81')}
        for name, (length, digest) in prior.items():
            value = self.data[name]
            prefix = value[:length] if isinstance(value, list) else dict(list(value.items())[:length])
            self.assertEqual(hashlib.sha256(json.dumps(prefix, sort_keys=True).encode()).hexdigest(), digest, name)
        self.assertEqual([(f['namespace'], f['name'], f['opcode']) for f in self.data['functions'][44:47]], [
            ('incant_animation_ik_v1', 'snapshot_len', 44),
            ('incant_animation_ik_v1', 'read_snapshot', 45),
            ('incant_animation_ik_v1', 'control', 46)])
        size = lambda name: sum(generator.SIZES[t] * (n[0] if n else 1) for _, t, *n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('AnimationIkHeader', 'AnimationIkRow', 'AnimationIkCommand')], [24, 104, 88])
        self.assertEqual(self.data['magic']['ANIMATION_IK_MAGIC'], 'INCIK001')
