"""Target, captured identity and C safety regressions for the iOS module route."""
import copy
import json
from pathlib import Path
import shlex
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_ios
import ios_archive
import ios_wasm
import native_wasm


class IosWasm(unittest.TestCase):
    def fixture(self, root):
        return {'bundle': str(root / 'bundle'), 'compiler': {'path': '/trusted/clang'}}

    def commands(self, root, target):
        triple = 'arm64-apple-ios-simulator' if target.endswith('-sim') else 'arm64-apple-ios'
        flags = ['-O3', '-std=c11', '-fno-optimize-sibling-calls', '-frounding-math',
                 '-ffp-contract=off', '-Werror=implicit-function-declaration',
                 '-DWASM_RT_USE_MMAP=0', '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1',
                 '-DWASM_RT_STACK_DEPTH_COUNT=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64',
                 '-DWASM_RT_USE_SEGUE=0', '--target=' + triple, '-isysroot', '/trusted/sdk']
        sources = [root / 'third_party/wabt/wasm-rt-impl.c', root / 'third_party/wabt/wasm-rt-mem-impl.c',
                   root / 'crates/incant_wasm_runtime/c/imports.c', root / 'bundle/m0.c', root / 'bundle/m0-facade.c']
        return ['[incant_wasm_runtime 0.0.1] running: ' + shlex.join([
            'LC_ALL=C', '/trusted/clang', *flags, '-c', str(source), '-o', str(root / f'{i}.o')])
            for i, source in enumerate(sources)]

    def test_actual_c_commands_require_each_distinct_source_and_exact_destination(self):
        with tempfile.TemporaryDirectory(prefix='incant path ') as directory:
            root = Path(directory).resolve()
            for target in ios_archive.TARGETS:
                lines = self.commands(root, target)
                good = '\n'.join(lines)
                audit = ios_wasm.compiler_audit(good, target, '/trusted/sdk', self.fixture(root), root)
                self.assertEqual(len(audit['sources']), 5)
                opposite = 'aarch64-apple-ios' if target.endswith('-sim') else 'aarch64-apple-ios-sim'
                with self.assertRaisesRegex(ValueError, 'destination'):
                    ios_wasm.compiler_audit(good, opposite, '/trusted/sdk', self.fixture(root), root)
                for bad in ('', '\n'.join(lines[:-1]), good + '\n' + lines[0],
                            good.replace('m0-facade.c', 'untrusted.c'),
                            good.replace('/trusted/clang', '/untrusted/clang'),
                            good.replace('/trusted/clang', '/untrusted/clang /trusted/clang'),
                            good.replace('/trusted/sdk', '/other/sdk')):
                    with self.subTest(bad=bad[:100]), self.assertRaises(ValueError):
                        ios_wasm.compiler_audit(bad, target, '/trusted/sdk', self.fixture(root), root)

    def test_safety_and_numerical_flags_cannot_be_missing_or_overridden(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            good = '\n'.join(self.commands(root, 'aarch64-apple-ios-sim'))
            for flag in ('-O3', '-frounding-math', '-ffp-contract=off', '-fno-optimize-sibling-calls',
                         '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64'):
                with self.subTest(flag=flag), self.assertRaises(ValueError):
                    ios_wasm.compiler_audit(good.replace(flag, ''), 'aarch64-apple-ios-sim',
                                            '/trusted/sdk', self.fixture(root), root)
            for extra in ('-O0', '-Ofast', '-ffast-math', '-ffp-contract=fast',
                          '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=0', '-UWASM_RT_STACK_DEPTH_COUNT'):
                with self.subTest(extra=extra), self.assertRaises(ValueError):
                    ios_wasm.compiler_audit(good.replace(' -c ', ' ' + extra + ' -c '),
                                            'aarch64-apple-ios-sim', '/trusted/sdk', self.fixture(root), root)

    def test_exact_complete_program_identity_and_legacy_default(self):
        expected = {'kind': 'wasm', 'module': '00000000000000000000000001',
                    'translator': 'wasm2c-1.0.42', **{key: str(i) * 64 for i, key in enumerate(
                    ('fingerprint', 'input_sha256', 'prepared_sha256', 'registry_sha256'))}}
        ios_wasm.program(expected, expected)
        ios_wasm.program(None, None)
        for key in expected:
            changed = dict(expected)
            changed[key] = 'different'
            with self.subTest(key=key), self.assertRaises(ValueError):
                ios_wasm.program(changed, expected)
        for actual, requested in ((None, expected), (expected, None), ({}, {}),
                                  ({**expected, 'module': '../code'}, {**expected, 'module': '../code'})):
            with self.assertRaises(ValueError):
                ios_wasm.program(actual, requested)

    def test_public_selector_rejects_unknown_mixed_and_malformed_modules_before_any_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            path = root / 'project.json'
            document = {'settings': {}, 'assets': {'m': {'kind': 'wasm', 'sha256': 'a' * 64}},
                        'scenes': {'s': {'entities': {}, 'program': {'kind': 'wasm', 'module': 'm'}}}}
            path.write_text(json.dumps(document))
            self.assertEqual(native_wasm.selection(path, None), {'module': 'm', 'fingerprint': 'a' * 64})
            self.assertEqual(ios_archive.preflight_project(path, None, root / 'trusted-wabt'), 'deterministic')
            variants = []
            mixed = copy.deepcopy(document)
            mixed['scenes']['s']['entities']['e'] = {'components': {'Script': {}}}
            variants.append(mixed)
            for program in ({'kind': 'native', 'module': 'm'}, {'kind': 'wasm', 'module': 'missing'},
                            {'kind': 'wasm', 'module': 'm', 'path': 'code.rs'}):
                bad = copy.deepcopy(document)
                bad['scenes']['s']['program'] = program
                variants.append(bad)
            for bad in variants:
                path.write_text(json.dumps(bad))
                with patch.object(export_ios, 'build') as build, self.assertRaises(ValueError):
                    export_ios.export(path, root / 'output', 'aarch64-apple-ios-sim', wabt=root / 'trusted-wabt')
                build.assert_not_called()
                self.assertFalse((root / 'output').exists())

    def test_ambient_native_compiler_and_flag_overrides_are_not_build_configuration(self):
        for key in ('CC', 'CC_aarch64_apple_ios', 'HOST_CC', 'AR', 'AR_aarch64_apple_ios',
                    'CFLAGS_aarch64_apple_ios', 'CC_ENABLE_DEBUG_OUTPUT'):
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'Unset'):
                ios_archive.environment({key: '/untrusted', 'PATH': '/normal'}, Path('/trusted/rustc'), Path('/project'), None)


if __name__ == '__main__':
    unittest.main()
