#!/usr/bin/env python3
"""CPU-only controls for the fail-closed analyzer (pattern of the accepted 0073
test_analyze.py, narrowly reused), for admission at the fixed-warmup plan's counts
(the real plan's 110 submissions fit; nothing beyond the numeric 5, visual 115,
overall 120 or 16-PNG ceilings is admitted; the fixed-sharpness, coverage and
corrected reviews' old grants are refused; the old two-control numeric layout fails)
and for the comparison (sharpness.py) on real retained runs: the accepted baseline
compared with itself fails honestly on sharpness, the aliased original fails on
stability, a tampered baseline or prior is not exact, missing candidate outputs fail,
the rejected a143b5c0 run evaluated as a candidate reproduces exactly its 13 recorded
failures with every prior identity verified, and a changed history-free frame is
detected. Synthetic evidence in a temporary directory stands in for a complete run;
only the visual-tree identity is a test double. Retained runs and old grants are
read only. Never starts a renderer, compiler or GPU child, and writes nothing outside
the temporary directories.

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
import sharpness  # noqa: E402

RECORDS = []
TREE = "a" * 40
# label: (namespace, stage prefix of its grants)
OLD_NAMESPACES = {"fixed-sharpness-002": (c.EVIDENCE_ROOT / "0072-fixed-sharpness-002", "fixed-sharpness"),
                  "fixed-sharpness-001": (c.EVIDENCE_ROOT / "0072-fixed-sharpness-001", "fixed-sharpness"),
                  "temporal-coverage": (c.EVIDENCE_ROOT / "0072-temporal-coverage-001", "temporal-coverage"),
                  "temporal-corrected": (c.EVIDENCE_ROOT / "0072-temporal-corrected-001", "temporal-corrected")}
PACKET_DIR = c.PACKET_DIR
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
                 "contract": {"exact_numeric_tests_listed": True}}]
        debits = []
        for test in self.plan["numeric"]["tests"]:
            item = "numeric-" + test["id"]
            rows.append({**base, "label": item, "kind": "numeric", "item": item, **self.log(item),
                         "test": test["test"], "contract": {"exact_test_ran_once_and_passed": True},
                         "debit": {"frames": test["submissions"]}})
            debits.append({"label": item, "item": item, "frames": test["submissions"],
                           "ui_submissions": 0, "pngs": 0})
        rows.append({**base, "label": "list-review", "kind": "list", "item": "review", **self.log("list-review"),
                     "contract": {"matches_plan": True}})
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
        self.assertEqual(sum(d["frames"] + d["ui_submissions"] for d in debits), 110)
        code, summary = self.verdict("complete")
        self.assertEqual((code, summary["verdict"], summary["problems"]), (0, "pass", []))
        analyze.publish()
        self.assertEqual(len(list(c.PUBLISHED.glob("*.png"))), 16)
        self.assertTrue((c.PUBLISHED_EVIDENCE / "manifest.json").is_file())

    def test_plan_counts_admitted_and_ceilings_refused(self):
        c.BUDGET.unlink()
        # Visual cases first, so the visual cap is reached while the overall one is not.
        for case in self.plan["review"]["order"]:
            details = self.plan["review"]["cases"][case]
            c.debit(f"review-{case}", case, details["frames"], details["ui_submissions"],
                    len(details["pngs"]), "synthetic")
        with self.assertRaisesRegex(SystemExit, "visual submission ceiling"):
            c.debit("over", "over", 11, 0, 0, "synthetic")  # 105 + 11 = 116 > 115
        for test in self.plan["numeric"]["tests"]:
            c.debit("numeric-" + test["id"], "numeric-" + test["id"], test["submissions"], 0, 0, "synthetic")
        used = c.budget_used()
        self.assertEqual((used["frames"] + used["ui_submissions"], used["pngs"]), (110, 16))
        with self.assertRaisesRegex(SystemExit, "numeric submission ceiling"):
            c.debit("over", "numeric-over", 1, 0, 0, "synthetic")  # 6 > 5, overall 111 <= 120
        with self.assertRaisesRegex(SystemExit, "PNG budget"):
            c.debit("over", "over", 0, 0, 1, "synthetic")  # 17 > 16
        c.debit("edge", "edge", 10, 0, 0, "synthetic")  # exactly 115 visual, 120 overall
        with self.assertRaisesRegex(SystemExit, "refused: submission ceiling"):
            c.debit("after", "after", 1, 0, 0, "synthetic")
        # 5 numeric + 96 world frames + 10 = 111 frames, plus 9 UI = 120 submissions.
        self.assertEqual(c.budget_used(), {"frames": 111, "ui_submissions": 9, "pngs": 16})
        self.assertEqual(len(c.read_lines(c.REFUSALS)), 4)
        RECORDS.append({"test": "plan_counts_admitted_and_ceilings_refused", "admitted": 110,
                        "edge_admitted": {"submissions": 120, "visual": 115, "pngs": 16}, "refused": 4})

    def test_old_grants_refused(self):
        sources = {}
        for label, (namespace, stage) in OLD_NAMESPACES.items():
            self.assertNotEqual(REAL["EVIDENCE"], namespace)
            for name, call in (("review-ready.json", c.grant), ("build-ready.json", c.build_grant)):
                old = namespace / name
                body = json.loads(old.read_text()) if old.is_file() else None
                if body is None:  # a stand-in shaped like the old grants, if one was never written
                    body = {"packet": c.PACKET, "holder": "synthetic", "visual_tree": TREE,
                            "stage": f"{stage}-visual" if name == "review-ready.json" else f"{stage}-build"}
                sources[f"{label}/{name}"] = {"read_only_from": str(old) if old.is_file() else None,
                                              "sha256": c.sha(old) if old.is_file() else None,
                                              "stage": body.get("stage")}
                target = c.GRANT if name == "review-ready.json" else c.BUILD_GRANT
                target.write_text(json.dumps(body))
                with self.assertRaisesRegex(SystemExit, "refused: " + name + " does not name this packet"):
                    call()
        self.assertEqual(REAL["GRANT"].parent, REAL["EVIDENCE"])
        RECORDS.append({"test": "old_grants_refused", "old_grants": sources})

    def test_old_two_control_layout_fails(self):
        rows, debits = self.rows()
        retained = [r for r in rows if r["kind"] == "numeric"][0]
        old = self.plan["numeric"]["retained_direct_controls"]["prior_results"]
        rows = [r for r in rows if r["kind"] != "numeric"]
        debits = [d for d in debits if not d["item"].startswith("numeric")]
        for index, (item, control) in enumerate(zip(("numeric-resolve", "numeric-coverage"), old), 1):
            rows.insert(index, {**retained, "label": item, "item": item, "test": control["test"],
                                "debit": {"frames": control["submissions"]}})
            debits.insert(index - 1, {"label": item, "item": item, "frames": control["submissions"],
                                      "ui_submissions": 0, "pngs": 0})
        self.write_ledger(rows, debits)
        self.failed("old_two_control_layout", "expected one numeric child per plan control")

    def test_wrong_numeric_test_fails(self):
        rows, debits = self.rows()
        for row in rows:
            if row.get("item") == "numeric-retained":
                row["test"] = self.plan["numeric"]["retained_direct_controls"]["prior_results"][0]["test"]
        self.write_ledger(rows, debits)
        self.failed("wrong_numeric_test", "numeric child retained did not run the exact test once and pass")

    def test_compare_requires_passing_check_and_fails_honestly(self):
        with self.assertRaisesRegex(SystemExit, "no analysis exists"):
            analyze.compare()
        code, _ = self.verdict("before_compare")
        self.assertEqual(code, 0)
        self.assertEqual(analyze.compare(), 1)  # synthetic captures cannot pass a real comparison
        result = c.read_json(analyze.latest_compare())
        self.assertEqual(result["verdict"], "fail")
        self.assertTrue(result["failed"])
        analyze.publish()
        self.assertTrue((c.PUBLISHED_EVIDENCE / "analysis" / analyze.latest_compare().name).is_file())
        RECORDS.append({"test": "compare_requires_passing_check_and_fails_honestly", "exit": 1,
                        "failed": result["failed"][:3], "published": True})

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
        self.write_ledger([r for r in rows if r["kind"] != "numeric"],
                          [d for d in debits if not d["item"].startswith("numeric")])
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



class Compare(unittest.TestCase):
    """The comparison on real retained runs (read only)."""

    @classmethod
    def setUpClass(cls):
        cls.plan = c.plan()
        b = cls.plan["comparison"]["baseline"]
        cls.cases = b["cases"]
        cls.evidence, cls.screenshots = c.ROOT / b["evidence"], c.ROOT / b["screenshots"]
        cls.baseline = sharpness.run_from_published("baseline", cls.evidence, cls.screenshots, cls.cases)
        ps = cls.plan["comparison"]["prior"]
        cls.prior_paths = (c.ROOT / ps["evidence"], c.ROOT / ps["screenshots"], ps["cases"])
        cls.prior = sharpness.run_from_published("prior", *cls.prior_paths)

    def copy_prior(self, temp):
        evidence, screenshots = Path(temp) / "evidence", Path(temp) / "screenshots"
        for source, target in ((self.prior_paths[0], evidence), (self.prior_paths[1], screenshots)):
            for path in source.rglob("*"):
                if path.is_file():
                    (target / path.relative_to(source)).parent.mkdir(parents=True, exist_ok=True)
                    (target / path.relative_to(source)).write_bytes(path.read_bytes())
        return evidence, screenshots

    def summary(self, result):
        return {ch["check"]: ch["pass"] for ch in result["checks"]}

    def test_baseline_against_itself_fails_honestly_on_sharpness(self):
        candidate = sharpness.run_from_published("candidate", self.evidence, self.screenshots, self.cases)
        result = sharpness.compare(candidate, self.baseline, self.plan)
        passed = self.summary(result)
        self.assertEqual(result["verdict"], "fail")
        exactness = {k: v for k, v in passed.items()
                     if k in ("baseline_manifest_sha256", "baseline_files_match_manifest", "baseline_identity",
                              "temporal_off_references_identical", "same_adapter", "same_toolchain_and_host",
                              "same_review_config")}
        self.assertTrue(all(exactness.values()), exactness)
        self.assertFalse(passed["candidate_source_commit"])  # the baseline is not the candidate commit
        self.assertFalse(passed["grille_detail_ratio"])
        self.assertTrue(all(not v for k, v in passed.items() if k.endswith("_sigma_px")))
        self.assertTrue(passed["stationary_activity_ge33"] and passed["overlay_activity_ge33"])
        self.assertTrue(all(v for k, v in passed.items() if k.startswith("off_edge_")))
        self.assertTrue(all(v for k, v in passed.items() if k.endswith("_max_step_vs_baseline")))
        self.assertFalse(passed["viewport_report_present"])
        RECORDS.append({"test": "baseline_against_itself", "verdict": result["verdict"],
                        "checks": [{k: ch[k] for k in ("section", "check", "value", "pass")} for ch in result["checks"]],
                        "baseline_context": result["baseline_context"]})

    def test_aliased_original_fails_stability(self):
        original = sharpness.run_from_published(
            "original", PACKET_DIR / "temporal-visual-evidence", PACKET_DIR / "temporal-visual-screenshots", self.cases)
        result = sharpness.compare(original, self.baseline, self.plan, sections=["sharpness", "stability"])
        passed = self.summary(result)
        self.assertEqual(result["verdict"], "fail")
        self.assertFalse(passed["stationary_activity_ge33"])
        self.assertFalse(passed["stationary_activity_saturated"])
        RECORDS.append({"test": "aliased_original_fails_stability",
                        "checks": [{k: ch[k] for k in ("section", "check", "value", "pass")} for ch in result["checks"]]})

    def test_tampered_baseline_is_not_exact(self):
        with tempfile.TemporaryDirectory(prefix="incant-0072-tampered-") as temp:
            evidence, screenshots = Path(temp) / "evidence", Path(temp) / "screenshots"
            for source, target in ((self.evidence, evidence), (self.screenshots, screenshots)):
                for path in source.rglob("*"):
                    if path.is_file():
                        (target / path.relative_to(source)).parent.mkdir(parents=True, exist_ok=True)
                        (target / path.relative_to(source)).write_bytes(path.read_bytes())
            png = screenshots / "stationary-off.png"
            data = bytearray(png.read_bytes())
            data[-20] ^= 1
            png.write_bytes(bytes(data))
            tampered = sharpness.run_from_published("tampered", evidence, screenshots, self.cases)
            result = sharpness.compare(self.baseline, tampered, self.plan, sections=["exactness"])
        passed = self.summary(result)
        self.assertEqual(result["verdict"], "fail")
        self.assertFalse(result["exact"])
        self.assertFalse(passed["baseline_files_match_manifest"])
        RECORDS.append({"test": "tampered_baseline_is_not_exact", "failed": result["failed"]})

    def test_prior_run_reproduces_its_recorded_failures(self):
        candidate = sharpness.run_from_published("candidate", *self.prior_paths)
        result = sharpness.compare(candidate, self.baseline, self.plan, prior=self.prior)
        passed = self.summary(result)
        recorded = self.plan["comparison"]["prior"]["recorded_quality_failures"]
        self.assertEqual(result["failed"], ["exactness: candidate_source_commit", *recorded])
        for name in ("prior_manifest_sha256", "prior_files_match_manifest", "prior_identity",
                     "prior_recorded_failures", "temporal_off_identical_to_prior", "prior_same_adapter",
                     "prior_same_toolchain_and_host", "prior_same_review_config",
                     "prior_recomputed_failures_match_record", "history_free_frames_identical"):
            self.assertTrue(passed[name], name)
        frames = next(ch["value"] for ch in result["checks"] if ch["check"] == "history_free_frames_identical")
        self.assertEqual((frames["frames"], frames["compared"]), (11, {"prior": 11, "baseline": 10}))
        history = result["baseline_context"]["failure_history"]
        self.assertEqual((history["resolved"], history["persisting"], history["new"]), ([], recorded, []))
        RECORDS.append({"test": "prior_run_reproduces_its_recorded_failures", "failed": result["failed"],
                        "history_free_frames": frames, "failure_history": history})

    def test_tampered_prior_is_not_exact(self):
        with tempfile.TemporaryDirectory(prefix="incant-0072-tampered-prior-") as temp:
            evidence, screenshots = self.copy_prior(temp)
            png = screenshots / "viewport-off-f24.png"
            data = bytearray(png.read_bytes())
            data[-20] ^= 1
            png.write_bytes(bytes(data))
            tampered = sharpness.run_from_published("prior", evidence, screenshots, self.prior_paths[2])
            result = sharpness.compare(self.prior, self.baseline, self.plan, sections=["exactness"], prior=tampered)
        passed = self.summary(result)
        self.assertFalse(result["exact"])
        self.assertFalse(passed["prior_files_match_manifest"])
        self.assertNotIn("prior_recomputed_failures_match_record", passed)  # no quality section requested
        RECORDS.append({"test": "tampered_prior_is_not_exact", "failed": result["failed"]})

    def test_changed_history_free_frame_is_detected(self):
        with tempfile.TemporaryDirectory(prefix="incant-0072-reset-frame-") as temp:
            evidence, screenshots = self.copy_prior(temp)
            report = evidence / "captures" / "transitions" / "report.json"
            body = json.loads(report.read_text())
            for frame in body["frames"]:
                if frame["name"] == "cut-7":
                    frame["rgba_sha256"] = "0" * 64
            report.write_text(json.dumps(body))
            changed = sharpness.run_from_published("candidate", evidence, screenshots, self.prior_paths[2])
            result = sharpness.compare(changed, self.baseline, self.plan, sections=["reset"], prior=self.prior)
        check = next(ch for ch in result["checks"] if ch["check"] == "history_free_frames_identical")
        self.assertFalse(check["pass"])
        self.assertEqual(check["value"]["differing"], ["transitions/cut-7"])
        RECORDS.append({"test": "changed_history_free_frame_is_detected", "value": check["value"]})

    def test_missing_candidate_outputs_fail(self):
        with tempfile.TemporaryDirectory(prefix="incant-0072-empty-") as temp:
            empty = sharpness.run_from_captures("empty", temp, self.plan["review"]["order"])
            result = sharpness.compare(empty, self.baseline, self.plan,
                                       sections=["sharpness", "stability", "exclusion", "reset", "viewport"])
        self.assertEqual(result["verdict"], "fail")
        self.assertTrue(all(not ch["pass"] for ch in result["checks"]))
        RECORDS.append({"test": "missing_candidate_outputs_fail", "failed": result["failed"]})


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--receipt"):
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    suite = unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromTestCase(Analyzer),
                                unittest.defaultTestLoader.loadTestsFromTestCase(Compare)])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args:
        with open(args[1], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                       "records": RECORDS}, file, indent=1)
            file.write("\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
