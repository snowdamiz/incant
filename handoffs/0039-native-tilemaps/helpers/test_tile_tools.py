#!/usr/bin/env python3
"""Self-tests for the handoff 0039 helpers. No engine calls.

  python3 -I -B handoffs/0039-native-tilemaps/helpers/test_tile_tools.py
"""

import json
import math
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
sys.path.insert(0, str(HELPERS))
import tile_model as tm  # noqa: E402
import tile_scene as ts  # noqa: E402

ART = HELPERS.parent / "art"
MANIFEST = json.loads((ART / "manifest.json").read_text(encoding="utf-8"))


def tiny(rows, camera_t, reference, kind="map", t=(0.0, 0.0), s=(1.0, 1.0)):
    """A one-item description viewed by one pixel-perfect camera at 16 px/m."""
    item = {"name": "M", "kind": kind, "slot": 1, "parent": None, "t": list(t), "r": 0.0, "s": list(s)}
    if kind == "map":
        item.update(size=[len(rows[0]), len(rows)], cell=[1.0, 1.0],
                    layers=[{"id": ts.layer_id(1), "rows": rows, "layer": 0, "order": 0, "label": "l"}])
    else:
        item.update(texture=["frame", rows], size=[1.0, 1.0], pivot=[0.0, 0.0], layer=0, order=0)
    camera = {"t": list(camera_t), "projection": {"kind": "pixel_perfect", "pixels_per_unit": 16,
                                                   "reference_resolution": list(reference)}}
    return {"items": [item], "ids": {"M": ts.entity_id(1 << 20)}, "cameras": {"c": camera},
            "views": {"v": ["c", list(reference)]}}


class SceneTests(unittest.TestCase):
    def test_rows_are_authored_top_down_and_stored_bottom_up(self):
        self.assertEqual(ts.cells_bottom_up(["AB", "C."]), ["C", None, "A", "B"])

    def test_control_ids_reproduce_documented_painter_order(self):
        for state in ("s0", "s1"):
            sc = ts.build(state)
            ids = {"atlas": "AT", "tiles": "TI", "plates": "PL", "badge": "BA"}
            control = ts.control_entities(sc, ts.Assets("control", ids, MANIFEST))
            drawn = [(e["components"]["SpriteRenderer"]["layer"], e["components"]["SpriteRenderer"]["order"],
                      e["id"], e["name"]) for e in control if "SpriteRenderer" in e["components"]]
            control_order = [d[3] for d in sorted(drawn)]
            textures = tm.Textures(ART, MANIFEST)
            items = tm.draw_items(json.loads(json.dumps(ts.describe(sc))), textures)
            documented = []
            for item in reversed(items):  # draw_items is topmost first
                name = item["name"]
                if name.count(":") >= 3:  # map cell: MAP:label:x,y:frame
                    name = name.rsplit(":", 1)[0]
                documented.append(name)
            self.assertEqual(control_order, documented, state)

    def test_control_cells_stay_between_neighbouring_entity_ids(self):
        sc = ts.build("s0")
        ids = {"atlas": "AT", "tiles": "TI", "plates": "PL", "badge": "BA"}
        control = ts.control_entities(sc, ts.Assets("control", ids, MANIFEST))
        for item in sc.items:
            if item["kind"] != "map":
                continue
            low, high = ts.eid(sc, item["name"]), ts.entity_id((item["slot"] + 1) << ts.SLOT_BITS)
            cells = [e["id"] for e in control if e["parent"] == low]
            self.assertTrue(cells and all(low < c < high for c in cells), item["name"])


class ModelTests(unittest.TestCase):
    textures = tm.Textures(ART, MANIFEST)

    def test_cell_zero_is_lower_left_and_frame_rows_are_top_down(self):
        # Map 1x2 at the origin, camera centred on it: 16 x 32 logical pixels.
        d = tiny(["A", "B"], (0.5, 1.0), (16, 32))
        (w, h), colours, _ = tm.predict(d, self.textures, "v")
        corners = MANIFEST["colours"]["corners"]
        self.assertEqual(colours[0], tuple(corners["tl"]))                   # top-left of A (row 1)
        self.assertEqual(colours[(h - 1) * w], tuple(corners["bl"]))         # bottom-left of B (row 0)
        self.assertEqual(colours[15 * w], tuple(corners["bl"]))              # A's bottom row ends at y = 1 m
        self.assertEqual(colours[16 * w], tuple(corners["tl"]))              # B's top row starts below it

    def test_exact_texel_tie_takes_increasing_screen_x_and_is_flagged(self):
        # Ruler W is 24 texels wide; at 16 px per cell texel 3 ends at x = 2.0 px. Shift
        # the sprite by half a pixel so that boundary passes through pixel centre 2.5.
        d = tiny("W", (0.5, 0.5), (16, 16), kind="sprite", t=(1 / 32, 0.0))
        screen, _ = tm.view_affine(d["cameras"]["c"], 16, 16)
        p = tm.Prepared(tm.draw_items(d, self.textures)[0], screen)
        self.assertEqual(p.biased_texel(2.5, 8.5)[0], 3)
        self.assertTrue(tm.near_tie([p], 2.5, 8.5))
        self.assertFalse(tm.near_tie([p], 3.5, 8.5))

    def test_pixel_perfect_snapping_matches_camera_rule(self):
        cam = {"t": [0.03, -0.03], "projection": {"kind": "pixel_perfect", "pixels_per_unit": 16,
                                                  "reference_resolution": [320, 180]}}
        m, _ = tm.view_affine(cam, 320, 180)
        self.assertAlmostEqual(m[2], 0.0 - 10.0)    # x: 0.48 px -> ceil(-0.02) = 0
        self.assertAlmostEqual(m[5], 0.0 + 5.625)   # y: -0.48 px -> floor(0.02) = 0
        odd = {"t": [0.0, 0.0], "projection": {"kind": "pixel_perfect", "pixels_per_unit": 16,
                                               "reference_resolution": [321, 181]}}
        m, _ = tm.view_affine(odd, 321, 181)
        self.assertAlmostEqual(m[2], 0.5 / 16 - 321 / 32)
        self.assertAlmostEqual(m[5], -0.5 / 16 + 181 / 32)

    def test_rotation_reflection_and_composition(self):
        a, b, d, e = tm.quat_matrix(90.0, (1.0, 1.0))
        for got, want in zip((a, b, d, e), (0.0, -1.0, 1.0, 0.0)):
            self.assertAlmostEqual(got, want)
        parent = tm.local_matrix({"t": [2.0, 3.0], "r": 0.0, "s": [1.0, -1.0]})
        child = tm.local_matrix({"t": [1.0, 1.0], "r": 0.0, "s": [2.0, 2.0]})
        x, y = tm.apply(tm.compose(parent, child), 0.5, 0.5)
        self.assertEqual((x, y), (2.0 + 2.0, 3.0 - 2.0))
        inverse = tm.invert(tm.compose(parent, child))
        self.assertTrue(all(math.isclose(p, q, abs_tol=1e-12)
                            for p, q in zip(tm.apply(inverse, x, y), (0.5, 0.5))))


if __name__ == "__main__":
    unittest.main(verbosity=1)
