import copy
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import android_archive as archive
import export_android as exporter


def ndk_paths():
    return {name: Path('/reviewed-ndk') / name for name in
            ('clang', 'clangxx', 'ar', 'llvm', 'sysroot', 'resource')}


class AndroidExport(unittest.TestCase):
    def test_closed_target_and_host_are_checked_before_project_io(self):
        for target in ('x86_64-linux-android', 'armv7-linux-androideabi', 'aarch64-apple-ios', archive.TARGET + '\n'):
            with self.subTest(target=target), self.assertRaises(ValueError):
                exporter.export(Path('/absent/project'), Path('/absent/output'), target, Path('/absent/ndk'))
        for host in ('x86_64-apple-darwin', 'aarch64-unknown-linux-gnu', 'aarch64-linux-android'):
            with self.assertRaises(ValueError):
                archive.admit(host, archive.TARGET)
        archive.admit(archive.PRODUCER, archive.TARGET)

    def test_wasm_and_invalid_mode_refuse_before_compilation_and_no_clobber(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            project, output = root / 'project', root / 'output'
            cases = [({'physics_mode': 'fast-simd8'}, {}, 'Unsupported project physics'),
                     ({}, {'program': {'kind': 'wasm', 'module': 'id'}}, 'static Wasm')]
            for settings, scene, error in cases:
                project.write_text(json.dumps({'settings': settings, 'scenes': {'scene': scene}}))
                with patch.object(exporter, 'build') as build, self.assertRaisesRegex(ValueError, error):
                    exporter.export(project, output, archive.TARGET, root / 'ndk')
                build.assert_not_called()
                self.assertFalse(output.exists())
            output.mkdir()
            (output / 'sentinel').write_bytes(b'preserved')
            with self.assertRaisesRegex(ValueError, 'Output already exists'):
                exporter.export(project, output, archive.TARGET, root / 'ndk')
            self.assertEqual((output / 'sentinel').read_bytes(), b'preserved')

    def test_both_project_modes_reach_cargo_with_only_the_selected_fixed_features(self):
        for mode in ('deterministic', 'fast'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                project = root / 'project'
                project.write_text(json.dumps({'settings': {'physics_mode': mode}, 'scenes': {'scene': {}}}))
                flags = 'parallel,enhanced-determinism' if mode == 'deterministic' else 'parallel'
                graph = 'incant_game_player v0.0.1|mobile-ffi,project,render-input\nincant_script v0.0.1|\n'
                graph += '\n'.join(f'{name} v1.0.0|{flags}' for name in ('rapier2d', 'rapier3d')) + '\n'
                graph += '\n'.join(f'{name} v1.0.0|rayon,{flags}' for name in ('parry2d', 'parry3d')) + '\n'
                graph += 'simba v1.0.0|' + ('libm_force' if mode == 'deterministic' else '') + '\n'
                commands = []
                class Audit:
                    def __init__(self, _):
                        pass
                    def execute(self, argv, **_):
                        argv = list(map(str, argv)); commands.append(argv)
                        result = graph if 'tree' in argv else '/trusted/rustc\n' if argv[0] == 'rustup' else (
                            'host: aarch64-apple-darwin\n' if '-vV' in argv else 'clang version 19.0.1\n')
                        return SimpleNamespace(stdout=result)
                with patch.object(exporter, 'CommandAudit', Audit), patch.object(archive, 'toolchain', return_value=ndk_paths()), \
                     patch.dict(exporter.os.environ, {'PATH': '/normal'}, clear=True), \
                     patch.object(exporter.native_process, 'run', side_effect=RuntimeError('reached target Cargo')) as run:
                    with self.assertRaisesRegex(RuntimeError, 'reached target Cargo'):
                        exporter.build(project, root, root / 'storage', archive.TARGET, None, None, root / 'ndk')
                argv = run.call_args.args[0]
                selected = argv[argv.index('--features') + 1].split(',')
                self.assertEqual(selected, archive.CAPABILITIES + (['physics-deterministic'] if mode == 'deterministic' else []))
                self.assertIn('--no-default-features', argv)
                self.assertNotIn('probe-android-bytecode', ','.join(selected))
                self.assertIn('-e', commands[-1])
                self.assertIn('normal', commands[-1])

    def test_target_toolchain_environment_is_explicit_and_rejects_overrides(self):
        for key in ('CFLAGS', 'TARGET_CFLAGS', 'BINDGEN_EXTRA_CLANG_ARGS', 'CC', 'CC_aarch64_linux_android',
                    'RUSTFLAGS', 'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER', 'CARGO_PROFILE_SHIPPING_PANIC'):
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'Unset'):
                archive.environment({key: 'unreviewed'}, Path('/trusted/rustc'), Path('/project'), None, ndk_paths())
        env, selected = archive.environment({'PATH': '/normal', 'SDKROOT': '/apple', 'INCANT_WASM_BUNDLE': '/wrong',
            'INCANT_NATIVE_SCRIPT_SOURCE': '/cache'}, Path('/trusted/rustc'), Path('/project'), 'scene', ndk_paths())
        self.assertNotIn('SDKROOT', env)
        self.assertNotIn('INCANT_WASM_BUNDLE', env)
        self.assertNotIn('INCANT_NATIVE_SCRIPT_SOURCE', env)
        self.assertEqual(selected['INCANT_MOBILE_PROJECT_SCENE'], 'scene')
        self.assertIn('--sysroot=/reviewed-ndk/sysroot', selected['BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android'])
        self.assertEqual(env['PATH'], '/trusted:/normal')

    def test_runtime_graph_and_codegen_refuse_compiler_probe_and_wrong_target(self):
        graph = 'incant_game_player v0.0.1|project\nincant_script v0.0.1|\n'
        archive.normal_dependencies(graph)
        for feature in ('compiler', 'probe-android-bytecode'):
            with self.assertRaises(ValueError):
                archive.normal_dependencies(graph.replace('incant_script v0.0.1|', 'incant_script v0.0.1|' + feature))
        with self.assertRaises(ValueError):
            archive.normal_dependencies(graph + 'incant_doc v0.0.1|\n')
        flags = '-C opt-level=3 -C codegen-units=1 -C panic=abort -C debuginfo=2 -C split-debuginfo=off'
        text = '\n'.join(f'Running `rustc --crate-name {name} --target {archive.TARGET} {flags}'
                         + (' --crate-type staticlib -C lto=fat' if name == archive.NAME else '') + '`'
                         for name in ('incant_runtime', 'incant_script', archive.NAME))
        archive.verify_codegen(text)
        for bad in (text.replace(archive.TARGET, archive.PRODUCER), text.replace('panic=abort', 'panic=unwind'),
                    text.replace('--crate-name incant_script', '--cfg feature="probe-android-bytecode" --crate-name incant_script')):
            with self.assertRaises(ValueError):
                archive.verify_codegen(bad)

    def test_archive_format_and_abi_are_not_inferred_from_receipt_label(self):
        headers = 'Arch: aarch64\nFormat: elf64-littleaarch64\n'
        archive.archive_platform(headers)
        for bad in ('', headers + 'Arch: x86_64\n', headers.replace('elf64-littleaarch64', 'Mach-O arm64')):
            with self.assertRaises(ValueError):
                archive.archive_platform(bad)
        receipt = {'format': 'incant-native-project-build', 'version': 1, 'target': archive.TARGET,
                   'producer_target': archive.PRODUCER, 'producer_kind': 'android-archive',
                   'physics_mode': 'fast', 'physics_variant': 'fast', 'project_sha256': 'p',
                   'startup_source_parsing': False, 'program': None, 'compiler': {'bundleSha256': 'a'*64, 'mapSha256': 'm'},
                   'script': {'format': 3, 'target': archive.TARGET, 'producer_target': archive.PRODUCER,
                              'engine': 'rquickjs-0.14.0/quickjs-ng-0.16.2/bytecode-28', 'bytecode_abi': archive.ABI,
                              'embedded_source': False, 'build_id': 'b'*64, 'source_sha256': 'a'*64,
                              'bytecode_sha256': 'c'*64, 'hardening_sha256': 'd'*64, 'bindings_sha256': 'e'*64,
                              'source_map_sha256': 'm'}}
        archive.producer_receipt(receipt, 'p', 'fast')
        for path, value in [(('target',), archive.PRODUCER), (('producer_kind',), 'ios-archive'),
                            (('physics_variant',), 'deterministic'), (('compiler', 'bundleSha256'), 'wrong'),
                            (('script', 'bindings_sha256'), None), (('script', 'bytecode_abi', 'value_bytes'), 8)]:
            bad = copy.deepcopy(receipt); parent = bad
            for key in path[:-1]:
                parent = parent[key]
            parent[path[-1]] = value
            with self.subTest(path=path), self.assertRaises(ValueError):
                archive.producer_receipt(bad, 'p', 'fast')

    def test_trusted_tool_symlinks_hash_targets_but_keep_invocation_and_refuse_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            ndk = root / 'ndk'
            ndk.mkdir()
            binary, link = ndk / 'llvm-objcopy', ndk / 'llvm-strip'
            binary.write_bytes(b'actual tool contents')
            link.symlink_to(binary.name)
            tools = {'root': ndk, 'strip': link, 'objcopy': binary}
            expected = archive.sha256(binary.read_bytes())
            self.assertEqual(archive.tool_hashes(tools), {'strip': expected, 'objcopy': expected})
            self.assertEqual(tools['strip'], link)
            self.assertTrue(link.is_symlink())
            with self.assertRaisesRegex(ValueError, 'Linked package input'):
                archive.read(link, 1024)  # Project/package file policy is unchanged.
            outside = root / 'outside'
            outside.write_bytes(b'foreign tool')
            link.unlink()
            link.symlink_to(outside)
            with self.assertRaisesRegex(ValueError, 'NDK tool escapes reviewed root'):
                archive.tool_hashes(tools)

    def test_failed_build_retains_raw_evidence_and_publishes_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            project = root / 'project'
            project.write_text('{"settings":{},"scenes":{"scene":{}}}')
            def fail(project, staging, *args):
                (staging / 'build.log').write_text('actual compiler failure')
                raise ValueError('build failed')
            with patch.object(exporter, 'build', side_effect=fail), self.assertRaisesRegex(ValueError, 'build failed'):
                exporter.export(project, root / 'output', archive.TARGET, root / 'ndk', evidence=root / 'evidence')
            self.assertFalse((root / 'output').exists())
            self.assertEqual((root / 'evidence/build.log').read_text(), 'actual compiler failure')


if __name__ == '__main__':
    unittest.main()
