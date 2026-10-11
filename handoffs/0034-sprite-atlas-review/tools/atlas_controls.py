#!/usr/bin/env python3
"""Negative controls for the handoff 0034 checks: each check must fail when wrong.

  * strip decoding with a wrong tie side, an origin one texel off and inverted flips;
  * cooked-page verification after flipping one bit in a packed, an extruded and an
    empty texel of the v1 page (copies in a new scratch directory; the run is untouched).

Painter-order controls are recorded by atlas_analyse.py itself. Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_controls.py RUN ANALYSIS.json OUT.json
"""

import argparse
import json
import math
import shutil
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import atlas_analyse as an  # noqa: E402

FRAMES = {"W": "pp-W-1280x200.png", "V": "pp-V-320x1080.png", "X": "pp-X-1024x832.png", "Y": "pp-Y-832x1024.png"}


def wrong_tie(t, length, m, flip):
    bias = -1.0 / (1024.0 * m)
    k = math.floor(length - t - bias) if flip else math.floor(t + bias)
    return min(max(k, 0), length - 1)


def strip_controls(run, scene, phase):
    original = an.expected_index
    out = {}
    for label, rule, offset, invert in (("documented rule", original, 0, False), ("wrong tie side", wrong_tie, 0, False),
                                        ("origin one texel off", original, 1, False),
                                        ("flips inverted", original, 0, True)):
        an.expected_index = rule
        failing = 0
        cases = an.strip_cases(scene)
        for case in cases:
            img = an.image(run / "states/s0-r1" / FRAMES[case["board"]])
            res = an.check_strip(img, case["axis"], case["start"], case["cross"], case["thickness"], case["length"],
                                 case["m"], case["flip"] != invert, case["offset"] + offset, case["base"], phase,
                                 (img.width, img.height))
            failing += bool(res["mismatches"])
        out[label] = {"cases": len(cases), "failing_cases": failing}
        an.expected_index = original
    return out


def page_controls(run, analysis, manifest):
    fp = analysis["fingerprints"]["s0-r1"]
    cache = run / "atlas/.incant/cache/textures"
    meta = json.loads((cache / f"{fp}.json").read_text(encoding="utf-8"))
    w, h = meta["atlas"]["size"]
    card7 = next(f["region"] for f in meta["atlas"]["frames"].values() if f["name"] == "card7")
    out = {}
    with tempfile.TemporaryDirectory() as tmp:
        for label, (x, y) in {"packed texel": (card7[0] + 3, card7[1] + 3),
                              "extruded texel": (card7[0] - 1, card7[1] + 3), "empty texel": (w - 1, h - 1)}.items():
            d = Path(tmp) / label.replace(" ", "-")
            (d / "atlas/.incant/cache/textures").mkdir(parents=True)
            (d / "art").symlink_to(run / "art")
            shutil.copy(cache / f"{fp}.json", d / "atlas/.incant/cache/textures/")
            data = bytearray((cache / f"{meta['content_sha256']}.ktx2").read_bytes())
            i = int.from_bytes(data[80:88], "little") + 4 * (y * w + x)
            data[i] ^= 0x10
            data[i + 3] |= 1
            (d / f"atlas/.incant/cache/textures/{meta['content_sha256']}.ktx2").write_bytes(bytes(data))
            an._CACHE.clear()
            res = an.verify_page(d, fp, manifest, "ballast-v1.png")
            out[label] = {"at": [x, y], "pass": res["pass"],
                          **{k: res[k] for k in ("content_mismatches", "extrusion_mismatches", "nonzero_empty_texels")}}
    an._CACHE.clear()
    out["unmodified"] = {"pass": an.verify_page(run, fp, manifest, "ballast-v1.png")["pass"]}
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    parser.add_argument("analysis", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    if args.out.exists():
        raise SystemExit(f"refusing to overwrite {args.out}")
    run = args.run.resolve()
    analysis = json.loads(args.analysis.read_text(encoding="utf-8"))
    manifest = json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))
    scene = json.loads((run / "scene-atlas.json").read_text(encoding="utf-8"))
    report = {"strips": strip_controls(run, scene, manifest["strip"]["phase"]),
              "pages": page_controls(run, analysis, manifest),
              "painter_order_reversed": {k: v["reversed_order_control_mismatches"]
                                         for k, v in analysis["painter_order"].items()}}
    args.out.write_text(json.dumps(report, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()
