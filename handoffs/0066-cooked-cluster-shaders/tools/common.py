"""Shared constants and guards for handoff 0066 tools. Standard library only.

Adapted from handoff 0064's committed tools/common.py (read-only reuse). Nothing
here starts a renderer. Guards refuse unless Astra's grant names this packet,
its exact source commit and the target; the frozen source still matches that
commit; this packet's target lease is held; and (for GPU work) the packet's own
release latch is absent.
"""

import datetime
import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = "0066-cooked-cluster-shaders"
PACKET_DIR = ROOT / "handoffs" / PACKET
TOOLS = PACKET_DIR / "tools"
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0066")
GRANT = EVIDENCE / "review-ready.json"
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
REFUSALS = EVIDENCE / "gpu-refusals.jsonl"
LOCK = EVIDENCE / "gpu.lock"
SOURCE_COMMIT = "96bb23df8eab86e00bef5b22e85525d36dd84972"
MANIFEST = ROOT / "docs/spikes/evidence/cooked-cluster-shaders.json"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
BASE = ("incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,"
        "incant_render/cooked-ui,incant_render/cooked-text,incant_render/cooked-materials")
VARIANTS = {"baseline": BASE, "candidate": BASE + ",incant_render/cooked-clusters"}
MAX_FRAMES, MAX_W, MAX_H = 24, 960, 540
# Production inputs whose bytes must not move during the interval (besides the 19 manifest inputs).
FROZEN_PATHS = ["crates", "Cargo.lock", "Cargo.toml", "rust-toolchain.toml", "third_party", "tools/cargo"]


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
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout


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
    """HEAD is the granted commit, production inputs are unmodified and the manifest inputs still match."""
    head = git("rev-parse", "HEAD").strip()
    if head != SOURCE_COMMIT:
        raise SystemExit(f"HEAD {head} is not the granted source commit; refusing")
    dirty = git("status", "--porcelain", "--untracked-files=all", "--", *FROZEN_PATHS).strip()
    if dirty:
        raise SystemExit(f"frozen production paths changed: {dirty[:400]}; refusing")
    digest = hashlib.sha256()
    for source in read_json(MANIFEST)["sources"]:
        actual = sha(ROOT / source["path"])
        if actual != source["sha256"]:
            raise SystemExit(f"{source['path']} no longer matches the reviewed manifest; refusing")
        digest.update(f"{source['path']} {actual}\n".encode())
    return {"head": head, "manifest_inputs": len(read_json(MANIFEST)["sources"]),
            "manifest_inputs_digest": digest.hexdigest()}


def check_lease():
    check_grant()
    if not LEASE.is_file():
        raise SystemExit("target lease absent; run lease.py acquire first")
    lease = read_json(LEASE)
    if lease.get("target_dir") != str(TARGET) or lease.get("released_at"):
        raise SystemExit("target lease missing or released; refusing to build")
    return lease


def build_env(extra=None):
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(("CARGO_", "RUSTFLAGS", "RUSTDOCFLAGS", "RUSTC_", "INCANT_", "WGPU_"))}
    env["PATH"] = RUSTUP_BIN + ":" + env.get("PATH", "/usr/bin:/bin")
    env["CARGO_TARGET_DIR"] = str(TARGET)
    env.update(extra or {})
    return env


def run_env():
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(("INCANT_", "RUST_", "WGPU_", "CARGO_", "MTL_", "METAL_"))}
    env["RUST_BACKTRACE"] = "1"
    return env
