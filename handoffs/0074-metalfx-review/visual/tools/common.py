"""Shared constants and guards for handoff 0074 stage B (native MetalFX visual
review). Standard library only. Importing this module has no side effects.

Adapted from handoff 0072's committed visual/tools/common.py (visual tree
ac8a48de, read-only reuse), itself adapted from handoff 0068. Stage
differences: no source is built until root's grant names an explicit combined
source (full commit and tree, a root-reviewed per-file freeze and an
allowlisted Cargo feature set); the superseded 4bc810fe/a337c86d/535003d7
sources and their freezes are refused by name; there is no regression-suite
build or run; budgets are 128 submissions and 20 PNGs. The second execution
uses the new 0074-metalfx-visual-002 evidence namespace and publishes to
visual-screenshots-002, so the first execution's evidence stays intact.

As in 0072: the grant pins the reviewed visual tree and names the Cargo target
(validated, never a forbidden shared one); GPU children run under the global
native-gpu.lock; review budgets are debited before launch; signals are deferred
so every owned child is killed and reaped; any failure latches stop.json;
frozen binaries must be executable before anything is charged.

Every mutating entry point calls grant() first. Until root's review-ready.json
exists, tools refuse without creating any file or directory. Child
environments are built from scratch: HOME is passed through (never recorded)
and every other variable is an explicit, recorded, nonsecret override.
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
PACKET = "0074-metalfx-review"
STAGE = "metalfx-visual"
PACKET_DIR = ROOT / "handoffs" / PACKET
VISUAL_RELATIVE = "handoffs/" + PACKET + "/visual"
VISUAL = ROOT / VISUAL_RELATIVE
TOOLS = VISUAL / "tools"
PLAN_PATH = VISUAL / "plan.json"
HELPER_SOURCE = VISUAL / "review" / "metalfx_visual_review.rs"
HELPER_DEST = "crates/incant_render/tests/metalfx_visual_review.rs"
EVIDENCE_ROOT = Path("/Users/andreyyurlov/.codex/incant-evidence")
EVIDENCE = EVIDENCE_ROOT / "0074-metalfx-visual-002"
GRANT = EVIDENCE / "review-ready.json"
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
PUBLISHED = PACKET_DIR / "visual-screenshots-002"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
TARGET_PARENT = Path("/Users/andreyyurlov/.codex")
FORBIDDEN_TARGETS = ("/Users/andreyyurlov/.codex/incant-gpu-timestamps-target",
                     "/tmp/incant-cpu-profiler-target")
# Superseded sources: the review needs root's later combined source, never these.
DENIED_SOURCES = {
    "4bc810fe0daf238f219801de76b06f19e4daf826": "native MetalFX producer reviewed for 0074's numeric stage only",
    "a337c86d8d74e819b3e879d4a7db830b74de8862": "temporal upscaling base before the corrected edge and overlay-depth fixes",
    "535003d766bcc1386fd2d2f454c83808a0fcc4c8": "first execution's source, which read MetalFX factors as content fractions",
}
DENIED_FREEZES = {
    "b68c62d5e58b89a07aebfd17f144902917cee5890c4d09fbf06f51a64146a77b": "native-metalfx-source-003 (the 4bc810fe freeze)",
    "bd8295b0eb3174b96eb5ff73b0e0e7c464c6c5d620fd66df37bd59d3eafe56a0": "temporal-metalfx-union-001 (the 535003d7 freeze)",
}
# Cargo feature sets root may grant for the review build (sorted). cooked-temporal
# is the combined branch's complete cooked native route.
FEATURE_SETS = (("native-metalfx",), ("cooked-temporal", "native-metalfx"))
JITTER_SOURCE = "crates/incant_render/src/temporal/jitter.rs"
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
    "INCANT_0074_REVIEW_OUT": "new output directory for one review child",
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


def plan():
    return read_json(PLAN_PATH)


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True,
                          check=True).stdout


def refuse(reason):
    """Record a refusal only once the grant exists; before it, touch nothing."""
    if GRANT.is_file():
        append_durable(REFUSALS, {"at": now(), "reason": reason, "argv": sys.argv[1:]})
    raise SystemExit("refused: " + reason)


def target_allowed(value):
    path = Path(value)
    return (isinstance(value, str) and path.is_absolute() and str(path) == value
            and ".." not in path.parts and path.parent == TARGET_PARENT
            and path.name.endswith("-target") and value not in FORBIDDEN_TARGETS)


def source_problem(data):
    """Why a grant cannot select the review source, or None. Pure; touches no file.
    The source must be explicit: a full commit and tree, a root-reviewed freeze
    (absolute path and SHA-256) and one allowlisted feature set. The superseded
    sources and freeze are refused by name, so nothing silently substitutes them."""
    full = lambda v: isinstance(v, str) and re.fullmatch(r"[0-9a-f]{40}", v) is not None
    if not isinstance(data, dict) or data.get("packet") != PACKET or data.get("stage") != STAGE:
        return "review-ready.json does not name this packet and stage"
    if not all(full(data.get(key)) for key in ("source_commit", "source_tree", "visual_tree")):
        return "the grant lacks a full source_commit, source_tree or visual_tree"
    if data["source_commit"] in DENIED_SOURCES:
        return (f"source {data['source_commit']} is superseded "
                f"({DENIED_SOURCES[data['source_commit']]}); an explicit combined source is required")
    freeze = data.get("source_freeze")
    if not (isinstance(freeze, dict) and isinstance(freeze.get("path"), str)
            and Path(freeze["path"]).is_absolute()
            and re.fullmatch(r"[0-9a-f]{64}", str(freeze.get("sha256", ""))) is not None):
        return "the grant lacks an absolute source_freeze path and SHA-256"
    if freeze["sha256"] in DENIED_FREEZES:
        return f"freeze {DENIED_FREEZES[freeze['sha256']]} is superseded"
    features = data.get("features")
    if not (isinstance(features, list) and all(isinstance(f, str) for f in features)
            and tuple(sorted(features)) in FEATURE_SETS):
        return f"features must be exactly one of {[list(s) for s in FEATURE_SETS]}"
    # target_allowed builds a Path before its own type check, so test the type here.
    if not (isinstance(data.get("target"), str) and target_allowed(data["target"])):
        return "the grant does not name an allowed Cargo target"
    return None


def grant():
    """Root's grant names this packet/stage, the explicit combined source (see
    source_problem), the exact committed visual tree it reviewed and an allowed
    Cargo target."""
    if not GRANT.is_file():
        raise SystemExit("refused: review-ready.json is absent; preparation only")
    data = read_json(GRANT)
    problem = source_problem(data)
    if problem:
        refuse(problem)
    return data, sha(GRANT)


def features(data):
    """The granted Cargo feature list in canonical order."""
    return sorted(data["features"])


def samples(text):
    """The `SAMPLES: [[f32; 2]; 8]` jitter table in Rust source, as float pairs
    (None when absent). The fixture's copy must equal the granted source's."""
    match = re.search(r"SAMPLES: \[\[f32; 2\]; 8\] = \[(.*?)\];", text, re.S)
    if not match:
        return None
    return [[float(a), float(b)]
            for a, b in re.findall(r"\[\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\]", match[1])]


def target(data):
    return Path(data["target"])


def gate(allow_stopped=False, after_gpu_release=False):
    """Grant, stop latch, GPU release latch, then the held lease of the same grant."""
    data, grant_sha = grant()
    if STOP.exists() and not allow_stopped:
        refuse("stop.json latched a failure; only analysis and releases may run")
    if LATCH.exists() and not after_gpu_release:
        refuse("GPU already released by this stage")
    if not LEASE.is_file():
        refuse("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("released_at") or lease.get("grant_sha256") != grant_sha:
        refuse("lease released, or the grant changed since the lease was taken")
    return data, grant_sha


def tools_identity(data):
    """The committed visual tools, plan and fixture run unchanged: no local edits,
    and HEAD's visual tree is exactly the one root granted."""
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


