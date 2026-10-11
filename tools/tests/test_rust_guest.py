"""Observable fixed-library cache invalidation, without invoking an OS compiler."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('rust_guest', ROOT/'tools/rust_guest.py')
guest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guest)


class GuestCacheTests(unittest.TestCase):
    def test_receipt_covers_the_actual_nested_abi_and_sdk_sources(self):
        paths = guest.provenance()['source_sha256']
        for base in (ROOT/'crates/incant_wasm_abi/src', ROOT/'sdk/rust/src'):
            for source in base.rglob('*.rs'):
                self.assertEqual(paths[source.relative_to(ROOT).as_posix()], hashlib.sha256(source.read_bytes()).hexdigest())

    def test_codec_change_rebuilds_both_libraries_before_reusing_sdk(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            abi = root/'crates/incant_wasm_abi/src'
            sdk = root/'sdk/rust/src'
            abi.mkdir(parents=True)
            sdk.mkdir(parents=True)
            (abi/'lib.rs').write_text('mod localization;')
            (abi/'localization.rs').write_text('first codec')
            (sdk/'lib.rs').write_text('SDK depends on ABI')
            output = root/'guest.wasm'
            builds = []

            def compile(command, **_):
                target = Path(command[command.index('-o') + 1])
                builds.append(target.name)
                target.write_bytes(b'trusted test receipt')
                return SimpleNamespace(returncode=0, stderr='')

            with patch.object(guest, 'ROOT', root), patch.object(guest.subprocess, 'check_output', return_value=b'pinned compiler'), patch.object(guest.subprocess, 'run', side_effect=compile):
                guest.compile(root/'rustc', root/'trusted.rs', output)
                self.assertEqual(builds, ['libincant_wasm_abi.rlib', 'libincant_module.rlib', 'guest.wasm'])
                builds.clear()
                guest.compile(root/'rustc', root/'trusted.rs', output)
                self.assertEqual(builds, ['guest.wasm'])
                builds.clear()
                (abi/'localization.rs').write_text('changed codec, unchanged ABI root')
                guest.compile(root/'rustc', root/'trusted.rs', output)
                self.assertEqual(builds, ['libincant_wasm_abi.rlib', 'libincant_module.rlib', 'guest.wasm'])
                builds.clear()
                (abi/'nested').mkdir()
                (abi/'nested/validation.rs').write_text('new compiled nested source')
                guest.compile(root/'rustc', root/'trusted.rs', output)
                self.assertEqual(builds, ['libincant_wasm_abi.rlib', 'libincant_module.rlib', 'guest.wasm'])


if __name__ == '__main__':
    unittest.main()
