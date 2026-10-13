#!/usr/bin/env python3
"""CPU-only real child-process tests for the stage B runners; never builds, never
calls a GPU binary, never touches the real evidence directory or global lock.

Adapted from handoff 0072's committed visual/tools/test_lifecycle.py (visual
tree ac8a48de) with 0074's budgets, plus grant-source refusals: an absent,
superseded or ill-formed source is refused before any checkout, export or
child, and the build's feature and freeze guards reject partial inputs. Helper
acceptance controls: a failing lint keeps its receipt, latches stop and exits
nonzero; the lease records the grant's root-assigned holder; only root's
{commit, files} freeze is accepted and its build-input coverage must be exact.

Only paths, plan and grant/source lookups are redirected to a temporary
fixture; the actual supervise/run_child/run_group code runs with /usr/bin/true,
/usr/bin/false, /bin/sleep and /bin/sh. Signals come from a separate process so
the main thread's deferred mask is genuinely exercised.

  python3 -I -B test_lifecycle.py [--receipt NEW_PATH]
"""

import contextlib
import hashlib
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402
import common as c  # noqa: E402
import gpu  # noqa: E402
import lease  # noqa: E402

RECORDS = []
PLAN = {"expected_backend": "Metal", "budgets": {"new_submissions_max": 128, "pngs_max": 20}}
GRANT = {"packet": c.PACKET, "stage": c.STAGE, "source_commit": "a" * 40, "source_tree": "c" * 40,
         "visual_tree": "b" * 40, "source_freeze": {"path": "/Users/andreyyurlov/.codex/freeze.json",
                                                    "sha256": "d" * 64},
         "features": ["native-metalfx"], "target": "/Users/andreyyurlov/.codex/lifecycle-test-target"}


def absent(pid):
    for probe in (os.kill, os.killpg):
        try:
            probe(pid, 0)
        except ProcessLookupError:
            continue
        return False
    return True


def lines(path):
    return [json.loads(x) for x in Path(path).read_text().splitlines()] if Path(path).exists() else []


