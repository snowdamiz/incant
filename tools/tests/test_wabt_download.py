"""Local transport-fault controls; these do not establish a live provider gate."""
import contextlib
import hashlib
import importlib.util
import io
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import socket
import ssl
import tarfile
import tempfile
import threading
import unittest
from unittest.mock import patch
import urllib.error

spec = importlib.util.spec_from_file_location('retry_shipping', Path(__file__).resolve().parents[1] / 'probes/shipping-wasm2c.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def archive():
    data = io.BytesIO()
    with tarfile.open(fileobj=data, mode='w:gz') as bundle:
        for name in ('wasm2c.exe', 'wat2wasm.exe'):
            member = tarfile.TarInfo('wabt-1.0.42/bin/' + name)
            member.size = 4
            bundle.addfile(member, io.BytesIO(b'tool'))
    return data.getvalue()


@contextlib.contextmanager
def local_transport(mode, body):
    calls = []
    class Handler(BaseHTTPRequestHandler):
        count = 0
        def do_GET(self):
            type(self).count += 1
            if mode == 'reset' or (mode == 'reset-once' and self.count == 1):
                self.connection.shutdown(socket.SHUT_RDWR)
                self.connection.close()
                return
            if mode == 'forbidden' or (mode == 'unavailable-once' and self.count == 1):
                self.send_error(403 if mode == 'forbidden' else 503)
                return
            self.send_response(200)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def log_message(self, *_):
            pass
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    original = probe.urllib.request.urlopen
    def local_open(url, *, timeout):
        calls.append((url, timeout))
        return original(f'http://127.0.0.1:{server.server_port}/archive', timeout=timeout)
    try:
        with patch.object(probe.urllib.request, 'urlopen', side_effect=local_open):
            yield calls
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)
        assert not thread.is_alive()


class WabtDownloadTests(unittest.TestCase):
    def setup_pins(self, data):
        stack = contextlib.ExitStack()
        stack.enter_context(patch.object(probe.platform, 'system', return_value='Windows'))
        stack.enter_context(patch.object(probe.platform, 'machine', return_value='AMD64'))
        stack.enter_context(patch.dict(probe.ASSETS, {('Windows', 'AMD64'): ('windows-x64', hashlib.sha256(data).hexdigest())}))
        return stack

    def test_real_connection_close_and_transient_http_recover_on_same_pinned_source(self):
        data = archive()
        for mode in ('reset-once', 'unavailable-once'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory, self.setup_pins(data), local_transport(mode, data) as calls:
                root, provenance = probe.provision(Path(directory))
                self.assertEqual((root/'bin/wasm2c.exe').read_bytes(), b'tool')
                self.assertEqual(len(calls), 2)
                self.assertEqual(calls, [(provenance['url'], 30)] * 2)
                self.assertEqual(provenance['archive_sha256'], hashlib.sha256(data).hexdigest())
                self.assertEqual((Path(directory)/'wabt.tar.gz').read_bytes(), data)

    def test_repeated_disconnect_stops_after_three_attempts_without_extraction(self):
        data = archive()
        with tempfile.TemporaryDirectory() as directory, self.setup_pins(data), local_transport('reset', data) as calls:
            with self.assertRaises(ConnectionError):
                probe.provision(Path(directory))
            self.assertEqual(len(calls), 3)
            self.assertFalse((Path(directory)/'wabt').exists())

    def test_permanent_http_and_checksum_failures_are_never_retried(self):
        data = archive()
        for mode, body, error in [('forbidden', data, urllib.error.HTTPError), ('ok', b'altered archive', ValueError)]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory, self.setup_pins(data), local_transport(mode, body) as calls:
                with self.assertRaises(error):
                    probe.provision(Path(directory))
                self.assertEqual(len(calls), 1)
                self.assertFalse((Path(directory)/'wabt').exists())

    def test_certificate_validation_failure_is_never_retried(self):
        error = urllib.error.URLError(ssl.SSLCertVerificationError('invalid certificate'))
        with tempfile.TemporaryDirectory() as directory, self.setup_pins(archive()), patch.object(probe.urllib.request, 'urlopen', side_effect=error) as opening:
            with self.assertRaises(urllib.error.URLError):
                probe.provision(Path(directory))
            self.assertEqual(opening.call_count, 1)

    def test_partial_transfer_is_truncated_before_retry(self):
        class Interrupted(io.BytesIO):
            def read(self, count):
                if self.tell():
                    raise ConnectionResetError('transfer interrupted')
                return super().read(count)
        data = archive()
        with tempfile.TemporaryDirectory() as directory, self.setup_pins(data), patch.object(probe.urllib.request, 'urlopen', side_effect=[Interrupted(b'partial prefix'), io.BytesIO(data)]) as opening:
            probe.provision(Path(directory))
            self.assertEqual(opening.call_count, 2)
            self.assertEqual((Path(directory)/'wabt.tar.gz').read_bytes(), data)
