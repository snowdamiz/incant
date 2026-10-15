import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from shipping_session import FreshSession, sha


class ShippingProbeLoaderTests(unittest.TestCase):
    def test_real_probes_load_with_sibling_helpers_in_a_clean_process(self):
        tools = Path(__file__).resolve().parents[1]
        program = f'''import sys
sys.path.insert(0, {str(tools)!r})
from shipping_suite import load_probe
before = sys.path[:]
for name in ('cooked-script', 'native-scene-audio', 'native-game'):
    probe = load_probe(name)
    assert callable(probe.phases), name
    assert sys.path == before, name
'''
        result = subprocess.run([sys.executable, '-I', '-c', program],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


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
        self.session.retired_player_units = []
        self.session.deleted = False
        self.session.check_frozen = Mock()
        self.artifact = {'reason': 'compiler-artifact', 'package_id': 'path+file:///repo/runtime#0.0.1',
                         'target': {'name': 'incant_runtime', 'kind': ['lib'], 'crate_types': ['lib']},
                         'features': ['default'], 'profile': {'opt_level': '3', 'test': False},
                         'filenames': [str(self.library)], 'fresh': False, 'executable': None}
        self.command = (f"Running `rustc --crate-name incant_runtime --crate-type lib --target {self.target} "
                        f"--out-dir {self.output} --cfg 'feature=\"default\"' "
                        '-C metadata=abc -C extra-filename=-123 -C opt-level=3 '
                        '-C codegen-units=1 -C panic=abort`')

    def record(self, *, fresh=False, command=None, artifact=None):
        value = copy.deepcopy(artifact or self.artifact)
        value['fresh'] = fresh
        return self.session.record(json.dumps(value), self.command if command is None else command)

    def player_units(self):
        library = self.output / 'libincant_game_player-lib.rlib'
        executable = self.output / 'incant_game_player-bin'
        library.write_bytes(b'fresh player library')
        executable.write_bytes(b'fresh player executable')
        lib = dict(self.artifact, target={'name': 'incant_game_player', 'kind': ['lib'],
                   'crate_types': ['lib']}, filenames=[str(library)])
        binary = dict(lib, target={'name': 'incant_game_player', 'kind': ['bin'],
                      'crate_types': ['bin']}, filenames=[str(executable)], executable=str(executable))
        command = self.command.replace('incant_runtime', 'incant_game_player')
        lib_command = command.replace('=-123', '=-lib')
        bin_command = command.replace('--crate-type lib', '--crate-type bin').replace('=-123', '=-bin')
        return lib, binary, lib_command, bin_command

    def test_player_library_and_binary_join_distinct_fresh_commands(self):
        lib, binary, lib_command, bin_command = self.player_units()
        lines = self.session.record(json.dumps(lib) + '\n' + json.dumps(binary),
                                    lib_command + '\n' + bin_command)
        self.assertEqual(lines, [lib_command, bin_command])
        self.assertEqual(len(self.session.units), 2)

    def test_player_library_command_cannot_prove_executable(self):
        _, binary, lib_command, _ = self.player_units()
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous'):
            self.record(artifact=binary, command=lib_command)

    def test_duplicate_player_binary_commands_still_refuse(self):
        _, binary, _, bin_command = self.player_units()
        with self.assertRaisesRegex(ValueError, 'Missing or ambiguous'):
            self.record(artifact=binary, command=bin_command + '\n' + bin_command)

    def packaged_player_units(self):
        lib, binary, lib_command, bin_command = self.player_units()
        identity = 'path+' + (self.root / 'crates/incant_game_player').as_uri() + '#0.0.1'
        lib['package_id'] = binary['package_id'] = identity
        self.session.record(json.dumps(lib) + '\n' + json.dumps(binary),
                            lib_command + '\n' + bin_command)
        self.session.builds.append({'completed': True})
        copy = self.root / 'packaged-player'
        copy.write_bytes(b'packaged and stripped player')
        self.session.packaged_player(copy)
        return lib, binary, lib_command, bin_command

    def test_next_locale_retires_only_player_and_keeps_exact_shared_origin(self):
        original = self.record()
        lib, binary, lib_command, bin_command = self.packaged_player_units()
        self.session.begin_next_player()
        self.assertEqual(len(self.session.retired_player_units), 2)
        retired = copy.deepcopy(self.session.retired_player_units)
        self.assertEqual(len(self.session.units), 1)
        self.assertEqual(self.record(fresh=True, command=''), original)
        self.assertEqual(self.session.last_units[0]['fresh_build'], 0)
        Path(lib['filenames'][0]).write_bytes(b'next locale library')
        Path(binary['executable']).write_bytes(b'next locale executable')
        self.session.record(json.dumps(lib) + '\n' + json.dumps(binary),
                            lib_command + '\n' + bin_command)
        self.assertEqual({row['fresh_build'] for row in self.session.last_units}, {1})
        self.assertEqual(len(self.session.units), 3)
        self.assertEqual(self.session.retired_player_units, retired)
        old_binary = next(row for row in retired if row['identity']['target']['kind'] == ['bin'])
        self.assertNotEqual(old_binary['hashes'][binary['executable']], sha(Path(binary['executable'])))

    def test_next_locale_cannot_reuse_retired_player_library(self):
        lib, _, _, _ = self.packaged_player_units()
        self.session.begin_next_player()
        with self.assertRaisesRegex(ValueError, 'Unproven'):
            self.record(fresh=True, command='', artifact=lib)

    def test_next_locale_does_not_relax_shared_rebuild_or_mutation_refusals(self):
        self.record()
        self.packaged_player_units()
        self.session.begin_next_player()
        with self.assertRaisesRegex(ValueError, 'unexpectedly rebuilt'):
            self.record()
        self.library.write_bytes(b'changed shared artifact')
        with self.assertRaisesRegex(ValueError, 'artifact changed'):
            self.record(fresh=True, command='')

    def test_next_locale_requires_current_package_and_preserves_retired_history(self):
        _, binary, _, _ = self.packaged_player_units()
        before = copy.deepcopy(self.session.units)
        self.session.players[0][0].write_bytes(b'changed package')
        with self.assertRaisesRegex(ValueError, 'Packaged player changed'):
            self.session.begin_next_player()
        self.assertEqual(self.session.units, before)
        self.assertEqual(self.session.retired_player_units, [])

    def test_next_locale_refuses_unregistered_latest_build(self):
        self.packaged_player_units()
        self.session.builds.append({'unpackaged': True})
        with self.assertRaisesRegex(ValueError, 'Package a player'):
            self.session.begin_next_player()

    def test_next_locale_refuses_nonengine_player_identity(self):
        self.packaged_player_units()
        for row in self.session.units.values():
            row['identity']['package_id'] = 'registry+untrusted/player#0.0.1'
        before = copy.deepcopy(self.session.units)
        with self.assertRaisesRegex(ValueError, 'exact engine player'):
            self.session.begin_next_player()
        self.assertEqual(self.session.units, before)

    def test_exact_reuse_retains_original_fresh_command_and_identity(self):
        first = self.record()
        self.session.builds.append({'completed': True})
        self.assertEqual(first, self.record(fresh=True, command=''))
        self.assertEqual(self.session.last_units[0]['fresh_build'], 0)
        self.assertTrue(self.session.last_units[0]['reused'])

    def test_unproven_cached_library_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unproven'):
            self.record(fresh=True, command='')

    def test_rebuilding_same_target_identity_remains_rejected(self):
        self.record()
        with self.assertRaisesRegex(ValueError, 'unexpectedly rebuilt'):
            self.record()

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

    def test_same_crate_name_and_features_join_their_own_artifact_suffix(self):
        second = self.output / 'libincant_runtime-456.rlib'
        second.write_bytes(b'different package version')
        other = dict(self.artifact, package_id='registry+example/runtime#2.0.0',
                     filenames=[str(second)])
        command = self.command + '\n' + self.command.replace('extra-filename=-123', 'extra-filename=-456')
        lines = self.session.record(json.dumps(self.artifact) + '\n' + json.dumps(other), command)
        self.assertEqual(len(lines), 2)
        self.assertIn('extra-filename=-123', lines[0])
        self.assertIn('extra-filename=-456', lines[1])
        other['fresh'] = True
        reused = self.session.record(json.dumps(other), '')
        self.assertEqual(reused, [lines[1]])

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

    def test_multiline_package_description_preserves_target_command(self):
        command = self.command.replace('Running `rustc',
            "Running `CARGO_PKG_DESCRIPTION='Type-level numbers\nwith `inline` text' rustc")
        lines = self.record(command=command)
        self.assertEqual(len(lines), 1)
        self.assertNotIn('CARGO_PKG_DESCRIPTION', lines[0])
        self.assertIn('--crate-name incant_runtime', lines[0])

    def test_preexisting_storage_is_rejected_before_compiler_probe(self):
        with self.assertRaisesRegex(ValueError, 'must not preexist'):
            FreshSession(self.root, self.storage, self.root / 'missing-rustc', self.target, 'packed')

    def test_wrong_profile_rejected_before_running_cargo(self):
        with self.assertRaisesRegex(ValueError, 'build setting'):
            self.session.build(['cargo', '--profile', 'release'], cwd=self.root, env={})

    def test_compiler_overrides_are_rejected_before_running_cargo(self):
        command = ['cargo', '--profile', 'shipping', '--target', self.target,
                   '--target-dir', str(self.storage), '--config',
                   'profile.shipping.split-debuginfo="packed"', '--locked',
                   '--message-format=json', '-vv']
        for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'CARGO_BUILD_RUSTC',
                    'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTC_WRAPPER',
                    'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'):
            with self.subTest(key=key), patch('shipping_session.subprocess.run') as run:
                with self.assertRaisesRegex(ValueError, 'compiler override'):
                    self.session.build(command, cwd=self.root, env={key: 'override'})
                run.assert_not_called()

    def test_shared_build_diagnostics_stay_constant_without_mutating_callers(self):
        command = ['cargo', '--profile', 'shipping', '--target', self.target,
                   '--target-dir', str(self.storage), '--config',
                   'profile.shipping.split-debuginfo="packed"', '--locked',
                   '--message-format=json', '-vv']
        # Model a static SDK caller with C diagnostics and the following script
        # caller without them. Neither caller nor compiler arguments may change.
        environments = [{'CC_ENABLE_DEBUG_OUTPUT': '1', 'PATH': '/pinned', 'CFLAGS': '-O3'},
                        {'PATH': '/pinned', 'CFLAGS': '-O3'}]
        original = copy.deepcopy(environments)
        original_command = command[:]
        self.session.last_units = []
        with patch.object(self.session, 'record', return_value=['fresh proof']), \
                patch('shipping_session.subprocess.run', return_value=
                      subprocess.CompletedProcess(command, 0, '{}', '')) as compile:
            for environment in environments:
                self.session.build(command, cwd=self.root, env=environment)
        self.assertEqual(environments, original)
        self.assertEqual(command, original_command)
        self.assertEqual(len(compile.call_args_list), 2)
        for call in compile.call_args_list:
            self.assertEqual(call.args[0], original_command)
            self.assertEqual(call.kwargs['env'], original[0])
        self.assertEqual([b['diagnostic_environment'] for b in self.session.builds],
                         [{'CC_ENABLE_DEBUG_OUTPUT': '1'}] * 2)

    def test_explicit_managed_executor_keeps_raw_and_audited_commands_separate(self):
        command = ['cargo', '--profile', 'shipping', '--target', self.target,
                   '--target-dir', str(self.storage), '--config',
                   'profile.shipping.split-debuginfo="packed"', '--locked',
                   '--message-format=json', '-vv']
        executor = Mock(return_value=subprocess.CompletedProcess(command, 0, '{"raw":true}', 'raw Cargo command log'))
        self.session.last_units = []
        with patch.object(self.session, 'record', return_value=['fresh-origin command']) as record, \
                patch('shipping_session.subprocess.run') as default:
            result, commands = self.session.build(command, cwd=self.root, env={}, execute=executor)
        default.assert_not_called()
        executor.assert_called_once()
        record.assert_called_once_with('{"raw":true}', 'raw Cargo command log')
        self.assertEqual(result.stderr, 'raw Cargo command log')
        self.assertEqual(commands, 'fresh-origin command')
        self.assertEqual(self.storage.parent.joinpath('build-0.stderr.log').read_text(), 'raw Cargo command log')

    def test_project_identity_change_during_build_cannot_publish_a_ledger_entry(self):
        command = ['cargo', '--profile', 'shipping', '--target', self.target,
                   '--target-dir', str(self.storage), '--config',
                   'profile.shipping.split-debuginfo="packed"', '--locked',
                   '--message-format=json', '-vv']
        project = self.root / 'project.incant'
        project.write_bytes(b'original scene-bound project')

        def execute(*args, **kwargs):
            project.write_bytes(b'changed during build')
            return subprocess.CompletedProcess(command, 0, '{}', '')

        with patch.object(self.session, 'record') as record:
            with self.assertRaisesRegex(ValueError, 'Compile input changed'):
                self.session.build(command, cwd=self.root,
                                   env={'INCANT_NATIVE_PROJECT': str(project)}, execute=execute)
            record.assert_not_called()
        self.assertEqual(self.session.builds, [])

    def test_static_bundle_must_be_disposable_and_without_symlinks(self):
        bundle = self.root / 'authoring/bundle'
        bundle.mkdir(parents=True)
        source = bundle / 'module.c'
        source.write_bytes(b'trusted generated C')
        env = {'INCANT_WASM_BUNDLE': str(bundle)}
        with self.assertRaisesRegex(ValueError, 'registered disposable'):
            self.session.bundle_inputs(env)
        self.session.input_directory(bundle.parent)
        recorded = self.session.bundle_inputs(env)
        self.assertEqual(recorded['files_sha256'], {'module.c': sha(source)})
        (bundle / 'external.c').symlink_to(self.library)
        with self.assertRaisesRegex(ValueError, 'Symlinked static bundle'):
            self.session.bundle_inputs(env)

    def test_static_bundle_mutation_during_build_is_refused(self):
        command = ['cargo', '--profile', 'shipping', '--target', self.target,
                   '--target-dir', str(self.storage), '--config',
                   'profile.shipping.split-debuginfo="packed"', '--locked',
                   '--message-format=json', '-vv']
        for mutation in ('changed', 'added', 'deleted'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory(dir=self.root) as temporary:
                bundle = Path(temporary)
                source = bundle / 'module.c'
                source.write_bytes(b'original C')
                self.session.input_directory(bundle)

                def compile(*args, **kwargs):
                    if mutation == 'changed': source.write_bytes(b'different C')
                    elif mutation == 'added': (bundle / 'extra.h').write_bytes(b'new input')
                    else: source.unlink()
                    return subprocess.CompletedProcess(args[0], 0, '{}', '')

                with patch('shipping_session.subprocess.run', side_effect=compile):
                    with self.assertRaisesRegex(ValueError, 'Static bundle (changed|is empty)'):
                        self.session.build(command, cwd=self.root, env={'INCANT_WASM_BUNDLE': str(bundle)})
                self.assertFalse(self.session.builds)

    def test_source_head_content_and_compiler_changes_are_rejected(self):
        session = self.session
        session.head = b'frozen-head'
        session.sources = {'tracked.rs': 'frozen-bytes'}
        session.rustc = self.root / 'pinned-rustc'
        session.compiler = 'frozen-compiler'
        cases = [('other-head', session.sources, session.compiler, 'HEAD changed'),
                 ('frozen-head', {'tracked.rs': 'changed'}, session.compiler, 'source changed'),
                 ('frozen-head', session.sources, 'other-compiler', 'Compiler changed')]
        for head, sources, compiler, message in cases:
            with self.subTest(message=message), \
                    patch('shipping_session.subprocess.check_output', side_effect=[head.encode(), compiler]), \
                    patch('shipping_session.snapshot', return_value=sources):
                with self.assertRaisesRegex(ValueError, message):
                    FreshSession.check_frozen(session)
        with patch('shipping_session.subprocess.check_output', side_effect=[session.head, session.compiler]), \
                patch('shipping_session.snapshot', return_value=session.sources):
            FreshSession.check_frozen(session)

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
