"""Check triangle geometry in delta-review captures against the camera math.

Usage: python3 -I refresh_geometry.py ANALYSIS_JSONL [NAME_PREFIX ...]

Content is each 2x capture minus its 64 px (32 pt) title bar. With a 60 degree
vertical field of view and the camera 8 units from the model plane, one world
unit spans H / (2 * 8 * tan 30deg) = H / 9.2376 px for content height H. A fresh
render keeps the 1x1-unit right triangle square at that size with its base on
row H/2. A non-uniformly stretched earlier framebuffer changes the width/height
ratio and resamples edges into extra colours (fresh frames here use 4-7 content
colours; resampled stale frames in the original review used 12-55, hence the
<= 8 bound). world_left is the triangle's left
edge in world units, (left_px - W/2) / px_per_unit, so it is comparable across
window sizes. One JSON object per capture is printed.
"""

import json
import math
import pathlib
import sys

TITLE_BAR_PX = 64
UNIT_DIVISOR = 2 * 8 * math.tan(math.radians(30))


def main():
    path = pathlib.Path(sys.argv[1])
    prefixes = sys.argv[2:]
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        name = pathlib.Path(record["path"]).name
        if prefixes and not any(name.startswith(prefix) for prefix in prefixes):
            continue
        width, height = record["w"], record["h"] - TITLE_BAR_PX
        unit = height / UNIT_DIVISOR
        result = {"capture": name, "content_px": [width, height], "expected_side_px": round(unit, 1),
                  "colors": record["distinct_colors"], "foreground_px": record["foreground_px"]}
        bbox = record.get("bbox")
        if bbox:
            side_w, side_h = bbox[2] - bbox[0] + 1, bbox[3] - bbox[1] + 1
            base_row = bbox[3] - TITLE_BAR_PX + 1
            result.update({
                "triangle_px": [side_w, side_h],
                "base_row": base_row,
                "half_height": height / 2,
                "world_left": round((bbox[0] - width / 2) / unit, 3),
                "fresh": abs(side_h - unit) <= 1.5 and abs(side_w - side_h) <= 1
                and abs(base_row - height / 2) <= 1 and record["distinct_colors"] <= 8,
            })
        print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
