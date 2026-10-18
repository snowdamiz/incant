"""Fixed input guest preparation; no compiler, guest or native child is launched."""
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
spec = importlib.util.spec_from_file_location('input_guest_preparation', ROOT / 'tools/probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class InputPreparation(unittest.TestCase):
    def test_actual_host_variants_bind_exact_sources_compiler_environment_and_cfg(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(probe.rust_guest, 'compile') as compile:
            root = Path(tmp)
            guests = root / 'guests'
            guests.mkdir()
            environment = {'PATH': 'fixed-toolchain'}
            rows = probe.input_guests('pinned-rustc', root, guests, environment)
            self.assertEqual([name for name, _ in rows],
                             ['rust-input', 'rust-input-scratch', 'rust-input-callback', 'rust-input-stale'])
            self.assertEqual(compile.call_count, 4)
            self.assertEqual(environment, {'PATH': 'fixed-toolchain'})
            for call, (name, output) in zip(compile.call_args_list, rows):
                selected = None if name == 'rust-input' else name.removeprefix('rust-input-')
                self.assertEqual(call.args[0], 'pinned-rustc')
                self.assertEqual(output, guests / (name + '.wasm'))
                self.assertEqual(call.args[2], output)
                self.assertEqual(call.kwargs['env'], environment)
                self.assertEqual(call.kwargs['cfg'], ['input_' + selected] if selected else ())
                source = 'fixtures/input_failure.rs' if selected else 'samples/input.rs'
                self.assertEqual(call.args[1].read_bytes(), (ROOT / 'sdk/rust' / source).read_bytes())

    def test_failed_guest_compilation_cannot_return_a_partial_module_selection(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(probe.rust_guest, 'compile',
                side_effect=[None, RuntimeError('controlled guest failure')]) as compile:
            root = Path(tmp)
            guests = root / 'guests'
            guests.mkdir()
            with self.assertRaisesRegex(RuntimeError, 'controlled guest failure'):
                probe.input_guests('pinned-rustc', root, guests, {})
            self.assertEqual(compile.call_count, 2)


if __name__ == '__main__':
    unittest.main()
