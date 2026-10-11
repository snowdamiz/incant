"""Recompute every number quoted in handoff 0040's result.md from the raw frames.

Reads artifacts/0040-static-fbx-import/runs/ and the ledger; writes
handoffs/0040-static-fbx-import/evidence/analysis.json. No engine call is made.
Usage: python3 -I handoffs/0040-static-fbx-import/tools/report_numbers.py
"""
import collections
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import analyse  # noqa: E402
import engine  # noqa: E402

RUNS = engine.ART / "runs"
EVIDENCE = Path(__file__).resolve().parents[1] / "evidence"
PPM = 540 / (864 / 262)  # ortho review cameras: 163.75 px per metre


def frames(path):
    return {p.stem: p for p in sorted(Path(path).glob("*.png"))}


def ortho_px(x, y, cx, cy):
    return 480 + (x - cx) * PPM, 270 - (y - cy) * PPM


def bevels(img, x, y, cx, cy):
    px, py = ortho_px(x, y, cx, cy)
    off = 0.3125 * PPM  # bevel band centre, between 0.1875 m and 0.4375 m from the island centre
    out = {}
    for label, dx, dy in [("top", 0, -off), ("bottom", 0, off), ("left", -off, 0), ("right", off, 0), ("flat", 0, 0)]:
        X, Y = int(round(px + dx)), int(round(py + dy))
        out[label] = analyse.luminance(analyse.region_mean(img, X - 7, Y - 7, X + 8, Y + 8))
    out["top_minus_bottom"] = round(out["top"] - out["bottom"], 2)
    out["right_minus_left"] = round(out["right"] - out["left"], 2)
    return out


def face_crop(x, y, cx, cy):
    px, py = ortho_px(x, y, cx, cy)
    side = int(0.9 * PPM)
    return int(round(px - 0.45 * PPM)), int(round(py - 0.45 * PPM)), side


def row_diff(a, b, y0, y1):
    w, h, c, pa = analyse.load(a)
    _, _, _, pb = analyse.load(b)
    return sum(1 for y in range(y0, y1) for x in range(w)
               if max(abs(pa[(y * w + x) * c + k] - pb[(y * w + x) * c + k]) for k in range(3)) > 0)


def main():
    out = {}
    fbx, twin = frames(RUNS / "r1/static-fbx/frames"), frames(RUNS / "r1/static-twin/frames")
    out["static_fbx_vs_twin"] = {k: analyse.compare(fbx[k], twin[k]) for k in fbx}

    tex, ttwin = frames(RUNS / "r1/textures/frames"), frames(RUNS / "r1/textures-twin/frames")
    out["textures_fbx_vs_twin"] = {k: analyse.compare(tex[k], ttwin[k]) for k in ttwin}
    out["textures_fbx_vs_twin_rows"] = {k: {"colour_row_y_lt_270": row_diff(tex[k], ttwin[k], 0, 270),
                                            "normal_row_y_ge_270": row_diff(tex[k], ttwin[k], 270, 540)}
                                        for k in ("controls_ortho", "instances_ortho")}
    img = analyse.load(tex["controls_ortho"])
    out["normal_bevel_luminance_controls_ortho"] = {
        name: bevels(img, x, -2, -3.2, -1.0) for name, x in
        [("relief_geometry", -5.6), ("gltf_explicit_tangent", -4.0), ("gltf_generated_tangent", -2.4), ("fbx_normal_cube", -0.8)]}
    for label, frame in [("fbx", tex["instances_ortho"]), ("twin_maya_tangents", ttwin["instances_ortho"])]:
        img = analyse.load(frame)
        out[f"normal_bevel_luminance_instances_{label}"] = {
            name: bevels(img, x, -2, 4.0, -1.0) for name, x in
            [("B_identity", 1.6), ("B_mirror_x", 3.2), ("B_nonuniform", 4.8), ("B_rot_z90", 6.4)]}
    crops = {}
    for label, a, b in [("tex_embedded_vs_tex_external", -4.0, -2.4), ("gltf_colour_quad_vs_tex_embedded", -5.6, -4.0)]:
        ax, ay, side = face_crop(a, 0, -3.2, -1.0)
        bx, _, _ = face_crop(b, 0, -3.2, -1.0)
        crops[label] = {"offset_px": bx - ax, **analyse.crop_compare(tex["controls_ortho"], (ax, ay, side, side), (bx, ay))}
    out["front_face_crops"] = crops
    out["textures_source_free"] = {k: analyse.compare(tex[k], tex[f"{k}-source-free"]) for k in ("controls_ortho", "instances_ortho")}

    mat = frames(RUNS / "r5/materials/frames")["materials_ortho"]
    cases = {}
    for i, name in enumerate(["diffuse_x_factor", "emissive_map", "emissive_constant", "opacity_half_over_backdrop"]):
        ax, ay, side = face_crop(1.6 * i, 0, 2.4, -0.8)
        _, by, _ = face_crop(1.6 * i, -1.6, 2.4, -0.8)
        cases[name] = {"offset_px": by - ay, **analyse.crop_compare(mat, (ax, ay, side, side), (ax, by)),
                       "fbx_centre_rgb": analyse.region_mean(mat, ax + side // 2 - 10, ay + side // 2 - 10, ax + side // 2 + 10, ay + side // 2 + 10)}
    out["lambert_cases_vs_expected_gltf"] = cases

    rei = frames(RUNS / "r2/reimport/frames")
    groups = collections.defaultdict(list)
    for k, p in rei.items():
        groups[hashlib.sha256(p.read_bytes()).hexdigest()].append(k)
    out["reimport_identical_frame_groups"] = {digest[:16]: sorted(v) for digest, v in sorted(groups.items(), key=lambda kv: sorted(kv[1]))}
    rep = frames(RUNS / "r3/repeat")
    out["fresh_process_repeats"] = {
        "suzanne_front": analyse.compare(fbx["suzanne_front"], rep["suzanne_front"]),
        "controls_ortho-source-free": analyse.compare(tex["controls_ortho-source-free"], rep["controls_ortho-source-free"])}

    derived = engine.ART / "derived"
    derived.mkdir(exist_ok=True)
    for view in ("instances_front_right_top", "instances_back_left_bottom"):  # DERIVED figures, not engine frames
        analyse.diff_figure(tex[view], ttwin[view], derived / f"DERIVED-diff-fbx-vs-twin-{view}.png")

    ledger = engine.ledger()
    shots = [r for r in ledger if r.get("kind") == "screenshot" and r.get("output_bytes")]
    expected = engine.expected_hash()
    out["ledger"] = {"engine_calls": len(ledger), "captures": len(shots), "raw_bytes": sum(r["output_bytes"] for r in shots),
                     "all_hash_checked": all(r["binary_sha256_before"] == expected == r["binary_sha256_after"] for r in ledger),
                     "timeouts": sum(r["timed_out"] for r in ledger), "adapters": sorted({r["adapter"] for r in shots}),
                     "max_dimensions": max((r["width"], r["height"]) for r in shots), "binary_sha256": expected}
    EVIDENCE.mkdir(exist_ok=True)
    (EVIDENCE / "analysis.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    return out


if __name__ == "__main__":
    result = main()
    print(json.dumps({k: result[k] for k in ("normal_bevel_luminance_controls_ortho", "front_face_crops", "lambert_cases_vs_expected_gltf",
                                             "reimport_identical_frame_groups", "ledger")}, indent=1))
