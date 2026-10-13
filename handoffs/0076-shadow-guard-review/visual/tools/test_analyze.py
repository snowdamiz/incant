#!/usr/bin/env python3
"""CPU-only controls for handoff 0076's fail-closed analyzer and comparison
(adapted from 0072's test_analyze.py, 48a688c5, the accepted 0073 pattern).

Analyzer: synthetic two-role evidence in a temporary directory stands in for a
complete run; the real plan's 52 submissions and 4 PNGs are admitted and nothing
beyond the 60 or 4 ceilings is; 0072's real grants are refused; each missing,
reordered, rebound or failed piece fails the integrity check and blocks
publication. Only the visual-tree identity is a test double.

Comparison: runs assembled from the real retained 0072 viewport run (report and
both PNGs read from git objects at 48a688c5) fill both roles. The diagnosed
mismatch fails agreement honestly while its defect check passes; a synthetic
candidate whose temporal-off grille shadow matches temporal-on passes; a blurred
reference does not; an altered temporal-on frame voids exactness; an outside,
border or grille-bar change fails its guard; missing outputs fail. Synthetic
images are labelled as such. Never starts a renderer, compiler or GPU child,
and writes nothing outside the temporary directories.

  python3 -I -B test_analyze.py [--receipt NEW_PATH]
"""

import contextlib
import io
import json
import struct
import subprocess
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
import pairs  # noqa: E402
import regions  # noqa: E402
import shadowfit  # noqa: E402

RECORDS = []
TREE = "a" * 40
OLD_NAMESPACE = c.EVIDENCE_ROOT / "0072-fixed-sharpness-002"  # read only
RETAINED = "48a688c58f15ce2951292471c5d37cd2cc1fdc9b"
RETAINED_RUN = "handoffs/0072-motion-history-numeric/fixed-sharpness-evidence"
RETAINED_PNGS = "handoffs/0072-motion-history-numeric/fixed-sharpness-screenshots"


