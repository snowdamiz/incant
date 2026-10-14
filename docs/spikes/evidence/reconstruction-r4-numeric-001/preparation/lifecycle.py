#!/usr/bin/env python3
"""Supervised single-child runner for the 0073 owner-attachment numeric test.
Standard library only. Preparation only: nothing in this file grants GPU use.

Adapted from this packet's reviewed visual/tools/common.py and gpu.py (from
handoff 0068's root-reviewed tools), reduced to one frozen-binary child.

  owner_runner.py check   CPU only: verify plan, identities and any grant; writes nothing
  owner_runner.py list    CPU only: root's `--list --exact` enumeration of the frozen
                          binary into a fresh CPU evidence directory; runs no test
  owner_runner.py run     the single GPU child; needs root's fresh grant.json

`run` refuses before touching anything unless root's grant.json exists in the
fresh evidence directory. It then takes the packet lock and the host-wide
native-gpu.lock, both non-blocking (busy refuses before any charge), and checks:
the grant (this runner's SHA-256, the plan's, the binary's, the predecessor
release), the plan's exact argv, budget, environment and acceptance, every pinned
identity, the binary's executable bit and that it lies outside any target
directory, a fresh evidence directory, and that no process runs the binary. It
appends the full attempt to debits.jsonl (fsynced, never refunded) and launches
exactly one child in its own process group: plan environment plus HOME inherited
unchanged (value not recorded), stdin closed, regular stdout/stderr files, 90 s.
INT/TERM/HUP stay pending from the debit until the attempt is recorded; any other
BaseException still kills the group, reaps, records and is re-raised afterwards.
release.json is written only when no owned process survives; otherwise
release-blocked.json is written and the runner fails closed. No retries.
"""

import datetime
import fcntl
import hashlib
import json
import os
import re
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True

PACKET = "0073-upscaling-review"
STAGE = "owner-attachment-numeric"
HERE = Path(__file__).resolve()
PACKET_DIR = HERE.parents[1]
CODEX = Path.home() / ".codex"
EVIDENCE_ROOT = CODEX / "incant-evidence"
TEST = ("models::retained::motion_owner_tests::"
        "exact_integer_owner_matches_visibility_and_leaves_motion_depth_color_unchanged")
MARKER = ("MOTION_OWNER renders=12 readback_submissions=12 max_extent=32x32 "
          "source_removed=true no_images=true")
BINARY_PATH = str(EVIDENCE_ROOT / "temporal-owner-attachment-001" / "binaries"
                  / "incant_render-288759d68012f619")
GRANT_SCHEMA = "incant-owner-numeric-gpu-grant-v1"
GRANT_FIELDS = ("schema", "packet", "stage", "authority", "issued_at", "plan_sha256",
                "runner_sha256", "binary_sha256", "evidence_dir", "predecessor_release_path",
                "predecessor_release_sha256", "process_launches",
                "total_explicit_queue_submissions")
BEFORE_RUN = {"grant.json", "refusals.jsonl", "owner.lock"}
ALIVE_FIELDS = ("owned_processes", "owned_processes_alive", "ledger_pids_alive",
                "process_groups_alive", "survivors")
IMAGES = (".png", ".jpg", ".jpeg", ".exr", ".hdr", ".tga", ".bmp", ".tif", ".tiff", ".ppm",
          ".pgm", ".webp", ".heic", ".ktx", ".dds")
BLOCKED = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                    r"(\d+) measured; (\d+) filtered out", re.M)

