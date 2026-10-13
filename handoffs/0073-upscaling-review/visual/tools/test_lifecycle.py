#!/usr/bin/env python3
"""CPU-only real child-process tests of the 0073 runners; never builds or calls a
GPU binary. Adapted from handoff 0068's committed tools/test_lifecycle.py.

Only the grant, committed-tree, checkout and frozen-binary lookups and the plan
are replaced, inside a temporary evidence directory with its own packet and
"shared" lock files. The real gate (latches and lease), budget, debit, launch,
poll, kill/reap and ledger code runs with /usr/bin/true, /usr/bin/false,
/bin/sleep, /bin/sh and this interpreter as children. Interrupts are sent from
a separate process so the runner's signal mask is exercised as in production.

  python3 -I -B test_lifecycle.py [--receipt NEW_PATH]
"""

import contextlib
import fcntl
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402
import common as c  # noqa: E402
import gpu  # noqa: E402

RECORDS = []
PLAN = {"budgets": {"frames_max": 96, "pngs_max": 16, "build_timeout_s": 60},
        "review": {"cwd": "crates/incant_render",
                   "backend_env": {"INCANT_EXPECT_GPU_BACKEND": "Metal"}}}


def absent(pid):
    for probe in (os.kill, os.killpg):
        try:
            probe(pid, 0)
        except ProcessLookupError:
            continue
        return False
    return True


