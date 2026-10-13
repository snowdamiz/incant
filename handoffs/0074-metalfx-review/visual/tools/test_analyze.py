#!/usr/bin/env python3
"""CPU-only controls for analyze.py (handoff 0074 stage B). A complete synthetic
evidence set that matches the plan must check and publish. Empty evidence, a
missing case, a false report check (a native-step portable fallback included),
a failed child, a changed PNG or report, an orphan debit, and a stale or failed
summary must each refuse; check always retains its diagnostic summary. Never
builds, runs a renderer or touches the real evidence directory: every evidence
path is redirected to a temporary directory.

  python3 -I -B test_analyze.py [--receipt NEW_PATH]
"""

import contextlib
import io
import json
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import common as c  # noqa: E402

RECORDS = []
GRANT = {"packet": c.PACKET, "stage": c.STAGE, "source_commit": "a" * 40, "source_tree": "c" * 40,
         "visual_tree": "b" * 40, "source_freeze": {"path": "/Users/andreyyurlov/.codex/freeze.json",
                                                    "sha256": "d" * 64},
         "features": ["native-metalfx"], "target": "/Users/andreyyurlov/.codex/analyze-test-target",
         "holder": "root supervised build"}
PATHS = {"EVIDENCE": "evidence", "GRANT": "evidence/review-ready.json", "LEDGER": "evidence/gpu-ledger.jsonl",
         "BUDGET": "evidence/budget.jsonl", "POST_RELEASE_LEDGER": "evidence/post-release-ledger.jsonl",
         "STOP": "evidence/stop.json", "LATCH": "evidence/gpu-released.json",
         "LEASE": "evidence/target-lease.json", "REFUSALS": "evidence/refusals.jsonl",
         "BUILD": "evidence/build", "LOGS": "evidence/logs", "CAPTURES": "evidence/captures",
         "PUBLISHED": "published"}


