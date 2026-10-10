import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('pinned_tool', Path(__file__).parents[1] / 'platforms/pinned-tool.py')
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class PinnedToolTests(unittest.TestCase):
    def fake_run(self, version, calls):
        def run(command, **options):
            calls.append(command)
            if command[0] == 'cargo':
                install_root = Path(command[command.index('--root') + 1])
                binary = install_root / 'bin' / command[command.index('--bin') + 1]
                binary.parent.mkdir()
                binary.write_bytes(b'verified executable')
                return subprocess.CompletedProcess(command, 0)
            if not Path(command[0]).is_file():
                raise FileNotFoundError(command[0])
            if Path(command[0]).name == 'cargo-ndk':
                self.assertTrue(options['env']['CARGO'])
            return subprocess.CompletedProcess(command, 0, stdout=version + '\n')
        return run

    def test_correct_cached_version_avoids_installation(self):
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / 'wasm-bindgen'
            binary.write_bytes(b'cached executable')
            calls = []
            with patch.object(TOOL.subprocess, 'run', self.fake_run('wasm-bindgen 0.2.126', calls)):
                self.assertEqual(TOOL.ensure('wasm-bindgen', Path(root)), binary)
            self.assertEqual(len(calls), 1)
            self.assertEqual(binary.read_bytes(), b'cached executable')

    def test_cache_miss_installs_exact_version_and_only_publishes_requested_binary(self):
        for tool, (package, version) in TOOL.TOOLS.items():
            with self.subTest(tool=tool), tempfile.TemporaryDirectory() as root:
                calls = []
                with patch.object(TOOL.subprocess, 'run', self.fake_run(f'{tool} {version}', calls)):
                    binary = TOOL.ensure(tool, Path(root))
                command = next(command for command in calls if command[0] == 'cargo')
                self.assertEqual(command[2], package)
                self.assertEqual(command[command.index('--version') + 1], '=' + version)
                self.assertIn('--locked', command)
                self.assertEqual(list(Path(root).iterdir()), [binary])
                self.assertEqual(binary.read_bytes(), b'verified executable')

    def test_cached_mismatch_falls_back_to_install_and_rechecks_before_replacing(self):
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / 'cargo-ndk'
            binary.write_bytes(b'wrong cached version')
            calls = []
            normal = self.fake_run('cargo-ndk 4.1.2', calls)
            def run(command, **options):
                if command[0] == str(binary):
                    return subprocess.CompletedProcess(command, 0, stdout='cargo-ndk 4.0.0\n')
                return normal(command, **options)
            with patch.object(TOOL.subprocess, 'run', run):
                TOOL.ensure('cargo-ndk', Path(root))
            self.assertTrue(any(command[0] == 'cargo' for command in calls))
            self.assertEqual(binary.read_bytes(), b'verified executable')

    def test_bad_installed_version_or_failed_install_never_publishes(self):
        for fail_install in [False, True]:
            with self.subTest(fail_install=fail_install), tempfile.TemporaryDirectory() as root:
                calls = []
                fake = self.fake_run('wasm-bindgen 0.2.125', calls)
                def run(command, **options):
                    if fail_install and command[0] == 'cargo':
                        raise subprocess.CalledProcessError(1, command)
                    return fake(command, **options)
                with patch.object(TOOL.subprocess, 'run', run):
                    with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                        TOOL.ensure('wasm-bindgen', Path(root))
                self.assertEqual(list(Path(root).iterdir()), [])

    def test_version_output_must_match_exactly_and_unexecutable_cache_is_miss(self):
        for output in ['wasm-bindgen 0.2.126-dev', 'wasm-bindgen 0.2.126\nextra', '0.2.126']:
            with patch.object(TOOL.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, stdout=output)):
                self.assertFalse(TOOL.correct_version(Path('/cache/tool'), 'wasm-bindgen'))
        with patch.object(TOOL.subprocess, 'run', side_effect=PermissionError):
            self.assertFalse(TOOL.correct_version(Path('/cache/tool'), 'wasm-bindgen'))

    def test_cache_key_separates_version_tool_architecture_and_host_abi(self):
        with patch.object(TOOL.platform, 'system', return_value='Linux'), \
             patch.object(TOOL.platform, 'machine', return_value='x86_64') as machine, \
             patch.object(TOOL.platform, 'freedesktop_os_release', return_value={'ID': 'ubuntu', 'VERSION_ID': '24.04'}) as release:
            baseline = TOOL.cache_key('wasm-bindgen')
            self.assertIn('wasm-bindgen-0.2.126-Linux-x86_64-ubuntu-24.04', baseline)
            self.assertNotEqual(baseline, TOOL.cache_key('cargo-ndk'))
            machine.return_value = 'aarch64'
            self.assertNotEqual(baseline, TOOL.cache_key('wasm-bindgen'))
            machine.return_value = 'x86_64'
            release.return_value = {'ID': 'ubuntu', 'VERSION_ID': '26.04'}
            self.assertNotEqual(baseline, TOOL.cache_key('wasm-bindgen'))

    def test_failed_validation_does_not_publish_a_path_to_later_steps(self):
        with tempfile.TemporaryDirectory() as root:
            output = Path(root) / 'github-path'
            output.write_text('existing-path\n')
            with patch.dict(TOOL.os.environ, {'GITHUB_PATH': str(output)}), \
                 patch.object(sys, 'argv', ['pinned-tool.py', 'wasm-bindgen', root]), \
                 patch.object(TOOL, 'ensure', side_effect=ValueError('wrong installed version')):
                with self.assertRaises(ValueError):
                    TOOL.main()
            self.assertEqual(output.read_text(), 'existing-path\n')


if __name__ == '__main__':
    unittest.main()
