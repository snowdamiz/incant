#!/usr/bin/env python3
"""CPU-only analysis of handoff 0062 captures (after the GPU slot release).

Decodes every PNG with the stdlib reader in pngio.py, compares baseline and
candidate pairs, compares source-free re-renders with pre-deletion frames and
evaluates in-scene controls whose crops must match within one capture. Writes
<out>/analysis.json; never renders anything.

  python3 -I -B analyze.py <evidence-dir> <out-json>
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402

BACKGROUND = (20, 21, 25, 255)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Frame:
    def __init__(self, path):
        data = Path(path).read_bytes()
        self.path, self.file_sha = str(path), sha(data)
        self.chunks = pngio.chunk_types(data)
        self.width, self.height, self.rows = pngio.read(data)
        self.rgba_sha = sha(b"".join(self.rows))

    def px(self, x, y):
        return tuple(self.rows[y][4 * x:4 * x + 4])

    def crop(self, x, y, w, h, flip_x=False, flip_y=False):
        ys = range(y + h - 1, y - 1, -1) if flip_y else range(y, y + h)
        out = []
        for yy in ys:
            row = [self.px(xx, yy) for xx in range(x, x + w)]
            out.append(row[::-1] if flip_x else row)
        return out

    def bbox(self, x0, y0, x1, y1):
        """Bounding box of non-background pixels inside [x0,x1) x [y0,y1)."""
        xs, ys = [], []
        for y in range(max(0, y0), min(self.height, y1)):
            for x in range(max(0, x0), min(self.width, x1)):
                if self.px(x, y) != BACKGROUND:
                    xs.append(x)
                    ys.append(y)
        return (min(xs), min(ys), max(xs) + 1, max(ys) + 1) if xs else None


def compare(a, b):
    differing = sum(1 for ra, rb in zip(a, b) for pa, pb in zip(ra, rb) if pa != pb)
    worst = max((max(abs(p - q) for p, q in zip(pa, pb)) for ra, rb in zip(a, b) for pa, pb in zip(ra, rb)),
                default=0)
    total = sum(len(r) for r in a)
    return {"pixels": total, "differing": differing, "worst_channel_delta": worst}


def control(frame, name, a, b, **flips):
    """a, b: (x, y, w, h); b is optionally flipped before comparison."""
    result = compare(frame.crop(*a), frame.crop(*b, **flips))
    return {"name": name, "a": list(a), "b": list(b), "flips": flips, **result}


def located(frame, name, search_a, search_b, **flips):
    """Compare the non-background bounding boxes found in two search windows."""
    box_a, box_b = frame.bbox(*search_a), frame.bbox(*search_b)
    if not box_a or not box_b:
        return {"name": name, "error": "empty search window", "box_a": box_a, "box_b": box_b}
    wa, ha = box_a[2] - box_a[0], box_a[3] - box_a[1]
    wb, hb = box_b[2] - box_b[0], box_b[3] - box_b[1]
    entry = {"name": name, "box_a": list(box_a), "box_b": list(box_b), "flips": flips}
    if (wa, ha) != (wb, hb):
        entry["error"] = "bounding boxes differ in size"
        return entry
    entry.update(compare(frame.crop(box_a[0], box_a[1], wa, ha), frame.crop(box_b[0], box_b[1], wb, hb, **flips)))
    return entry


def board_a(frame):
    out = [
        located(frame, "A6 half-pixel tie 2x == A2 aligned 2x", (366, 30, 426, 90), (66, 30, 126, 90)),
        located(frame, "A7 tie flipped xy == A2 flipped xy", (426, 30, 486, 90), (66, 30, 126, 90),
                flip_x=True, flip_y=True),
        control(frame, "C5 mirrored parent == C2 flip_x", (348, 258, 72, 72), (96, 258, 72, 72)),
        control(frame, "C6 mirrored parent + flip_x == C1", (432, 258, 72, 72), (12, 258, 72, 72)),
        control(frame, "C8 rot180 == C4 flip_xy", (600, 258, 72, 72), (264, 258, 72, 72)),
        control(frame, "C2 flip_x == C1 mirrored", (96, 258, 72, 72), (12, 258, 72, 72), flip_x=True),
        control(frame, "C3 flip_y == C1 flipped vertically", (180, 258, 72, 72), (12, 258, 72, 72), flip_y=True),
    ]
    # Exact 2x replication: every 6x6 block of A2 equals the matching 3x3 block of A1.
    a1, a2 = frame.crop(24, 48, 24, 24), frame.crop(72, 36, 48, 48)
    replicated = all(a2[y][x] == a1[y // 2][x // 2] for y in range(48) for x in range(48))
    out.append({"name": "A2 2x nearest == A1 1x replicated 2x2", "exact": replicated})
    return out


def board_w(frame):
    return [
        control(frame, "W1 trailing 144 px == W1c tail control", (312, 81, 144, 18), (312, 111, 144, 18)),
        control(frame, "W2 flipped leading 144 px == W2c control", (504, 441, 144, 18), (504, 411, 144, 18)),
        control(frame, "W3 bottom 144 px == W3c control", (651, 102, 18, 144), (621, 102, 18, 144)),
        # W6 (rows 321.5-339.5, extending far left) is drawn over both W4 and W4c in rows
        # 321-338 by entity-ID painter order: a fixture overlap, so compare the rest.
        control(frame, "W4 flipped top rows 294-320 == W4c (above W6 overlap)", (291, 294, 18, 27),
                (321, 294, 18, 27)),
        control(frame, "W4 flipped rows 339-437 == W4c (below W6 overlap)", (291, 339, 18, 99),
                (321, 339, 18, 99)),
        {"name": "W4/W4c rows 321-338 are the W6 strip drawn on top (each column constant over the band)",
         "exact": all(frame.px(x, y) == frame.px(x, 321) for y in range(321, 339) for x in range(285, 345))
         and frame.px(300, 321) != frame.px(300, 320)},
        control(frame, "W5 linear trailing 135 px == W5c (excluding first texel)", (321, 171, 135, 18),
                (321, 201, 135, 18)),
        located(frame, "W6 half-pixel tie tail == W6c tie control", (672, 316, 820, 345), (672, 346, 820, 375)),
    ]


FLIP_COLUMNS = {"F1": 504, "F2": 600, "F3": 696, "F4": 792, "F5": 888}


def board_b(frame, tick):
    out = [control(frame, "T1 tilemap == T1c raw cell sprites", (30, 114, 192, 96), (30, 264, 192, 96))]
    row = 144 if tick == 0 else 234
    which = "first" if tick == 0 else "final"
    for name, x in FLIP_COLUMNS.items():
        out.append(control(frame, f"{name} at tick {tick} == {name}c-{which}", (x, 54, 72, 72), (x, row, 72, 72)))
    out.append(control(frame, f"F6 paused at tick {tick} == F6c-held", (504, 354, 72, 72), (600, 354, 72, 72)))
    x7 = 792 if tick == 0 else 888
    out.append(control(frame, f"F7 rot90 at tick {tick} == F7c-{which}", (696, 354, 72, 72), (x7, 354, 72, 72)))
    if tick != 0:
        out.append(control(frame, "F8 rot30 linear at final == F8c-final (+96 px)", (490, 446, 100, 94),
                           (586, 446, 100, 94)))
    return out


ANIMATED_PP = {"F1": (504, 54, 576, 126), "F2": (600, 54, 672, 126), "F3": (696, 54, 768, 126),
               "F4": (792, 54, 864, 126), "F5": (888, 54, 960, 126), "F7": (696, 354, 768, 426),
               "F8": (490, 445, 590, 540)}


def changed_region(a, b):
    xs, ys, count = [], [], 0
    for y in range(a.height):
        ra, rb = a.rows[y], b.rows[y]
        if ra == rb:
            continue
        for x in range(a.width):
            if ra[4 * x:4 * x + 4] != rb[4 * x:4 * x + 4]:
                xs.append(x)
                ys.append(y)
                count += 1
    inside = [sum(1 for x, y in zip(xs, ys) if box[0] <= x < box[2] and box[1] <= y < box[3])
              for box in ANIMATED_PP.values()]
    outside = sum(1 for x, y in zip(xs, ys)
                  if not any(b[0] <= x < b[2] and b[1] <= y < b[3] for b in ANIMATED_PP.values()))
    return {"changed_pixels": count, "bbox": [min(xs), min(ys), max(xs) + 1, max(ys) + 1] if xs else None,
            "per_animated_box_pp": dict(zip(ANIMATED_PP, inside)), "outside_animated_boxes_pp": outside}


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    captures = evidence / "captures"
    report = {"pairs": [], "source_free": [], "controls": {}, "tick_change": {}}
    frames = {}
    for directory in sorted((captures / "baseline").iterdir()):
        for png in sorted(directory.glob("*.png")):
            base = Frame(png)
            cand = Frame(captures / "candidate" / directory.name / png.name)
            frames[(directory.name, png.name)] = cand
            report["pairs"].append({
                "case": directory.name, "png": png.name, "size": [cand.width, cand.height],
                "chunks": cand.chunks, "baseline_file_sha256": base.file_sha, "candidate_file_sha256": cand.file_sha,
                "baseline_rgba_sha256": base.rgba_sha, "candidate_rgba_sha256": cand.rgba_sha,
                "file_identical": base.file_sha == cand.file_sha, "rgba_identical": base.rgba_sha == cand.rgba_sha,
                **compare(base.crop(0, 0, base.width, base.height), cand.crop(0, 0, cand.width, cand.height))})
    for directory in sorted((captures / "candidate").glob("*-source-free")):
        case = directory.name.removesuffix("-source-free")
        for png in sorted(directory.glob("*.png")):
            free = Frame(png)
            before = frames[(case, png.name)]
            report["source_free"].append({"case": case, "png": png.name, "file_sha256": free.file_sha,
                                          "identical_to_pre_deletion": free.file_sha == before.file_sha,
                                          "rgba_identical": free.rgba_sha == before.rgba_sha})
    report["controls"]["A-pp"] = board_a(frames[("A-pp", "tick-00000.png")])
    report["controls"]["W-pp"] = board_w(frames[("W-pp", "tick-00000.png")])
    report["controls"]["B-pp tick 0"] = board_b(frames[("B-pp", "tick-00000.png")], 0)
    report["controls"]["B-pp tick 48"] = board_b(frames[("B-pp", "tick-00048.png")], 48)
    report["tick_change"]["B-pp 0->48"] = changed_region(frames[("B-pp", "tick-00000.png")],
                                                         frames[("B-pp", "tick-00048.png")])
    report["tick_change"]["B-ortho 0->48"] = changed_region(frames[("B-ortho", "tick-00000.png")],
                                                            frames[("B-ortho", "tick-00048.png")])
    report["summary"] = {
        "pairs": len(report["pairs"]),
        "pairs_file_identical": sum(p["file_identical"] for p in report["pairs"]),
        "pairs_rgba_identical": sum(p["rgba_identical"] for p in report["pairs"]),
        "source_free_identical": sum(s["identical_to_pre_deletion"] for s in report["source_free"]),
        "source_free_total": len(report["source_free"]),
    }
    with open(out, "x") as handle:
        json.dump(report, handle, indent=1)
        handle.write("\n")
    print(json.dumps(report["summary"]))
    for group, entries in report["controls"].items():
        for entry in entries:
            status = entry.get("exact", entry.get("differing", entry.get("error")))
            print(f"{group:13s} {entry['name']:58s} -> {status}"
                  + (f" worst {entry['worst_channel_delta']}" if entry.get("differing") else ""))
    print(json.dumps(report["tick_change"]))


if __name__ == "__main__":
    main()
