#!/usr/bin/env python3
"""Full-frame stationary coverage for handoff 0076, under root's
~/.codex/incant-evidence/0076-shadow-fullframe-001/root-grant.json. Standard
library only. Adapted narrowly from determinism/run.py (93ab2d85): the same grant,
hash, debit, lock, supervise/reap and release machinery over the frozen
visual/tools primitives, for the fixture's existing `stationary` case run once per
role. The case's frame count, PNG names and contract checks are read from the
committed fixture source and must equal 0072's committed plan. Not a new
framework; the determinism wrapper and its namespace are left untouched.

common.py's paths name the released 0076-shadow-guard-review-001 namespace. Every
path this wrapper writes is rebound to the new namespace below, so nothing is
written to 001: its frozen binaries, build records, exported checkouts and
original captures are only read and hash-checked.

  python3 -I -B run.py validate             read-only preflight; launches nothing
  python3 -I -B run.py run <label>          one child, in grant order:
                                            baseline-stationary, candidate-stationary
  python3 -I -B run.py release              after the terminal child: GPU latch
  python3 -I -B run.py summary <new path>   CPU only, after release: frame and PNG
                                            hashes, baseline against candidate
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve()
sys.path.insert(0, str(HERE.parents[1] / "visual" / "tools"))
import analyze  # noqa: E402  (png_size only)
import common as c  # noqa: E402
import gpu  # noqa: E402  (libtest_summary only)

ORIGINAL = c.EVIDENCE  # 0076-shadow-guard-review-001: read only
NEW = c.EVIDENCE_ROOT / "0076-shadow-fullframe-001"
GRANT = NEW / "root-grant.json"
FORMAT = "incant-root-bounded-gpu-diagnostic-v1"
RUNS = {"baseline-stationary": "baseline", "candidate-stationary": "candidate"}
CASE = "stationary"
ARGS = [CASE, "--ignored", "--exact", "--nocapture", "--test-threads=1"]
FRAMES = ["off", *(f"on-{k:02}" for k in range(1, 17))]
PLAN_0072 = "48a688c58f15ce2951292471c5d37cd2cc1fdc9b:handoffs/0072-motion-history-numeric/visual/plan.json"
ALIVE = ("owned_processes", "owned_processes_alive", "ledger_pids_alive", "target_users",
         "processes_on_target_at_release")

for _name, _path in {"EVIDENCE": NEW, "GRANT": GRANT, "BUILD_GRANT": NEW / "no-build-grant",
                     "REFUSALS": NEW / "refusals.jsonl", "PACKET_LOCK": NEW / "packet.lock",
                     "LEDGER": NEW / "ledger.jsonl", "BUDGET": NEW / "budget.jsonl", "STOP": NEW / "stop.json",
                     "LATCH": NEW / "released.json", "LOGS": NEW / "logs", "SCRATCH": NEW / "tmp",
                     "CAPTURES": NEW / "runs", "ANALYSIS": NEW / "analysis"}.items():
    setattr(c, _name, _path)  # common.CHECKOUT, BUILD and BIN keep naming 001, read only


def case_spec():
    """The stationary case as the committed fixture declares it: Report::open
    limits, PNG names and contract checks, which must equal 0072's committed plan."""
    source = c.HELPER_SOURCE.read_text()
    body = re.search(r"\nfn " + CASE + r"\((.*?)\n\}", source, re.S)[1]
    frames, ui = (int(v) for v in re.search(r'Report::open\("' + CASE + r'", (\d+), (\d+)\)', body).groups())
    checks = re.findall(r'\(\s*"([a-z0-9_]+)",', re.search(r"let checks = \[(.*?)\n    \];", body, re.S)[1])
    spec = {"frames": frames, "ui_submissions": ui, "pngs": re.findall(r'report\.png\("([\w-]+\.png)"', body),
            "checks": checks}
    planned = json.loads(c.git("show", PLAN_0072))["review"]["cases"][CASE]
    if any(spec[k] != planned[k] for k in spec):
        c.refuse(f"the fixture's {CASE} case differs from 0072's committed plan: {spec}")
    return spec


