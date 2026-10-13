"""Shared constants and guards for handoff 0067 tools. Standard library only.

Adapted from handoff 0066's tools/common.py (read-only reuse of the
environment-safe logger and guards; the 0065 logger is not used). Nothing here
starts a renderer. Guards refuse unless Astra's grant names this packet, its
exact source commit and the target; the frozen source still matches that commit
and the reviewed manifest; this packet's target lease is held; and (for GPU
work) the packet's own release latch is absent.

Environment: children never inherit the session environment. Builds and runs get
an explicit minimal environment, and receipts record only ALLOWLIST settings
(HOME is recorded as set/unset, never its value).
"""

import datetime
import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = "0067-dynamic-resolution"
PACKET_DIR = ROOT / "handoffs" / PACKET
TOOLS = PACKET_DIR / "tools"
HELPER = TOOLS / "drs"
CASES = TOOLS / "cases.json"
PLAN = TOOLS / "appearance.json"
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0067")
GRANT = EVIDENCE / "review-ready.json"
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
REFUSALS = EVIDENCE / "gpu-refusals.jsonl"
LOCK = EVIDENCE / "gpu.lock"
FREEZE = EVIDENCE / "build" / "source-freeze.json"
SOURCE_COMMIT = "bdd1b205aa34f3494da2fa49241b99efc502ec3e"
IMPLEMENTATION_COMMIT = "1a1c6c07e62e67310c0f51ec4556344a8d67f4d6"
BASE_COMMIT = "d5702546b34fcc6452a9fe2b22a0e2b23802991a"
MANIFEST = ROOT / "docs/spikes/evidence/dynamic-resolution.json"
PRIOR = EVIDENCE.parent / "0066"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
NUMERIC_MAX, APPEARANCE_MAX, MAX_W, MAX_H = 48, 24, 960, 540
# Repository inputs whose bytes must not move during the interval.
FROZEN_PATHS = ["crates", "Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "third_party", "tools",
                ".cargo", "docs", "samples", "schemas"]
# Nonsecret settings only: paths under the evidence directory and fixed values.
ALLOWLIST = ("PATH", "TMPDIR", "CARGO_TARGET_DIR", "RUST_BACKTRACE", "INCANT_0067_FRAMES", "INCANT_0067_OUT")


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


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True,
                          env=minimal_env()).stdout


def minimal_env(extra=None):
    env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": os.environ["HOME"]}
    env.update(extra or {})
    return env


def recorded_env(env):
    """Receipt form of a child environment: allowlisted values plus key names only."""
    return {"allowlisted": {k: env[k] for k in ALLOWLIST if k in env},
            "HOME": "set" if "HOME" in env else "unset",
            "keys": sorted(env)}


def build_env():
    tmp = EVIDENCE / "tmp" / "build"
    tmp.mkdir(parents=True, exist_ok=True)
    return minimal_env({"PATH": RUSTUP_BIN + ":/usr/bin:/bin:/usr/sbin:/sbin", "CARGO_TARGET_DIR": str(TARGET),
                        "TMPDIR": str(tmp)})


def run_env():
    tmp = EVIDENCE / "tmp" / "run"
    tmp.mkdir(parents=True, exist_ok=True)
    return minimal_env({"TMPDIR": str(tmp), "RUST_BACKTRACE": "1"})


def check_grant():
    """Astra's grant must exist and name this packet, its source commit and the leased target."""
    if not GRANT.is_file():
        raise SystemExit("review-ready.json is absent; refusing")
    grant = read_json(GRANT)
    if (grant.get("packet") != PACKET or grant.get("target") != str(TARGET)
            or grant.get("source_commit") != SOURCE_COMMIT):
        raise SystemExit("review-ready.json does not grant this packet, source commit and target; refusing")
    return sha(GRANT)


def check_source():
    """HEAD is the granted commit, repository inputs are unmodified and the manifest inputs match."""
    head = git("rev-parse", "HEAD").strip()
    if head != SOURCE_COMMIT:
        raise SystemExit(f"HEAD {head} is not the granted source commit; refusing")
    dirty = git("status", "--porcelain", "--untracked-files=all", "--", *FROZEN_PATHS).strip()
    if dirty:
        raise SystemExit(f"frozen repository paths changed: {dirty[:400]}; refusing")
    digest = hashlib.sha256()
    sources = read_json(MANIFEST)["sources"]
    for source in sources:
        actual = sha(ROOT / source["path"])
        if actual != source["sha256"]:
            raise SystemExit(f"{source['path']} no longer matches the reviewed manifest; refusing")
        digest.update(f"{source['path']} {actual}\n".encode())
    return {"head": head, "manifest_inputs": len(sources), "manifest_inputs_digest": digest.hexdigest()}


def helper_files():
    """Packet helper sources (no build output lives here; the target is external)."""
    return sorted(p for p in HELPER.rglob("*") if p.is_file())


def helper_digest():
    digest = hashlib.sha256()
    files = {}
    for path in helper_files():
        files[str(path.relative_to(ROOT))] = sha(path)
        digest.update(f"{path.relative_to(ROOT)} {files[str(path.relative_to(ROOT))]}\n".encode())
    for path in (CASES, PLAN):
        files[str(path.relative_to(ROOT))] = sha(path)
        digest.update(f"{path.relative_to(ROOT)} {files[str(path.relative_to(ROOT))]}\n".encode())
    return digest.hexdigest(), files


def check_lease():
    check_grant()
    if not LEASE.is_file():
        raise SystemExit("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("target_dir") != str(TARGET) or lease.get("released_at"):
        raise SystemExit("target lease missing or released; refusing to build")
    return lease
