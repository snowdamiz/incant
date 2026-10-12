import copy
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_ios
import ios_archive

TARGET = 'aarch64-apple-ios-sim'


def solver_features(mode):
    common = ['parallel'] + (['enhanced-determinism'] if mode == 'deterministic' else [])
    return {name: (['libm_force'] if mode == 'deterministic' else []) if name == 'simba'
            else common + (['rayon'] if name.startswith('parry') else [])
            for name in ('rapier2d', 'rapier3d', 'parry2d', 'parry3d', 'simba')}


def kernel_commands(mode, target):
    lines = []
    for name, flags in solver_features(mode).items():
        cfg = ' '.join("--cfg 'feature=\"" + flag + "\"'" for flag in flags)
        lines.append(f'Running `rustc --crate-name {name} --target {target} {cfg}`')
    return '\n'.join(lines)


class IosExport(unittest.TestCase):
    def test_actual_tool_failures_retain_raw_streams_and_exit_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            audit = export_ios.CommandAudit(Path(directory).resolve() / 'audit')
            with self.assertRaisesRegex(ValueError, 'failed: refused'):
                audit.execute([sys.executable, '-c', 'import sys;print("observed");print("refused",file=sys.stderr);sys.exit(7)'])
            records = json.loads((audit.directory / 'commands.json').read_text())
            self.assertEqual(records[0]['exit_code'], 7)
            self.assertEqual((audit.directory / records[0]['stdout']).read_text(), 'observed\n')
            self.assertEqual((audit.directory / records[0]['stderr']).read_text(), 'refused\n')
            self.assertEqual(records[0]['stdout_sha256'], ios_archive.sha256(b'observed\n'))

    def test_module_selector_refuses_before_build_and_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / 'project'
            project.write_text(json.dumps({'settings': {}, 'scenes': {'scene': {'program': {'kind': 'wasm', 'module': 'id'}}}}))
            with patch.object(export_ios, 'build') as build, self.assertRaisesRegex(ValueError, 'static Wasm'):
                export_ios.export(project, root / 'output', TARGET)
            build.assert_not_called()
            self.assertFalse((root / 'output').exists())
            (root / 'output').mkdir()
            (root / 'output' / 'sentinel').write_text('preserved')
            with self.assertRaisesRegex(ValueError, 'Output already exists'):
                export_ios.export(project, root / 'output', TARGET)
            self.assertEqual((root / 'output' / 'sentinel').read_text(), 'preserved')

    def test_selected_normal_graph_refuses_runtime_authoring_jit_and_malformed_lines(self):
        graph = 'incant_game_player v0.0.1 (/engine/player)|project\nincant_runtime v0.0.1|\n'
        self.assertEqual(ios_archive.normal_dependencies(graph), ['incant_game_player', 'incant_runtime'])
        self.assertEqual(ios_archive.normal_dependencies(graph + 'incant_runtime v0.0.1| (*)\n'),
                         ['incant_game_player', 'incant_runtime'])
        for name in ('incant_cook', 'incant_doc', 'wasmtime', 'cranelift_codegen'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'Authoring/JIT'):
                ios_archive.normal_dependencies(graph + name + ' v1.0.0|\n')
        for invalid in ('', 'incant_runtime v0.0.1|\n', graph + 'unrecognized output\n'):
            with self.assertRaises(ValueError):
                ios_archive.normal_dependencies(invalid)

    def test_only_fixed_project_families_are_admitted_before_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            project = root / 'project'
            for settings in ({}, {'physics_mode': 'deterministic'}, {'physics_mode': 'fast'}):
                project.write_text(json.dumps({'settings': settings, 'scenes': {'scene': {}}}))
                self.assertEqual(ios_archive.preflight_project(project, None), settings.get('physics_mode', 'deterministic'))
            for mode in ('fast-simd8', 'fast_simd8', None, True, []):
                project.write_text(json.dumps({'settings': {'physics_mode': mode}, 'scenes': {'scene': {}}}))
                with self.subTest(mode=mode), patch.object(export_ios, 'build') as build:
                    with self.assertRaisesRegex(ValueError, 'Unsupported project physics mode'):
                        export_ios.export(project, root / 'output', TARGET)
                    build.assert_not_called()
                    self.assertFalse((root / 'output').exists())

    def test_each_target_selects_only_its_authored_fixed_features_before_cargo(self):
        for target in ios_archive.TARGETS:
            for mode in ('deterministic', 'fast'):
                with self.subTest(target=target, mode=mode), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory).resolve()
                    project = root / 'project'
                    project.write_text(json.dumps({'settings': {'physics_mode': mode}, 'scenes': {'scene': {}}}))
                    graph = 'incant_game_player v0.0.1|mobile-ffi,project,render-input\nincant_script v0.0.1|\n'
                    graph += '\n'.join(f'{name} v1.0.0|' + ','.join(flags) for name, flags in solver_features(mode).items())
                    commands = []
                    class Audit:
                        def __init__(self, _):
                            pass
                        def execute(self, argv, **_):
                            argv = list(map(str, argv)); commands.append(argv)
                            result = graph if 'tree' in argv else '/trusted/rustc\n' if argv[0] == 'rustup' else (
                                'host: aarch64-apple-darwin\n' if '-vV' in argv else '/trusted/sdk\n')
                            return SimpleNamespace(stdout=result)
                    with patch.object(export_ios, 'CommandAudit', Audit), \
                         patch.dict(export_ios.os.environ, {'PATH': '/normal'}, clear=True), \
                         patch.object(export_ios.native_process, 'run', side_effect=RuntimeError('reached Cargo')) as run:
                        with self.assertRaisesRegex(RuntimeError, 'reached Cargo'):
                            export_ios.build(project, root, root / 'storage', target, None, None)
                    argv = run.call_args.args[0]
                    self.assertEqual(argv[argv.index('--target') + 1], target)
                    self.assertEqual(argv[argv.index('--features') + 1].split(','),
                                     ios_archive.CAPABILITIES + (['physics-deterministic'] if mode == 'deterministic' else []))
                    self.assertIn('--no-default-features', argv)
                    self.assertIn('normal', commands[-1])

    def test_closed_destinations_refuse_before_project_or_output_io(self):
        for target in ('aarch64-linux-android', 'aarch64-apple-darwin', 'x86_64-apple-ios', TARGET + '\n'):
            with self.subTest(target=target), self.assertRaises(ValueError):
                export_ios.export(Path('/not-read/project'), Path('/not-created/output'), target)
        for host in ('x86_64-apple-darwin', 'aarch64-unknown-linux-gnu', 'aarch64-apple-ios'):
            with self.subTest(host=host), self.assertRaises(ValueError):
                ios_archive.admit(host, TARGET)
        for target in ios_archive.TARGETS:
            ios_archive.admit('aarch64-apple-darwin', target)

    def test_runtime_overrides_refused_and_executable_selectors_removed(self):
        for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_PROFILE_SHIPPING_PANIC',
                    'CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS', 'CFLAGS', 'HOST_CFLAGS',
                    'CFLAGS_aarch64_apple_ios', 'RUSTC_WRAPPER'):
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'Unset'):
                ios_archive.environment({key: 'bad'}, Path('/trusted/rustc'), Path('/project'), None)
        result = ios_archive.environment({'PATH': '/normal', 'INCANT_NATIVE_SCRIPT_SOURCE': '/cache',
            'INCANT_NATIVE_PROJECT': '/other', 'INCANT_MOBILE_PROJECT_SCENE': 'old',
            'INCANT_WASM_BUNDLE': '/wrong', 'INCANT_AUTHORED_WASM': 'x'}, Path('/trusted/rustc'), Path('/project'), 'selected')
        self.assertEqual(result, {'PATH': '/trusted:/normal', 'INCANT_MOBILE_PROJECT': '/project',
                                  'INCANT_MOBILE_PROJECT_SCENE': 'selected'})

    def test_target_codegen_and_kernel_audits_do_not_accept_host_invocations(self):
        flags = '-C opt-level=3 -C codegen-units=1 -C panic=abort -C debuginfo=2 -C split-debuginfo=off'
        lines = [f'Running `rustc --crate-name {name} --target {TARGET} {flags}'
                 + (' -C lto=fat --crate-type staticlib' if name == 'incant_game_player' else '') + '`'
                 for name in ('incant_runtime', 'incant_script', 'incant_game_player')]
        text = '\n'.join(lines)
        ios_archive.verify_codegen(text, TARGET)
        for bad in (text.replace(TARGET, 'aarch64-apple-darwin'), text.replace('panic=abort', 'panic=unwind'),
                    text + '\n' + lines[0], text.replace('--crate-name incant_script',
                    '--cfg feature="compiler" --crate-name incant_script'), text.replace('--crate-type staticlib', '--crate-type rlib'),
                    text + f'\nRunning `rustc --crate-name incant_doc --target {TARGET}`'):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                ios_archive.verify_codegen(bad, TARGET)
        for mode in ('deterministic', 'fast'):
            kernel = kernel_commands(mode, TARGET)
            self.assertEqual(ios_archive.kernel_provenance(kernel, TARGET, mode)['mode'], mode)
            opposite = 'fast' if mode == 'deterministic' else 'deterministic'
            with self.assertRaises(ValueError):
                ios_archive.kernel_provenance(kernel, TARGET, opposite)
            for bad in (kernel.replace(TARGET, 'aarch64-apple-darwin'), kernel.replace('parallel', 'simd8'),
                        kernel + '\n' + kernel, '\n'.join(kernel.splitlines()[:-1]),
                        kernel.replace('--crate-name parry2d', '--crate-name wrong')):
                with self.subTest(mode=mode, bad=bad), self.assertRaises(ValueError):
                    ios_archive.kernel_provenance(bad, TARGET, mode)

    def test_device_target_cannot_be_satisfied_by_simulator_prefix(self):
        simulator = 'aarch64-apple-ios-sim'
        device = 'aarch64-apple-ios'
        flags = '-C opt-level=3 -C codegen-units=1 -C panic=abort -C debuginfo=2 -C split-debuginfo=off'
        text = '\n'.join(f'Running `rustc --crate-name {name} --target {simulator} {flags}'
                         + (' --crate-type staticlib -C lto=fat' if name == 'incant_game_player' else '') + '`'
                         for name in ('incant_runtime', 'incant_script', 'incant_game_player'))
        with self.assertRaisesRegex(ValueError, 'Missing fresh target invocation'):
            ios_archive.verify_codegen(text, device)
        for mode in ('deterministic', 'fast'):
            with self.assertRaisesRegex(ValueError, 'Missing fresh target physics invocation'):
                ios_archive.kernel_provenance(kernel_commands(mode, simulator), device, mode)

    def test_archive_headers_are_checked_independently_of_receipt_target(self):
        headers = 'Arch: aarch64\n  Platform: iossimulator\n  Version: 15.0\n'
        self.assertEqual(ios_archive.archive_platform(headers, TARGET)['object_minimum_versions'], ['15.0'])
        for bad in (headers.replace('aarch64', 'x86_64'), headers.replace('iossimulator', 'ios'),
                    headers + 'Arch: x86_64\n', headers + '  Platform: macos\n', ''):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                ios_archive.archive_platform(bad, TARGET)

    def test_producer_requires_exact_abi_source_graph_and_binding(self):
        receipt = {'format': 'incant-native-project-build', 'version': 1, 'target': TARGET,
                   'physics_mode': 'deterministic', 'physics_variant': 'deterministic',
                   'producer_target': 'aarch64-apple-darwin', 'producer_kind': 'ios-archive',
                   'project_sha256': 'p', 'startup_source_parsing': False, 'program': None,
                   'compiler': {'bundleSha256': 'a' * 64, 'mapSha256': 'm'},
                   'script': {'format': 3, 'target': TARGET, 'producer_target': 'aarch64-apple-darwin',
                              'engine': 'rquickjs-0.14.0/quickjs-ng-0.16.2/bytecode-28',
                              'bytecode_abi': {'version': 28, 'pointer_bits': 64, 'little_endian': True,
                              'value_bytes': 16, 'value_alignment': 8, 'atom_bytes': 4,
                              'float_mantissa': 53, 'float_max_exponent': 1024, 'write_flags': 17,
                              'default_c_flags': True}, 'embedded_source': False,
                              'build_id': 'b' * 64, 'source_sha256': 'a' * 64, 'bytecode_sha256': 'c' * 64,
                              'hardening_sha256': 'd' * 64, 'bindings_sha256': 'e' * 64, 'source_map_sha256': 'm'}}
        ios_archive.producer_receipt(receipt, TARGET, 'p')
        for mode in ('deterministic', 'fast'):
            matching = copy.deepcopy(receipt)
            matching.update(physics_mode=mode, physics_variant=mode)
            ios_archive.producer_receipt(matching, TARGET, 'p', mode)
            opposite = 'fast' if mode == 'deterministic' else 'deterministic'
            with self.assertRaisesRegex(ValueError, 'selected export kernel'):
                ios_archive.producer_receipt(matching, TARGET, 'p', opposite)
        for path, value in [(('target',), 'aarch64-apple-darwin'), (('program',), {'kind': 'wasm'}),
                            (('physics_mode',), 'fast'), (('physics_variant',), 'fast-simd8'),
                            (('physics_variant',), None),
                            (('script', 'bindings_sha256'), None), (('script', 'target'), 'aarch64-apple-ios'),
                            (('script', 'bytecode_abi', 'pointer_bits'), 32), (('compiler', 'bundleSha256'), 'f' * 64),
                            (('script', 'source_map_sha256'), 'wrong')]:
            bad = copy.deepcopy(receipt)
            parent = bad
            for key in path[:-1]:
                parent = parent[key]
            parent[path[-1]] = value
            with self.subTest(path=path), self.assertRaises(ValueError):
                ios_archive.producer_receipt(bad, TARGET, 'p')

    def test_failed_build_retains_requested_raw_evidence_but_publishes_no_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / 'project'
            project.write_text('{"settings":{},"scenes":{"scene":{}}}')
            evidence = root / 'evidence'
            output = root / 'archive'

            def fail(project, staging, storage, *args):
                (staging / 'build.log').write_text('actual compiler failure')
                (staging / 'build-command.json').write_text(json.dumps({'exit_code': 101}))
                raise ValueError('build failed')

            with patch.object(export_ios, 'build', side_effect=fail), self.assertRaisesRegex(ValueError, 'build failed'):
                export_ios.export(project, output, TARGET, evidence=evidence)
            self.assertFalse(output.exists())
            self.assertEqual((evidence / 'build.log').read_text(), 'actual compiler failure')
            with patch.object(export_ios, 'build') as build, self.assertRaises(ValueError):
                export_ios.export(project, output, TARGET, evidence=evidence)
            build.assert_not_called()


if __name__ == '__main__':
    unittest.main()
