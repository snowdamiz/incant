#!/usr/bin/env python3
"""CPU-only, fail-closed analysis, comparison and publication for handoff 0076's
shadow receiver guard comparison. Standard library only; never starts a renderer,
compiler or GPU child. Adapted narrowly from 0072's analyze.py (48a688c5), whose
verdict/binding/publication rules follow the accepted 0073 analyzer; now per role.
Appearance is judged only by personal inspection of the PNGs and the pairs.

  analyze.py check     validate the complete current evidence; always writes
                       analysis/summary-<time>.json; exit 0 only on a pass
  analyze.py compare   only after a current passing check: compare the candidate
                       run with the baseline run (shadowfit.py); always writes
                       analysis/compare-<time>.json; exit 0 only on a pass
  analyze.py pairs     only after a current passing check: write the plan's
                       magnified image pairs (viewing aids, not screenshots) into
                       a new analysis/pairs-<time>/ directory
  analyze.py publish   copy receipts and the plan's 4 PNGs into the packet
                       (create-only), only from the latest summary, which must
                       pass and still bind the current evidence byte for byte

`check` passes only if all of these hold; none is optional, and absence fails:
- the GPU grant and the committed visual tree it names; each role's build record
  is the one the grant binds by SHA-256;
- no stop latch; the build lease is released; the GPU release latch exists,
  recorded no stop, lists no owned or live process or orphan debit, and binds
  the current ledger and debits;
- every ledgered build succeeded and its logs match;
- exactly one passing `list <role>` per role naming exactly the fixture's cases;
- exactly one passing review child per role and plan case, baseline first, and
  no other GPU child kind; one debit per charged child, equal to the plan;
  totals within the ceiling and the PNG budget;
- per case: exit 0, no timeout or interrupt, empty process group, the build
  record's binary before and after; log, report, events and PNG hashes equal
  the files now; the output directory holds exactly report.json, events.jsonl
  and the plan's PNGs; report.json names the case, says pass, lists exactly the
  plan's contract checks, all true, and charged exactly the plan's frames and UI
  submissions; every PNG is 960x540 with the hash the report and ledger name.
Image metrics in report.json are observations; they never decide the verdict.
"""

import json
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def digest(path):
    path = Path(path) if path else None
    return c.sha(path) if path is not None and path.is_file() else None


def png_size(path):
    with open(path, "rb") as source:
        head = source.read(24)
    if len(head) < 24 or head[:8] != PNG_SIGNATURE or head[12:16] != b"IHDR":
        return None
    return list(struct.unpack(">II", head[16:24]))


def bindings(entries, visual_tree):
    """Current hashes of everything a verdict depends on. A summary binds these;
    publication recomputes them, so any later change makes the summary stale."""
    files = {}
    for entry in entries:
        for key in ("stdout", "stderr"):
            if entry.get(key):
                files[entry[key]] = digest(entry[key])
        if entry.get("kind") == "review" and entry.get("output"):
            output = Path(entry["output"])
            files[str(output)] = "directory" if output.is_dir() else None
            if output.is_dir():
                for path in sorted(output.iterdir()):
                    files[str(path)] = digest(path) if path.is_file() else "directory"
    return {"visual_tree": visual_tree, "plan": digest(c.PLAN_PATH), "grant": digest(c.GRANT),
            "build_grant": digest(c.BUILD_GRANT),
            "build_records": {role: digest(c.build_dir(role) / "review.json") for role in c.ROLES},
            "lease": digest(c.LEASE), "ledger": digest(c.LEDGER), "debits": digest(c.BUDGET),
            "latch": digest(c.LATCH), "stop": digest(c.STOP), "files": files}


