#!/usr/bin/env python3
"""CPU-only evidence check and publication for handoff 0074 stage B.
Standard library only; never starts a renderer or reads the GPU. Adapted from
handoff 0072's committed visual/tools/analyze.py (visual tree ac8a48de); handoff
0073's corrected analyzer was a read-only reference only.

  analyze.py check     after gpu.py release and lease.py release: validate the
                       complete current evidence, always retain a new diagnostic
                       summary, and exit nonzero unless it passes
  analyze.py publish   validate the same evidence again, require the latest
                       summary to be a pass bound to exactly that evidence, then
                       copy the retained PNGs (hash-verified, never overwritten)

Complete evidence means:
- stop.json is absent, and no build, listing or review child failed;
- the review binary was built from the granted commit with exact freeze coverage
  and admitted features, still has its frozen hash, and listed exactly the
  plan's cases;
- the five plan cases ran exactly once each, in plan order, bound to that binary
  and the current grant, each with every required report check true, the exact
  planned frames, UI submissions and PNG names, and logs, report, events and
  PNGs matching the hashes bound when the child ran;
- the debits are exactly those five runs' planned charges (no orphan debit);
- the GPU latch and the target lease record clean releases bound to the final
  ledger, budget and grant.

Personal inspection of every retained PNG is still required; this file only
proves which pixels were produced and that nothing was lost or replaced.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

BINARY = "metalfx_visual_review"
KINDS = {"build", "list", "review"}


def jsonl(path):
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def completed(row):
    """A recorded child that exited 0 with an empty group and no failure."""
    return (row.get("exit") == 0 and not row.get("timed_out") and not row.get("wait_error")
            and row.get("group_empty_after") is True and not row.get("failed"))


class Evidence:
    """Problems found so far and the SHA-256 of every file a pass depends on."""

    def __init__(self):
        self.problems, self.binding = [], {}

    def fault(self, message):
        self.problems.append(message)

    def bind(self, path):
        path = Path(path)
        if not path.is_file():
            self.fault(f"missing {path}")
            return None
        self.binding[str(path)] = c.sha(path)
        return self.binding[str(path)]

    def load(self, path):
        if self.bind(path) is None:
            return {}
        try:
            value = c.read_json(path)
        except ValueError as error:
            self.fault(f"unreadable {path}: {error}")
            return {}
        return value if isinstance(value, dict) else {}


def review_case(e, row, planned, binary_sha, grant_sha, backend):
    case = row["item"]

    def fault(message):
        e.fault(f"{case}: {message}")

    if not completed(row):
        fault(f"child failed: exit {row.get('exit')}, failed {row.get('failed')}")
    if not binary_sha or not row.get("binary_sha256_before") == binary_sha == row.get("binary_sha256_after"):
        fault("not bound to the frozen review binary")
    if row.get("grant_sha256") != grant_sha:
        fault("ran under another grant")
    charge = {"frames": planned["frames"], "ui_submissions": planned["ui_submissions"],
              "pngs": len(planned["pngs"])}
    debit = row.get("debit") or {}
    if {key: debit.get(key) for key in charge} != charge or debit.get("grant_sha256") != grant_sha:
        fault(f"debit {debit} is not the planned charge {charge}")
    for key in ("stdout", "stderr"):
        if not row.get(key) or e.bind(row[key]) != row.get(key + "_sha256"):
            fault(f"{key} log missing or changed")
    output = Path(row.get("output") or "/nonexistent")
    report = e.load(output / "report.json")
    if e.binding.get(str(output / "report.json")) != row.get("report_sha256"):
        fault("report.json missing or changed since its run")
    if e.bind(output / "events.jsonl") != row.get("events_sha256"):
        fault("events.jsonl missing or changed since its run")
    checks = report.get("checks") if isinstance(report.get("checks"), dict) else {}
    untrue = sorted(name for name, value in checks.items() if value is not True)
    if sorted(checks) != sorted(planned["checks"]) or untrue:
        fault(f"required checks are not exactly the plan's, all true (not true: {untrue})")
    if report.get("case") != case or report.get("outcome") != "pass":
        fault(f"report outcome {report.get('outcome')!r} is not this case's pass")
    if (report.get("frames_rendered"), report.get("ui_submissions")) != (planned["frames"],
                                                                         planned["ui_submissions"]):
        fault("rendered frames or UI submissions differ from the plan")
    adapter = (report.get("renderer") or {}).get("adapter") or {}
    if adapter.get("backend") != backend:
        fault(f"adapter backend {adapter.get('backend')!r} is not {backend}")
    pngs = row.get("pngs") or {}
    reported = {x.get("name"): x.get("sha256") for x in report.get("pngs") or [] if isinstance(x, dict)}
    on_disk = sorted(x.name for x in output.glob("*.png")) if output.is_dir() else []
    if not sorted(pngs) == sorted(reported) == on_disk == sorted(planned["pngs"]):
        fault("PNG names differ from the plan")
    for name, digest in sorted(pngs.items()):
        if e.bind(output / name) != digest or reported.get(name) != digest:
            fault(f"{name} changed since its run")
    return {"label": row.get("label"), "outcome": report.get("outcome"), "checks": checks,
            "frames_rendered": report.get("frames_rendered"),
            "ui_submissions": report.get("ui_submissions"), "pngs": pngs, "adapter": adapter,
            "observations": report.get("observations")}


def validate(data, grant_sha):
    """(problems, binding, cases) for the evidence as it is now. Evidence faults
    become problems, never exceptions, so check can always retain a summary."""
    p, e = c.plan(), Evidence()
    planned = p["review"]["cases"]
    try:
        ledger, post, budget = jsonl(c.LEDGER), jsonl(c.POST_RELEASE_LEDGER), c.budget_rows()
    except ValueError as error:
        e.fault(f"unreadable ledger or budget: {error}")
        ledger, post, budget = [], [], []
    if c.STOP.exists():
        e.fault("stop.json is latched: " + c.STOP.read_text()[:200])
    for row in ledger + post:
        if not completed(row):
            e.fault(f"child {row.get('label')} failed: exit {row.get('exit')}, failed {row.get('failed')}")
    if {row.get("kind") for row in ledger} - KINDS:
        e.fault("unexpected child kinds in the ledger")
    for failed in sorted(c.BUILD.glob("*.failed.json")) if c.BUILD.is_dir() else []:
        e.fault(f"failed build record {failed.name}")

    source = e.load(c.BUILD / "source.json")
    if source.get("source_commit") != data["source_commit"] or (source.get("coverage") or {}).get("exact") is not True:
        e.fault("the review source is not the granted commit with exact freeze coverage")
    e.bind(c.BUILD / "checkout.json")
    review = e.load(c.BUILD / "review.json")
    frozen = (review.get("frozen") or {}).get(BINARY) or {}
    if (review.get("library_features_admitted") is not True or not frozen.get("path")
            or e.bind(frozen["path"]) != frozen.get("sha256")):
        e.fault("the frozen review binary is missing, changed or built with other features")

    listings = [row for row in ledger if row.get("kind") == "list"]
    if (len(listings) != 1 or listings[0].get("item") != "review"
            or sorted(listings[0].get("listed") or []) != sorted(planned)):
        e.fault("the review binary was not listed exactly once with exactly the plan's cases")
    reviews = [row for row in ledger if row.get("kind") == "review"]
    if [row.get("item") for row in reviews] != list(planned):
        e.fault(f"review cases ran as {[row.get('item') for row in reviews]}, "
                f"not {list(planned)} once each in order")
    cases = {row["item"]: review_case(e, row, planned[row["item"]], frozen.get("sha256"), grant_sha,
                                      p["expected_backend"])
             for row in reviews if row.get("item") in planned}

    if budget != [row.get("debit") for row in reviews]:
        e.fault("the debits are not exactly the review runs' debits (orphan, missing or changed)")
    totals = p["totals"]
    if c.budget_used() != {key: totals[key] for key in ("frames", "ui_submissions", "pngs")}:
        e.fault(f"charged {c.budget_used()}, not the planned totals")

    latch = e.load(c.LATCH)
    if (latch.get("owned_processes") != [] or latch.get("ledger_pids_alive") != []
            or latch.get("stopped") is not None
            or latch.get("ledger_sha256") != (c.sha(c.LEDGER) if c.LEDGER.exists() else None)
            or latch.get("budget_sha256") != (c.sha(c.BUDGET) if c.BUDGET.exists() else None)
            or latch.get("grant_sha256_at_release") != grant_sha):
        e.fault("the GPU release is not clean or not bound to the final ledger, budget and grant")
    lease = e.load(c.LEASE)
    if (not lease.get("released_at") or lease.get("processes_on_target_at_release") != []
            or lease.get("grant_sha256") != grant_sha):
        e.fault("the target lease is not cleanly released under this grant")
    for path in (c.LEDGER, c.BUDGET, *([c.POST_RELEASE_LEDGER] if c.POST_RELEASE_LEDGER.exists() else [])):
        e.bind(path)
    return e.problems, e.binding, cases


def check():
    data, grant_sha = c.grant()
    problems, binding, cases = validate(data, grant_sha)
    recorded = c.now()
    summary = {"recorded": recorded, "grant_sha256": grant_sha, "pass": not problems,
               "problems": problems, "binding": binding, "cases": cases,
               "budget": c.budget_used(), "stop_latched": c.STOP.exists()}
    path = c.EVIDENCE / "analysis" / ("analysis-" + recorded.replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({"pass": summary["pass"], "problems": problems[:20], "summary": str(path)},
                     indent=1))
    if problems:
        raise SystemExit(f"evidence incomplete or failed ({len(problems)} problems); "
                         f"summary retained at {path}")


def latest_summary():
    folder = c.EVIDENCE / "analysis"
    found = sorted(folder.glob("analysis-*.json")) if folder.is_dir() else []
    return found[-1] if found else None


def publish():
    data, grant_sha = c.grant()
    problems, binding, _ = validate(data, grant_sha)
    if problems:
        c.refuse("the current evidence is incomplete or failed: " + "; ".join(problems[:3]))
    latest = latest_summary()
    summary = c.read_json(latest) if latest else {}
    if not (summary.get("pass") is True and summary.get("grant_sha256") == grant_sha
            and summary.get("binding") == binding):
        c.refuse("no fresh passing summary bound to the current evidence; run analyze.py check")
    if c.PUBLISHED.exists():
        c.refuse(f"{c.PUBLISHED} exists; publication is never repeated")
    c.PUBLISHED.mkdir(parents=True)
    manifest = []
    for row in (r for r in jsonl(c.LEDGER) if r.get("kind") == "review"):
        output = Path(row["output"])
        for name, digest in sorted(row["pngs"].items()):
            target = c.PUBLISHED / name
            with open(target, "xb") as out:
                out.write((output / name).read_bytes())
            if c.sha(target) != digest:
                c.refuse(f"copy mismatch for {target}")
            manifest.append({"case": row["item"], "name": name, "sha256": digest,
                             "source": str(output / name)})
    c.write_new_json(c.PUBLISHED / "manifest.json", {
        "grant_sha256": grant_sha, "visual_tree": data["visual_tree"],
        "analysis": {"path": str(latest), "sha256": c.sha(latest)}, "pngs": manifest,
        "recorded": c.now()})
    print(json.dumps(manifest, indent=1))


if __name__ == "__main__":
    steps = {"check": check, "publish": publish}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
