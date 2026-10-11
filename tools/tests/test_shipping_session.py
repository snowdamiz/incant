import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from shipping_session import FreshSession, sha


class FreshShippingLedgerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.target = 'aarch64-apple-darwin'
        self.storage = self.root / 'cargo'
        self.output = self.storage / self.target / 'shipping/deps'
        self.output.mkdir(parents=True)
        self.library = self.output / 'libincant_runtime-123.rlib'
        self.library.write_bytes(b'fresh unit')
        self.session = FreshSession.__new__(FreshSession)
        self.session.root = self.root
        self.session.path = self.storage
        self.session.target = self.target
        self.session.split = 'packed'
        self.session.units = {}
        self.session.builds = []
        self.session.inputs = []
        self.session.players = []
        self.session.deleted = False
        self.session.check_frozen = Mock()
        self.artifact = {'reason': 'compiler-artifact', 'package_id': 'path+file:///repo/runtime#0.0.1',
                         'target': {'name': 'incant_runtime', 'kind': ['lib']},
                         'features': ['default'], 'profile': {'opt_level': '3', 'test': False},
                         'filenames': [str(self.library)], 'fresh': False, 'executable': None}
        self.command = (f"Running `rustc --crate-name incant_runtime --target {self.target} "
                        f"--out-dir {self.output} --cfg 'feature=\"default\"' "
                        '-C metadata=abc -C extra-filename=-123 -C opt-level=3 '
                        '-C codegen-units=1 -C panic=abort`')

    def record(self, *, fresh=False, command=None, artifact=None):
        value = copy.deepcopy(artifact or self.artifact)
        value['fresh'] = fresh
        return self.session.record(json.dumps(value), self.command if command is None else command)

    def test_exact_reuse_retains_original_fresh_command_and_identity(self):
        first = self.record()
        self.session.builds.append({'completed': True})
        self.assertEqual(first, self.record(fresh=True, command=''))
        self.assertEqual(self.session.last_units[0]['fresh_build'], 0)
        self.assertTrue(self.session.last_units[0]['reused'])

    def test_unproven_cached_library_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unproven'):
            self.record(fresh=True, command='')

    def test_changed_reused_bytes_are_rejected(self):
        self.record()
        self.library.write_bytes(b'another artifact')
        with self.assertRaisesRegex(ValueError, 'artifact changed'):
            self.record(fresh=True, command='')

    def test_different_feature_identity_cannot_borrow_default_evidence(self):
        self.record()
        altered = dict(self.artifact, features=['compiler'])
        with self.assertRaisesRegex(ValueError, 'Unproven'):
            self.record(fresh=True, command='', artifact=altered)

    def test_wrong_metadata_or_ambiguous_command_is_rejected(self):
        for command in (self.command.replace('extra-filename=-123', 'extra-filename=-456'),
                        self.command + '\n' + self.command):
            with self.subTest(command=command), self.assertRaises(ValueError):
                self.record(command=command)

    def test_missing_output_or_command_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous'):
            self.record(command='')
        self.library.unlink()
        with self.assertRaisesRegex(ValueError, 'files are missing'):
            self.record()

    def test_root_player_must_not_reuse_library_proof(self):
        self.record()
        altered = dict(self.artifact, executable=str(self.library))
        with self.assertRaisesRegex(ValueError, 'root player'):
            self.record(fresh=True, command='', artifact=altered)

    def test_host_compiler_command_cannot_prove_target_runtime(self):
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous'):
            self.record(command=self.command.replace(f'--target {self.target} ', ''))

    def test_preexisting_storage_is_rejected_before_compiler_probe(self):
        with self.assertRaisesRegex(ValueError, 'must not preexist'):
            FreshSession(self.root, self.storage, self.root / 'missing-rustc', self.target, 'packed')

    def test_wrong_profile_rejected_before_running_cargo(self):
        with self.assertRaisesRegex(ValueError, 'build setting'):
            self.session.build(['cargo', '--profile', 'release'], cwd=self.root, env={})

    def test_deletion_barrier_rejects_retained_inputs_and_missing_player(self):
        player = self.root / 'player'
        player.write_bytes(b'packaged')
        self.session.packaged_player(player)
        self.session.builds.append({'completed': True})
        inputs = self.root / 'authoring'
        inputs.mkdir()
        self.session.input_directory(inputs)
        with self.assertRaisesRegex(ValueError, 'input storage remains'):
            self.session.seal()
        inputs.rmdir()
        player.unlink()
        with self.assertRaisesRegex(ValueError, 'player missing'):
            self.session.seal()

    def test_barrier_requires_deletion_and_unchanged_player(self):
        player = self.root / 'player'
        player.write_bytes(b'packaged')
        self.session.packaged_player(player)
        self.session.builds.append({'completed': True})
        with self.assertRaisesRegex(ValueError, 'build storage remains'):
            self.session.assert_ready()
        self.session.seal()
        self.assertFalse(self.storage.exists())
        self.session.assert_ready()
        player.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'player changed'):
            self.session.assert_ready()


if __name__ == '__main__':
    unittest.main()
