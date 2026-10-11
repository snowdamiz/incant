"""CPU-only import probe for handoff 0040: every derived, upstream, control and twin source.

Imports each file through the public `import` CLI into one scratch project and prints
the outcome; rejected imports must leave project and journal bytes unchanged. This is
the saved form of the probe run recorded in the ledger as run "probe" (no renderer).
Usage: python3 -I probe_imports.py RUN_DIR_NAME
"""
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import engine  # noqa: E402

DERIVED = ["tex_embedded", "tex_external", "normal_embedded", "instances", "crate_v1", "panel", "parented_shear_mirror",
           "bad_uv_transform", "bad_bump_channel", "bad_transparency_map", "bad_specular_channel", "bad_escape_path",
           "bad_missing_image", "bad_phong", "bad_second_uv", "crate_unsupported", "crate_malformed"]


def main(name):
    src = engine.ART / "sources"
    d = engine.ART / name
    if d.exists():
        raise SystemExit(f"{d} exists")
    d.mkdir(parents=True)
    project = d / "probe.incant.json"
    engine.init(project, "Probe", run_id=name)
    shutil.copytree(src / "derived", d / "models")
    (d / "models/textures").mkdir()
    shutil.copy(src / "art/face_art_v1.png", d / "models/textures/face_art.png")
    shutil.copy(src / "art/panel_v1.png", d / "models/textures/panel.png")
    for sub in ("upstream", "controls", "twins"):
        shutil.copytree(src / sub, d / sub)
    sources = [f"models/{f}.fbx" for f in DERIVED]
    for sub in ("upstream", "controls", "twins"):
        sources += [f"{sub}/{p.name}" for p in sorted((d / sub).iterdir())]
    for rel in sources:
        r = engine.import_source(project, rel, label=f"{name} {rel}", run_id=name)
        same = r["state_before"]["project_sha256"] == r["state_after"]["project_sha256"] and \
            r["state_before"]["journal_sha256"] == r["state_after"]["journal_sha256"]
        print("OK  " if r["ok"] else "FAIL", rel, "" if r["ok"] else ("unchanged" if same else "CHANGED"), r["stderr"][:200])


if __name__ == "__main__":
    main(sys.argv[1])