def grant():
    if not GRANT.is_file():
        raise SystemExit("refused: root-grant.json is absent")
    g = c.read_json(GRANT)
    case = case_spec()
    if not (g.get("format") == FORMAT and g.get("packet") == c.PACKET and c.holder_ok(g.get("holder"))
            and g.get("runs") == list(RUNS) and g.get("case") == CASE and g.get("args") == ARGS
            and g.get("submissions_per_run") == case["frames"] == 17 and case["ui_submissions"] == 0
            and len(case["pngs"]) == 3 and g.get("submissions_total_max") == 34 and g.get("png_total_max") == 6
            and g.get("max_size") == [960, 540] and g.get("timeout_seconds_per_child") == 120
            and g.get("automatic_retries") == 0 and g.get("source_or_fixture_edits_allowed") is False
            and sorted(g.get("build_records") or {}) == ["baseline", "candidate"]):
        c.refuse("root-grant.json does not name this packet with the two stationary runs, their arguments "
                 "and the 17 / 34 / 6 PNG / 960x540 / 120 s / no-retry limits")
    return g, c.sha(GRANT)


def inputs(g):
    """Every frozen input, read only: prior release, visual tree, this wrapper,
    both build records and binaries, and the fixture they were built from."""
    prior = Path(g["prior_release"]["path"])
    body = c.read_json(prior) if prior.is_file() else {}
    alive = {k: body[k] for k in ALIVE if k in body}
    if c.sha(prior) != g["prior_release"]["sha256"] or not alive or any(v != [] for v in alive.values()):
        c.refuse("the prior release does not match its pin or lists a live process")
    tools = c.tools_identity({"visual_tree": g["frozen_visual_tree"]})
    wrapper = str(HERE.relative_to(c.ROOT))
    if c.git("status", "--porcelain", "--", wrapper).strip() or not c.git("ls-files", "--", wrapper).strip():
        c.refuse("run.py is not committed unchanged; the reviewed commit is what runs")
    binaries = {}
    for role, pin in g["build_records"].items():
        record = Path(pin["path"])
        body = c.read_json(record)
        frozen = body["frozen"]["temporal_visual_review"]
        binary = Path(pin["binary"]["path"])
        same = {k: frozen.get(k) == pin["binary"].get(k)
                for k in ("path", "sha256", "bytes", "mode", "features", "package_id", "built_from", "fresh")}
        if (record != ORIGINAL / "build" / role / "review.json" or c.sha(record) != pin["sha256"]
                or not all(same.values()) or body["source_commit"] != c.plan()["sources"][role]["commit"]
                or body["helper_sha256"] != c.sha(c.HELPER_SOURCE) or binary.stat().st_size != pin["binary"]["bytes"]
                or oct(binary.stat().st_mode & 0o777) != pin["binary"]["mode"] or c.sha(binary) != pin["binary"]["sha256"]):
            c.refuse(f"the {role} build record, frozen binary or fixture does not match the grant")
        c.require_executable(binary)
        binaries[role] = {"path": str(binary), "sha256": pin["binary"]["sha256"]}
    return {"prior_release_alive": alive, "tools": tools, "wrapper": {"path": wrapper, "sha256": c.sha(HERE)},
            "binaries": binaries}


def validate():
    g, grant_sha = grant()
    found = inputs(g)
    checkouts = {role: c.verify_checkout(role) for role in ("baseline", "candidate")}
    print(json.dumps({"grant_sha256": grant_sha, **found, "checkouts": checkouts,
                      "ledger": [r["label"] for r in c.read_lines(c.LEDGER)],
                      "stop": c.STOP.exists(), "released": c.LATCH.exists()}, indent=1))


def contract(stdout, output, binary_after, expected_sha):
    """The child's receipts: libtest, report, PNGs and binary identity."""
    case = case_spec()
    summary = gpu.libtest_summary(stdout)
    present = sorted(p.name for p in output.iterdir()) if output.is_dir() else []
    report = c.read_json(output / "report.json") if (output / "report.json").is_file() else {}
    named = {p.get("name"): p for p in report.get("pngs", [])}
    pngs = {n: {"sha256": c.sha(output / n), "size": analyze.png_size(output / n)}
            for n in case["pngs"] if (output / n).is_file()}
    checks = report.get("checks") or {}
    result = {
        "libtest_one_passed_none_failed": summary["running"] == [1] and len(summary["results"]) == 1
        and summary["results"][0]["passed"] == 1 and summary["results"][0]["failed"] == 0 and not summary["failures"],
        "outputs_exact": present == sorted(["report.json", "events.jsonl", *case["pngs"]]),
        "report_pass": report.get("case") == CASE and report.get("outcome") == "pass",
        "checks_exact_and_true": sorted(checks) == sorted(case["checks"]) and all(v is True for v in checks.values()),
        "frames_17_ui_0": report.get("frames_rendered") == 17 and report.get("ui_submissions") == 0
        and [f.get("name") for f in report.get("frames", [])] == FRAMES,
        "pngs_3_at_960x540": sorted(pngs) == sorted(case["pngs"]) and len(pngs) == 3
        and all(p["size"] == [960, 540] for p in pngs.values())
        and all(named.get(n, {}).get("sha256") == p["sha256"] for n, p in pngs.items()),
        "binary_unchanged": binary_after == expected_sha,
    }
    return result, summary, pngs, report


