"""Shipping rustc audit regressions; synthetic logs are not export evidence."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native


def invocation(name, kind, host='aarch64-apple-darwin', split='packed'):
    source = 'main.rs' if kind == 'bin' else 'lib.rs'
    return (f"Running `rustc --crate-name {name} --edition=2024 crates/{name}/src/{source} "
            f"--crate-type {kind} --emit=dep-info,link -C opt-level=3 -C panic=abort "
            f"-C codegen-units=1 -C debuginfo=2 -C split-debuginfo={split} "
            + ('-C lto=fat ' if kind == 'bin' else '-C linker-plugin-lto ')
            + f"--cfg 'feature=\"project\"' --target {host} --out-dir target/{host}/shipping/deps`")


def units(host='aarch64-apple-darwin', split='packed'):
    return [invocation(name, kind, host, split) for name, kind in (
        ('incant_runtime', 'lib'), ('incant_script', 'lib'),
        ('incant_game_player', 'lib'), ('incant_game_player', 'bin'))]


class NativeCodegenTests(unittest.TestCase):
    def test_actual_library_and_binary_shape_are_audited_separately(self):
        for host, split in [('aarch64-apple-darwin', 'packed'),
                            ('x86_64-unknown-linux-gnu', 'off'),
                            ('x86_64-pc-windows-msvc', 'packed')]:
            with self.subTest(host=host):
                flags = export_native.verify_codegen('\n'.join(units(host, split)), host, split)
                self.assertEqual(set(flags), {'incant_runtime', 'incant_script',
                                             'incant_game_player_library', 'incant_game_player'})
                self.assertEqual(flags['incant_game_player']['lto'], 'fat')
                self.assertNotIn('lto', flags['incant_game_player_library'])
                for kind in ('incant_game_player', 'incant_game_player_library'):
                    self.assertEqual(flags[kind]['panic'], 'abort')
                    self.assertEqual(flags[kind]['split-debuginfo'], split)

    def test_player_library_and_executable_each_require_shipping_flags(self):
        for unit in (2, 3):
            for before, after in [('-C opt-level=3', '-C opt-level=1'),
                                  ('-C codegen-units=1', '-C codegen-units=16'),
                                  ('-C panic=abort', '-C panic=unwind'),
                                  ('-C debuginfo=2', ''),
                                  ('-C split-debuginfo=packed', '-C split-debuginfo=off'),
                                  ('--emit=dep-info,link', '--emit=dep-info,link -C target-cpu=native')]:
                with self.subTest(unit=unit, flag=before):
                    lines = units(); lines[unit] = lines[unit].replace(before, after)
                    with self.assertRaisesRegex(ValueError, 'Incorrect shipping codegen'):
                        export_native.verify_codegen('\n'.join(lines), 'aarch64-apple-darwin', 'packed')
        lines = units(); lines[3] = lines[3].replace('-C lto=fat', '-C lto=thin')
        with self.assertRaisesRegex(ValueError, 'Incorrect shipping codegen'):
            export_native.verify_codegen('\n'.join(lines), 'aarch64-apple-darwin', 'packed')

    def test_source_compiler_and_test_harness_on_either_player_unit_refuse(self):
        for unit in (2, 3):
            for flag, error in [("--cfg 'feature=\"compiler\"'", 'source compiler'),
                                ('--test', 'Test harness')]:
                with self.subTest(unit=unit, flag=flag):
                    lines = units(); lines[unit] = lines[unit].replace('--emit', flag + ' --emit')
                    with self.assertRaisesRegex(ValueError, error):
                        export_native.verify_codegen('\n'.join(lines), 'aarch64-apple-darwin', 'packed')

    def test_missing_or_duplicate_player_unit_cannot_pass_through_other_kind(self):
        for unit in (2, 3):
            for lines in [units()[:unit] + units()[unit + 1:], units() + [units()[unit]]]:
                with self.subTest(unit=unit, count=len(lines)):
                    with self.assertRaisesRegex(ValueError, 'unique fresh target invocation'):
                        export_native.verify_codegen('\n'.join(lines), 'aarch64-apple-darwin', 'packed')

    def test_host_units_do_not_substitute_for_a_missing_target_library(self):
        host_line = invocation('incant_game_player', 'lib').replace('--target aarch64-apple-darwin ', '')
        flags = export_native.verify_codegen('\n'.join(units() + [host_line]), 'aarch64-apple-darwin', 'packed')
        self.assertIn('incant_game_player_library', flags)
        with self.assertRaisesRegex(ValueError, 'unique fresh target invocation'):
            export_native.verify_codegen('\n'.join(units()[:2] + [host_line, units()[3]]),
                                         'aarch64-apple-darwin', 'packed')

    def test_target_authoring_compile_still_refuses(self):
        for name in ('incant_doc', 'incant_cmd', 'incant_cook'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'Authoring dependency'):
                export_native.verify_codegen('\n'.join(units() + [invocation(name, 'lib')]),
                                             'aarch64-apple-darwin', 'packed')


if __name__ == '__main__':
    unittest.main()