# Everything the plan must say, checked value for value (the plan hash is also pinned).
EXPECTED_PLAN = {
    "schema": "incant-owner-numeric-preparation-v1",
    "producer": "613ee59aec1f2bce080d4a1e335dc45f732b993f",
    "source_freeze_sha256": "95e4b8308badb57837d15b2d5759613d5dce1a50a1682a65b0492b8fb31f7365",
    "manifest_sha256": "eff501183d1226cba85a192b8b5db30e9f8b78eec9a630c5f791501071c523fc",
    "root_cpu_review_sha256": "d232a2241667c7258a4211489b851b2b7bdb7f72bf72b245d613fa678d23d3e2",
    "argv": [BINARY_PATH, "--ignored", "--exact", TEST, "--nocapture", "--test-threads=1"],
    "budget": {"process_launches": 1, "render_submissions": 12, "batched_readback_submissions": 12,
               "total_explicit_queue_submissions": 24, "maximum_extent": [32, 32],
               "timeout_seconds": 90, "image_files": 0, "builds": 0, "retries": 0},
    "environment": {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "TMPDIR": "<fresh evidence root>/tmp",
                    "INCANT_EXPECT_GPU_BACKEND": "Metal", "RUST_BACKTRACE": "0"},
    "acceptance": {"exit": 0, "tests_passed": 1, "tests_failed": 0, "tests_ignored": 0,
                   "exact_final_marker": MARKER, "required_backend": "Metal",
                   "adapter_identity_must_be_recorded": True,
                   "unsupported_early_return": "failure: final marker absent",
                   "process_groups_alive_after_release": 0},
}
EXPECTED_BINARY = {"path": BINARY_PATH, "sha256":
                   "fc3cd75685839d4b2fa465b853925ee2841ef3b28a568cc9c5cf5e8c650693d1",
                   "bytes": 33339024, "mode": "0755"}


def production():
    """Every path and pin the real run uses. Tests build sandbox specs instead."""
    return {
        "plan": PACKET_DIR / "owner-attachment-plan.json",
        "plan_sha256": "d7da3f1ccbe73b18a0f6d90f7da9d6c4ac586be351b04541afe7ae619c6b6512",
        "expected_plan": EXPECTED_PLAN, "expected_binary": EXPECTED_BINARY,
        "identities": {
            "source_freeze": (EVIDENCE_ROOT / "temporal-owner-attachment-001" / "source-003"
                              / "freeze.json", EXPECTED_PLAN["source_freeze_sha256"]),
            "root_cpu_review": (CODEX / "incant-reviews"
                                / "temporal-owner-attachment-final-cpu-root-20261013.json",
                                EXPECTED_PLAN["root_cpu_review_sha256"]),
            "cpu_enumeration": (EVIDENCE_ROOT / "temporal-owner-attachment-preparation-001"
                                / "result.json",
                                "68c5434e926678071f4291896f6897137b4010b3885ab8043db7d9785136cfba"),
        },
        "manifest": (CODEX / "worktrees" / "native-agent-play" / "incant",
                     EXPECTED_PLAN["producer"] + ":docs/spikes/evidence/temporal-owner-attachment.json",
                     EXPECTED_PLAN["manifest_sha256"]),
        "root_list_stdout_sha256": "0b2289fa1dca8f61bd98210221026c4e4308120bae5b6caeb9ab0f178e1f2be1",
        "runner": HERE,
        "evidence": EVIDENCE_ROOT / "0073-owner-attachment-gpu-001",
        "cpu_evidence": EVIDENCE_ROOT / "0073-owner-preparation-cpu-001",
        "global_lock": EVIDENCE_ROOT / "native-gpu.lock",
        "timeout": 90.0,
    }


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


class Refused(SystemExit):
    pass


def refuse(reason, evidence=None):
    """Refusals are logged only once root's grant exists; before that, touch nothing."""
    if evidence is not None and (Path(evidence) / "grant.json").is_file():
        with open(Path(evidence) / "refusals.jsonl", "a") as log:
            log.write(json.dumps({"at": now(), "reason": reason, "argv": sys.argv[1:]}) + "\n")
    raise Refused("refused: " + reason)


def write_new_json(path, value):
    with open(path, "x") as out:
        json.dump(value, out, indent=1)
        out.write("\n")
        out.flush()
        os.fsync(out.fileno())


def append_synced(path, value):
    with open(path, "a") as out:
        out.write(json.dumps(value) + "\n")
        out.flush()
        os.fsync(out.fileno())


# ------------------------------------------------------------------ identities
def load_plan(spec, evidence=None):
    if not spec["plan"].is_file() or sha(spec["plan"]) != spec["plan_sha256"]:
        refuse("the plan is absent or is not the pinned plan", evidence)
    plan = json.loads(spec["plan"].read_text())
    for key, value in spec["expected_plan"].items():
        if plan.get(key) != value:
            refuse(f"plan field {key} differs from the reviewed value", evidence)
    binary = plan.get("binary")
    if not isinstance(binary, dict) or any(binary.get(k) != v
                                           for k, v in spec["expected_binary"].items()):
        refuse("plan binary identity differs from the reviewed value", evidence)
    return plan


