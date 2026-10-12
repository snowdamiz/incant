import sys
import io
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native


class UiExport(unittest.TestCase):
    def test_public_cli_preserves_independent_wabt_and_ui_options(self):
        arguments = ['export_native.py', 'project', 'output', '--wabt', 'pinned-wabt',
                     '--ui-viewport', '320x200']
        with patch.object(sys, 'argv', arguments), patch.object(sys, 'stdout', io.StringIO()), \
                patch.object(export_native, 'export', return_value={'target': 'cpu', 'player_sha256': 'checked'}) as export:
            export_native.main()
        export.assert_called_once_with(Path('project'), Path('output'), None, None, 'headless',
                                       wabt=Path('pinned-wabt'), ui_viewport='320x200')

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
