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
        self.assertEqual([(f['name'],f['opcode']) for f in self.data['functions'][39:]], [('query_batch',39),('trigger_events',40),('read_result',41)])
        size=lambda name: sum(generator.SIZES[t]*(n[0] if n else 1) for _,t,*n in self.data['records'][name])
        self.assertEqual([size(n) for n in ('PhysicsHeader','PhysicsQuery','PhysicsRow','PhysicsEvent','PhysicsEventHeader')],[16,112,96,40,32])
