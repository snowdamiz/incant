import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import physics_export
import export_native


class PhysicsExport(unittest.TestCase):
    def test_selected_runtime_graph_rejects_authoring_and_malformed_input(self):
        graph = 'incant_game_player v0.0.1 (/engine/player)|project\nincant_runtime v0.0.1|\n'
        self.assertEqual(export_native.normal_dependencies(graph), ['incant_game_player', 'incant_runtime'])
        self.assertEqual(export_native.normal_dependencies(graph + 'incant_runtime v0.0.1| (*)\n'),
                         ['incant_game_player', 'incant_runtime'])
        for name in ('incant_doc', 'incant_wasm_dev', 'wasmtime', 'cranelift-codegen'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'Authoring/JIT'):
                export_native.normal_dependencies(graph + name + ' v1.0.0|\n')
        for invalid in ('', 'incant_runtime v0.0.1|\n', graph + 'unrecognized output\n'):
            with self.assertRaises(ValueError):
                export_native.normal_dependencies(invalid)

    def test_legacy_project_preserves_deterministic_semantics(self):
        self.assertEqual(physics_export.project_mode(b'{"settings":{}}'), "deterministic")
        for mode in physics_export.MODES:
            self.assertEqual(physics_export.project_mode(json.dumps({"settings":{"physics_mode":mode}})), mode)

    def test_unrecognized_mode_and_missing_settings_fail_closed(self):
        for value in (None, True, 0, "Fast", "fast-simd8", "--features=anything", [], {}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                physics_export.project_mode(json.dumps({"settings":{"physics_mode":value}}))
        for value in ({}, {"settings":[]}, []):
            with self.subTest(value=value), self.assertRaises(ValueError):
                physics_export.project_mode(json.dumps(value))

    def test_fixed_feature_selection_and_cooker_receipt_must_agree(self):
        self.assertEqual(physics_export.cargo_features("fast", ["project"]), ["project"])
        self.assertEqual(physics_export.cargo_features("deterministic", ["project"]),
                         ["project", "physics-deterministic"])
        for mode in physics_export.MODES:
            physics_export.verify_producer({"physics_mode":mode,"physics_variant":mode}, mode)
            for variant in (None, "fast_simd8", "other"):
                with self.subTest(mode=mode,variant=variant), self.assertRaises(ValueError):
                    physics_export.verify_producer({"physics_mode":mode,"physics_variant":variant}, mode)
        with self.assertRaises(ValueError):
            physics_export.verify_producer({"physics_mode":"fast","physics_variant":"fast"}, "deterministic")


if __name__ == '__main__':
    unittest.main()
