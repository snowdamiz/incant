"""Pure canary protocol and owned-loopback setup controls. No OS child/socket is opened."""
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock

from tools.tests.test_rust_guest_build import Fixture


def load_probe():
    path = Path(__file__).resolve().parents[1] / 'probes/rust-guest-sandbox.py'
    spec = importlib.util.spec_from_file_location('rust_guest_network_probe', path)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    return probe


def control():
    return {'criterion': 'network-io-v2', 'read_denied': True, 'write_denied': True,
            'exec_denied': True, 'clean_environment': True, 'sockets_created': 4,
            'socket_creation_denied': False, 'network': {
                name: {'return': -1, 'errno': 1 if index % 2 else 13}
                for index, name in enumerate(('tcp_connect', 'tcp_bind', 'udp_bind', 'udp_send_to'))}}


class ProtocolTests(unittest.TestCase):
    def test_exact_permission_denials_and_distinct_creation_report(self):
        probe = load_probe()
        value = control()
        self.assertEqual(probe.validate_control(value), value)
        with self.assertRaises(probe.Refused):
            probe.validate_control({'socket_denied': False})  # Failed native005 stays failed.

    def test_success_other_errno_missing_unknown_and_boolean_integers_refuse(self):
        probe = load_probe()
        for name in control()['network']:
            for row in ({'return': 0, 'errno': None}, {'return': 1, 'errno': 1},
                        {'return': -1, 'errno': 61}, {'return': -1, 'errno': 60},
                        {'return': -1, 'errno': 22}, {'return': -1, 'errno': 35},
                        {'return': -1, 'errno': True}, {'return': -1, 'errno': 1, 'extra': 0}):
                value = control()
                value['network'][name] = row
                with self.subTest(name=name, row=row), self.assertRaises(probe.Refused):
                    probe.validate_control(value)
        for key in control():
            value = control()
            del value[key]
            with self.subTest(missing=key), self.assertRaises(probe.Refused):
                probe.validate_control(value)
        for key, changed in [('criterion', 'creation-v1'), ('sockets_created', True),
                             ('socket_creation_denied', True), ('exec_denied', False), ('unknown', 1)]:
            value = control()
            value[key] = changed
            with self.subTest(key=key), self.assertRaises(probe.Refused):
                probe.validate_control(value)
        value = control()
        del value['network']['tcp_bind']
        with self.assertRaises(probe.Refused):
            probe.validate_control(value)