def run(label):
    g, grant_sha = grant()
    if label not in RUNS:
        c.refuse(f"unknown run {label}")
    found = inputs(g)
    role = RUNS[label]
    binary, expected = Path(found["binaries"][role]["path"]), found["binaries"][role]["sha256"]
    output = c.CAPTURES / label / CASE
    stdout_path, stderr_path = c.LOGS / f"{label}.stdout.txt", c.LOGS / f"{label}.stderr.txt"
    with c.packet_lock(), c.gpu_lock(), c.deferred_signals():
        if c.LATCH.exists():
            c.refuse("already released; nothing launches after release")
        if c.STOP.exists():
            c.refuse("stop.json latched a failure; release next")
        done = [r["label"] for r in c.read_lines(c.LEDGER)]
        if label in done or len(done) >= len(RUNS) or list(RUNS)[len(done)] != label:
            c.refuse(f"{label} is not next in grant order after {done}; no retry")
        if output.exists() or stdout_path.exists() or stderr_path.exists():
            c.refuse(f"{label} outputs exist; they are never overwritten")
        debits = c.read_lines(c.BUDGET)
        frames = sum(d["frames"] for d in debits)
        pngs = sum(d["pngs"] for d in debits)
        if frames + g["submissions_per_run"] > g["submissions_total_max"] or pngs + 3 > g["png_total_max"]:
            c.refuse(f"ceiling: {frames} submissions and {pngs} PNGs already debited")
        source = c.verify_checkout(role)
        if c.sha(binary) != expected:
            c.refuse(f"{binary} does not match its frozen hash")
        c.LOGS.mkdir(parents=True, exist_ok=True)
        output.parent.mkdir(parents=True, exist_ok=True)
        overrides = {"PATH": c.SYSTEM_PATH, "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)), "RUST_BACKTRACE": "1",
                     "INCANT_EXPECT_GPU_BACKEND": c.plan()["expected_backend"], "INCANT_0072_REVIEW_OUT": str(output)}
        debit = {"label": label, "role": role, "frames": g["submissions_per_run"], "ui_submissions": 0, "pngs": 3,
                 "grant_sha256": grant_sha, "at": c.now()}
        c.append_durable(c.BUDGET, debit)  # fsynced before launch, never refunded
        argv = [str(binary), *g["args"]]
        cwd = ORIGINAL / "checkout" / role / "crates" / "incant_render"
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            pid, code, timed_out, original, group_empty = c.supervise(
                argv, cwd, c.child_env(overrides), out, err, g["timeout_seconds_per_child"])
        stdout = stdout_path.read_text(errors="replace")
        checks, libtest, png_hashes, report = contract(stdout, output, c.sha(binary), expected)
        entry = {"label": label, "role": role, "holder": g["holder"], "grant_sha256": grant_sha, "inputs": found,
                 "checkout": source, "argv": argv, "cwd": str(cwd), "env": c.env_receipt(overrides), "debit": debit,
                 "pid": pid, "started": started, "ended": c.now(), "exit": code, "timed_out": timed_out,
                 "wait_error": type(original[1]).__name__ if original else None, "group_empty_after": group_empty,
                 "timeout_s": g["timeout_seconds_per_child"], "output": str(output),
                 "stdout": str(stdout_path), "stdout_sha256": c.sha(stdout_path),
                 "stderr": str(stderr_path), "stderr_sha256": c.sha(stderr_path), "libtest": libtest,
                 "report_sha256": c.sha(output / "report.json") if (output / "report.json").is_file() else None,
                 "events_sha256": c.sha(output / "events.jsonl") if (output / "events.jsonl").is_file() else None,
                 "pngs": png_hashes, "frames": {f["name"]: f.get("rgba_sha256") for f in report.get("frames", [])},
                 "contract": checks}
        failed = [k for k, v in checks.items() if not v]
        failed += [f"exit {code}"] if code != 0 else []
        failed += [k for k in ("timed_out", "wait_error") if entry[k]]
        failed += [] if group_empty else ["group not empty"]
        entry["failed"] = failed
        c.append_durable(c.LEDGER, entry)
        if failed:
            c.latch_stop(label, ", ".join(failed))
        if original:
            raise original[1].with_traceback(original[2])
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "failed", "contract", "libtest")},
                     indent=1))
    if failed:
        raise SystemExit(f"{label} failed and latched stop.json: {failed}")


