#!/usr/bin/env python3
"""0073 r4_quality: CPU-only guard/source self-check of the adapted execution copy.
Never calls the runner's run branch, Cargo, a GPU or an image tool. Read-only git on the
candidate checkout; mutated plans go only to /tmp/incant-0073-r4-quality-* scratch.

    python3 -I -B handoffs/0073-upscaling-review/r4_quality/execution/selfcheck.py \\
        --checkout /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant
"""
import argparse, copy, difflib, hashlib, importlib.util, json, pathlib, subprocess, sys, tempfile
sys.dont_write_bytecode = True
HERE = pathlib.Path(__file__).resolve().parent
PKT = HERE.parents[1]
OLD = "76be9608e99bf089fcb1e0908827330f70c28ac1"
SOURCE = "825ff047284e84252d00f786a7b14cb6ecb44293"
PRODUCTION = "72b1ba8ec351e86d66a90c401e0a75a530633616"
CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def sha(p):
    return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--checkout", required=True); a = ap.parse_args()
    git = lambda *args: subprocess.run(["git", "-C", a.checkout, *args], capture_output=True, text=True)
    fixture, old_fixture = PKT / "r4_quality/review/upscaling_owner_review.rs", PKT / "owner_quality/review/upscaling_owner_review.rs"
    check("fixture copy is byte-identical to the reviewed 650316a fixture",
          fixture.read_bytes() == old_fixture.read_bytes() and sha(fixture) == "650316a21f30bfb4f7cbe8c99b67f9b5be269b51eb62b4d017b519bd5130d997")
    old_runner = (PKT / "owner_output_quality/execution/runner.py").read_text().splitlines()
    new_runner = (HERE / "runner.py").read_text().splitlines()
    delta = [l for l in difflib.unified_diff(old_runner, new_runner, lineterm="", n=0) if l[:1] in "+-" and l[:3] not in ("---", "+++")]
    allowed = {"-FIXTURE = PACKET / 'review/upscaling_owner_review.rs'", "+FIXTURE = HERE.parents[1] / 'review/upscaling_owner_review.rs'",
               "-PRODUCTION = '76be9608e99bf089fcb1e0908827330f70c28ac1'", "+PRODUCTION = '72b1ba8ec351e86d66a90c401e0a75a530633616'",
               "+SOURCE = '825ff047284e84252d00f786a7b14cb6ecb44293'",
               "-PRODUCTION_FREEZE_SHA = '54c13c01c4b1de1c4de42f80ba01b026cf53c3752b4d365af058a4c6048d2361'",
               f"+PRODUCTION_FREEZE_SHA = '{sha(PKT / 'r4_quality/production-freeze.json')}'",
               "-E = Path.home() / '.codex/incant-evidence/0073-owner-output-quality-gpu-001'",
               "+E = Path.home() / '.codex/incant-evidence/0073-r4-quality-gpu-001'",
               "-            and build['source_commit'] == PRODUCTION", "+            and build['source_commit'] == SOURCE"}
    count_old = "            and build['verified_production_files'] == 2835 and build['gpu_executions'] == 0,"
    allowed |= {"-" + count_old, "+" + count_old.replace("2835", "2839")}
    check("runner differs from the 76be execution copy only in source/path/hash metadata", set(delta) == allowed and len(delta) == 13, delta)
    spec = importlib.util.spec_from_file_location("r4_runner", HERE / "runner.py")
    r = importlib.util.module_from_spec(spec); spec.loader.exec_module(r)   # loads the reviewed lifecycle only
    check("lifecycle helper is the reviewed b9e12895 file", sha(r.LIFECYCLE) == r.LIFECYCLE_SHA == "b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34")
    plan_path = HERE / "plan.json"
    plan, q = r.read_plan(plan_path, allow_unbuilt=True)
    groups = {g: q["groups"][g]["budget"] for g in r.ORDER}
    check("five ordered groups, primary first", plan["groups"] == r.ORDER == ["stationary", "motion", "transitions", "c7_disocclusion", "p1_crossing"])
    b = plan["budget"]
    check("exact maximums: 5 children, 380 scenes + 18 UI + 380 readbacks = 778 submissions, 380 maps, 15 PNGs 960x540, 90 s, 0 builds/retries",
          (b["processes"], b["scene_submissions"], b["ui_submissions"], b["readback_submissions"], b["queue_submissions"],
           b["readback_maps"], b["rendered_frames"], b["pngs"], b["maximum_extent"], b["timeout_seconds_each"], b["builds"], b["retries"])
          == (5, 380, 18, 380, 778, 380, 380, 15, [960, 540], 90, 0, 0)
          and b["scene_submissions"] + b["ui_submissions"] + b["readback_submissions"] == 778, groups)
    def refused(fn):
        try:
            fn(); return None
        except r.life.Refused as e:
            return str(e)
    check("unbuilt plan cannot execute (fail closed)", (refused(lambda: r.read_plan(plan_path, allow_unbuilt=False)) or "").endswith("unbuilt preparation cannot execute"))
    check("no grant path without a frozen binary", (refused(lambda: r.grant(plan_path, plan)) or "").endswith("binary not frozen"))
    with tempfile.TemporaryDirectory(prefix="incant-0073-r4-quality-", dir="/tmp") as tmp:
        def mutated(change):
            m = copy.deepcopy(plan); change(m)
            p = pathlib.Path(tmp) / "plan.json"; p.write_text(json.dumps(m))
            return refused(lambda: r.read_plan(p, allow_unbuilt=True))
        check("mutation: claimed frozen state without build/binary receipts refuses", mutated(lambda m: m.update(state="FROZEN_READY_FOR_ROOT_GRANT")) is not None)
        check("mutation: a 16th PNG refuses", mutated(lambda m: m["budget"].update(pngs=16)) is not None)
        check("mutation: the old 76be producer refuses", mutated(lambda m: m.update(producer=OLD)) is not None)
        check("mutation: a retry refuses", mutated(lambda m: m["budget"].update(retries=1)) is not None)
        check("mutation: dropping p1_crossing refuses", mutated(lambda m: m.update(groups=m["groups"][:4])) is not None)
    fz = json.loads((PKT / "r4_quality/production-freeze.json").read_text())
    check("freeze binds candidate 825ff to production-equivalent 72b1 with 2839 files",
          fz["source_commit"] == SOURCE and fz["production_equivalent"] == PRODUCTION and len(fz["files"]) == 2839)
    # API compatibility of the fixture against the candidate (read-only Git objects).
    owner = git("show", f"{SOURCE}:crates/incant_render/src/motion/owner.rs").stdout
    check("candidate keeps MotionFrame::jitter_phase -> Option<u32> (fixture lines 523/568/601)", "pub fn jitter_phase(&self) -> Option<u32> {\n        self.phase" in owner)
    used = ["has_previous", "camera_static", "sequence", "epoch", "same_owner", "bypassed", "internal_size"]
    check("candidate keeps every other MotionFrame accessor the fixture uses", all(f"pub fn {n}(" in owner for n in used))
    unchanged = ["crates/incant_render/src/upscaling/mod.rs", "crates/incant_render/src/resolution", "crates/incant_render/Cargo.toml",
                 "crates/incant_render/tests", "crates/incant_assets", "crates/incant_doc", "crates/incant_text",
                 "crates/incant_render_data", "crates/incant_render_authoring", "crates/incant_types", "Cargo.toml", "Cargo.lock"]
    same = git("diff", "--quiet", OLD, SOURCE, "--", *unchanged).returncode == 0
    check("fixture-facing crates, Cargo files, resolution and upscaling API module unchanged 76be -> 825ff", same, unchanged)
    pubs = [l for l in git("diff", OLD, SOURCE, "--", "crates/incant_render/src").stdout.splitlines()
            if l[:1] in "+-" and l[:3] not in ("+++", "---") and "pub fn" in l and "pub(crate)" not in l]
    check("changed pub fns are unused by the fixture (Prepared::jitter_offset, evidence/kernel test oracles)",
          all(not any(n in fixture.read_text() for n in ("jitter_offset", "historical(", ".resolve(")) for _ in [0]) and len(pubs) == 6, pubs)
    head = git("rev-parse", "HEAD").stdout.strip()
    out = {"scope": "CPU-only preparation self-check; no build, run branch, GPU, capture or image",
           "candidate": SOURCE, "production": PRODUCTION, "checkout_head_observed": head,
           "runner_sha256": sha(HERE / "runner.py"), "plan_sha256": sha(plan_path), "per_group": groups,
           "checks": CHECKS, "all_checks_pass": all(c["ok"] for c in CHECKS)}
    (HERE / "selfcheck.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    print(f"checks={len(CHECKS)} failed={sum(not c['ok'] for c in CHECKS)}")
    for c in CHECKS:
        if not c["ok"]:
            print("FAILED", c)
    return 0 if out["all_checks_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
