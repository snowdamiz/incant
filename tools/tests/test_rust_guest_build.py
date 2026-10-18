"""Pure capture/policy/worker/publication controls. Never starts an OS child.

Mock compiler/validator outcomes exercise worker decisions, not sandbox safety.
Actual compiler escape controls have a separate, explicitly managed probe.
"""
from dataclasses import replace
import ctypes
import errno
import io
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import rust_guest_build as build
import rust_guest_sandbox as sandbox
import rust_guest_source as source


class Fixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.root = Path(temporary.name).resolve()

        def cleanup():
            for directory, _, _ in os.walk(self.root):
                Path(directory).chmod(0o700)
            temporary.cleanup()
        self.addCleanup(cleanup)
        self.package = self.root / 'package'
        self.package.mkdir()
        (self.package / 'module.rs').write_text('#![no_std]\nmod helper;\n')
        (self.package / 'helper.rs').write_text('pub const VALUE: u32 = 7;\n')

    def write(self, name, data=b'fn module() {}'):
        path = self.package / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path


class SourceTests(Fixture):
    def test_relative_module_bytes_are_immutable_and_bound(self):
        files = source.capture(self.package, 'module.rs')
        self.write('helper.rs', b'changed after capture')
        rows = source.materialize(files, self.root / 'snapshot')
        self.assertEqual((self.root / 'snapshot/helper.rs').read_bytes(), b'pub const VALUE: u32 = 7;\n')
        self.assertEqual({row['path'] for row in rows}, {'module.rs', 'helper.rs'})
        self.assertEqual(rows[0]['sha256'], build.digest(files['helper.rs']))

    def test_manifests_build_scripts_and_native_inputs_refuse(self):
        for name in ('Cargo.toml', 'Cargo.lock', 'build.rs', 'BUILD.rs', 'macro.dylib', 'data.txt'):
            with self.subTest(name=name):
                path = self.write(name)
                with self.assertRaises(sandbox.Refused):
                    source.capture(self.package, 'module.rs')
                path.unlink()

    def test_symlink_hardlink_directory_escape_and_fifo_refuse(self):
        secret = self.root / 'secret.rs'
        secret.write_bytes(b'private data')
        escape = self.package / 'escape.rs'
        escape.symlink_to(secret)
        with self.assertRaises(sandbox.Refused):
            source.capture(self.package, 'module.rs')
        escape.unlink()
        os.link(secret, escape)
        with self.assertRaises(sandbox.Refused):
            source.capture(self.package, 'module.rs')
        escape.unlink()
        link = self.package / 'linked'
        link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(sandbox.Refused):
            source.capture(self.package, 'module.rs')
        link.unlink()
        os.mkfifo(escape)
        with self.assertRaises(sandbox.Refused):
            source.capture(self.package, 'module.rs')

    def test_entry_paths_and_missing_entry_refuse(self):
        for name in ('../module.rs', '/module.rs', './module.rs', 'a//b.rs',
                     'a/../module.rs', 'module.txt', 'build.rs', '', 'a\\b.rs', 'absent.rs'):
            with self.subTest(name=name), self.assertRaises(sandbox.Refused):
                source.capture(self.package, name)

    def test_file_count_file_bytes_total_depth_and_inventory_are_bounded(self):
        large = self.write('large.rs', b'x' * (source.MAX_FILE + 1))
        with self.assertRaisesRegex(sandbox.Refused, 'file bound'):
            source.capture(self.package, 'module.rs')
        large.unlink()
        for index in range(31):
            self.write(f'file{index}.rs')
        with self.assertRaisesRegex(sandbox.Refused, 'file bound'):
            source.capture(self.package, 'module.rs')
        for path in self.package.glob('file*.rs'):
            path.unlink()
        for index in range(5):
            self.write(f'file{index}.rs', b'x' * source.MAX_FILE)
        with self.assertRaisesRegex(sandbox.Refused, 'total bound'):
            source.capture(self.package, 'module.rs')
        for path in self.package.glob('file*.rs'):
            path.unlink()
        deep = '/'.join(['deep'] * 8) + '/file.rs'
        self.write(deep)
        with self.assertRaisesRegex(sandbox.Refused, 'depth'):
            source.capture(self.package, 'module.rs')

    def test_binary_source_refuses(self):
        self.write('binary.rs', b'\xff')
        with self.assertRaises(UnicodeError):
            source.capture(self.package, 'module.rs')

    def test_mutation_while_read_refuses(self):
        original = source.os.read
        changed = False

        def read(fd, count):
            nonlocal changed
            data = original(fd, count)
            if data and not changed:
                changed = True
                self.write('helper.rs', b'changed concurrently and larger')
            return data
        with mock.patch.object(source.os, 'read', side_effect=read):
            with self.assertRaisesRegex(sandbox.Refused, 'changed'):
                source.capture(self.package, 'module.rs')


