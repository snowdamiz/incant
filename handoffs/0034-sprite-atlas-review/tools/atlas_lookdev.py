#!/usr/bin/env python3
"""Real-engine cooked-atlas look-dev for handoff 0034. Standard library only.

Builds disposable projects in a NEW output directory through public interfaces of
the frozen artifacts/tools/incant_headless only (init, import, rpc, watch-assets,
validate, screenshot). The binary's SHA-256 is checked against binary.json before
and after every call. No project or journal file is ever written by this script;
source art and atlas descriptors are authoring inputs and are copied into place.

  atlas    every sprite references a stable atlas frame ID (one raw-rect sprite
           demonstrates why), plus one ordinary badge texture for batch breaks.
  control  the same entities drawn from the original images with explicit source
           rectangles: the differential oracle for atlas packing and resolution.
  limits   a scratch project for the exact decoded-source-texel limit.

Atlas states: s0 v1 (rendered twice, fresh processes) -> s1 ballast image resized
(relocation, page-size change) -> s2 Undo -> s3 Redo -> s4 live watcher dependency
revert to the v1 image -> s5 v3 descriptor (padding/maximum change) -> s6 rejected
replacements -> s7 live watcher with a failing descriptor -> s8 accepted imports
that cannot render, each undone -> s9 every source file removed.

Usage (repo root):
  python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_lookdev.py OUT_DIR [--binary B] [--round R]
Round one ran this sequence on the pre-followup binary: --binary pre-followup --round r1.
(Its s8 imports are rejected at import time on the follow-up build by design.)
"""

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
sys.path.insert(0, str(TOOLS))
import atlas_scene as sc  # noqa: E402

PACKET_DIR = ROOT / "artifacts/0034-sprite-atlas-review"
# Frozen executables: "active" is the current candidate; "pre-followup" is round one's
# b72389ee build, preserved by Astra for before/after comparisons.
BINARIES = {"active": ("incant_headless", "binary.json"),
            "pre-followup": ("incant_headless-pre-followup", "binary-pre-followup.json")}
# Each review round has its own budget and ledger: round one 2,048 frames / 1 GiB,
# the follow-up 600 frames / 512 MiB. Lifetime totals are their sum.
ROUNDS = {"r1": ("ledger.jsonl", 2048, 1 << 30), "followup": ("ledger-followup.jsonl", 600, 512 << 20)}
CALL_FRAMES, CALL_RAW = 128, 256 << 20


def use_binary(name):
    global CLI, BINARY_JSON
    executable, metadata = BINARIES[name]
    CLI, BINARY_JSON = ROOT / "artifacts/tools" / executable, ROOT / "artifacts/tools" / metadata


def use_round(name):
    global LEDGER, PACKET_FRAMES, PACKET_RAW
    ledger, PACKET_FRAMES, PACKET_RAW = ROUNDS[name]
    LEDGER = PACKET_DIR / ledger


def add_selectors(parser):
    """--binary and --round for every engine-driving tool (defaults: active, followup).
    Round one's commands are reproduced with --binary pre-followup --round r1."""
    parser.add_argument("--binary", choices=sorted(BINARIES), default="active")
    parser.add_argument("--round", choices=sorted(ROUNDS), default="followup")


def apply_selectors(args):
    use_binary(args.binary)
    use_round(args.round)


use_binary("active")
use_round("followup")
SOURCES = ["sheet.png", "cards.png", "probe.png", "hstrip.png", "vstrip.png", "halo.png", "labels34.png"]
FAILURES = ["fail-nofit", "fail-bounds-area", "fail-bounds-axis", "fail-padding0", "fail-padding17",
            "fail-identity", "fail-rect", "fail-frames257", "fail-texels"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = sha(CLI)
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


CALLS = {"count": 0, "unchanged": True, "screenshots": 0, "failed_screenshots": 0}


def run(args, check=True, stdin=None):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, cwd=ROOT, input=stdin)
    if check and result.returncode != 0:
        raise SystemExit(f"{[str(a) for a in args[:4]]} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}")
    return result


def engine(*args, check=True, stdin=None):
    before = verify_binary()
    result = run([CLI, *args], check=check, stdin=stdin)
    after = verify_binary()
    CALLS["count"] += 1
    CALLS["unchanged"] &= before == after
    return result


def ledger_used():
    frames = raw = 0
    if LEDGER.exists():
        for line in LEDGER.read_text(encoding="utf-8").splitlines():
            entry = json.loads(line)
            frames += entry["frames"]
            raw += entry["raw"]
    return frames, raw


