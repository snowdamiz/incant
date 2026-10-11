"""Tangent-delta comparisons from unedited frames (CPU only; launches nothing).

Compares the candidate's textures/textures-twin frames with each other, with the
original frozen-binary frames (r1) and with the real-geometry and supplied-tangent
controls. Writes evidence/tangent-delta/analysis.json and two DERIVED diff maps.
Usage: python3 -I delta_numbers.py DELTA_RUN   (e.g. d1)
"""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import analyse  # noqa: E402
import report_numbers as rn  # noqa: E402  (reuses its unchanged bevel/crop/row helpers)

ROOT = Path(__file__).resolve().parents[3]
ORIGINAL = ROOT / "artifacts/0040-static-fbx-import/runs/r1"
DELTA = ROOT / "artifacts/0040-static-fbx-import/tangent-delta"
EVIDENCE = Path(__file__).resolve().parents[1] / "evidence/tangent-delta"
CONTROLS = [("relief_geometry", -5.6), ("gltf_supplied_tangent", -4.0), ("gltf_generated_tangent", -2.4), ("fbx_normal_cube", -0.8)]
INSTANCES = [("B_identity", 1.6), ("B_mirror_x", 3.2), ("B_nonuniform", 4.8), ("B_rot_z90", 6.4)]


def frames(path):
    return {p.stem: p for p in sorted(Path(path).glob("*.png"))}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def bevel_table(frame, rows, cx, cy):
    img = analyse.load(frame)
    return {name: rn.bevels(img, x, -2, cx, cy) for name, x in rows}


def crop(frame, a, b, y, cx, cy):
    ax, ay, side = rn.face_crop(a, y, cx, cy)
    bx, _, _ = rn.face_crop(b, y, cx, cy)
    return {"offset_px": bx - ax, **analyse.crop_compare(frame, (ax, ay, side, side), (bx, ay))}


def main(run):
    fbx, twin = frames(DELTA / f"runs/{run}/textures/frames"), frames(DELTA / f"runs/{run}/textures-twin/frames")
    ofbx, otwin = frames(ORIGINAL / "textures/frames"), frames(ORIGINAL / "textures-twin/frames")
    out = {"delta_run": run}
    out["bevels_controls"] = {"candidate": bevel_table(fbx["controls_ortho"], CONTROLS, -3.2, -1.0),
                              "original": bevel_table(ofbx["controls_ortho"], CONTROLS, -3.2, -1.0)}
    out["bevels_instances"] = {"candidate_fbx": bevel_table(fbx["instances_ortho"], INSTANCES, 4.0, -1.0),
                               "candidate_twin_maya_frame": bevel_table(twin["instances_ortho"], INSTANCES, 4.0, -1.0),
                               "original_fbx": bevel_table(ofbx["instances_ortho"], INSTANCES, 4.0, -1.0)}
    controls = fbx["controls_ortho"]
    out["controls_crops_candidate"] = {
        "generated_vs_supplied_gltf_quad": crop(controls, -4.0, -2.4, -2, -3.2, -1.0),
        "supplied_gltf_quad_vs_fbx_front_face": crop(controls, -4.0, -0.8, -2, -3.2, -1.0),
        "gltf_colour_quad_vs_fbx_embedded": crop(controls, -5.6, -4.0, 0, -3.2, -1.0),
        "fbx_embedded_vs_project_relative": crop(controls, -4.0, -2.4, 0, -3.2, -1.0),
    }
    rows = {}
    for view in ("controls_ortho", "instances_ortho"):
        rows[view] = {
            "candidate_fbx_vs_candidate_twin": {"colour": rn.row_diff(fbx[view], twin[view], 0, 270),
                                                "normal": rn.row_diff(fbx[view], twin[view], 270, 540)},
            "candidate_fbx_vs_original_fbx": {"colour": rn.row_diff(fbx[view], ofbx[view], 0, 270),
                                              "normal": rn.row_diff(fbx[view], ofbx[view], 270, 540)},
            "candidate_fbx_vs_original_twin": {"colour": rn.row_diff(fbx[view], otwin[view], 0, 270),
                                               "normal": rn.row_diff(fbx[view], otwin[view], 270, 540)},
        }
    out["row_differing_pixels"] = rows
    out["supplied_tangent_preservation"] = {k: analyse.compare(twin[k], otwin[k]) for k in otwin}
    out["whole_frame_candidate_vs_original_fbx"] = {k: analyse.compare(fbx[k], ofbx[k]) for k in ofbx}
    out["whole_frame_candidate_fbx_vs_candidate_twin"] = {k: analyse.compare(fbx[k], twin[k]) for k in twin}
    out["source_free"] = {k: analyse.compare(fbx[k], fbx[f"{k}-source-free"]) for k in ("controls_ortho", "instances_ortho")}
    out["frame_sha256"] = {f"textures/{k}": sha(p) for k, p in fbx.items()} | {f"textures-twin/{k}": sha(p) for k, p in twin.items()}
    derived = DELTA / "derived"
    derived.mkdir(exist_ok=True)
    for view in ("instances_front_right_top", "instances_back_left_bottom"):  # DERIVED, not engine frames
        analyse.diff_figure(fbx[view], twin[view], derived / f"DERIVED-diff-candidate-fbx-vs-twin-{view}.png")
        analyse.diff_figure(fbx[view], ofbx[view], derived / f"DERIVED-diff-candidate-vs-original-fbx-{view}.png")
    # Supplied-tangent preservation across binaries: only the generated-tangent glTF quad may change.
    analyse.diff_figure(twin["controls_front_right_top"], otwin["controls_front_right_top"],
                        derived / "DERIVED-diff-candidate-twin-vs-original-twin-controls_front_right_top.png")
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "analysis.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    return out


if __name__ == "__main__":
    result = main(sys.argv[1] if len(sys.argv) > 1 else "d1")
    for key in ("bevels_controls", "bevels_instances", "controls_crops_candidate", "row_differing_pixels", "source_free"):
        print(key, json.dumps(result[key], indent=1))