class Lifecycle(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="incant-0074-lifecycle-")
        root = Path(self.directory.name)
        self.paths = {name: root / relative for name, relative in {
            "EVIDENCE": "evidence", "GRANT": "evidence/review-ready.json",
            "LEASE": "evidence/target-lease.json", "LOGS": "evidence/logs",
            "LEDGER": "evidence/ledger.jsonl", "BUDGET": "evidence/budget.jsonl",
            "POST_RELEASE_LEDGER": "evidence/post.jsonl", "STOP": "evidence/stop.json",
            "LATCH": "evidence/released.json", "REFUSALS": "evidence/refusals.jsonl",
            "SCRATCH": "evidence/tmp", "CHECKOUT": "evidence/checkout", "BUILD": "evidence/build",
            "PACKET_LOCK": "evidence/packet.lock", "GPU_LOCK": "native-gpu.lock"}.items()}
        for name in ("EVIDENCE", "SCRATCH", "CHECKOUT"):
            self.paths[name].mkdir(parents=True, exist_ok=True)
        self.patches = contextlib.ExitStack()
        self.patches.enter_context(mock.patch.multiple(c, **self.paths))
        self.patches.enter_context(mock.patch.object(c, "plan", return_value=PLAN))
        self.patches.enter_context(mock.patch.object(c, "tools_identity", return_value={}))
        self.patches.enter_context(mock.patch.object(c, "verify_checkout", return_value={}))
        self.patches.enter_context(contextlib.redirect_stdout(io.StringIO()))

    def tearDown(self):
        self.patches.close()
        self.directory.cleanup()

    def fake_gate(self):
        return mock.patch.object(c, "gate", return_value=({}, "cpu-only-grant"))

    def child(self, label, program, args=(), timeout=5, charge=(8, 0, 1), killer=None):
        """run_child with a dummy program; returns (raised exception or None, rows)."""
        program = Path(program)
        sender = None
        if killer:
            sender = subprocess.Popen(["/bin/sh", "-c", f"sleep 0.4; kill -{killer} {os.getpid()}"])
        raised = None
        with self.fake_gate(), mock.patch.object(gpu, "frozen", return_value=(program, c.sha(program))):
            try:
                gpu.run_child(label, "review", label, "cpu-fixture", list(args), ".", timeout,
                              charge=charge)
            except BaseException as error:  # interrupts and SystemExit are the subject here
                raised = error
            finally:
                if sender is not None:
                    sender.wait()
        return raised, lines(c.LEDGER)

    def record(self, name, row, raised):
        RECORDS.append({"test": name, "row": {k: row.get(k) for k in (
            "pid", "exit", "timed_out", "wait_error", "group_empty_after", "failed", "debit")},
            "raised": type(raised).__name__ if raised else None})

    def assert_reaped(self, row):
        self.assertTrue(absent(row["pid"]))
        self.assertTrue(row["group_empty_after"])

    def test_gpu_success_debits_before_launch(self):
        raised, rows = self.child("success", "/usr/bin/true")
        self.assertIsNone(raised)
        row = rows[0]
        self.assert_reaped(row)
        self.assertEqual((row["exit"], row["failed"]), (0, []))
        debit = lines(c.BUDGET)[0]
        self.assertLessEqual(debit["at"], row["started"])
        self.assertEqual((debit["frames"], debit["pngs"]), (8, 1))
        self.assertFalse(c.STOP.exists())
        self.record("gpu_success", row, raised)

    def test_gpu_timeout_kills_and_stops(self):
        raised, rows = self.child("timeout", "/bin/sleep", ["30"], timeout=0.3)
        row = rows[0]
        self.assert_reaped(row)
        self.assertEqual(row["exit"], -signal.SIGKILL)
        self.assertTrue(row["timed_out"])
        self.assertIsInstance(raised, SystemExit)
        self.assertTrue(c.STOP.exists())
        self.record("gpu_timeout", row, raised)

    def test_gpu_failure_exit_latches_stop(self):
        raised, rows = self.child("false", "/usr/bin/false")
        row = rows[0]
        self.assert_reaped(row)
        self.assertEqual(row["exit"], 1)
        self.assertIsInstance(raised, SystemExit)
        self.assertEqual(c.read_json(c.STOP)["label"], "false")
        self.record("gpu_failure_exit", row, raised)

    def interrupted(self, name, killer, expected):
        raised, rows = self.child(name, "/bin/sleep", ["30"], timeout=20, killer=killer)
        row = rows[0]
        self.assert_reaped(row)
        self.assertEqual(row["exit"], -signal.SIGKILL)
        self.assertEqual(row["wait_error"], expected.__name__)
        self.assertIsInstance(raised, expected)
        self.assertTrue(c.STOP.exists())
        self.assertEqual(len(lines(c.BUDGET)), 1)  # the interrupted attempt stays charged
        self.record(name, row, raised)

    def test_gpu_sigint(self):
        self.interrupted("sigint", "INT", KeyboardInterrupt)

    def test_gpu_sigterm(self):
        self.interrupted("sigterm", "TERM", c.Interrupted)

    def test_gpu_sighup(self):
        self.interrupted("sighup", "HUP", c.Interrupted)

    def test_child_starts_with_default_signal_mask(self):
        # A blocked SIGTERM would let the shell reach `exit 3`.
        raised, rows = self.child("mask", "/bin/sh", ["-c", "kill -TERM $$; exit 3"])
        row = rows[0]
        self.assertEqual(row["exit"], -signal.SIGTERM)
        self.assertIsInstance(raised, SystemExit)
        self.record("child_default_mask", row, raised)

    def test_non_executable_binary_refused_before_anything(self):
        program = Path(self.directory.name) / "not-executable"
        program.write_bytes(b"#!/bin/sh\nexit 0\n")
        program.chmod(0o644)
        raised, rows = self.child("mode", program)
        self.assertIsInstance(raised, SystemExit)
        self.assertIn("not executable", str(raised))
        self.assertEqual((rows, lines(c.BUDGET)), ([], []))
        RECORDS.append({"test": "non_executable_refused", "raised": str(raised)})

    def test_budget_exhaustion_refused_before_launch(self):
        raised, rows = self.child("excess", "/usr/bin/true", charge=(120, 9, 1))
        self.assertIsInstance(raised, SystemExit)
        self.assertIn("submission budget", str(raised))
        self.assertEqual((rows, lines(c.BUDGET)), ([], []))
        RECORDS.append({"test": "budget_exhaustion_refused", "raised": str(raised)})

    def test_busy_global_gpu_lock_refused_before_charge(self):
        holder = subprocess.Popen(
            [sys.executable, "-I", "-B", "-c",
             "import fcntl,sys,time\nf=open(sys.argv[1],'a')\nfcntl.flock(f,fcntl.LOCK_EX)\n"
             "print('locked',flush=True)\ntime.sleep(10)", str(c.GPU_LOCK)],
            stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(holder.stdout.readline().strip(), "locked")
            raised, rows = self.child("busy", "/usr/bin/true")
        finally:
            holder.kill()
            holder.wait()
            holder.stdout.close()
        self.assertIsInstance(raised, SystemExit)
        self.assertIn("native-gpu.lock is held", str(raised))
        self.assertEqual((rows, lines(c.BUDGET)), ([], []))
        RECORDS.append({"test": "busy_gpu_lock_refused", "raised": str(raised)})

    def test_stop_latch_blocks_the_next_child(self):
        c.write_new_json(c.GRANT, GRANT)
        c.write_new_json(c.LEASE, {"grant_sha256": c.sha(c.GRANT)})
        c.latch_stop("earlier", "simulated failure")
        program = Path("/usr/bin/true")
        with mock.patch.object(gpu, "frozen", return_value=(program, c.sha(program))):
            with self.assertRaisesRegex(SystemExit, "stop.json latched a failure"):
                gpu.run_child("after-stop", "review", "after", "cpu-fixture", [], ".", 5,
                              charge=(1, 0, 0))
        self.assertEqual((lines(c.LEDGER), lines(c.BUDGET)), ([], []))
        self.assertEqual(len(lines(c.REFUSALS)), 1)
        RECORDS.append({"test": "stop_latch_blocks_next_child", "refusals": lines(c.REFUSALS)})

    def build_case(self, name, program, args, timeout, killer=None):
        sender = None
        if killer:
            sender = subprocess.Popen(["/bin/sh", "-c", f"sleep 0.4; kill -{killer} {os.getpid()}"])
        raised = None
        try:
            build.run_group(name, [program, *args], Path(self.directory.name),
                            {"PATH": c.SYSTEM_PATH}, timeout, f"{name}.out", f"{name}.err")
        except BaseException as error:
            raised = error
        finally:
            if sender is not None:
                sender.wait()
        row = lines(c.LEDGER)[0]
        self.assert_reaped(row)
        self.record("build_" + name, row, raised)
        return row, raised

    def test_build_success(self):
        row, raised = self.build_case("ok", "/usr/bin/true", [], 5)
        self.assertEqual((row["exit"], raised, c.STOP.exists()), (0, None, False))

    def test_build_timeout_latches_stop(self):
        row, _ = self.build_case("slow", "/bin/sleep", ["30"], 0.3)
        self.assertTrue(row["timed_out"])
        self.assertTrue(c.STOP.exists())

    def test_build_sigterm(self):
        row, raised = self.build_case("term", "/bin/sleep", ["30"], 20, killer="TERM")
        self.assertEqual(row["wait_error"], "Interrupted")
        self.assertIsInstance(raised, c.Interrupted)
        self.assertTrue(c.STOP.exists())

    def test_absent_grant_refuses_without_files(self):
        with self.assertRaisesRegex(SystemExit, "review-ready.json is absent"):
            c.grant()
        self.assertFalse(c.REFUSALS.exists())
        RECORDS.append({"test": "absent_grant_refused_without_files"})

    def test_superseded_source_refused_before_checkout(self):
        # Any git call or export during these refusals would raise instead.
        for commit in sorted(c.DENIED_SOURCES):
            c.GRANT.unlink(missing_ok=True)
            c.write_new_json(c.GRANT, {**GRANT, "source_commit": commit})
            with mock.patch.object(build.subprocess, "Popen", side_effect=AssertionError("export")), \
                    mock.patch.object(build.subprocess, "run", side_effect=AssertionError("git")):
                with self.assertRaisesRegex(SystemExit, "superseded"):
                    build.checkout()
        self.assertFalse((c.BUILD / "checkout.json").exists())
        self.assertEqual(list(c.CHECKOUT.iterdir()), [])
        reasons = [row["reason"] for row in lines(c.REFUSALS)]
        self.assertEqual(len(reasons), len(c.DENIED_SOURCES))
        RECORDS.append({"test": "superseded_sources_refused_before_checkout", "refusals": reasons})

    def test_ill_formed_sources_refused(self):
        variants = {
            "superseded_freeze": {**GRANT, "source_freeze": {
                "path": "/Users/andreyyurlov/.codex/freeze.json", "sha256": next(iter(c.DENIED_FREEZES))}},
            "partial_cooked_features": {**GRANT, "features": ["native-metalfx", "cooked-materials"]},
            "missing_tree": {k: v for k, v in GRANT.items() if k != "source_tree"},
            "missing_freeze": {k: v for k, v in GRANT.items() if k != "source_freeze"},
        }
        for grant in variants.values():
            c.GRANT.unlink(missing_ok=True)
            c.write_new_json(c.GRANT, grant)
            with self.assertRaises(SystemExit):
                c.grant()
        reasons = [row["reason"] for row in lines(c.REFUSALS)]
        self.assertEqual(len(reasons), len(variants))
        RECORDS.append({"test": "ill_formed_sources_refused", "refusals": reasons})

    def test_explicit_source_passes_the_grant_gate(self):
        c.write_new_json(c.GRANT, GRANT)
        data, digest = c.grant()
        self.assertEqual((data["source_commit"], digest), (GRANT["source_commit"], c.sha(c.GRANT)))
        self.assertEqual(build.feature_args({"features": ["native-metalfx", "cooked-temporal"]}),
                         ["--features", "cooked-temporal,native-metalfx"])
        self.assertFalse(c.REFUSALS.exists())
        RECORDS.append({"test": "explicit_source_passes_the_grant_gate"})

    def test_listing_passes_the_plan_filter(self):
        # The late plan's filter keeps the frozen 002 cases in the binary unlisted.
        plan = {"budgets": {"listing_timeout_s": 60},
                "review": {"binary": "review", "cwd": ".", "cases": {"late_a": {}, "late_b": {}},
                           "list_args": ["--list", "--ignored", "late_"]}}
        with mock.patch.object(c, "plan", return_value=plan), \
                mock.patch.object(gpu, "run_child", return_value={}) as child:
            gpu.listing()
        args, after = child.call_args.args[4], child.call_args.kwargs["after"]
        self.assertEqual(args, ["--list", "--ignored", "late_"])
        self.assertTrue(after("late_a: test\nlate_b: test\n")["contract"]["matches_plan"])
        self.assertFalse(after("late_a: test\nlate_b: test\nstationary: test\n")["contract"]["matches_plan"])
        RECORDS.append({"test": "listing_passes_the_plan_filter", "args": args})

    def test_feature_and_jitter_guards(self):
        admitted = build.features_admitted
        self.assertTrue(admitted(["native-metalfx"], ["native-metalfx", "physics-deterministic"]))
        self.assertFalse(admitted(["native-metalfx"], ["cooked-materials", "native-metalfx"]))
        self.assertTrue(admitted(["cooked-temporal", "native-metalfx"],
                                 ["cooked-materials", "cooked-temporal", "native-metalfx"]))
        self.assertFalse(admitted(["cooked-temporal", "native-metalfx"], ["native-metalfx"]))
        text = c.HELPER_SOURCE.read_text()
        table = c.samples(text)
        self.assertEqual(len(table), 8)
        self.assertNotEqual(c.samples(text.replace("[0.125, -0.125]", "[-0.125, 0.125]", 1)), table)
        RECORDS.append({"test": "feature_and_jitter_guards", "jitter_table": table})

    def test_lint_failure_exits_nonzero_after_receipt_and_stop(self):
        c.write_new_json(c.GRANT, GRANT)
        c.write_new_json(c.LEASE, {"grant_sha256": c.sha(c.GRANT)})
        cargo = c.CHECKOUT / "tools" / "cargo"
        cargo.parent.mkdir(parents=True)
        cargo.write_text("#!/bin/sh\necho 'error: simulated clippy failure' >&2\nexit 101\n")
        cargo.chmod(0o755)
        with mock.patch.object(c, "plan", return_value=json.loads(c.PLAN_PATH.read_text())), \
                mock.patch.object(c, "verify_checkout", return_value={"helper_copy_sha256": "copied"}):
            with self.assertRaisesRegex(SystemExit, "lint failed"):
                build.lint()
        receipt = c.read_json(c.BUILD / "lint.json")
        self.assertEqual((receipt["exit"], lines(c.LEDGER)[0]["exit"]), (101, 101))
        self.assertTrue(c.STOP.exists())
        RECORDS.append({"test": "lint_failure_exits_nonzero_after_receipt_and_stop",
                        "receipt_exit": receipt["exit"], "stop": c.read_json(c.STOP)})

    def test_lease_records_root_assigned_holder(self):
        release = Path(self.directory.name) / "prior-released.json"
        c.write_new_json(release, {"owned_processes": [], "ledger_pids_alive": []})
        pins = {"prior_numeric_release_path": str(release), "prior_numeric_release_sha256": c.sha(release)}
        c.write_new_json(c.GRANT, {**GRANT, **pins})
        with self.assertRaisesRegex(SystemExit, "root-assigned lease holder"):
            lease.acquire()
        self.assertFalse(c.LEASE.exists())
        holder = "root (Astra) supervised build for handoff 0074"
        c.GRANT.unlink()
        c.write_new_json(c.GRANT, {**GRANT, **pins, "holder": holder})
        lease.acquire()
        self.assertEqual(c.read_json(c.LEASE)["holder"], holder)
        RECORDS.append({"test": "lease_records_root_assigned_holder", "holder": holder})

    def test_freeze_rows_accept_only_the_committed_files_shape(self):
        commit = "e" * 40
        good = {"commit": commit, "files": [{"path": "Cargo.lock", "sha256": "1" * 64, "bytes": 3}]}
        rows, problem = build.freeze_rows(good, commit)
        self.assertEqual((problem, sorted(rows)), (None, ["Cargo.lock"]))
        bad = {"other_commit": {**good, "commit": "f" * 40},
               "no_commit": {"files": good["files"]},
               "old_sources_shape": {"commit": commit, "sources": good["files"]},
               "empty": {"commit": commit, "files": []},
               "absolute_path": {"commit": commit, "files": [{"path": "/etc/hosts", "sha256": "1" * 64}]},
               "parent_path": {"commit": commit, "files": [{"path": "../x", "sha256": "1" * 64}]},
               "short_digest": {"commit": commit, "files": [{"path": "a", "sha256": "12"}]},
               "duplicate_path": {"commit": commit, "files": good["files"] * 2}}
        reasons = {name: build.freeze_rows(freeze, commit)[1] for name, freeze in bad.items()}
        self.assertTrue(all(reasons.values()), reasons)
        RECORDS.append({"test": "freeze_rows_accept_only_the_committed_files_shape", "refused": reasons})

    def test_freeze_coverage_is_exact_or_refused(self):
        files = {"crates/a.rs": b"fn a() {}\n", "Cargo.lock": b"lock\n",
                 "tools/probes/probe.py": b"probe\n", "docs/outside.md": b"doc\n"}
        for path, data in files.items():
            (c.CHECKOUT / path).parent.mkdir(parents=True, exist_ok=True)
            (c.CHECKOUT / path).write_bytes(data)
        entries = [{"path": path} for path in files]
        row = lambda path: {"sha256": hashlib.sha256(files[path]).hexdigest(), "bytes": len(files[path])}
        roots = ["crates", "Cargo.lock"]
        exact = {path: row(path) for path in ("crates/a.rs", "Cargo.lock", "tools/probes/probe.py")}
        coverage = build.freeze_coverage(exact, entries, roots)
        self.assertTrue(coverage["exact"])
        self.assertEqual(coverage["freeze_rows_outside_roots"], ["tools/probes/probe.py"])
        variants = {"missing_input": {k: v for k, v in exact.items() if k != "Cargo.lock"},
                    "row_not_in_commit": {**exact, "crates/gone.rs": row("Cargo.lock")},
                    "changed_content": {**exact, "crates/a.rs": {**row("crates/a.rs"), "sha256": "0" * 64}},
                    "changed_size": {**exact, "Cargo.lock": {**row("Cargo.lock"), "bytes": 99}},
                    "changed_probe": {**exact, "tools/probes/probe.py": {**row("tools/probes/probe.py"),
                                                                         "sha256": "0" * 64}}}
        results = {name: build.freeze_coverage(rows, entries, roots)["exact"] for name, rows in variants.items()}
        self.assertFalse(any(results.values()), results)
        RECORDS.append({"test": "freeze_coverage_is_exact_or_refused", "exact_controls": results})

    def test_checkout_refuses_a_freeze_naming_another_commit_before_export(self):
        commit, tree = "077f7c39c0f4122aae36ea3a88537be006722822", "408c4bb2948983cfde980fa40c31f443b5d78a0c"
        freeze = Path(self.directory.name) / "freeze.json"
        c.write_new_json(freeze, {"commit": "4bc810fe0daf238f219801de76b06f19e4daf826",
                                  "files": [{"path": "Cargo.lock", "sha256": "1" * 64}]})
        c.write_new_json(c.GRANT, {**GRANT, "source_commit": commit, "source_tree": tree,
                                   "source_freeze": {"path": str(freeze), "sha256": c.sha(freeze)}})
        c.write_new_json(c.LEASE, {"grant_sha256": c.sha(c.GRANT)})
        c.CHECKOUT.rmdir()
        with self.assertRaisesRegex(SystemExit, "does not name the granted commit"):
            build.checkout()
        self.assertFalse(c.CHECKOUT.exists())  # nothing was exported
        RECORDS.append({"test": "checkout_refuses_a_freeze_naming_another_commit_before_export",
                        "refusal": lines(c.REFUSALS)[-1]["reason"]})


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--receipt"):
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    started = time.time()
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Lifecycle)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args:
        with open(args[1], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(),
                       "tests_run": result.testsRun, "seconds": round(time.time() - started, 2),
                       "python": sys.version.split()[0], "records": RECORDS}, file, indent=1)
            file.write("\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