class PolicyTests(Fixture):
    def policy(self):
        return sandbox.MacPolicy(*(self.root / name for name in (
            'tool/bin/rustc', 'tool/bin/rust-lld', 'engine/cook', 'tool', 'sdk', 'snapshot', 'run')))

    def test_deny_default_precise_executables_and_no_user_read_or_network_rule(self):
        policy = self.policy()
        text = policy.text()
        self.assertTrue(text.startswith('(version 1)\n(deny default)\n'))
        self.assertNotIn('(import ', text)
        self.assertNotIn('(allow network', text)
        self.assertNotIn('(allow mach-', text)
        self.assertNotIn('(allow file-write* (subpath "/")', text)
        self.assertIn('(subpath "' + str(policy.run) + '")', text)
        self.assertIn('(literal "' + str(policy.linker) + '")', text)
        for executable in (policy.rustc, policy.linker, policy.validator):
            self.assertIn('(allow file-read* (literal "' + str(executable) + '"))', text)
        self.assertIn('(allow file-read* (literal "' + str(policy.validator) + '"))', text)
        self.assertNotIn('(subpath "' + str(policy.validator.parent) + '")', text)
        with mock.patch.object(sandbox.MacPolicy, 'backend', return_value=Path('/usr/bin/sandbox-exec')):
            with self.assertRaises(sandbox.Refused):
                policy.command(self.root / 'policy', ['/bin/sh', '-c', 'anything'])

    def test_canary_outside_read_trees_has_only_exact_executable_read(self):
        policy = replace(self.policy(), rustc=self.root / 'outside/sandbox-control')
        text = policy.text()
        self.assertIn('(allow file-read* (literal "' + str(policy.rustc) + '"))', text)
        self.assertIn('(allow file-read-metadata (literal "' + str(policy.rustc.parent) + '"))', text)
        self.assertNotIn('(subpath "' + str(policy.rustc.parent) + '")', text)

    def test_executable_mapping_excludes_source_sdk_and_writable_run(self):
        policy = self.policy()
        mapped = [line for line in policy.text().splitlines() if line.startswith('(allow file-map-executable')]
        for executable in (policy.rustc, policy.linker, policy.validator):
            self.assertIn('(allow file-map-executable (literal "' + str(executable) + '"))', mapped)
        self.assertIn('(allow file-map-executable (subpath "' + str(policy.sysroot / 'lib') + '"))', mapped)
        for untrusted_or_data in (policy.source, policy.run, policy.sdk):
            self.assertFalse(any(str(untrusted_or_data) in line for line in mapped))

    def test_loader_root_read_is_exact_and_does_not_admit_descendant_data(self):
        text = self.policy().text()
        self.assertEqual(text.splitlines().count('(allow file-read-data (literal "/"))'), 1)
        self.assertNotIn('(subpath "/")', text)
        self.assertNotIn('(allow file-read*)', text)
        self.assertNotIn('(allow file-map-executable (literal "/"))', text)
        self.assertNotIn('(allow file-write* (literal "/"))', text)

    def test_profile_quoting_cannot_inject_a_rule(self):
        policy = replace(self.policy(), source=self.root / 'x") (allow default) ("')
        text = policy.text()
        self.assertIn('x\\") (allow default) (\\"', text)
        with self.assertRaises(sandbox.Refused):
            replace(policy, source=self.root / 'control\nvalue').text()

    def test_clean_environment_does_not_copy_credentials_or_wrappers(self):
        caller = {'AWS_SECRET_ACCESS_KEY': 'sentinel', 'RUSTC_WRAPPER': '/evil',
                  'RUSTFLAGS': '--extern evil', 'DYLD_INSERT_LIBRARIES': '/evil'}
        with mock.patch.dict(os.environ, caller):
            environment = sandbox.clean_environment(self.root / 'run', self.root / 'tool/rustc')
        self.assertFalse(set(caller) & set(environment))
        self.assertEqual(environment['HOME'], str(self.root / 'run/home'))
        self.assertEqual(environment['RAYON_NUM_THREADS'], '1')

    def test_other_os_or_missing_backend_fails_closed(self):
        with mock.patch.object(sandbox.sys, 'platform', 'linux'):
            with self.assertRaises(sandbox.Unsupported):
                sandbox.MacPolicy.backend()
        with mock.patch.object(sandbox.sys, 'platform', 'darwin'), \
                mock.patch.object(sandbox, 'trusted_file', side_effect=FileNotFoundError):
            with self.assertRaisesRegex(sandbox.Unsupported, 'no unrestricted fallback'):
                sandbox.MacPolicy.backend()


