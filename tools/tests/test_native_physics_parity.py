import importlib.util
from pathlib import Path
import struct
import unittest

PATH=Path(__file__).resolve().parents[1]/'probes/native-physics/run.py'
SPEC=importlib.util.spec_from_file_location('native_physics_parity',PATH)
PROBE=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)

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
