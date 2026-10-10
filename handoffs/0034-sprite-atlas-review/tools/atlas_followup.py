#!/usr/bin/env python3
"""Focused follow-up for handoff 0034 on the active (follow-up) binary. Standard library only.

Reuses round one's art, scene, cameras and control binding, and only renders what the
delta needs: the bounded skyline packer, import-time sprite-reference rejection,
`validate` diagnostics, and the 64-texel span renderer.

  f0  v1: full view set once, plus the subset again in fresh processes
  f1  ballast image resized (relocation) -> f2 Undo -> f3 Redo
  f4  v3 descriptor (padding 5, other maximum)
  f5  rejected imports: cook errors and new binding errors (missing frame, shrunken
      nine-slice, undersized ordinary texture, non-colour usage); project, journal,
      revision and history must not change; orphan cache files are recorded
  f6  a valid replacement that drops an unreferenced frame, then Undo/Redo
  f7  live watcher: one valid source (recoloured badge) and one invalid atlas in the
      same polling cycle, then the atlas restored; the badge is then reimported back
  f8  validate: clean project (raw-rectangle warning), a frame ID authored on an
      ordinary texture (error, nonzero exit, then undone), a missing cooked cache
      in a scratch copy
  f9  every source removed: full view set, source-free validate, failing reimport
  control: the explicit-rectangle control project, full view set

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_followup.py OUT_DIR [--binary B] [--round R]
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_followup.py OUT_DIR --import-diagnostics
The second form renders nothing; it records the v2/v3 reimport replies' diagnostics.
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_followup.py OUT.json --full-messages IMPORT_DIAG_RUN
The third renders nothing; it re-runs the four binding rejections and keeps their full text.
"""

import argparse
import json
import shutil
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(TOOLS))
import atlas_lookdev as lk  # noqa: E402
import atlas_scene as sc  # noqa: E402

ART_0032 = ROOT / "handoffs/0032-sprite-rendering/evidence/art"
REJECTIONS = ["fail-nofit", "fail-bounds-area", "fail-bounds-axis", "fail-padding0", "fail-padding17",
              "fail-identity", "fail-rect", "fail-frames257", "fail-texels", "render-fail-shrink-panel",
              "render-fail-drop-cardB"]


def cache_files(project):
    return sorted(p.name for p in (project.dir / ".incant/cache/textures").iterdir())


def revision(project):
    reply = project.rpc([{"id": 1, "method": "project.read"}, {"id": 2, "method": "history.read"}])
    return {"revision": reply[0]["result"]["revision"], "history": [h["id"] for h in reply[1]["result"]]}


def validate(project):
    result = lk.engine("validate", project.file, check=False)
    try:
        reply = json.loads(result.stdout)
    except json.JSONDecodeError:
        reply = None
    return {"exit": result.returncode, "stderr": result.stderr.strip()[-1500:], "reply": reply}


def fingerprints(project):
    return {k: v["sha256"] for k, v in json.loads(project.file.read_text(encoding="utf-8"))["assets"].items()}


def rejection(project, source, usage=None):
    """A rejected import must leave authored bytes, revision, history and asset
    fingerprints unchanged; new content-addressed cache files are recorded, not judged."""
    before = {"authored": project.authored(), "state": revision(project), "cache": cache_files(project),
              "assets": fingerprints(project)}
    entry = project.import_(source, usage=usage, expect_error=True)
    after = {"authored": project.authored(), "state": revision(project), "cache": cache_files(project),
             "assets": fingerprints(project)}
    entry.update(authored_unchanged=before["authored"] == after["authored"],
                 revision_unchanged=before["state"] == after["state"],
                 asset_fingerprints_unchanged=before["assets"] == after["assets"],
                 new_cache_files=sorted(set(after["cache"]) - set(before["cache"])),
                 revision=before["state"]["revision"])
    return entry