class GroupSampleTests(unittest.TestCase):
    def api(self, count=2, error=0):
        class Function:
            def __init__(self, call):
                self.call = call

            def __call__(self, *args):
                return self.call(*args)

        def list_pids(group, buffer, size):
            self.assertEqual((group, size), (88, 1024))
            buffer[0], buffer[1] = 10, 20
            ctypes.set_errno(error)
            return count

        def info(pid, flavor, _arg, output, size):
            self.assertEqual((flavor, size), (4, 96))
            row = ctypes.cast(output, ctypes.POINTER(sandbox._TaskInfo)).contents
            row.resident_size = pid * 100
            return size
        return SimpleNamespace(proc_listpgrppids=Function(list_pids), proc_pidinfo=Function(info))

    def test_api_returns_pid_count_not_buffer_bytes(self):
        with mock.patch.object(sandbox.ctypes, 'CDLL', return_value=self.api()), \
                mock.patch.object(sandbox.sys, 'platform', 'darwin'), \
                mock.patch.object(sandbox.os, 'getpgid', return_value=88):
            self.assertEqual(sandbox.group_sample(88), [
                {'pid': 10, 'pgid': 88, 'rss': 1000}, {'pid': 20, 'pgid': 88, 'rss': 2000}])

    def test_group_overflow_or_inspection_error_refuses(self):
        for count, error in ((256, 0), (-1, 0), (0, errno.EPERM)):
            with self.subTest(count=count, errno=error), \
                    mock.patch.object(sandbox.ctypes, 'CDLL', return_value=self.api(count, error)), \
                    mock.patch.object(sandbox.sys, 'platform', 'darwin'):
                with self.assertRaisesRegex(sandbox.Refused, 'inspect bounded owned group'):
                    sandbox.group_sample(88)


class PublicationTests(Fixture):
    def test_post_link_failure_does_not_unlink_concurrent_replacement(self):
        output = self.root / 'result.wasm'
        original = build.os.fsync
        calls = 0

        def fsync(fd):
            nonlocal calls
            calls += 1
            if calls == 2:  # Final link exists; replace it before directory fsync fails.
                output.unlink()
                output.write_bytes(b'unrelated concurrent replacement')
                raise OSError('directory fsync failed')
            return original(fd)
        with mock.patch.object(build.os, 'fsync', side_effect=fsync):
            with self.assertRaises(build.native_process.CleanupError) as caught:
                build.publish(b'our bytes', output, lambda: False)
        self.assertIn('fsync failed', repr(caught.exception.operation_error))
        self.assertIn('replaced publication', repr(caught.exception.cleanup_error))
        self.assertEqual(output.read_bytes(), b'unrelated concurrent replacement')

    def test_post_link_parent_replacement_is_refused_and_own_inode_retired(self):
        parent = self.root / 'publish'
        parent.mkdir()
        output = parent / 'result.wasm'
        moved = self.root / 'moved'
        original = build.os.link

        def link(*args, **kwargs):
            original(*args, **kwargs)
            parent.rename(moved)
            parent.mkdir()
            (parent / 'result.wasm').write_bytes(b'unrelated new directory')
        with mock.patch.object(build.os, 'link', side_effect=link):
            with self.assertRaisesRegex(sandbox.Refused, 'parent replaced after'):
                build.publish(b'ours', output, lambda: False)
        self.assertEqual(output.read_bytes(), b'unrelated new directory')
        self.assertFalse((moved / 'result.wasm').exists())

    def test_create_only_publication_and_collision_preserve_original(self):
        output = self.root / 'result.wasm'
        device, inode = build.publish(b'raw-wasm', output, lambda: False)
        self.assertEqual(output.read_bytes(), b'raw-wasm')
        self.assertEqual((device, inode), (output.stat().st_dev, output.stat().st_ino))
        with self.assertRaises(FileExistsError):
            build.publish(b'replacement', output, lambda: False)
        self.assertEqual(output.read_bytes(), b'raw-wasm')
        self.assertEqual(list(self.root.glob('.incant-module-*')), [])

    def test_cancel_and_replaced_directory_never_publish(self):
        output = self.root / 'result.wasm'
        with self.assertRaisesRegex(sandbox.Refused, 'cancelled'):
            build.publish(b'raw', output, lambda: True)
        self.assertFalse(output.exists())
        self.assertEqual(list(self.root.glob('.incant-module-*')), [])

    def test_bounded_generated_output_read_rejects_link_and_oversize(self):
        path = self.root / 'module.wasm'
        path.write_bytes(b'x' * 9)
        with self.assertRaises(sandbox.Refused):
            build.read_file(path, 8)
        path.unlink()
        path.symlink_to(self.package / 'module.rs')
        with self.assertRaises(sandbox.Refused):
            build.read_file(path, 4096)


