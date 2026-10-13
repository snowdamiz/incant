"""CPU-only lifecycle and transport controls for the experimental Android probe."""
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[2]
probe = runpy.run_path(str(ROOT / 'tools/probes/android-bytecode/verify.py'))
AndroidProof = probe['AndroidProof']


class AndroidProbeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name)
        self.proof = AndroidProof(self.output, '/unused-adb', 'emulator-5584', '/data/local/tmp/owned')

    def child(self):
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
        self.addCleanup(lambda: child.poll() is None and (child.kill(), child.wait(timeout=10)))
        return child

    def test_failed_adb_kill_reaps_the_actual_owned_child(self):
        child = self.child()
        self.proof.adb_command = Mock(side_effect=['owned\nOK\n', RuntimeError('offline')])
        result = probe['stop_owned_emulator'](self.proof, child, 'owned')
        self.assertTrue(result['fallback'])
        self.assertEqual(result['adb_error'], 'offline')
        self.assertIsNotNone(child.returncode)
        self.assertEqual(self.proof.adb_command.call_args_list[-1].args, ('emu', 'kill'))

    def test_changed_serial_owner_never_receives_kill(self):
        child = self.child()
        self.proof.adb_command = Mock(return_value='some-other-avd\nOK\n')
        result = probe['stop_owned_emulator'](self.proof, child, 'owned')
        self.assertTrue(result['fallback'])
        self.assertIsNotNone(child.returncode)
        self.assertEqual(self.proof.adb_command.call_count, 1)
        self.assertIn('no longer identifies', result['adb_error'])

    def test_exited_owned_process_is_reaped_without_addressing_adb(self):
        child = subprocess.Popen([sys.executable, '-c', 'pass'])
        child.wait(timeout=10)
        self.proof.adb_command = Mock()
        result = probe['stop_owned_emulator'](self.proof, child, 'owned')
        self.assertEqual(result['exit_code'], 0)
        self.proof.adb_command.assert_not_called()

    def test_only_safe_owned_paths_cross_the_shell_boundary(self):
        self.assertEqual(self.proof.remote_path(self.output / 'fixture/world.scene'),
                         '/data/local/tmp/owned/fixture/world.scene')
        for path in [self.output.parent / 'outside', self.output / '../outside',
                     self.output / 'bad;command', self.output / 'with space']:
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.proof.remote_path(path)

    def test_malformed_request_is_preserved_and_does_not_pull_output(self):
        request = self.output / 'bad.request.json'
        request.write_bytes(b'broken JSON')
        self.proof.adb_command = Mock(return_value='{"abi_status":2}')
        self.proof.command(['@candidate-caller', request], expected=2)
        self.assertEqual((self.output / 'wire-000.json').read_bytes(), request.read_bytes())
        calls = [row.args for row in self.proof.adb_command.call_args_list]
        self.assertFalse(any(row[0] == 'pull' for row in calls))

    def test_typed_refusal_maps_known_paths_and_requires_output_absence(self):
        package = self.output / 'package'
        package.mkdir()
        (package / 'world.scene').write_bytes(b'scene')
        request = self.output / 'request.json'
        request.write_text(json.dumps({'package': str(package), 'version': 2,
                                       'output_directory': str(self.output / 'result')}))
        self.proof.adb_command = Mock(return_value='{"abi_status":2}')
        self.proof.command(['@candidate-caller', request], expected=2)
        wire = json.loads((self.output / 'wire-000.json').read_text())
        self.assertEqual(wire['package'], '/data/local/tmp/owned/package')
        self.assertEqual(wire['output_directory'], '/data/local/tmp/owned/result')
        self.assertEqual(wire['version'], 2)
        self.proof.adb_command.assert_any_call('shell', 'test', '-d',
                                              '/data/local/tmp/owned/result', expected=1)

    def test_no_clobber_checks_the_remote_directory_not_only_local_original(self):
        dest = self.output / 'result'
        dest.mkdir()
        (dest / 'result.json').write_bytes(b'original')
        request = self.output / 'request.json'
        request.write_text(json.dumps({'output_directory': str(dest)}))
        def adb(*parts, **_):
            if parts[0] == 'pull':
                pulled = Path(parts[2])
                pulled.mkdir()
                (pulled / 'result.json').write_bytes(b'changed')
            return '{"abi_status":3}'
        self.proof.adb_command = adb
        with self.assertRaisesRegex(AssertionError, 'Create-only remote output changed'):
            self.proof.command(['@candidate-caller', request], expected=3)
        self.assertEqual((dest / 'result.json').read_bytes(), b'original')

    def test_adb_timeout_retains_raw_streams_and_failed_identity(self):
        fake = self.output / 'adb'
        fake.write_text('#!' + sys.executable + '\nimport time\nprint("started", flush=True)\ntime.sleep(60)\n')
        fake.chmod(0o700)
        self.proof.adb = str(fake)
        with self.assertRaises(subprocess.TimeoutExpired):
            self.proof.adb_command('shell', 'getprop', timeout=0.2)
        row = self.proof.commands[0]
        self.assertTrue(row['timed_out'])
        self.assertIsNone(row['exit_code'])
        self.assertEqual(Path(row['stdout']).read_text(), 'started\n')
        self.assertEqual(row['argv'][1:3], ['-s', 'emulator-5584'])

    def test_failed_avd_command_retains_command_and_both_streams(self):
        with self.assertRaisesRegex(RuntimeError, 'avd-create failed'):
            probe['avd_command'](self.output, 'avd-create', [sys.executable, '-c',
                'import sys; print("out"); print("err", file=sys.stderr); sys.exit(7)'], {})
        row = json.loads((self.output / 'avd-create.json').read_text())
        self.assertEqual(row['exit_code'], 7)
        self.assertEqual(Path(row['stdout']).read_text(), 'out\n')
        self.assertEqual(Path(row['stderr']).read_text(), 'err\n')


if __name__ == '__main__':
    unittest.main()
