#!/usr/bin/env python3
"""CPU-only guard and process-lifecycle tests for owner_runner.py. Standard library only.

They never run the frozen binary, a GPU or the ignored test. Every child is an
ordinary small process (a /bin/sh script standing in for the binary, a copy of
/bin/sleep, /usr/bin/python3 for one escape case) inside a fresh sandbox under
the owner-preparation CPU evidence directory, with its own lock files: the real
native-gpu.lock is never opened. Every PID a test learns (children and
grandchildren) goes to pids.jsonl; the last test proves none is alive.

    python3 -I -B handoffs/0073-upscaling-review/owner/test_owner_runner.py
"""
import fcntl
import json
import os
import shlex
import shutil
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import owner_runner as r  # noqa: E402

CPU = r.production()["cpu_evidence"]
RUN = None
ADAPTER = ('MOTION_OWNER adapter=AdapterInfo { name: "Sandbox", vendor: 0, device: 0, '
           'device_type: IntegratedGpu, driver: "", driver_info: "", backend: Metal }')
LIBTEST_OK = ("\nrunning 1 test\ntest " + r.TEST + " ... ok\n\ntest result: ok. 1 passed; "
              "0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n")
LIBTEST_FAIL = ("\nrunning 1 test\ntest " + r.TEST + " ... FAILED\n\nfailures:\n\nfailures:\n    "
                + r.TEST + "\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; "
                "0 filtered out\n\n")
ESCAPE = ('/usr/bin/python3 -c "import os,sys; os.setsid(); '
          'os.execv(sys.argv[1], [sys.argv[1], \'30\'])" "$0.sleep" &\nESC=$!\n'
          'echo $ESC > "$TMPDIR/escaped.pid"\ni=0\n'
          'while [ "$(ps -o pgid= -p $ESC | tr -d \' \')" = "$(ps -o pgid= -p $$ | tr -d \' \')" ] '
          '&& [ $i -lt 100 ]; do /bin/sleep 0.05; i=$((i+1)); done')


def script(stderr_lines, stdout_text, code=0, before=""):
    lines = ["#!/bin/sh", 'echo $$ > "$TMPDIR/leader.pid"', before]
    lines += [f"printf '%s\\n' {shlex.quote(line)} >&2" for line in stderr_lines]
    lines += [f"printf '%s' {shlex.quote(stdout_text)}", f"exit {code}"]
    return "\n".join(lines) + "\n"


OK = script([ADAPTER, r.MARKER], LIBTEST_OK)
EARLY = script([ADAPTER], LIBTEST_OK)
HANG = '#!/bin/sh\necho $$ > "$TMPDIR/leader.pid"\nexec /bin/sleep 30\n'
LIST_OUT = r.TEST + ": test\n\n1 test, 0 benchmarks\n"


def note_pids(label, *pids):
    with open(RUN / "pids.jsonl", "a") as out:
        for pid in pids:
            if pid:
                out.write(json.dumps({"pid": int(pid), "test": label}) + "\n")


def gone(pid, wait=5.0):
    deadline = time.monotonic() + wait
    while time.monotonic() < deadline:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return True
        time.sleep(0.05)
    return False


