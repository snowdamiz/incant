"""Shared constants and guards for handoff 0076's shadow receiver guard comparison.
Standard library only. Importing this module has no side effects.

Adapted narrowly from 0072's fixed-sharpness tools (48a688c5, visual tree
b11a7d3a, with root's cd4f4cd7 build fix): the same two root grants gate the
stage and every mutating entry point checks its grant first; without it, tools
refuse without creating any file or directory. What changed is that one stage
now builds and runs two roles, `baseline` (main224) and `candidate` (the
shadow receiver guard), each from its own exported commit into its own frozen
binary, with the same fixture, flags and Cargo target used one after the other.
- build-ready.json (root-supervised CPU build): this packet and build stage, the
  actual holder, per role a committed source as a 40-hex commit (never a branch
  name) equal to the plan's, and root's source freeze for it by path and
  SHA-256; the committed visual tree and one dedicated Cargo target. Every build
  input of each exported commit must equal its freeze (build.py checkout).
- review-ready.json (GPU review, written after both builds): this packet and
  review stage, the actual holder, the visual tree, the SHA-256 of each role's
  frozen build record and at least one prior release pin.
0072 namespaces are never written, and their grants name a packet and stages these
tools refuse.

GPU children run under the global non-waiting native-gpu.lock; budgets are
debited before launch, monotonic and never refunded; signals are deferred so
every owned child is killed and reaped; any failure latches stop.json; frozen
binaries must be executable before anything is charged. Child environments are
built from scratch: HOME is passed through (never recorded) and every other
variable is an explicit, recorded, nonsecret override.
"""

import contextlib
import datetime
import fcntl
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
PACKET = "0076-shadow-guard-review"
STAGE = "shadow-guard-visual"
BUILD_STAGE = "shadow-guard-build"
ROLES = ("baseline", "candidate")  # built and run in this order
PACKET_DIR = ROOT / "handoffs" / PACKET
VISUAL_RELATIVE = "handoffs/" + PACKET + "/visual"
VISUAL = ROOT / VISUAL_RELATIVE
TOOLS = VISUAL / "tools"
PLAN_PATH = VISUAL / "plan.json"
HELPER_SOURCE = VISUAL / "review" / "temporal_visual_review.rs"
HELPER_DEST = "crates/incant_render/tests/temporal_visual_review.rs"
CODEX = Path("/Users/andreyyurlov/.codex")
EVIDENCE_ROOT = CODEX / "incant-evidence"
EVIDENCE = EVIDENCE_ROOT / "0076-shadow-guard-review-001"
GRANT = EVIDENCE / "review-ready.json"
BUILD_GRANT = EVIDENCE / "build-ready.json"
GPU_LOCK = EVIDENCE_ROOT / "native-gpu.lock"  # global; shared with root's runners
PACKET_LOCK = EVIDENCE / "packet.lock"
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
STOP = EVIDENCE / "stop.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"  # one row per owned child, always with "pid"
BUDGET = EVIDENCE / "budget.jsonl"  # prelaunch debits only; never mixed into the ledger
POST_RELEASE_LEDGER = EVIDENCE / "post-release-ledger.jsonl"
REFUSALS = EVIDENCE / "refusals.jsonl"
CHECKOUT = EVIDENCE / "checkout"
BUILD = EVIDENCE / "build"
BIN = EVIDENCE / "bin"
LOGS = EVIDENCE / "logs"
SCRATCH = EVIDENCE / "tmp"
CAPTURES = EVIDENCE / "captures"
ANALYSIS = EVIDENCE / "analysis"
PUBLISHED = PACKET_DIR / "screenshots"
PUBLISHED_EVIDENCE = PACKET_DIR / "evidence"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
FORBIDDEN_TARGETS = ("/Users/andreyyurlov/.codex/incant-gpu-timestamps-target",
                     "/tmp/incant-cpu-profiler-target")
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
INHERITED = ("HOME",)
# The only variables a child receives besides INHERITED, all nonsecret.
OVERRIDES = {
    "PATH": "fixed tool search path (rustup proxies for builds, system only for tests)",
    "TMPDIR": "fresh packet scratch directory under the evidence directory",
    "RUST_BACKTRACE": "1, for panic locations in test children",
    "CARGO_TARGET_DIR": "the granted Cargo target directory (builds only)",
    "CARGO_TERM_COLOR": "never, so build logs carry no terminal escapes",
    "RUSTUP_AUTO_INSTALL": "0, so rustup never downloads a toolchain",
    "INCANT_EXPECT_GPU_BACKEND": "required adapter backend, read only by tests/support checked()",
    # The reused fixture keeps 0072's variable name byte for byte.
    "INCANT_0072_REVIEW_OUT": "new output directory for one review child",
}
DEFERRED = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def read_json(path):
    return json.loads(Path(path).read_text())


