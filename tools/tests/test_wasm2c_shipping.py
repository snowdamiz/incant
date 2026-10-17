import hashlib
import importlib.util
import io
import os
from pathlib import Path
import tarfile
import sys
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    'wasm2c_shipping', Path(__file__).resolve().parents[1] / 'probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class StaticShippingTests(unittest.TestCase):
    def test_localization_conformance_prepares_the_fixed_sample_and_all_controls(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(probe.rust_guest, 'compile') as compile:
            inputs = Path(directory)
            guests = inputs/'sdk-guests'
            guests.mkdir()
            modules = probe.localization_guests('pinned-rustc', inputs, guests, {'fixed': 'environment'})
            self.assertEqual([name for name, _ in modules], ['rust-localization', 'rust-localization-capacity', 'rust-localization-number', 'rust-localization-duplicate', 'rust-localization-callback'])
            self.assertEqual(compile.call_count, 5)
            for call, (name, output) in zip(compile.call_args_list, modules):
                self.assertEqual(call.args[0], 'pinned-rustc')
                self.assertEqual(call.args[2], output)
                self.assertEqual(call.kwargs['env'], {'fixed': 'environment'})
                self.assertEqual(call.kwargs['cfg'], ['sdk_locale_'+name.removeprefix('rust-localization-')] if name != 'rust-localization' else ())
                source = probe.ROOT/('sdk/rust/fixtures/localization_failure.rs' if name != 'rust-localization' else 'sdk/rust/samples/localization.rs')
                self.assertEqual(call.args[1].read_bytes(), source.read_bytes())

    def test_persistent_conformance_uses_actual_sdk_and_startup_refusal_guest(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(probe.rust_guest, 'compile') as compile:
            inputs = Path(directory); guests = inputs / 'guests'; guests.mkdir()
            modules = probe.persistent_guests('pinned-rustc', inputs, guests, {'fixed': 'environment'})
            self.assertEqual([name for name, _ in modules], ['rust-persistent', 'rust-persistent-owner', 'rust-persistent-callback', 'rust-persistent-quota', 'rust-persistent-restart', 'rust-persistent-start'])
            self.assertEqual(compile.call_count, 6)
            for call, (name, output) in zip(compile.call_args_list, modules):
                self.assertEqual(call.args[0], 'pinned-rustc')
                self.assertEqual(call.args[2], output)
                self.assertEqual(call.kwargs['env'], {'fixed': 'environment'})
                suffix = name.removeprefix('rust-persistent-')
                self.assertEqual(call.kwargs['cfg'], () if name == 'rust-persistent' else ['state_' + suffix])
                source = probe.ROOT / ('sdk/rust/fixtures/persistent_start.rs' if suffix == 'start' else 'sdk/rust/samples/persistent_state.rs')
                self.assertEqual(call.args[1].read_bytes(), source.read_bytes())

    def test_windows_archive_extracts_only_verified_named_executables(self):
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode='w:gz') as bundle:
            for name, content in (
                    ('wasm2c.exe', b'translator'), ('wat2wasm.exe', b'assembler'),
                    ('../../../unwanted', b'not extracted')):
                info = tarfile.TarInfo('wabt-1.0.42/bin/' + name)
                info.size = len(content)
                bundle.addfile(info, io.BytesIO(content))
        data = archive.getvalue()
        with tempfile.TemporaryDirectory() as directory, \
                patch.object(probe.platform, 'system', return_value='Windows'), \
                patch.object(probe.platform, 'machine', return_value='AMD64'), \
                patch.dict(probe.ASSETS, {('Windows', 'AMD64'): ('windows-x64', hashlib.sha256(data).hexdigest())}), \
                patch.object(probe.urllib.request, 'urlopen', return_value=io.BytesIO(data)):
            root, _ = probe.provision(Path(directory))
            self.assertEqual({p.name for p in root.rglob('*') if p.is_file()},
                             {'wasm2c.exe', 'wat2wasm.exe'})
            self.assertEqual((root / 'bin/wasm2c.exe').read_bytes(), b'translator')

    def test_mismatched_archive_is_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as directory, \
                patch.object(probe.platform, 'system', return_value='Windows'), \
                patch.object(probe.platform, 'machine', return_value='AMD64'), \
                patch.object(probe.urllib.request, 'urlopen', return_value=io.BytesIO(b'altered')):
            with self.assertRaisesRegex(ValueError, 'checksum'):
                probe.provision(Path(directory))
            self.assertFalse((Path(directory) / 'wabt').exists())

    def test_linker_map_rejects_missing_module_and_compiler_symbols(self):
        good = ' 0001:00001100 wasm2c_m0_instantiate 0000000140002100 f static_wasm:m0.obj'
        probe.verify_module_symbols(good)
        for bad in ('unrelated', good + '\n wasmtime::Engine', good + '\n cranelift_codegen_codegen'):
            with self.assertRaises(ValueError):
                probe.verify_module_symbols(bad)

    def test_msvc_optimized_audit_requires_strict_arithmetic_bounds_and_crt(self):
        flags = '-O2 /TC /std:c11 /experimental:c11atomics /fp:strict -MT -DWASM_RT_MEMCHECK_BOUNDS_CHECK=1 -DWASM_RT_MAX_CALL_STACK_DEPTH=64'
        probe.verify_c_flags(flags, 'pdb', '2')
        for removed in ('/fp:strict', '-MT', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64'):
            with self.assertRaises(ValueError):
                probe.verify_c_flags(flags.replace(removed, ''), 'pdb', '2')
        with self.assertRaises(ValueError):
            probe.verify_c_flags(flags, 'pdb', '0')


class CargoBuildEnvironment(unittest.TestCase):
    def test_actual_child_receives_pinned_path_and_utf8_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'cook.log'
            environment = dict(os.environ, PATH=directory + os.pathsep + os.environ.get('PATH', ''),
                               INCANT_TEST_COMPILER_PATH=directory)
            source = ("import os, sys; "
                      "assert os.environ['PATH'].split(os.pathsep)[0] == os.environ['INCANT_TEST_COMPILER_PATH']; "
                      "sys.stdout.buffer.write('toolchain selected\\n'.encode('utf-8')); "
                      "sys.stderr.buffer.write('diagnostic: café\\n'.encode('utf-8'))")
            probe.cargo_build([sys.executable, '-c', source], cwd=directory, env=environment, log=log)
            self.assertEqual(log.read_text(encoding='utf-8'), 'toolchain selected\n' + 'diagnostic: café\n')

    def test_actual_build_failure_is_reported_and_full_diagnostic_is_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / 'cook.log'
            source = "import sys; print('build refused', file=sys.stderr); sys.exit(7)"
            with self.assertRaisesRegex(ValueError, 'Cargo build failed'):
                probe.cargo_build([sys.executable, '-c', source], cwd=directory,
                                  env=os.environ.copy(), log=log)
            self.assertEqual(log.read_text(encoding='utf-8'), 'build refused\n')
