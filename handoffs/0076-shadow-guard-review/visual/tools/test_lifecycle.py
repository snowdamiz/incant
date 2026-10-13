#!/usr/bin/env python3
"""CPU-only real child-process tests for handoff 0076's runners (adapted from
0072's test_lifecycle.py, 48a688c5); never builds, never calls a GPU binary,
never touches the real evidence directory or global lock.

Only paths, plan and grant/source lookups are redirected to a temporary
fixture; the actual supervise/run_child/run_group code runs with /usr/bin/true,
/usr/bin/false, /bin/sleep and /bin/sh. Signals come from a separate process so
the main thread's deferred mask is genuinely exercised.

  python3 -I -B test_lifecycle.py [--receipt NEW_PATH]
"""

import contextlib
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

RECORDS = []
PLAN = {"expected_backend": "Metal", "budgets": {"submissions_max": 60, "pngs_max": 4},
        "review": {"binary": "cpu-fixture"}}
ROLE = "baseline"
HOLDER = "lifecycle-test-holder"


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
        self.directory = tempfile.TemporaryDirectory(prefix="incant-0076-lifecycle-")
        root = Path(self.directory.name)
        self.paths = {name: root / relative for name, relative in {
            "EVIDENCE": "evidence", "GRANT": "evidence/review-ready.json",
            "BUILD_GRANT": "evidence/build-ready.json",
            "LEASE": "evidence/target-lease.json", "LOGS": "evidence/logs",
            "LEDGER": "evidence/ledger.jsonl", "BUDGET": "evidence/budget.jsonl",
            "POST_RELEASE_LEDGER": "evidence/post.jsonl", "STOP": "evidence/stop.json",
            "LATCH": "evidence/released.json", "REFUSALS": "evidence/refusals.jsonl",
            "SCRATCH": "evidence/tmp", "CHECKOUT": "evidence/checkout",
            "PACKET_LOCK": "evidence/packet.lock", "GPU_LOCK": "native-gpu.lock"}.items()}
        for name in ("EVIDENCE", "SCRATCH", "CHECKOUT"):
            self.paths[name].mkdir(parents=True, exist_ok=True)
        (self.paths["CHECKOUT"] / ROLE).mkdir()
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
        return mock.patch.object(c, "gate", return_value=({"holder": HOLDER}, "cpu-only-grant"))

    def child(self, label, program, args=(), timeout=5, charge=(8, 0, 1), killer=None):
        """run_child with a dummy program; returns (raised exception or None, rows)."""
        program = Path(program)
        sender = None
        if killer:
            sender = subprocess.Popen(["/bin/sh", "-c", f"sleep 0.4; kill -{killer} {os.getpid()}"])
        raised = None
        with self.fake_gate(), mock.patch.object(gpu, "frozen", return_value=(program, c.sha(program), "cpu-fixture")):
            try:
                gpu.run_child(label, "review", label, ROLE, list(args), ".", timeout,
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
        self.assertEqual(row["holder"], HOLDER)  # receipts name the actual grant holder
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
        raised, rows = self.child("excess", "/usr/bin/true", charge=(53, 8, 1))  # 61 > 60
        self.assertIsInstance(raised, SystemExit)
        self.assertIn("submission ceiling", str(raised))
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
        grant = {"packet": c.PACKET, "stage": c.STAGE, "holder": HOLDER, "visual_tree": "b" * 40,
                 "build_records": {role: "c" * 64 for role in c.ROLES},
                 "prior_test_release_path": "/nonexistent/gpu-released.json",
                 "prior_test_release_sha256": "d" * 64}
        c.write_new_json(c.GRANT, grant)
        c.latch_stop("earlier", "simulated failure")
        program = Path("/usr/bin/true")
        with mock.patch.object(gpu, "frozen", return_value=(program, c.sha(program), "cpu-fixture")):
            with self.assertRaisesRegex(SystemExit, "stop.json latched a failure"):
                gpu.run_child("after-stop", "review", "after", ROLE, [], ".", 5,
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
                            {"PATH": c.SYSTEM_PATH}, timeout, f"{name}.out", f"{name}.err", HOLDER)
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