class WorkerTests(Fixture):
    def setUp(self):
        super().setUp()
        self.compiler = self.root / 'tool/bin/rustc'
        self.validator = self.root / 'engine/cook'
        self.linker = self.root / 'tool/lib/rustlib/aarch64-apple-darwin/bin/rust-lld'
        for path in (self.compiler, self.validator, self.linker):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b'fixed trusted tool')
            path.chmod(0o700)
        target = self.root / 'tool/lib/rustlib/wasm32-unknown-unknown/lib'
        target.mkdir(parents=True)
        (target / 'libcore-fixed.rlib').write_bytes(b'core')
        self.request = build.Request(self.package, 'module.rs', self.root / 'result.wasm', self.root / 'attempt')
        self.tools = build.Tools(self.compiler, self.validator)
        self.labels = []
        self.refuse = None
        self.cleanup = True
        self.tamper = False
        self.raw = b'\0asm\x01\0\0\0raw'

    def run_build(self, cancel=lambda: False):
        outer = self

        class FakeOwner(build.Owner):
            def run(self, command, *, cwd, env, label):
                self.check()
                outer.labels.append(label)
                self.calls.append({'label': label, 'argv': list(map(str, command)),
                                   'returncode': 1 if outer.refuse == label else 0,
                                   'group_absent': outer.cleanup})
                if outer.refuse == label:
                    return SimpleNamespace(returncode=1, stdout=b'', stderr=b'refused fixture')
                if label == 'compiler':
                    return SimpleNamespace(returncode=0, stdout=b'rustc 1.99.0\nrelease: 1.99.0\nhost: aarch64-apple-darwin\n', stderr=b'')
                if label == 'sdk':
                    Path(command[command.index('-o') + 1]).write_bytes(b'vetted sdk library')
                if label == 'untrusted-compile':
                    Path(command[command.index('-o') + 1]).write_bytes(outer.raw)
                if label == 'normal-validator':
                    Path(command[-1]).write_bytes(b'\0asm\x01\0\0\0prepared')
                    if outer.tamper:
                        Path(command[-2]).write_bytes(outer.raw + b'tampered')
                return SimpleNamespace(returncode=0, stdout=b'target_arch="wasm32"\n', stderr=b'')

        with mock.patch.object(build.MacPolicy, 'backend', return_value=Path('/usr/bin/sandbox-exec')), \
                mock.patch.object(build, 'Owner', FakeOwner):
            return build.build(self.request, self.tools, cancel=cancel)

    def test_worker_validates_then_publishes_raw_not_prepared(self):
        result = self.run_build()
        self.assertEqual(result['status'], 'complete')
        self.assertEqual(result['artifact_trust'], 'untrusted-data')
        self.assertEqual(self.request.output.read_bytes(), self.raw)
        self.assertNotEqual(result['raw_sha256'], result['prepared_sha256'])
        self.assertEqual(self.labels, ['compiler', 'sdk', 'sdk', 'sandbox-preflight',
                                      'untrusted-compile', 'normal-validator'])
        self.assertEqual(json.loads((self.request.work / 'result.json').read_text()), result)
        argv = result['calls'][-2]['argv']
        self.assertIn('--crate-type=cdylib', argv)
        self.assertNotIn('Cargo.toml', ' '.join(argv))
        self.assertEqual(argv.count('--extern'), 1)
        self.assertIn(f'linker={self.linker}', argv)

    def test_each_compiler_validator_or_backend_failure_stops_before_publication(self):
        for label in ('sdk', 'sandbox-preflight', 'untrusted-compile', 'normal-validator'):
            with self.subTest(label=label):
                self.refuse = label
                self.request = replace(self.request, work=self.root / f'attempt-{label}')
                self.labels.clear()
                with self.assertRaises((sandbox.Refused, RuntimeError)):
                    self.run_build()
                self.assertFalse(self.request.output.exists())
                self.assertFalse((self.request.work / 'result.json').exists())
                self.assertEqual(json.loads((self.request.work / 'failure.json').read_text())['status'], 'failed')
                self.assertEqual(self.labels[-1], label)

    def test_raw_tamper_or_unproven_cleanup_refuses(self):
        self.tamper = True
        with self.assertRaisesRegex(sandbox.Refused, 'changed during validation'):
            self.run_build()
        self.assertFalse(self.request.output.exists())
        self.request = replace(self.request, work=self.root / 'attempt-cleanup')
        self.tamper = False
        self.cleanup = False
        with self.assertRaisesRegex(sandbox.Refused, 'cleanup incomplete'):
            self.run_build()
        self.assertFalse(self.request.output.exists())

    def test_cancel_before_setup_starts_nothing(self):
        with self.assertRaisesRegex(sandbox.Refused, 'cancelled before setup'):
            self.run_build(cancel=lambda: True)
        self.assertFalse(self.request.work.exists())
        self.assertEqual(self.labels, [])

    def test_success_receipt_failure_retires_only_our_publication(self):
        original = build.write_json

        def write(path, value):
            if path.name == 'result.json':
                raise OSError('receipt write refused')
            original(path, value)
        with mock.patch.object(build, 'write_json', side_effect=write):
            with self.assertRaisesRegex(OSError, 'receipt write refused'):
                self.run_build()
        self.assertFalse(self.request.output.exists())
        self.assertEqual(json.loads((self.request.work / 'failure.json').read_text())['status'], 'failed')

    def test_unsupported_platform_precedes_source_and_work_setup(self):
        with mock.patch.object(build.MacPolicy, 'backend', side_effect=sandbox.Unsupported('no backend')):
            with self.assertRaises(sandbox.Unsupported):
                build.build(self.request, self.tools)
        self.assertFalse(self.request.work.exists())


