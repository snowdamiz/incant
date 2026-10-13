#!/usr/bin/env python3
"""CPU-only lifecycle, lock, latch and reconcile controls for the 0070 runner.

Standard library only. Children are Python, sleep, touch and env stand-ins;
nothing builds, renders or touches a GPU. Scratch files live in
`handoffs/0070-ambient-occlusion/.selftest-<pid>/` (inside the worktree,
outside tools/) and are removed at the end. That scratch directory is also the
EVIDENCE_ROOT of the end-to-end checks, so the shared GPU lock they exercise is
`<scratch>/native-gpu.lock`, never the real one. End-to-end checks drive the
real runner commands with stand-in frozen binaries in several independent
evidence directories ("packets"); only the harness process repoints
EVIDENCE_ROOT and the helper lock location, and it shortens watchdogs in
memory. They require the helper files to be committed (the source freeze
refuses uncommitted helper bytes, by design).

  python3 -I -B selftest.py            # all controls
  python3 -I -B selftest.py --unit     # skip end-to-end runner checks
"""

import fcntl
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
import runner  # noqa: E402
from ledger import Ledger, Refused, exclusive  # noqa: E402

PYTHON = sys.executable
RESULTS = []
SUITES = ("lib", "model_gpu", "gpu_host-game", "gpu_host-ui")


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
    """Running and not a zombie."""
    return lifecycle.identity(pid) is not None


def wait_gone(pid, seconds=10.0):
    return wait_for(lambda: not alive(pid), seconds)


def wait_for(predicate, seconds=20.0):
    end = time.monotonic() + seconds
    while not predicate():
        if time.monotonic() >= end:
            return False
        time.sleep(0.05)
    return True


def lock_held(path):
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return False
    except BlockingIOError:
        return True
    finally:
        os.close(fd)


def run_child(scratch, name, argv, timeout, env=None, **kwargs):
    kwargs.setdefault("on_spawn", lambda *_: None)
    return lifecycle.run(argv, env=env or {"PATH": c.SYSTEM_PATH}, cwd=scratch,
                         stdout_path=scratch / f"{name}.out", stderr_path=scratch / f"{name}.err",
                         timeout_s=timeout, **kwargs)


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


