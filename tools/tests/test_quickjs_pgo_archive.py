"""Bundled runtime admission controls; inert synthetic bytes, no native tools."""
import copy
from pathlib import Path
import shlex
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import quickjs_pgo as p
from quickjs_pgo_archive import archive_member, bundled_payload, runtime_archive
from tools.tests import test_quickjs_pgo as fixtures


class Archive(unittest.TestCase):
    setUp = fixtures.Protocol.setUp
    runtime_fixture = fixtures.Protocol.runtime_fixture

    def link(self):
        runtime, messages, owner = self.runtime_fixture()
        temporary = owner.parent/'rustcABC123'
        return [str(self.compiler), '-o', str(self.target/'binary'), str(temporary/owner.name),
                '-Wl,-force_load', '-Wl,'+str(temporary/Path(runtime['path']).name)]

    def admit(self, tokens, messages=None):
        runtime, actual, _ = self.runtime_fixture()
        return p.final_link(shlex.join(tokens), 'generate', runtime, compiler=str(self.compiler),
                            target=self.target, artifacts=actual if messages is None else messages, root=self.root)

    def test_exact_pinned_payload_is_bound_to_actual_runtime_rlib_and_unique_extraction(self):
        runtime, messages, owner = self.runtime_fixture()
        result = self.admit(self.link())['bundled_runtime']
        self.assertEqual(result['runtime_rlib'], p.file_identity(owner))
        self.assertEqual(result['pinned_archive'], runtime)
        self.assertFalse(Path(result['force_load_operand']).exists(), 'rustc retires this temporary output')
        # The host archive is a different role, even with the same package name.
        host = copy.deepcopy(messages[0]);host['filenames'] = [str(self.target/'shipping/deps'/owner.name)]
        self.admit(self.link(), [host, *messages])

    def test_substituted_archive_payload_and_changed_pinned_input_refuse(self):
        runtime, _, owner = self.runtime_fixture()
        raw = owner.read_bytes();needle = Path(runtime['path']).read_bytes()
        self.assertEqual(raw.count(needle), 1)
        owner.write_bytes(raw.replace(needle, b'X'+needle[1:]))
        with self.assertRaisesRegex(ValueError, 'payload differs'):
            self.admit(self.link())
        owner.write_bytes(raw)
        Path(runtime['path']).write_bytes(b'changed pinned archive')
        with self.assertRaisesRegex(ValueError, 'Pinned profile runtime changed'):
            self.admit(self.link())

    def test_runtime_artifact_wrong_owner_role_foreign_filename_or_ambiguity_refuse(self):
        _, messages, owner = self.runtime_fixture()
        for mutate in (lambda rows: rows.clear(), lambda rows: rows.append(copy.deepcopy(rows[0])),
                       lambda rows: rows[0].update(package_id=rows[0]['package_id'].replace('/third_party/', '/foreign/')),
                       lambda rows: rows[0].update(features=['diagnostic-quickjs-use']),
                       lambda rows: rows[0].update(filenames=[str(self.target/'shipping/deps'/owner.name)]),
                       lambda rows: rows[0].update(filenames=[str(owner), '/foreign/lib.rmeta'])):
            bad = copy.deepcopy(messages);mutate(bad)
            with self.subTest(messages=bad), self.assertRaises(ValueError):
                self.admit(self.link(), bad)

    def test_foreign_temp_path_unrelated_rlib_missing_or_duplicate_force_load_refuse(self):
        tokens = self.link();operand = tokens[-1];owner = tokens[3]
        for bad in (tokens[:-1], tokens[:-2], tokens+tokens[-2:], tokens+[operand],
                    [v.replace('rustcABC123', 'unrelated') for v in tokens],
                    [v.replace('rustcABC123', 'rustcABC123/..') for v in tokens],
                    [*tokens[:-1], operand.replace(str(self.target), '/foreign')],
                    [*tokens[:3], owner.replace('0123456789abcdef', 'fedcba9876543210'), *tokens[4:]],
                    [*tokens[:3], owner.replace('rustcABC123', 'rustcXYZ789'), *tokens[4:]],
                    [*tokens[:3], owner, *tokens[3:]]):
            with self.subTest(tokens=bad), self.assertRaises(ValueError):
                self.admit(bad)

    def test_malformed_truncated_duplicate_or_unbounded_archive_containers_refuse(self):
        _, _, owner = self.runtime_fixture()
        raw = owner.read_bytes();member = archive_member(raw, 'libclang_rt.profile_osx.a')
        for bad in (b'!<thin>\n'+raw[8:], raw[:-1], raw+raw[8:], raw[:56]+b'9999999999'+raw[66:]):
            with self.subTest(archive=bad[:16]), self.assertRaises(ValueError):
                archive_member(bad, 'libclang_rt.profile_osx.a')
        for bad in (member[:31], member[:16]+struct.pack('<I', 1000)+member[20:],
                    member[:4]+struct.pack('<I', 0x1000007)+member[8:],
                    member.replace(b'.bundled_lib', b'.substitute'), member[:-32], member+b'nonzero tail'):
            with self.subTest(object=bad[:24]), self.assertRaises(ValueError):
                bundled_payload(bad)

    def test_linked_or_oversized_rlib_and_unexpected_runtime_are_not_inspected(self):
        runtime, messages, owner = self.runtime_fixture();original = owner.read_bytes()
        backing = owner.with_suffix('.original');backing.write_bytes(original);owner.unlink();owner.symlink_to(backing)
        with self.assertRaisesRegex(ValueError, 'Linked package input'):
            runtime_archive(messages, runtime, self.target, self.root, p.TARGET)
        owner.unlink()
        with owner.open('wb') as stream:stream.truncate(64*1024**2+1)
        with self.assertRaisesRegex(ValueError, 'oversized package input'):
            runtime_archive(messages, runtime, self.target, self.root, p.TARGET)


if __name__ == '__main__':
    unittest.main()
