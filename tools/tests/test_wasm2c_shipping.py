import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    'wasm2c_shipping', Path(__file__).resolve().parents[1] / 'probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class StaticShippingTests(unittest.TestCase):
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
