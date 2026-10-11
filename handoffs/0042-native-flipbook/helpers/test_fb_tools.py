#!/usr/bin/env python3
"""Self-tests for the handoff 0042 helpers. Standard library only; no engine calls.

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/test_fb_tools.py
"""

import json
import subprocess
import sys
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
PACKET = HELPERS.parent
sys.path.insert(0, str(HELPERS))
import fb_art  # noqa: E402
import fb_png  # noqa: E402
import fb_scene as fs  # noqa: E402


def item(frames=5, rate=(50, 7), looping=True, playing=True, controls=()):
    return {"frames": ["f"] * frames, "rate": list(rate), "looping": looping, "playing": playing,
            "controls": [list(c) for c in controls]}


def stepped(it, ticks):
    """Incremental reference: remainder clock stepped one tick at a time (as the brief
    describes the runtime), used only to cross-check the closed-form model."""
    num, den = it["rate"]
    d, n = den * fs.TICK_RATE, len(it["frames"])
    frame, rem, playing, completed = 0, 0, it["playing"], False
    controls = {}
    for c in it["controls"]:
        controls.setdefault(c[0], []).append(c[1:])
    out = {}
    for t in range(max(ticks) + 1):
        if t and playing and not completed:
            phase = rem + num
            nxt, rem = frame + phase // d, phase % d
            if it["looping"]:
                frame = nxt % n
            elif nxt >= n:
                frame, rem, completed, playing = n - 1, 0, True, False
            else:
                frame = nxt
        for action in controls.get(t, []):
            if action[0] == "play":
                if completed:
                    frame, rem, playing, completed = 0, 0, True, False
                else:
                    playing = True
            elif action[0] == "pause":
                playing = False
            elif action[0] == "restart":
                frame, rem, playing, completed = 0, 0, True, False
            else:
                frame, rem, completed = action[1], 0, False
        if t in ticks:
            out[t] = (frame, playing, completed)
    return out


