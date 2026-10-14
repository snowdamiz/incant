"""Numerical/identity/lifecycle controls; no engine build, guest run or performance claim."""
import copy
import gzip
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

PROBE = Path(__file__).resolve().parents[1] / 'probes/wasm-ts-cost'
sys.path.insert(0, str(PROBE))

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

common = load('wasm_cost_common', PROBE / 'cost_workload.py')
measure = load('wasm_cost_measure', PROBE / 'measure.py')
rpc = load('wasm_cost_rpc', PROBE / 'cost_rpc.py')


def documents():
    base = {'id': 'project', 'scenes': {'scene': {'entities': {
        common.owner(n): common.entity(n) for n in range(common.COUNT)}}},
        'assets': {}, 'scripts': {}, 'settings': {'tick_rate': 60, 'input_actions': {'right': {'kind': 'button',
            'threshold': 0.5, 'bindings': [{'type': 'key', 'code': 'ArrowRight', 'scale': [1, 0]}]}}}}
    ts, wasm = copy.deepcopy(base), copy.deepcopy(base)
    ts['scripts'][common.SCRIPT] = {'id': common.SCRIPT, 'name': 'Matched transform system', 'path': 'behavior.ts'}
    ts['scenes']['scene']['entities'][common.owner(0)]['components']['Script'] = {'source': common.SCRIPT, 'props': {}}
    ts['scenes']['scene']['entities'][common.owner(0)]['provenance'] = {'transaction': 'changed'}
    wasm['assets']['module'] = {'kind': 'wasm'}
    wasm['scenes']['scene']['program'] = {'kind': 'wasm', 'module': 'module'}
    return ts, wasm


class Workload(unittest.TestCase):
    def test_1024_exact_canonical_ids_and_bounded_pages(self):
        self.assertEqual(len({common.owner(n) for n in range(1024)}), 1024)
        from native_package import ulid_hex
        for n in range(1024): self.assertEqual(int(ulid_hex(common.owner(n)), 16), 10000 + n)
        pages = [common.observation(list(range(i, i + 256))) for i in range(0, 1024, 256)]
        self.assertEqual(sum(len(p['checks']) for p in pages), 1024)
        with self.assertRaises(ValueError): common.observation(list(range(256)), population=True)
        with self.assertRaises(ValueError): common.owner(True)
        with self.assertRaises(ValueError): common.owner(1024)

    def test_input_edges_and_f64_oracle(self):
        self.assertEqual(len(common.recording()['frames']), 12)
        self.assertEqual(common.position(8, 1), [1.1, 1, 1])
        self.assertEqual(common.position(0, 300)[0], common.position(0, 600)[0])
        self.assertAlmostEqual(common.position(0, 3600)[0], 180, places=9)
        self.assertEqual(common.position(1023, 3600)[1:], [3600, 6])
        self.assertEqual(common.position(0, 601)[2], 2)
        with self.assertRaises(ValueError): common.position(0, 3601)

    def test_actual_import_separates_original_prepared_registry_and_fingerprint(self):
        # Actual source002 public import, whose fingerprint differs from input.
        value = {'asset': {'kind': 'wasm', 'path': 'behavior.wasm',
                          'sha256': '9bb7244d1998610976d5221c10c052964624a2fcd3108588c5ab9d719a065816'},
                 'details': {
                     'input_sha256': 'bc82c38d74d52288c6475f39395f779c335c4f576747e3ebd15a2f87702df700',
                     'prepared_sha256': '090eba6cc10b89152ea4161378e2f16a638fd5e23ee43f738dea589683eb2a43',
                     'registry_sha256': '7a1d89e8b3f0e777fa713721376e36ae82ea9bae6311d588055b7a18740fd74d'}}
        original, registry = (value['details'][k] for k in ('input_sha256', 'registry_sha256'))
        identity = common.module_identity(value, original, registry)
        self.assertNotEqual(identity['fingerprint'], original)
        for section, field in (('asset', 'sha256'), ('details', 'input_sha256'),
                               ('details', 'prepared_sha256'), ('details', 'registry_sha256')):
            changed = copy.deepcopy(value); changed[section][field] = 'f' * 64
            with self.subTest(field=field), self.assertRaises(ValueError):
                common.module_identity(changed, original, registry)

    def test_only_exact_backend_registration_delta_admitted(self):
        ts, wasm = documents()
        self.assertEqual(common.validate_documents(ts, wasm), 'scene')
        for change in (
            lambda d: d['settings']['input_actions']['right'].__setitem__('threshold', 0.6),
            lambda d: d['settings']['input_actions']['right']['bindings'][0].__setitem__('scale', [0, 1]),
        ):
            changed = copy.deepcopy(ts); change(changed)
            with self.assertRaises(ValueError): common.validate_documents(changed, wasm)
        for mutate in (
            lambda d: d['scenes']['scene']['entities'].pop(common.owner(99)),
            lambda d: d['scenes']['scene']['entities'][common.owner(3)]['components']['Transform']['translation'].__setitem__(0, 9),
            lambda d: d['settings'].__setitem__('tick_rate', 30),
            lambda d: d['scenes']['scene']['entities'][common.owner(4)].__setitem__('provenance', {'different': True}),
            lambda d: d['scripts'].__setitem__('extra', {}),
        ):
            changed = copy.deepcopy(wasm); mutate(changed)
            with self.subTest(mutate=mutate), self.assertRaises(ValueError): common.validate_documents(ts, changed)

    def test_package_equivalence_refuses_unexplained_feature_data_or_codegen_delta(self):
        left = {'identity': {'data': {'world.scene': 'exact', 'input-actions.json': 'same'}},
                'exported': {'target': 'mac', 'compiler': 'pinned', 'codegen': {'opt': 3},
                    'physics_mode': 'deterministic', 'scene_id': 'same', 'camera': None,
                    'profiling': {'compiler': {'features': {'incant_script': ['profile'],
                        'incant_game_player': ['profile'], 'incant_profile': ['summary']}}}}}
        right = copy.deepcopy(left)
        for name in ('incant_script', 'incant_game_player'):
            right['exported']['profiling']['compiler']['features'][name].append('static-module')
        self.assertEqual(measure.package_equivalence(left, right)['incant_script'], ['static-module'])
        for mutate in (
            lambda d: d['identity']['data'].__setitem__('world.scene', 'changed'),
            lambda d: d['exported']['codegen'].__setitem__('opt', 0),
            lambda d: d['exported']['profiling']['compiler']['features']['incant_script'].append('other'),
            lambda d: d['exported']['profiling']['compiler']['features']['incant_game_player'].remove('static-module'),
        ):
            changed = copy.deepcopy(right); mutate(changed)
            with self.subTest(mutate=mutate), self.assertRaises(ValueError): measure.package_equivalence(left, changed)

    def test_ratio_bounds_are_rational_not_rounded_pass_flags(self):
        value = measure.ratio({'lower': 11, 'upper': 20}, {'lower': 31, 'upper': 40})
        self.assertEqual(value, {'defined': True, 'lower': [11, 40], 'upper': [20, 31]})
        self.assertFalse(measure.ratio({'lower': 0, 'upper': 0}, {'lower': 0, 'upper': 10})['defined'])
        with self.assertRaises(ValueError): measure.ratio({'lower': 20, 'upper': 10}, {'lower': 1, 'upper': 2})

    def test_functional_requires_observed_rows_not_only_pass_boolean(self):
        report = {'tick': common.TICKS, 'frames': 2880000, 'logs': {'entries': [], 'dropped': 0}, 'sources': [], 'flipbooks': [],
                  'observations': {'tick_rate': 60, 'passed': True,
                    'entities': {common.owner(0): {'translation': common.position(0), 'parent': None, 'rotation': [0, 0, 0, 1],
                        'scale': [1, 1, 1], 'velocity': None, 'world_transform': [
                            [1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], common.position(0) + [1]]}}}}
        self.assertEqual(measure.functional(report, [0])['tick'], 3600)
        bad_logs = copy.deepcopy(report); bad_logs['logs']['dropped'] = 1
        with self.assertRaises(ValueError): measure.functional(bad_logs, [0])
        report['observations']['entities'][common.owner(0)]['translation'][0] += .1
        with self.assertRaises(ValueError): measure.functional(report, [0])
        report['observations']['entities'] = {}
        with self.assertRaises(ValueError): measure.functional(report, [0])