def validate_review(entry, details, frozen_sha, size):
    problems = []

    def need(condition, message):
        if not condition:
            problems.append(message)

    need(entry.get("failed") == [] and entry.get("exit") == 0, "the child failed or exited nonzero")
    need(not entry.get("timed_out") and not entry.get("wait_error"), "the child timed out or was interrupted")
    need(entry.get("group_empty_after") is True, "the process group was not empty after cleanup")
    need(entry.get("binary_sha256_before") == frozen_sha == entry.get("binary_sha256_after"),
         "the child did not run the build record's binary unchanged")
    for key in ("stdout", "stderr"):
        need(entry.get(key) and digest(entry[key]) == entry.get(key + "_sha256"), f"{key} differs from the ledger")
    output = Path(entry.get("output") or "/nonexistent")
    wanted = {"report.json", "events.jsonl", *details["pngs"]}
    present = {path.name for path in output.iterdir()} if output.is_dir() else set()
    need(present == wanted, f"output holds {sorted(present)}, not {sorted(wanted)}")
    report = output / "report.json"
    body = c.read_json(report) if report.is_file() else {}
    need(body, "report.json is absent or empty")
    need(digest(report) == entry.get("report_sha256"), "report.json differs from the ledger")
    need(digest(output / "events.jsonl") == entry.get("events_sha256"), "events.jsonl differs from the ledger")
    need(body.get("case") == entry.get("case") and body.get("outcome") == "pass", "the report does not pass this case")
    checks = body.get("checks") if isinstance(body.get("checks"), dict) else {}
    need(sorted(checks) == sorted(details["checks"]), f"report checks {sorted(checks)} are not the plan's")
    need(checks and all(value is True for value in checks.values()), "a contract check is false or missing")
    need(body.get("frames_rendered") == details["frames"] and body.get("ui_submissions") == details["ui_submissions"],
         "the report did not charge exactly the plan's frames and UI submissions")
    named = {png.get("name"): png.get("sha256") for png in body.get("pngs", []) if isinstance(png, dict)}
    need(sorted(named) == sorted(details["pngs"]), "the report does not name exactly the plan's PNGs")
    for name in details["pngs"]:
        path = output / name
        need(path.is_file() and png_size(path) == size, f"{name} is absent or not {size[0]}x{size[1]}")
        need(path.is_file() and digest(path) == named.get(name) == entry.get("pngs", {}).get(name),
             f"{name} differs from its report and ledger hashes")
    return problems


def validate(entries, debits, grant):
    p = c.plan()
    spec = p["review"]
    order = spec.get("order") or []
    problems = []

    def need(condition, message):
        if not condition:
            problems.append(message)
        return bool(condition)

    need(order and sorted(order) == sorted(spec["cases"]), "the plan has no cases or its order differs")
    if c.STOP.exists():
        need(False, "stop latched: " + json.dumps(c.read_json(c.STOP))[:300])
    lease = c.read_json(c.LEASE) if c.LEASE.is_file() else {}
    need(lease.get("released_at"), "the CPU build lease is absent or still held")
    latch = c.read_json(c.LATCH) if c.LATCH.is_file() else None
    if need(latch is not None, "the GPU release latch is absent; run gpu.py release first"):
        need(latch.get("owned_processes") == [] and latch.get("ledger_pids_alive") == [],
             "the release latch lists owned or live processes")
        need(latch.get("debits_without_ledger_row") == [], "the release latch lists orphan debits")
        need(latch.get("ledger_sha256") and latch.get("ledger_sha256") == digest(c.LEDGER),
             "the release latch does not bind the current ledger")
        need(latch.get("debits_sha256") and latch.get("debits_sha256") == digest(c.BUDGET),
             "the release latch does not bind the current debits")
        need(latch.get("stop_latched") is False, "the release latch recorded a stop latch")
    frozen = {}
    for role in c.ROLES:
        record = c.build_dir(role) / "review.json"
        body = c.read_json(record) if record.is_file() else {}
        frozen[role] = (body.get("frozen") or {}).get(spec["binary"], {}).get("sha256")
        need(frozen[role], f"the {role} build record or its frozen review binary hash is absent")
        need(digest(record) == (grant.get("build_records") or {}).get(role),
             f"the {role} build record is not the one review-ready.json binds")
        need(body.get("source_commit") == p["sources"][role]["commit"],
             f"the {role} build record does not name the plan's {role} commit")
    need(frozen["baseline"] != frozen["candidate"], "both roles froze the same binary bytes")
    builds = [e for e in entries if e.get("kind") == "build"]
    need(all(e.get("exit") == 0 and not e.get("timed_out") and not e.get("wait_error")
             and e.get("group_empty_after") is True for e in builds), "a ledgered build did not succeed")
    need(all(e.get(key) and digest(e[key]) == e.get(key + "_sha256") for e in builds for key in ("stdout", "stderr")),
         "a build log differs from the ledger")
    gpu = [e for e in entries if e.get("kind") != "build"]
    need(all(e.get("kind") in ("list", "review") for e in gpu), "the ledger holds an unknown child kind")
    for role in c.ROLES:
        rows = [e for e in gpu if e.get("kind") == "list" and e.get("item") == role]
        need(len(rows) == 1 and rows[0].get("failed") == [] and rows[0].get("contract")
             and all(rows[0]["contract"].values()), f"expected one passing {role} listing, found {len(rows)}")
    expected = [f"{role}/{case}" for role in c.ROLES for case in order]
    reviews = [e for e in gpu if e.get("kind") == "review"]
    need([e.get("item") for e in reviews] == expected,
         f"review children {[e.get('item') for e in reviews]} are not {expected} once each in order")
    charged = [e for e in gpu if e.get("debit")]
    need(len(debits) == len(charged) and sorted(d.get("label") for d in debits)
         == sorted(e.get("label") for e in charged), "debits and charged GPU children are not one to one")
    budgets = p["budgets"]
    need(sum(d.get("frames", 0) + d.get("ui_submissions", 0) for d in debits) <= budgets["submissions_max"]
         and sum(d.get("pngs", 0) for d in debits) <= budgets["pngs_max"], "debits exceed the ceiling or PNG budget")
    cases = {}
    for item in expected:
        role, case = item.split("/")
        details = spec["cases"][case]
        mine = [d for d in debits if d.get("item") == item]
        need(len(mine) == 1 and mine[0].get("frames") == details["frames"]
             and mine[0].get("ui_submissions") == details["ui_submissions"]
             and mine[0].get("pngs") == len(details["pngs"]), f"{item}: not exactly one debit of the plan's charge")
        rows = [e for e in reviews if e.get("item") == item]
        if not need(len(rows) == 1, f"{item}: missing or duplicate review child ({len(rows)} rows)"):
            cases[item] = {"present": False}
            continue
        need(rows[0].get("output") == str(c.captures_dir(role) / case), f"{item}: output outside its role directory")
        case_problems = validate_review(rows[0], details, frozen[role], spec["size"])
        cases[item] = {"present": True, "problems": case_problems}
        problems.extend(f"{item}: {message}" for message in case_problems)
    return cases, problems


