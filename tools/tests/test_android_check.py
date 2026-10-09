"""An adb exit status alone must never count as a successful device test."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "android_check", Path(__file__).resolve().parents[1] / "platforms/android-check.py"
)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class InstrumentationResultTests(unittest.TestCase):
    def test_requires_both_success_marker_and_android_result_code(self):
        self.assertTrue(probe.instrumentation_passed(
            "INSTRUMENTATION_RESULT: stream=INCANT_SMOKE_PASS\nINSTRUMENTATION_CODE: -1\n"
        ))
        for output in (
            "", "INSTRUMENTATION_CODE: -1\n", "INCANT_SMOKE_PASS\n",
            "INCANT_SMOKE_PASS\nINSTRUMENTATION_CODE: 0\n",
            "INCANT_SMOKE_FAIL\nINSTRUMENTATION_CODE: -1\n",
            "INCANT_SMOKE_PASS\nINSTRUMENTATION_FAILED: crash\nINSTRUMENTATION_CODE: -1\n",
            "INCANT_SMOKE_PASS\nException in native host\nINSTRUMENTATION_CODE: -1\n",
            "INCANT_SMOKE_PASS\nINSTRUMENTATION_CODE: -1\nINSTRUMENTATION_CODE: 0\n",
        ):
            with self.subTest(output=output):
                self.assertFalse(probe.instrumentation_passed(output))
