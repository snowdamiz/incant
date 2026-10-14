"""Public build selection routes every WABT command through its managed executor.

Only tool outputs are simulated. The public build/capture/translation preparation
and byte identity checks run normally; no Cargo, WABT or player is launched.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native
import native_process
import wasm2c


class ReachedGraph(Exception):
    """Stop after capture, before selected dependency graph or shipping build."""


class NativeExportWasmRouting(unittest.TestCase):
    def capture(self, *, wasm=True, failure=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            project = root / 'project.json'
            scene = {'entities': {}}
            if wasm:
                scene['program'] = {'kind': 'wasm', 'module': 'module'}
            project.write_text(json.dumps({'settings': {}, 'scenes': {'scene': scene},
                'assets': {'module': {'kind': 'wasm', 'sha256': 'a' * 64}}}))
            storage, staging, wabt = root / 'storage', root / 'staging', root / 'wabt'
            storage.mkdir(); staging.mkdir()
            translator = wabt / ('bin/wasm2c.exe' if os.name == 'nt' else 'bin/wasm2c')
            rustc = root / 'trusted-rust/bin/rustc'
            original, prepared = b'trusted original input', b'validated metered input'
            digest = lambda data: hashlib.sha256(data).hexdigest()
            identity = {'fingerprint': 'a' * 64, 'input_sha256': digest(original),
                        'prepared_sha256': digest(prepared), 'registry_sha256': 'b' * 64}
            calls = []

            def execute(command, **options):
                args = list(map(str, command))
                env = options.get('env')
                calls.append((args, None if env is None else dict(env), options.get('timeout')))
                output = ''
                if args == ['rustup', 'which', 'rustc']:
                    output = str(rustc)
                elif args == [str(rustc), '-vV']:
                    output = 'host: aarch64-apple-darwin\n'
                elif args == [str(rustc), '--print', 'sysroot']:
                    output = str(root / 'sysroot')
                elif args[1] == 'tree':
                    raise ReachedGraph()
                elif args[1] == 'build':
                    self.assertIn('incant_wasm_cook', args)
                    self.assertEqual(options['timeout'], 600)
                elif args[1] == '--capture':
                    captured = Path(args[-1]); captured.mkdir()
                    (captured / 'original.wasm').write_bytes(original)
                    (captured / 'prepared.wasm').write_bytes(prepared)
                    output = json.dumps(identity)
                elif args[0] == str(translator):
                    phase = 'version' if args[1:] == ['--version'] else 'translate'
                    if phase == failure:
                        raise ValueError('controlled WABT ' + phase + ' failure')
                    if phase == 'version':
                        output = '1.0.42\n'
                    else:
                        self.assertEqual(Path(args[1]).read_bytes(), prepared)
                        generated = Path(args[args.index('-o') + 1])
                        generated.write_text('/* simulated trusted translation */\n')
                        generated.with_suffix('.h').write_text('void wasm2c_m0_instantiate(w2c_m0*);\n')
                else:
                    self.fail('Unexpected command: ' + repr(args))
                return subprocess.CompletedProcess(args, 0, output, '')

            expected_error = ValueError if failure else ReachedGraph
            with mock.patch.dict(os.environ, {'PATH': 'trusted-parent-path',
                    'INCANT_NATIVE_SCRIPT_UNTRUSTED': 'must be removed'}, clear=True), \
                    mock.patch.object(export_native, 'execute', side_effect=execute), \
                    mock.patch.object(wasm2c, 'run', side_effect=AssertionError('unmanaged WABT')), \
                    mock.patch.object(native_process, 'run', side_effect=AssertionError('actual process')), \
                    self.assertRaises(expected_error) as caught:
                export_native.build(project, staging, storage, 'scene', None, 'headless', wabt)
            if failure:
                self.assertIn('controlled WABT ' + failure, str(caught.exception))
            translated = [row for row in calls if row[0][0] == str(translator)]
            for _, env, _ in translated:
                self.assertEqual(env['INCANT_NATIVE_PROJECT'], str(project))
                self.assertEqual(env['INCANT_NATIVE_PROJECT_SCENE'], 'scene')
                self.assertNotIn('INCANT_NATIVE_SCRIPT_UNTRUSTED', env)
            manifest = storage / 'wasm-bundle/manifest.json'
            if wasm and not failure:
                actual = json.loads(manifest.read_text())
                self.assertEqual(actual['modules'][0]['input_sha256'], identity['input_sha256'])
                self.assertEqual(actual['modules'][0]['prepared_sha256'], identity['prepared_sha256'])
                self.assertEqual(translated[0][1], translated[1][1])
                graph_env = next(env for args, env, _ in calls if args[1] == 'tree')
                self.assertEqual(graph_env['INCANT_NATIVE_WASM_INPUT'], identity['input_sha256'])
            else:
                self.assertFalse(manifest.exists())
            self.assertFalse((staging / 'export.json').exists())
            self.assertFalse((storage / 'target').exists())
            return calls, translated

    def test_public_export_routes_version_and_translation_with_sanitized_environment(self):
        _, translated = self.capture()
        self.assertEqual(len(translated), 2)
        self.assertEqual(translated[0][0][1:], ['--version'])
        self.assertIn('--disable-simd', translated[1][0])

    def test_wabt_failure_cannot_proceed_to_graph_build_or_publication(self):
        for phase, count in [('version', 1), ('translate', 2)]:
            with self.subTest(phase=phase):
                calls, translated = self.capture(failure=phase)
                self.assertEqual(len(translated), count)
                self.assertFalse(any(args[1] == 'tree' for args, _, _ in calls))

    def test_non_wasm_project_never_launches_translation_or_cooker(self):
        calls, translated = self.capture(wasm=False)
        self.assertEqual(translated, [])
        self.assertFalse(any(args[1] in ('build', '--capture') for args, _, _ in calls))


if __name__ == '__main__':
    unittest.main()