class OwnerTests(Fixture):
    def fake_child(self, stdout=b'output', stderr=b'diagnostic'):
        outer = self

        class FakeChild:
            def __init__(self, *_args, **_kwargs):
                self.stdin = io.BytesIO()
                self.closed = False
                self.returncode = None
                self.tree = SimpleNamespace(process=SimpleNamespace(pid=424242), exited=lambda: True)
                pipes = []
                for data in (stdout, stderr):
                    read, write = os.pipe()
                    os.write(write, data)
                    os.close(write)
                    pipes.append(os.fdopen(read, 'rb', buffering=0))
                self.stdout, self.stderr = pipes
                outer.child = self

            def __enter__(self):
                return self

            def __exit__(self, *_):
                self.close()

            def wait(self, **_):
                self.close()
                return self.returncode

            def close(self):
                self.closed = True
                self.returncode = 0
                self.stdin.close()
                self.stdout.close()
                self.stderr.close()

        return FakeChild

    def test_owner_drains_bounded_bytes_after_managed_wait_and_records_cleanup(self):
        owner = build.Owner(self.root, build.Limits(), lambda: False)
        with mock.patch.object(build.native_process, 'Interactive', self.fake_child()), \
                mock.patch.object(build, 'group_sample', return_value=[{'pid': 424242, 'rss': 123}]), \
                mock.patch.object(build.native_process, '_darwin_group_is_empty', return_value=True), \
                mock.patch.object(build.sys, 'platform', 'darwin'):
            result = owner.run(['/trusted/fixture'], cwd=self.root, env={}, label='fixture')
        self.assertEqual((result.stdout, result.stderr), (b'output', b'diagnostic'))
        self.assertTrue(self.child.closed)
        self.assertTrue(owner.calls[0]['group_absent'])
        self.assertEqual(owner.calls[0]['observed_pids'], [424242])
        self.assertEqual(owner.peak_rss, 123)

    def test_output_cap_closes_owner_and_never_returns_success(self):
        owner = build.Owner(self.root, build.Limits(stream_bytes=2), lambda: False)
        with mock.patch.object(build.native_process, 'Interactive', self.fake_child()), \
                mock.patch.object(build, 'group_sample', return_value=[]), \
                mock.patch.object(build.native_process, '_darwin_group_is_empty', return_value=True), \
                mock.patch.object(build.sys, 'platform', 'darwin'):
            with self.assertRaisesRegex(sandbox.Refused, 'output cap'):
                owner.run(['/trusted/fixture'], cwd=self.root, env={}, label='capped')
        self.assertTrue(self.child.closed)
        self.assertTrue(owner.calls[0]['group_absent'])

    def test_unknown_or_live_cleanup_cannot_return_success(self):
        owner = build.Owner(self.root, build.Limits(), lambda: False)
        with mock.patch.object(build.native_process, 'Interactive', self.fake_child()), \
                mock.patch.object(build, 'group_sample', return_value=[]), \
                mock.patch.object(build.native_process, '_darwin_group_is_empty', side_effect=OSError('unknown')), \
                mock.patch.object(build.sys, 'platform', 'darwin'):
            with self.assertRaisesRegex(OSError, 'unknown'):
                owner.run(['/trusted/fixture'], cwd=self.root, env={}, label='unknown')
        self.assertTrue(self.child.closed)

    def test_resource_error_survives_unknown_group_cleanup_and_receipt_write(self):
        owner = build.Owner(self.root, build.Limits(stream_bytes=2), lambda: False)
        original = build.write_json

        def write(path, value):
            if path.name.endswith('.result.json'):
                raise OSError('receipt unavailable')
            return original(path, value)
        with mock.patch.object(build.native_process, 'Interactive', self.fake_child()), \
                mock.patch.object(build, 'group_sample', return_value=[]), \
                mock.patch.object(build.native_process, '_darwin_group_is_empty', side_effect=OSError('group unknown')), \
                mock.patch.object(build.sys, 'platform', 'darwin'), \
                mock.patch.object(build, 'write_json', side_effect=write):
            with self.assertRaises(build.native_process.CleanupError) as caught:
                owner.run(['/trusted/fixture'], cwd=self.root, env={}, label='multiple')
        self.assertIn('output cap', repr(caught.exception.operation_error))
        self.assertIn('group unknown', repr(caught.exception.cleanup_error))
        self.assertIn('receipt unavailable', repr(caught.exception.cleanup_error))
        self.assertTrue(self.child.closed)

    def test_actual_constructor_and_prelaunch_resource_checks_without_child(self):
        owner = build.Owner(self.root, build.Limits(), lambda: False)
        owner.check()
        self.assertEqual(owner.calls, [])
        self.assertGreater(owner.peak_scratch, 0)
        owner.cancel = lambda: True
        with mock.patch.object(build.native_process, 'Interactive') as child:
            with self.assertRaisesRegex(sandbox.Refused, 'cancelled'):
                owner.run(['/not-started'], cwd=self.root, env={}, label='cancel')
            child.assert_not_called()

    def test_limits_refuse_nonfinite_negative_and_oversized_module_admission(self):
        for limits in (build.Limits(seconds=float('inf')), build.Limits(child_seconds=0),
                       build.Limits(raw_bytes=build.MIB + 1), build.Limits(scratch_bytes=-1)):
            with self.subTest(limits=limits), self.assertRaises(sandbox.Refused):
                limits.validate()


