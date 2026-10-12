import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import native_fonts
from native_package import read

class FontPackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.cache = Path(self.temp.name).resolve()
        self.data = b'bounded sfnt fixture data'  # Face validity belongs to the Rust reader.
        self.content = hashlib.sha256(self.data).hexdigest()
        self.key = 'a' * 64
        self.asset = {'id': '00000000000000000000000001', 'fingerprint': self.key}
        self.metadata = {'version': 1, 'fingerprint': self.key, 'bytes': len(self.data),
                         'content_sha256': self.content, 'dependency': {'sha256': self.content}}
        self.blob = b'INCTFNT\0' + struct.pack('<II', 1, len(self.data)) + bytes.fromhex(self.content) + self.data
        self.save()
    def save(self):
        (self.cache / (self.key + '.json')).write_text(json.dumps(self.metadata))
        (self.cache / (self.content + '.ifont')).write_bytes(self.blob)
    def run_package(self, assets=None):
        out = {}
        native_fonts.package_fonts(self.cache, assets or [self.asset], lambda k,v: out.__setitem__(k,v), read)
        return out
    def test_exact_cooked_envelope_copied_and_source_is_not_requested(self):
        out = self.run_package()
        self.assertEqual(out['fonts/' + self.content + '.ifont'], self.blob)
        self.assertEqual(len(out), 2)
    def test_corrupt_envelope_digest_length_and_duplicate_ids_refused(self):
        original = self.blob
        for offset in (0, 8, 12, 16, 48):
            with self.subTest(offset=offset):
                data = bytearray(original); data[offset] ^= 1
                self.blob = bytes(data); self.save()
                with self.assertRaises(ValueError): self.run_package()
        self.blob = original + b'x'; self.save()
        with self.assertRaises(ValueError): self.run_package()
        self.blob = original; self.save()
        with self.assertRaisesRegex(ValueError, 'Duplicate'): self.run_package([self.asset]*2)
    def test_aggregate_logical_limit_is_checked_before_next_payload_read(self):
        self.metadata['bytes'] = 32 * 1024**2
        encoded = json.dumps(self.metadata).encode()
        reads = []
        # Individual envelope refusals remain mandatory; simulate accepted first
        # two font payloads with real 32MiB bytes, bounded by the same read API.
        data = b'\0' * (32 * 1024**2)
        content = hashlib.sha256(data).hexdigest()
        self.metadata.update(content_sha256=content, dependency={'sha256': content})
        encoded = json.dumps(self.metadata).encode()
        payload = b'INCTFNT\0' + struct.pack('<II', 1, len(data)) + bytes.fromhex(content) + data
        def bounded_read(path, limit):
            reads.append(path.suffix)
            return encoded if path.suffix == '.json' else payload
        assets = [dict(self.asset, id=f'{n:026d}') for n in (1,2,3)]
        with self.assertRaisesRegex(ValueError, 'Total font'):
            native_fonts.package_fonts(self.cache, assets, lambda *_:None, bounded_read)
        self.assertEqual(reads, ['.json','.ifont','.json','.ifont','.json'])