def import_diagnostics(out):
    """Frame-free: the same scene, then the v2 and v3 reimports; records each import
    reply's sprite diagnostics (the raw-rectangle warning must stay visible)."""
    out.mkdir(parents=True)
    lk.run([sys.executable, "-I", TOOLS / "atlas_art.py", out / "art"])
    art = out / "art"
    manifest = json.loads((art / "manifest.json").read_text(encoding="utf-8"))
    atlas = lk.Project(out / "atlas", "atlas")
    (atlas.dir / "art").mkdir(parents=True)
    lk.engine("init", atlas.file, "--name", "Atlas import diagnostics", "--entities", "0")
    for name in lk.SOURCES + ["badge.png"]:
        shutil.copyfile(art / name, atlas.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png")
    shutil.copyfile(art / "atlas-v1.atlas.json", atlas.dir / "art/atlas.atlas.json")
    shutil.copyfile(art / "alt.atlas.json", atlas.dir / "art/alt.atlas.json")
    v1 = atlas.import_("art/atlas.atlas.json", usage="color")
    alt = atlas.import_("art/alt.atlas.json", usage="color")
    badge = atlas.import_("art/badge.png", usage="color")
    card7 = next(fid for fid, f in manifest["frames"].items() if f["name"] == "card7")
    binding = sc.Binding("atlas", {"atlas": v1["asset"]["id"], "alt": alt["asset"]["id"],
                                   "badge": badge["asset"]["id"], "ballast_v1_rect": [0, 0, 1200, 40]}, manifest,
                         v1["details"]["atlas"]["frames"][card7]["region"])
    s = sc.build(binding, badge["asset"]["id"])
    sc.camera_entities(s)
    lk.create_entities(atlas, s, "Atlas import diagnostics scene (handoff 0034)")
    shutil.copyfile(art / "ballast-v2.png", atlas.dir / "art/ballast.png")
    v2 = atlas.import_("art/atlas.atlas.json")
    shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png")
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    v3 = atlas.import_("art/atlas.atlas.json")
    name = {v: k for k, v in s.ids.items()}
    result = {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
              "binary_sha256": lk.verify_binary(),
              "imports": {k: {"exit": e["exit"], "fingerprint": e["asset"]["sha256"],
                              "page": e["details"]["atlas"]["size"],
                              "diagnostics": [dict(d, entity=name.get(d["entity_id"])) for d in e["diagnostics"]]}
                          for k, e in (("v1-before-scene", v1), ("v2", v2), ("v3", v3))}}
    (out / "import-diagnostics.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
    print(json.dumps(result, indent=1))


def full_messages(run, out):
    """Frame-free: in an --import-diagnostics run's project (v3, sources present), import each
    binding-rejection case once more and keep the complete rejection text."""
    atlas = lk.Project(run / "atlas", "atlas")
    art = run / "art"
    shutil.copyfile(art / "badge-small.png" if (art / "badge-small.png").exists() else ART_0032 / "badge-small.png",
                    run / "badge-small.png")
    cases = {}
    for name, source, usage, prepare in (
            ("render-fail-shrink-panel", "art/atlas.atlas.json", None, ("render-fail-shrink-panel.atlas.json",
                                                                          "atlas.atlas.json")),
            ("render-fail-drop-cardB", "art/atlas.atlas.json", None, ("render-fail-drop-cardB.atlas.json",
                                                                        "atlas.atlas.json")),
            ("badge-undersized-8x8", "art/badge.png", None, (None, "badge.png")),
            ("badge-usage-linear", "art/badge.png", "linear", ("badge.png", "badge.png"))):
        src = run / "badge-small.png" if prepare[0] is None else art / prepare[0]
        shutil.copyfile(src, atlas.dir / "art" / prepare[1])
        cases[name] = rejection(atlas, source, usage=usage)
        shutil.copyfile(art / ("atlas-v3.atlas.json" if prepare[1] == "atlas.atlas.json" else "badge.png"),
                        atlas.dir / "art" / prepare[1])
    result = {"binary_unchanged_every_call": lk.CALLS["unchanged"], "engine_calls": lk.CALLS["count"],
              "binary_sha256": lk.verify_binary(),
              "cases": {k: {"exit": v["exit"], "authored_unchanged": v["authored_unchanged"],
                            "revision_unchanged": v["revision_unchanged"],
                            "asset_fingerprints_unchanged": v["asset_fingerprints_unchanged"],
                            "new_cache_files": len(v["new_cache_files"]), "stderr": v["stderr"]}
                        for k, v in cases.items()}}
    out.write_text(json.dumps(result, indent=1), encoding="utf-8")
    print(json.dumps({k: (v["exit"], v["authored_unchanged"], v["revision_unchanged"], v["stderr"].splitlines()[0])
                      for k, v in result["cases"].items()}, indent=1))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--import-diagnostics", action="store_true", help="frame-free reimport warning capture")
    parser.add_argument("--full-messages", type=Path, metavar="IMPORT_DIAGNOSTICS_RUN",
                        help="frame-free: complete binding-rejection text, written to OUT (a file)")
    lk.add_selectors(parser)
    args = parser.parse_args()
    lk.apply_selectors(args)
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    if args.full_messages:
        full_messages(args.full_messages.resolve(), out)
        return
    if args.import_diagnostics:
        import_diagnostics(out)
        return
    plans = ([sc.FULL, sc.SUBSET, sc.FULL, sc.SUBSET, sc.SUBSET, sc.FULL] + [sc.PP_SMALL] * 4
             + [sc.FULL, sc.FULL])
    frames, raw = lk.plan_cost(plans)
    used = lk.ledger_used()
    if used[0] + frames > lk.PACKET_FRAMES or used[1] + raw > lk.PACKET_RAW:
        raise SystemExit(f"budget: needs {frames} frames / {raw / 2**20:.1f} MiB; round used {used[0]} frames / "
                         f"{used[1] / 2**20:.1f} MiB of {lk.PACKET_FRAMES} / {lk.PACKET_RAW / 2**20:.0f} MiB")
    start = lk.verify_binary()
    out.mkdir(parents=True)
    lk.run([sys.executable, "-I", TOOLS / "atlas_art.py", out / "art"])
    art = out / "art"
    for name in ("badge-orange.png", "badge-small.png"):
        shutil.copyfile(ART_0032 / name, art / name)
    manifest = json.loads((art / "manifest.json").read_text(encoding="utf-8"))
    metadata = json.loads(lk.BINARY_JSON.read_text(encoding="utf-8"))
    summary = {"run": out.name, "binary": args.binary, "binary_source_commit": metadata["source_commit"],
               "binary_sha256_start": start, "planned": {"frames": frames, "raw_bytes": raw}}

    atlas = lk.Project(out / "atlas", "atlas")
    (atlas.dir / "art").mkdir(parents=True)
    lk.engine("init", atlas.file, "--name", "Atlas follow-up", "--entities", "0")
    for name in lk.SOURCES + ["badge.png"]:
        shutil.copyfile(art / name, atlas.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png")
    shutil.copyfile(art / "atlas-v1.atlas.json", atlas.dir / "art/atlas.atlas.json")
    shutil.copyfile(art / "alt.atlas.json", atlas.dir / "art/alt.atlas.json")
    imports = {"v1": atlas.import_("art/atlas.atlas.json", usage="color")}
    alt = atlas.import_("art/alt.atlas.json", usage="color")
    badge = atlas.import_("art/badge.png", usage="color")
    atlas_id = imports["v1"]["asset"]["id"]
    card7 = next(fid for fid, f in manifest["frames"].items() if f["name"] == "card7")
    raw_rect = imports["v1"]["details"]["atlas"]["frames"][card7]["region"]
    binding = sc.Binding("atlas", {"atlas": atlas_id, "alt": alt["asset"]["id"], "badge": badge["asset"]["id"],
                                   "ballast_v1_rect": [0, 0, 1200, 40]}, manifest, raw_rect)
    s = sc.build(binding, badge["asset"]["id"])
    cams = sc.camera_entities(s)
    scene_id = lk.create_entities(atlas, s, "Atlas follow-up scene (handoff 0034)")
    (out / "scene-atlas.json").write_text(json.dumps({"entities": s.entities, "meta": s.meta, "ids": s.ids,
                                                       "cameras": cams}, indent=1), encoding="utf-8")
    summary["atlas"] = {"asset": atlas_id, "alt": alt["asset"]["id"], "badge": badge["asset"]["id"],
                        "scene": scene_id, "raw_rect_card7_v1": raw_rect, "entities": len(s.entities)}
    shooter = lk.Shooter(out, out.name, cams)
    states = {}

    def state(name, plan, note, check=True, **extra):
        states[name] = {"note": note, "authored": atlas.authored(), "fingerprint": atlas.fingerprint(atlas_id),
                        "shots": shooter.shots(atlas, name, plan, check=check), **extra}
        return states[name]

    summary.update(imports=imports, states=states)
    try:
        sequence(out, art, manifest, atlas, atlas_id, s, cams, scene_id, badge, imports, states, state, shooter,
                 summary)
    finally:
        summary["binary_sha256_end"] = lk.verify_binary()
        summary["engine_calls"] = lk.CALLS["count"]
        summary["screenshots"] = lk.CALLS["screenshots"]
        summary["failed_screenshots"] = lk.CALLS["failed_screenshots"]
        summary["binary_unchanged_every_call"] = lk.CALLS["unchanged"] and start == summary["binary_sha256_end"]
        summary["ledger_after"] = dict(zip(("frames", "raw_bytes"), lk.ledger_used()))
        summary["complete"] = "control" in summary
        (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary_unchanged": summary["binary_unchanged_every_call"], "engine_calls": lk.CALLS["count"],
                      "screenshots": lk.CALLS["screenshots"], "failed_screenshots": lk.CALLS["failed_screenshots"],
                      "fingerprints": {k: v["fingerprint"][:12] for k, v in states.items() if "fingerprint" in v},
                      "rejections": {k: (v["exit"], v.get("authored_unchanged"), v.get("revision_unchanged"),
                                         v.get("asset_fingerprints_unchanged"))
                                     for k, v in summary["rejections"].items()},
                      "validate": {k: v["exit"] for k, v in summary["validations"].items()},
                      "watch": summary["watch_exit"], "ledger": summary["ledger_after"]}, indent=1))


def sequence(out, art, manifest, atlas, atlas_id, s, cams, scene_id, badge, imports, states, state, shooter,
             summary):

    validations = summary.setdefault("validations", {})
    validations["after-create"] = validate(atlas)
    state("f0-v1-r1", sc.FULL, "v1 under the bounded skyline packer")
    state("f0-v1-r2", sc.SUBSET, "v1 subset again, fresh processes")
    shutil.copyfile(art / "ballast-v2.png", atlas.dir / "art/ballast.png")
    imports["v2"] = atlas.import_("art/atlas.atlas.json")
    state("f1-v2", sc.FULL, "ballast image 1200x40 -> 1240x150; same descriptor and frame IDs")
    undo = atlas.rpc([{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])
    state("f2-undo", sc.SUBSET, "history.undo of the v2 reimport", rpc=[r.get("result", {}).get("transaction_id")
                                                                       for r in undo])
    redo = atlas.rpc([{"id": 1, "method": "history.redo"}, {"id": 2, "method": "project.save"}])
    state("f3-redo", sc.SUBSET, "history.redo of the v2 reimport", rpc=[r.get("result", {}).get("transaction_id")
                                                                       for r in redo])
    shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png")
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    imports["v3"] = atlas.import_("art/atlas.atlas.json")
    state("f4-v3", sc.FULL, "descriptor padding 2 -> 5, maximum 2048x2048 -> 1600x2600, ballast v1")

    # f5: every rejection must leave project, journal, revision and history untouched.
    rejections = summary.setdefault("rejections", {})
    for name in REJECTIONS:
        if name == "fail-texels":
            for extra in ("big2048.png", "one.png"):
                shutil.copyfile(art / extra, atlas.dir / "art" / extra)
        shutil.copyfile(art / f"{name}.atlas.json", atlas.dir / "art/atlas.atlas.json")
        rejections[name] = rejection(atlas, "art/atlas.atlas.json")
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    rejections["atlas-usage-linear"] = rejection(atlas, "art/atlas.atlas.json", usage="linear")
    shutil.copyfile(art / "badge-small.png", atlas.dir / "art/badge.png")
    rejections["badge-undersized-8x8"] = rejection(atlas, "art/badge.png")
    shutil.copyfile(art / "badge.png", atlas.dir / "art/badge.png")
    rejections["badge-usage-linear"] = rejection(atlas, "art/badge.png", usage="linear")
    for extra in ("big2048.png", "one.png"):
        (atlas.dir / "art" / extra).unlink()
    rejections["unchanged-reimport"] = atlas.import_("art/atlas.atlas.json")
    validations["after-rejections"] = validate(atlas)
    state("f5-after-rejections", sc.PP_SMALL, "after every rejected import", rejections=rejections)

    # f6: a valid replacement that drops a frame no sprite references, then Undo/Redo.
    referenced = {m["frame"] for m in s.meta.values() if m.get("frame")}
    descriptor = json.loads((art / "atlas-v3.atlas.json").read_text(encoding="utf-8"))
    unused = sorted(fid for fid, f in descriptor["frames"].items() if f["name"] not in referenced)
    dropped = unused[0]
    descriptor["frames"].pop(dropped)
    (out / "art-followup").mkdir()
    (out / "art-followup/drop-unused.atlas.json").write_text(json.dumps(descriptor, indent=1), encoding="utf-8")
    shutil.copyfile(out / "art-followup/drop-unused.atlas.json", atlas.dir / "art/atlas.atlas.json")
    imports["drop-unused"] = atlas.import_("art/atlas.atlas.json", expect_error=True)
    if imports["drop-unused"]["exit"] != 0:
        raise SystemExit(f"valid replacement was rejected: {imports['drop-unused']['stderr']}")
    state("f6-drop-unused", sc.PP_SMALL, f"valid replacement dropping unreferenced frame {dropped}",
          unreferenced_frames=len(unused), dropped=dropped)
    states["f6-drop-unused"]["undo"] = [r.get("result", {}).get("transaction_id") for r in atlas.rpc(
        [{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])]
    states["f6-drop-unused"]["fingerprint_after_undo"] = atlas.fingerprint(atlas_id)
    states["f6-drop-unused"]["redo"] = [r.get("result", {}).get("transaction_id") for r in atlas.rpc(
        [{"id": 1, "method": "history.redo"}, {"id": 2, "method": "project.save"}])]
    states["f6-drop-unused"]["fingerprint_after_redo"] = atlas.fingerprint(atlas_id)
    states["f6-drop-unused"]["undo_back_to_v3"] = [r.get("result", {}).get("transaction_id") for r in atlas.rpc(
        [{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])]
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    states["f6-drop-unused"]["fingerprint_final"] = atlas.fingerprint(atlas_id)

    # f7: one polling cycle sees a valid badge recolour and an atlas that drops card B.
    def change_both():
        shutil.copyfile(art / "badge-orange.png", atlas.dir / "art/badge.png")
        shutil.copyfile(art / "render-fail-drop-cardB.atlas.json", atlas.dir / "art/atlas.atlas.json")
    before_watch = atlas.authored()
    watch = lk.live_watch(atlas, [
        (lambda e: e.get("event") == "assets", change_both),
        (lambda e: e.get("event") == "assets" and e["report"]["diagnostics"],
         lambda: shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json"))], polls=50)
    state("f7-watch-mixed", sc.PP_SMALL, "watcher imported the orange badge and rejected the atlas",
          check=False, watch=watch, authored_changed=before_watch != atlas.authored())
    shutil.copyfile(art / "badge.png", atlas.dir / "art/badge.png")
    imports["badge-restored"] = atlas.import_("art/badge.png")
    state("f7-badge-restored", sc.PP_SMALL, "original badge reimported through import")

    # f8: validate diagnostics.
    card0 = s.ids["S:card0:1x"]
    value = dict(next(e for e in s.entities if e["id"] == card0)["components"]["SpriteRenderer"],
                 texture=badge["asset"]["id"])
    rev = atlas.rpc([{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    atlas.rpc([{"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "set_component", "scene_id": scene_id, "entity_id": card0,
                      "component": "SpriteRenderer", "value": value}],
        "expected_revision": rev, "description": "Frame reference on an ordinary texture"}},
        {"id": 3, "method": "project.save"}])
    validations["frame-on-ordinary"] = validate(atlas)
    states["f8-frame-on-ordinary"] = {"shots": shooter.shots(atlas, "f8-frame-on-ordinary", [("pp-S", (320, 180))],
                                                             check=False)}
    atlas.rpc([{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])
    validations["after-undo"] = validate(atlas)
    scratch = out / "scratch-missing-cache"
    shutil.copytree(atlas.dir, scratch)
    copy = lk.Project(scratch, "atlas")
    cache = scratch / ".incant/cache/textures"
    meta = json.loads((cache / f"{atlas.fingerprint(atlas_id)}.json").read_text(encoding="utf-8"))
    (cache / f"{meta['content_sha256']}.ktx2").unlink()
    validations["missing-cache-copy"] = validate(copy)

    # f9: source-free.
    removed = sorted(str(p.relative_to(atlas.dir)) for p in (atlas.dir / "art").iterdir() if p.is_file())
    for rel in removed:
        (atlas.dir / rel).unlink()
    state("f9-source-free", sc.FULL, "all art/ sources removed; cooked cache only", removed=removed)
    validations["source-free"] = validate(atlas)
    states["f9-source-free"]["reimport_without_sources"] = atlas.import_("art/atlas.atlas.json", expect_error=True)
    summary.update(validations=validations, rejections=rejections, watch_exit=watch["exit"],
                   history=atlas.rpc([{"id": 1, "method": "history.read"}])[0]["result"])
    summary["control"] = lk.build_control(out, art, manifest, s, cams, shooter, sc.FULL)


if __name__ == "__main__":
    main()