def check_binary(expected, evidence=None):
    path = Path(expected["path"])
    if path.is_symlink() or not path.is_file():
        refuse(f"{path} is absent, not a regular file or a symlink", evidence)
    if "target" in path.parts:
        refuse(f"{path} lies inside a target directory", evidence)
    info = path.stat()
    if format(stat.S_IMODE(info.st_mode), "04o") != expected["mode"] or not os.access(path, os.X_OK):
        refuse(f"{path} mode is {stat.S_IMODE(info.st_mode):04o}, not {expected['mode']}", evidence)
    if info.st_size != expected["bytes"]:
        refuse(f"{path} is {info.st_size} bytes, not {expected['bytes']}", evidence)
    digest = sha(path)
    if digest != expected["sha256"]:
        refuse(f"{path} SHA-256 {digest} is not the pinned {expected['sha256']}", evidence)
    return {"path": str(path), "sha256": digest, "bytes": info.st_size, "mode": expected["mode"]}


def check_identities(spec, evidence=None):
    found = {}
    for name, (path, expected) in spec["identities"].items():
        if not Path(path).is_file() or sha(path) != expected:
            refuse(f"{name} at {path} is absent or not {expected}", evidence)
        found[name] = {"path": str(path), "sha256": expected}
    review = json.loads(Path(spec["identities"]["root_cpu_review"][0]).read_text())
    for key in ("manifest_sha256", "source_freeze_sha256"):
        if review.get(key) != spec["expected_plan"][key]:
            refuse(f"root's CPU review {key} differs from the plan", evidence)
    if spec.get("manifest"):
        checkout, obj, expected = spec["manifest"]
        blob = subprocess.run(["git", "-C", str(checkout), "cat-file", "blob", obj],
                              capture_output=True, check=False)
        if blob.returncode != 0 or hashlib.sha256(blob.stdout).hexdigest() != expected:
            refuse(f"manifest {obj} is absent or not {expected}", evidence)
        found["manifest"] = {"object": obj, "sha256": expected}
    found["binary"] = check_binary(spec["expected_binary"], evidence)
    return found


def check_predecessor(path, digest, evidence=None):
    path = Path(path)
    if not path.is_absolute() or not path.is_file() or sha(path) != digest:
        refuse("predecessor release receipt is absent or not the pinned SHA-256", evidence)
    body = json.loads(path.read_text())
    if not isinstance(body, dict) or not (body.get("released_at") or body.get("released") is True):
        refuse("predecessor receipt does not record a release", evidence)
    for field in ALIVE_FIELDS:
        if field in body and body[field] not in ([], 0):
            refuse(f"predecessor release lists live processes in {field}", evidence)
    return {"path": str(path), "sha256": digest}


def load_grant(spec, evidence):
    path = Path(evidence) / "grant.json"
    if not path.is_file():
        raise Refused("refused: grant.json is absent; preparation only")
    try:
        data = json.loads(path.read_text())
    except ValueError:
        refuse("grant.json is not JSON", evidence)
    ok = (isinstance(data, dict) and set(data) == set(GRANT_FIELDS)
          and data["schema"] == GRANT_SCHEMA and data["packet"] == PACKET
          and data["stage"] == STAGE and data["authority"] == "root"
          and isinstance(data["issued_at"], str) and data["issued_at"][:2] == "20"
          and data["plan_sha256"] == spec["plan_sha256"]
          and data["runner_sha256"] == sha(spec["runner"])
          and data["binary_sha256"] == spec["expected_binary"]["sha256"]
          and data["evidence_dir"] == str(evidence)
          and isinstance(data["predecessor_release_path"], str)
          and HEX64.fullmatch(str(data["predecessor_release_sha256"]))
          and data["process_launches"] == 1 and data["total_explicit_queue_submissions"] == 24)
    if not ok:
        refuse("grant.json is malformed or does not bind this runner, plan, binary, evidence "
               "directory, one launch and 24 submissions", evidence)
    return data, sha(path)


def check_fresh(evidence):
    names = set(os.listdir(evidence))
    if not names <= BEFORE_RUN or "grant.json" not in names:
        refuse("evidence directory is not fresh: " + ", ".join(sorted(names - BEFORE_RUN)), evidence)


