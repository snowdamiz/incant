#!/usr/bin/env python3
"""CPU-only controls for the fail-closed analyzer (pattern of the accepted 0073
test_analyze.py, narrowly reused) and for admission at the fixed-coverage plan's
counts: the real plan's 229 submissions fit, nothing beyond the 256 ceiling or 16
PNGs is admitted, and the corrected-source review's old grants are refused.
Synthetic evidence in a temporary directory stands in for a complete run; only the
visual-tree identity is a test double. Old grants are read only. Never starts a
renderer, compiler or GPU child, and writes nothing outside the temporary directory.

  python3 -I -B test_analyze.py [--receipt NEW_PATH]
"""

import contextlib
import hashlib
import io
import json
import struct
import sys
import tempfile
import unittest
import zlib
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import common as c  # noqa: E402

RECORDS = []
TREE = "a" * 40
OLD_NAMESPACE = c.EVIDENCE_ROOT / "0072-temporal-corrected-001"
OLD_NUMERIC = "temporal::system::device_tests::actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion"
REAL = {"EVIDENCE": c.EVIDENCE, "GRANT": c.GRANT, "BUILD_GRANT": c.BUILD_GRANT}  # before any patching


def sha(data):
    return hashlib.sha256(data).hexdigest()


def png_header(width=960, height=540):
    """A structurally valid PNG (signature, IHDR, IEND); the analyzer reads its size."""
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    return (analyze.PNG_SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IEND", b""))


class Analyzer(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="incant-0072-analyze-")
        e = Path(self.temp.name) / "evidence"
        self.paths = {"EVIDENCE": e, "GRANT": e / "review-ready.json", "BUILD_GRANT": e / "build-ready.json",
                      "LEASE": e / "target-lease.json", "LATCH": e / "gpu-released.json", "STOP": e / "stop.json",
                      "LEDGER": e / "gpu-ledger.jsonl", "BUDGET": e / "budget.jsonl", "BUILD": e / "build",
                      "LOGS": e / "logs", "CAPTURES": e / "captures", "ANALYSIS": e / "analysis",
                      "REFUSALS": e / "refusals.jsonl",
                      "PUBLISHED": Path(self.temp.name) / "published-screenshots",
                      "PUBLISHED_EVIDENCE": Path(self.temp.name) / "published-evidence"}
        for name in ("BUILD", "LOGS", "CAPTURES"):
            self.paths[name].mkdir(parents=True)
        self.stack = contextlib.ExitStack()
        self.stack.enter_context(mock.patch.multiple(c, **self.paths))
        self.stack.enter_context(mock.patch.object(
            c, "tools_identity", side_effect=lambda data: {"visual_tree": data["visual_tree"]}))
        self.stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
        self.plan = c.plan()
        self.write_complete()

    def tearDown(self):
        self.stack.close()
        self.temp.cleanup()

    # ----------------------------------------------------------- synthetic evidence

    def log(self, label):
        out, err = c.LOGS / f"gpu-{label}.stdout.txt", c.LOGS / f"gpu-{label}.stderr.txt"
        out.write_text(f"synthetic {label}\n")
        err.write_text("")
        return {"stdout": str(out), "stdout_sha256": c.sha(out), "stderr": str(err), "stderr_sha256": c.sha(err)}

    def write_complete(self):
        binary = "b" * 64
        (c.BUILD / "review.json").write_text(json.dumps({"frozen": {"temporal_visual_review": {"sha256": binary}}}))
        c.BUILD_GRANT.write_text(json.dumps({"synthetic": True}))
        c.GRANT.write_text(json.dumps({
            "packet": c.PACKET, "stage": c.STAGE, "holder": "synthetic", "visual_tree": TREE,
            "build_record_sha256": c.sha(c.BUILD / "review.json"),
            "prior_synthetic_release_path": "/nonexistent", "prior_synthetic_release_sha256": "d" * 64}))
        c.LEASE.write_text(json.dumps({"released_at": "synthetic"}))
        base = {"exit": 0, "timed_out": False, "wait_error": None, "group_empty_after": True, "failed": []}
        rows = [{**base, "label": "list-lib", "kind": "list", "item": "lib", **self.log("list-lib"),
                 "contract": {"exact_numeric_test_listed": True}},
                {**base, "label": "numeric", "kind": "numeric", "item": "numeric", **self.log("numeric"),
                 "test": self.plan["numeric"]["test"], "contract": {"exact_test_ran_once_and_passed": True},
                 "debit": {"frames": self.plan["numeric"]["submissions"]}},
                {**base, "label": "list-review", "kind": "list", "item": "review", **self.log("list-review"),
                 "contract": {"matches_plan": True}}]
        debits = [{"label": "numeric", "item": "numeric", "frames": self.plan["numeric"]["submissions"],
                   "ui_submissions": 0, "pngs": 0}]
        for case in self.plan["review"]["order"]:
            details = self.plan["review"]["cases"][case]
            output = c.CAPTURES / case
            output.mkdir()
            pngs = {}
            for name in details["pngs"]:
                (output / name).write_bytes(png_header() + name.encode())
                pngs[name] = c.sha(output / name)
            report = {"case": case, "outcome": "pass", "checks": {k: True for k in details["checks"]},
                      "frames_rendered": details["frames"], "ui_submissions": details["ui_submissions"],
                      "pngs": [{"name": n, "sha256": d} for n, d in pngs.items()]}
            (output / "report.json").write_text(json.dumps(report))
            (output / "events.jsonl").write_text("{}\n")
            charge = {"frames": details["frames"], "ui_submissions": details["ui_submissions"],
                      "pngs": len(details["pngs"])}
            rows.append({**base, "label": f"review-{case}", "kind": "review", "item": case,
                         **self.log(f"review-{case}"), "output": str(output), "pngs": pngs,
                         "report_sha256": c.sha(output / "report.json"),
                         "events_sha256": c.sha(output / "events.jsonl"),
                         "binary_sha256_before": binary, "binary_sha256_after": binary,
                         "debit": {"label": f"review-{case}", **charge}})
            debits.append({"label": f"review-{case}", "item": case, **charge})
        self.write_ledger(rows, debits)

    def write_ledger(self, rows, debits, stop_latched=False):
        c.LEDGER.write_text("".join(json.dumps(r) + "\n" for r in rows))
        c.BUDGET.write_text("".join(json.dumps(d) + "\n" for d in debits))
        if c.LATCH.exists():
            c.LATCH.unlink()
        c.LATCH.write_text(json.dumps({
            "owned_processes": [], "ledger_pids_alive": [], "debits_without_ledger_row": [],
            "ledger_sha256": c.sha(c.LEDGER), "debits_sha256": c.sha(c.BUDGET), "stop_latched": stop_latched}))

    def rows(self):
        return c.read_lines(c.LEDGER), c.read_lines(c.BUDGET)

    def verdict(self, test):
        code = analyze.check()
        summary = c.read_json(analyze.latest_summary())
        RECORDS.append({"test": test, "exit": code, "verdict": summary["verdict"],
                        "problems": summary["problems"][:3]})
        return code, summary

    def failed(self, test, fragment):
        code, summary = self.verdict(test)
        self.assertEqual((code, summary["verdict"]), (1, "fail"))
        self.assertTrue(any(fragment in p for p in summary["problems"]), summary["problems"])
        with self.assertRaisesRegex(SystemExit, "did not pass"):
            analyze.publish()
        self.assertFalse(c.PUBLISHED.exists() or c.PUBLISHED_EVIDENCE.exists())

    # ----------------------------------------------------------- controls

    def test_complete_current_evidence_passes_and_publishes(self):
        _, debits = self.rows()
        self.assertEqual(sum(d["frames"] + d["ui_submissions"] for d in debits), 229)
        code, summary = self.verdict("complete")
        self.assertEqual((code, summary["verdict"], summary["problems"]), (0, "pass", []))
        analyze.publish()
        self.assertEqual(len(list(c.PUBLISHED.glob("*.png"))), 14)
        self.assertTrue((c.PUBLISHED_EVIDENCE / "manifest.json").is_file())

    def test_plan_counts_admitted_and_ceilings_refused(self):
        c.BUDGET.unlink()
        c.debit("numeric", "numeric", self.plan["numeric"]["submissions"], 0, 0, "synthetic")
        for case in self.plan["review"]["order"]:
            details = self.plan["review"]["cases"][case]
            c.debit(f"review-{case}", case, details["frames"], details["ui_submissions"],
                    len(details["pngs"]), "synthetic")
        used = c.budget_used()
        self.assertEqual((used["frames"] + used["ui_submissions"], used["pngs"]), (229, 14))
        with self.assertRaisesRegex(SystemExit, "submission ceiling"):
            c.debit("over", "over", 28, 0, 0, "synthetic")  # 257 > 256
        with self.assertRaisesRegex(SystemExit, "PNG budget"):
            c.debit("over", "over", 0, 0, 3, "synthetic")  # 17 > 16
        c.debit("edge", "edge", 27, 0, 2, "synthetic")  # exactly 256 and 16
        with self.assertRaisesRegex(SystemExit, "submission ceiling"):
            c.debit("after", "after", 1, 0, 0, "synthetic")
        # 150 numeric + 70 world frames + 27 = 247 frames, plus 9 UI = 256 submissions.
        self.assertEqual(c.budget_used(), {"frames": 247, "ui_submissions": 9, "pngs": 16})
        self.assertEqual(len(c.read_lines(c.REFUSALS)), 3)
        RECORDS.append({"test": "plan_counts_admitted_and_ceilings_refused", "admitted": 229,
                        "edge_admitted": {"submissions": 256, "pngs": 16}, "refused": 3})

    def test_old_corrected_grants_refused(self):
        self.assertNotEqual(REAL["EVIDENCE"], OLD_NAMESPACE)
        self.assertEqual(REAL["GRANT"].parent, REAL["EVIDENCE"])
        self.assertEqual(REAL["BUILD_GRANT"].parent, REAL["EVIDENCE"])
        sources = {}
        for name, call in (("review-ready.json", c.grant), ("build-ready.json", c.build_grant)):
            old = OLD_NAMESPACE / name
            body = json.loads(old.read_text()) if old.is_file() else None
            if body is None:  # a stand-in shaped like the old grants, if they were ever removed
                body = {"packet": c.PACKET, "holder": "synthetic", "visual_tree": TREE,
                        "stage": "temporal-corrected-visual" if name == "review-ready.json"
                        else "temporal-corrected-build"}
            sources[name] = {"read_only_from": str(old) if old.is_file() else None,
                             "sha256": c.sha(old) if old.is_file() else None, "stage": body.get("stage")}
            target = c.GRANT if name == "review-ready.json" else c.BUILD_GRANT
            target.write_text(json.dumps(body))
            with self.assertRaisesRegex(SystemExit, "refused: " + name + " does not name this packet"):
                call()
        RECORDS.append({"test": "old_corrected_grants_refused", "old_grants": sources})

    def test_old_numeric_identity_fails(self):
        rows, debits = self.rows()
        for row in rows:
            if row["kind"] == "numeric":
                row["test"], row["debit"] = OLD_NUMERIC, {"frames": 14}
        for debit in debits:
            if debit["item"] == "numeric":
                debit["frames"] = 14
        self.write_ledger(rows, debits)
        self.failed("old_numeric_identity", "did not run the exact test once and pass")
        summary = c.read_json(analyze.latest_summary())
        self.assertTrue(any("not debited the plan's submissions" in p for p in summary["problems"]))

    def test_missing_lib_listing_fails(self):
        rows, debits = self.rows()
        self.write_ledger([r for r in rows if r.get("label") != "list-lib"], debits)
        self.failed("missing_lib_listing", "expected one passing lib listing")

    def test_empty_evidence_fails(self):
        for path in (c.LEDGER, c.BUDGET, c.LATCH):
            path.unlink()
        self.failed("empty_evidence", "release latch is absent")

    def test_missing_case_fails(self):
        rows, debits = self.rows()
        self.write_ledger([r for r in rows if r.get("item") != "skinned"],
                          [d for d in debits if d.get("item") != "skinned"])
        self.failed("missing_case", "are not the plan cases")

    def test_numeric_not_run_fails(self):
        rows, debits = self.rows()
        self.write_ledger([r for r in rows if r["kind"] != "numeric"], [d for d in debits if d["item"] != "numeric"])
        self.failed("numeric_not_run", "expected one numeric child")

    def test_missing_report_fails(self):
        (c.CAPTURES / "motion" / "report.json").unlink()
        self.failed("missing_report", "report.json is absent")

    def test_false_contract_check_fails(self):
        report = c.CAPTURES / "transitions" / "report.json"
        body = c.read_json(report)
        body["checks"]["cut_advances_epoch"] = False
        report.write_text(json.dumps(body))
        rows, debits = self.rows()
        for row in rows:
            if row.get("item") == "transitions":
                row["report_sha256"] = c.sha(report)  # current hashes: only the false check differs
        self.write_ledger(rows, debits)
        self.failed("false_contract_check", "contract check is false")

    def test_failed_child_fails(self):
        rows, debits = self.rows()
        rows[-1]["failed"] = ["exit 101"]
        self.write_ledger(rows, debits)
        self.failed("failed_child", "the child failed")

    def test_stop_latch_fails(self):
        c.STOP.write_text(json.dumps({"label": "synthetic", "reason": "control"}))
        self.failed("stop_latch", "stop latched")

    def test_stale_summary_blocks_publication(self):
        code, summary = self.verdict("stale_before_change")
        self.assertEqual(code, 0)
        (c.CAPTURES / "excluded" / "events.jsonl").write_text("{\"changed\": true}\n")
        with self.assertRaisesRegex(SystemExit, "is stale"):
            analyze.publish()
        self.assertFalse(c.PUBLISHED.exists() or c.PUBLISHED_EVIDENCE.exists())
        RECORDS.append({"test": "stale_summary_blocks_publication", "publish": "refused"})


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--receipt"):
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Analyzer)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args:
        with open(args[1], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                       "records": RECORDS}, file, indent=1)
            file.write("\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