class Lifecycle(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="incant-0073-lifecycle-")
        root = Path(self.directory.name)
        evidence = root / "evidence"
        self.paths = {"EVIDENCE": evidence, "GRANT": evidence / "review-ready.json",
                      "LEASE": evidence / "target-lease.json", "LATCH": evidence / "gpu-released.json",
                      "STOP": evidence / "stop.json", "LEDGER": evidence / "gpu-ledger.jsonl",
                      "DEBITS": evidence / "debits.jsonl", "POST_RELEASE_LEDGER": evidence / "post.jsonl",
                      "REFUSALS": evidence / "refusals.jsonl", "LOCK": evidence / "gpu.lock",
                      "GLOBAL_LOCK": root / "native-gpu.lock", "CHECKOUT": evidence / "checkout",
                      "BUILD": evidence / "build", "BIN": evidence / "bin", "LOGS": evidence / "logs",
                      "SCRATCH": evidence / "tmp", "CAPTURES": evidence / "captures"}
        (self.paths["CHECKOUT"] / "crates/incant_render").mkdir(parents=True)
        self.paths["SCRATCH"].mkdir()
        self.paths["LEASE"].write_text(json.dumps({"grant_sha256": "cpu-only-grant"}))
        self.previous = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGHUP)}
        c.install_interrupts()
        self.stack = contextlib.ExitStack()
        self.stack.enter_context(mock.patch.multiple(c, **self.paths))
        self.stack.enter_context(mock.patch.object(
            c, "grant", return_value=({"target": str(root / "no-target")}, "cpu-only-grant")))
        self.stack.enter_context(mock.patch.object(c, "tools_identity", return_value={}))
        self.stack.enter_context(mock.patch.object(c, "verify_checkout", return_value={}))
        self.stack.enter_context(mock.patch.object(c, "plan", return_value=PLAN))
        self.stack.enter_context(contextlib.redirect_stdout(io.StringIO()))

    def tearDown(self):
        self.stack.close()
        for signum, handler in self.previous.items():
            signal.signal(signum, handler)
        self.directory.cleanup()

    def use(self, program):
        program = Path(program)
        return mock.patch.object(gpu, "frozen", return_value=(program, c.sha(program)))

    def rows(self, name):
        return c.read_lines(self.paths[name])

    def record(self, test, row, **extra):
        RECORDS.append({"test": test, "pid": row.get("pid"), "exit": row.get("exit"),
                        "timed_out": row.get("timed_out"), "interrupted_by": row.get("interrupted_by"),
                        "group_empty_after": row.get("group_empty_after"), "ok": row.get("ok"),
                        "pid_and_group_absent": row.get("pid") is None or absent(row["pid"]), **extra})

    def assert_reaped(self, row):
        self.assertTrue(row["pid"] is None or absent(row["pid"]))
        self.assertTrue(row["group_empty_after"])

    def test_gpu_debit_is_durable_before_launch(self):
        script = f"cat {self.paths['DEBITS']}"
        with self.use("/bin/sh"):
            gpu.run_child("debit", "review", "probe", ["-c", script], 10, frames=7, pngs=2)
        [row] = self.rows("LEDGER")
        stdout = Path(row["stdout"]).read_text()
        self.assertIn('"label": "debit"', stdout)
        self.assertIn('"frames": 7', stdout)
        self.assertTrue(row["ok"])
        self.assert_reaped(row)
        self.assertEqual([(d["frames"], d["pngs"]) for d in self.rows("DEBITS")], [(7, 2)])
        self.record("gpu_debit_is_durable_before_launch", row, child_saw_its_debit=True)

    def test_gpu_failure_latches_stop_and_keeps_debit(self):
        with self.use("/usr/bin/false"):
            with self.assertRaisesRegex(SystemExit, "failed; stop latched"):
                gpu.run_child("false", "review", "probe", [], 10, frames=5)
            with self.assertRaisesRegex(SystemExit, "stop.json"):
                gpu.run_child("after-stop", "review", "stationary", [], 10, frames=5)
        [row] = self.rows("LEDGER")
        self.assertEqual((row["exit"], row["ok"]), (1, False))
        self.assertTrue(self.paths["STOP"].exists())
        self.assertEqual(len(self.rows("DEBITS")), 1)
        self.assert_reaped(row)
        self.record("gpu_failure_latches_stop_and_keeps_debit", row,
                    later_child_refused_without_debit=True)

    def test_gpu_timeout_kills_group(self):
        with self.use("/bin/sleep"):
            with self.assertRaises(SystemExit):
                gpu.run_child("timeout", "review", "probe", ["30"], 0.3, frames=3)
        [row] = self.rows("LEDGER")
        self.assertTrue(row["timed_out"])
        self.assertEqual(row["exit"], -signal.SIGKILL)
        self.assert_reaped(row)
        self.assertTrue(self.paths["STOP"].exists())
        self.record("gpu_timeout_kills_group", row)

    def interrupt(self, runner, signum):
        sender = subprocess.Popen(["/bin/sh", "-c", f"sleep 0.5; kill -{signum.name[3:]} {os.getpid()}"])
        try:
            with self.assertRaises(KeyboardInterrupt):
                with self.use("/bin/sleep"):
                    if runner == "gpu":
                        gpu.run_child("interrupt", "review", "probe", ["30"], 20, frames=4)
                    else:
                        build.run_group("interrupt", ["/bin/sleep", "30"], self.paths["CHECKOUT"],
                                        {"PATH": c.SYSTEM_PATH}, 20, "out", "err")
        finally:
            sender.wait()
        [row] = self.rows("LEDGER")
        self.assertEqual(row["interrupted_by"], [signum.name])
        self.assertEqual(row["exit"], -signal.SIGKILL)
        self.assert_reaped(row)
        if runner == "gpu":
            self.assertTrue(self.paths["STOP"].exists())
            self.assertEqual(len(self.rows("DEBITS")), 1)
        self.record(f"{runner}_{signum.name.lower()}_records_then_raises", row,
                    keyboard_interrupt_after_recording=True)

    def test_gpu_sigterm(self):
        self.interrupt("gpu", signal.SIGTERM)

    def test_gpu_sigint(self):
        self.interrupt("gpu", signal.SIGINT)

    def test_gpu_sighup(self):
        self.interrupt("gpu", signal.SIGHUP)

    def test_build_sigterm(self):
        self.interrupt("build", signal.SIGTERM)

    def test_child_starts_with_empty_signal_mask(self):
        probe = "import signal; print(sorted(int(s) for s in signal.pthread_sigmask(signal.SIG_BLOCK, [])))"
        with self.use(sys.executable):
            gpu.run_child("mask", "review", "probe", ["-I", "-S", "-c", probe], 20)
        [row] = self.rows("LEDGER")
        self.assertEqual(Path(row["stdout"]).read_text().strip(), "[]")
        self.assertTrue(row["ok"])
        self.record("child_starts_with_empty_signal_mask", row, child_blocked_signals=[])

    def test_exec_failure_recorded_and_charged(self):
        bogus = Path(self.directory.name) / "not-a-binary"
        bogus.write_text("not an executable\n")
        bogus.chmod(0o755)
        with self.use(bogus):
            with self.assertRaises(SystemExit):
                gpu.run_child("exec", "review", "probe", [], 10, frames=2)
        [row] = self.rows("LEDGER")
        self.assertIn("launch_error", row)
        self.assertFalse(row["ok"])
        self.assertEqual(len(self.rows("DEBITS")), 1)
        self.assertTrue(self.paths["STOP"].exists())
        self.record("exec_failure_recorded_and_charged", row, launch_error=row["launch_error"])

    def refused_without_charge(self, test, pattern, program="/usr/bin/true", frames=1, pngs=0):
        with self.use(program):
            with self.assertRaisesRegex(SystemExit, pattern):
                gpu.run_child(test, "review", "probe", [], 10, frames=frames, pngs=pngs)
        self.assertFalse(self.paths["LEDGER"].exists())
        self.assertFalse((self.paths["LOGS"] / f"gpu-{test}.stdout.txt").exists())
        RECORDS.append({"test": test, "refused": pattern, "charged": False})

    def test_busy_shared_gpu_lock_refuses_before_debit(self):
        with open(self.paths["GLOBAL_LOCK"], "a") as holder:
            fcntl.flock(holder, fcntl.LOCK_EX)
            self.refused_without_charge("busy_shared_lock", "native-gpu.lock is held")
        self.assertFalse(self.paths["DEBITS"].exists())

    def test_frame_budget_refuses_before_debit(self):
        c.append_synced(self.paths["DEBITS"], {"label": "x", "item": "x", "frames": 90, "pngs": 0})
        self.refused_without_charge("frame_budget", "frame budget", frames=7)
        self.assertEqual(len(self.rows("DEBITS")), 1)

    def test_png_budget_refuses_before_debit(self):
        c.append_synced(self.paths["DEBITS"], {"label": "x", "item": "x", "frames": 0, "pngs": 14})
        self.refused_without_charge("png_budget", "PNG budget", pngs=3)
        self.assertEqual(len(self.rows("DEBITS")), 1)

    def test_stop_latch_refuses_before_debit(self):
        c.write_new_json(self.paths["STOP"], {"reason": "test"})
        self.refused_without_charge("stop_latch", "stop.json")
        self.assertFalse(self.paths["DEBITS"].exists())

    def test_release_latch_refuses_before_debit(self):
        c.write_new_json(self.paths["LATCH"], {"reason": "test"})
        self.refused_without_charge("release_latch", "already released")
        self.assertFalse(self.paths["DEBITS"].exists())

    def test_non_executable_binary_refuses_before_debit(self):
        plain = Path(self.directory.name) / "plain"
        plain.write_bytes(b"\0")
        plain.chmod(0o644)
        self.refused_without_charge("not_executable", "not executable", program=plain)
        self.assertFalse(self.paths["DEBITS"].exists())

    def test_build_success_and_timeout(self):
        build.run_group("ok", ["/usr/bin/true"], self.paths["CHECKOUT"], {"PATH": c.SYSTEM_PATH},
                        10, "ok.out", "ok.err")
        build.run_group("slow", ["/bin/sleep", "30"], self.paths["CHECKOUT"], {"PATH": c.SYSTEM_PATH},
                        0.3, "slow.out", "slow.err")
        ok, slow = self.rows("LEDGER")
        self.assertEqual((ok["exit"], ok["timed_out"]), (0, False))
        self.assertEqual((slow["exit"], slow["timed_out"]), (-signal.SIGKILL, True))
        for row in (ok, slow):
            self.assert_reaped(row)
            self.record(f"build_{row['label']}", row)

    def test_release_after_children(self):
        with self.use("/usr/bin/true"):
            gpu.run_child("before-release", "review", "probe", [], 10, frames=1)
        # release() hashes the grant file it was granted under.
        self.paths["GRANT"].write_text('{"cpu_only": true}\n')
        gpu.release()
        latch = c.read_json(self.paths["LATCH"])
        self.assertEqual((latch["owned_processes"], latch["ledger_pids_alive"]), ([], []))
        self.assertEqual(latch["debits_without_ledger_row"], [])
        self.assertEqual(latch["frames_debited"], 1)
        with self.use("/usr/bin/true"):
            with self.assertRaisesRegex(SystemExit, "already released"):
                gpu.run_child("after-release", "review", "probe", [], 10, frames=1)
        RECORDS.append({"test": "release_after_children", "latch": {
            k: latch[k] for k in ("children", "debits", "frames_debited", "owned_processes",
                                  "ledger_pids_alive", "debits_without_ledger_row")},
            "later_child_refused": True})


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Lifecycle)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if len(sys.argv) == 3 and sys.argv[1] == "--receipt":
        with open(sys.argv[2], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(),
                       "tests_run": result.testsRun, "failures": len(result.failures),
                       "errors": len(result.errors), "python": sys.version.split()[0],
                       "records": RECORDS}, file, indent=2)
            file.write("\n")
    elif len(sys.argv) != 1:
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    raise SystemExit(0 if result.wasSuccessful() else 1)
