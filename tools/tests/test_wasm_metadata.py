"""Actual child-process channel regressions for the static Wasm graph audit."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('static_wasm_probe', ROOT / 'tools/probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class CargoMetadataChannels(unittest.TestCase):
    def execute(self, source):
        return probe.cargo_metadata([sys.executable, '-c', source], cwd=ROOT, env=os.environ.copy())

    def test_stderr_downloads_and_warnings_do_not_corrupt_json(self):
        result = self.execute("import sys; print('[Downloading] target package', file=sys.stderr); "
                              "print('warning: target-specific resolution', file=sys.stderr); "
                              "print('{\"packages\": [], \"resolve\": {\"nodes\": []}}')")
        self.assertEqual(result, {'packages': [], 'resolve': {'nodes': []}})

    def test_nonzero_exit_with_valid_stdout_is_not_a_valid_graph(self):
        with self.assertRaisesRegex(ValueError, 'Cargo metadata failed: resolution refused'):
            self.execute("import sys; print('{}'); print('resolution refused', file=sys.stderr); sys.exit(2)")

    def test_stderr_json_cannot_replace_missing_stdout(self):
        with self.assertRaises(json.JSONDecodeError):
            self.execute("import sys; print('{}', file=sys.stderr)")