def png_header(width=960, height=540):
    """A structurally valid PNG (signature, IHDR, IEND); the analyzer reads its size."""
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    return (analyze.PNG_SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IEND", b""))


def retained(path):
    return subprocess.run(["git", "cat-file", "blob", f"{RETAINED}:{path}"], cwd=c.ROOT,
                          capture_output=True, check=True).stdout


class Analyzer(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="incant-0076-analyze-")
        e = Path(self.temp.name) / "evidence"
        self.paths = {"EVIDENCE": e, "GRANT": e / "review-ready.json", "BUILD_GRANT": e / "build-ready.json",
                      "LEASE": e / "target-lease.json", "LATCH": e / "gpu-released.json", "STOP": e / "stop.json",
                      "LEDGER": e / "gpu-ledger.jsonl", "BUDGET": e / "budget.jsonl", "BUILD": e / "build",
                      "LOGS": e / "logs", "CAPTURES": e / "captures", "ANALYSIS": e / "analysis",
                      "REFUSALS": e / "refusals.jsonl", "CHECKOUT": e / "checkout", "BIN": e / "bin",
                      "PUBLISHED": Path(self.temp.name) / "published-screenshots",
                      "PUBLISHED_EVIDENCE": Path(self.temp.name) / "published-evidence"}
        for name in ("LOGS", "CAPTURES"):
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
        records = {}
        for index, role in enumerate(c.ROLES):
            c.build_dir(role).mkdir(parents=True)
            record = c.build_dir(role) / "review.json"
            record.write_text(json.dumps({"source_commit": self.plan["sources"][role]["commit"],
                                          "frozen": {"temporal_visual_review": {"sha256": str(index) * 64}}}))
            records[role] = c.sha(record)
        c.BUILD_GRANT.write_text(json.dumps({"synthetic": True}))
        c.GRANT.write_text(json.dumps({
            "packet": c.PACKET, "stage": c.STAGE, "holder": "synthetic", "visual_tree": TREE,
            "build_records": records,
            "prior_synthetic_release_path": "/nonexistent", "prior_synthetic_release_sha256": "d" * 64}))
        c.LEASE.write_text(json.dumps({"released_at": "synthetic"}))
        base = {"exit": 0, "timed_out": False, "wait_error": None, "group_empty_after": True, "failed": []}
        rows, debits = [], []
        for index, role in enumerate(c.ROLES):
            rows.append({**base, "label": f"list-{role}", "kind": "list", "item": role, "role": role,
                         **self.log(f"list-{role}"), "contract": {"matches_fixture_cases": True}})
            for case in self.plan["review"]["order"]:
                details = self.plan["review"]["cases"][case]
                output = c.captures_dir(role) / case
                output.mkdir(parents=True)
                pngs = {}
                for name in details["pngs"]:
                    (output / name).write_bytes(png_header() + f"{role}/{name}".encode())
                    pngs[name] = c.sha(output / name)
                report = {"case": case, "outcome": "pass", "checks": {k: True for k in details["checks"]},
                          "frames_rendered": details["frames"], "ui_submissions": details["ui_submissions"],
                          "pngs": [{"name": n, "sha256": d} for n, d in pngs.items()]}
                (output / "report.json").write_text(json.dumps(report))
                (output / "events.jsonl").write_text("{}\n")
                charge = {"frames": details["frames"], "ui_submissions": details["ui_submissions"],
                          "pngs": len(details["pngs"])}
                label, binary = f"review-{role}-{case}", str(index) * 64
                rows.append({**base, "label": label, "kind": "review", "item": f"{role}/{case}", "role": role,
                             "case": case, **self.log(label), "output": str(output), "pngs": pngs,
                             "report_sha256": c.sha(output / "report.json"),
                             "events_sha256": c.sha(output / "events.jsonl"),
                             "binary_sha256_before": binary, "binary_sha256_after": binary,
                             "debit": {"label": label, **charge}})
                debits.append({"label": label, "item": f"{role}/{case}", **charge})
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
        self.assertEqual(sum(d["frames"] + d["ui_submissions"] for d in debits), 52)
        code, summary = self.verdict("complete")
        self.assertEqual((code, summary["verdict"], summary["problems"]), (0, "pass", []))
        analyze.publish()
        self.assertEqual(sorted(p.name for p in c.PUBLISHED.glob("*.png")),
                         sorted(f"{r}-{n}" for r in c.ROLES for n in self.plan["review"]["cases"]["viewport"]["pngs"]))
        self.assertTrue((c.PUBLISHED_EVIDENCE / "manifest.json").is_file())

    def test_plan_counts_admitted_and_ceilings_refused(self):
        c.BUDGET.unlink()
        details = self.plan["review"]["cases"]["viewport"]
        for role in c.ROLES:
            c.debit(f"review-{role}-viewport", f"{role}/viewport", details["frames"], details["ui_submissions"],
                    len(details["pngs"]), "synthetic")
        self.assertEqual(c.budget_used(), {"frames": 52, "ui_submissions": 0, "pngs": 4})
        with self.assertRaisesRegex(SystemExit, "refused: submission ceiling"):
            c.debit("retry", "baseline/viewport", 26, 0, 0, "synthetic")  # 78 > 60: no retry fits
        with self.assertRaisesRegex(SystemExit, "PNG budget"):
            c.debit("over", "over", 0, 0, 1, "synthetic")  # 5 > 4
        c.debit("edge", "edge", 8, 0, 0, "synthetic")  # exactly 60
        with self.assertRaisesRegex(SystemExit, "refused: submission ceiling"):
            c.debit("after", "after", 1, 0, 0, "synthetic")
        self.assertEqual(c.budget_used(), {"frames": 60, "ui_submissions": 0, "pngs": 4})
        self.assertEqual(len(c.read_lines(c.REFUSALS)), 3)
        RECORDS.append({"test": "plan_counts_admitted_and_ceilings_refused", "admitted": 52,
                        "edge_admitted": {"submissions": 60, "pngs": 4}, "refused": 3})

    def test_old_grants_refused(self):
        sources = {}
        for name, call in (("review-ready.json", c.grant), ("build-ready.json", c.build_grant)):
            old = OLD_NAMESPACE / name
            body = json.loads(old.read_text()) if old.is_file() else None
            if body is None:  # a stand-in shaped like the old grant, if it was ever removed
                body = {"packet": "0072-motion-history-numeric", "holder": "synthetic", "visual_tree": TREE,
                        "stage": "fixed-sharpness-visual" if name == "review-ready.json" else "fixed-sharpness-build"}
            sources[name] = {"read_only_from": str(old) if old.is_file() else None,
                             "sha256": c.sha(old) if old.is_file() else None, "packet": body.get("packet")}
            target = c.GRANT if name == "review-ready.json" else c.BUILD_GRANT
            target.write_text(json.dumps(body))
            with self.assertRaisesRegex(SystemExit, "refused: " + name + " does not name this packet"):
                call()
        RECORDS.append({"test": "old_grants_refused", "old_grants": sources})

    def test_missing_listing_fails(self):
        rows, debits = self.rows()
        self.write_ledger([r for r in rows if r.get("label") != "list-candidate"], debits)
        self.failed("missing_listing", "expected one passing candidate listing")

    def test_candidate_before_baseline_fails(self):
        rows, debits = self.rows()
        reviews = [r for r in rows if r["kind"] == "review"]
        others = [r for r in rows if r["kind"] != "review"]
        self.write_ledger(others + reviews[::-1], debits)
        self.failed("candidate_first", "once each in order")

    def test_missing_review_fails(self):
        rows, debits = self.rows()
        self.write_ledger([r for r in rows if r.get("item") != "candidate/viewport"],
                          [d for d in debits if d.get("item") != "candidate/viewport"])
        self.failed("missing_review", "once each in order")

    def test_rebound_build_record_fails(self):
        grant = c.read_json(c.GRANT)
        grant["build_records"]["candidate"] = grant["build_records"]["baseline"]
        c.GRANT.write_text(json.dumps(grant))
        self.failed("rebound_build_record", "candidate build record is not the one review-ready.json binds")

    def test_same_binary_for_both_roles_fails(self):
        records = {}
        for role in c.ROLES:
            record = c.build_dir(role) / "review.json"
            body = c.read_json(record)
            body["frozen"]["temporal_visual_review"]["sha256"] = "0" * 64
            record.write_text(json.dumps(body))
            records[role] = c.sha(record)
        grant = c.read_json(c.GRANT)
        grant["build_records"] = records
        c.GRANT.write_text(json.dumps(grant))
        self.failed("same_binary", "both roles froze the same binary bytes")

    def test_build_record_of_another_commit_fails(self):
        record = c.build_dir("candidate") / "review.json"
        body = c.read_json(record)
        body["source_commit"] = self.plan["sources"]["baseline"]["commit"]
        record.write_text(json.dumps(body))
        grant = c.read_json(c.GRANT)
        grant["build_records"]["candidate"] = c.sha(record)
        c.GRANT.write_text(json.dumps(grant))
        self.failed("other_commit", "does not name the plan's candidate commit")

    def test_empty_evidence_fails(self):
        for path in (c.LEDGER, c.BUDGET, c.LATCH):
            path.unlink()
        self.failed("empty_evidence", "release latch is absent")

    def test_missing_report_fails(self):
        (c.captures_dir("baseline") / "viewport" / "report.json").unlink()
        self.failed("missing_report", "report.json is absent")

    def test_false_contract_check_fails(self):
        report = c.captures_dir("candidate") / "viewport" / "report.json"
        body = c.read_json(report)
        body["checks"]["outside_viewport_matches_reference"] = False
        report.write_text(json.dumps(body))
        rows, debits = self.rows()
        for row in rows:
            if row.get("item") == "candidate/viewport":
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
        code, _ = self.verdict("stale_before_change")
        self.assertEqual(code, 0)
        (c.captures_dir("candidate") / "viewport" / "events.jsonl").write_text("{\"changed\": true}\n")
        with self.assertRaisesRegex(SystemExit, "is stale"):
            analyze.publish()
        with self.assertRaisesRegex(SystemExit, "is stale"):
            analyze.pairs()
        self.assertFalse(c.PUBLISHED.exists() or c.PUBLISHED_EVIDENCE.exists())
        RECORDS.append({"test": "stale_summary_blocks_publication", "publish": "refused", "pairs": "refused"})

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


class Compare(unittest.TestCase):
    """The comparison on the real retained 0072 viewport run (read only)."""

    @classmethod
    def setUpClass(cls):
        cls.plan = c.plan()
        cls.spec = cls.plan["comparison"]
        cls.report = json.loads(retained(f"{RETAINED_RUN}/captures/viewport/report.json"))
        cls.toolchain = retained(f"{RETAINED_RUN}/build/toolchain.json")
        cls.on = shadowfit.Image(size=(960, 540), rgba=pngread_bytes(retained(f"{RETAINED_PNGS}/{cls.spec['on']}")))
        cls.off = shadowfit.Image(size=(960, 540), rgba=pngread_bytes(retained(f"{RETAINED_PNGS}/{cls.spec['off']}")))
        cls.region = regions.region(cls.spec["pose"], cls.spec["region"])

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="incant-0076-compare-")
        self.evidence = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def run_dir(self, role, on=None, off=None, report=None):
        """One role's captures and build records; images default to the retained pair."""
        output = self.evidence / "captures" / role / "viewport"
        build = self.evidence / "build" / role
        output.mkdir(parents=True)
        build.mkdir(parents=True)
        for name, image in ((self.spec["on"], on or self.on), (self.spec["off"], off or self.off)):
            pairs.write_png(output / name, image.w, image.h, image.rgba)
        (output / "report.json").write_text(json.dumps(report or self.report))
        builds = self.plan["builds"]
        (build / "review.json").write_text(json.dumps({
            "helper_sha256": self.plan["fixture"]["sha256"],
            "frozen": {"temporal_visual_review": {"features": builds["expected_features"]}},
            "builds": [{"argv": [f"/synthetic/{role}/tools/cargo", *builds["review"], *builds["common_flags"]]}]}))
        (build / "source.json").write_text(json.dumps({
            "source_commit": self.plan["sources"][role]["commit"], "cargo_lock_sha256": "1" * 64,
            "rust_toolchain": "[toolchain]\n"}))
        (build / "toolchain.json").write_bytes(self.toolchain)
        return shadowfit.run_from_captures(role, self.evidence, role, "viewport")

    def summary(self, result):
        return {ch["check"]: ch["pass"] for ch in result["checks"]}

    def fixed(self):
        """Synthetic candidate: the retained temporal-off reference with its grille-shadow
        region (grown 8 px, never the card or outside the viewport) taken from
        temporal-on, plus a consistent still-pose report value."""
        rgba = bytearray(self.off.rgba)
        card = set(regions.mask([tuple(v) for v in regions.pose(self.spec["pose"])["card"]], 0,
                                self.spec["viewport_rect"]))
        for i in regions.grown(self.region, 8) - regions.grown(card, 1):
            rgba[i * 4:i * 4 + 4] = self.on.rgba[i * 4:i * 4 + 4]
        report = json.loads(json.dumps(self.report))
        report["observations"]["still_f16_vs_off"]["interior"]["max_delta"] = 24
        return shadowfit.Image(size=(960, 540), rgba=rgba), report

    def record(self, test, result):
        RECORDS.append({"test": test, "verdict": result["verdict"], "exact": result["exact"],
                        "checks": [{k: ch[k] for k in ("section", "check", "value", "pass")}
                                   for ch in result["checks"]]})

    def test_diagnosed_mismatch_fails_agreement_and_reproduces_defect(self):
        result = shadowfit.compare(self.run_dir("baseline"), self.run_dir("candidate"), self.plan)
        passed = self.summary(result)
        self.assertEqual(result["verdict"], "fail")
        self.assertTrue(result["exact"])
        self.assertTrue(all(v for k, v in passed.items() if k.startswith(("outside_", "candidate_still_f16_border",
                                                                         "candidate_pan_f24_border",
                                                                         "off_change", "mask_"))))
        self.assertFalse(any(v for k, v in passed.items() if k.startswith("candidate_grille_shadow_")))
        self.assertFalse(passed["candidate_still_interior_max_delta"])
        self.assertTrue(passed["baseline_reproduces_on_off_mismatch"])
        self.assertFalse(passed["candidate_better_than_baseline"])
        self.record("diagnosed_mismatch_in_both_roles", result)

    def test_synthetic_agreeing_candidate_passes(self):
        off, report = self.fixed()
        result = shadowfit.compare(self.run_dir("baseline"), self.run_dir("candidate", off=off, report=report),
                                   self.plan)
        self.assertEqual((result["verdict"], result["failed"]), ("pass", []))
        self.record("synthetic_agreeing_candidate", result)

    def test_blurred_reference_is_not_agreement(self):
        box = pairs.grow_box(regions.bbox(self.region), 12)
        blurred = shadowfit.blurred_copy(self.off, 1.3, box)
        result = shadowfit.compare(self.run_dir("baseline"), self.run_dir("candidate", off=blurred), self.plan)
        passed = self.summary(result)
        self.assertEqual(result["verdict"], "fail")
        self.assertFalse(passed["candidate_grille_shadow_mean_gap"])
        self.record("blurred_reference_is_not_agreement", result)

    def test_changed_temporal_on_frame_voids_exactness(self):
        off, report = self.fixed()
        report["frames"] = [dict(f, rgba_sha256="0" * 64) if f["name"] == "on-07" else f for f in report["frames"]]
        result = shadowfit.compare(self.run_dir("baseline"), self.run_dir("candidate", off=off, report=report),
                                   self.plan)
        self.assertEqual(result["verdict"], "fail")
        self.assertFalse(result["exact"])
        self.assertEqual(result["failed"], ["exactness: temporal_on_frames_identical"])
        self.record("changed_temporal_on_frame", result)

    def test_guards_fail_on_outside_border_and_bar_changes(self):
        off, report = self.fixed()
        failures = {}
        bar = sorted(shadowfit.bar_mask(self.off, set(regions.mask(
            [tuple(v) for v in regions.pose(self.spec["pose"])["card"]], 0, self.spec["viewport_rect"])),
            self.spec["bars"]))[0]
        for name, index in (("outside", 5 * 960 + 5), ("border", 300 * 960 + 38), ("bar", bar)):
            rgba = bytearray(off.rgba)
            rgba[index * 4:index * 4 + 3] = {"outside": bytes([255, 255, 255]), "border": bytes([0, 0, 0]),
                                               "bar": bytes([128, 128, 128])}[name]
            with tempfile.TemporaryDirectory(prefix="incant-0076-guard-") as temp:
                self.evidence = Path(temp)
                result = shadowfit.compare(
                    self.run_dir("baseline"),
                    self.run_dir("candidate", off=shadowfit.Image(size=(960, 540), rgba=rgba), report=report),
                    self.plan)
            failures[name] = result["failed"]
        self.assertEqual(failures["outside"], ["guards: outside_viewport_identical_viewport-off-f24.png"])
        self.assertEqual(failures["border"], ["guards: off_change_not_concentrated_at_border"])
        self.assertEqual(failures["bar"], ["guards: mask_card_bars_identical_off"])
        RECORDS.append({"test": "guards_fail_on_outside_border_and_bar_changes", "failed": failures})

    def test_missing_outputs_fail(self):
        empty = shadowfit.run_from_captures("candidate", self.evidence, "candidate", "viewport")
        result = shadowfit.compare(self.run_dir("baseline"), empty, self.plan)
        self.assertEqual((result["verdict"], result["exact"]), ("fail", False))
        RECORDS.append({"test": "missing_outputs_fail", "failed": result["failed"]})


def pngread_bytes(data):
    with tempfile.NamedTemporaryFile(suffix=".png") as temp:
        temp.write(data)
        temp.flush()
        return shadowfit.pngread.read(temp.name)[2]


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
