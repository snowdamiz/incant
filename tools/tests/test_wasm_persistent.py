"""Bounded data-only metadata controls; these never execute a Wasm/native tool."""
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from wasm_persistent import declaration, linked_expression

HEADER = b'\0asm\x01\0\0\0'
VALUE = b'INCWST01' + struct.pack('<II', 1, 16) + bytes([7]) * 32 + bytes(8)

def module(value=VALUE):
    body = bytes([len(b'incant_state_v1')]) + b'incant_state_v1' + value
    return HEADER + bytes([0, len(body)]) + body

class PersistentDeclarations(unittest.TestCase):
    def test_opt_in_exact_bounded_record(self):
        self.assertIsNone(declaration(HEADER))
        self.assertEqual(declaration(module()), {'bytes': 16, 'schema_sha256': '07' * 32})
        for offset, replacement in [(0, b'NO'), (8, bytes(4)), (12, bytes(4)),
                                    (12, struct.pack('<I', 65537)), (16, bytes(32)),
                                    (48, b'\1')]:
            bad = VALUE[:offset] + replacement + VALUE[offset + len(replacement):]
            with self.assertRaises(ValueError): declaration(module(bad))

    def test_duplicate_truncation_oversize_and_length_overflow(self):
        for data in [module() + module()[8:], module(VALUE + b'x'), HEADER + b'\0\xff\xff\xff\xff\x10',
                     HEADER + bytes(1024 * 1024)]:
            with self.assertRaises(ValueError): declaration(data)
        for count in range(len(HEADER) + 1, len(module())):
            with self.assertRaises(ValueError): declaration(module()[:count])

    def test_linked_identity_binds_actual_code_policy_and_generated_units(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for suffix in ['.original.wasm', '.wasm']:
                (output / ('m0' + suffix)).write_bytes(module())
            for suffix in ['.c', '.h', '-facade.c']:
                (output / ('m0' + suffix)).write_text('trusted generated unit')
            row = {'prefix': 'm0', 'persistent_state': declaration(module())}
            first = linked_expression(row, output, root)
            self.assertIn('from_linked_persistent', first)
            original = row['persistent_fingerprint']
            (output / 'm0.c').write_text('different actual unit')
            self.assertNotEqual(first, linked_expression(row, output, root))
            self.assertNotEqual(original, row['persistent_fingerprint'])
            (output / 'm0.wasm').write_bytes(HEADER)
            with self.assertRaises(ValueError): linked_expression(row, output, root)

    def test_coroutine_linker_policy_changes_invalidate_the_saved_program(self):
        source = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'bundle'; output.mkdir()
            root = Path(directory) / 'policy'
            for name in ('sdk/abi/native-v1.json', 'crates/incant_wasm_cook/src/lib.rs',
                         'crates/incant_wasm_cook/src/captured.rs', 'tools/wasm2c.py',
                         'tools/wasm_persistent.py', 'tools/wasm_coroutines.py'):
                path = root / name; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((source / name).read_bytes())
            for suffix in ('.original.wasm', '.wasm'):
                (output / ('m0' + suffix)).write_bytes(module())
            for suffix in ('.c', '.h', '-facade.c'):
                (output / ('m0' + suffix)).write_text('actual generated unit')
            row = {'prefix':'m0', 'persistent_state':declaration(module())}
            linked_expression(row, output, root)
            before = row['persistent_fingerprint']
            helper = root / 'tools/wasm_coroutines.py'
            helper.write_bytes(helper.read_bytes() + b'\n# changed linker policy\n')
            linked_expression(row, output, root)
            self.assertNotEqual(before, row['persistent_fingerprint'])
            helper.unlink()
            with self.assertRaises(FileNotFoundError): linked_expression(row, output, root)

if __name__ == '__main__': unittest.main()