def ledger_add(run_name, path, w, h):
    with LEDGER.open("a", encoding="utf-8") as f:
        f.write(json.dumps({"run": run_name, "path": str(Path(path).relative_to(ROOT)), "frames": 1,
                            "raw": w * h * 4}) + "\n")


class Project:
    def __init__(self, directory, stem):
        self.dir = directory
        self.file = directory / f"{stem}.incant.json"
        self.journal = directory / f"{stem}.incant.journal.jsonl"

    def authored(self):
        return {p.name: sha(p) for p in (self.file, self.journal) if p.exists()}

    def rpc(self, requests, expect_error=False):
        result = engine("rpc", self.file, "--journal", self.journal,
                        stdin="".join(json.dumps(r) + "\n" for r in requests), check=not expect_error)
        replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        if not expect_error and any("error" in r for r in replies):
            raise SystemExit(f"RPC error: {replies}")
        return replies

    def import_(self, source, usage=None, expect_error=False):
        prior = self.authored()
        args = ["import", self.file, source] + (["--texture-usage", usage] if usage else [])
        result = engine(*args, check=not expect_error)
        entry = {"source": source, "usage": usage, "exit": result.returncode,
                 "stderr": result.stderr.strip()[-20000:], "authored_unchanged": prior == self.authored()}
        if result.returncode == 0:
            reply = json.loads(result.stdout)
            entry.update(asset=reply["asset"], changed=reply["changed"], cache_hit=reply["cache_hit"],
                         details=reply["details"], revision=reply.get("revision"),
                         diagnostics=reply.get("diagnostics", []))
        return entry

    def fingerprint(self, asset_id):
        return json.loads(self.file.read_text(encoding="utf-8"))["assets"][asset_id]["sha256"]


class Shooter:
    def __init__(self, out, run_name, cams):
        self.out, self.run_name, self.cams = out, run_name, cams

    def shots(self, project, folder, plan, check=True):
        folder = self.out / "states" / folder
        folder.mkdir(parents=True, exist_ok=False)
        record = {}
        for name, (w, h) in plan:
            png = folder / f"{name}-{w}x{h}.png"
            result = engine("screenshot", project.file, png, "--camera", self.cams[name], "--width", w,
                            "--height", h, check=check)
            entry = {"exit": result.returncode, "written": png.exists(), "stderr": result.stderr.strip()[-1500:]}
            if png.exists():
                CALLS["screenshots"] += 1
                ledger_add(self.run_name, png, w, h)
                reply = json.loads(result.stdout)
                entry.update(sha256=sha(png), adapter=reply.get("adapter"), geometry=reply.get("geometry"))
            else:
                CALLS["failed_screenshots"] += 1
            record[f"{name}-{w}x{h}"] = entry
        return record


def plan_cost(plans):
    frames = sum(len(p) for p in plans)
    raw = sum(w * h * 4 for p in plans for _, (w, h) in p)
    return frames, raw


def scene_entities(binding, badge):
    s = sc.build(binding, badge)
    cams = sc.camera_entities(s)
    return s, cams


def create_entities(project, s, description):
    scene_id = next(iter(json.loads(project.file.read_text(encoding="utf-8"))["scenes"]))
    revision = project.rpc([{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    project.rpc([
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in s.entities],
            "expected_revision": revision, "description": description}},
        {"id": 3, "method": "project.save"}])
    engine("validate", project.file)
    return scene_id


def live_watch(project, actions, polls=40, interval=100, debounce=150, timeout=30.0):
    """Run watch-assets; apply each action after the event matching its predicate."""
    before = verify_binary()
    proc = subprocess.Popen([str(CLI), "watch-assets", str(project.file), "--polls", str(polls),
                             "--interval-ms", str(interval), "--debounce-ms", str(debounce)],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=ROOT)
    events, pending = [], list(actions)
    start = time.monotonic()
    for line in proc.stdout:
        event = json.loads(line)
        event["t_ms"] = round(1000 * (time.monotonic() - start))
        events.append(event)
        while pending and pending[0][0](event):
            pending.pop(0)[1]()
            events.append({"event": "action-applied", "t_ms": round(1000 * (time.monotonic() - start))})
        if time.monotonic() - start > timeout:
            proc.kill()
            break
    stderr = proc.stderr.read()
    code = proc.wait()
    after = verify_binary()
    CALLS["count"] += 1
    CALLS["unchanged"] &= before == after
    return {"exit": code, "stderr": stderr.strip()[-1500:], "events": events, "unapplied_actions": len(pending)}


