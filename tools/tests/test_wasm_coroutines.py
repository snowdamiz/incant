import struct
import unittest
from wasm_coroutines import catalog


def leb(n):
    result = bytearray()
    while n >= 128:
        result.append((n & 127) | 128)
        n >>= 7
    return bytes(result) + bytes([n])


def module(rows):
    value = b'INCCAT01' + struct.pack('<II', 1, len(rows)) + b''.join(rows)
    payload = leb(len(b'incant_coroutines_v1')) + b'incant_coroutines_v1' + value
    return b'\0asm\1\0\0\0' + b'\0' + leb(len(payload)) + payload


class CatalogTests(unittest.TestCase):
    def test_catalog_exact_names_layout_and_all_truncations(self):
        row = b'walk' + bytes(60) + struct.pack('<II', 4, 8) + bytes([1])*32
        data = module([row])
        self.assertEqual(catalog(data), [{'name': 'walk', 'bytes': 8, 'schema_sha256': '01'*32}])
        for end in range(len(data)):
            if end == 8:
                self.assertEqual(catalog(data[:end]), [])
            else:
                with self.subTest(end=end), self.assertRaises(ValueError): catalog(data[:end])
        with self.assertRaises(ValueError): catalog(module([row,row]))
        for offset in [0,63,64,69,70,71]:
            bad = bytearray(row); bad[offset] = 255
            with self.subTest(offset=offset), self.assertRaises(ValueError): catalog(module([bytes(bad)]))
        with self.assertRaises(ValueError): catalog(module([row[:68]+bytes(4)+row[72:]]))
        with self.assertRaises(ValueError): catalog(module([row[:72]+bytes(32)]))
        with self.assertRaises(ValueError): catalog(data+data[8:])


if __name__ == '__main__': unittest.main()