def release():
    g, grant_sha = grant()
    prefixes = (str(ORIGINAL / "bin"), str(ORIGINAL / "checkout"))
    with c.packet_lock():
        if c.LATCH.exists():
            raise SystemExit("already released")
        listing = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
        owned = [line.strip() for line in listing.splitlines()
                 if len(line.split(None, 1)) == 2 and line.split(None, 1)[1].startswith(prefixes)]
        if owned:
            c.refuse("owned processes still running: " + json.dumps(owned)[:400])
        rows = c.read_lines(c.LEDGER)
        alive = [r["pid"] for r in rows if subprocess.run(["ps", "-p", str(r["pid"]), "-o", "args="],
                                                          capture_output=True, text=True).stdout.strip().startswith(prefixes)]
        if alive:
            c.refuse(f"ledger PIDs still alive: {alive}")
        checkouts = {}
        for role in ("baseline", "candidate"):
            try:
                checkouts[role] = c.verify_checkout(role)
            except SystemExit as refused:  # recorded, never blocks the release
                checkouts[role] = {"refused": str(refused)}
        debits = c.read_lines(c.BUDGET)
        record = {"packet": c.PACKET, "holder": g["holder"], "grant_sha256": grant_sha, "released_at": c.now(),
                  "children": len(rows), "runs": [r["label"] for r in rows],
                  "failed": {r["label"]: r["failed"] for r in rows if r["failed"]},
                  "stop_latched": c.STOP.exists(), "stopped": c.read_json(c.STOP) if c.STOP.exists() else None,
                  "debited": {"submissions": sum(d["frames"] + d["ui_submissions"] for d in debits),
                              "pngs": sum(d["pngs"] for d in debits)},
                  "debits_without_ledger_row": sorted({d["label"] for d in debits} - {r["label"] for r in rows}),
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "debits_sha256": c.sha(c.BUDGET) if c.BUDGET.exists() else None,
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "original_checkouts_after": checkouts,
                  "note": "GPU released for root; nothing was written to 0076-shadow-guard-review-001."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def summary(path):
    """Deterministic comparison of the two runs: sorted keys, no times."""
    if not c.LATCH.is_file():
        raise SystemExit("refused: release first; the summary reads finished runs only")
    rows = {r["label"]: r for r in c.read_lines(c.LEDGER)}
    reports = {label: Path(rows[label]["output"]) / "report.json"
               for label in RUNS if label in rows and not rows[label]["failed"]}
    loaded = {k: c.read_json(p) for k, p in reports.items()}
    frames = {k: {f["name"]: f["rgba_sha256"] for f in body["frames"]} for k, body in loaded.items()}
    pngs = {k: {p["name"]: p["rgba_sha256"] for p in body["pngs"]} for k, body in loaded.items()}
    complete = sorted(loaded) == sorted(RUNS)
    same = {n: frames["baseline-stationary"][n] == frames["candidate-stationary"][n] for n in FRAMES} if complete else {}
    result = {"inputs": {k: c.sha(p) for k, p in sorted(reports.items())},
              "grant_sha256": c.sha(GRANT), "ledger_sha256": c.sha(c.LEDGER), "released_sha256": c.sha(c.LATCH),
              "runs_compared": sorted(loaded), "missing_runs": sorted(set(RUNS) - set(loaded)), "frames": FRAMES,
              "frames_identical_across_binaries": [n for n in FRAMES if same.get(n)],
              "frames_differing_across_binaries": [n for n in FRAMES if n in same and not same[n]],
              "pngs_identical_across_binaries": {n: pngs["baseline-stationary"][n] == pngs["candidate-stationary"][n]
                                                 for n in sorted(pngs.get("baseline-stationary", {}))} if complete else {},
              "frame_hashes": frames, "png_rgba_hashes": pngs,
              "notes": ["One run per binary: a differing frame here cannot be separated from same-binary "
                        "variation, which 0076 demonstrated once in 144 temporal-on frames; it is reported "
                        "as observed, never passed by a modal rule."]}
    with open(path, "x") as out:
        json.dump(result, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({"summary": str(path), "differing": result["frames_differing_across_binaries"],
                      "pngs_identical": result["pngs_identical_across_binaries"]}, indent=1))


if __name__ == "__main__":
    command, rest = (sys.argv[1], sys.argv[2:]) if len(sys.argv) > 1 else ("", [])
    if command == "validate" and not rest:
        validate()
    elif command == "run" and len(rest) == 1:
        run(rest[0])
    elif command == "release" and not rest:
        release()
    elif command == "summary" and len(rest) == 1:
        summary(rest[0])
    else:
        raise SystemExit(__doc__)