class Sandbox:
    count = 0

    def __init__(self, label, binary_script=OK):
        Sandbox.count += 1
        self.label = f"{Sandbox.count:02d}-{label}"
        self.root = RUN / self.label
        (self.root / "frozen").mkdir(parents=True)
        self.binary = self.root / "frozen" / "incant_render-sandbox"
        self.binary.write_text(binary_script)
        self.binary.chmod(0o755)
        sleeper = Path(str(self.binary) + ".sleep")  # contents only: /bin/sleep's flags are protected
        shutil.copyfile("/bin/sleep", sleeper)
        sleeper.chmod(0o755)
        ids = self.root / "ids"
        ids.mkdir()
        freeze, review, enum = ids / "freeze.json", ids / "review.json", ids / "enumeration.json"
        freeze.write_text('{"freeze": "sandbox"}\n')
        enum.write_text('{"rows": []}\n')
        expected = json.loads(json.dumps(r.EXPECTED_PLAN))
        expected["source_freeze_sha256"] = r.sha(freeze)
        expected["manifest_sha256"] = "a" * 64
        review.write_text(json.dumps({"manifest_sha256": "a" * 64,
                                      "source_freeze_sha256": r.sha(freeze)}) + "\n")
        expected["root_cpu_review_sha256"] = r.sha(review)
        expected["argv"] = [str(self.binary)] + r.EXPECTED_PLAN["argv"][1:]
        binary = {"path": str(self.binary), "sha256": r.sha(self.binary),
                  "bytes": self.binary.stat().st_size, "mode": "0755"}
        self.plan_path = self.root / "plan.json"
        self.write_plan({**expected, "state": "PREPARATION_ONLY_NO_GPU_AUTHORITY",
                         "binary": {**binary, "profile": "ci"}})
        self.evidence = self.root / "evidence"
        self.evidence.mkdir()
        self.prior = self.root / "prior-release.json"
        self.prior.write_text(json.dumps({"released_at": r.now(), "owned_processes": [],
                                          "ledger_pids_alive": []}) + "\n")
        self.spec = {"plan": self.plan_path, "plan_sha256": r.sha(self.plan_path),
                     "expected_plan": expected, "expected_binary": binary,
                     "identities": {"source_freeze": (freeze, r.sha(freeze)),
                                    "root_cpu_review": (review, r.sha(review)),
                                    "cpu_enumeration": (enum, r.sha(enum))},
                     "manifest": None,
                     "root_list_stdout_sha256": r.hashlib.sha256(LIST_OUT.encode()).hexdigest(),
                     "runner": r.HERE, "evidence": self.evidence, "cpu_evidence": self.root / "cpu",
                     "global_lock": self.root / "native-gpu.lock", "timeout": 5.0}

    def write_plan(self, plan):
        self.plan_path.write_text(json.dumps(plan, indent=2) + "\n")

    def plan(self):
        return json.loads(self.plan_path.read_text())

    def grant(self, drop=(), **changes):
        data = {"schema": r.GRANT_SCHEMA, "packet": r.PACKET, "stage": r.STAGE,
                "authority": "root", "issued_at": r.now(), "plan_sha256": self.spec["plan_sha256"],
                "runner_sha256": r.sha(r.HERE),
                "binary_sha256": self.spec["expected_binary"]["sha256"],
                "evidence_dir": str(self.evidence), "predecessor_release_path": str(self.prior),
                "predecessor_release_sha256": r.sha(self.prior), "process_launches": 1,
                "total_explicit_queue_submissions": 24, **changes}
        for key in drop:
            data.pop(key)
        (self.evidence / "grant.json").write_text(json.dumps(data) + "\n")
        return self

    def tmp_pid(self, name):
        path = self.evidence / "tmp" / name
        return int(path.read_text().split()[0]) if path.exists() else None

    def rows(self, name):
        path = self.evidence / name
        return [json.loads(x) for x in path.read_text().splitlines()] if path.exists() else []

    def run(self):
        try:
            return r.run(self.spec)
        finally:
            note_pids(self.label, *[row.get("pid") for row in self.rows("ledger.jsonl")],
                      self.tmp_pid("leader.pid"), self.tmp_pid("grandchild.pid"),
                      self.tmp_pid("escaped.pid"))


