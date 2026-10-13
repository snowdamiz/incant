"""Closed target/compiler/captured module controls for Android static exports."""
import copy
import json
from pathlib import Path
import shlex
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import android_archive
import android_wasm
import export_android


class AndroidWasm(unittest.TestCase):
    def fixture(self, root):
        return {'bundle': str(root / 'bundle'), 'compiler': {
            'path': '/ndk/bin/aarch64-linux-android24-clang',
            'target': android_archive.TARGET, 'api': 24}}

    def commands(self, root):
        flags = ['-O3', '-std=c11', '-fno-optimize-sibling-calls', '-frounding-math',
                 '-ffp-contract=off', '-Werror=implicit-function-declaration',
                 '-DWASM_RT_USE_MMAP=0', '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1',
                 '-DWASM_RT_STACK_DEPTH_COUNT=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64',
                 '-DWASM_RT_USE_SEGUE=0']
        sources = [root / 'third_party/wabt/wasm-rt-impl.c', root / 'third_party/wabt/wasm-rt-mem-impl.c',
                   root / 'crates/incant_wasm_runtime/c/imports.c', root / 'bundle/m0.c', root / 'bundle/m0-facade.c']
        return ['[incant_wasm_runtime 0.0.1] running: ' + shlex.join([
            'LC_ALL=C', self.fixture(root)['compiler']['path'], *flags, '-c', str(source), '-o', str(root / f'{i}.o')])
            for i, source in enumerate(sources)]

    def test_five_unique_actual_compiles_and_closed_wrapper_destination(self):
        with tempfile.TemporaryDirectory(prefix='incant path ') as directory:
            root = Path(directory).resolve(); target = android_archive.TARGET
            lines = self.commands(root); good = '\n'.join(lines)
            self.assertEqual(len(android_wasm.compiler_audit(good, target, self.fixture(root), root)['sources']), 5)
            for bad in ('', '\n'.join(lines[:-1]), good + '\n' + lines[0],
                        good.replace('m0-facade.c', 'foreign.c'),
                        good.replace('android24-clang', 'android21-clang'),
                        good.replace('/ndk/bin/', '/outside/bin/')):
                with self.subTest(bad=bad[:70]), self.assertRaises(ValueError):
                    android_wasm.compiler_audit(bad, target, self.fixture(root), root)
            for destination in ('aarch64-apple-ios', 'x86_64-linux-android', target + '24'):
                with self.subTest(target=destination), self.assertRaises(ValueError):
                    android_wasm.compiler_audit(good, destination, self.fixture(root), root)
            changed = self.fixture(root); changed['compiler']['api'] = 21
            with self.assertRaises(ValueError):
                android_wasm.compiler_audit(good, target, changed, root)

    def test_numerical_bounds_and_destination_overrides_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve(); target = android_archive.TARGET
            good = '\n'.join(self.commands(root))
            for flag in ('-O3', '-frounding-math', '-ffp-contract=off', '-fno-optimize-sibling-calls',
                         '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64'):
                with self.subTest(flag=flag), self.assertRaises(ValueError):
                    android_wasm.compiler_audit(good.replace(flag, ''), target, self.fixture(root), root)
            for extra in ('-O0', '-Ofast', '-ffast-math', '-ffp-contract=fast',
                          '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=0', '-UWASM_RT_STACK_DEPTH_COUNT',
                          '--target=aarch64-linux-android21', '-target aarch64-linux-android21',
                          '--sysroot=/outside', '-isysroot /outside', '-resource-dir=/outside',
                          '@override.rsp', '-march=armv9-a', '--gcc-toolchain=/outside'):
                with self.subTest(extra=extra), self.assertRaises(ValueError):
                    android_wasm.compiler_audit(good.replace(' -c ', ' ' + extra + ' -c '),
                                                target, self.fixture(root), root)

    def test_only_exact_bound_cc_flag_probe_is_tolerated(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve(); out = root / 'target/out'
            good = '\n'.join(self.commands(root))
            probe = '[incant_wasm_runtime 0.0.1] running: ' + shlex.join([
                self.fixture(root)['compiler']['path'], '-O0', '-mno-omit-leaf-frame-pointer',
                '-o', str(out / 'flag_check'), '-c', str(out / 'flag_check.c')])
            audit = android_wasm.compiler_audit(good + '\n' + probe, android_archive.TARGET, self.fixture(root), root, out)
            self.assertEqual(len(audit['compiler_probes']), 1)
            for bad in (probe + '\n' + probe, probe.replace('-O0', '-O3'),
                        probe.replace(str(out / 'flag_check') + ' -c', str(root / 'foreign') + ' -c'),
                        probe.replace('flag_check.c', 'untrusted.c')):
                with self.subTest(bad=bad), self.assertRaises(ValueError):
                    android_wasm.compiler_audit(good + '\n' + bad, android_archive.TARGET, self.fixture(root), root, out)

    def test_unknown_or_mixed_programs_refuse_before_build_and_publish(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve(); project = root / 'game.json'
            document = {'settings': {}, 'assets': {'m': {'kind': 'wasm', 'sha256': 'a' * 64}},
                        'scenes': {'s': {'entities': {}, 'program': {'kind': 'wasm', 'module': 'm'}}}}
            project.write_text(json.dumps(document))
            self.assertEqual(android_archive.preflight_project(project, None, root / 'wabt'), 'deterministic')
            variants = []
            mixed = copy.deepcopy(document)
            mixed['scenes']['s']['entities']['e'] = {'components': {'Script': {}}}; variants.append(mixed)
            for program in ({'kind': 'native', 'module': 'm'}, {'kind': 'wasm', 'module': 'absent'},
                            {'kind': 'wasm', 'module': 'm', 'path': 'native.c'}):
                bad = copy.deepcopy(document); bad['scenes']['s']['program'] = program; variants.append(bad)
            for bad in variants:
                project.write_text(json.dumps(bad))
                with patch.object(export_android, 'build') as build, self.assertRaises(ValueError):
                    export_android.export(project, root / 'output', android_archive.TARGET, root / 'ndk', wabt=root / 'wabt')
                build.assert_not_called(); self.assertFalse((root / 'output').exists())

    def test_wrapper_predefines_and_actual_driver_trace_cannot_relabel_another_abi(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve(); llvm = root / 'llvm'; (llvm / 'bin').mkdir(parents=True)
            wrapper, driver = llvm / 'bin/aarch64-linux-android24-clang', llvm / 'bin/clang'
            wrapper.write_bytes(b'reviewed wrapper'); driver.write_bytes(b'reviewed driver')
            ndk = {'clang': wrapper, 'llvm': llvm, 'root': root, 'resource': llvm / 'lib/clang/19', 'sysroot': llvm / 'sysroot'}
            values = {'__ANDROID__': '1', '__aarch64__': '1', '__SIZEOF_POINTER__': '8',
                      '__ANDROID_API__': '__ANDROID_MIN_SDK_VERSION__', '__ANDROID_MIN_SDK_VERSION__': '24',
                      '__BYTE_ORDER__': '__ORDER_LITTLE_ENDIAN__', '__ORDER_LITTLE_ENDIAN__': '1234',
                      '__SIZEOF_DOUBLE__': '8', '__DBL_MANT_DIG__': '53', '__DBL_MAX_EXP__': '1024'}
            macros = '\n'.join(f'#define {k} {v}' for k, v in values.items())
            trace = ' '.join(json.dumps(str(x)) for x in [driver, '-cc1', '-triple', 'aarch64-unknown-linux-android24',
                    '-resource-dir', ndk['resource'], '-internal-externc-isystem', ndk['sysroot'] / 'usr/include'])
            def execute(argv):
                return SimpleNamespace(stdout=macros, stderr=trace)
            self.assertEqual(android_wasm.compiler_identity(ndk, execute)['api'], 24)
            valid_macros, valid_trace = macros, trace
            for key in values:
                macros = valid_macros.replace(f'#define {key} {values[key]}', f'#define {key} wrong')
                with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'macros'):
                    android_wasm.compiler_identity(ndk, execute)
            macros = valid_macros
            for trace in ('', valid_trace.replace('android24', 'android21'),
                          valid_trace.replace(str(driver), str(root / 'foreign-clang')),
                          valid_trace.replace(str(ndk['resource']), str(root / 'foreign-resource')),
                          valid_trace.replace(str(ndk['sysroot']), str(root / 'foreign-sysroot'))):
                with self.subTest(trace=trace[:80]), self.assertRaises(ValueError):
                    android_wasm.compiler_identity(ndk, execute)

    def test_module_receipt_keeps_exact_capture_physics_and_script_abi_identity(self):
        module = {'kind': 'wasm', 'module': '00000000000000000000000001', 'translator': 'wasm2c-1.0.42',
                  **{key: str(i) * 64 for i, key in enumerate(('fingerprint', 'input_sha256', 'prepared_sha256', 'registry_sha256'))}}
        receipt = {'format': 'incant-native-project-build', 'version': 1, 'target': android_archive.TARGET,
                   'producer_target': android_archive.PRODUCER, 'producer_kind': 'android-archive',
                   'physics_mode': 'fast', 'physics_variant': 'fast', 'project_sha256': 'p',
                   'startup_source_parsing': False, 'program': module, 'compiler': None,
                   'script': {'format': 3, 'target': android_archive.TARGET, 'producer_target': android_archive.PRODUCER,
                              'engine': 'rquickjs-0.14.0/quickjs-ng-0.16.2/bytecode-28', 'bytecode_abi': android_archive.ABI,
                              'embedded_source': False, 'bindings_sha256': None,
                              **{key: str(i) * 64 for i, key in enumerate(('build_id', 'source_sha256', 'bytecode_sha256', 'hardening_sha256'))}}}
        android_archive.producer_receipt(receipt, 'p', 'fast', module)
        for field in module:
            bad = copy.deepcopy(receipt); bad['program'][field] = 'wrong'
            with self.subTest(field=field), self.assertRaises(ValueError):
                android_archive.producer_receipt(bad, 'p', 'fast', module)
        for fields, value in [(('compiler',), {}), (('target',), 'aarch64-apple-ios'),
                              (('physics_variant',), 'deterministic'), (('script', 'bytecode_abi', 'pointer_bits'), 32)]:
            bad = copy.deepcopy(receipt); obj = bad
            for field in fields[:-1]:
                obj = obj[field]
            obj[fields[-1]] = value
            with self.subTest(fields=fields), self.assertRaises(ValueError):
                android_archive.producer_receipt(bad, 'p', 'fast', module)
        with self.assertRaises(ValueError):
            android_archive.producer_receipt(receipt, 'p', 'fast')


if __name__ == '__main__':
    unittest.main()