class Tests(unittest.TestCase):
    def test_png_round_trip_and_crop(self):
        rows = [[(x * 20, y * 30, 7, 255 if (x + y) % 2 else 0) for x in range(5)] for y in range(3)]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "a.png"
            fb_png.write_rgba(path, 5, 3, rows)
            img = fb_png.read(path)
            self.assertEqual([img.px(x, y) for y in range(3) for x in range(5)],
                             [p for row in rows for p in row])
            crop = img.crop(-1, 1, 3, 3)
            self.assertEqual(crop.px(0, 0), (0, 0, 0, 0))
            self.assertEqual(crop.px(1, 0), rows[1][0])
            self.assertEqual(crop.px(2, 2), (0, 0, 0, 0))

    def test_ulid_hex_matches_crockford_value(self):
        self.assertEqual(fs.ulid_hex("00000000000000000000000001"), "0" * 31 + "1")
        self.assertEqual(fs.ulid_hex("7ZZZZZZZZZZZZZZZZZZZZZZZZZ"), "f" * 32)
        self.assertEqual(fs.ulid_hex(fs.entity_id(1))[-1], "1")

    def test_closed_form_model_matches_stepped_reference(self):
        cases = [item(), item(rate=(150, 1)), item(rate=(1000, 1)), item(rate=(1, 7)), item(rate=(13, 3)),
                 item(looping=False), item(frames=1, looping=False, rate=(2, 1)),
                 item(controls=[(12, "pause"), (20, "play")]),
                 item(controls=[(10, "pause"), (16, "seek", 3), (25, "play"), (41, "restart")]),
                 item(looping=False, controls=[(60, "play")]), item(playing=False, controls=[(33, "play")]),
                 item(controls=[(16, "pause"), (16, "restart")]),
                 item(controls=[(16, "restart"), (16, "pause"), (50, "play")]),
                 item(looping=False, controls=[(50, "seek", 1), (59, "play")]),
                 item(looping=False, controls=[(41, "pause"), (45, "play")]),
                 item(looping=False, controls=[(20, "seek", 4), (30, "pause"), (31, "seek", 0)])]
        ticks = list(range(0, 130)) + [419, 420, 2099, 2100]
        for case in cases:
            self.assertEqual(fs.segments_model(case, ticks), stepped(case, ticks), case)

    def test_fraction_cross_check_and_exact_boundary(self):
        it = item()
        self.assertEqual(fs.fraction_check(it, 41)[0], 4)
        self.assertEqual(fs.fraction_check(it, 42)[0], 0)  # 42 * 50 / 420 = 5 exactly
        self.assertEqual(Fraction(50, 7) * 2100 / 60, 250)

    def test_scene_controls_and_positions(self):
        sc = fs.build()
        anims = [i for i in sc.items if i["kind"] == "anim"]
        self.assertEqual(len(anims), 55)
        self.assertLessEqual(len(sc.items), 4096)
        request = fs.request(sc, fs.MAIN_TICKS)
        self.assertEqual([c["tick"] for c in request["controls"]], sorted(c["tick"] for c in request["controls"]))
        c7 = fs.ulid_hex(fs.eid(sc, "C7-pause-then-restart"))
        self.assertEqual([c["action"]["kind"] for c in request["controls"] if c["entity"] == c7],
                         ["pause", "restart"])
        moving = sc.by_name["D3-moving-paused"]
        x = moving["t"][0]
        for _ in range(69):
            x = x + 3.0 * (1.0 / 60.0)
        self.assertEqual(fs.position(moving, 69)[0], x)
        self.assertEqual(fs.position(sc.by_name["A1-nearest"], 69), sc.by_name["A1-nearest"]["t"])

    def test_control_entities_replace_flipbooks_and_keep_ids(self):
        sc = fs.build()
        manifest = {"frames": {n: {"id": fb_art.frame_id(n)} for n in fb_art.FRAMES},
                    "tiles": {n: {"id": fb_art.frame_id(n)} for n in fb_art.TILES},
                    "sheet": {"regions": {n: [0, 0, 1, 1] for n in fb_art.FRAMES}},
                    "plates": {b: [0, 0, 4, 4] for b in "ABCDE"}, "badge": [10, 10]}
        assets = fs.Assets({"atlas": "A", "sheet": "S", "badge": "B", "plates": "P"}, manifest)
        animated = fs.animated_entities(sc, assets)
        control = fs.control_entities(sc, assets, 42)
        self.assertEqual([e["id"] for e in animated], [e["id"] for e in control])
        self.assertFalse(any("SpriteFlipbook" in e["components"] or "Velocity" in e["components"] for e in control))
        by_name = {e["name"]: e for e in control}
        c3 = by_name["C3-once-hold-replay"]["components"]["SpriteRenderer"]
        self.assertEqual(c3["region"], {"frame": fb_art.frame_id("f4")})  # completed once sequence holds f4
        a7 = by_name["A7-reversed"]["components"]["SpriteRenderer"]
        self.assertEqual(a7["region"], {"frame": fb_art.frame_id("f4")})  # index 0 of the reversed list
        static = {e["name"]: e for e in animated}["A1-nearest"]["components"]["SpriteRenderer"]["region"]
        self.assertEqual(static, {"frame": fb_art.frame_id("t2")})  # deliberately wrong unused region

    def test_art_regenerates_committed_copy(self):
        committed = PACKET / "art"
        if not (committed / "manifest.json").exists():
            self.skipTest("art not committed yet")
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "art"
            subprocess.run([sys.executable, "-I", "-B", str(HELPERS / "fb_art.py"), str(out)], check=True,
                           capture_output=True)
            self.assertEqual(sorted(p.name for p in out.iterdir()), sorted(p.name for p in committed.iterdir()))
            for path in out.iterdir():
                self.assertEqual(path.read_bytes(), (committed / path.name).read_bytes(), path.name)
            manifest = json.loads((out / "manifest.json").read_text())
            self.assertTrue(all(o > 0 for r in manifest["sheet"]["regions"].values() for o in r[:2]))


if __name__ == "__main__":
    unittest.main(verbosity=1)