def write_new_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "x") as out:
        json.dump(value, out, indent=1)
        out.write("\n")
        out.flush()
        os.fsync(out.fileno())


def append_durable(path, entry):
    with open(path, "a") as out:
        out.write(json.dumps(entry) + "\n")
        out.flush()
        os.fsync(out.fileno())


def read_lines(path):
    path = Path(path)
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def plan():
    return read_json(PLAN_PATH)


def git(*args, cwd=None):
    return subprocess.run(["git", *args], cwd=cwd or ROOT, capture_output=True, text=True,
                          check=True).stdout


def refuse(reason):
    """Record a refusal only once a grant exists; before any grant, touch nothing."""
    if GRANT.is_file() or BUILD_GRANT.is_file():
        append_durable(REFUSALS, {"at": now(), "reason": reason, "argv": sys.argv[1:]})
    raise SystemExit("refused: " + reason)


def target_allowed(value):
    """A dedicated directory directly under ~/.codex named *-target; never a shared
    or forbidden target, the worktree or an evidence directory."""
    if not isinstance(value, str) or not value.startswith("/"):
        return False
    path = Path(value)
    return (str(path) == value and ".." not in path.parts and path.parent == CODEX
            and path.name.endswith("-target") and value not in FORBIDDEN_TARGETS
            and ROOT not in path.parents)


def holder_ok(value):
    return isinstance(value, str) and 0 < len(value.strip()) <= 120


def release_pins(data):
    return [k for k in data if k.startswith("prior_") and k.endswith("_path") and "release" in k]


def source_ok(source, expected):
    freeze = source.get("source_freeze") if isinstance(source, dict) else None
    return (isinstance(source, dict) and HEX40.fullmatch(str(source.get("source_commit", "")))
            and source.get("source_commit") == expected
            and isinstance(freeze, dict) and str(freeze.get("path", "")).startswith("/")
            and HEX64.fullmatch(str(freeze.get("sha256", ""))) is not None)


def build_grant():
    """Root's supervised CPU build grant (0073 routine, per role): for baseline and
    candidate a committed source (40-hex commit, never a branch name) equal to the
    plan's and root's source freeze by path and SHA-256; the committed visual tree,
    one allowed target and the actual holder."""
    if not BUILD_GRANT.is_file():
        raise SystemExit("refused: build-ready.json is absent; preparation only")
    data = read_json(BUILD_GRANT)
    sources = data.get("sources")
    expected = {role: plan()["sources"][role]["commit"] for role in ROLES}
    if (data.get("packet") != PACKET or data.get("stage") != BUILD_STAGE
            or not holder_ok(data.get("holder"))
            or not isinstance(sources, dict) or sorted(sources) != sorted(ROLES)
            or not all(source_ok(sources[role], expected[role]) for role in ROLES)
            or not HEX40.fullmatch(str(data.get("visual_tree", "")))
            or not target_allowed(data.get("target"))):
        refuse("build-ready.json does not name this packet and build stage, the holder, for "
               "baseline and candidate exactly the plan's 40-hex source commit with root's source "
               "freeze path and SHA-256, the visual tree and an allowed ~/.codex/*-target")
    return data, sha(BUILD_GRANT)


def grant():
    """Root's GPU review grant, written after both builds: this packet and review
    stage, the holder, the visual tree, each role's build record SHA-256 and a
    prior release pin."""
    if not GRANT.is_file():
        raise SystemExit("refused: review-ready.json is absent; preparation only")
    data = read_json(GRANT)
    records = data.get("build_records")
    if (data.get("packet") != PACKET or data.get("stage") != STAGE
            or not holder_ok(data.get("holder"))
            or not HEX40.fullmatch(str(data.get("visual_tree", "")))
            or not isinstance(records, dict) or sorted(records) != sorted(ROLES)
            or not all(HEX64.fullmatch(str(records[role])) for role in ROLES)
            or not release_pins(data)):
        refuse("review-ready.json does not name this packet and review stage, the holder, the "
               "visual tree, the SHA-256 of both roles' build records and a prior release pin")
    return data, sha(GRANT)


def role_ok(role):
    if role not in ROLES:
        raise SystemExit(f"refused: unknown role {role!r}; roles are {', '.join(ROLES)}")
    return role


def checkout_dir(role):
    return CHECKOUT / role_ok(role)


def build_dir(role):
    return BUILD / role_ok(role)


def captures_dir(role):
    return CAPTURES / role_ok(role)


def target(data):
    return Path(data["target"])


