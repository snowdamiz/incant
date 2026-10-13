"""Review aids for the 0070-ssao-filter-002 appearance renders (derived after
GPU release from the retained PNGs; not new renders, not acceptance evidence).

Every 002 PNG is decoded and checked against the run's index.json rgba_sha256.
Pairs show build003 (shipped sampling, left) beside 002 (C2, right) with the
build003 review's own rectangles and gains (../../appearance-review, read
only): signed mean-RGB differences, blue darker and red brighter. It also
measures every pixel an SSAO-only frame makes brighter than its off frame.
Writes create-new PNGs and a manifest.

  /opt/homebrew/bin/python3 -I -B make_review.py
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "appearance-review"))
import pngtool  # noqa: E402

B003 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-003/outputs/appearance")
F002 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-002/outputs/appearance")
GAP = 6


def load(directory):
    index = {f["label"]: f for f in json.loads((directory / "index.json").read_text())["frames"]}
    frames = {}
    for label, entry in index.items():
        size, rgba = pngtool.decode(directory / entry["file"])
        assert list(size) == entry["size"] and hashlib.sha256(rgba).hexdigest() == entry["rgba_sha256"], label
        frames[label] = (size, rgba, hashlib.sha256((directory / entry["file"]).read_bytes()).hexdigest())
    return frames


def side(left, right):
    (w, h), a = left
    _, b = right
    out = bytearray()
    for y in range(h):
        out += a[y * w * 4:(y + 1) * w * 4] + bytes((255, 255, 255, 255)) * GAP + b[y * w * 4:(y + 1) * w * 4]
    return (2 * w + GAP, h), bytes(out)


def brighter(on, off, size):
    """Pixels where the SSAO frame's mean RGB exceeds the off frame's."""
    w = size[0]
    found = []
    for i in range(0, len(on), 4):
        d = sum(on[i + c] - off[i + c] for c in range(3)) / 3
        if d > 0:
            found.append(((i // 4) % w, (i // 4) // w, round(d, 2), max(on[i + c] - off[i + c] for c in range(3))))
    return {"pixels": len(found), "max_mean_code": max((f[2] for f in found), default=0),
            "max_channel_code": max((f[3] for f in found), default=0),
            "bbox": [min(f[0] for f in found), min(f[1] for f in found), max(f[0] for f in found),
                     max(f[1] for f in found)] if found else None, "first": found[:12]}


def main():
    b003, f002 = load(B003), load(F002)
    items, outputs = {}, {}
    for view, gain in (("v1", 16), ("v2", 16), ("v4", 12)):
        pair = []
        for frames in (b003, f002):
            size, on, _ = frames[f"{view}_ssao"]
            pair.append((size, pngtool.diff(on, frames[f"{view}_off"][1], gain)))
        outputs[f"diff_{view}_ssao_b003_vs_002.png"] = (side(*pair), f"{view}_ssao minus {view}_off, mean RGB x{gain}: "
                                                                     "build003 (left) | 002 C2 (right)")
    pair = []
    for frames in (b003, f002):
        size, on, _ = frames["v3_both_8"]
        pair.append((size, pngtool.diff(on, frames["v3_off_8"][1], 12)))
    outputs["diff_v3_both_8_b003_vs_002.png"] = (side(*pair), "v3_both_8 minus v3_off_8, mean RGB x12: "
                                                              "build003 | 002 C2")
    for name, view, rect, scale in (("crop_v2_pillar_stripes", "v2", [185, 215, 250, 180], 2),
                                    ("crop_v2_sphere_crate", "v2", [470, 260, 240, 140], 2),
                                    ("crop_v4_crate_sphere", "v4", [180, 120, 230, 220], 2)):
        crops = [pngtool.crop(frames[f"{view}_ssao"][0], frames[f"{view}_ssao"][1], rect, scale)
                 for frames in (b003, f002)]
        outputs[f"{name}_ssao_b003_vs_002.png"] = (side(*crops), f"{view}_ssao crop {rect} x{scale}: build003 | 002 C2")
    for name, ((size, data), description) in outputs.items():
        path = HERE / name
        if path.exists():
            raise SystemExit(f"{path} exists; review aids are never replaced")
        pngtool.encode(path, size, data)
        items[name] = {"description": description, "size": list(size),
                       "png_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    bright = {}
    for view in ("v1", "v2", "v4"):
        for tag, frames in (("build003", b003), ("002", f002)):
            size, on, _ = frames[f"{view}_ssao"]
            bright[f"{view}_{tag}"] = brighter(on, frames[f"{view}_off"][1], size)
    manifest = {"format": "incant-0070-ssao-filter-002-review-v1",
                "note": "Derived after release from retained PNGs, decoded and verified against index.json; "
                        "not renders and not acceptance evidence.",
                "tool": {"make_review.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                         "pngtool.py": hashlib.sha256((HERE.parents[1] / "appearance-review" / "pngtool.py")
                                                      .read_bytes()).hexdigest()},
                "sources": {"build003": {k: v[2] for k, v in b003.items()}, "002": {k: v[2] for k, v in f002.items()}},
                "outputs": items, "ssao_brighter_than_off": bright}
    with open(HERE / "manifest.json", "x") as out:
        json.dump(manifest, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({"outputs": {k: v["size"] for k, v in items.items()},
                      "ssao_brighter_than_off": {k: {kk: v[kk] for kk in ("pixels", "max_mean_code", "max_channel_code",
                                                                            "bbox")} for k, v in bright.items()}},
                     indent=1))


if __name__ == "__main__":
    main()
