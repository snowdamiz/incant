#!/usr/bin/env python3
"""0073 owner_quality CPU self-check. Standard library, deterministic, read-only.

    python3 -I -B handoffs/0073-upscaling-review/owner_quality/tools/check.py \
        --out handoffs/0073-upscaling-review/owner_quality/preparation/selfcheck.json

No Cargo, renderer, GPU, capture or grant. Production objects are read with
`git ls-tree`, `git grep` and `git cat-file` from the repository and commit named
in plan.json; nothing is written there. Checks:
  1. identity: plan.json pins the fixture SHA-256; the injected test path is new
     at the production commit; pinned production blobs match.
  2. budgets: the fixture's Budget constants equal plan.json, the plan's
     per-group composition recomputes them, and the totals add up.
  3. schedule constants and loop bounds in the fixture match the composition.
  4. PNGs: report.png names per test equal the plan's names and counts.
  5. tests: exactly the planned #[test] functions, each #[ignore]d.
  6. API admission: every production item the fixture relies on is pub at the
     production commit (or present for test modules and fonts).
  7. source identity: every input of the 608ad362 freeze equals its blob at the
     production commit.
  8. static rules: one owner attachment call (inside variant()), one motion-owner
     opt-in, no process/network use, file writes only to the fixture temp root
     or through Report, unchanged historical threshold values, and C7/C9 reported
     as rendered-output observations whose undefined ratios are null, never a pass.
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
PACKET = os.path.dirname(HERE)
CHECKOUT = os.path.abspath(os.path.join(PACKET, "..", "..", ".."))
PLAN = os.path.join(PACKET, "plan.json")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(repo, *args, data=None):
    result = subprocess.run(["git", "-C", repo, *args], input=data, capture_output=True, check=True)
    return result.stdout


def blob(repo, commit, path):
    return git(repo, "show", "{}:{}".format(commit, path))


def functions(source):
    """Map each top-level `fn name(` to its body text (until the next top-level item)."""
    starts = [(m.start(), m.group(1)) for m in re.finditer(r"^fn ([a-z0-9_]+)\(", source, re.M)]
    bodies = {}
    for k, (start, name) in enumerate(starts):
        end = starts[k + 1][0] if k + 1 < len(starts) else len(source)
        bodies[name] = source[start:end]
    return bodies


def check_identity(plan, fixture_bytes, repo, commit):
    pinned = plan["fixture"]["sha256"] == sha(fixture_bytes)
    listed = git(repo, "ls-tree", "--name-only", commit, "--", plan["fixture"]["inject_to"]).strip()
    blobs = {path: sha(blob(repo, commit, path)) for path in plan["production"]["pinned_blobs"]}
    blob_ok = blobs == plan["production"]["pinned_blobs"]
    derived = os.path.join(PACKET, "..", plan["fixture"]["derived_from"]["path"])
    with open(derived, "rb") as fh:
        derived_ok = sha(fh.read()) == plan["fixture"]["derived_from"]["sha256"]
    ok = pinned and listed == b"" and blob_ok and derived_ok
    return ok, {"fixture_sha256_pinned": pinned, "inject_path_new_at_commit": listed == b"",
                "pinned_blobs_match": blob_ok, "derived_from_unchanged": derived_ok}


def check_budgets(plan, source):
    consts = {}
    for name, body in re.findall(r"const ([A-Z0-9_]+): Budget = Budget \{(.*?)\};", source, re.S):
        consts[name] = {k: int(v) for k, v in re.findall(r"(scenes|ui|readbacks|pngs): (\d+)", body)}
    rows, ok = {}, True
    totals = {"scene_submissions": 0, "ui_submissions": 0, "readback_submissions": 0, "readback_maps": 0,
              "queue_submissions": 0, "rendered_frames": 0, "pngs": 0}
    for group, spec in plan["groups"].items():
        budget = spec["budget"]
        fixture = consts.get(spec["budget_const"], {})
        scenes = sum(spec["composition"]["scenes"].values())
        ui = sum(spec["composition"]["ui"].values())
        recomputed = {"scene_submissions": scenes, "ui_submissions": ui, "readback_submissions": scenes,
                      "readback_maps": scenes, "queue_submissions": scenes + ui + scenes,
                      "rendered_frames": scenes, "pngs": len(spec["pngs"])}
        from_fixture = {"scene_submissions": fixture.get("scenes"), "ui_submissions": fixture.get("ui"),
                        "readback_submissions": fixture.get("readbacks"), "readback_maps": fixture.get("readbacks"),
                        "queue_submissions": sum(fixture.get(k, 0) for k in ("scenes", "ui", "readbacks")),
                        "rendered_frames": fixture.get("scenes"), "pngs": fixture.get("pngs")}
        good = budget == recomputed == from_fixture
        ok = ok and good
        rows[group] = {"plan": budget, "recomputed": recomputed, "fixture": from_fixture, "ok": good}
        for key in totals:
            totals[key] += budget[key]
    totals_ok = totals == plan["totals"]["budget"] and plan["totals"]["processes"] == len(plan["groups"])
    return ok and totals_ok, {"groups": rows, "totals": totals, "totals_match_plan": totals_ok}


def check_schedule(plan, source):
    bodies = functions(source)
    schedule = [tuple(int(x) if x.isdigit() else x for x in m)
                for m in re.findall(r"\((\d+), (\d+), (true|false)\)", source.split("const SCHEDULE")[1].split("];")[0])]
    consts = dict(re.findall(r"const (FIXED16_FRAMES|C7_STEPS|P1_STEPS|C7_WARMUP|P1_STOP): u32 = (\d+);", source))
    transitions = plan["groups"]["transitions"]
    found = {
        "schedule": [[s, n, c == "true"] for s, n, c in schedule],
        "schedule_frames": sum(n for _, n, _ in schedule),
        "fixed16_frames": int(consts["FIXED16_FRAMES"]),
        "c7_steps": int(consts["C7_STEPS"]),
        "p1_steps": int(consts["P1_STEPS"]),
        "stationary_loop": bool(re.search(r"for k in 1\.\.=24u32", bodies["stationary"])),
        "stationary_ui_after": bool(re.search(r"if k > 16", bodies["stationary"])),
        "motion_loop": bool(re.search(r"for step in 1\.\.=20u32", bodies["motion"])),
        "motion_controls_at": bool(re.search(r"step == 14 \|\| step == 20", bodies["motion"])),
    }
    comp = {g: plan["groups"][g]["composition"]["scenes"] for g in plan["groups"]}
    expected_schedule = transitions["schedule"]
    ok = (found["schedule"] == expected_schedule
          and found["schedule_frames"] == comp["transitions"]["up"] == comp["transitions"]["own"]
          and found["fixed16_frames"] == comp["transitions"]["fixed16_up"] == comp["transitions"]["fixed16_own"]
          and found["c7_steps"] == comp["c7_disocclusion"]["up_moving"] == comp["c7_disocclusion"]["own_reference"]
          and found["p1_steps"] == comp["p1_crossing"]["up_bar"] == comp["p1_crossing"]["own_reference"]
          and found["stationary_loop"] and found["stationary_ui_after"]
          and comp["stationary"]["up"] == comp["stationary"]["own"] == 24
          and found["motion_loop"] and found["motion_controls_at"]
          and comp["motion"]["up"] == comp["motion"]["own"] == 20
          and [n for s, n, c in found["schedule"] if s == 16] == [4])
    return ok, found


def check_pngs(plan, source):
    bodies = functions(source)
    rows, ok = {}, True
    for group, spec in plan["groups"].items():
        names = re.findall(r'report\.png\("([^"]+)"', bodies.get(group, ""))
        good = names == spec["pngs"]
        ok = ok and good
        rows[group] = {"fixture": names, "ok": good}
    every = [n for spec in plan["groups"].values() for n in spec["pngs"]]
    unique = len(every) == len(set(every))
    return ok and unique, {"groups": rows, "unique_names": unique}


def check_tests(plan, source):
    tests = re.findall(r'#\[test\]\n#\[ignore = "[^"]+"\]\nfn ([a-z0-9_]+)\(\)', source)
    all_tests = re.findall(r"#\[test\]", source)
    ok = tests == plan["run"]["order"] == list(plan["groups"]) and len(all_tests) == len(tests)
    return ok, {"ignored_tests": tests, "test_attributes": len(all_tests)}


API = [
    ("crates/incant_render/src", r"pub fn prepare_retained\("),
    ("crates/incant_render/src", r"pub fn set_dynamic_resolution\("),
    ("crates/incant_render/src", r"pub fn enable_motion_history\("),
    ("crates/incant_render/src", r"pub fn set_motion_owner_attachment\("),
    ("crates/incant_render/src", r"pub fn motion_owner_attachment_enabled\("),
    ("crates/incant_render/src", r"pub fn enable_temporal_upscaling\("),
    ("crates/incant_render/src", r"pub async fn enable_temporal_upscaling\("),
    ("crates/incant_render/src", r"pub async fn enable_motion_owner\("),
    ("crates/incant_render/src", r"pub fn invalidate_motion_history\("),
    ("crates/incant_render/src", r"pub fn upscale_frame\("),
    ("crates/incant_render/src", r"pub fn resolution_frame\("),
    ("crates/incant_render/src", r"pub fn camera_static\("),
    ("crates/incant_render/src", r"pub fn same_owner\("),
    ("crates/incant_render/src", r"pub fn jitter_phase\("),
    ("crates/incant_render/src", r"pub fn has_previous\("),
    ("crates/incant_render/src", r"pub fn bypassed\("),
    ("crates/incant_render/src", r"pub fn output_size\("),
    ("crates/incant_render/src", r"pub fn ui_layer\("),
    ("crates/incant_render/src", r"pub fn headless\("),
    ("crates/incant_render/src", r"pub use owner::UpscaleFrame"),
    ("crates/incant_render/src", r"TemporalConfig as UpscaleConfig"),
    ("crates/incant_render/tests/materials/mod.rs", r"pub fn new\(material: Value\)"),
    ("crates/incant_render/tests/materials/mod.rs", r"pub fn cook\("),
    ("crates/incant_render/tests/materials/mod.rs", r"pub fn environment\("),
    ("crates/incant_render/tests/materials/mod.rs", r"pub fn write_png\("),
    ("crates/incant_render/tests/materials/mod.rs", r"pub fn emissive\("),
    ("crates/incant_render/tests/support/mod.rs", r"pub fn checked\("),
]


def check_api(repo, commit):
    rows, ok = [], True
    for path, pattern in API:
        try:
            hits = git(repo, "grep", "-n", "-E", pattern, commit, "--", path).decode().strip().splitlines()
        except subprocess.CalledProcessError:
            hits = []
        rows.append({"path": path, "pattern": pattern, "hits": len(hits)})
        ok = ok and bool(hits)
    fonts = ["crates/incant_text/tests/fonts/" + f for f in
             ("NotoSans-Regular.ttf", "NotoSansArabic.ttf", "IncantCJKFixture.ttf")]
    present = [git(repo, "ls-tree", "--name-only", commit, "--", f).strip() == f.encode() for f in fonts]
    return ok and all(present), {"items": rows, "fonts_present": all(present)}


def check_freeze(plan, repo, commit):
    with open(plan["production"]["source_freeze"]["path"], "rb") as fh:
        raw = fh.read()
    freeze = json.loads(raw)
    tree = {}
    for line in git(repo, "ls-tree", "-r", commit).decode().splitlines():
        meta, path = line.split("\t", 1)
        tree[path] = meta.split()[2]
    inputs = freeze["inputs"]
    missing = sorted(p for p in inputs if p not in tree)
    order = [p for p in sorted(inputs) if p in tree]
    stream = git(repo, "cat-file", "--batch", data="".join(tree[p] + "\n" for p in order).encode())
    mismatched, offset = [], 0
    for path in order:
        header_end = stream.index(b"\n", offset)
        size = int(stream[offset:header_end].split()[2])
        content = stream[header_end + 1:header_end + 1 + size]
        offset = header_end + 1 + size + 1
        if sha(content) != inputs[path]:
            mismatched.append(path)
    ok = (sha(raw) == plan["production"]["source_freeze"]["sha256"] and not missing and not mismatched
          and len(inputs) == plan["production"]["source_freeze"]["inputs"])
    return ok, {"freeze_sha256": sha(raw), "inputs": len(inputs), "missing_at_commit": missing[:20],
                "mismatched": mismatched[:20], "mismatched_count": len(mismatched)}


def code_only(source):
    """Source with // and //! comments removed (string literals here hold no //)."""
    return "\n".join(line.split("//", 1)[0] if '"' not in line.split("//", 1)[0] or line.count('"') % 2 == 0
                     else line for line in source.splitlines())