def check():
    data, _ = c.grant()
    tools = c.tools_identity(data)
    try:
        entries, debits = c.read_lines(c.LEDGER), c.read_lines(c.BUDGET)
    except (OSError, ValueError) as error:
        entries, debits, unreadable = [], [], repr(error)
    else:
        unreadable = None
    try:
        cases, problems = validate(entries, debits, data)
    except Exception as error:  # a malformed record is a failed analysis, never a pass
        cases, problems = {}, [f"analysis error: {error!r}"]
    if unreadable:
        problems.insert(0, "ledger or debits unreadable: " + unreadable)
    recorded = c.now()
    summary = {"recorded": recorded, "verdict": "fail" if problems else "pass",
               "complete": not problems, "problems": problems, "cases": cases,
               "bindings": bindings(entries, tools["visual_tree"]),
               "observations": "report.json observations are reference data; they never decide the verdict"}
    path = c.ANALYSIS / ("summary-" + recorded.replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({"summary": str(path), "verdict": summary["verdict"], "problems": problems[:20]},
                     indent=1))
    return 0 if not problems else 1


def latest_summary():
    summaries = sorted(c.ANALYSIS.glob("summary-*.json")) if c.ANALYSIS.is_dir() else []
    return summaries[-1] if summaries else None


def latest_compare():
    compares = sorted(c.ANALYSIS.glob("compare-*.json")) if c.ANALYSIS.is_dir() else []
    return compares[-1] if compares else None


def current_passing_summary(tools):
    """The latest integrity summary must pass and still bind the current evidence."""
    latest = latest_summary()
    if latest is None:
        c.refuse("no analysis exists; run analyze.py check first")
    summary = c.read_json(latest)
    if summary.get("verdict") != "pass" or summary.get("problems") or summary.get("complete") is not True:
        c.refuse(f"the latest analysis {latest.name} did not pass; publication needs a complete passing analysis")
    if summary.get("bindings") != bindings(c.read_lines(c.LEDGER), tools["visual_tree"]):
        c.refuse(f"the latest analysis {latest.name} is stale: evidence changed since it ran")
    return latest


