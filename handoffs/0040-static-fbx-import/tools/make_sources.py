"""Build every review source for handoff 0040 into artifacts/, with a hash manifest.

Upstream fixtures are copied byte for byte after checking provenance.json. Derived
FBX files, original art, glTF controls and glTF twins are regenerated
deterministically; a second run must reproduce identical bytes.
Usage: python3 -I make_sources.py [OUT_DIR] [--manifest PATH]
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import art  # noqa: E402
import controls  # noqa: E402
import derive  # noqa: E402
import twins  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / derive.FIXTURES


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main(out, manifest_path=None):
    out = Path(out)
    if out.exists():
        shutil.rmtree(out)
    entries = {}

    def put(rel, data, provenance):
        path = out / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        entries[rel] = {"sha256": sha(data), "bytes": len(data), "provenance": provenance}
        return data

    expected = json.loads((FIXTURES / "provenance.json").read_text())["files_sha256"]
    for name, digest in sorted(expected.items()):
        data = (FIXTURES / name).read_bytes()
        if sha(data) != digest:
            raise SystemExit(f"upstream fixture hash mismatch: {name}")
        put(f"upstream/{name}", data, "ufbx v0.23.1 data/, MIT; unmodified, verified against provenance.json")

    tmp = out / "art"
    tmp.mkdir(parents=True)
    images = {
        "face_art_v1.png": art.face_art(tmp / "face_art_v1.png", "F"),
        "face_art_v2.png": art.face_art(tmp / "face_art_v2.png", "G", tint=(190, 60, 60)),
        "panel_v1.png": art.face_art(tmp / "panel_v1.png", "F", tint=(150, 150, 150)),
        "panel_v2.png": art.face_art(tmp / "panel_v2.png", "2", tint=(60, 160, 80)),
        "bevels.png": art.normal_map(tmp / "bevels.png"),
    }
    notes = {"face_art_v1.png": "original asymmetric per-island art (F, axis labels, corner marks)",
             "face_art_v2.png": "accepted-change art: G glyph, red islands",
             "panel_v1.png": "project-relative panel art v1 (grey)", "panel_v2.png": "panel art v2 (glyph 2, green)",
             "bevels.png": "OpenGL/glTF-convention normal map: raised 35-degree button per island"}
    for name, data in images.items():
        entries[f"art/{name}"] = {"sha256": sha(data), "bytes": len(data), "provenance": "original (art.py): " + notes[name]}

    cube = (FIXTURES / derive.CUBE).read_text()
    parented = (FIXTURES / derive.PARENTED).read_text()
    derived = {}

    def fbx(rel, text_edits, note):
        text, edits = text_edits
        put(f"derived/{rel}", text.encode(), {"base": derive.CUBE if "parented" not in rel else derive.PARENTED,
                                               "edits": edits, "purpose": note})
        derived[rel] = text

    v1, v2 = images["face_art_v1.png"], images["face_art_v2.png"]
    fbx("tex_embedded.fbx", derive.textured(cube, v1), "embedded colour map")
    fbx("tex_external.fbx", derive.textured(cube, v1, relative="textures\\face_art.png", embedded=False),
        "project-relative colour map (Windows separator, as Maya writes)")
    fbx("normal_embedded.fbx", derive.normal_cube(cube, images["bevels.png"]), "embedded NormalMap on Lambert")
    fbx("instances.fbx", derive.instances(cube, v1, images["bevels.png"]), "mirrored/rotated/nonuniform instances")
    fbx("crate_v1.fbx", derive.textured(cube, v1), "reimport v1 (embedded art v1)")
    fbx("crate_v2.fbx", derive.textured(cube, v2), "reimport accepted change (embedded art v2)")
    fbx("crate_unsupported.fbx", derive.textured(cube, v2, extra_channel="SpecularColor"),
        "rejected reimport: same texture also on SpecularColor")
    put("derived/crate_malformed.fbx", derived["crate_v2.fbx"].encode()[: len(derived["crate_v2.fbx"]) * 6 // 10],
        {"base": "derived/crate_v2.fbx", "edits": ["truncated to the first 60% of bytes"], "purpose": "rejected malformed reimport"})
    fbx("panel.fbx", derive.textured(cube, None, relative="textures\\panel.png", embedded=False),
        "project-relative dependency-change subject")
    refusals = {
        "bad_uv_transform.fbx": derive.textured(cube, v1, texture_props=['P: "Translation", "Vector", "", "A",0.25,0,0']),
        "bad_bump_channel.fbx": derive.textured(cube, v1, channel="Bump"),
        "bad_transparency_map.fbx": derive.textured(cube, v1, channel="TransparentColor"),
        "bad_specular_channel.fbx": derive.textured(cube, v1, extra_channel="SpecularColor"),
        "bad_escape_path.fbx": derive.textured(cube, None, relative="..\\..\\escape.png", embedded=False),
        "bad_missing_image.fbx": derive.textured(cube, None, relative="textures\\missing.png", embedded=False),
    }
    for rel, value in refusals.items():
        fbx(rel, value, "expected explicit refusal")
    text, edits = derive.textured(cube, v1)
    fbx("bad_phong.fbx", (derive.phong(text, edits), edits), "expected explicit refusal: legacy Phong")
    text, edits = derive.textured(cube, v1)
    fbx("bad_second_uv.fbx", (derive.second_uv_set(text, edits), edits), "expected explicit refusal: two UV sets")
    fbx("parented_shear_mirror.fbx", derive.shear_mirror(parented), "mirrored nonuniform parent, rotated child")
    fbx("materials.fbx", derive.materials(cube, v1), "Lambert diffuse x factor, emissive map, constant emissive, opacity 0.5")

    relief, relief_info = controls.relief()
    put("controls/axes.glb", controls.axes(), "original glTF control: world axes")
    put("controls/ruler.glb", controls.ruler(), "original glTF control: 1 m ruler, 10 cm segments")
    put("controls/relief.glb", relief, {"note": "original glTF control: real-geometry button", **relief_info})
    put("controls/quad_normal_explicit.glb", controls.quad_normal(images["bevels.png"], True),
        "original glTF control: normal map, TANGENT (1,0,0,+1) per glTF 2.0")
    put("controls/quad_normal_generated.glb", controls.quad_normal(images["bevels.png"], False),
        "original glTF control: normal map, no TANGENT (engine-generated)")
    put("controls/quad_face_art.glb", controls.quad_face_art(v1), "original glTF control: colour art UV reference")
    put("controls/material_quads.glb", controls.material_quads(v1), "original glTF control: expected glTF values of the Lambert cases")
    put("controls/backdrop.glb", controls.backdrop(), "original glTF control: uniform backdrop behind the opacity case")

    twin_sources = {"maya_cube": FIXTURES / "maya_cube_7500_ascii.fbx",
                    "blender_cube": FIXTURES / "blender_272_cube_7400_binary.fbx",
                    "maya_parented": FIXTURES / "maya_parented_cubes_7500_ascii.fbx",
                    "max_instances": FIXTURES / "max_geometry_transform_instances_7700_ascii.fbx",
                    "suzanne": FIXTURES / "blender_suzanne_multimaterial_7400_binary.fbx",
                    "shear_mirror": out / "derived/parented_shear_mirror.fbx"}
    for name, path in twin_sources.items():
        data, summary = twins.build(path)
        origin = path.relative_to(out) if path.is_relative_to(out) else path.relative_to(ROOT)
        put(f"twins/{name}.glb", data, {"twin_of": str(origin),
                                        "summary": summary})
    for name in ("tex_embedded", "normal_embedded", "instances"):
        data, summary = twins.build(out / f"derived/{name}.fbx", textures=True)
        put(f"twins/{name}.glb", data, {"twin_of": f"derived/{name}.fbx", "textures": "embedded images; Maya tangent frame",
                                        "summary": summary})
    manifest = {"generator": "handoffs/0040-static-fbx-import/tools/make_sources.py", "files": entries}
    text = json.dumps(manifest, indent=1, sort_keys=True) + "\n"
    (out / "manifest.json").write_text(text)
    if manifest_path:
        Path(manifest_path).write_text(text)
    return manifest


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    target = args[0] if args else ROOT / "artifacts/0040-static-fbx-import/sources"
    m = sys.argv[sys.argv.index("--manifest") + 1] if "--manifest" in sys.argv else None
    result = main(target, m)
    print(json.dumps({k: v["sha256"][:12] for k, v in result["files"].items()}, indent=1))