def rows(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def rewrite(path, values):
    path.write_text("".join(json.dumps(value) + "\n" for value in values))


class Analyze(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="incant-0074-analyze-")
        root = Path(self.directory.name)
        self.patches = contextlib.ExitStack()
        self.patches.enter_context(mock.patch.multiple(c, **{k: root / v for k, v in PATHS.items()}))
        self.patches.enter_context(contextlib.redirect_stdout(io.StringIO()))
        c.EVIDENCE.mkdir()
        c.write_new_json(c.GRANT, GRANT)
        self.grant_sha = c.sha(c.GRANT)
        self.plan = c.plan()

    def tearDown(self):
        self.patches.close()
        self.directory.cleanup()

    def complete(self, false_check=None, failed_case=None):
        """Synthetic evidence exactly as the runners record it for the plan."""
        binary = c.BUILD / "bin" / "metalfx_visual_review"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"frozen review binary")
        binary_sha = c.sha(binary)
        c.write_new_json(c.BUILD / "checkout.json", {"commit": GRANT["source_commit"]})
        c.write_new_json(c.BUILD / "source.json", {"source_commit": GRANT["source_commit"],
                                                   "coverage": {"exact": True}})
        c.write_new_json(c.BUILD / "review.json", {"library_features_admitted": True, "frozen": {
            "metalfx_visual_review": {"path": str(binary), "sha256": binary_sha}}})
        clean = {"exit": 0, "timed_out": False, "wait_error": None, "group_empty_after": True}
        c.append_durable(c.LEDGER, {"label": "review", "kind": "build", "pid": 1, **clean})
        cases = self.plan["review"]["cases"]
        c.append_durable(c.LEDGER, {"label": "list-review", "kind": "list", "item": "review", "pid": 2,
                                    "listed": list(cases), "failed": [], **clean})
        c.LOGS.mkdir(parents=True)
        for index, (case, planned) in enumerate(cases.items()):
            label, output = f"review-{case}", c.CAPTURES / case
            output.mkdir(parents=True)
            pngs = {}
            for name in planned["pngs"]:
                (output / name).write_bytes(f"{case} {name}".encode())
                pngs[name] = c.sha(output / name)
            checks = {name: name != false_check for name in planned["checks"]}
            c.write_new_json(output / "report.json", {
                "case": case, "outcome": "pass" if all(checks.values()) else "fail", "checks": checks,
                "frames_rendered": planned["frames"], "ui_submissions": planned["ui_submissions"],
                "renderer": {"adapter": {"name": "Apple M5 Pro", "backend": "Metal"}},
                "pngs": [{"name": name, "sha256": digest} for name, digest in pngs.items()]})
            (output / "events.jsonl").write_text('{"event": "open"}\n')
            stdout, stderr = c.LOGS / f"gpu-{label}.stdout.txt", c.LOGS / f"gpu-{label}.stderr.txt"
            stdout.write_text("test result: ok. 1 passed\n")
            stderr.write_text("")
            debit = {"label": label, "item": case, "frames": planned["frames"],
                     "ui_submissions": planned["ui_submissions"], "pngs": len(planned["pngs"]),
                     "grant_sha256": self.grant_sha, "at": f"2026-10-13T00:00:0{index}+00:00"}
            c.append_durable(c.BUDGET, debit)
            failed = ["exit 101"] if case == failed_case else []
            c.append_durable(c.LEDGER, {
                "label": label, "kind": "review", "item": case, "pid": 10 + index, **clean,
                "exit": 101 if failed else 0, "failed": failed, "binary_sha256_before": binary_sha,
                "binary_sha256_after": binary_sha, "grant_sha256": self.grant_sha, "debit": debit,
                "stdout": str(stdout), "stdout_sha256": c.sha(stdout), "stderr": str(stderr),
                "stderr_sha256": c.sha(stderr), "output": str(output),
                "report_sha256": c.sha(output / "report.json"),
                "events_sha256": c.sha(output / "events.jsonl"), "pngs": pngs})
        self.release()
        c.write_new_json(c.LEASE, {"grant_sha256": self.grant_sha, "released_at": "2026-10-13T00:01:00+00:00",
                                   "processes_on_target_at_release": []})

    def release(self):
        """A clean GPU latch bound to the ledger and budget as they are now."""
        c.LATCH.unlink(missing_ok=True)
        c.write_new_json(c.LATCH, {"owned_processes": [], "ledger_pids_alive": [], "stopped": None,
                                   "ledger_sha256": c.sha(c.LEDGER), "budget_sha256": c.sha(c.BUDGET),
                                   "grant_sha256_at_release": self.grant_sha})

    def check_refused(self):
        """check exits nonzero, yet retains a failing diagnostic summary."""
        with self.assertRaises(SystemExit):
            analyze.check()
        summary = c.read_json(analyze.latest_summary())
        self.assertFalse(summary["pass"])
        self.assertTrue(summary["problems"])
        time.sleep(0.002)  # distinct summary names
        return summary["problems"]

    def publish_refused(self, reason):
        with self.assertRaisesRegex(SystemExit, reason):
            analyze.publish()
        self.assertFalse(c.PUBLISHED.exists())

    def record(self, name, problems):
        RECORDS.append({"test": name, "problems": len(problems), "first": problems[:3]})

    def test_complete_evidence_checks_and_publishes(self):
        self.complete()
        analyze.check()
        summary = c.read_json(analyze.latest_summary())
        self.assertTrue(summary["pass"])
        analyze.publish()
        manifest = c.read_json(c.PUBLISHED / "manifest.json")
        self.assertEqual(len(manifest["pngs"]), self.plan["totals"]["pngs"])
        RECORDS.append({"test": "complete_evidence_checks_and_publishes", "pngs": len(manifest["pngs"]),
                        "bound_files": len(summary["binding"])})

    def test_empty_evidence_refuses(self):
        problems = self.check_refused()
        self.publish_refused("incomplete or failed")
        self.record("empty_evidence_refuses", problems)

    def test_missing_case_refuses(self):
        self.complete()
        rewrite(c.LEDGER, rows(c.LEDGER)[:-1])
        rewrite(c.BUDGET, rows(c.BUDGET)[:-1])
        self.release()
        problems = self.check_refused()
        self.assertTrue(any("once each in order" in p for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("missing_case_refuses", problems)

    def test_false_check_refuses(self):
        self.complete(false_check="outside_viewport_matches_off")
        problems = self.check_refused()
        self.assertTrue(any(p.startswith("viewport: required checks") for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("false_check_refuses", problems)

    def test_scale_fallback_selection_refuses(self):
        # A portable fallback at a native 12/16 or 8/16 step makes selections_as_planned false.
        self.complete(false_check="selections_as_planned")
        problems = self.check_refused()
        for case in ("transitions", "viewport"):
            self.assertTrue(any(p.startswith(f"{case}: required checks") for p in problems), case)
        self.publish_refused("incomplete or failed")
        self.record("scale_fallback_selection_refuses", problems)

    def test_failed_child_refuses(self):
        self.complete(failed_case="motion")
        problems = self.check_refused()
        self.assertTrue(any(p.startswith("motion: child failed") for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("failed_child_refuses", problems)

    def test_changed_png_refuses(self):
        self.complete()
        png = c.CAPTURES / "stationary" / "stationary-native-f16.png"
        png.write_bytes(png.read_bytes() + b"!")
        problems = self.check_refused()
        self.assertTrue(any("stationary-native-f16.png changed" in p for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("changed_png_refuses", problems)

    def test_changed_report_refuses(self):
        self.complete()
        report = c.CAPTURES / "excluded" / "report.json"
        report.write_text(report.read_text().replace('"Apple M5 Pro"', '"Apple M5 Max"'))
        problems = self.check_refused()
        self.assertTrue(any("excluded: report.json missing or changed" in p for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("changed_report_refuses", problems)

    def test_orphan_debit_refuses(self):
        self.complete()
        extra = {**rows(c.BUDGET)[0], "label": "review-orphan"}
        c.append_durable(c.BUDGET, extra)
        self.release()
        problems = self.check_refused()
        self.assertTrue(any("orphan" in p for p in problems))
        self.publish_refused("incomplete or failed")
        self.record("orphan_debit_refuses", problems)

    def test_stale_summary_refuses_publish(self):
        self.complete()
        analyze.check()
        time.sleep(0.002)
        # Evidence stays complete (a passing post-release lint), but the summary predates it.
        c.append_durable(c.POST_RELEASE_LEDGER, {"label": "lint", "kind": "build", "exit": 0,
                                                 "timed_out": False, "wait_error": None,
                                                 "group_empty_after": True})
        self.publish_refused("no fresh passing summary")
        analyze.check()
        analyze.publish()
        self.assertTrue((c.PUBLISHED / "manifest.json").exists())
        RECORDS.append({"test": "stale_summary_refuses_publish", "republished_after_fresh_check": True})

    def test_failed_summary_refuses_publish(self):
        self.complete()
        lease_text = c.LEASE.read_text()
        c.LEASE.write_text(lease_text.replace('"processes_on_target_at_release": []',
                                              '"processes_on_target_at_release": ["1 cargo"]'))
        problems = self.check_refused()
        self.assertTrue(any("lease" in p for p in problems))
        c.LEASE.write_text(lease_text)  # the evidence is complete again, the latest summary is not
        self.publish_refused("no fresh passing summary")
        self.record("failed_summary_refuses_publish", problems)


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--receipt"):
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    started = time.time()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Analyze)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args:
        with open(args[1], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(),
                       "tests_run": result.testsRun, "seconds": round(time.time() - started, 2),
                       "python": sys.version.split()[0], "records": RECORDS}, file, indent=1)
            file.write("\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
