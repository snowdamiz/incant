"""Shared constants and guards for handoff 0068 tools. Standard library only.

Adapted from handoff 0066's committed tools/common.py and lease.py (read-only
reuse of the 0064/0066 environment-safe logger pattern; nothing from 0065's
environment logger). Importing this module has no side effects.

Every mutating entry point calls grant() first. Until Astra's review-ready.json
exists, tools refuse without creating any file or directory. The grant must name
this packet, the expected target and a full source commit. The packet's GPU
release latch makes every tool refuse afterwards.

Child environments are constructed from scratch: HOME is passed through
(its value is never recorded) and every other variable is an explicit override
from OVERRIDES, recorded verbatim in receipts. The inherited environment, ACP
or session transport values, credentials and account metadata are never
serialized.
"""

import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = "0068-gpu-timestamps"
PACKET_DIR = ROOT / "handoffs" / PACKET
TOOLS = PACKET_DIR / "tools"
PLAN_PATH = PACKET_DIR / "plan.json"
HELPER_SOURCE = TOOLS / "review" / "gpu_timestamp_review.rs"
HELPER_DEST = "crates/incant_render/tests/gpu_timestamp_review.rs"
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0068")
GRANT = EVIDENCE / "review-ready.json"
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
ROOT_TARGET = Path("/Users/andreyyurlov/.codex/incant-gpu-timestamps-target")
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
REFUSALS = EVIDENCE / "gpu-refusals.jsonl"
LOCK = EVIDENCE / "gpu.lock"
CHECKOUT = EVIDENCE / "checkout"
BUILD = EVIDENCE / "build"
BIN = EVIDENCE / "bin"
LOGS = EVIDENCE / "logs"
SCRATCH = EVIDENCE / "tmp"
CAPTURES = EVIDENCE / "captures"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
FREEZE = ROOT / "docs/spikes/evidence/gpu-timestamps/freeze.json"
INHERITED = ("HOME",)
# The only variables a child receives besides INHERITED, all nonsecret.
OVERRIDES = {
    "PATH": "fixed tool search path (rustup proxies for builds, system only for tests)",
    "TMPDIR": "fresh packet scratch directory under the evidence directory",
    "RUST_BACKTRACE": "1, for panic locations in test children",
    "CARGO_TARGET_DIR": "the granted Cargo target directory (builds only)",
    "CARGO_TERM_COLOR": "never, so build logs carry no terminal escapes",
    "RUSTUP_AUTO_INSTALL": "0, so rustup never downloads a toolchain",
    "INCANT_0068_REVIEW_OUT": "new output directory for one review-helper child",
}


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


def plan():
    return read_json(PLAN_PATH)


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True,
                          check=True).stdout


def refuse(reason):
    """Record a refusal only once the grant exists; before it, touch nothing."""
    if GRANT.is_file():
        with open(REFUSALS, "a") as log:
            log.write(json.dumps({"at": now(), "reason": reason, "argv": sys.argv[1:]}) + "\n")
    raise SystemExit("refused: " + reason)


def grant():
    """Astra's grant must name this packet, the target and a full source commit."""
    if not GRANT.is_file():
        raise SystemExit("refused: review-ready.json is absent; preparation only")
    data = read_json(GRANT)
    commit = data.get("source_commit")
    if (data.get("packet") != PACKET or data.get("target") != str(TARGET)
            or not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit)):
        refuse("review-ready.json does not name this packet, the target and a source commit")
    if TARGET == ROOT_TARGET:
        refuse("the granted target is root's own target")
    return data, sha(GRANT)


def gate():
    """Grant, then the release latch, then the lease (held, unreleased, same grant)."""
    data, grant_sha = grant()
    if LATCH.exists():
        refuse("GPU and target already released by this packet")
    if not LEASE.is_file():
        refuse("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("released_at") or lease.get("grant_sha256") != grant_sha:
        refuse("lease released, or the grant changed since the lease was taken")
    return data, grant_sha


def tools_identity():
    """The committed packet tools are what runs: no local edits or untracked files."""
    paths = ["handoffs/" + PACKET + "/tools", "handoffs/" + PACKET + "/plan.json"]
    dirty = git("status", "--porcelain", "--untracked-files=all", "--", *paths).strip()
    if dirty:
        refuse("packet tools or plan differ from HEAD: " + dirty[:300])
    return {"head": git("rev-parse", "HEAD").strip(),
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
        if entry["mode"] == "120000":
            if not path.is_symlink() or os.readlink(path) != entry["link"]:
                refuse("checkout symlink changed: " + entry["path"])
        elif path.is_symlink() or git_blob_sha1(path) != entry["blob"]:
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
