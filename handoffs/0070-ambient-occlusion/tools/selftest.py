#!/usr/bin/env python3
"""CPU-only process-lifecycle and refusal controls for the 0070 runner.

Standard library only. Children are Python/`sleep` stand-ins; nothing builds,
renders or touches a GPU. Scratch files live in `handoffs/0070-ambient-occlusion/
.selftest-<pid>/` (inside the worktree, outside tools/) and are removed at the
end. End-to-end checks drive the real runner commands with stand-in frozen
binaries; for that, the harness process points `EVIDENCE_ROOT` and the helper
lock location at the scratch directory and shortens watchdogs in memory only.
They require the helper files to be committed (the source freeze refuses
uncommitted helper bytes, by design).

  python3 -I -B selftest.py            # all controls
  python3 -I -B selftest.py --unit     # skip end-to-end runner checks
"""

import json
import os
import shutil
import signal
import subprocess
import sys
import time
import traceback
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import common as c  # noqa: E402
import lifecycle  # noqa: E402
from ledger import Ledger, Refused  # noqa: E402

PYTHON = sys.executable
RESULTS = []


def check(name):
    def wrap(function):
        def run(*args):
            started = time.monotonic()
            try:
                function(*args)
                RESULTS.append({"control": name, "ok": True, "seconds": round(time.monotonic() - started, 2)})
            except BaseException as error:  # record and continue with the other controls
                RESULTS.append({"control": name, "ok": False, "error": f"{type(error).__name__}: {error}",
                                "trace": traceback.format_exc()[-1500:]})
        run.__name__ = function.__name__
        return run
    return wrap


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    # A zombie still answers kill(0); report it as gone only once reaped.
    state = subprocess.run(["/bin/ps", "-o", "stat=", "-p", str(pid)], capture_output=True, text=True).stdout
    return bool(state.strip()) and not state.strip().startswith("Z")


def wait_gone(pid, seconds=10.0):
    end = time.monotonic() + seconds
    while alive(pid) and time.monotonic() < end:
        time.sleep(0.05)
    return not alive(pid)


def run_child(scratch, name, argv, timeout, **kwargs):
    outcome = {}
    result = lifecycle.run(argv, env={"PATH": c.SYSTEM_PATH}, cwd=scratch,
                           stdout_path=scratch / f"{name}.out", stderr_path=scratch / f"{name}.err",
                           timeout_s=timeout, outcome=outcome, **kwargs)
    return result


# ------------------------------------------------------------------ unit controls

@check("porcelain -z keeps leading status columns; a stripped stream is refused")
def porcelain(_scratch):
    data = (b" M crates/a.rs\0?? handoffs/0070-ambient-occlusion/x y.md\0R  new.rs\0old.rs\0"
            b"A  b.txt\0 D gone.rs\0")
    entries = c.parse_porcelain_z(data)
    assert entries == [(" M", "crates/a.rs", None), ("??", "handoffs/0070-ambient-occlusion/x y.md", None),
                       ("R ", "new.rs", "old.rs"), ("A ", "b.txt", None), (" D", "gone.rs", None)], entries
    try:
        c.parse_porcelain_z(data.strip())
    except Refused:
        pass
    else:
        raise AssertionError("a stripped porcelain stream must not parse")


