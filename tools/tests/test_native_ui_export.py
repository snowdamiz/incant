import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native


class UiExport(unittest.TestCase):
    def test_unsupported_viewport_is_refused_before_any_project_or_output_io(self):
        for mode, viewport in [('window', '320x200'), ('headless', '0x20'), ('headless', '65537x1'),
                               ('headless', 'nanx1'), ('headless', '1x1x1'), ('headless', '1x2\n'),
                               ('headless', 123)]:
            with self.subTest(mode=mode, viewport=viewport), self.assertRaises(ValueError):
                export_native.export(Path('/not-read/project'), Path('/not-created/output'), mode=mode, ui_viewport=viewport)
        self.assertEqual(export_native.ui_startup_options('headless', None), [])
        self.assertEqual(export_native.ui_startup_options('headless', '65536x1'), ['--ui-viewport', '65536x1'])


if __name__ == '__main__':
    unittest.main()
