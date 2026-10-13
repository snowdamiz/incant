"""Shared constants and guards for the 0073 upscaling visual tools. Standard library only.

Adapted from handoff 0068's committed, root-reviewed tools/common.py (read-only
reuse). Importing this module has no side effects.

Two root grants gate the stage, and every mutating entry point checks its grant
first; without it, tools refuse without creating any file or directory.
- build-ready.json (supervised CPU build): this packet, a committed source as a
  40-hex commit (never a branch name), root's source freeze by path and SHA-256,
  the committed visual tree and a dedicated Cargo target. Every build input of
  the exported commit must equal that freeze (build.py checkout).
- review-ready.json (GPU review, written after the build): this packet, the
  visual tree, the SHA-256 of the frozen build record and at least one prior
  release pin.
The packet's GPU release latch and stop latch make GPU steps refuse afterwards.

Child environments are constructed from scratch: HOME is passed through (its
value is never recorded) and every other variable is an explicit override from
OVERRIDES, recorded verbatim in receipts. The inherited environment, ACP or
session transport values, credentials and account metadata are never
serialized.

Every owned child goes through launch(): its own session and process group, an
empty signal mask, and SIGKILL of the whole group plus a reap after every
outcome. Runners keep INT, TERM and HUP blocked (blocked()) from the debit until
the attempt is recorded, polling the pending set while they wait.
"""

import datetime
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
PACKET = "0073-upscaling-review"
STAGE = "upscaling-visual"
BUILD_STAGE = "upscaling-build"
PACKET_DIR = ROOT / "handoffs" / PACKET
VISUAL = PACKET_DIR / "visual"
TOOLS = VISUAL / "tools"
PLAN_PATH = VISUAL / "plan.json"
HELPER_SOURCE = VISUAL / "review" / "upscaling_review.rs"
HELPER_DEST = "crates/incant_render/tests/upscaling_review.rs"
VISUAL_PATH = "handoffs/" + PACKET + "/visual"
CODEX = Path.home() / ".codex"
EVIDENCE_ROOT = CODEX / "incant-evidence"
EVIDENCE = EVIDENCE_ROOT / "0073-upscaling-visual-002"
GRANT = EVIDENCE / "review-ready.json"
BUILD_GRANT = EVIDENCE / "build-ready.json"
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
STOP = EVIDENCE / "stop.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
DEBITS = EVIDENCE / "debits.jsonl"
# CPU-only steps after the GPU latch append here, so the latched ledger hash stays valid.
POST_RELEASE_LEDGER = EVIDENCE / "post-release-ledger.jsonl"
REFUSALS = EVIDENCE / "gpu-refusals.jsonl"
LOCK = EVIDENCE / "gpu.lock"
# The shared host-wide GPU lock root's numeric runners also take (non-blocking).
GLOBAL_LOCK = EVIDENCE_ROOT / "native-gpu.lock"
CHECKOUT = EVIDENCE / "checkout"
BUILD = EVIDENCE / "build"
BIN = EVIDENCE / "bin"
LOGS = EVIDENCE / "logs"
SCRATCH = EVIDENCE / "tmp"
CAPTURES = EVIDENCE / "captures"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
INHERITED = ("HOME",)
# The only variables a child receives besides INHERITED, all nonsecret.
OVERRIDES = {
    "PATH": "fixed tool search path (rustup proxies for builds, system only for tests)",
    "TMPDIR": "fresh packet scratch directory under the evidence directory",
    "RUST_BACKTRACE": "1, for panic locations in test children",
    "CARGO_TARGET_DIR": "the granted Cargo target directory (builds only)",
    "CARGO_TERM_COLOR": "never, so build logs carry no terminal escapes",
    "RUSTUP_AUTO_INSTALL": "0, so rustup never downloads a toolchain",
    "INCANT_0073_REVIEW_OUT": "new output directory for one review-helper child",
    "INCANT_EXPECT_GPU_BACKEND": "Metal, enforced by tests/support::checked",
}
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
# Interrupts that stay pending while an attempt is debited, run and recorded.
BLOCKED = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def sha_bytes(data):
    return hashlib.sha256(data).hexdigest()


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


