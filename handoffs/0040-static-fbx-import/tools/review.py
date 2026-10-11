"""Handoff 0040 review runs through the frozen incant_headless public CLI only.

  python3 -I review.py static RUN     FBX static project + identical glTF-twin control project
  python3 -I review.py textures RUN   colour/normal maps, instances, then source-free recapture
  python3 -I review.py textures-twin RUN   same layout with glTF twins carrying Maya's tangents
  python3 -I review.py materials RUN  Lambert diffuse x factor, emissive map/constant, opacity vs glTF values
  python3 -I review.py reimport RUN   stable/changed/rejected reimports, Undo/Redo, source removal
  python3 -I review.py repeat RUN     fresh-process repeats of two earlier captures

Projects, caches and raw frames go to artifacts/0040-static-fbx-import/runs/RUN/.
Scene edits use rpc command.execute; Undo/Redo and saves use rpc history.* and
project.save. Source files are authoring inputs copied into each project.
"""
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))  # -I omits the script directory

import engine  # noqa: E402
import scene as S  # noqa: E402

SOURCES = engine.ART / "sources"
EVIDENCE = Path(__file__).resolve().parents[1] / "evidence"
W, H = 960, 540


class Run:
    def __init__(self, run, name):
        self.run, self.name = run, name
        self.dir = engine.ART / "runs" / run / name
        if self.dir.exists():
            raise SystemExit(f"{self.dir} exists; choose a new RUN id")
        self.dir.mkdir(parents=True)
        self.project = self.dir / f"{name}.incant.json"
        self.record = {"run": run, "project": name, "binary_sha256": engine.check_binary(), "steps": []}
        engine.init(self.project, name, run_id=run)
        doc = self.doc()
        self.scene_id = next(iter(doc["scenes"]))
        self.assets = {}

    def doc(self):
        return json.loads(self.project.read_text())

    def step(self, kind, **data):
        self.record["steps"].append({"kind": kind, **data})

    def copy(self, rel_src, rel_dst):
        dst = self.dir / rel_dst
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(SOURCES / rel_src, dst)

    def import_(self, key, rel, expect_ok=True):
        result = engine.import_source(self.project, rel, label=f"{self.name} import {rel}", run_id=self.run)
        entry = {"source": rel, "exit": result["exit"], "stderr": result["stderr"],
                 "state_before": result["state_before"], "state_after": result["state_after"]}
        if result["ok"]:
            reply = result["reply"]
            entry["reply"] = reply
            self.assets[key] = reply["asset"]["id"]
        entry["unchanged"] = (result["state_before"]["project_sha256"] == result["state_after"]["project_sha256"]
                              and result["state_before"]["journal_sha256"] == result["state_after"]["journal_sha256"])
        self.step("import", key=key, expect_ok=expect_ok, **entry)
        if result["ok"] != expect_ok:
            raise SystemExit(f"{rel}: import exit {result['exit']} (expected ok={expect_ok}): {result['stderr']}")
        return entry

    def rpc(self, requests, label):
        responses = engine.rpc(self.project, requests, label=f"{self.name} {label}", run_id=self.run)
        errors = [r for r in responses if "error" in r]
        summary = [{"id": r["id"], "error": r.get("error"),
                    "result": {k: v for k, v in (r.get("result") or {}).items() if k != "project"}
                    if isinstance(r.get("result"), dict) else r.get("result")} for r in responses]
        self.step("rpc", label=label, methods=[q["method"] for q in requests], responses=summary)
        if errors:
            raise SystemExit(f"{label}: {errors}")
        return responses

    def revision(self):
        return self.rpc([{"method": "project.read"}], "read revision")[0]["result"]["revision"]

    def author(self, entities, description):
        rev = self.revision()
        self.rpc([S.execute(S.create(self.scene_id, entities), rev, description), {"method": "project.save"}], description)

    def capture(self, label, camera, state):
        out = self.dir / "frames" / f"{label}.png"
        shot = engine.screenshot(self.project, out, label=f"{self.name} {label}", camera=camera, width=W, height=H, run_id=self.run)
        if not shot["ok"]:
            raise SystemExit(f"capture {label} failed: {shot}")
        self.step("capture", label=label, camera=camera, state=state, path=str(out.relative_to(engine.ROOT)),
                  sha256=shot["sha256"], bytes=shot["bytes"], adapter=shot["adapter"], geometry=shot["geometry"],
                  shading=shot["shading"], project_state=engine.project_state(self.project))
        return shot

    def save_record(self):
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        path = EVIDENCE / f"{self.run}-{self.name}.json"
        path.write_text(json.dumps(self.record, indent=1, sort_keys=True) + "\n")
        return path


