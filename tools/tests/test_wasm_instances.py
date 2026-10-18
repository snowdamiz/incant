"""Selector/fixture preparation only. No Cargo, compiler, Wasm or player child."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_wasm
spec = importlib.util.spec_from_file_location('instance_guest_preparation', ROOT / 'tools/probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class InstancePreparation(unittest.TestCase):
    def document(self):
        return {'scenes': {'scene': {'program': {'kind': 'wasm', 'module': 'module'},
                'entities': {'owner': {'components': {}}}}},
                'assets': {'module': {'kind': 'wasm', 'sha256': '1' * 64}}}

    def selection(self, document):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp).resolve() / 'project.json'
            path.write_text(json.dumps(document))
            return native_wasm.selection(path, 'scene')

    def test_legacy_and_typed_catalog_use_the_same_module_translation_selection(self):
        original = self.document()
        expected = self.selection(original)
        for props in [{}, {'owner': {'speed': 10, 'nested': [True, None, {'label': '語'}]}}]:
            document = copy.deepcopy(original)
            document['scenes']['scene']['program']['instances'] = props
            self.assertEqual(self.selection(document), expected)
            # Catalog stays in authored data; it never becomes a path or compiler option.
            self.assertEqual(set(expected), {'module', 'fingerprint'})

    def test_selector_refuses_unknown_authority_unbound_owners_and_mixed_scripts(self):
        for change in [
            lambda p: p.update(root='/outside'),
            lambda p: p.update(cache='/outside'),
            lambda p: p.update(instances=None),
            lambda p: p.update(instances=[]),
            lambda p: p.update(instances={'missing': {}}),
            lambda p: p.update(instances={'owner': 42}),
        ]:
            document = self.document()
            change(document['scenes']['scene']['program'])
            with self.assertRaises(ValueError):
                self.selection(document)
        document = self.document()
        document['scenes']['scene']['entities']['owner']['components']['Script'] = {}
        with self.assertRaisesRegex(ValueError, 'cannot mix'):
            self.selection(document)

    def test_all_actual_host_fixture_variants_use_identical_pinned_guest_sources(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(probe.rust_guest, 'compile') as compile:
            root = Path(tmp); guests = root / 'guests'; guests.mkdir()
            rows = probe.instance_guests('pinned-rustc', root, guests, {'fixed': 'environment'})
            names = ['rust-instances'] + ['rust-instances-' + v for v in ('retire', 'scratch', 'callback', 'trap', 'stale')]
            self.assertEqual([name for name, _ in rows], names)
            self.assertEqual(compile.call_count, 6)
            for call, (name, output) in zip(compile.call_args_list, rows):
                self.assertEqual(call.args[0], 'pinned-rustc')
                self.assertEqual(call.args[2], output)
                self.assertEqual(call.kwargs['env'], {'fixed': 'environment'})
                self.assertEqual(call.kwargs['cfg'], () if name == 'rust-instances' else
                                 ['instance_' + name.removeprefix('rust-instances-')])
                self.assertEqual(call.args[1].read_bytes(), (ROOT / 'sdk/rust/samples/instances.rs').read_bytes())


if __name__ == '__main__':
    unittest.main()
