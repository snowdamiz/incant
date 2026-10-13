"""Acceptance-harness refusal controls, not platform execution evidence."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).with_name('verify.py')
spec = importlib.util.spec_from_file_location('mobile_verify', MODULE)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class Receipts(unittest.TestCase):
    def test_archive_and_header_pairing_and_guest_identity_are_checked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive, header, receipt = (root / name for name in ('game.a', 'game.h', 'receipt.json'))
            archive.write_bytes(b'archive')
            header.write_bytes(b'header')
            data = {'archive': {'path': str(archive), 'sha256': verify.digest(archive)},
                    'header': {'path': str(header), 'sha256': verify.digest(header)},
                    'features': ['mobile-ffi'], 'command': {'exit_code': 0},
                    'fixture': 'default trusted fixtures/game.js',
                    'sources': {'crates/incant_game_player/fixtures/game.js':
                                verify.digest(verify.ROOT / 'crates/incant_game_player/fixtures/game.js')}}
            receipt.write_text(json.dumps(data))
            checksum = verify.digest(receipt)
            self.assertEqual(verify.read_receipt(receipt, checksum), data)
            for target in (archive, header):
                original = target.read_bytes()
                target.write_bytes(original + b'changed')
                with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                    verify.read_receipt(receipt, checksum)
                target.write_bytes(original)
            data['sources']['crates/incant_game_player/fixtures/game.js'] = '0' * 64
            receipt.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, 'receipt checksum'):
                verify.read_receipt(receipt, checksum)
            with self.assertRaisesRegex(ValueError, 'guest differs'):
                verify.read_receipt(receipt, verify.digest(receipt))

    def test_numeric_log_epoch_and_full_records_are_required(self):
        messages = [f'message {i}' for i in range(15)]
        numeric = {'scenarios': {'held': {'messages': messages}}}
        entries = [{'tick': i // 5 + 1, 'level': 'info', 'message': value}
                   for i, value in enumerate(messages)]
        report = {'game': {'observations': {'entities': {
            '00000000000000000000000001': {'translation': [0.1, 0, 0]}}},
            'logs': {'entries': entries, 'dropped': 0}}}
        verify.checked_game(report, 'held', numeric)
        entries[0]['tick'] = 2
        with self.assertRaises(AssertionError):
            verify.checked_game(report, 'held', numeric)
        entries[0]['tick'] = 1
        entries.pop()
        with self.assertRaises(AssertionError):
            verify.checked_game(report, 'held', numeric)

    def test_failure_return_alone_does_not_satisfy_expected_diagnostic(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dest = root / 'run'
            dest.mkdir()
            report = {'format': 'incant-mobile-runtime', 'version': 1,
                      'passed': False, 'execution': 'offline_cpu', 'architecture': 'aarch64',
                      'game': None, 'error': 'unrelated failure'}
            (dest / 'result.json').write_text(json.dumps(report))
            proof = verify.Proof(root)
            proof.command = lambda *_: '{"abi_status":1}'
            with self.assertRaises(AssertionError):
                proof.control('expected-failure', [], {'output_directory': str(dest)}, 1,
                              error='native package startup checksum mismatch')
            self.assertEqual(proof.controls, [])

    def test_malformed_request_cannot_publish_a_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dest = root / 'bad'
            dest.mkdir()
            proof = verify.Proof(root)
            proof.command = lambda *_: '{"abi_status":2}'
            with self.assertRaises(AssertionError):
                proof.control('bad', [], {'output_directory': str(dest)}, 2, malformed=True)
            self.assertEqual(proof.controls, [])


if __name__ == '__main__':
    unittest.main()
