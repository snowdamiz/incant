"""Unit tests for the handoff 0040 helpers (standard library only).

Run: python3 -I handoffs/0040-static-fbx-import/tools/test_tools.py
The twin checks reuse the importer's own numeric oracles from
crates/incant_assets/tests/fbx.rs as independent expectations.
"""
import json
import math
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import art  # noqa: E402
import controls  # noqa: E402
import derive  # noqa: E402
import fbx_probe  # noqa: E402
import pngio  # noqa: E402
import scene  # noqa: E402
import twins  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
FIX = ROOT / derive.FIXTURES


def glb_json(data):
    length = struct.unpack_from("<I", data, 12)[0]
    return json.loads(data[20:20 + length])


def mat_from_trs(node):
    t = node.get("translation", [0, 0, 0])
    x, y, z, w = node.get("rotation", [0, 0, 0, 1])
    s = node.get("scale", [1, 1, 1])
    r = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
         [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
         [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    return [[r[i][0] * s[0], r[i][1] * s[1], r[i][2] * s[2], t[i]] for i in range(3)] + [[0, 0, 0, 1]]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def world_nodes(doc):
    out = {}
    stack = [(n, [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]) for n in doc["scenes"][0]["nodes"]]
    while stack:
        index, parent = stack.pop()
        node = doc["nodes"][index]
        world = mul(parent, mat_from_trs(node))
        out[node["name"]] = world
        stack += [(c, world) for c in node.get("children", [])]
    return out


def origin(m):
    return [m[0][3], m[1][3], m[2][3]]


class Tests(unittest.TestCase):
    def test_png_round_trip(self):
        with tempfile.TemporaryDirectory() as d:
            px = bytes(range(48))
            pngio.write(Path(d) / "a.png", 4, 4, px, 3)
            w, h, c, out = pngio.read(Path(d) / "a.png")
            self.assertEqual((w, h, c, bytes(out)), (4, 4, 3, px))

    def test_ulid_and_look_rotation(self):
        self.assertEqual(scene.ulid("AB", 1), "01JA40AB000000000000000001")
        self.assertEqual(len(scene.ulid("ZZ", 31 ** 5)), 26)
        q = scene.look_rotation((0, 0, 5), (0, 0, 0))
        self.assertTrue(all(abs(a - b) < 1e-12 for a, b in zip(q, [0, 0, 0, 1])))
        # Looking down from +X: local -Z must map to world -X.
        x, y, z, w = scene.look_rotation((5, 0, 0), (0, 0, 0))
        forward = [-(2 * (x * z + y * w)), -(2 * (y * z - x * w)), -(1 - 2 * (x * x + y * y))]
        self.assertTrue(all(abs(a - b) < 1e-12 for a, b in zip(forward, [-1, 0, 0])))

    def test_probe_reads_ascii_and_binary_axes_and_units(self):
        maya = fbx_probe.summarize(FIX / "maya_cube_7500_ascii.fbx")
        self.assertEqual(maya["global_settings"]["UnitScaleFactor"], 1)
        mx = fbx_probe.summarize(FIX / "max_geometry_transform_instances_7700_ascii.fbx")
        self.assertEqual((mx["global_settings"]["UpAxis"], mx["global_settings"]["UnitScaleFactor"]), (2, 2.54))
        suz = fbx_probe.summarize(FIX / "blender_suzanne_multimaterial_7400_binary.fbx")
        geo = next(iter(suz["geometries"].values()))
        self.assertEqual(sum(v["polygons"] for v in geo["polygons_per_material_slot"].values()), 500)
        self.assertEqual(len(next(iter(suz["models"].values()))["materials"]), 7)

    def test_max_twin_matches_importer_numeric_oracle(self):
        # fbx.rs: Box001 geometry origin (1.016, 0, 0); Box002 (0, 0, -1.016).
        data, _ = twins.build(FIX / "max_geometry_transform_instances_7700_ascii.fbx")
        worlds = world_nodes(glb_json(data))
        for name, expected in [("Box001 geometry", [1.016, 0, 0]), ("Box002 geometry", [0, 0, -1.016])]:
            got = origin(worlds[name])
            self.assertTrue(all(abs(a - b) < 1e-6 for a, b in zip(got, expected)), (name, got))

    def test_parented_twin_matches_importer_numeric_oracle(self):
        data, summary = twins.build(FIX / "maya_parented_cubes_7500_ascii.fbx")
        worlds = world_nodes(glb_json(data))
        parent, child = worlds["Parent"], worlds["Child"]
        self.assertTrue(all(abs(a - b) < 1e-12 for a, b in zip(origin(parent), [0.001, 0.002, 0.003])))
        expected = [sum(parent[i][k] * [0, -1, 0, 1][k] for k in range(4)) for i in range(3)]
        self.assertTrue(all(abs(a - b) < 1e-12 for a, b in zip(origin(child), expected)))
        self.assertEqual(summary["models"][1]["order"] if summary["models"][1]["name"] == "Parent" else summary["models"][0]["order"], "ZYX")

    def test_euler_order_semantics(self):
        # XYZ applies X first: rotating +Y by X90 then Z90 gives +Z, not -X.
        q = twins.euler_quat([90, 0, 90], 0)
        m = mat_from_trs({"rotation": q})
        y = [m[0][1], m[1][1], m[2][1]]
        self.assertTrue(all(abs(a - b) < 1e-9 for a, b in zip(y, [0, 0, 1])), y)

    def test_suzanne_twin_keeps_seven_materials_and_faces_plus_z(self):
        data, summary = twins.build(FIX / "blender_suzanne_multimaterial_7400_binary.fbx")
        doc = glb_json(data)
        self.assertEqual(summary["materials"], 7)
        self.assertEqual(len(doc["meshes"][0]["primitives"]), 7)
        world = world_nodes(doc)["Suzanne"]
        # Blender front (-Y) must face +Z after the exporter's -90 X rotation; up (+Z) maps to +Y.
        front = [world[i][1] * -1 for i in range(3)]
        up = [world[i][2] for i in range(3)]
        self.assertTrue(front[2] > 0.99 and up[1] > 0.99, (front, up))

    def test_normal_map_bevels_follow_opengl_convention(self):
        self.assertEqual(art.bevel_normal(0, 40)[1] > 0, True)   # image-up bevel tilts toward +Y
        self.assertEqual(art.bevel_normal(40, 0)[0] > 0, True)   # image-right bevel tilts toward +X
        with tempfile.TemporaryDirectory() as d:
            art.normal_map(Path(d) / "n.png")
            w, h, c, px = pngio.read(Path(d) / "n.png")
            island_x, island_y = 192, 384           # front island (top-left in image space)
            top = (island_y + 64 - 40) * w + island_x + 64   # 40 texels above the centre
            self.assertGreater(px[top * 3 + 1], 200)         # green high = toward image top

    def test_relief_matches_normal_map_geometry(self):
        _, info = controls.relief()
        self.assertAlmostEqual(info["height"], (info["outer_half"] - info["inner_half"]) * math.tan(math.radians(35)))

    def test_derivatives_change_only_named_anchors(self):
        cube = (FIX / derive.CUBE).read_text()
        text, edits = derive.textured(cube, b"x", relative="t.png")
        self.assertIn('C: "OP",90002,1907674217712, "DiffuseColor"', text)
        self.assertEqual(len(edits), 4)
        with self.assertRaises(ValueError):
            derive._replace("a a", "a", "b", [], "duplicate anchor")


if __name__ == "__main__":
    unittest.main(verbosity=2)