class SetupTests(Fixture):
    def setUp(self):
        super().setUp()
        self.probe = load_probe()
        self.output = self.root / 'output'
        self.output.mkdir()
        (self.output / 'private-sentinel').write_bytes(b'private')
        self.valid = self.root / 'valid'
        self.valid.mkdir()
        self.tools = self.probe.service.Tools(self.root / 'tool/bin/rustc', self.root / 'engine/cook')
        self.probe.service.write_json(self.valid / 'result.json', {'tools': [
            {'path': str(path)} for path in (self.tools.rustc, self.root / 'tool/bin/rust-lld', self.tools.validator)]})
        self.received = None
        self.run_error = None
        self.fail_second_socket = False
        self.address = '127.0.0.1'
        self.sockets = []
        self.commands = []
        self.plan = {'native_linker': '/trusted/clang', 'macos_sdk': '/trusted/sdk'}

    def execute(self):
        outer = self

        class Socket:
            def __init__(self, index):
                self.index, self.closed, self.inheritable = index, False, True
                self.family = outer.probe.socket.AF_INET
                self.type = outer.probe.socket.SOCK_STREAM if index == 0 else outer.probe.socket.SOCK_DGRAM
                outer.sockets.append(self)

            def set_inheritable(self, value): self.inheritable = value
            def get_inheritable(self): return self.inheritable
            def bind(self, address): outer.assertEqual(address, ('127.0.0.1', 0))
            def getsockname(self): return (outer.address, 42100 + self.index)
            def getsockopt(self, *_): raise AssertionError('Unsupported optional inspection must not run')
            def settimeout(self, value): outer.assertEqual(value, 0.05)
            def close(self): self.closed = True
            def fileno(self): return -1 if self.closed else 42000 + self.index

            def recvfrom(self, cap):
                outer.assertEqual(cap, 256)
                if outer.received is None:
                    raise TimeoutError()
                return outer.received, ('127.0.0.1', 42102)

        def run(command, **kwargs):
            self.commands.append(command)
            if kwargs['label'] == 'trusted-control-build':
                return SimpleNamespace(returncode=0)
            self.assertEqual(command[-2:], ['42100', '42101'])
            self.assertTrue(all(not endpoint.inheritable for endpoint in self.sockets))
            if self.run_error:
                raise self.run_error
            (self.output / 'control-run/actual-write').write_bytes(b'control executed')
            return SimpleNamespace(returncode=0, stdout=json.dumps(control()).encode())

        def make_socket(*_):
            if self.fail_second_socket and self.sockets:
                raise OSError('second endpoint allocation failed')
            return Socket(len(self.sockets))

        with mock.patch.object(self.probe.socket, 'socket', side_effect=make_socket), \
                mock.patch.object(self.probe.MacPolicy, 'backend', return_value=Path('/usr/bin/sandbox-exec')), \
                mock.patch.object(self.probe.service.native_process, 'Interactive') as child:
            try:
                return self.probe.run_control(SimpleNamespace(run=run), self.tools, self.valid, self.output,
                                              self.plan)
            finally:
                child.assert_not_called()

    def test_actual_setup_sequence_closes_noninherited_loopback_reservations(self):
        self.assertEqual(self.execute(), control())
        self.assertTrue(all(endpoint.closed for endpoint in self.sockets))
        receipt = json.loads((self.output / 'control-network.json').read_text())
        self.assertEqual((receipt['sink_datagrams'], receipt['endpoints_closed']), (0, True))
        self.assertFalse(receipt['tcp_listen_called'])
        self.assertEqual(len(self.commands), 2)

    def prepare_reuse(self):
        self.execute()  # Mock compilation, no native child or actual socket.
        binary = self.output / 'sandbox-control'
        binary.write_bytes(b'trusted native fixture')
        binary.chmod(0o700)
        receipt = self.output / 'compiled.json'
        self.probe.service.write_json(receipt, {'argv': self.commands[0], 'returncode': 0, 'group_absent': True})
        row = {'path': str(binary), 'bytes': binary.stat().st_size,
               'sha256': self.probe.service.digest(binary.read_bytes()),
               'source_sha256': self.probe.service.digest((self.probe.ROOT / 'tools/probes/rust-guest-sandbox-control.rs').read_bytes()),
               'build_receipt': str(receipt)}
        self.commands, self.sockets = [], []
        self.plan['trusted_canary_reuse'] = row
        return row

    def test_exact_reuse_runs_only_the_bound_trusted_canary(self):
        row = self.prepare_reuse()
        self.output = self.root / 'reuse-output'
        self.output.mkdir()
        self.assertEqual(self.execute(), control())
        self.assertEqual(len(self.commands), 1)
        self.assertEqual(self.commands[0][3], row['path'])
        self.assertTrue(all(endpoint.closed for endpoint in self.sockets))

    def test_changed_canary_bytes_source_or_recipe_refuse_before_socket_setup(self):
        original = self.prepare_reuse()
        for name in ('bytes', 'source', 'recipe'):
            self.output = self.root / ('reuse-refusal-' + name)
            self.output.mkdir()
            row = dict(original)
            if name == 'bytes':
                row['sha256'] = '0' * 64
            elif name == 'source':
                row['source_sha256'] = '0' * 64
            else:
                receipt = json.loads(Path(row['build_receipt']).read_text())
                receipt['argv'][0] = '/wrong/compiler'
                Path(row['build_receipt']).write_text(json.dumps(receipt))
            self.plan['trusted_canary_reuse'] = row
            with self.subTest(name=name), self.assertRaises(self.probe.Refused):
                self.execute()
            self.assertEqual((self.commands, self.sockets), ([], []))

    def test_empty_datagram_is_still_a_failure_and_all_endpoints_close(self):
        self.received = b''
        with self.assertRaisesRegex(self.probe.Refused, 'received a datagram'):
            self.execute()
        receipt = json.loads((self.output / 'control-network.json').read_text())
        self.assertEqual(receipt['sink_datagrams'], 1)
        self.assertTrue(receipt['endpoints_closed'])

    def test_second_endpoint_allocation_failure_closes_first(self):
        self.fail_second_socket = True
        with self.assertRaisesRegex(OSError, 'second endpoint allocation'):
            self.execute()
        self.assertEqual(len(self.sockets), 1)
        self.assertTrue(self.sockets[0].closed)
        self.assertEqual(len(self.commands), 1)

    def test_nonloopback_endpoint_refuses_before_sandbox_execution(self):
        self.address = '192.0.2.1'  # Mock value only; no actual socket.
        with self.assertRaisesRegex(self.probe.Refused, 'Owned loopback'):
            self.execute()
        self.assertEqual(len(self.commands), 1)  # Only mocked compile, no sandbox child.
        self.assertTrue(all(endpoint.closed for endpoint in self.sockets))

    def test_primary_child_error_survives_receipt_error_after_all_socket_closes(self):
        self.run_error = RuntimeError('original managed child failure')
        with mock.patch.object(self.probe.service, 'write_json', side_effect=OSError('receipt failed')):
            with self.assertRaises(self.probe.service.native_process.CleanupError) as caught:
                self.execute()
        self.assertIn('original managed child failure', repr(caught.exception.operation_error))
        self.assertIn('receipt failed', repr(caught.exception.cleanup_error))
        self.assertTrue(all(endpoint.closed for endpoint in self.sockets))


if __name__ == '__main__':
    unittest.main()