def append_synced(path, value):
    """One JSON line, flushed and fsynced before returning."""
    with open(path, "a") as out:
        out.write(json.dumps(value) + "\n")
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
        with open(REFUSALS, "a") as log:
            log.write(json.dumps({"at": now(), "reason": reason, "argv": sys.argv[1:]}) + "\n")
    raise SystemExit("refused: " + reason)


def target_allowed(value):
    """A dedicated directory under ~/.codex named *-target; never the shared /tmp
    CPU target, the worktree or the evidence directory."""
    if not isinstance(value, str) or not value.startswith("/"):
        return False
    path = Path(value)
    return (path.parent == CODEX and path.name.endswith("-target")
            and path != EVIDENCE and ROOT not in path.parents)


def release_pins(data):
    return [k for k in data if k.startswith("prior_") and k.endswith("_path") and "release" in k]


def build_grant():
    """Root's supervised CPU build grant: a committed source (40-hex commit, never
    a branch name), root's source freeze by path and SHA-256, the committed visual
    tree and an allowed target."""
    if not BUILD_GRANT.is_file():
        raise SystemExit("refused: build-ready.json is absent; preparation only")
    data = read_json(BUILD_GRANT)
    freeze = data.get("source_freeze")
    if (data.get("packet") != PACKET or data.get("stage") != BUILD_STAGE
            or not HEX40.fullmatch(str(data.get("source_commit", "")))
            or not isinstance(freeze, dict) or not str(freeze.get("path", "")).startswith("/")
            or not HEX64.fullmatch(str(freeze.get("sha256", "")))
            or not HEX40.fullmatch(str(data.get("visual_tree", "")))
            or not target_allowed(data.get("target"))):
        refuse("build-ready.json does not name this packet and build stage, a 40-hex source "
               "commit, root's source freeze path and SHA-256, the visual tree and an allowed "
               "~/.codex/*-target")
    return data, sha(BUILD_GRANT)


def grant():
    """Root's GPU review grant, written after the build: this packet, the committed
    visual tree, the SHA-256 of the frozen build record and a prior release pin."""
    if not GRANT.is_file():
        raise SystemExit("refused: review-ready.json is absent; preparation only")
    data = read_json(GRANT)
    if (data.get("packet") != PACKET or data.get("stage") != STAGE
            or not HEX40.fullmatch(str(data.get("visual_tree", "")))
            or not HEX64.fullmatch(str(data.get("build_record_sha256", "")))
            or not release_pins(data)):
        refuse("review-ready.json does not name this packet and review stage, the visual tree, "
               "the build record SHA-256 and a prior release pin")
    return data, sha(GRANT)


