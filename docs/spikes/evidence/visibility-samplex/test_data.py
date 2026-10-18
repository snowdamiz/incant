"""Small saved-data negative controls; no native process or GPU access."""
import base64
import gzip
import json
from pathlib import Path
import struct
import unittest

import dependency_data as data


class DataControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        records = json.loads(gzip.decompress((Path(__file__).parent / "records.json.gz").read_bytes()))
        cls.files = {r["path"]: base64.b64decode(r["base64"]) for r in records}
        cls.rows = list(struct.iter_unpack("<8f12I", cls.files["capture/rgba-sample-x/frame-10-storage.bin"]))

    def test_exact_companion_and_clear_packing(self):
        self.assertEqual(data.packing(self.rows, "rgba-sample-x"), [])

    def test_changed_companion_refused(self):
        rows = self.rows.copy()
        i = next(i for i, r in enumerate(rows) if r[8])
        r = list(rows[i]); r[18] ^= 1; rows[i] = tuple(r)
        self.assertEqual(data.packing(rows, "rgba-sample-x"), [i])

    def test_nonzero_uncovered_companion_refused(self):
        rows = self.rows.copy()
        i = next(i for i, r in enumerate(rows) if not r[8])
        r = list(rows[i]); r[18] = 1; rows[i] = tuple(r)
        self.assertEqual(data.packing(rows, "rgba-sample-x"), [i])

    def test_coherent_owner_change_still_fails_geometry(self):
        rows = self.rows.copy()
        i = (44 * 64 + 6) * 4 + 3
        r = list(rows[i]); r[8] = 0x80000002; r[9] = r[16] = 2; rows[i] = tuple(r)
        self.assertEqual(data.packing(rows, "rgba-sample-x"), [])
        fixture = json.loads(self.files["capture/fixture.json"])
        frame = json.loads(self.files["capture/rgba-sample-x/frame-10.json"])
        result = data.prior.inspection(rows, fixture["measured_sample_offsets"], data.POSES[-1], data.POSES[-2], frame["owner_words"], True)
        self.assertEqual(result["mismatches"], [[6, 44, 3, "owner/depth/previous correspondence"]])


if __name__ == "__main__":
    unittest.main()