def check_static(source):
    bodies = functions(source)
    code = code_only(source)
    writes = re.findall(r"std::fs::(?:write|create_dir)\(\s*([^,)]*)", code, re.S)
    rules = {
        "one_owner_attachment_call": code.count("set_motion_owner_attachment(") == 1
            and "set_motion_owner_attachment(" in bodies["variant"],
        "owner_attachment_only_for_owner_variant": bool(re.search(
            r"if kind == Variant::Owner \{\s*scene\s*\.set_motion_owner_attachment\(renderer, true\)",
            bodies["variant"])),
        "one_motion_owner_opt_in": code.count("enable_motion_owner()") == 1
            and "enable_motion_owner()" in bodies["owner_renderer"],
        "production_startup_order": bodies["owner_renderer"].index("enable_temporal_upscaling()")
            < bodies["owner_renderer"].index("enable_motion_owner()"),
        "scene_order_history_attachment_upscaling": (
            bodies["variant"].index("enable_motion_history(") < bodies["variant"].index("set_motion_owner_attachment(")
            < bodies["variant"].index(".enable_temporal_upscaling(")),
        "review_policy_unchanged": "history_weight: 0.9" in source and "relative_depth: 0.05" in source
            and "absolute_depth: 0.02" in source and "reactive_threshold: 4." in source
            and "reactive_strength: 0.5" in source,
        "no_process_or_network": not re.search(r"std::process|Command::new|TcpStream|UdpSocket|reqwest", code),
        "file_writes_only_temp_root_or_report": bool(writes) and all("root" in w for w in writes),
        "output_env": 'const OUTPUT_ENV: &str = "INCANT_0073_OWNER_REVIEW_OUT";' in source,
        "historical_threshold_values_preserved": "const C7_LIMIT: f64 = 0.06;" in source
            and "const C9_THIRD: f64 = 0.75;" in source and "const C9_FOURTH: f64 = 0.90;" in source
            and "const C9_FIRST_LOW: f64 = 0.90;" in source,
        "historical_thresholds_reported_as_observations": '"historical_threshold_observations"' in code
            and not re.search(r"historical_gates|as_written|as written|c[79]_gate", source, re.I)
            and code.count('"status": "rendered-output observation') == 2,
        "c9_ratios_guarded": bool(re.search(
            r"numerator\.is_finite\(\) && denominator\.is_finite\(\) && denominator > 0\.",
            bodies["contrast_ratio"]))
            and len(re.findall(r"contrast_ratio\(", bodies["c9_observation"])) == 3
            and not re.search(r"/ (reference|last12\[r\])", bodies["c9_observation"])
            and "ratio.map_or(Value::Null," in bodies["meets"]
            and "!regions.is_empty()" in bodies["all_regions"],
        "c7_undefined_is_null": bool(re.search(
            r"let defined = !rows\.is_empty\(\) && !values\.is_empty\(\)", bodies["c7_observation"]))
            and '"worst_at_most_0_06": if defined' in bodies["c7_observation"]
            and '"pass"' not in bodies["c7_observation"],
    }
    return all(rules.values()), {"rules": rules, "file_write_targets": [w.strip() for w in writes]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    with open(PLAN, "rb") as fh:
        plan_bytes = fh.read()
    plan = json.loads(plan_bytes)
    with open(os.path.join(PACKET, plan["fixture"]["packet_path"]), "rb") as fh:
        fixture_bytes = fh.read()
    source = fixture_bytes.decode()
    repo, commit = plan["production"]["repository"], plan["production"]["commit"]
    results = {}
    for name, fn in (("identity", lambda: check_identity(plan, fixture_bytes, repo, commit)),
                     ("budgets", lambda: check_budgets(plan, source)),
                     ("schedule", lambda: check_schedule(plan, source)),
                     ("pngs", lambda: check_pngs(plan, source)),
                     ("tests", lambda: check_tests(plan, source)),
                     ("api", lambda: check_api(repo, commit)),
                     ("freeze", lambda: check_freeze(plan, repo, commit)),
                     ("static", lambda: check_static(source))):
        ok, detail = fn()
        results[name] = {"ok": bool(ok), "detail": detail}
    report = {"model": "0073 owner_quality CPU self-check", "python": sys.version.split()[0],
              "plan_sha256": sha(plan_bytes), "fixture_sha256": sha(fixture_bytes),
              "checks": {k: v["ok"] for k, v in results.items()}, "details": results}
    text = json.dumps(report, indent=1, sort_keys=True)
    if args.out:
        os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(json.dumps(report["checks"], indent=1, sort_keys=True))
    return 0 if all(report["checks"].values()) else 1


if __name__ == "__main__":
    sys.exit(main())