def compare():
    """Baseline against candidate on the frozen case; thresholds fail honestly."""
    import shadowfit
    data, _ = c.grant()
    tools = c.tools_identity(data)
    latest = current_passing_summary(tools)
    p = c.plan()
    case = p["comparison"]["case"]
    baseline = shadowfit.run_from_captures("baseline", c.EVIDENCE, "baseline", case)
    candidate = shadowfit.run_from_captures("candidate", c.EVIDENCE, "candidate", case)
    try:
        result = shadowfit.compare(baseline, candidate, p)
    except Exception as error:  # a malformed input is a failed comparison, never a pass
        result = {"verdict": "fail", "exact": False, "failed": [f"comparison error: {error!r}"], "checks": []}
    recorded = c.now()
    result.update({"recorded": recorded, "integrity_summary": {"path": latest.name, "sha256": c.sha(latest)},
                   "visual_tree": tools["visual_tree"],
                   "note": "Thresholds are calibrated proposals (plan.json comparison), not plan gates."})
    path = c.ANALYSIS / ("compare-" + recorded.replace(":", "") + ".json")
    c.write_new_json(path, result)
    print(json.dumps({"compare": str(path), "verdict": result["verdict"], "exact": result.get("exact"),
                      "failed": result["failed"][:40]}, indent=1))
    return 0 if result["verdict"] == "pass" else 1


def pairs():
    """The plan's magnified image pairs, for personal inspection only."""
    import pairs as pairing
    data, _ = c.grant()
    tools = c.tools_identity(data)
    latest = current_passing_summary(tools)
    case = c.plan()["comparison"]["case"]
    images = {role: c.captures_dir(role) / case for role in c.ROLES}
    output = c.ANALYSIS / ("pairs-" + c.now().replace(":", ""))
    written = pairing.write_all(images, output, c.plan())
    c.write_new_json(output / "manifest.json", {"integrity_summary": latest.name, "files": written,
                                                "note": "viewing aids derived from the run's PNGs; "
                                                        "not screenshots and not counted against the PNG budget"})
    print(json.dumps({"pairs": str(output), "files": len(written)}, indent=1))
    return 0


def publish():
    data, _ = c.grant()
    tools = c.tools_identity(data)
    latest = current_passing_summary(tools)
    entries = c.read_lines(c.LEDGER)
    if c.PUBLISHED.exists() or c.PUBLISHED_EVIDENCE.exists():
        c.refuse("published evidence already exists; it is never replaced")
    picks = [(c.GRANT, "review-ready.json"), (c.BUILD_GRANT, "build-ready.json"), (c.LEDGER, "gpu-ledger.jsonl"),
             (c.BUDGET, "budget.jsonl"), (c.LATCH, "gpu-released.json"), (c.LEASE, "target-lease.json"),
             (latest, "analysis/" + latest.name)]
    if latest_compare() is not None:  # a failing comparison is published too
        picks.append((latest_compare(), "analysis/" + latest_compare().name))
    for role in c.ROLES:
        for name in ("source.json", "toolchain.json", "review.json", "lint.json"):
            if (c.build_dir(role) / name).exists():
                picks.append((c.build_dir(role) / name, f"build/{role}/{name}"))
    for entry in entries:
        for key in ("stdout", "stderr"):
            picks.append((Path(entry[key]), "logs/" + Path(entry[key]).name))
        if entry["kind"] == "review":
            for name in ("report.json", "events.jsonl"):
                picks.append((Path(entry["output"]) / name, f"captures/{entry['item']}/{name}"))
    manifest = []
    for source, relative in picks:
        raw = source.read_bytes()
        target = c.PUBLISHED_EVIDENCE / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        with open(target, "xb") as out:
            out.write(raw)
        manifest.append({"path": relative, "source": str(source), "sha256": c.sha(target)})
    selection = []
    for entry in entries:
        if entry["kind"] != "review":
            continue
        for name in c.plan()["review"]["cases"][entry["case"]]["pngs"]:
            source = Path(entry["output"]) / name
            target = c.PUBLISHED / f"{entry['role']}-{name}"
            target.parent.mkdir(parents=True, exist_ok=True)
            with open(target, "xb") as out:
                out.write(source.read_bytes())
            if c.sha(target) != entry["pngs"][name]:
                raise SystemExit(f"screenshot copy mismatch for {target}")
            selection.append({"name": target.name, "source": str(source), "sha256": entry["pngs"][name]})
    if len(selection) > c.plan()["budgets"]["pngs_max"]:
        raise SystemExit("more PNGs than the packet allows")
    c.write_new_json(c.PUBLISHED_EVIDENCE / "manifest.json", {
        "published": c.now(), "analysis": {"path": latest.name, "sha256": c.sha(latest)},
        "files": manifest, "screenshots": selection})
    print(json.dumps({"files": len(manifest), "screenshots": len(selection), "analysis": latest.name}, indent=1))
    return 0


def main(argv):
    steps = {"check": check, "compare": compare, "pairs": pairs, "publish": publish}
    if len(argv) != 1 or argv[0] not in steps:
        raise SystemExit(__doc__)
    return steps[argv[0]]()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