class GuardTests(unittest.TestCase):
    def refused(self, box, text):
        with self.assertRaises(r.Refused) as raised:
            box.run()
        self.assertIn(text, str(raised.exception))
        self.assertEqual(box.rows("debits.jsonl"), [], "a refusal must charge nothing")
        self.assertFalse((box.evidence / "stdout.txt").exists())

    def test_check_reports_absent_grant(self):
        box = Sandbox("check")
        report = r.check(box.spec)
        self.assertTrue(report["grant"].startswith("absent"))
        self.assertEqual(report["identities"]["binary"]["mode"], "0755")

    def test_no_grant_touches_nothing(self):
        box = Sandbox("no-grant")
        self.refused(box, "preparation only")
        self.assertEqual(os.listdir(box.evidence), [])

    def test_template_grant_is_refused(self):
        box = Sandbox("template")
        shutil.copy(r.HERE.parent / "owner-grant.template.json", box.evidence / "grant.json")
        self.refused(box, "malformed")

    def test_malformed_grants(self):
        cases = {"missing": {"drop": ["process_launches"]}, "extra": {"extra": 1},
                 "runner": {"runner_sha256": "0" * 64}, "plan": {"plan_sha256": "0" * 64},
                 "binary": {"binary_sha256": "0" * 64}, "evidence": {"evidence_dir": "/tmp/x"},
                 "launches": {"process_launches": 2},
                 "submissions": {"total_explicit_queue_submissions": 25},
                 "authority": {"authority": "claude"}, "schema": {"schema": "v0"},
                 "predecessor-hex": {"predecessor_release_sha256": "<root>"}}
        for name, change in cases.items():
            with self.subTest(name):
                box = Sandbox("grant-" + name)
                box.grant(**change)
                self.refused(box, "malformed")
        box = Sandbox("grant-not-json")
        (box.evidence / "grant.json").write_text("{not json")
        self.refused(box, "not JSON")

    def test_changed_executable(self):
        box = Sandbox("bytes").grant()
        with open(box.binary, "a") as out:
            out.write("# changed\n")
        self.refused(box, "bytes")
        for mode, name in ((0o644, "mode-0644"), (0o775, "mode-0775")):
            with self.subTest(name):
                box = Sandbox(name).grant()
                box.binary.chmod(mode)
                self.refused(box, "mode")
        box = Sandbox("symlink").grant()
        real = box.binary.with_name("real")
        box.binary.rename(real)
        box.binary.symlink_to(real)
        self.refused(box, "symlink")

    def test_executable_inside_target_directory(self):
        box = Sandbox("target")
        moved = box.root / "target" / box.binary.name
        moved.parent.mkdir()
        box.binary.rename(moved)
        plan = box.plan()
        plan["argv"][0] = plan["binary"]["path"] = str(moved)
        box.write_plan(plan)
        box.spec["plan_sha256"] = r.sha(box.plan_path)
        box.spec["expected_plan"]["argv"][0] = box.spec["expected_binary"]["path"] = str(moved)
        box.grant()
        self.refused(box, "inside a target directory")

    def test_altered_plan_fields(self):
        mutations = {"filter": lambda p: p["argv"].__setitem__(3, r.TEST + "_extended"),
                     "not-exact": lambda p: p["argv"].remove("--exact"),
                     "budget": lambda p: p["budget"].__setitem__("total_explicit_queue_submissions", 25),
                     "timeout": lambda p: p["budget"].__setitem__("timeout_seconds", 120),
                     "retries": lambda p: p["budget"].__setitem__("retries", 1),
                     "environment": lambda p: p["environment"].__setitem__("WGPU_BACKEND", "metal"),
                     "marker": lambda p: p["acceptance"].__setitem__("exact_final_marker", "MOTION_OWNER")}
        for name, mutate in mutations.items():
            for repinned in (False, True):
                with self.subTest(name, repinned=repinned):
                    box = Sandbox(f"plan-{name}-{int(repinned)}")
                    plan = box.plan()
                    mutate(plan)
                    box.write_plan(plan)
                    if repinned:  # even a re-pinned plan must match the reviewed values
                        box.spec["plan_sha256"] = r.sha(box.plan_path)
                    box.grant()
                    self.refused(box, "plan field" if repinned else "not the pinned plan")

    def test_changed_identities(self):
        box = Sandbox("freeze").grant()
        box.spec["identities"]["source_freeze"][0].write_text('{"freeze": "changed"}\n')
        self.refused(box, "source_freeze")
        box = Sandbox("review")
        review = box.spec["identities"]["root_cpu_review"][0]
        review.write_text(json.dumps({"manifest_sha256": "b" * 64,
                                      "source_freeze_sha256": "c" * 64}) + "\n")
        box.spec["identities"]["root_cpu_review"] = (review, r.sha(review))
        box.grant()
        self.refused(box, "root's CPU review")

    def test_predecessor_release(self):
        box = Sandbox("prior-absent").grant(predecessor_release_path="/nonexistent/release.json")
        self.refused(box, "predecessor release receipt")
        box = Sandbox("prior-sha").grant(predecessor_release_sha256="0" * 64)
        self.refused(box, "predecessor release receipt")
        for name, body in (("prior-not-release", {"owned_processes": []}),
                           ("prior-alive", {"released_at": "2026", "owned_processes": ["123 x"]})):
            with self.subTest(name):
                box = Sandbox(name)
                box.prior.write_text(json.dumps(body) + "\n")
                box.grant()
                self.refused(box, "predecessor")

    def test_evidence_must_be_fresh(self):
        box = Sandbox("stale").grant()
        (box.evidence / "debits.jsonl").write_text("{}\n")
        with self.assertRaises(r.Refused) as raised:
            box.run()
        self.assertIn("not fresh", str(raised.exception))

    def test_busy_locks_charge_nothing(self):
        for name, path in (("global-lock", "global_lock"), ("packet-lock", None)):
            with self.subTest(name):
                box = Sandbox(name).grant()
                lock_path = box.spec[path] if path else box.evidence / "owner.lock"
                with open(lock_path, "a") as held:
                    fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    self.refused(box, "held")

    def test_running_binary_refuses(self):
        box = Sandbox("busy-binary").grant()
        other = subprocess.Popen([str(box.binary) + ".sleep", "30"], start_new_session=True)
        try:
            self.refused(box, "already run the frozen binary")
        finally:
            other.kill()
            other.wait()
            note_pids(box.label, other.pid)