def build_gate():
    """Build grant, no GPU release or stop latch, and the build lease (held,
    unreleased, taken under this build grant)."""
    data, grant_sha = build_grant()
    if LATCH.exists():
        refuse("GPU and target already released by this stage")
    if STOP.exists():
        refuse("stop.json latched a failure; only analysis and releases may run")
    if not LEASE.is_file():
        refuse("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("released_at") or lease.get("grant_sha256") != grant_sha:
        refuse("lease released, or the build grant changed since the lease was taken")
    return data, grant_sha


def gate():
    """GPU grant, then the release and stop latches. GPU children need no Cargo
    target: they run frozen binaries the grant binds by hash."""
    data, grant_sha = grant()
    if LATCH.exists():
        refuse("GPU already released by this stage")
    if STOP.exists():
        refuse("stop.json latched a failure; only analysis and releases may run")
    return data, grant_sha


def prior_receipts(data, required):
    """Every prior_<name>_path / _sha256 pin must verify, and every pinned release
    body must list no live process (0073 routine)."""
    alive_fields = ("owned_processes", "owned_processes_alive", "ledger_pids_alive",
                    "target_users", "processes_on_target_at_release")
    pins = {}
    for key, value in data.items():
        if key.startswith("prior_") and key.endswith("_path"):
            name = key[len("prior_"):-len("_path")]
            path = Path(value)
            pins[name] = {"path": value, "expected": data.get("prior_" + name + "_sha256"),
                          "sha256": sha(path) if path.is_file() else None}
    releases = sorted(name for name in pins if "release" in name)
    ok = (bool(releases) or not required) and all(
        p["expected"] and p["sha256"] == p["expected"] for p in pins.values())
    bodies = {name: read_json(pins[name]["path"]) for name in releases} if ok else {}
    alive = {name: {k: body[k] for k in alive_fields if k in body} for name, body in bodies.items()}
    ok = ok and all(v == [] for fields in alive.values() for v in fields.values())
    return {"pins": pins, "release_pins": releases, "release_alive_fields": alive}, ok


def tools_identity(data):
    """The committed visual tools, plan and fixture run unchanged: no local edits,
    and HEAD's visual tree is exactly the one the grant names."""
    dirty = git("status", "--porcelain", "--untracked-files=all", "--", VISUAL_RELATIVE).strip()
    if dirty:
        refuse("visual tools, plan or fixture differ from HEAD: " + dirty[:300])
    tree = git("rev-parse", "HEAD:" + VISUAL_RELATIVE).strip()
    if tree != data["visual_tree"]:
        refuse(f"committed visual tree {tree} is not the granted {data['visual_tree']}")
    return {"head": git("rev-parse", "HEAD").strip(), "visual_tree": tree,
            "plan_sha256": sha(PLAN_PATH), "helper_sha256": sha(HELPER_SOURCE)}


def git_blob_sha1(path):
    data = Path(path).read_bytes()
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def verify_checkout(role):
    """Every file of the role's export still matches its blob; the only extra file
    allowed is the review fixture copy, and only with the committed fixture's bytes."""
    root, record = checkout_dir(role), build_dir(role) / "checkout.json"
    if not record.is_file():
        refuse(f"no {role} checkout; run build.py checkout {role} first")
    manifest = read_json(record)
    for entry in manifest["files"]:
        path = root / entry["path"]
        if path.is_symlink() or git_blob_sha1(path) != entry["blob"]:
            refuse(f"{role} checkout file changed: " + entry["path"])
    expected = {entry["path"] for entry in manifest["files"]}
    extra = sorted(str(Path(d, n).relative_to(root)) for d, _, names in os.walk(root)
                   for n in names if str(Path(d, n).relative_to(root)) not in expected)
    if extra not in ([], [HELPER_DEST]):
        refuse(f"unexpected files in the {role} checkout: " + ", ".join(extra[:10]))
    helper = root / HELPER_DEST
    if extra and sha(helper) != sha(HELPER_SOURCE):
        refuse(f"the {role} checkout fixture copy differs from the committed fixture")
    return {"role": role, "commit": manifest["commit"], "tree": manifest["tree"],
            "files": len(manifest["files"]), "manifest_sha256": sha(record),
            "helper_copy_sha256": sha(helper) if extra else None}


def require_executable(path):
    """Content hashes do not cover mode bits; refuse before any debit or lock."""
    mode = Path(path).stat().st_mode
    if not (os.access(path, os.X_OK) and mode & 0o100):
        refuse(f"{path} is not executable (mode {oct(mode & 0o777)})")


def child_env(overrides):
    unknown = set(overrides) - set(OVERRIDES)
    if unknown:
        raise SystemExit("refused: override outside the documented allowlist: "
                         + ", ".join(sorted(unknown)))
    env = {key: os.environ[key] for key in INHERITED if key in os.environ}
    env.update(overrides)
    return env


def env_receipt(overrides):
    """What receipts may say about a child's environment."""
    return {"inherited_keys_values_not_recorded": [k for k in INHERITED if k in os.environ],
            "overrides": dict(overrides)}


def fresh_dir(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.mkdir()
    return path


@contextlib.contextmanager
def packet_lock():
    """Serializes this stage's own steps (builds and GPU children)."""
    with open(PACKET_LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


@contextlib.contextmanager
def gpu_lock():
    """Global native GPU ownership shared with root's runners; never waits."""
    with open(GPU_LOCK, "a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            refuse("native-gpu.lock is held by another runner; nothing launched or charged")
        yield


def latch_stop(label, reason):
    """The first failure latches stop.json; every later build/GPU step refuses."""
    if not STOP.exists():
        write_new_json(STOP, {"at": now(), "label": label, "reason": reason})


def budget_rows():
    return read_lines(BUDGET)


def budget_used():
    rows = budget_rows()
    return {key: sum(row.get(key, 0) for row in rows) for key in ("frames", "ui_submissions", "pngs")}


def debit(label, item, frames, ui, pngs, grant_sha):
    """Monotonic prelaunch debit, fsynced before the child starts, never refunded.
    The ceiling includes failed and interrupted attempts."""
    budgets = plan()["budgets"]
    used = budget_used()
    if used["frames"] + used["ui_submissions"] + frames + ui > budgets["submissions_max"]:
        refuse(f"submission ceiling: {used} used, {frames} frames + {ui} UI requested")
    # Separate caps where the plan names them: numeric controls and visual world + UI.
    numeric = str(item).startswith("numeric")
    cap = budgets.get("numeric_submissions_max" if numeric else "visual_submissions_max")
    if cap is not None:
        same = sum(r.get("frames", 0) + r.get("ui_submissions", 0) for r in budget_rows()
                   if str(r.get("item", "")).startswith("numeric") == numeric)
        if same + frames + ui > cap:
            refuse(f"{'numeric' if numeric else 'visual'} submission ceiling: {same} used, "
                   f"{frames} frames + {ui} UI requested, cap {cap}")
    if used["pngs"] + pngs > budgets["pngs_max"]:
        refuse(f"PNG budget: {used['pngs']} used, {pngs} requested")
    entry = {"label": label, "item": item, "frames": frames, "ui_submissions": ui, "pngs": pngs,
             "grant_sha256": grant_sha, "at": now()}
    append_durable(BUDGET, entry)
    return entry


class Interrupted(BaseException):
    """SIGTERM or SIGHUP while waiting on an owned child; handled like Ctrl-C."""


def _interrupt(signum, _frame):
    raise Interrupted(signal.Signals(signum).name)


def _child_default_mask():
    signal.pthread_sigmask(signal.SIG_UNBLOCK, DEFERRED)


@contextlib.contextmanager
def deferred_signals():
    """INT/TERM/HUP stay pending except inside supervise()'s wait, so they can
    only interrupt the wait, never launch bookkeeping, kill/reap or ledger
    writes. Pending signals act with their original dispositions on exit."""
    handlers = {s: signal.signal(s, _interrupt) for s in (signal.SIGTERM, signal.SIGHUP)}
    signal.pthread_sigmask(signal.SIG_BLOCK, DEFERRED)
    try:
        yield
    finally:
        for number, handler in handlers.items():
            signal.signal(number, handler)
        signal.pthread_sigmask(signal.SIG_UNBLOCK, DEFERRED)


def supervise(argv, cwd, env, stdout, stderr, timeout):
    """Launch one process group and always SIGKILL and reap it, after exit,
    timeout or interrupt. Call inside deferred_signals(); the child starts with
    the default signal mask. Returns (pid, exit, timed_out, exc_info or None,
    group_empty)."""
    child = subprocess.Popen(argv, cwd=cwd, env=env, stdout=stdout, stderr=stderr,
                             start_new_session=True, preexec_fn=_child_default_mask)
    timed_out, original = False, None
    try:
        signal.pthread_sigmask(signal.SIG_UNBLOCK, DEFERRED)
        child.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
    except BaseException:  # re-raised by the caller after the attempt is recorded
        original = sys.exc_info()
    finally:
        signal.pthread_sigmask(signal.SIG_BLOCK, DEFERRED)
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass  # also covers exit racing the timeout
        code = child.wait()
    group_empty = False
    for _ in range(200):  # killed descendants may linger briefly before reparenting
        try:
            os.killpg(child.pid, 0)
        except ProcessLookupError:
            group_empty = True
            break
        time.sleep(0.01)
    return child.pid, code, timed_out, original, group_empty