def build_gate():
    """Build grant, no GPU release or stop latch, and the build lease (held,
    unreleased, taken under this build grant)."""
    data, grant_sha = build_grant()
    if LATCH.exists():
        refuse("GPU and target already released by this packet")
    if STOP.exists():
        refuse("a failed step latched stop.json; only release and analysis may run")
    if not LEASE.is_file():
        refuse("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("released_at") or lease.get("grant_sha256") != grant_sha:
        refuse("lease released, or the build grant changed since the lease was taken")
    return data, grant_sha


def gate():
    """GPU grant, then the release latch and the stop latch. GPU children need no
    Cargo target: they run the frozen binary the grant binds by its build record."""
    data, grant_sha = grant()
    if LATCH.exists():
        refuse("GPU and target already released by this packet")
    if STOP.exists():
        refuse("a failed step latched stop.json; only release and analysis may run")
    return data, grant_sha


def granted_target():
    """The build grant's target when one is readable; never raises."""
    try:
        data = read_json(BUILD_GRANT)
    except (OSError, ValueError):
        return None
    value = data.get("target") if isinstance(data, dict) else None
    return Path(value) if target_allowed(value) else None


def prior_receipts(data, required):
    """Every prior_<name>_path / _sha256 pin must verify, and every pinned release
    body must list no live process."""
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


def tools_identity(granted_tree):
    """The committed visual tree is what runs: no local edits or untracked files,
    and HEAD's tree for it is the one the grant names."""
    dirty = git("status", "--porcelain", "--untracked-files=all", "--", VISUAL_PATH).strip()
    if dirty:
        refuse("visual tools, plan or helper differ from HEAD: " + dirty[:300])
    tree = git("rev-parse", "HEAD:" + VISUAL_PATH).strip()
    if tree != granted_tree:
        refuse(f"HEAD visual tree {tree} is not the granted {granted_tree}")
    return {"head": git("rev-parse", "HEAD").strip(), "visual_tree": tree,
            "plan_sha256": sha(PLAN_PATH), "helper_sha256": sha(HELPER_SOURCE)}


def git_blob_sha1(path):
    data = Path(path).read_bytes()
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def verify_checkout():
    """Every exported file still matches its blob; the only extra file allowed is
    the review helper copy, and only with the committed helper's bytes."""
    manifest = read_json(BUILD / "checkout.json")
    for entry in manifest["files"]:
        path = CHECKOUT / entry["path"]
        if path.is_symlink() or git_blob_sha1(path) != entry["blob"]:
            refuse("checkout file changed: " + entry["path"])
    expected = {entry["path"] for entry in manifest["files"]}
    extra = []
    for directory, _, names in os.walk(CHECKOUT):
        for name in names:
            relative = str(Path(directory, name).relative_to(CHECKOUT))
            if relative not in expected:
                extra.append(relative)
    helper = CHECKOUT / HELPER_DEST
    if extra not in ([], [HELPER_DEST]):
        refuse("unexpected files in the checkout: " + ", ".join(sorted(extra)[:10]))
    if extra and sha(helper) != sha(HELPER_SOURCE):
        refuse("the checkout helper copy differs from the committed helper")
    return {"commit": manifest["commit"], "tree": manifest["tree"],
            "files": len(manifest["files"]), "manifest_sha256": sha(BUILD / "checkout.json"),
            "helper_copy_sha256": sha(helper) if extra else None}


def child_env(overrides):
    unknown = set(overrides) - set(OVERRIDES)
    if unknown:
        raise SystemExit("refused: override outside the documented allowlist: " + ", ".join(sorted(unknown)))
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


def raise_interrupt(signum, _frame):
    raise KeyboardInterrupt(f"signal {signum}")


def install_interrupts():
    """SIGTERM and SIGHUP become KeyboardInterrupt, like SIGINT."""
    for signum in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, raise_interrupt)


class blocked:
    """INT/TERM/HUP stay pending from the debit until the attempt is recorded; a
    pending interrupt is delivered on exit, after recording. Single-threaded
    runners only (a mask is per thread)."""

    def __enter__(self):
        self.previous = signal.pthread_sigmask(signal.SIG_BLOCK, BLOCKED)
        return self

    def __exit__(self, *_):
        signal.pthread_sigmask(signal.SIG_SETMASK, self.previous)
        return False


def default_mask():
    """preexec_fn: the child starts with no blocked signals."""
    signal.pthread_sigmask(signal.SIG_SETMASK, [])


def launch(argv, cwd, env, stdout, stderr, timeout):
    """Run one child in its own session and process group inside blocked(). The
    runner polls the child and the pending-signal set, so neither a timeout nor an
    interrupt can skip the SIGKILL of the whole group and the reap."""
    started = time.monotonic()
    child = subprocess.Popen(argv, cwd=cwd, env=env, stdout=stdout, stderr=stderr,
                             start_new_session=True, preexec_fn=default_mask)
    timed_out = False
    interrupted = []
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
    finally:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass  # also covers exit racing the timeout
        code = child.wait()
    try:
        os.killpg(child.pid, 0)
        group_empty = False
    except ProcessLookupError:
        group_empty = True
    return {"pid": child.pid, "exit": code, "timed_out": timed_out,
            "interrupted_by": interrupted, "group_empty_after": group_empty,
            "seconds": round(time.monotonic() - started, 3)}


def launch_recorded(argv, cwd, env, stdout, stderr, timeout):
    """launch(), with a failed exec recorded instead of raised (the debit stands)."""
    try:
        return launch(argv, cwd, env, stdout, stderr, timeout)
    except Exception as error:  # e.g. exec failure; there is no child to reap
        return {"pid": None, "exit": None, "timed_out": False, "interrupted_by": [],
                "group_empty_after": True, "seconds": 0.0, "launch_error": repr(error)}
