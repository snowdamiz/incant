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
