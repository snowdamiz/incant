"""Data-only refusal controls; these do not simulate renderer correctness."""
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import temporal_parity as parity
from native_package import sha256


class TemporalParity(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.paths = [Path(self.tmp.name).resolve() / str(i) for i in range(4)]
        for i, path in enumerate(self.paths):
            path.mkdir()
            test = 'motion-owner' if i < 2 else 'temporal-resolve'
            rows = []
            for name, (w, h, format) in parity.expected(test).items():
                if test == 'motion-owner':
                    frame = int(name.split('-')[1][-1])
                    ids = [n % 6 for n in range(w * h)]
                    if format == 'r32uint':
                        raw = b''.join(struct.pack('<I', n | (0x80000000 if n and (frame == 0 or frame == 2 and n in (3, 5)) else 0)) for n in ids)
                    elif format == 'rgba32float':
                        raw = b''.join(struct.pack('<4f', 0.03125 if frame == 2 and n == 3 else 0., 0.,
                            0.5 if frame else 0., 1. if frame else 0.) for n in ids)
                    elif format == 'depth32float': raw = struct.pack('<f', 0.5) * (w * h)
                    else: raw = bytes([80, 120, 160, 255]) * (w * h)
                else:
                    mode = int(name[4:6])
                    pixel = ((0.375, 0.375, 0.375, 1.) if mode in (0, 1, 8, 9, 11, 13) else
                        (0., 0., 0., 0.) if mode == 4 else (0.2, 0.1, 0., 0.5) if mode == 5 else
                        (2., 0.5, 100., 1.) if mode == 7 else
                        (0.2 + 0.03 * 3.375 + 0.02 * 2.625, 0.25, 0.5, 1.) if mode == 12 else (0.5, 0.5, 0.5, 1.))
                    raw = struct.pack('<4e', *pixel) * (w * h)
                file = name + '.raw'
                (path / file).write_bytes(raw)
                rows.append(dict(name=name, file=file, width=w, height=h, format=format, bytes=len(raw), sha256=sha256(raw)))
            self.write(path, dict(format='incant-fixed-temporal-parity-v1', test=test,
                complete=True, cooked_temporal=bool(i % 2), records=rows, bytes=sum(r['bytes'] for r in rows),
                comparison='exact raw cross-mode equality; existing analytical assertions unchanged'))

    def write(self, path, summary):
        (path / 'summary.json').write_text(json.dumps(summary))

    def change(self, i, name, raw):
        path = self.paths[i]
        summary = json.loads((path / 'summary.json').read_text())
        row = next(r for r in summary['records'] if r['name'] == name)
        (path / row['file']).write_bytes(raw)
        row['sha256'] = sha256(raw)
        self.write(path, summary)

    def test_exact_comparison_passes_then_rehashed_raw_difference_refuses(self):
        result = parity.compare(*self.paths)
        self.assertEqual(result['tolerance_codes'], 0)
        self.assertTrue(all(n > 0 for n in result['nonzero_motion_pixels']))
        name = 'scale16-frame0-on-color'
        raw = bytearray((self.paths[1] / (name + '.raw')).read_bytes()); raw[0] ^= 1
        self.change(1, name, raw)
        with self.assertRaisesRegex(ValueError, 'raw mismatch'): parity.compare(*self.paths)

    def test_incomplete_wrong_route_duplicate_missing_and_changed_shape_refuse(self):
        path = self.paths[1]
        original = json.loads((path / 'summary.json').read_text())
        for field, value in [('complete', False), ('cooked_temporal', False)]:
            self.write(path, dict(original, **{field: value}))
            with self.assertRaises(ValueError): parity.compare(*self.paths)
        for kind in ('missing', 'duplicate', 'shape', 'path'):
            summary = json.loads(json.dumps(original))
            if kind == 'missing': summary['records'].pop()
            elif kind == 'duplicate': summary['records'][-1] = summary['records'][0]
            elif kind == 'shape': summary['records'][0]['width'] = True
            else: summary['records'][0]['file'] = '../outside'
            self.write(path, summary)
            with self.assertRaises(ValueError): parity.compare(*self.paths)

    def test_equal_zero_motion_is_not_sufficient(self):
        for i in (0, 1):
            for name in ('scale16-frame2-on-motion', 'scale16-frame2-off-motion'):
                raw = struct.pack('<4f', 0., 0., 0.5, 1.) * (32 * 32)
                self.change(i, name, raw)
        with self.assertRaisesRegex(ValueError, 'no motion'): parity.compare(*self.paths)

    def test_equal_lost_owner_coverage_or_unbound_raw_mutation_refuses(self):
        name = 'scale16-frame0-on-owner'
        for i in (0, 1): self.change(i, name, bytes(32 * 32 * 4))
        with self.assertRaisesRegex(ValueError, 'coverage missing'): parity.compare(*self.paths)
        raw = self.paths[0] / (name + '.raw')
        raw.write_bytes(bytes([1]) + raw.read_bytes()[1:])
        with self.assertRaisesRegex(ValueError, 'Raw identity'): parity.compare(*self.paths)

    def test_equal_history_fallback_or_nonfinite_output_refuses_analytical_contract(self):
        for i in (2, 3):
            self.change(i, 'mode00-resolved', struct.pack('<4e', 0.5, 0.5, 0.5, 1.) * 64)
        with self.assertRaisesRegex(ValueError, 'analytical oracle'): parity.compare(*self.paths)
        for i in (2, 3):
            self.change(i, 'mode00-resolved', struct.pack('<4e', float('nan'), 0.5, 0.5, 1.) * 64)
        with self.assertRaisesRegex(ValueError, 'Nonfinite'): parity.compare(*self.paths)


if __name__ == '__main__':
    unittest.main()
