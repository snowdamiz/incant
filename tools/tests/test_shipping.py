"""Parser/guard fixtures only; live desktop artifacts are verified by shipping.py."""
from pathlib import Path
import struct
import sys
import unittest
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import shipping
import shipping_symbols as symbols


GUID = uuid.UUID('00112233-4455-6677-8899-aabbccddeeff')


def pdb_fixture():
    data = bytearray(6 * 512)
    data[:32] = b'Microsoft C/C++ MSF 7.00\r\n\x1aDS\0\0\0'
    struct.pack_into('<6I', data, 32, 512, 0, 6, 36, 0, 2)
    struct.pack_into('<I', data, 2 * 512, 1)
    struct.pack_into('<9I', data, 512, 5, 0, 28, 0, 80, 4, 3, 4, 5)
    struct.pack_into('<III', data, 3 * 512, 20000404, 0, 1)
    data[3 * 512 + 12:3 * 512 + 28] = GUID.bytes_le
    struct.pack_into('<iII', data, 4 * 512, -1, 19990903, 1)
    struct.pack_into('<H', data, 4 * 512 + 20, 4)
    struct.pack_into('<4i', data, 4 * 512 + 24, 8, 0, 0, 8)
    data[5 * 512:5 * 512 + 4] = b'SYMB'
    return data


def pe_fixture():
    data = bytearray(1024)
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 0x3c, 64)
    data[64:68] = b'PE\0\0'
    struct.pack_into('<H', data, 70, 1)
    struct.pack_into('<H', data, 84, 240)
    struct.pack_into('<H', data, 88, 0x20b)
    struct.pack_into('<II', data, 88 + 112 + 6 * 8, 0x1000, 28)
    struct.pack_into('<IIII', data, 88 + 240 + 8, 512, 0x1000, 512, 512)
    struct.pack_into('<IIII', data, 512 + 12, 2, 38, 0x1030, 560)
    data[560:584] = b'RSDS' + GUID.bytes_le + struct.pack('<I', 1)
    data[584:598] = b'run_scene.pdb\0'
    return data


class ShippingSymbols(unittest.TestCase):
    def test_pe_and_pdb_report_matching_guid_and_age(self):
        expected = {'guid': str(GUID), 'age': 1}
        self.assertEqual(symbols.pe_identity(pe_fixture()), expected)
        self.assertEqual(symbols.pdb_identity(pdb_fixture()), expected)
        changed = pdb_fixture()
        changed[3 * 512 + 12] ^= 1
        self.assertNotEqual(symbols.pdb_identity(changed), symbols.pe_identity(pe_fixture()))

    def test_pdb_rejects_missing_module_information_and_truncated_blocks(self):
        empty = pdb_fixture()
        struct.pack_into('<i', empty, 4 * 512 + 24, 0)
        with self.assertRaisesRegex(ValueError, 'no module'):
            symbols.pdb_identity(empty)
        with self.assertRaisesRegex(ValueError, 'Truncated'):
            symbols.pdb_identity(pdb_fixture()[:-1])
        invalid = pdb_fixture()
        struct.pack_into('<I', invalid, 2 * 512, 100)
        with self.assertRaisesRegex(ValueError, 'out of range'):
            symbols.pdb_identity(invalid)

    def test_pe_rejects_unbacked_debug_directory_and_unknown_codeview(self):
        invalid = pe_fixture()
        struct.pack_into('<I', invalid, 88 + 112 + 6 * 8, 0x9000)
        with self.assertRaisesRegex(ValueError, 'outside'):
            symbols.pe_identity(invalid)
        invalid = pe_fixture()
        invalid[560:564] = b'NB10'
        with self.assertRaisesRegex(ValueError, 'Unsupported CodeView'):
            symbols.pe_identity(invalid)

    def test_platform_identity_readers_reject_missing_or_ambiguous_ids(self):
        self.assertEqual(symbols.mach_uuids(f'UUID: {str(GUID).upper()} (arm64) run_scene'), {'arm64': str(GUID)})
        self.assertEqual(symbols.elf_build_id('Build ID: aAbB1234'), 'aabb1234')
        for text in ('', 'Build ID: aa\nBuild ID: bb'):
            with self.assertRaises(ValueError):
                symbols.elf_build_id(text)
        with self.assertRaises(ValueError):
            symbols.mach_uuids('no UUID in this artifact')

    def test_unsupported_targets_fail_instead_of_claiming_split_symbols(self):
        for target in ('wasm32-unknown-unknown', 'aarch64-linux-android', 'aarch64-apple-ios', 'x86_64-pc-windows-gnu'):
            with self.subTest(target=target), self.assertRaisesRegex(ValueError, 'Unsupported'):
                symbols.backend(target)
        self.assertEqual(symbols.backend('aarch64-apple-darwin'), 'dsym')
        self.assertEqual(symbols.backend('x86_64-unknown-linux-gnu'), 'dwarf')
        self.assertEqual(symbols.backend('x86_64-pc-windows-msvc'), 'pdb')

    def test_empty_debug_sections_do_not_count_as_symbols(self):
        sections = 'Name: .debug_info (1)\n  Size: 0\nName: .debug_line (2)\n  Size: 0x20\n'
        self.assertEqual(symbols.section_size(sections, '.debug_info'), 0)
        self.assertEqual(symbols.section_size(sections, '.debug_line'), 32)
        self.assertEqual(symbols.section_size(sections, '.debug'), 0)


class ShippingCompilerGuard(unittest.TestCase):
    target = 'aarch64-apple-darwin'

    def commands(self, extra=''):
        flags = '-C opt-level=3 -C codegen-units=1 -C panic=abort -C debuginfo=2 -C split-debuginfo=packed'
        return '\n'.join(f'Running `rustc --crate-name {crate} --target {self.target} {flags} -C lto=fat {extra}`'
                         for crate in ('incant_runtime', 'run_scene'))

    def test_reads_flags_from_actual_target_commands(self):
        result = shipping.verify_commands(self.commands(), self.target, 'packed')
        self.assertEqual(result['run_scene']['lto'], 'fat')
        self.assertEqual(result['incant_runtime']['panic'], 'abort')

    def test_overrides_test_harnesses_and_native_cpu_cannot_pass(self):
        for extra in ('-C panic=unwind', '-C opt-level=0', '-C lto=thin', '-C codegen-units=16',
                      '-C target-cpu=native', '--test'):
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                shipping.verify_commands(self.commands(extra), self.target, 'packed')

    def test_stale_or_missing_build_cannot_pass_without_compiler_evidence(self):
        with self.assertRaisesRegex(ValueError, 'fresh target rustc'):
            shipping.verify_commands('Fresh incant_runtime\nFresh run_scene', self.target, 'packed')


if __name__ == '__main__':
    unittest.main()
