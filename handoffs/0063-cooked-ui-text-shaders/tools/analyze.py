#!/usr/bin/env python3
"""CPU-only analysis of the handoff 0063 captures. Standard library only.

Full-frame results (baseline vs candidate, pre-deletion vs source-free) are
recorded separately from bounded in-scene controls, which compare regions of
one frame. No region is ever excluded from a full-frame comparison.

  python3 -I -B analyze.py <evidence-dir> <new-output-json>
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import boards as bd  # noqa: E402
import common as c  # noqa: E402
from pngio import Raster, chunk_types, diff  # noqa: E402

# Display-referred theme colours (crates/incant_render/src/ui/theme.rs, unchanged since before f8b95de5).
THEME = {"TEXT": (236, 238, 242), "BUTTON": (46, 50, 58), "BUTTON_DISABLED": (35, 38, 44),
         "FIELD": (21, 23, 28), "FIELD_BORDER": (100, 106, 119), "ACCENT": (76, 146, 255)}


def ox(x):
    """Orthographic board W: metres to pixel columns (100 px/m, x = -4.8 at column 0)."""
    return round((x + 4.8) * 100)


def oy(y):
    return round((2.7 - y) * 100)


def flip_h(r):
    rows = [r.rgba[y * r.width * 4:(y + 1) * r.width * 4] for y in range(r.height)]
    return Raster(r.width, r.height, b"".join(b"".join(row[i:i + 4] for i in range(len(row) - 4, -4, -4))
                                              for row in rows))


def flip_v(r):
    rows = [r.rgba[y * r.width * 4:(y + 1) * r.width * 4] for y in range(r.height)]
    return Raster(r.width, r.height, b"".join(reversed(rows)))


def region_stats(r, box):
    x0, y0, x1, y1 = box
    values = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = r.pixel(x, y)
            values[p] = values.get(p, 0) + 1
    mode = max(values, key=values.get)
    return {"box": list(box), "distinct_colours": len(values), "mode": list(mode), "mode_count": values[mode],
            "pixels": (x1 - x0) * (y1 - y0)}


def control(frame, subject, reference, transform, label, background):
    """subject/reference are [x0, y0, x1, y1]; transform maps the subject crop onto the reference crop."""
    a = frame.crop(subject[0], subject[1], subject[2] - subject[0], subject[3] - subject[1])
    b = frame.crop(reference[0], reference[1], reference[2] - reference[0], reference[3] - reference[1])
    mapped = {"none": a, "flip_h": flip_h(a), "flip_v": flip_v(a)}[transform]
    result = diff(mapped, b)
    ink = sum(1 for i in range(0, len(b.rgba), 4) if b.rgba[i:i + 3] != bytes(background[:3]))
    return {"label": label, "subject": subject, "reference": reference, "transform": transform,
            "reference_non_background_pixels": ink, **result}


DRAWN = ("title", "body", "resume", "options", "online", "continue", "clip_a", "clip_b", "clip_c",
         "name_label", "name", "long_label", "long", "arabic_label", "arabic", "cjk_label", "cjk")


def widget_boxes(report_capture, scale, key="bounds"):
    """Physical [l, t, r, b] per widget from the accepted UI input (logical px, snapped like ui.wgsl).
    key "bounds" is the border box; "visible" is the border box after ancestor and viewport clips."""
    def snap(v):
        return int(v * scale + 0.5)
    return {w["id"]: [snap(w[key][0]), snap(w[key][1]), snap(w[key][2]), snap(w[key][3])]
            for w in report_capture["ui"]["widgets"]}


def colour_checks(frame, boxes, ring=None, visible=None):
    """Exact theme values at surface interiors and borders; ring = (widget, physical width).
    A probe outside its widget's visible (clipped) box is recorded as clipped, not as a colour result."""
    ids = bd.U_IDS
    out = []
    owner = {}

    def check(name, x, y, expected):
        box = visible[owner[name]] if visible else [0, 0, frame.width, frame.height]
        if not (box[0] <= x < box[2] and box[1] <= y < box[3] and x < frame.width and y < frame.height):
            out.append({"check": name, "at": [x, y], "clipped": True, "visible_box": box, "exact": None})
            return
        actual = frame.pixel(x, y)[:3]
        out.append({"check": name, "at": [x, y], "expected": list(THEME[expected]), "actual": list(actual),
                    "exact": tuple(actual) == THEME[expected]})

    class Owned(dict):
        def __missing__(self, name):
            return next(ids[w] for w in DRAWN if name.startswith(w + " "))
    owner = Owned()

    for widget, colour in (("resume", "BUTTON"), ("options", "BUTTON"), ("online", "BUTTON_DISABLED"),
                           ("clip_a", "BUTTON")):
        l, t, r, b = boxes[ids[widget]]
        check(f"{widget} surface interior", l + 6, (t + b) // 2, colour)
    for widget in ("name", "long", "cjk"):
        l, t, r, b = boxes[ids[widget]]
        if ring and ring[0] == widget:
            for k in range(ring[1]):
                check(f"{widget} focus ring column {k}", l + k, (t + b) // 2, "ACCENT")
            check(f"{widget} field interior inside ring", l + ring[1] + 2, (t + b) // 2, "FIELD")
        else:
            check(f"{widget} field border", l, (t + b) // 2, "FIELD_BORDER")
            check(f"{widget} field interior", l + 4, (t + b) // 2, "FIELD")
    if ring and ring[0] == "resume":
        l, t, r, b = boxes[ids["resume"]]
        for k in range(ring[1]):
            check(f"resume focus ring column {k}", l + k, (t + b) // 2, "ACCENT")
        check("resume surface inside ring", l + ring[1] + 2, (t + b) // 2, "BUTTON")
    l, t, r, b = boxes[ids["title"]]
    brightest = max((frame.pixel(x, y)[:3] for y in range(t, b) for x in range(l, min(r, l + 120))), key=sum)
    out.append({"check": "title glyph full-coverage colour", "box": [l, t, min(r, l + 120), b],
                "expected": list(THEME["TEXT"]), "actual": list(brightest), "exact": tuple(brightest) == THEME["TEXT"]})
    return out


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, output = Path(sys.argv[1]), Path(sys.argv[2])
    captures = evidence / "captures"
    result = {"full_frame": [], "source_free": [], "superseded_r2": [], "scope": {}, "controls": {},
              "colours": {}, "srgb_vs_unorm": None}
    frames = {}
    for case_dir in sorted((captures / "baseline").iterdir()):
        for png in sorted(case_dir.glob("*.png")):
            key = f"{case_dir.name}/{png.stem}"
            base, cand = png, captures / "candidate" / case_dir.name / png.name
            a, b = Raster.load(base), Raster.load(cand)
            frames[key] = b
            result["full_frame"].append({
                "frame": key, "size": [a.width, a.height],
                "baseline_png_sha256": c.sha(base), "candidate_png_sha256": c.sha(cand),
                "png_bytes_identical": base.read_bytes() == cand.read_bytes(),
                "baseline_rgba_sha256": c.sha_bytes(a.rgba), "candidate_rgba_sha256": c.sha_bytes(b.rgba),
                "chunks": sorted(set(chunk_types(base.read_bytes()) + chunk_types(cand.read_bytes()))),
                **diff(a, b)})
    for case_dir in sorted(p for p in (captures / "candidate").iterdir() if p.name.endswith("-source-free")):
        case = case_dir.name[:-len("-source-free")]
        for png in sorted(case_dir.glob("*.png")):
            pre = captures / "candidate" / case / png.name
            result["source_free"].append({"frame": f"{case}/{png.stem}", "png_bytes_identical":
                                          png.read_bytes() == pre.read_bytes(), "png_sha256": c.sha(png),
                                          **diff(Raster.load(pre), Raster.load(png))})
    superseded = evidence / "captures-r2-superseded/baseline"
    for png in sorted(superseded.rglob("*.png")):
        result["superseded_r2"].append({"frame": str(png.relative_to(superseded)), "png_sha256": c.sha(png)})

    reports = {f"{d.name}": c.read_json(d / "report.json") for d in sorted((captures / "candidate").iterdir())
               if (d / "report.json").exists()}
    result["reports"] = {name: {"adapter": r["adapter"], "target_format": r["target_format"], "size": r["size"],
                                "ui_viewport": r["ui_viewport"], "final_tick": r["final_tick"], "failed": r["failed"],
                                "captures": [{k: cap[k] for k in ("capture", "tick", "accepted_frames", "scene_stats",
                                                                   "uploads_since_startup", "startup_uploads")}
                                             | {"ui_stats": cap["ui"]["stats"] if cap["ui"] else None,
                                                "focus": cap["ui"]["focus"] if cap["ui"] else None}
                                             for cap in r["captures"]]}
                         for name, r in reports.items()}

    # State and motion scope between captures of one run.
    moving = diff(frames["W-persp/t1"], frames["W-persp/t61"])
    result["scope"]["W-persp t1 -> t61"] = {**moving, "expected": "only the moving text (row 5, right column) changes"}
    idle, select = frames["U-1x/idle"], frames["U-1x/select"]
    boxes_1x = widget_boxes(reports["U-1x"]["captures"][1], 1.0)
    name_box = boxes_1x[bd.U_IDS["name"]]
    inside = diff(idle, select, name_box)
    whole = diff(idle, select)
    outside_box = None
    for y in range(idle.height):
        for x in range(idle.width):
            if name_box[0] <= x < name_box[2] and name_box[1] <= y < name_box[3]:
                continue
            if idle.pixel(x, y) != select.pixel(x, y):
                outside_box = [min(outside_box[0], x), min(outside_box[1], y), max(outside_box[2], x + 1),
                               max(outside_box[3], y + 1)] if outside_box else [x, y, x + 1, y + 1]
    result["scope"]["U-1x idle -> select"] = {
        "whole_frame": whole, "inside_name_field": inside, "name_field_box": name_box,
        "outside_name_field_pixels": whole["differing_pixels"] - inside["differing_pixels"],
        "outside_name_field_bounding_box": outside_box,
        "expected": "focus ring, selection and caret inside the name field; the world crate moves 2 ticks (Velocity)"}

    # Bounded in-scene controls on the orthographic world-text frame (uniform wall background).
    w = frames["W-ortho/t1"]
    wall = region_stats(w, (440, 236, 640, 258))
    background = tuple(wall["mode"])
    result["controls"]["wall_background"] = wall
    result["controls"]["checks"] = [
        control(w, [ox(-4.65) - 2, oy(1.75) - 2, ox(-4.65) + 302, oy(1.75) + 36],
                [ox(-4.65) - 2, oy(2.6) - 2, ox(-4.65) + 302, oy(2.6) + 36], "none",
                "w_latin_dup (row 1) equals w_latin (row 0): same string 85 px lower", background),
        control(w, [ox(-1.65) - 302, oy(0.9) - 2, ox(-1.65) + 2, oy(0.9) + 36],
                [ox(-1.45) - 2, oy(0.9) - 2, ox(-1.45) + 302, oy(0.9) + 36], "flip_h",
                "w_mirror (scale -1, 1, 1) mirrored back equals its unmirrored control", background),
        control(w, [ox(4.75) - 302, oy(0.9) - 2, ox(4.75) + 2, oy(0.9) + 36],
                [ox(-1.65) - 302, oy(0.9) - 2, ox(-1.65) + 2, oy(0.9) + 36], "none",
                "w_back (rotated 180 deg about Y, seen from behind) equals w_mirror", background),
        control(w, [ox(-1.45) - 2, oy(-0.6) - 34, ox(-1.45) + 302, oy(-0.6) + 2],
                [ox(1.75) - 2, oy(0.05) - 2, ox(1.75) + 302, oy(0.05) + 34], "flip_v",
                "w_child (parent scale 1, -1, 1) flipped back equals its unflipped control", background),
    ]

    # Exact packed UI colours at surface interiors (bounded controls, not parity).
    for label, frame, case, index, scale, ring in (
            ("U-1x/idle (Rgba8UnormSrgb)", "U-1x/idle", "U-1x", 0, 1.0, None),
            ("U-1x-unorm/idle (Rgba8Unorm)", "U-1x-unorm/idle", "U-1x-unorm", 0, 1.0, None),
            ("U-1x/select (name focused, 2 px ring)", "U-1x/select", "U-1x", 1, 1.0, ("name", 2)),
            ("U-2x/focus (Resume focused, 4 px ring)", "U-2x/focus", "U-2x", 0, 2.0, ("resume", 4)),
            ("U-15x-name/preedit (name focused, 3 px ring)", "U-15x-name/preedit", "U-15x-name", 0, 1.5,
             ("name", 3))):
        capture = reports[case]["captures"][index]
        result["colours"][label] = colour_checks(frames[frame], widget_boxes(capture, scale), ring,
                                                 widget_boxes(capture, scale, "visible"))

    # Same UI and world into an sRGB target and a UNORM target: a documented format difference, not parity.
    srgb, unorm = frames["U-1x/idle"], frames["U-1x-unorm/idle"]
    visible_1x = widget_boxes(reports["U-1x"]["captures"][0], 1.0, "visible")
    drawn = [visible_1x[bd.U_IDS[name]] for name in DRAWN]
    inside = outside = 0
    worst = {"inside": 0, "outside": 0}
    for y in range(srgb.height):
        for x in range(srgb.width):
            a, b = srgb.pixel(x, y), unorm.pixel(x, y)
            if a == b:
                continue
            delta = max(abs(a[k] - b[k]) for k in range(4))
            # Label glyph shadows extend one physical pixel below their visible box.
            hit = any(box[0] <= x < box[2] and box[1] <= y < box[3] + 2 for box in drawn)
            inside += hit
            outside += not hit
            worst["inside" if hit else "outside"] = max(worst["inside" if hit else "outside"], delta)
    result["srgb_vs_unorm"] = {"whole_frame": diff(srgb, unorm), "inside_drawn_widgets": inside,
                               "outside_drawn_widgets": outside, "max_delta": worst,
                               "note": "One UI and world into an sRGB and a UNORM target. UI blends encoded values "
                                       "on UNORM targets and linear values on sRGB targets (ui.wgsl frame.encode); "
                                       "the world output pass encodes in the shader for UNORM and on store for sRGB. "
                                       "A target-format property, identical in both variants; not a parity result."}
    result["summary"] = {
        "frames_compared": len(result["full_frame"]),
        "png_identical": sum(f["png_bytes_identical"] for f in result["full_frame"]),
        "rgba_identical": sum(f["differing_pixels"] == 0 for f in result["full_frame"]),
        "source_free_identical": sum(f["png_bytes_identical"] for f in result["source_free"]),
        "source_free_frames": len(result["source_free"]),
        "controls_exact": sum(ch["differing_pixels"] == 0 for ch in result["controls"]["checks"]),
        "controls": len(result["controls"]["checks"]),
        "colour_checks_exact": sum(x["exact"] is True for v in result["colours"].values() for x in v),
        "colour_checks": sum(x["exact"] is not None for v in result["colours"].values() for x in v),
        "colour_probes_clipped": sum(x["exact"] is None for v in result["colours"].values() for x in v),
    }
    c.write_new_json(output, result)
    print(json.dumps(result["summary"], indent=1))


if __name__ == "__main__":
    main()
