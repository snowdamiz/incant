"""Zoomed stripe-base review aids for 0070-ssao-filter-002 (after release, from
the retained PNGs; not renders, not acceptance evidence). The thin-occluder
comb that N8 measures lives under the four stripes; these crops show the
signed SSAO difference there (mean RGB x16, blue darker, red brighter),
build003 (left) beside 002 C2 (right), x3. Create-new outputs and manifest.

  /opt/homebrew/bin/python3 -I -B make_stripes_review.py
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import make_review as mr  # noqa: E402

CROPS = {"diff_crop_v1_stripes_b003_vs_002.png": ("v1", [290, 260, 160, 160]),
         "diff_crop_v2_stripes_b003_vs_002.png": ("v2", [300, 260, 160, 160])}


def main():
    b003, f002 = mr.load(mr.B003), mr.load(mr.F002)
    items = {}
    for name, (view, rect) in CROPS.items():
        panels = []
        for frames in (b003, f002):
            size, on, _ = frames[f"{view}_ssao"]
            panels.append(mr.pngtool.crop(size, mr.pngtool.diff(on, frames[f"{view}_off"][1], 16), rect, 3))
        path = HERE / name
        if path.exists():
            raise SystemExit(f"{path} exists; review aids are never replaced")
        size, data = mr.side(*panels)
        mr.pngtool.encode(path, size, data)
        items[name] = {"description": f"{view}_ssao minus {view}_off at the stripe bases, crop {rect} x3, mean RGB x16: "
                                      "build003 | 002 C2", "size": list(size),
                       "png_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    manifest = {"format": "incant-0070-ssao-filter-002-review-stripes-v1",
                "tool": {"make_stripes_review.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
                "outputs": items}
    with open(HERE / "manifest-stripes.json", "x") as out:
        json.dump(manifest, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({k: v["size"] for k, v in items.items()}))


if __name__ == "__main__":
    main()