def cams(group, specs):
    """specs: (name, eye, target, kwargs) -> (entities, {name: id})."""
    entities, ids = [], {}
    for n, (name, eye, target, kw) in enumerate(specs, 1):
        ids[name] = S.ulid(group, 900 + n)
        entities.append(S.camera_entity(ids[name], f"camera {name}", eye, target, **kw))
    return entities, ids


# --------------------------------------------------------------------------- static
STATIC_SUBJECTS = [  # key, FBX source, twin
    ("blender_cube", "upstream/blender_272_cube_7400_binary.fbx", "twins/blender_cube.glb"),
    ("suzanne", "upstream/blender_suzanne_multimaterial_7400_binary.fbx", "twins/suzanne.glb"),
    ("max_instances", "upstream/max_geometry_transform_instances_7700_ascii.fbx", "twins/max_instances.glb"),
    ("maya_cube", "upstream/maya_cube_7500_ascii.fbx", "twins/maya_cube.glb"),
    ("maya_parented", "upstream/maya_parented_cubes_7500_ascii.fbx", "twins/maya_parented.glb"),
    ("shear_mirror", "derived/parented_shear_mirror.fbx", "twins/shear_mirror.glb"),
]
STATIC_PLACEMENTS = [  # entity n, name, asset key, translation, scale
    (1, "overview blender cube", "blender_cube", (-4.2, 1.0, 0), 1),
    (2, "overview suzanne", "suzanne", (-0.6, 1.0, 0), 1),
    (3, "overview max", "max_instances", (2.6, 0, 0), 1),
    (4, "overview maya cube (1 cm)", "maya_cube", (4.6, 0.005, 0.8), 1),
    (5, "overview maya parented (cm)", "maya_parented", (4.9, 0.017, 0.8), 1),
    (6, "overview ruler 1 m", "ruler", (-5.2, 0, 1.4), 1),
    (7, "overview axes", "axes", (0.9, 0, 1.6), 1),
    (11, "suzanne station", "suzanne", (40, 1.0, 0), 1),
    (21, "max station", "max_instances", (-40, 0, 0), 1),
    (31, "maya parented station x100", "maya_parented", (80, 0, 0), 100),
    (41, "shear mirror station x100", "shear_mirror", (-80, 0, 0), 100),
    (51, "units maya cube (1 cm)", "maya_cube", (0, 0.005, -60), 1),
    (52, "units ruler x0.1", "ruler", (-0.03, 0, -59.985), 0.1),
]
STATIC_CAMERAS = [
    ("overview", (-0.6, 2.4, 10.5), (-0.6, 0.9, 0), {"fov": 40}),
    ("suzanne_front", (40, 1.0, 6.0), (40, 1.0, 0), {"fov": 35}),
    ("suzanne_back3q", (36.2, 2.4, -4.6), (40, 0.95, 0), {"fov": 35}),
    ("suzanne_profile", (46.0, 1.0, 0), (40, 1.0, 0), {"fov": 35}),
    ("max_3q", (-36.4, 2.6, 4.2), (-40, 0.55, 0), {"fov": 35}),
    ("max_top", (-40, 8, 0), (-40, 0, 0), {"ortho": 3.2, "up": (0, 0, -1)}),
    ("parented_3q", (83.4, 2.2, 4.4), (80.3, -0.2, 0.1), {"fov": 35}),
    ("shear_mirror_3q", (-76.6, 2.2, 4.4), (-79.75, -0.2, 0.1), {"fov": 35}),
    ("units", (0.01, 0.035, -59.88), (0.01, 0.004, -60), {"fov": 35, "near": 0.005, "far": 5}),
]