def build_control(out, art, manifest, s, cams, shooter, plan):
    control = Project(out / "control", "control")
    (control.dir / "art").mkdir(parents=True)
    engine("init", control.file, "--name", "Atlas control", "--entities", "0")
    textures = {"ballast_v1_rect": [0, 0, 1200, 40]}
    for name in SOURCES + ["badge.png"]:
        shutil.copyfile(art / name, control.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", control.dir / "art/ballast.png")
    control_imports = {}
    for name in SOURCES + ["ballast.png", "badge.png"]:
        control_imports[name] = control.import_(f"art/{name}", usage="color")
        textures[name] = control_imports[name]["asset"]["id"]
    textures["badge"] = textures["badge.png"]
    cbinding = sc.Binding("control", textures, manifest)
    cs, ccams = scene_entities(cbinding, textures["badge"])
    if [e["id"] for e in cs.entities] != [e["id"] for e in s.entities] or ccams != cams:
        raise SystemExit("control entity IDs differ from the atlas scene")
    create_entities(control, cs, "Atlas control scene (handoff 0034)")
    (out / "scene-control.json").write_text(json.dumps({"entities": cs.entities, "meta": cs.meta, "ids": cs.ids,
                                                         "cameras": ccams}, indent=1), encoding="utf-8")
    return {"imports": control_imports, "authored": control.authored(),
            "shots": shooter.shots(control, "control", plan)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--smoke", action="store_true", help="build both projects, render the subset once each")
    add_selectors(parser)
    args = parser.parse_args()
    apply_selectors(args)
    out = args.out.resolve()
    if not CLI.exists():
        raise SystemExit(f"Missing {CLI}; see artifacts/tools/binary.json")
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    run_name = out.name
    start_hash = verify_binary()
    plans = ([sc.FULL + sc.VIEW] * 2 + [sc.FULL] + [sc.SUBSET] * 3 + [sc.FULL, sc.PP_SMALL, sc.PP_SMALL,
             sc.PP_SMALL, [("pp-S", (320, 180))], sc.FULL, sc.FULL + sc.VIEW])
    if args.smoke:
        plans = [sc.SUBSET, sc.SUBSET]
    frames, raw = plan_cost(plans)
    used_frames, used_raw = ledger_used()
    if used_frames + frames > PACKET_FRAMES or used_raw + raw > PACKET_RAW:
        raise SystemExit(f"budget: this run needs {frames} frames / {raw / 2**20:.1f} MiB; packet used "
                         f"{used_frames} frames / {used_raw / 2**20:.1f} MiB of {PACKET_FRAMES} / 1024 MiB")
    if max(w * h * 4 for p in plans for _, (w, h) in p) > CALL_RAW:
        raise SystemExit("a single frame exceeds the per-invocation budget")
    out.mkdir(parents=True)
    PACKET_DIR.mkdir(parents=True, exist_ok=True)
    run([sys.executable, "-I", TOOLS / "atlas_art.py", out / "art"])
    art = out / "art"
    manifest = json.loads((art / "manifest.json").read_text(encoding="utf-8"))
    metadata = json.loads(BINARY_JSON.read_text(encoding="utf-8"))
    summary = {"run": run_name, "binary_source_commit": metadata["source_commit"],
               "binary_sha256_start": start_hash, "planned": {"frames": frames, "raw_bytes": raw},
               "art_manifest_sha256": sha(art / "manifest.json")}

    # ---------------------------------------------------------- atlas project --
    atlas = Project(out / "atlas", "atlas")
    (atlas.dir / "art").mkdir(parents=True)
    engine("init", atlas.file, "--name", "Atlas look-dev", "--entities", "0")
    for name in SOURCES + ["badge.png"]:
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
    s, cams = scene_entities(binding, badge["asset"]["id"])
    scene_id = create_entities(atlas, s, "Atlas look-dev scene (handoff 0034)")
    summary["atlas"] = {"asset": atlas_id, "alt": alt, "badge": badge["asset"]["id"], "scene": scene_id,
                        "raw_rect_card7_v1": raw_rect, "entities": len(s.entities)}
    (out / "scene-atlas.json").write_text(json.dumps({"entities": s.entities, "meta": s.meta, "ids": s.ids,
                                                       "cameras": cams}, indent=1), encoding="utf-8")
    shooter = Shooter(out, run_name, cams)
    states = {}
    if args.smoke:
        summary["smoke_atlas"] = shooter.shots(atlas, "smoke-atlas", sc.SUBSET)
        summary["control"] = build_control(out, art, manifest, s, cams, shooter, sc.SUBSET)
        summary["engine_calls"] = CALLS["count"]
        summary["binary_unchanged_every_call"] = CALLS["unchanged"] and start_hash == verify_binary()
        (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
        print(json.dumps({"smoke": True, "engine_calls": CALLS["count"], "ledger": ledger_used(),
                          "binary_unchanged": summary["binary_unchanged_every_call"]}))
        return

    def state(name, plan, note, check=True, **extra):
        record = {"note": note, "authored": atlas.authored(), "fingerprint": atlas.fingerprint(atlas_id),
                  "shots": shooter.shots(atlas, name, plan, check=check)}
        record.update(extra)
        states[name] = record
        return record

    state("s0-r1", sc.FULL + sc.VIEW, "v1, first fresh-process render")
    state("s0-r2", sc.FULL + sc.VIEW, "v1, repeated fresh-process render")
    # Relocation: only the ballast image changes; the descriptor and every frame ID stay.
    shutil.copyfile(art / "ballast-v2.png", atlas.dir / "art/ballast.png")
    imports["v2"] = atlas.import_("art/atlas.atlas.json")
    state("s1-v2", sc.FULL, "ballast image 1200x40 -> 1240x150; same descriptor and frame IDs")
    history = {"after_v2": atlas.rpc([{"id": 1, "method": "history.read"}])[0]["result"]}
    undo = atlas.rpc([{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])
    state("s2-undo", sc.SUBSET, "history.undo of the v2 reimport", rpc=[r.get("result", {}).get("transaction_id")
                                                                       for r in undo])
    redo = atlas.rpc([{"id": 1, "method": "history.redo"}, {"id": 2, "method": "project.save"}])
    state("s3-redo", sc.SUBSET, "history.redo of the v2 reimport", rpc=[r.get("result", {}).get("transaction_id")
                                                                       for r in redo])
    # Live watcher: revert only the dependency image; the atlas must reimport by itself.
    watch_revert = live_watch(atlas, [(lambda e: e.get("event") == "assets",
                                       lambda: shutil.copyfile(art / "ballast-v1.png", atlas.dir / "art/ballast.png"))])
    state("s4-watch-revert", sc.SUBSET, "watch-assets reimport after reverting ballast.png to v1",
          watch=watch_revert)
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    imports["v3"] = atlas.import_("art/atlas.atlas.json")
    state("s5-v3", sc.FULL, "descriptor padding 2 -> 5, maximum 2048x2048 -> 1600x2600; same frames")

    # Rejected replacements: every import must fail and leave project, journal and pixels.
    failures = {}
    for name in FAILURES:
        if name == "fail-texels":
            for extra in ("big2048.png", "one.png"):
                shutil.copyfile(art / extra, atlas.dir / "art" / extra)
        shutil.copyfile(art / f"{name}.atlas.json", atlas.dir / "art/atlas.atlas.json")
        failures[name] = atlas.import_("art/atlas.atlas.json", expect_error=True)
    shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
    failures["usage-linear"] = atlas.import_("art/atlas.atlas.json", usage="linear", expect_error=True)
    for extra in ("big2048.png", "one.png"):
        (atlas.dir / "art" / extra).unlink()
    failures["unchanged-reimport"] = atlas.import_("art/atlas.atlas.json")
    state("s6-after-rejections", sc.PP_SMALL, "after nine rejected descriptors and a linear-usage request",
          imports=failures)

    # Live watcher: a failing descriptor while running, then the good one restored.
    watch_fail = live_watch(atlas, [
        (lambda e: e.get("event") == "assets",
         lambda: shutil.copyfile(art / "fail-nofit.atlas.json", atlas.dir / "art/atlas.atlas.json")),
        (lambda e: e.get("event") == "assets" and e["report"]["diagnostics"],
         lambda: shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json"))])
    states["s7-watch-fail"] = {"note": "watch-assets with an unfittable descriptor, then restored",
                               "authored": atlas.authored(), "fingerprint": atlas.fingerprint(atlas_id),
                               "watch": watch_fail}

    # Accepted imports whose frames no longer satisfy the scene; each is undone.
    render_failures = {}
    for name, probe_shots in (("render-fail-shrink-panel", [("pp-N", (320, 180)), ("pp-S", (320, 180))]),
                              ("render-fail-drop-cardB", [("pp-S", (320, 180)), ("pp-O", (320, 180))])):
        shutil.copyfile(art / f"{name}.atlas.json", atlas.dir / "art/atlas.atlas.json")
        entry = {"import": atlas.import_("art/atlas.atlas.json")}
        entry["validate"] = engine("validate", atlas.file, check=False).stdout.strip()
        entry["shots"] = shooter.shots(atlas, f"s8-{name}", probe_shots, check=False)
        entry["undo"] = [r.get("result", {}).get("transaction_id") for r in
                         atlas.rpc([{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])]
        shutil.copyfile(art / "atlas-v3.atlas.json", atlas.dir / "art/atlas.atlas.json")
        entry["restored"] = shooter.shots(atlas, f"s8-{name}-undone", sc.PP_SMALL)
        entry["fingerprint_after_undo"] = atlas.fingerprint(atlas_id)
        render_failures[name] = entry
    # A frame reference on an ordinary (non-atlas) texture.
    card0 = s.ids["S:card0:1x"]
    value = dict(next(e for e in s.entities if e["id"] == card0)["components"]["SpriteRenderer"],
                 texture=badge["asset"]["id"])
    revision = atlas.rpc([{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    atlas.rpc([{"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "set_component", "scene_id": scene_id, "entity_id": card0,
                      "component": "SpriteRenderer", "value": value}],
        "expected_revision": revision, "description": "Frame reference on an ordinary texture"}},
        {"id": 3, "method": "project.save"}])
    entry = {"shots": shooter.shots(atlas, "s8-frame-on-ordinary", [("pp-S", (320, 180))], check=False)}
    atlas.rpc([{"id": 1, "method": "history.undo"}, {"id": 2, "method": "project.save"}])
    entry["restored"] = shooter.shots(atlas, "s8-frame-on-ordinary-undone", [("pp-S", (320, 180))])
    render_failures["frame-on-ordinary"] = entry
    states["s8-render-failures"] = {"note": "accepted imports that make scene preparation fail",
                                    "authored": atlas.authored(), "fingerprint": atlas.fingerprint(atlas_id),
                                    "cases": render_failures}

    # Source-free: remove every authored source (descriptor and images), keep the cache.
    removed = sorted(str(p.relative_to(atlas.dir)) for p in (atlas.dir / "art").iterdir() if p.is_file())
    for rel in removed:
        (atlas.dir / rel).unlink()
    state("s9-source-free", sc.FULL, "all art/ sources removed; cooked cache only", removed=removed)
    failures_after = atlas.import_("art/atlas.atlas.json", expect_error=True)
    states["s9-source-free"]["reimport_without_sources"] = failures_after
    history["final"] = atlas.rpc([{"id": 1, "method": "history.read"}])[0]["result"]
    summary.update(imports=imports, history=history, states=states)

    summary["control"] = build_control(out, art, manifest, s, cams, shooter, sc.FULL + sc.VIEW)

    # ---------------------------------------------------- exact texel limit --
    limits = Project(out / "limits", "limits")
    (limits.dir / "art").mkdir(parents=True)
    engine("init", limits.file, "--name", "Atlas limits", "--entities", "0")
    shutil.copyfile(art / "big2048.png", limits.dir / "art/big2048.png")
    shutil.copyfile(art / "limit-texels-exact.atlas.json", limits.dir / "art/limit.atlas.json")
    summary["limits"] = {"exact_4194304": limits.import_("art/limit.atlas.json", usage="color")}

    summary["binary_sha256_end"] = verify_binary()
    summary["engine_calls"] = CALLS["count"]
    summary["screenshots"] = CALLS["screenshots"]
    summary["failed_screenshots"] = CALLS["failed_screenshots"]
    summary["binary_unchanged_every_call"] = CALLS["unchanged"] and start_hash == summary["binary_sha256_end"]
    summary["ledger_after"] = dict(zip(("frames", "raw_bytes"), ledger_used()))
    (out / "helper.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")
    print(json.dumps({"binary_unchanged": summary["binary_unchanged_every_call"], "engine_calls": CALLS["count"],
                      "screenshots": CALLS["screenshots"], "failed_screenshots": CALLS["failed_screenshots"],
                      "fingerprints": {k: v["fingerprint"][:12] for k, v in states.items()},
                      "rejections": {k: v["exit"] for k, v in failures.items()},
                      "watch": {"revert": watch_revert["exit"], "fail": watch_fail["exit"]},
                      "ledger": summary["ledger_after"]}))


if __name__ == "__main__":
    main()