@check("constructed environments exclude ambient overrides and record no forwarded values")
def environment(scratch):
    saved = dict(os.environ)
    try:
        os.environ.update({"RUSTFLAGS": "-Cfoo", "WGPU_BACKEND": "gl", "INCANT_HDR_EVIDENCE": "/x",
                           "TRACY_ENABLE": "1", "CARGO_FEATURE_X": "1", "RUSTC_WRAPPER": "sccache",
                           "CARGO_ENCODED_RUSTFLAGS": "x", "AWS_SECRET_ACCESS_KEY": "not-a-secret"})
        build = c.build_env(scratch / "target", scratch / "tmp")
        run = c.run_env(scratch / "tmp", "Metal")
        for env in (build, run):
            leaked = [k for k in env if k in ("RUSTFLAGS", "WGPU_BACKEND", "INCANT_HDR_EVIDENCE", "TRACY_ENABLE",
                                              "CARGO_FEATURE_X", "RUSTC_WRAPPER", "CARGO_ENCODED_RUSTFLAGS",
                                              "AWS_SECRET_ACCESS_KEY")]
            assert not leaked, leaked
        try:
            c.recorded_env({**build, "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "cc"})
        except Refused:
            pass
        else:
            raise AssertionError("target-specific linker override accepted")
        recorded = c.recorded_env(build)
        assert "HOME" in recorded["forwarded_unchanged"] and "HOME" not in recorded["values"]
        assert set(run) == {"PATH", "TMPDIR", "RUST_BACKTRACE", "LC_ALL", "INCANT_EXPECT_GPU_BACKEND"}, run
        try:
            c.recorded_env({**run, "WGPU_POWER_PREF": "high"})
        except Refused:
            pass
        else:
            raise AssertionError("forbidden key accepted")
        try:
            c.run_env(scratch, "Gl")
        except Refused:
            pass
        else:
            raise AssertionError("unknown backend accepted")
    finally:
        os.environ.clear()
        os.environ.update(saved)


@check("ledger ceilings refuse before any reservation; reservations are never released")
def ledger_budget(scratch):
    directory = scratch / "ledger"
    directory.mkdir()
    ledger = Ledger(directory)
    budgets = c.load_plan()["budgets"]
    with ledger.locked():
        ledger.check_and_reserve(budgets, {"label": "a", "kind": "suite", "existing_frames": 2000,
                                           "new_frames": 120, "timeout_s": 10})
        for request in ({"label": "b", "existing_frames": 49}, {"label": "c", "new_frames": 9},
                        {"label": "d", "pngs": 33}, {"label": "a"}, {"label": "e", "timeout_s": 3000}):
            try:
                ledger.check_and_reserve(budgets, {"kind": "numeric", "timeout_s": 10, **request})
            except Refused:
                continue
            raise AssertionError(f"accepted {request}")
        assert ledger.totals()["existing_frames"] == 2000 and ledger.totals()["new_frames"] == 120
        assert [r["label"] for r in ledger.unresolved()] == ["a"]
        ledger.append({"type": "complete", "label": "a"})
        assert ledger.unresolved() == [] and ledger.totals()["new_frames"] == 120


@check("the packet lock is exclusive and never waits")
def lock(scratch):
    directory = scratch / "lock"
    directory.mkdir()
    first, second = Ledger(directory), Ledger(directory)
    with first.locked():
        try:
            with second.locked():
                raise AssertionError("second lock acquired")
        except Refused:
            pass
    with second.locked():
        pass


@check("normal exit: leader reaped, no signals, group absent")
def normal(scratch):
    out = run_child(scratch, "normal", [PYTHON, "-I", "-c", "print('ok')"], 10, on_spawn=lambda *_: None)
    assert out["exit"] == 0 and out["signals_sent"] == [] and out["group_absent"], out
    assert (scratch / "normal.out").read_text() == "ok\n"


@check("timeout: SIGTERM then reap; group absent")
def timeout(scratch):
    started = time.monotonic()
    out = run_child(scratch, "timeout", [PYTHON, "-I", "-c", "import time; time.sleep(30)"], 0.5,
                    on_spawn=lambda *_: None)
    assert out["timed_out"] and out["signal"] == "SIGTERM" and out["group_absent"], out
    assert time.monotonic() - started < 6 and not alive(out["pid"])


@check("SIGTERM ignored: escalates to SIGKILL after the grace period")
def escalation(scratch):
    code = "import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print('ready', flush=True); time.sleep(30)"
    out = run_child(scratch, "escalate", [PYTHON, "-I", "-c", code], 1.0, on_spawn=lambda *_: None, grace_s=0.5)
    assert out["signals_sent"][:2] == ["SIGTERM", "SIGKILL"] and out["signal"] == "SIGKILL", out
    assert out["group_absent"], out


@check("grandchild left in the group after the leader exits is killed")
def grandchild(scratch):
    code = "import subprocess; p = subprocess.Popen(['/bin/sleep', '30']); print(p.pid, flush=True)"
    out = run_child(scratch, "grandchild", [PYTHON, "-I", "-c", code], 10, on_spawn=lambda *_: None)
    pid = int((scratch / "grandchild.out").read_text().split()[0])
    assert out["exit"] == 0 and "SIGKILL" in out["signals_sent"] and out["group_absent"], out
    assert wait_gone(pid), f"grandchild {pid} survived"


@check("on_spawn failure (ledger write) still kills and reaps the child")
def spawn_failure(scratch):
    outcome = {}

    def fail(pid, pgid):
        raise OSError("simulated ledger write failure")

    try:
        lifecycle.run([PYTHON, "-I", "-c", "import time; time.sleep(30)"], env={"PATH": c.SYSTEM_PATH},
                      cwd=scratch, stdout_path=scratch / "spawn.out", stderr_path=scratch / "spawn.err",
                      timeout_s=10, on_spawn=fail, outcome=outcome)
    except OSError:
        pass
    else:
        raise AssertionError("on_spawn failure was swallowed")
    assert outcome["interrupted"].startswith("OSError") and outcome["group_absent"], outcome
    assert not alive(outcome["pid"])


HARNESS = r'''
import json, sys, time
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import lifecycle
scratch = Path(sys.argv[2])
lifecycle.install_signal_handlers()
outcome = {}
def on_spawn(pid, pgid):
    (scratch / "harness.pid").write_text(str(pid))
code = "import subprocess,time; subprocess.Popen(['/bin/sleep','60']); time.sleep(60)"
try:
    lifecycle.run([sys.executable, "-I", "-c", code], env={"PATH": "/usr/bin:/bin"}, cwd=scratch,
                  stdout_path=scratch / "h.out", stderr_path=scratch / "h.err", timeout_s=60,
                  on_spawn=on_spawn, outcome=outcome)
    status = "completed"
except BaseException as error:
    status = type(error).__name__
(scratch / "harness.json").write_text(json.dumps({"status": status, "outcome": outcome}))
'''


def harness(scratch, sig):
    work = scratch / f"harness-{sig}"
    work.mkdir()
    proc = subprocess.Popen([PYTHON, "-I", "-B", "-c", HARNESS, str(TOOLS), str(work)],
                            start_new_session=True)
    end = time.monotonic() + 10
    while not (work / "harness.pid").exists() and time.monotonic() < end:
        time.sleep(0.05)
    child = int((work / "harness.pid").read_text())
    time.sleep(0.3)
    os.kill(proc.pid, sig)
    proc.wait(timeout=20)
    return work, child, proc


@check("interrupted wait: SIGINT to the runner kills and reaps the child group first")
def interrupt(scratch):
    work, child, _ = harness(scratch, signal.SIGINT)
    report = json.loads((work / "harness.json").read_text())
    assert report["status"] == "KeyboardInterrupt", report
    assert report["outcome"]["group_absent"] and "SIGTERM" in report["outcome"]["signals_sent"], report
    assert wait_gone(child)


@check("SIGTERM to the runner is converted and cleans up the same way")
def terminate(scratch):
    work, child, _ = harness(scratch, signal.SIGTERM)
    report = json.loads((work / "harness.json").read_text())
    assert report["status"] == "Terminated", report
    assert report["outcome"]["group_absent"], report
    assert wait_gone(child)


@check("SIGKILL limitation: the orphaned group survives; recorded identity kills it")
def sigkill(scratch):
    work, child, proc = harness(scratch, signal.SIGKILL)
    assert proc.returncode == -signal.SIGKILL
    assert not (work / "harness.json").exists(), "an uncatchable kill cannot run cleanup"
    assert alive(child), "expected the documented orphan"
    members = lifecycle.group_members(child)
    assert members and all(lifecycle._started_after(start, time.time() - 60) for _, start, _ in members)
    os.killpg(child, signal.SIGKILL)
    assert wait_gone(child)
    end = time.monotonic() + 10
    while lifecycle.group_alive(child) and time.monotonic() < end:
        time.sleep(0.05)
    assert not lifecycle.group_alive(child)


# ------------------------------------------------------------------ end to end

STANDIN = r'''#!{python} -I -B
import json, os, sys, time
args = sys.argv[1:]
def receipt(case, kind, body):
    print("EFFECTS-RECEIPT " + json.dumps({{"case": case, "kind": kind, "body": body}}), flush=True)
if args[:2] == ["--list", "--ignored"]:
    for name in json.loads(open({names!r}).read()):
        print(name + ": test")
    print("\n0 tests, 0 benchmarks")
elif args and args[0] == "--ignored":
    names = [a for a in args if not a.startswith("--")]
    print("\nrunning %d tests" % len(names))
    for name in names:
        print("test %s ... ok" % name)
    print("\ntest result: ok. %d passed; 0 failed; 0 ignored; 0 measured; 0 filtered out" % len(names))
elif args[0] == "case":
    name, out = args[1], args[2]
    os.mkdir(out)
    if name == "ssao_materials":
        time.sleep(30)
    if name == "bloom_numeric":
        print("EFFECTS-RECEIPT {{not json", flush=True)
    declared = {{"ssao_core": 20, "ssao_materials": 10, "ssao_geometry": 16, "lifecycle_drs": 18,
                "bloom_numeric": 8}}[name]
    open(os.path.join(out, "frame.rgba"), "wb").write(b"\0" * 16)
    receipt(name, "budget", {{"declared": declared, "frames": declared}})
    receipt(name, "verdict", {{"failures": []}})
elif args[0] == "appearance":
    os.mkdir(args[1])
    receipt("appearance", "budget", {{"declared": 16, "frames": 16}})
'''

E2E = r'''
import json, sys
from pathlib import Path
sys.dont_write_bytecode = True
tools, scratch, command = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
sys.path.insert(0, str(tools))
import common as c
import runner
c.EVIDENCE_ROOT = scratch
c.HELPER = scratch / "effects"
c.REQUIRE_EXTERNAL_EVIDENCE = False
plan = c.load_plan()
plan["timeouts_s"].update({"listing": 5, "numeric": 2, "appearance": 5, "retry": 5})
plan["timeouts_s"]["suite"] = {k: 5 for k in plan["timeouts_s"]["suite"]}
runner.lifecycle.install_signal_handlers()
args = runner.argparse.Namespace(command=command[0], grant=command[1], receipt=command[2] if len(command) > 3 else None,
                                 target=command[-1] if len(command) > 2 else None)
try:
    runner.COMMANDS[args.command](args, plan)
    print("E2E-OK")
except runner.Refused as refusal:
    print("E2E-REFUSED " + str(refusal))
    sys.exit(2)
'''


class Rig:
    def __init__(self, scratch):
        self.root = scratch / "e2e"
        self.evidence = self.root / "evidence"
        (self.evidence / "bin").mkdir(parents=True)
        (self.root / "effects").mkdir()
        (self.root / "effects" / "Cargo.lock").write_text("# stand-in helper lock\n")
        plan = c.load_plan()
        names = self.root / "names.json"
        names.write_text(json.dumps(plan["suites"]["lib"]["ignored"]))
        script = STANDIN.format(python=PYTHON, names=str(names))
        frozen = {}
        for name in ("lib", "model_gpu", "gpu_host-game", "gpu_host-ui", "effects_gpu"):
            path = self.evidence / "bin" / name
            path.write_text(script)
            path.chmod(0o555)
            frozen[name] = {"path": str(path), "sha256": c.sha256_file(path)}
        head = c.git("rev-parse", "HEAD").strip()
        build = {"helper_digest": c.helper_digest()[0], "source_before": {"head": head}, "frozen": frozen,
                 "helper_lock": {"helper_lock_sha256": c.sha256_file(self.root / "effects" / "Cargo.lock")}}
        (self.evidence / "build").mkdir()
        (self.evidence / "build" / "build.json").write_text(json.dumps(build))
        self.grant = self.evidence / "gpu-grant.json"
        self.grant.write_text(json.dumps({
            "format": "incant-0070-gpu-grant-v1", "packet": c.PACKET, "kind": "gpu", "source_commit": head,
            "plan_sha256": c.sha256_file(c.PLAN), "helper_digest": c.helper_digest()[0],
            "evidence_dir": str(self.evidence), "expected_backend": "Metal",
            "build_record_sha256": c.sha256_file(self.evidence / "build" / "build.json")}))

    def run(self, *command, background=False):
        argv = [PYTHON, "-I", "-B", "-c", E2E, str(TOOLS), str(self.root), *command]
        if background:
            return subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                    start_new_session=True)
        return subprocess.run(argv, capture_output=True, text=True, timeout=120)

    def ledger(self):
        return Ledger(self.evidence)


def e2e_ok(result):
    return result.returncode == 0 and "E2E-OK" in result.stdout


@check("end-to-end: listing must match before a suite; reservations precede spawns")
def e2e_suite(scratch, rig):
    refused = rig.run("suite", str(rig.grant), "lib")
    assert refused.returncode == 2 and "no matching frozen listing" in refused.stdout, refused.stdout
    assert e2e_ok(rig.run("list", str(rig.grant), "lib"))
    assert e2e_ok(rig.run("suite", str(rig.grant), "lib"))
    records = rig.ledger().records()
    suite = [r for r in records if r.get("label") == "suite-lib"]
    assert [r["type"] for r in suite] == ["reserve", "spawned", "complete"], suite
    post = suite[-1]["postprocess"]
    assert post["executed"] == post["expected"] and not post["missing"] and not post["unexpected"], post
    assert suite[0]["existing_frames"] == 425 and suite[0]["new_frames"] == 20


@check("end-to-end: helper receipts, malformed receipts and a timed-out case are recorded")
def e2e_cases(scratch, rig):
    assert e2e_ok(rig.run("case", str(rig.grant), "ssao_core"))
    timed = rig.run("case", str(rig.grant), "ssao_materials")
    assert e2e_ok(timed), timed.stdout + timed.stderr
    malformed = rig.run("case", str(rig.grant), "bloom_numeric")
    assert malformed.returncode != 0 and "Traceback" in malformed.stderr
    records = {(r["type"], r["label"]): r for r in rig.ledger().records()}
    done = records[("complete", "case-ssao_core")]["postprocess"]
    assert done["frames_within_declaration"] and done["receipts"] == 2, done
    late = records[("complete", "case-ssao_materials")]["outcome"]
    assert late["timed_out"] and late["group_absent"], late
    broken = records[("complete", "case-bloom_numeric")]
    assert broken["postprocess_error"].startswith("JSONDecodeError"), broken
    assert rig.ledger().unresolved() == []


@check("end-to-end: duplicate labels and exhausted budgets refuse without a process")
def e2e_refusals(scratch, rig):
    again = rig.run("case", str(rig.grant), "ssao_core")
    assert again.returncode == 2 and "already used" in again.stdout, again.stdout
    before = len(rig.ledger().records())
    receipt = rig.evidence / "retry.json"
    receipt.write_text(json.dumps({"format": "incant-0070-retry-v1", "target": "ssao_geometry",
                                   "original_label": "case-ssao_geometry", "approved_by": "root", "reason": "x"}))
    bad = rig.run("retry", str(rig.grant), str(receipt), "ssao_geometry")
    assert bad.returncode == 2 and "retry receipt" in bad.stdout, bad.stdout
    assert len(rig.ledger().records()) == before


@check("end-to-end: runner SIGKILL leaves an unresolved reservation; reconcile and release")
def e2e_sigkill(scratch, rig):
    receipt = rig.evidence / "retry-geometry.json"
    receipt.write_text(json.dumps({"format": "incant-0070-retry-v1", "target": "ssao_materials",
                                   "original_label": "case-ssao_materials", "approved_by": "root",
                                   "reason": "timed out in the stand-in"}))
    proc = rig.run("retry", str(rig.grant), str(receipt), "ssao_materials", background=True)
    end = time.monotonic() + 20
    spawned = []
    while not spawned and time.monotonic() < end:
        spawned = [r for r in rig.ledger().records() if r.get("type") == "spawned"
                   and r["label"].startswith("retry")]
        time.sleep(0.05)
    assert spawned, "retry child never spawned"
    os.killpg(proc.pid, signal.SIGKILL)  # runner only: its own session
    proc.wait(timeout=10)
    pgid = spawned[0]["pgid"]
    assert lifecycle.group_alive(pgid), "documented limitation: the child outlives a SIGKILLed runner"
    blocked = rig.run("case", str(rig.grant), "ssao_geometry")
    assert blocked.returncode == 2 and "unresolved" in blocked.stdout, blocked.stdout
    assert e2e_ok(rig.run("reconcile", str(rig.grant)))
    assert not lifecycle.group_alive(pgid)
    assert e2e_ok(rig.run("release", str(rig.grant)))
    after = rig.run("case", str(rig.grant), "ssao_geometry")
    assert after.returncode == 2 and "released" in after.stdout, after.stdout
    release = json.loads((rig.evidence / "released.json").read_text())
    assert release["owned_groups_alive"] == [] and release["frozen_processes"] == []


@check("grant refusals: absent, outside the evidence root, wrong kind")
def grants(scratch):
    plan = c.load_plan()
    for path in (scratch / "absent.json", Path("relative.json")):
        try:
            c.load_grant(path, "gpu", plan)
        except Refused:
            continue
        raise AssertionError(f"accepted {path}")
    outside = scratch / "outside"
    outside.mkdir()
    (outside / "g.json").write_text(json.dumps({"format": "incant-0070-gpu-grant-v1", "packet": c.PACKET,
                                                "kind": "build", "evidence_dir": str(outside)}))
    try:
        c.load_grant(outside / "g.json", "gpu", plan)
    except Refused as refusal:
        assert "kind" in str(refusal) and "evidence_dir must be under" in str(refusal), refusal
    else:
        raise AssertionError("accepted a misdirected grant")


def main():
    unit_only = "--unit" in sys.argv[1:]
    scratch = c.ROOT / c.PACKET_REL / f".selftest-{os.getpid()}"
    scratch.mkdir()
    try:
        for control in (porcelain, environment, ledger_budget, lock, normal, timeout, escalation, grandchild,
                        spawn_failure, interrupt, terminate, sigkill, grants):
            control(scratch)
        if not unit_only:
            try:
                rig = Rig(scratch)
            except BaseException as error:
                RESULTS.append({"control": "end-to-end rig", "ok": False, "error": repr(error)})
            else:
                for control in (e2e_suite, e2e_cases, e2e_refusals, e2e_sigkill):
                    control(scratch, rig)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    failed = [r for r in RESULTS if not r["ok"]]
    print(json.dumps({"controls": len(RESULTS), "failed": len(failed), "results": RESULTS}, indent=1))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