# ------------------------------------------------------------------ processes
class blocked:
    """INT/TERM/HUP stay pending from the debit until the attempt is recorded."""

    def __enter__(self):
        self.previous = signal.pthread_sigmask(signal.SIG_BLOCK, BLOCKED)
        return self

    def __exit__(self, *_):
        signal.pthread_sigmask(signal.SIG_SETMASK, self.previous)
        return False


def default_mask():
    signal.pthread_sigmask(signal.SIG_SETMASK, [])


def group_alive(pgid, settle=2.0):
    """Members can stay visible briefly while launchd reaps them; poll before deciding."""
    deadline = time.monotonic() + settle
    while True:
        try:
            os.killpg(pgid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            pass
        if time.monotonic() >= deadline:
            return True
        time.sleep(0.05)


def owned_processes(binary, pgid=None):
    """Processes whose arguments mention the frozen binary, or in the child's group.
    If ps itself fails, report an unknown survivor so callers fail closed."""
    try:
        listing = subprocess.run(["ps", "-axo", "pid=,pgid=,args="], capture_output=True,
                                 text=True, check=True).stdout
    except Exception as error:
        return [{"unknown": repr(error)}]
    found = []
    for line in listing.splitlines():
        parts = line.split(None, 2)
        if len(parts) < 3 or int(parts[0]) == os.getpid():
            continue
        if str(binary) in parts[2] or (pgid is not None and int(parts[1]) == pgid):
            found.append({"pid": int(parts[0]), "pgid": int(parts[1]), "args": parts[2][:200]})
    return found


def launch(argv, cwd, env, stdout, stderr, timeout, reap_timeout=30.0):
    """One child in its own session and group. Returns (record, exception or None);
    the group is SIGKILLed and the leader reaped on every outcome."""
    started = time.monotonic()
    try:
        child = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=stdout,
                                 stderr=stderr, start_new_session=True, preexec_fn=default_mask)
    except Exception as error:  # exec failure: no process exists
        return {"pid": None, "pgid": None, "exit": None, "launch_error": repr(error),
                "timed_out": False, "interrupted_by": [], "reaped": True,
                "group_empty_after": True, "seconds": 0.0}, None
    timed_out, interrupted, caught = False, [], None
    try:
        while True:
            try:
                child.wait(timeout=0.05)
                break
            except subprocess.TimeoutExpired:
                pass
            pending = signal.sigpending() & BLOCKED
            if pending:
                interrupted = sorted(signal.Signals(int(s)).name for s in pending)
                break
            if time.monotonic() - started >= timeout:
                timed_out = True
                break
    except BaseException as error:  # preserved; the caller re-raises after recording
        caught = error
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    code, reaped = None, False
    deadline = time.monotonic() + reap_timeout
    while not reaped:
        try:
            code, reaped = child.wait(timeout=0.1), True
        except subprocess.TimeoutExpired:
            if time.monotonic() >= deadline:
                break
    record = {"pid": child.pid, "pgid": child.pid, "exit": code, "timed_out": timed_out,
              "interrupted_by": interrupted, "reaped": reaped,
              "group_empty_after": not group_alive(child.pid),
              "seconds": round(time.monotonic() - started, 3)}
    if caught is not None:
        record["exception"] = repr(caught)
    return record, caught


def child_env(spec_env, tmpdir):
    """The plan's variables with the fresh TMPDIR, plus HOME passed through unchanged.
    Nothing else is inherited (no INCANT/WGPU/RUST/Cargo flags)."""
    env = {key: (str(tmpdir) if key == "TMPDIR" else value) for key, value in spec_env.items()}
    if "HOME" in os.environ:
        env["HOME"] = os.environ["HOME"]
    return env


def env_receipt(env):
    return {"keys": sorted(env), "values_recorded": {k: v for k, v in env.items() if k != "HOME"},
            "home": "inherited unchanged, value not recorded" if "HOME" in env else "absent"}


# ------------------------------------------------------------------ acceptance
def images_under(root):
    found = []
    for directory, _, names in os.walk(root):
        for name in names:
            path = Path(directory, name)
            head = b""
            if path.is_file() and not path.is_symlink():
                with open(path, "rb") as source:
                    head = source.read(8)
            if name.lower().endswith(IMAGES) or head.startswith((b"\x89PNG", b"\xff\xd8\xff")):
                found.append(str(path))
    return sorted(found)