@check("compiler inputs hash the resolved tools, change on a swap and refuse an unpinned release")
def toolchain(scratch):
    work, host = scratch / "toolchain", "aarch64-apple-darwin"
    for relative in ("bin/rustup", "bin/cargo", "bin/rustc", "tools/rustc", "tools/cargo", "tools/clippy-driver",
                     "tools/cargo-clippy", "tools/rustfmt", "sysroot/lib/librustc_driver.dylib",
                     f"sysroot/lib/rustlib/{host}/lib/libstd.rlib"):
        (work / relative).parent.mkdir(parents=True, exist_ok=True)
        (work / relative).write_bytes(relative.encode())
    pinned = c.tomllib.loads((c.ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    release = {"value": pinned}

    def run_text(argv, env, cwd=c.ROOT):  # stands in for rustup/rustc/cargo; nothing is executed
        if argv[1:2] == ["which"]:
            return str(work / "tools" / argv[2])
        if argv[1:] == ["--print", "sysroot"]:
            return str(work / "sysroot")
        if argv[1:] == ["-vV"]:
            return f"rustc {release['value']}\nhost: {host}\nrelease: {release['value']}"
        if argv[1:] == ["-V"]:
            return f"cargo {release['value']}"
        raise AssertionError(argv)

    saved = c._run_text, c.RUSTUP_BIN
    c._run_text, c.RUSTUP_BIN = run_text, str(work / "bin")
    try:
        record = c.compiler_inputs({})
        assert record["pinned_channel"] == pinned and record["host"] == host, record
        assert set(record["rustup_proxies"]) == {"rustup", "cargo", "rustc"}, record
        assert len(record["sysroot_libraries"]) == 2, record["sysroot_libraries"]
        assert c.compiler_inputs({}) == record
        (work / "tools" / "clippy-driver").write_bytes(b"swapped")
        assert c.compiler_inputs({}) != record, "a swapped tool must change the record"
        release["value"] = "0.0.0"
        try:
            c.compiler_inputs({})
        except Refused:
            pass
        else:
            raise AssertionError("an unpinned rustc release was accepted")
    finally:
        c._run_text, c.RUSTUP_BIN = saved


@check("ledger ceilings (8192 existing, 128 new, 8320 combined) refuse before any reservation")
def ledger_budget(scratch):
    directory = scratch / "ledger"
    directory.mkdir()
    ledger = Ledger(directory)
    budgets = c.load_plan()["budgets"]
    assert (budgets["existing_frames_max"], budgets["new_frames_max"], budgets["combined_frames_max"]) == \
        (8192, 128, 8320), budgets
    with ledger.locked():
        ledger.check_and_reserve(budgets, {"label": "a", "kind": "suite", "existing_frames": 8100,
                                           "new_frames": 120, "timeout_s": 10})
        for request in ({"label": "b", "existing_frames": 93}, {"label": "c", "new_frames": 9},
                        {"label": "d", "pngs": 33}, {"label": "a"}, {"label": "e", "timeout_s": 3000}):
            try:
                ledger.check_and_reserve(budgets, {"kind": "numeric", "timeout_s": 10, **request})
            except Refused:
                continue
            raise AssertionError(f"accepted {request}")
        assert ledger.totals()["existing_frames"] == 8100 and ledger.totals()["new_frames"] == 120
        assert [r["label"] for r in ledger.records() if r["type"] == "reserve"] == ["a"]


@check("blocked and latched state: only a verified group unblocks; only an explicit success avoids the latch")
def ledger_state(scratch):
    directory = scratch / "state"
    directory.mkdir()
    ledger = Ledger(directory)
    verified = {"group_absent": True, "survivors": [], "leader_unreaped": False}
    with ledger.locked():
        for label in ("ok", "failed", "lost", "survivor", "implicit", "build-x"):
            ledger.append({"type": "reserve", "label": label, "kind": "build" if label == "build-x" else "numeric"})
        ledger.append({"type": "complete", "label": "ok", "success": True, "outcome": verified})
        ledger.append({"type": "complete", "label": "failed", "success": False,
                       "failure_reasons": ["exit status 1"], "outcome": verified})
        ledger.append({"type": "complete", "label": "survivor", "success": False,
                       "outcome": {**verified, "group_absent": False, "survivors": [{"pid": 1, "start": [1, 2]}]}})
        ledger.append({"type": "complete", "label": "implicit", "outcome": verified})
        ledger.append({"type": "complete", "label": "build-x", "success": False, "outcome": verified})
        ledger.append({"type": "reconcile", "label": "survivor", "resolved": False})
    assert sorted(ledger.blocked()) == ["lost", "survivor"], sorted(ledger.blocked())
    assert sorted(ledger.failures()) == ["failed", "implicit", "lost", "survivor"], ledger.failures()
    with ledger.locked():
        ledger.append({"type": "reconcile", "label": "survivor", "resolved": True})
        ledger.append({"type": "reconcile", "label": "lost", "resolved": True})
    assert ledger.blocked() == {} and "lost" in ledger.failures()


@check("packet and shared locks are exclusive, never wait, and release on close")
def locks(scratch):
    directory = scratch / "lock"
    directory.mkdir()
    first, second = Ledger(directory), Ledger(directory)
    with first.locked():
        try:
            with second.locked():
                raise AssertionError("second packet lock acquired")
        except Refused:
            pass
    with second.locked():
        pass
    shared = scratch / "shared.lock"
    with exclusive(shared, "the shared native GPU lock"):
        assert lock_held(shared)
        try:
            with exclusive(shared, "the shared native GPU lock"):
                raise AssertionError("second shared lock acquired")
        except Refused as refusal:
            assert "shared native GPU lock" in str(refusal), refusal
    assert not lock_held(shared)


@check("exact identity: microsecond start time and uid; zombies have none; groups list zombies")
def identity(_scratch):
    me = lifecycle.identity(os.getpid())
    assert me["pid"] == os.getpid() and me["pgid"] == os.getpgid(0) and me["uid"] == os.getuid(), me
    assert lifecycle.same_process(lifecycle.identity(os.getpid()), me)
    assert not lifecycle.same_process(me, dict(me, start=[me["start"][0], me["start"][1] + 1]))
    child = subprocess.Popen(["/bin/sleep", "0.2"], start_new_session=True)
    os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOWAIT)
    assert lifecycle.identity(child.pid) is None and child.pid in lifecycle.group_pids(child.pid)
    child.wait()
    assert lifecycle.group_pids(child.pid) == []


@check("normal exit: exact environment through the gate, recorded leader identity, group absent")
def normal(scratch):
    env = {"PATH": c.SYSTEM_PATH, "LC_ALL": "C", "SELFTEST_VALUE": "a b=c"}
    spawned = []
    out = run_child(scratch, "normal", ["/usr/bin/env"], 10, env=env,
                    on_spawn=lambda pid, pgid, ident: spawned.append((pid, pgid, ident)))
    printed = dict(line.split("=", 1) for line in (scratch / "normal.out").read_text().splitlines())
    assert printed == env, printed
    assert out["gate_opened"] and out["exit"] == 0 and out["signals_sent"] == [] and out["group_absent"], out
    assert out["reaped"] and spawned and spawned[0][2] == out["leader"] and spawned[0][0] == out["pid"], out


@check("timeout: SIGTERM while the leader is unreaped, then reap; group absent")
def timeout(scratch):
    started = time.monotonic()
    out = run_child(scratch, "timeout", [PYTHON, "-I", "-c", "import time; time.sleep(30)"], 0.5)
    assert out["timed_out"] and out["signal"] == "SIGTERM" and out["group_absent"], out
    assert time.monotonic() - started < 6 and not alive(out["pid"])


@check("SIGTERM ignored: escalates to SIGKILL after the grace period")
def escalation(scratch):
    code = "import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print('ready', flush=True); time.sleep(30)"
    out = run_child(scratch, "escalate", [PYTHON, "-I", "-c", code], 1.0, grace_s=0.5)
    assert out["signals_sent"][:2] == ["SIGTERM", "SIGKILL"] and out["signal"] == "SIGKILL", out
    assert out["group_absent"], out


@check("a grandchild left behind is recorded, then killed before the leader is reaped")
def grandchild(scratch):
    code = "import subprocess; p = subprocess.Popen(['/bin/sleep', '30']); print(p.pid, flush=True)"
    out = run_child(scratch, "grandchild", [PYTHON, "-I", "-c", code], 10)
    pid = int((scratch / "grandchild.out").read_text().split()[0])
    assert out["exit"] == 0 and "SIGKILL" in out["signals_sent"] and out["group_absent"] and out["reaped"], out
    assert pid in [m["pid"] for m in out["members_seen"]], out["members_seen"]
    assert wait_gone(pid), f"grandchild {pid} survived"


@check("an on_spawn failure never opens the gate: the program never runs; the group is gone")
def spawn_failure(scratch):
    outcome, marker = {}, scratch / "spawn-executed"

    def fail(pid, pgid, ident):
        raise OSError("simulated ledger write failure")

    try:
        lifecycle.run(["/usr/bin/touch", str(marker)], env={"PATH": c.SYSTEM_PATH}, cwd=scratch,
                      stdout_path=scratch / "spawn.out", stderr_path=scratch / "spawn.err",
                      timeout_s=10, on_spawn=fail, outcome=outcome)
    except OSError:
        pass
    else:
        raise AssertionError("on_spawn failure was swallowed")
    assert outcome["interrupted"].startswith("OSError") and not outcome["gate_opened"], outcome
    assert outcome["group_absent"] and not marker.exists(), outcome


HARNESS = r'''
import fcntl, json, os, sys, time
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import lifecycle
work, mode = Path(sys.argv[2]), sys.argv[3]
lifecycle.install_signal_handlers()
lock = os.open(work / "shared.lock", os.O_RDWR | os.O_CREAT, 0o600)
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
def on_spawn(pid, pgid, ident):
    (work / "spawned.json").write_text(json.dumps({"pid": pid, "identity": ident}))
    if mode == "hold-gate":
        time.sleep(60)
if mode == "hold-gate":
    argv = ["/usr/bin/touch", str(work / "executed")]
else:
    argv = ["/bin/sh", "-c", "/bin/sleep 60 & exec /bin/sleep 60"]
outcome = {}
try:
    lifecycle.run(argv, env={"PATH": "/usr/bin:/bin"}, cwd=work, stdout_path=work / "h.out",
                  stderr_path=work / "h.err", timeout_s=60, on_spawn=on_spawn, inherit_fds=(lock,),
                  outcome=outcome)
    status = "completed"
except BaseException as error:
    status = type(error).__name__
(work / "harness.json").write_text(json.dumps({"status": status, "outcome": outcome}))
'''


def harness(scratch, sig, mode="run"):
    work = scratch / f"harness-{mode}-{sig.name}"
    work.mkdir()
    proc = subprocess.Popen([PYTHON, "-I", "-B", "-c", HARNESS, str(TOOLS), str(work), mode],
                            start_new_session=True)
    assert wait_for(lambda: (work / "spawned.json").exists(), 10), "harness never spawned"
    spawned = json.loads((work / "spawned.json").read_text())
    time.sleep(0.3)
    os.kill(proc.pid, sig)
    proc.wait(timeout=20)
    return work, spawned, proc


@check("interrupted wait: SIGINT to the runner kills the group before the leader is reaped")
def interrupt(scratch):
    work, spawned, _ = harness(scratch, signal.SIGINT)
    report = json.loads((work / "harness.json").read_text())
    assert report["status"] == "KeyboardInterrupt", report
    out = report["outcome"]
    assert out["group_absent"] and out["reaped"] and "SIGTERM" in out["signals_sent"], out
    assert wait_gone(spawned["pid"]) and not lock_held(work / "shared.lock")


@check("SIGTERM to the runner is converted and cleans up the same way")
def terminate(scratch):
    work, spawned, _ = harness(scratch, signal.SIGTERM)
    report = json.loads((work / "harness.json").read_text())
    assert report["status"] == "Terminated", report
    assert report["outcome"]["group_absent"] and report["outcome"]["reaped"], report
    assert wait_gone(spawned["pid"]) and not lock_held(work / "shared.lock")


@check("runner SIGKILL: the orphan is exactly the recorded process and keeps the shared lock")
def sigkill(scratch):
    work, spawned, proc = harness(scratch, signal.SIGKILL)
    try:
        assert proc.returncode == -signal.SIGKILL and not (work / "harness.json").exists()
        orphan = lifecycle.identity(spawned["pid"])
        assert lifecycle.same_process(orphan, spawned["identity"]), (orphan, spawned)
        assert orphan["comm"] != spawned["identity"]["comm"], "exec replaced the trampoline but kept its identity"
        assert lock_held(work / "shared.lock"), "the orphan must keep the inherited shared lock"
    finally:
        if lifecycle.same_process(lifecycle.identity(spawned["pid"]), spawned["identity"]):
            os.killpg(spawned["pid"], signal.SIGKILL)  # proven by exact identity
    assert wait_gone(spawned["pid"]) and wait_for(lambda: not lifecycle.group_pids(spawned["pid"]), 10)
    assert wait_for(lambda: not lock_held(work / "shared.lock"), 10)


@check("runner SIGKILL before the gate opens: the program never executes")
def gate_sigkill(scratch):
    work, spawned, proc = harness(scratch, signal.SIGKILL, mode="hold-gate")
    assert proc.returncode == -signal.SIGKILL
    assert wait_gone(spawned["pid"]), "the gated trampoline must exit once its runner is gone"
    assert not (work / "executed").exists(), "the program ran without a durable identity"
    assert wait_for(lambda: not lock_held(work / "shared.lock"), 10)


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


@check("only an explicit success counts: flawed exits, tests, verdicts or frame receipts are failures")
def classification(scratch):
    work = scratch / "classify"
    work.mkdir()
    good = {"gate_opened": True, "exit": 0, "signal": None, "timed_out": False, "interrupted": None,
            "group_absent": True, "survivors": [], "leader_unreaped": False}
    valid = {"postprocess": {"valid": True}}
    assert runner.assess(good, valid, True) == []
    for flaw in ({"exit": 1}, {"exit": None, "signal": "SIGKILL"}, {"timed_out": True},
                 {"interrupted": "KeyboardInterrupt"}, {"group_absent": False}, {"survivors": [{"pid": 1}]},
                 {"gate_opened": False}, {"leader_unreaped": True}, {"cleanup_error": "x"},
                 {"observe_errors": ["x"]}):
        assert runner.assess({**good, **flaw}, valid, True), flaw
    for completion in ({}, {"postprocess": {"valid": False, "problems": ["x"]}}, {"postprocess_error": "x", **valid},
                       {"binary_unchanged": False, **valid}, {"error": "OSError: x", **valid}):
        assert runner.assess(good, completion, True), completion
    counter = iter(range(10_000))

    def stdout(text):
        path = work / f"out-{next(counter)}.txt"
        path.write_text(text)
        return path

    tests = ["m::a", "m::b"]
    suite = runner.suite_postprocess(tests)
    passing = ("\nrunning 2 tests\ntest m::a ... ok\ntest m::b ... ok\n\n"
               "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.01s\n")
    assert suite(good, stdout(passing), None)["valid"]
    for broken in (passing.replace("test m::b ... ok\n", ""), passing.replace("m::b ... ok", "m::b ... FAILED"),
                   passing.replace("running 2", "running 3"), passing.replace("ok. 2 passed", "ok. 1 passed"),
                   passing + "test m::b ... ok\n", "", passing.replace("test m::a", "test m::z")):
        assert not suite(good, stdout(broken), None)["valid"], broken
    listed = "m::a: test\nm::b: test\n\n2 tests, 0 benchmarks\n"
    variants = [listed, listed.replace("m::b: test\n", ""), listed.replace("2 tests", "3 tests"),
                listed.replace("0 benchmarks", "1 benchmark"), "m::a: test\n" + listed]
    for n, text in enumerate(variants):
        result = runner.listing_postprocess(work, "lib", tests, f"list-{n}")(good, stdout(text), None)
        assert result["valid"] is (n == 0), (n, result)

    def helper(n, mutate=None, pngs=0):
        out = work / f"helper-{n}"
        out.mkdir()
        data = bytes(16)
        (out / "f0.rgba").write_bytes(data)
        index = {"frames": [{"label": "f0", "file": "f0.rgba", "size": [2, 2], "rgba_sha256": c.sha256_bytes(data)}]}
        receipts = [{"case": "ssao_core", "kind": "budget", "body": {"declared": 3, "frames": 3,
                                                                     "labels": ["a", "b", "c"]}},
                    {"case": "ssao_core", "kind": "verdict", "body": {"failures": [], "checks": [{"ok": True}]}}]
        extra = []
        if mutate is not None:
            index, receipts, extra = mutate(out, index, receipts, extra)
        (out / "index.json").write_text(json.dumps(index))
        text = "\n".join([runner.RECEIPT + json.dumps(r) for r in receipts] + extra) + "\n"
        return runner.helper_postprocess(work, "ssao_core", 3, pngs, out, [960, 540])(good, stdout(text), None)

    def unlisted(out, index, receipts, extra):
        (out / "extra.bin").write_bytes(b"x")
        return index, receipts, extra

    budget = lambda r, **body: dict(r, body=dict(r["body"], **body))  # noqa: E731
    mutations = [
        lambda o, i, r, e: (i, r[:1], e),                                                    # no verdict
        lambda o, i, r, e: (i, r[:1] + [dict(r[1], body={"failures": ["x"], "checks": [{"ok": False}]})], e),
        lambda o, i, r, e: (i, r[:1] + [dict(r[1], body={"failures": [], "checks": []})], e),  # no checks
        lambda o, i, r, e: (i, [budget(r[0], frames=4, labels=list("abcd"))] + r[1:], e),     # over declared
        lambda o, i, r, e: (i, [budget(r[0], labels=["a"])] + r[1:], e),                      # label count
        lambda o, i, r, e: (i, [budget(r[0], declared=2)] + r[1:], e),                        # wrong declaration
        lambda o, i, r, e: (i, r, e + [runner.RECEIPT + "{not json"]),                        # malformed receipt
        lambda o, i, r, e: (i, r + [r[0]], e),                                                # two budgets
        lambda o, i, r, e: (i, r + [dict(r[1], case="ssao_materials")], e),                   # another case
        lambda o, i, r, e: ({"frames": [dict(i["frames"][0], rgba_sha256="0" * 64)]}, r, e),  # hash mismatch
        lambda o, i, r, e: ({"frames": [dict(i["frames"][0], size=[3, 2])]}, r, e),           # byte length
        lambda o, i, r, e: ({"frames": []}, r, e),                                            # empty index
        unlisted,                                                                             # unlisted file
    ]
    assert helper(0)["valid"], helper.__name__
    for n, mutate in enumerate(mutations, 1):
        result = helper(n, mutate)
        assert not result["valid"], (n, result)
    assert not helper(len(mutations) + 1, pngs=1)["valid"]  # PNG count must match the plan


@check("release sees a frozen binary still running from the evidence directory")
def frozen(scratch):
    evidence = scratch / "frozen-evidence"
    (evidence / "bin").mkdir(parents=True)
    copy = evidence / "bin" / "sleep"
    shutil.copyfile("/bin/sleep", copy)
    copy.chmod(0o555)
    proc = subprocess.Popen([str(copy), "30"], start_new_session=True)
    try:
        assert wait_for(lambda: any(p["pid"] == proc.pid for p in runner.frozen_processes(evidence)), 5)
    finally:
        proc.kill()
        proc.wait()
    assert not any(p["pid"] == proc.pid for p in runner.frozen_processes(evidence))


# ------------------------------------------------------------------ end to end

STANDIN = r'''#!@PYTHON@ -I -B
import hashlib, json, os, sys, time
ROOT = @ROOT@
DECLARED = @DECLARED@
args, name = sys.argv[1:], os.path.basename(sys.argv[0])


def marker(name):
    return os.path.join(ROOT, name)


def receipt(case, kind, body):
    print("EFFECTS-RECEIPT " + json.dumps({"case": case, "kind": kind, "body": body}), flush=True)


def plural(n, word):
    return "%d %s%s" % (n, word, "" if n == 1 else "s")


if args[:2] == ["--list", "--ignored"]:
    names = json.load(open(marker("names-" + name + ".json")))
    for test in names:
        print(test + ": test")
    print()
    print(plural(len(names), "test") + ", 0 benchmarks")
elif args and args[0] == "--ignored":
    tests = [a for a in args if not a.startswith("--")]
    failing = open(marker("fail-test")).read().strip() if os.path.exists(marker("fail-test")) else None
    print()
    print("running " + plural(len(tests), "test"))
    for test in tests:
        print("test %s ... %s" % (test, "FAILED" if test == failing else "ok"))
    bad = 1 if failing in tests else 0
    print()
    print("test result: %s. %d passed; %d failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s"
          % ("FAILED" if bad else "ok", len(tests) - bad, bad))
    sys.exit(101 if bad else 0)
else:
    case = args[1] if args[0] == "case" else "appearance"
    out = args[-1]
    if os.path.exists(marker("hold-" + case)):
        open(marker("held-" + case), "w").close()
        end = time.time() + 30
        while not os.path.exists(marker("proceed-" + case)) and time.time() < end:
            time.sleep(0.05)
    os.mkdir(out)
    entries = []
    if case == "appearance":
        for i in range(16):
            label = "v%d_%d" % (i // 4 + 1, i % 4)
            data = b"stand-in png %d" % i
            open(os.path.join(out, label + ".png"), "wb").write(data)
            entries.append({"label": label, "file": label + ".png", "size": [959, 539],
                            "rgba_sha256": "0" * 64, "png_sha256": hashlib.sha256(data).hexdigest(), "meta": {}})
    else:
        data = bytes(16)
        open(os.path.join(out, "f0.rgba"), "wb").write(data)
        entries.append({"label": "f0", "file": "f0.rgba", "size": [2, 2],
                        "rgba_sha256": hashlib.sha256(data).hexdigest(), "meta": {}})
    with open(os.path.join(out, "index.json"), "w") as index:
        json.dump({"frames": entries, "summary": {}}, index)
    declared = DECLARED[case]
    receipt(case, "budget", {"declared": declared, "frames": declared,
                             "labels": ["frame%d" % i for i in range(declared)]})
    if case == "ssao_geometry" and not os.path.exists(marker("repaired")):
        sys.exit(0)  # exit 0 without a verdict must never count as a success
    receipt(case, "verdict", {"failures": [], "checks": [{"check": "stand-in", "ok": True}]})
'''

E2E = r'''
import sys
from pathlib import Path
sys.dont_write_bytecode = True
tools, evidence_root, rig_root, argv = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4:]
sys.path.insert(0, str(tools))
import common as c
import runner
c.EVIDENCE_ROOT = evidence_root
c.HELPER = rig_root / "effects"
c.REQUIRE_EXTERNAL_EVIDENCE = False
plan = c.load_plan()
plan["timeouts_s"].update({"listing": 10, "numeric": 10, "appearance": 10, "retry": 10})
plan["timeouts_s"]["suite"] = {k: 10 for k in plan["timeouts_s"]["suite"]}
runner.lifecycle.install_signal_handlers()
sys.exit(runner.run_command(argv, plan))
'''

LOCK_HOLDER = ("import fcntl, os, sys, time\n"
               "fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT, 0o600)\n"
               "fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)\n"
               "print('held', flush=True)\n"
               "time.sleep(60)\n")


class Rig:
    """One independent evidence directory ("packet") with stand-in binaries."""

    def __init__(self, scratch, name):
        self.scratch, self.root = scratch, scratch / f"rig-{name}"
        self.evidence = self.root / "evidence"
        (self.evidence / "bin").mkdir(parents=True)
        (self.root / "effects").mkdir()
        (self.root / "effects" / "Cargo.lock").write_text("# stand-in helper lock\n")
        plan = c.load_plan()
        for suite in SUITES:
            (self.root / f"names-{suite}.json").write_text(json.dumps(plan["suites"][suite]["ignored"]))
        declared = {case["name"]: case["frames"] for case in plan["numeric_cases"]}
        declared["appearance"] = plan["appearance"]["renders"]
        script = (STANDIN.replace("@PYTHON@", PYTHON).replace("@ROOT@", json.dumps(str(self.root)))
                  .replace("@DECLARED@", json.dumps(declared)))
        frozen = {}
        for binary in (*SUITES, "effects_gpu"):
            path = self.evidence / "bin" / binary
            path.write_text(script)
            path.chmod(0o555)
            frozen[binary] = {"path": str(path), "sha256": c.sha256_file(path)}
        self.head = c.git("rev-parse", "HEAD").strip()
        build = {"helper_digest": c.helper_digest()[0], "source_before": {"head": self.head}, "frozen": frozen,
                 "helper_lock": {"helper_lock_sha256": c.sha256_file(self.root / "effects" / "Cargo.lock")}}
        (self.evidence / "build").mkdir()
        (self.evidence / "build" / "build.json").write_text(json.dumps(build))
        self.grant = self.evidence / "gpu-grant.json"
        self.grant.write_text(json.dumps({
            "format": "incant-0070-gpu-grant-v1", "packet": c.PACKET, "kind": "gpu", "source_commit": self.head,
            "plan_sha256": c.sha256_file(c.PLAN), "helper_digest": c.helper_digest()[0],
            "evidence_dir": str(self.evidence), "expected_backend": "Metal",
            "build_record_sha256": c.sha256_file(self.evidence / "build" / "build.json")}))

    def gpu(self, command, target=None, receipt=None, background=False):
        argv = [command, "--grant", str(self.grant), *([target] if target else []),
                *(["--receipt", str(receipt)] if receipt else [])]
        full = [PYTHON, "-I", "-B", "-c", E2E, str(TOOLS), str(self.scratch), str(self.root), *argv]
        if background:
            return subprocess.Popen(full, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                    start_new_session=True)
        return subprocess.run(full, capture_output=True, text=True, timeout=120)

    def ledger(self):
        return Ledger(self.evidence)

    def children(self):
        try:
            return self.ledger().children()
        except Refused:  # a line being appended concurrently; the caller polls again
            return {}

    def count(self):
        return len(self.ledger().records())

    def hold(self, case):
        (self.root / f"hold-{case}").touch()

    def wait_held(self, case):
        label = f"case-{case}"
        assert wait_for(lambda: (self.root / f"held-{case}").exists()
                        and self.children().get(label, {}).get("spawned")), f"{case} never started"
        return self.children()[label]["spawned"][0]

    def proceed(self, case):
        (self.root / f"proceed-{case}").touch()

    def receipt(self, name, **fields):
        path = self.evidence / f"{name}.json"
        path.write_text(json.dumps({"format": "incant-0070-retry-v2", "packet": c.PACKET, "approved_by": "root",
                                    "source_commit": self.head, **fields}))
        return path


def ok(result):
    return result.returncode == 0


def refused(result, text):
    return result.returncode == 2 and text in result.stderr


@check("end-to-end: listing before suite; spawned identity; explicit successes; nothing replaced")
def e2e_main(_scratch, rigs):
    rig = rigs["main"]
    first = rig.gpu("suite", "lib")
    assert refused(first, "no successful matching listing"), first.stderr
    for command in (("list", "lib"), ("suite", "lib"), ("case", "ssao_core"), ("appearance",)):
        result = rig.gpu(*command)
        assert ok(result), (command, result.stderr)
    children = rig.children()
    suite = children["suite-lib"]
    assert suite["reserve"]["existing_frames"] == 425 and suite["reserve"]["new_frames"] == 20, suite["reserve"]
    assert suite["spawned"][0]["identity"]["pid"] == suite["spawned"][0]["pid"]
    assert all(children[label]["complete"]["success"] is True
               for label in ("list-lib", "suite-lib", "case-ssao_core", "appearance"))
    assert children["appearance"]["complete"]["postprocess"]["index"]["pngs"] == 16
    before = rig.count()
    again = rig.gpu("case", "ssao_core")
    assert again.returncode == 2 and ("exists" in again.stderr or "already used" in again.stderr), again.stderr
    assert rig.count() == before


@check("two packets: one refuses before any reservation while the other owns the shared GPU lock")
def e2e_two_packets(scratch, rigs):
    a, b = rigs["main"], rigs["other"]
    a.hold("lifecycle_drs")
    proc = a.gpu("case", "lifecycle_drs", background=True)
    try:
        a.wait_held("lifecycle_drs")
        result = b.gpu("list", "lib")
        assert refused(result, "shared native GPU lock"), result.stderr
        assert not (b.evidence / "ledger.jsonl").exists() and not (b.evidence / "runner.lock").exists(), \
            "packet B must refuse before its own lock and before any reservation"
    finally:
        a.proceed("lifecycle_drs")
        _, err = proc.communicate(timeout=60)
    assert proc.returncode == 0, err
    assert ok(b.gpu("list", "lib"))
    holder = subprocess.Popen([PYTHON, "-I", "-c", LOCK_HOLDER, str(scratch / "native-gpu.lock")],
                              stdout=subprocess.PIPE, text=True, start_new_session=True)
    try:
        assert holder.stdout.readline().strip() == "held"
        before = a.count()
        result = a.gpu("case", "ssao_materials")
        assert refused(result, "shared native GPU lock") and a.count() == before, result.stderr
    finally:
        holder.kill()
        holder.wait()
    assert ok(a.gpu("case", "ssao_materials"))


@check("a missing verdict latches the packet; only a precise root-receipted retry may run")
def e2e_latch(scratch, rigs):
    rig = rigs["latch"]
    failed = rig.gpu("case", "ssao_geometry")
    assert failed.returncode == 1, failed.stderr
    done = rig.children()["case-ssao_geometry"]["complete"]
    assert done["success"] is False and done["outcome"]["exit"] == 0, done
    assert any("verdict" in reason for reason in done["failure_reasons"]), done["failure_reasons"]
    before = rig.count()
    assert refused(rig.gpu("case", "ssao_core"), "latched failure")
    good = dict(original_label="case-ssao_geometry", kind="numeric", target="ssao_geometry",
                reason="stand-in verdict restored")
    for n, fields in enumerate((dict(good, original_label="case-ssao_core"), dict(good, target="ssao_core"),
                                dict(good, kind="suite"), dict(good, approved_by="astra"),
                                dict(good, format="incant-0070-retry-v1"), dict(good, reason=""),
                                dict(good, source_commit="0" * 40))):
        result = rig.gpu("retry", receipt=rig.receipt(f"bad-{n}", **fields))
        assert result.returncode == 2, (fields, result.stderr)
    outside = scratch / "retry-outside.json"
    outside.write_text(rig.receipt("copied", **good).read_text())
    assert refused(rig.gpu("retry", receipt=outside), "directly in the evidence directory")
    assert rig.count() == before, "imprecise receipts must not reserve anything"
    (rig.root / "repaired").touch()
    path = rig.receipt("good", **good)
    assert ok(rig.gpu("retry", receipt=path))
    retry = rig.children()["retry-case-ssao_geometry"]
    assert retry["complete"]["success"] is True and retry["reserve"]["retry_of"] == "case-ssao_geometry", retry
    assert refused(rig.gpu("retry", receipt=path), "already retried")
    assert refused(rig.gpu("case", "ssao_core"), "latched failure"), "a passing retry does not unlatch"


@check("a failed suite test latches; only that exact test may be retried")
def e2e_suite_retry(_scratch, rigs):
    rig, plan = rigs["suite"], c.load_plan()
    names = plan["suites"]["lib"]["run"]
    assert ok(rig.gpu("list", "lib"))
    (rig.root / "fail-test").write_text(names[0])
    failed = rig.gpu("suite", "lib")
    assert failed.returncode == 1, failed.stderr
    post = rig.children()["suite-lib"]["complete"]["postprocess"]
    assert post["failed"] == [names[0]] and post["valid"] is False, post
    before = rig.count()
    for n, tests in enumerate(([names[1]], [], [names[0], names[0]], None)):
        fields = dict(original_label="suite-lib", kind="suite", target="lib", reason="flake check")
        if tests is not None:
            fields["tests"] = tests
        result = rig.gpu("retry", receipt=rig.receipt(f"bad-suite-{n}", **fields))
        assert refused(result, "imprecise"), (tests, result.stderr)
    assert rig.count() == before
    (rig.root / "fail-test").unlink()
    good = rig.receipt("good-suite", original_label="suite-lib", kind="suite", target="lib", tests=[names[0]],
                       reason="re-run the failed test only")
    assert ok(rig.gpu("retry", receipt=good))
    retry = rig.children()["retry-suite-lib"]
    entry = plan["suites"]["lib"]
    frames = (entry["existing"].get(names[0]) or entry["new_in_suite"][names[0]])["frames"]
    assert retry["reserve"]["tests"] == [names[0]], retry["reserve"]
    assert retry["reserve"]["existing_frames"] + retry["reserve"]["new_frames"] == frames, retry["reserve"]
    assert retry["complete"]["success"] is True
    assert refused(rig.gpu("case", "ssao_core"), "latched failure")


@check("runner SIGKILL: the orphan keeps the shared lock; reconcile proves and stops it; the latch stays")
def e2e_sigkill(_scratch, rigs):
    rig, other = rigs["kill"], rigs["other"]
    rig.hold("lifecycle_drs")
    proc = rig.gpu("case", "lifecycle_drs", background=True)
    spawned = rig.wait_held("lifecycle_drs")
    try:
        os.killpg(proc.pid, signal.SIGKILL)  # the runner only: the child has its own session
        proc.wait(timeout=10)
        assert lifecycle.same_process(lifecycle.identity(spawned["pid"]), spawned["identity"]), spawned
        assert refused(other.gpu("list", "model_gpu"), "shared native GPU lock"), \
            "a second packet must not render beside the orphan"
        before = rig.count()
        assert rig.gpu("case", "ssao_core").returncode == 2 and rig.count() == before
        reconciled = rig.gpu("reconcile")
        assert ok(reconciled), reconciled.stderr
        result = rig.children()["case-lifecycle_drs"]["reconcile"][-1]
        assert result["resolved"] and result["signals"][:1] == ["SIGTERM"], result
        assert wait_gone(spawned["pid"])
    finally:
        rig.proceed("lifecycle_drs")
    assert ok(other.gpu("list", "model_gpu")), "the shared lock is free once the orphan is gone"
    assert refused(rig.gpu("case", "ssao_core"), "latched failure")
    assert ok(rig.gpu("release"))
    release = json.loads((rig.evidence / "released.json").read_text())
    assert "case-lifecycle_drs" in release["latched_failures"] and release["owned_processes_alive"] == []
    assert refused(rig.gpu("case", "ssao_core"), "released")


@check("reconcile never signals a recycled group: start-time order alone is not ownership")
def e2e_recycled(_scratch, rigs):
    rig = rigs["foreign"]
    ledger = rig.ledger()
    with ledger.locked():
        reserve = ledger.append({"type": "reserve", "label": "case-ssao_core", "kind": "numeric",
                                 "target": "ssao_core", "new_frames": 20, "timeout_s": 10})
    time.sleep(0.05)
    stranger = subprocess.Popen([PYTHON, "-I", "-c", "import time; time.sleep(60)"], start_new_session=True)
    try:
        ident = lifecycle.identity(stranger.pid)
        assert ident["start"][0] + ident["start"][1] / 1e6 > reserve["at_epoch"], "must start after the reservation"
        recorded = dict(ident, start=[ident["start"][0], ident["start"][1] + 1])  # same number, other process
        with ledger.locked():
            ledger.append({"type": "spawned", "label": "case-ssao_core", "pid": stranger.pid, "pgid": stranger.pid,
                           "identity": recorded})
        reconciled = rig.gpu("reconcile")
        assert refused(reconciled, "unproven"), reconciled.stderr
        assert lifecycle.same_process(lifecycle.identity(stranger.pid), ident), "an unrelated group was signalled"
        result = rig.children()["case-ssao_core"]["reconcile"][-1]
        assert result["resolved"] is False and result["signals"] == [] and result["unproven"], result
        for command in (("case", "ssao_materials"), ("release",)):
            assert refused(rig.gpu(*command), "never verified absent"), command
    finally:
        stranger.kill()
        stranger.wait()
    assert ok(rig.gpu("reconcile"))
    final = rig.children()["case-ssao_core"]["reconcile"][-1]
    assert final["resolved"] and final["signals"] == [], final
    assert refused(rig.gpu("case", "ssao_materials"), "latched failure")


@check("survivors keep every child refused until reconcile proves and stops them")
def e2e_survivors(_scratch, rigs):
    rig = rigs["survivor"]
    code = "import subprocess, sys; p = subprocess.Popen(['/bin/sleep', '60']); print(p.pid, flush=True); sys.stdin.readline()"
    leader = subprocess.Popen([PYTHON, "-I", "-c", code], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                              start_new_session=True)
    member = int(leader.stdout.readline())
    try:
        leader_ident, survivor = lifecycle.identity(leader.pid), lifecycle.identity(member)
        assert survivor["pgid"] == leader.pid, survivor
        leader.stdin.close()
        leader.wait(timeout=10)  # the leader is reaped; its member survives in the group
        wrong = dict(survivor, start=[survivor["start"][0], survivor["start"][1] + 1])
        ledger = rig.ledger()
        with ledger.locked():
            ledger.append({"type": "reserve", "label": "case-bloom_numeric", "kind": "numeric",
                           "target": "bloom_numeric", "new_frames": 8, "timeout_s": 10})
            ledger.append({"type": "spawned", "label": "case-bloom_numeric", "pid": leader.pid, "pgid": leader.pid,
                           "identity": leader_ident})
            ledger.append({"type": "complete", "label": "case-bloom_numeric", "success": False,
                           "failure_reasons": ["process group not verified absent"],
                           "outcome": {"group_absent": False, "reaped": True, "survivors": [wrong]}})
        assert refused(rig.gpu("case", "ssao_core"), "never verified absent")
        assert refused(rig.gpu("reconcile"), "unproven")
        assert lifecycle.same_process(lifecycle.identity(member), survivor), "an unproven survivor was signalled"
        assert refused(rig.gpu("case", "ssao_core"), "never verified absent"), "a failed reconcile must not unblock"
        with ledger.locked():
            ledger.append({"type": "member", "label": "case-bloom_numeric", "identity": survivor})
        assert ok(rig.gpu("reconcile"))
        result = rig.children()["case-bloom_numeric"]["reconcile"][-1]
        assert result["resolved"] and result["signals"][:1] == ["SIGTERM"], result
        assert wait_gone(member)
    finally:
        if lifecycle.same_process(lifecycle.identity(member), survivor):
            os.kill(member, signal.SIGKILL)
    assert refused(rig.gpu("case", "ssao_core"), "latched failure")


def sweep(rigs):
    """Last resort after a failed control: stop anything a rig recorded exactly."""
    for rig in rigs.values():
        try:
            ledger = rig.ledger()
            for entry in ledger.children().values():
                for ident in lifecycle.still_alive(ledger.retained_identities(entry)):
                    os.killpg(ident["pgid"], signal.SIGKILL)
        except Exception:
            pass


def main():
    unit_only = "--unit" in sys.argv[1:]
    scratch = c.ROOT / c.PACKET_REL / f".selftest-{os.getpid()}"
    scratch.mkdir()
    rigs = {}
    try:
        for control in (porcelain, environment, toolchain, ledger_budget, ledger_state, locks, identity, normal,
                        timeout, escalation, grandchild, spawn_failure, interrupt, terminate, sigkill,
                        gate_sigkill, grants, classification, frozen):
            control(scratch)
        if not unit_only:
            try:
                for name in ("main", "other", "latch", "suite", "kill", "foreign", "survivor"):
                    rigs[name] = Rig(scratch, name)
            except BaseException as error:
                RESULTS.append({"control": "end-to-end rigs", "ok": False, "error": repr(error)})
            else:
                for control in (e2e_main, e2e_two_packets, e2e_latch, e2e_suite_retry, e2e_sigkill,
                                e2e_recycled, e2e_survivors):
                    control(scratch, rigs)
    finally:
        sweep(rigs)
        shutil.rmtree(scratch, ignore_errors=True)
    failed = [r for r in RESULTS if not r["ok"]]
    print(json.dumps({"controls": len(RESULTS), "failed": len(failed), "results": RESULTS}, indent=1))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
