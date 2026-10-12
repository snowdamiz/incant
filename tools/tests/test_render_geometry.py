"""Exercise the imported-model receipt with the actual checkpoint11 field set."""
from pathlib import Path
import runpy
import unittest

ROOT = Path(__file__).resolve().parents[2]
validate = runpy.run_path(str(ROOT / 'tools/platforms/render-check.py'))['validate_model_geometry']
# Actual Windows checkpoint11 response. This model-only fixture has no text.
GEOMETRY = {
    'diagnostic_entities': 0, 'model_draw_calls': 1, 'model_entities': 2,
    'model_triangles': 2, 'primitive_instances': 2, 'sprite_entities': 0,
    'text_entities': 0, 'text_glyph_quads': 0, 'text_gpu_bytes': 0,
    'text_missing_glyphs': 0, 'tile_cells': 0, 'tilemap_entities': 0,
}


class ModelGeometryReceipt(unittest.TestCase):
    def test_observed_model_only_response_is_accepted(self):
        validate(GEOMETRY)

    def test_all_counts_and_complete_field_set_are_required(self):
        for field in GEOMETRY:
            with self.subTest(field=field):
                wrong = dict(GEOMETRY)
                wrong[field] += 1
                with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
                    validate(wrong)
                missing = dict(GEOMETRY)
                del missing[field]
                with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
                    validate(missing)
        with self.assertRaisesRegex(ValueError, 'Unexpected imported geometry projection'):
            validate(dict(GEOMETRY, unknown_count=0))


if __name__ == '__main__':
    unittest.main()
