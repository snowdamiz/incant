"""Planned image pairs for handoff 0076's personal inspection. Standard library
only; CPU only. Each pair is a magnified, side-by-side crop of the run's own PNGs
(magenta separators, nearest-neighbour scaling, no resampling), or a difference
map; they are viewing aids, never screenshots, and are not counted against the
PNG budget. Called by `analyze.py pairs` after a passing integrity check, and by
selfcheck.py on retained 0072 PNGs as a CPU control.
"""

import struct
import zlib
from pathlib import Path

import pngread
import regions

SEPARATOR = bytes([255, 0, 255, 255])


def write_png(path, width, height, rgba):
    """Create-only 8-bit RGBA PNG, filter 0, deterministic zlib level 9."""
    raw = b"".join(b"\x00" + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(path, "xb") as out:
        out.write(data)


def crop(image, box, scale):
    width, _, rgba = image
    x0, y0, x1, y1 = box
    rows = []
    for y in range(y0, y1):
        line = b"".join(rgba[(y * width + x) * 4:(y * width + x) * 4 + 4] * scale for x in range(x0, x1))
        rows.extend([line] * scale)
    return rows


def side_by_side(panels):
    gap = SEPARATOR * 4
    return [gap.join(panel[i] for panel in panels) for i in range(len(panels[0]))]


def difference(a, b, box, gain):
    """Grey map of the max-channel difference x gain (saturating), inside box."""
    width, _, pa = a
    pb = b[2]
    x0, y0, x1, y1 = box
    rows = []
    for y in range(y0, y1):
        line = bytearray()
        for x in range(x0, x1):
            i = (y * width + x) * 4
            v = min(255, gain * max(abs(pa[i + k] - pb[i + k]) for k in range(3)))
            line += bytes([v, v, v, 255])
        rows.append(bytes(line))
    return rows


def grow_box(box, margin, size=regions.OUTPUT):
    return [max(0, box[0] - margin), max(0, box[1] - margin),
            min(int(size[0]), box[2] + margin), min(int(size[1]), box[3] + margin)]


def boxes(plan):
    """Named crop boxes [x0, y0, x1, y1) from the plan's region and viewport."""
    spec = plan["comparison"]
    rect = [int(v) for v in spec["viewport_rect"]]
    shadow = regions.bbox(regions.region(spec["pose"], spec["region"]))
    card = regions.bbox(regions.mask([tuple(v) for v in regions.pose(spec["pose"])["card"]], 0, spec["viewport_rect"]))
    x0, y0, x1, y1 = rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]
    width, height = spec["pairs"]["border_crop_px"]
    left, right = spec["pairs"]["ground_meets_border_y"]["left"], spec["pairs"]["ground_meets_border_y"]["right"]
    return {"grille-shadow": grow_box(shadow, spec["pairs"]["margin_px"]),
            "card": grow_box(card, spec["pairs"]["margin_px"]),
            "border-left-ground": [x0 - 8, left - height // 2, x0 + width, left + height // 2],
            "border-right-ground": [x1 - width, right - height // 2, x1 + 8, right + height // 2],
            "border-bottom-left": [x0 - 8, y1 - height, x0 + width, y1 + 8],
            "border-bottom-right": [x1 - width, y1 - height, x1 + 8, y1 + 8],
            "viewport": [x0, y0, x1, y1]}


def write_all(directories, output, plan):
    """directories: {role: case output dir holding the plan's two PNGs}. Returns
    the written files' names; the output directory must be new."""
    spec = plan["comparison"]
    images = {(role, kind): pngread.read(Path(directory) / spec[kind])
              for role, directory in directories.items() for kind in ("on", "off")}
    output = Path(output)
    output.mkdir(parents=True)
    named = boxes(plan)
    written = []
    for pair in spec["pairs"]["crops"]:
        box, scale = named[pair["box"]], pair["scale"]
        panels = [crop(images[tuple(panel.split(":"))], box, scale) for panel in pair["panels"]]
        rows = side_by_side(panels)
        name = f"{pair['name']}.png"
        write_png(output / name, len(rows[0]) // 4, len(rows), b"".join(rows))
        written.append({"name": name, "box": box, "scale": scale, "panels": pair["panels"]})
    for pair in spec["pairs"]["differences"]:
        box = named[pair["box"]]
        rows = difference(images[tuple(pair["a"].split(":"))], images[tuple(pair["b"].split(":"))],
                          box, pair["gain"])
        name = f"{pair['name']}.png"
        write_png(output / name, box[2] - box[0], box[3] - box[1], b"".join(rows))
        written.append({"name": name, "box": box, "gain": pair["gain"], "a": pair["a"], "b": pair["b"]})
    return written