def static(run):
    results = {}
    for variant in ("fbx", "twin"):
        r = Run(run, f"static-{variant}")
        for key, fbx, twin in STATIC_SUBJECTS:
            src = fbx if variant == "fbx" else twin
            rel = f"models/{Path(src).name}"
            r.copy(src, rel)
            r.import_(key, rel)
        for key in ("ruler", "axes"):
            r.copy(f"controls/{key}.glb", f"controls/{key}.glb")
            r.import_(key, f"controls/{key}.glb")
        ents = [S.model_entity(S.ulid("ST", n), name, r.assets[key], S.transform(t, scale=(s, s, s)))
                for n, name, key, t, s in STATIC_PLACEMENTS]
        cam_ents, cam_ids = cams("ST", STATIC_CAMERAS)
        r.author(ents + cam_ents, "Static FBX review layout")
        r.record["cameras"] = cam_ids
        for name, _, _, _ in STATIC_CAMERAS:
            r.capture(name, cam_ids[name], "authored")
        results[variant] = str(r.save_record())
    return results


# --------------------------------------------------------------------------- textures
TEX_PLACEMENTS = [  # n, name, asset key, translation, scale
    (1, "colour control quad (glTF)", "quad_face_art", (-5.6, 0, 0.5), 100 / 100),
    (2, "tex embedded", "tex_embedded", (-4.0, 0, 0), 100),
    (3, "tex project-relative", "tex_external", (-2.4, 0, 0), 100),
    (4, "relief geometry control (glTF)", "relief", (-5.6, -2, 0.5), 1),
    (5, "normal quad explicit tangents (glTF)", "quad_normal_explicit", (-4.0, -2, 0.5), 1),
    (6, "normal quad generated tangents (glTF)", "quad_normal_generated", (-2.4, -2, 0.5), 1),
    (7, "normal cube (FBX)", "normal_embedded", (-0.8, -2, 0), 100),
    (8, "instances (FBX)", "instances", (1.6, 0, 0), 100),
]
VS = 864 / 262  # 163.75 px/m at 540 px: 1.6 m spacing = 262 px exactly
TEX_CAMERAS = [
    ("controls_ortho", (-3.2, -1.0, 10), (-3.2, -1.0, 0), {"ortho": VS}),
    ("instances_ortho", (4.0, -1.0, 10), (4.0, -1.0, 0), {"ortho": VS}),
    ("instances_front_right_top", (9.6, 3.4, 7.2), (4.0, -1.0, 0), {"fov": 40}),
    ("instances_back_left_bottom", (-0.6, -5.4, -6.8), (4.0, -1.0, 0), {"fov": 40}),
    ("controls_front_right_top", (0.6, 2.4, 6.6), (-3.2, -1.0, 0), {"fov": 40}),
]


def textures(run):
    r = Run(run, "textures")
    for key, src in [("tex_embedded", "derived/tex_embedded.fbx"), ("tex_external", "derived/tex_external.fbx"),
                     ("normal_embedded", "derived/normal_embedded.fbx"), ("instances", "derived/instances.fbx")]:
        r.copy(src, f"models/{Path(src).name}")
    r.copy("art/face_art_v1.png", "models/textures/face_art.png")
    for key in ("quad_face_art", "relief", "quad_normal_explicit", "quad_normal_generated"):
        r.copy(f"controls/{key}.glb", f"controls/{key}.glb")
    for key in ("tex_embedded", "tex_external", "normal_embedded", "instances"):
        r.import_(key, f"models/{key}.fbx")
    for key in ("quad_face_art", "relief", "quad_normal_explicit", "quad_normal_generated"):
        r.import_(key, f"controls/{key}.glb")
    ents = [S.model_entity(S.ulid("TX", n), name, r.assets[key], S.transform(t, scale=(s, s, s)))
            for n, name, key, t, s in TEX_PLACEMENTS]
    cam_ents, cam_ids = cams("TX", TEX_CAMERAS)
    r.author(ents + cam_ents, "Texture review layout")
    r.record["cameras"] = cam_ids
    for name, _, _, _ in TEX_CAMERAS:
        r.capture(name, cam_ids[name], "with sources")
    removed = sorted(str(p.relative_to(r.dir)) for p in (r.dir / "models").rglob("*") if p.is_file())
    removed += sorted(str(p.relative_to(r.dir)) for p in (r.dir / "controls").rglob("*") if p.is_file())
    for rel in removed:
        (r.dir / rel).unlink()
    r.step("remove_sources", files=removed)
    for name in ("controls_ortho", "instances_ortho"):
        r.capture(f"{name}-source-free", cam_ids[name], "all sources removed")
    r.step("validate", **engine.validate(r.project, label="textures validate source-free", run_id=run))
    return str(r.save_record())