class ProbeContractTests(Fixture):
    def test_proc_macro_probe_declares_a_macro_under_fixed_wasm_cdylib_recipe(self):
        path = Path(__file__).resolve().parents[1] / 'probes/rust-guest-sandbox.py'
        spec = importlib.util.spec_from_file_location('rust_sandbox_macro_fixture', path)
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        self.assertIn('#[proc_macro]\npub fn forbidden(', probe.PROC_MACRO)
        self.assertIn('tokens: proc_macro::TokenStream) -> proc_macro::TokenStream', probe.PROC_MACRO)
        # The attribute/extern-only native004 fixture lacked a procedural macro.
        old_fixture = '#![crate_type="proc-macro"]\nextern crate proc_macro;\n'
        self.assertNotIn('#[proc_macro]', old_fixture)
        command = build.guest_command(Path('/trusted/rustc'), Path('/trusted/rust-lld'),
                                      self.package / 'module.rs', self.root / 'sdk', self.root / 'module.wasm')
        self.assertIn('--crate-type=cdylib', command)
        self.assertIn('--target=wasm32-unknown-unknown', command)
        self.assertNotIn('--crate-type=proc-macro', command)
        self.assertEqual(command.count('--extern'), 1)

    def test_actual_probe_callback_does_not_shadow_sdk_macro_exports(self):
        tools = Path(__file__).resolve().parents[1]
        spec = importlib.util.spec_from_file_location('rust_sandbox_fixture', tools / 'probes/rust-guest-sandbox.py')
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        sdk = (tools.parent / 'sdk/rust/src/lib.rs').read_text()
        macro = sdk.split('macro_rules! module {', 1)[1].split('#[doc(hidden)]', 1)[0]
        exports = set(re.findall(r'pub extern "C" fn (\w+)\(', macro))
        self.assertEqual(exports, {'incant_abi', 'update'})

        def callback_is_distinct(text):
            callback = re.search(r'incant_module::module!\((\w+)\);', text).group(1)
            self.assertRegex(text, rf'fn {callback}\(')
            return callback not in exports

        self.assertTrue(callback_is_distinct(probe.VALID))
        # Retained native003's exact E0428 fixture must fail this control.
        self.assertFalse(callback_is_distinct(probe.VALID.replace('on_update', 'update')))

    def test_complete_frozen_plan_checks_bindings_and_exact_grant_without_child(self):
        path = Path(__file__).resolve().parents[1] / 'probes/rust-guest-sandbox.py'
        spec = importlib.util.spec_from_file_location('rust_sandbox_probe', path)
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        bound = self.package / 'module.rs'
        plan = {'version': 1, 'scope': 'untrusted-rust-macos-isolation', 'source': 'fixture-source',
                'output': str(self.root / 'new-output'), 'rustc': '/trusted/rustc', 'validator': '/trusted/cook',
                'native_linker': '/trusted/clang', 'macos_sdk': '/trusted/MacOSX.sdk',
                'bounds': {'seconds': 180, 'child_seconds': 90, 'scratch_bytes': 512 * build.MIB,
                           'log_bytes': 16 * build.MIB, 'stream_bytes': 4 * build.MIB,
                           'rss_bytes': 8 * 1024 * build.MIB, 'floor_bytes': 6 * 1024 * build.MIB},
                'bindings': [{'path': str(bound), 'bytes': bound.stat().st_size,
                              'sha256': build.digest(bound.read_bytes())}]}
        manifest = self.root / 'plan.json'
        build.write_json(manifest, plan)
        grant = self.root / 'grant.json'
        build.write_json(grant, {'allowed': True, 'plan_sha256': build.digest(manifest.read_bytes()),
                                 'probe_sha256': build.digest(path.read_bytes())})
        with mock.patch.object(build.native_process, 'Interactive') as child:
            checked = probe.check(manifest, grant)
            build.Limits(**checked['bounds']).validate()
            child.assert_not_called()
        # Actual complete-key reads reject the classic log_bytes/logs_bytes drift.
        plan['bounds']['logs_bytes'] = plan['bounds'].pop('log_bytes')
        manifest.write_text(json.dumps(plan))
        with self.assertRaisesRegex(sandbox.Refused, 'exact bounds'):
            probe.check(manifest)
        plan['bounds']['log_bytes'] = plan['bounds'].pop('logs_bytes')
        manifest.write_text(json.dumps(plan))
        self.write('module.rs', b'changed source')
        with self.assertRaises(sandbox.Refused):
            probe.check(manifest)


if __name__ == '__main__':
    unittest.main()