def judge(stdout, stderr, record, survivors, binary_after, expected_binary, images):
    results = RESULT.findall(stdout)
    lines = [line for line in stderr.splitlines() if line.startswith("MOTION_OWNER ")]
    adapter = lines[0] if lines and lines[0].startswith("MOTION_OWNER adapter=AdapterInfo {") else None
    checks = {
        "exit_zero": record["exit"] == 0,
        "no_timeout_interrupt_exception": not (record["timed_out"] or record["interrupted_by"]
                                                or "exception" in record or "launch_error" in record),
        "running_one_test": "running 1 test" in stdout.splitlines(),
        "libtest_ok_1_passed_0_failed_0_ignored": len(results) == 1
        and results[0][:5] == ("ok", "1", "0", "0", "0"),
        "no_failures_section": "\nfailures:\n" not in stdout,
        "adapter_recorded_with_metal": adapter is not None and "backend: Metal" in adapter,
        "adapter_then_exact_final_marker_only": len(lines) == 2 and lines[1] == MARKER,
        "binary_unchanged": binary_after == expected_binary,
        "no_image_files": images == [],
        "reaped_group_empty_no_survivors": record["reaped"] and record["group_empty_after"]
        and survivors == [],
    }
    return all(checks.values()), checks, adapter


# ------------------------------------------------------------------ commands
def check(spec):
    plan = load_plan(spec)
    identities = check_identities(spec)
    report = {"plan": {"path": str(spec["plan"]), "sha256": spec["plan_sha256"]},
              "identities": identities, "runner_sha256": sha(spec["runner"]),
              "argv": plan["argv"], "budget": plan["budget"]}
    grant = Path(spec["evidence"]) / "grant.json"
    if grant.is_file():
        data, digest = load_grant(spec, spec["evidence"])
        report["grant"] = {"sha256": digest, "predecessor": check_predecessor(
            data["predecessor_release_path"], data["predecessor_release_sha256"])}
    else:
        report["grant"] = "absent: preparation only, no GPU authority"
    return report


def listing(spec, name="list-001"):
    """CPU only: root's enumeration argv once, in a fresh directory; no test runs."""
    plan = load_plan(spec)
    binary = check_binary(spec["expected_binary"])
    root = Path(spec["cpu_evidence"]) / name
    root.mkdir(parents=True, exist_ok=False)
    tmp = root / "tmp"
    tmp.mkdir()
    env = child_env(plan["environment"], tmp)
    argv = [binary["path"], "--list", "--exact", TEST]
    with blocked():
        with open(root / "stdout.txt", "x") as out, open(root / "stderr.txt", "x") as err:
            record, caught = launch(argv, tmp, env, out, err, 30.0)
        survivors = owned_processes(binary["path"], record["pgid"])
        stdout = (root / "stdout.txt").read_text()
        entry = {"at": now(), "argv": argv, "env": env_receipt(env), **record,
                 "survivors": survivors, "stdout_sha256": sha(root / "stdout.txt"),
                 "stderr_sha256": sha(root / "stderr.txt"),
                 "root_stdout_sha256": spec["root_list_stdout_sha256"],
                 "listed": [line[:-len(": test")] for line in stdout.splitlines()
                            if line.endswith(": test")],
                 "binary_sha256_after": sha(binary["path"]), "gpu_execution": False}
        entry["ok"] = (record["exit"] == 0 and record["reaped"] and record["group_empty_after"]
                       and not survivors and entry["listed"] == [TEST]
                       and entry["stdout_sha256"] == spec["root_list_stdout_sha256"]
                       and entry["binary_sha256_after"] == binary["sha256"] and caught is None)
        write_new_json(root / "receipt.json", entry)
    if caught is not None:
        raise caught
    return entry