def textures_twin(run):
    """Same entity IDs, layout and cameras; FBX assets replaced by textured twins that
    carry Maya's own exported tangent frame (tex_external maps to the same twin as
    tex_embedded: identical geometry and image bytes)."""
    r = Run(run, "textures-twin")
    for key in ("tex_embedded", "normal_embedded", "instances"):
        r.copy(f"twins/{key}.glb", f"models/{key}.glb")
        r.import_(key, f"models/{key}.glb")
    r.assets["tex_external"] = r.assets["tex_embedded"]
    for key in ("quad_face_art", "relief", "quad_normal_explicit", "quad_normal_generated"):
        r.copy(f"controls/{key}.glb", f"controls/{key}.glb")
        r.import_(key, f"controls/{key}.glb")
    ents = [S.model_entity(S.ulid("TX", n), name, r.assets[key], S.transform(t, scale=(s, s, s)))
            for n, name, key, t, s in TEX_PLACEMENTS]
    cam_ents, cam_ids = cams("TX", TEX_CAMERAS)
    r.author(ents + cam_ents, "Texture review layout (twin control)")
    r.record["cameras"] = cam_ids
    for name, _, _, _ in TEX_CAMERAS:
        r.capture(name, cam_ids[name], "twin control")
    return str(r.save_record())


# --------------------------------------------------------------------------- materials
MAT_PLACEMENTS = [
    (1, "Lambert cases (FBX)", "materials", (0, 0, 0), 100),
    (2, "expected glTF values (control quads)", "material_quads", (0, -1.6, 0.5), 1),
    (3, "uniform backdrop", "backdrop", (4.8, -0.8, -0.6), 1),
]
MAT_CAMERAS = [
    ("materials_ortho", (2.4, -0.8, 10), (2.4, -0.8, 0), {"ortho": VS}),
    ("materials_3q", (6.8, 1.6, 5.4), (2.4, -0.8, 0), {"fov": 40}),
]


def materials(run):
    r = Run(run, "materials")
    r.copy("derived/materials.fbx", "models/materials.fbx")
    r.import_("materials", "models/materials.fbx")
    for key in ("material_quads", "backdrop"):
        r.copy(f"controls/{key}.glb", f"controls/{key}.glb")
        r.import_(key, f"controls/{key}.glb")
    ents = [S.model_entity(S.ulid("MT", n), name, r.assets[key], S.transform(t, scale=(s, s, s)))
            for n, name, key, t, s in MAT_PLACEMENTS]
    cam_ents, cam_ids = cams("MT", MAT_CAMERAS)
    r.author(ents + cam_ents, "Lambert material case layout")
    r.record["cameras"] = cam_ids
    for name, _, _, _ in MAT_CAMERAS:
        r.capture(name, cam_ids[name], "with sources")
    return str(r.save_record())


# --------------------------------------------------------------------------- reimport
RE_CAMERAS = [
    ("r3q", (3.4, 2.0, 3.6), (0.8, 0, 0), {"fov": 40}),
    ("rfront", (0.8, 0, 10), (0.8, 0, 0), {"ortho": 1.6}),
]