class LifecycleTests(unittest.TestCase):
    def test_accepted_child_is_reaped_and_released(self):
        box = Sandbox("accepted").grant()
        entry = box.run()
        self.assertTrue(entry["accepted"], entry["checks"])
        self.assertTrue(entry["reaped"] and entry["group_empty_after"])
        self.assertEqual(entry["survivors"], [])
        self.assertTrue(gone(entry["pid"]))
        self.assertEqual(len(box.rows("debits.jsonl")), 1)
        release = json.loads((box.evidence / "release.json").read_text())
        self.assertTrue(release["released"] and release["owned_processes"] == [])
        expected_keys = sorted(set(r.EXPECTED_PLAN["environment"]) | ({"HOME"} & set(os.environ)))
        self.assertEqual(entry["env"]["keys"], expected_keys)
        self.assertNotIn("HOME", entry["env"]["values_recorded"])
        with self.assertRaises(r.Refused):  # no retry: a second run is refused
            box.run()
        self.assertEqual(len(box.rows("debits.jsonl")), 1)

    def test_unaccepted_outputs(self):
        cases = {"early-return": (EARLY, "adapter_then_exact_final_marker_only"),
                 "vulkan": (script([ADAPTER.replace("Metal", "Vulkan"), r.MARKER], LIBTEST_OK),
                            "adapter_recorded_with_metal"),
                 "marker-twice": (script([ADAPTER, r.MARKER, r.MARKER], LIBTEST_OK),
                                  "adapter_then_exact_final_marker_only"),
                 "failed": (script([ADAPTER], LIBTEST_FAIL, code=101), "exit_zero"),
                 "two-passed": (script([ADAPTER, r.MARKER], LIBTEST_OK.replace("1 passed", "2 passed")),
                                "libtest_ok_1_passed_0_failed_0_ignored"),
                 "image": (script([ADAPTER, r.MARKER], LIBTEST_OK,
                                  before="printf '\\211PNG\\r\\n\\032\\n' > \"$TMPDIR/frame.dat\""),
                           "no_image_files")}
        for name, (body, failed_check) in cases.items():
            with self.subTest(name):
                box = Sandbox(name, body).grant()
                entry = box.run()
                self.assertFalse(entry["accepted"])
                self.assertFalse(entry["checks"][failed_check])
                self.assertTrue(entry["reaped"] and entry["group_empty_after"])
                self.assertTrue((box.evidence / "release.json").exists())

    def test_timeout_kills_the_group(self):
        box = Sandbox("timeout", HANG).grant()
        box.spec["timeout"] = 1.0
        started = time.monotonic()
        entry = box.run()
        self.assertTrue(entry["timed_out"] and not entry["accepted"])
        self.assertLess(time.monotonic() - started, 10)
        self.assertTrue(entry["reaped"] and gone(entry["pid"]))

    def test_grandchild_in_group_is_killed(self):
        box = Sandbox("grandchild", script([ADAPTER, r.MARKER], LIBTEST_OK,
                                           before='/bin/sleep 30 &\necho $! > "$TMPDIR/grandchild.pid"')).grant()
        entry = box.run()
        self.assertTrue(entry["accepted"], entry["checks"])
        self.assertTrue(gone(box.tmp_pid("grandchild.pid")))

    def test_escaped_survivor_blocks_release(self):
        box = Sandbox("escape", script([ADAPTER, r.MARKER], LIBTEST_OK, before=ESCAPE)).grant()
        with self.assertRaises(SystemExit) as raised:
            box.run()
        self.assertIn("release blocked", str(raised.exception))
        escaped = box.tmp_pid("escaped.pid")
        blocked = json.loads((box.evidence / "release-blocked.json").read_text())
        self.assertFalse(blocked["released"])
        self.assertIn(escaped, [s.get("pid") for s in blocked["survivors"]])
        self.assertFalse((box.evidence / "release.json").exists())
        os.kill(escaped, signal.SIGKILL)
        self.assertTrue(gone(escaped))

    def test_base_exception_is_recorded_then_reraised(self):
        class Abort(BaseException):
            pass
        box = Sandbox("base-exception", HANG).grant()
        box.spec["timeout"] = 10.0
        calls = {"n": 0}
        real = signal.sigpending

        def pending():
            calls["n"] += 1
            if calls["n"] == 3:
                raise Abort("injected")
            return real()
        with mock.patch.object(r.signal, "sigpending", pending):
            with self.assertRaises(Abort):
                box.run()
        [row] = box.rows("ledger.jsonl")
        self.assertIn("Abort", row["exception"])
        self.assertTrue(row["reaped"] and row["group_empty_after"] and gone(row["pid"]))
        self.assertTrue((box.evidence / "release.json").exists())

    def test_interrupt_is_recorded_before_delivery(self):
        box = Sandbox("interrupt", HANG).grant()
        box.spec["timeout"] = 10.0
        previous = signal.signal(signal.SIGTERM, r.raise_interrupt)
        sender = subprocess.Popen(["/bin/sh", "-c", f"/bin/sleep 1; kill -TERM {os.getpid()}"])
        try:
            with self.assertRaises(KeyboardInterrupt):
                box.run()
        finally:
            sender.wait()
            signal.signal(signal.SIGTERM, previous)
            note_pids(box.label, sender.pid)
        [row] = box.rows("ledger.jsonl")
        self.assertEqual(row["interrupted_by"], ["SIGTERM"])
        self.assertTrue(row["reaped"] and gone(row["pid"]))

    def test_exec_failure_is_recorded(self):
        box = Sandbox("exec-failure")
        with open(box.root / "o.txt", "x") as out, open(box.root / "e.txt", "x") as err:
            record, caught = r.launch(["/nonexistent/binary"], box.root, {}, out, err, 1.0)
        self.assertIsNone(record["pid"])
        self.assertIn("launch_error", record)
        self.assertIsNone(caught)

    def test_child_environment_is_constructed(self):
        inherited = {"HOME": "/home/x", "INCANT_FLAG": "1", "WGPU_BACKEND": "vulkan",
                     "RUST_LOG": "trace", "CARGO_TARGET_DIR": "/t", "PATH": "/opt/bin"}
        with mock.patch.dict(os.environ, inherited, clear=True):
            env = r.child_env(r.EXPECTED_PLAN["environment"], "/fresh/tmp")
        self.assertEqual(sorted(env), ["HOME", "INCANT_EXPECT_GPU_BACKEND", "PATH",
                                       "RUST_BACKTRACE", "TMPDIR"])
        self.assertEqual((env["TMPDIR"], env["PATH"], env["HOME"]),
                         ("/fresh/tmp", "/usr/bin:/bin:/usr/sbin:/sbin", "/home/x"))
        self.assertNotIn("HOME", r.env_receipt(env)["values_recorded"])

    def test_listing_is_cpu_only_enumeration(self):
        box = Sandbox("listing", "#!/bin/sh\nprintf '%s' " + shlex.quote(LIST_OUT) + "\n")
        entry = r.listing(box.spec)
        note_pids(box.label, entry["pid"])
        self.assertTrue(entry["ok"], entry)
        self.assertEqual(entry["argv"][1:], ["--list", "--exact", r.TEST])
        self.assertTrue(gone(entry["pid"]))


class ZProcessesReaped(unittest.TestCase):
    def test_every_learned_pid_is_gone(self):
        rows = [json.loads(x) for x in (RUN / "pids.jsonl").read_text().splitlines()]
        alive = []
        for row in rows:
            if not gone(row["pid"], wait=2.0):
                args = subprocess.run(["ps", "-p", str(row["pid"]), "-o", "args="],
                                      capture_output=True, text=True).stdout.strip()
                if str(RUN) in args or args.startswith("/bin/sleep"):
                    alive.append({**row, "args": args})
        r.write_new_json(RUN / "pids-final.json", {"pids": rows, "count": len(rows), "alive": alive})
        self.assertEqual(alive, [])


def setUpModule():
    global RUN
    n = 1
    while (CPU / f"tests-{n:03d}").exists():
        n += 1
    RUN = CPU / f"tests-{n:03d}"
    RUN.mkdir(parents=True)


if __name__ == "__main__":
    unittest.main(verbosity=2)