@unittest.skipUnless(os.name == 'posix', 'POSIX bounded RPC helper')
class Lifecycle(unittest.TestCase):
    def test_partial_eof_stdout_is_retained_and_child_reaped(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            original = rpc.native_process.Interactive
            child = []
            def start(_argv, **kwargs):
                p = original([sys.executable, '-c',
                    'import sys; sys.stdin.readline(); sys.stdout.write("{partial"); sys.stdout.flush()'], **kwargs)
                child.append(p); return p
            with patch.object(rpc.native_process, 'Interactive', side_effect=start):
                session = rpc.Session('ignored', root / 'p', root / 'j', [], root / 'rpc', timeout=3)
                with self.assertRaisesRegex(ValueError, 'before reply'): session.call('project.read')
            self.assertEqual(gzip.decompress((root / 'rpc.stdout.gz').read_bytes()), b'{partial')
            self.assertIsNotNone(child[0].returncode)
            self.assertTrue(json.loads((root / 'rpc.json').read_text())['owned_process_reaped'])

    def test_timeout_retains_partial_response_and_reaps(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); child = []
            original = rpc.native_process.Interactive
            def start(_argv, **kwargs):
                p = original([sys.executable, '-c',
                    'import sys,time; sys.stdin.readline(); sys.stdout.write("partial"); sys.stdout.flush(); time.sleep(30)'], **kwargs)
                child.append(p); return p
            with patch.object(rpc.native_process, 'Interactive', side_effect=start):
                session = rpc.Session('ignored', root / 'p', root / 'j', [], root / 'rpc', timeout=.2)
                with self.assertRaises(TimeoutError): session.call('project.read')
            self.assertEqual(gzip.decompress((root / 'rpc.stdout.gz').read_bytes()), b'partial')
            self.assertIsNotNone(child[0].returncode)
            self.assertIn('TimeoutError', json.loads((root / 'rpc.json').read_text())['failure'])

    def test_constructor_failure_closes_created_process(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); child = []
            original = rpc.native_process.Interactive
            def start(_argv, **kwargs):
                p = original([sys.executable, '-c', 'import time; time.sleep(30)'], **kwargs)
                child.append(p); return p
            with patch.object(rpc.native_process, 'Interactive', side_effect=start), \
                    patch.object(rpc.os, 'set_blocking', side_effect=OSError('injected')):
                with self.assertRaisesRegex(OSError, 'injected'):
                    rpc.Session('ignored', root / 'p', root / 'j', [], root / 'rpc')
            self.assertIsNotNone(child[0].returncode)
            self.assertTrue(json.loads((root / 'rpc.json').read_text())['owned_process_reaped'])


if __name__ == '__main__': unittest.main()
