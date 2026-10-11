import importlib.util
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location('native_physics_run', Path(__file__).parents[1] / 'probes/native-physics/run.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class NativePhysicsParity(unittest.TestCase):
    def fixture(self):
        data = bytearray(32 + 35 * 641 * 169)
        struct.pack_into('<8I', data, 0, 0x4e503244, 1, 35, 641, 169, 16, 8, 1)
        return data

    def test_full_planar_numeric_and_sleep_fields_beyond_first_frame_are_compared(self):
        reference = self.fixture()
        for offset, field in [(0, 'stable ID'), (32, 'numeric field 2'),
                              (56, 'numeric field 5'), (80, 'numeric field 8'),
                              (160, 'numeric field 18'), (168, 'sleeping')]:
            other = bytearray(reference)
            other[32 + (35 + 2) * 169 + offset] = 1
            with self.assertRaisesRegex(ValueError, f'frame 1, body 3, {field}'):
                probe.compare(reference, other, '2d')

    def test_wrong_dimension_missing_frame_and_unexercised_coverage_fail(self):
        reference = self.fixture()
        probe.inspect(reference, '2d')
        with self.assertRaises(ValueError):
            probe.inspect(reference, '3d')
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            probe.inspect(reference[:-169], '2d')
        for word in (5, 6, 7):
            other = bytearray(reference)
            struct.pack_into('<I', other, word * 4, 0)
            with self.assertRaisesRegex(ValueError, 'coverage'):
                probe.inspect(other, '2d')


PROBE = probe

class NativePhysicsParityTests(unittest.TestCase):
    def reference(self):
        data=bytearray(32+35*641*121)
        struct.pack_into('<8I',data,0,0x4e504831,1,35,641,121,16,8,2)
        return data

    def test_angular_and_sleep_differences_are_reported_at_the_actual_frame(self):
        before=self.reference()
        for field,offset in [('numeric field 10',16+10*8),('sleeping',120)]:
            after=before.copy();after[32+(12*35+7)*121+offset]=1
            with self.assertRaisesRegex(ValueError,f'frame 12, body 8, {field}'):
                PROBE.compare(before,after)

    def test_missing_last_tick_or_unexercised_sleep_is_rejected(self):
        data=self.reference()
        with self.assertRaisesRegex(ValueError,'incomplete'):
            PROBE.inspect(data[:-121])
        struct.pack_into('<I',data,20,0)
        with self.assertRaisesRegex(ValueError,'missing native execution coverage'):
            PROBE.inspect(data)


if __name__ == '__main__':
    unittest.main()