def run(spec):
    evidence = Path(spec["evidence"])
    if not (evidence / "grant.json").is_file():
        raise Refused("refused: grant.json is absent; preparation only, nothing touched")
    with open(evidence / "owner.lock", "a") as lock, open(spec["global_lock"], "a") as shared:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            refuse("the packet lock is held; nothing was charged", evidence)
        try:
            fcntl.flock(shared, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            refuse("native-gpu.lock is held by another runner; nothing was charged", evidence)
        grant, grant_sha = load_grant(spec, evidence)
        plan = load_plan(spec, evidence)
        identities = check_identities(spec, evidence)
        predecessor = check_predecessor(grant["predecessor_release_path"],
                                        grant["predecessor_release_sha256"], evidence)
        check_fresh(evidence)
        binary = identities["binary"]["path"]
        before = owned_processes(binary)
        if before:
            refuse("processes already run the frozen binary: " + json.dumps(before)[:300], evidence)
        tmp = evidence / "tmp"
        tmp.mkdir()
        env = child_env(plan["environment"], tmp)
        out_path, err_path = evidence / "stdout.txt", evidence / "stderr.txt"
        with blocked():
            append_synced(evidence / "debits.jsonl", {
                "at": now(), "attempt": 1, "process_launches": 1,
                "total_explicit_queue_submissions": 24, "grant_sha256": grant_sha,
                "runner_sha256": sha(spec["runner"]), "plan_sha256": spec["plan_sha256"],
                "binary_sha256": identities["binary"]["sha256"], "refundable": False})
            started = now()
            with open(out_path, "x") as out, open(err_path, "x") as err:
                record, caught = launch(plan["argv"], tmp, env, out, err, spec["timeout"])
            survivors = owned_processes(binary, record["pgid"])
            try:
                after = sha(binary)
                stdout = out_path.read_text(errors="replace")
                stderr = err_path.read_text(errors="replace")
                images = images_under(evidence)
                accepted, checks, adapter = judge(stdout, stderr, record, survivors, after,
                                                  identities["binary"]["sha256"], images)
                entry = {"at": now(), "started": started, "argv": plan["argv"], "cwd": str(tmp),
                         "env": env_receipt(env), **record, "survivors": survivors,
                         "binary_sha256_before": identities["binary"]["sha256"],
                         "binary_sha256_after": after, "grant_sha256": grant_sha,
                         "predecessor": predecessor, "identities": identities,
                         "stdout_sha256": sha(out_path), "stderr_sha256": sha(err_path),
                         "adapter": adapter, "images": images, "checks": checks,
                         "accepted": accepted, "tmp_leftovers": sorted(os.listdir(tmp)),
                         "os": {"uname": list(os.uname())}}
            except BaseException as error:  # analysis must not lose the attempt
                accepted = False
                entry = {"at": now(), "started": started, **record, "survivors": survivors,
                         "grant_sha256": grant_sha, "accepted": False,
                         "analysis_error": repr(error)}
                caught = caught if caught is not None else error
            append_synced(evidence / "ledger.jsonl", entry)
            clean = record["reaped"] and record["group_empty_after"] and not survivors
            release = {"at": now(), "released": clean, "child_pid": record["pid"],
                       "child_pgid": record["pgid"], "reaped": record["reaped"],
                       "group_empty_after": record["group_empty_after"], "survivors": survivors,
                       "ledger_sha256": sha(evidence / "ledger.jsonl"),
                       "debits_sha256": sha(evidence / "debits.jsonl"), "accepted": accepted}
            if clean:
                release["released_at"] = release["at"]
                release["owned_processes"] = []
                release["ledger_pids_alive"] = []
            write_new_json(evidence / ("release.json" if clean else "release-blocked.json"), release)
    if caught is not None:
        raise caught
    if record["interrupted_by"]:
        raise KeyboardInterrupt("interrupted by " + ", ".join(record["interrupted_by"]))
    print(json.dumps({k: entry[k] for k in ("pid", "exit", "timed_out", "accepted", "checks",
                                            "adapter")}, indent=1))
    if not clean:
        raise SystemExit("owned processes survive cleanup; release blocked")
    return entry


def raise_interrupt(signum, _frame):
    raise KeyboardInterrupt(f"signal {signum}")


def main(argv):
    for signum in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, raise_interrupt)
    spec = production()
    if argv == ["check"]:
        print(json.dumps(check(spec), indent=1))
    elif argv == ["list"]:
        entry = listing(spec)
        print(json.dumps(entry, indent=1))
        return 0 if entry["ok"] else 1
    elif argv == ["run"]:
        return 0 if run(spec)["accepted"] else 1
    else:
        raise SystemExit(__doc__)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
