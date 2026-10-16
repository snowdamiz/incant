"""Captured-input races in trusted static preparation; no guest execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
spec = importlib.util.spec_from_file_location('wasm2c_prepare', TOOLS / 'wasm2c.py')
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class CapturedInputTests(unittest.TestCase):
    def prepare_with_external_change(self, phase, remove=False):
        captured = b'\0asm\x01\0\0\0\0\x09\x08original'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'input.wasm'
            source.write_bytes(captured)
            output = root / 'bundle'
            cook = root / 'cook'
            observed_inputs = []

            def change_external_input():
                if remove:
                    source.unlink()
                else:
                    source.write_bytes(b'replaced external module')

            def run(args):
                # Controlled tool boundaries make the race deterministic. Read
                # the actual supplied file; do not assume its path or contents.
                if args[1:] == ['--version']:
                    return '1.0.42\n'
                if args[0] == cook:
                    if phase == 'before-cook-read':
                        change_external_input()
                    data = Path(args[1]).read_bytes()
                    observed_inputs.append(data)
                    Path(args[2]).write_bytes(data + b'\0\x09\x08prepared')
                    return ''
                if phase == 'after-cook-read':
                    change_external_input()
                module = args[args.index('--module-name') + 1]
                generated = Path(args[args.index('-o') + 1])
                generated.write_text('/* trusted translated output */\n')
                generated.with_suffix('.h').write_text(
                    f'void wasm2c_{module}_instantiate(w2c_{module}*);\n')
                return ''

            with patch.object(generator, 'run', run):
                report = generator.prepare(output, [('game', source)], root / 'wabt', cook)

            self.assertEqual(observed_inputs, [captured])
            self.assertEqual((output / 'm0.original.wasm').read_bytes(), captured)
            self.assertEqual((output / 'm0.wasm').read_bytes(), captured + b'\0\x09\x08prepared')
            self.assertEqual(report, json.loads((output / 'manifest.json').read_text()))
            self.assertEqual(report['modules'][0]['input_sha256'],
                             hashlib.sha256(captured).hexdigest())
            self.assertEqual(report['files']['m0.original.wasm'],
                             report['modules'][0]['input_sha256'])
            self.assertEqual(report['modules'][0]['prepared_sha256'],
                             hashlib.sha256(captured + b'\0\x09\x08prepared').hexdigest())

    def test_replaced_external_input_before_cook_cannot_change_validated_bytes(self):
        self.prepare_with_external_change('before-cook-read')

    def test_replaced_external_input_after_cook_cannot_change_manifest_identity(self):
        self.prepare_with_external_change('after-cook-read')

    def test_removed_external_input_after_capture_is_not_read_again(self):
        self.prepare_with_external_change('before-cook-read', remove=True)


if __name__ == '__main__':
    unittest.main()