def reimport(run):
    r = Run(run, "reimport")
    r.copy("derived/crate_v1.fbx", "models/crate.fbx")
    r.copy("derived/panel.fbx", "models/panel.fbx")
    r.copy("art/panel_v1.png", "models/textures/panel.png")
    r.import_("crate", "models/crate.fbx")
    r.import_("panel", "models/panel.fbx")
    ents = [S.model_entity(S.ulid("RW", 1), "crate", r.assets["crate"], S.transform((0, 0, 0), scale=(100, 100, 100))),
            S.model_entity(S.ulid("RW", 2), "panel", r.assets["panel"], S.transform((1.6, 0, 0), scale=(100, 100, 100)))]
    cam_ents, cam = cams("RW", RE_CAMERAS)
    r.author(ents + cam_ents, "Reimport review layout")
    r.record["cameras"] = cam

    def history(label):
        resp = r.rpc([{"method": "history.read"}, {"method": "project.read"}], f"history {label}")
        project = resp[1]["result"]["project"]
        r.step("state", label=label, revision=resp[1]["result"]["revision"], history=resp[0]["result"],
               assets={k: {"path": v["path"], "sha256": v["sha256"]} for k, v in project["assets"].items()})

    history("s0 imported")
    r.capture("s0-r3q", cam["r3q"], "s0 v1/v1")
    r.capture("s0-rfront", cam["rfront"], "s0 v1/v1")
    r.import_("crate", "models/crate.fbx")
    r.import_("panel", "models/panel.fbx")
    history("s1 stable reimport")
    r.capture("s1-r3q", cam["r3q"], "s1 stable reimport")
    r.copy("derived/crate_v2.fbx", "models/crate.fbx")
    r.import_("crate", "models/crate.fbx")
    history("s2 crate v2")
    r.capture("s2-r3q", cam["r3q"], "s2 crate v2")
    r.copy("art/panel_v2.png", "models/textures/panel.png")
    r.import_("panel", "models/panel.fbx")
    history("s3 panel dependency v2")
    r.capture("s3-r3q", cam["r3q"], "s3 panel png v2")
    r.capture("s3-rfront", cam["rfront"], "s3 panel png v2")
    for n, (method, label) in enumerate([("history.undo", "s4 undo panel"), ("history.undo", "s5 undo crate"),
                                         ("history.redo", "s6 redo crate"), ("history.redo", "s7 redo panel")], 4):
        r.rpc([{"method": method}, {"method": "project.save"}], label)
        history(label)
        r.capture(f"s{n}-r3q", cam["r3q"], label)
    r.copy("derived/crate_unsupported.fbx", "models/crate.fbx")
    r.import_("crate", "models/crate.fbx", expect_ok=False)
    history("s8 rejected unsupported")
    r.capture("s8-r3q", cam["r3q"], "s8 rejected unsupported reimport")
    r.copy("derived/crate_malformed.fbx", "models/crate.fbx")
    r.import_("crate", "models/crate.fbx", expect_ok=False)
    history("s9 rejected malformed")
    r.capture("s9-r3q", cam["r3q"], "s9 rejected malformed reimport")
    removed = sorted(str(p.relative_to(r.dir)) for p in (r.dir / "models").rglob("*") if p.is_file())
    for rel in removed:
        (r.dir / rel).unlink()
    r.step("remove_sources", files=removed, remaining_models_dir=sorted(str(p.relative_to(engine.ROOT)) for p in (r.dir / "models").rglob("*")))
    r.capture("s10-r3q", cam["r3q"], "s10 sources removed, fresh process")
    r.capture("s10-rfront", cam["rfront"], "s10 sources removed, fresh process")
    r.step("validate", **engine.validate(r.project, label="reimport validate source-free", run_id=run))
    r.import_("crate", "models/crate.fbx", expect_ok=False)
    history("s11 reimport without source")
    return str(r.save_record())


def repeat(run, captures):
    """captures: list of (project path, camera id, label) to re-render in fresh processes."""
    out = []
    for project, camera, label in captures:
        frame = engine.ART / "runs" / run / "repeat" / f"{label}.png"
        shot = engine.screenshot(Path(project), frame, label=f"repeat {label}", camera=camera, width=W, height=H, run_id=run)
        out.append({"label": label, "project": str(Path(project).relative_to(engine.ROOT)), "camera": camera,
                    "sha256": shot.get("sha256"), "path": str(frame.relative_to(engine.ROOT))})
    path = EVIDENCE / f"{run}-repeat.json"
    path.write_text(json.dumps(out, indent=1) + "\n")
    return str(path)


if __name__ == "__main__":
    command, run_id = sys.argv[1], sys.argv[2]
    if command == "repeat":
        specs = [tuple(arg.split(",")) for arg in sys.argv[3:]]
        print(repeat(run_id, specs))
    else:
        print({"static": static, "textures": textures, "textures-twin": textures_twin, "materials": materials, "reimport": reimport}[command](run_id))