def verify_checkout():
    """Every exported file still matches its blob; the only extra file allowed is
    the review fixture copy, and only with the committed fixture's bytes."""
    manifest = read_json(BUILD / "checkout.json")
    for entry in manifest["files"]:
        path = CHECKOUT / entry["path"]
        if path.is_symlink() or git_blob_sha1(path) != entry["blob"]:
            refuse("checkout file changed: " + entry["path"])
    expected = {entry["path"] for entry in manifest["files"]}
    extra = sorted(str(Path(d, n).relative_to(CHECKOUT)) for d, _, names in os.walk(CHECKOUT)
                   for n in names if str(Path(d, n).relative_to(CHECKOUT)) not in expected)
    if extra not in ([], [HELPER_DEST]):
        refuse("unexpected files in the checkout: " + ", ".join(extra[:10]))
    helper = CHECKOUT / HELPER_DEST
    if extra and sha(helper) != sha(HELPER_SOURCE):
        refuse("the checkout fixture copy differs from the committed fixture")
    return {"commit": manifest["commit"], "tree": manifest["tree"], "files": len(manifest["files"]),
            "manifest_sha256": sha(BUILD / "checkout.json"),
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
    if not BUDGET.exists():
        return []
    return [json.loads(line) for line in BUDGET.read_text().splitlines() if line.strip()]


def budget_used():
    rows = budget_rows()
    return {key: sum(row.get(key, 0) for row in rows) for key in ("frames", "ui_submissions", "pngs")}


def debit(label, item, frames, ui, pngs, grant_sha):
    """Monotonic prelaunch debit, fsynced before the child starts, never refunded."""
    budgets = plan()["budgets"]
    used = budget_used()
    if used["frames"] + used["ui_submissions"] + frames + ui > budgets["new_submissions_max"]:
        refuse(f"submission budget: {used} used, {frames} frames + {ui} UI requested")
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
