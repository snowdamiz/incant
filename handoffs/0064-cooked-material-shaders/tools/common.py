"""Shared constants and guards for handoff 0064 tools. Standard library only.

Nothing here starts a process. Guards refuse unless Astra's grant names this
packet, this packet's target lease is held, and (for GPU work) the packet's own
release latch is absent.
"""

import datetime
import hashlib
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PACKET = "0064-cooked-material-shaders"
PACKET_DIR = ROOT / "handoffs" / PACKET
TOOLS = PACKET_DIR / "tools"
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0064")
GRANT = EVIDENCE / "review-ready.json"
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
LOCK = EVIDENCE / "gpu.lock"
SOURCE_COMMIT = "6bcb839a7264a12fbcb3666041e17b3445468b09"
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
BASE = ("incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites,"
        "incant_render/cooked-ui,incant_render/cooked-text")
VARIANTS = {"baseline": BASE, "candidate": BASE + ",incant_render/cooked-materials"}
MAX_FRAMES, MAX_W, MAX_H = 24, 960, 540


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


def check_grant():
    """Astra's grant must exist, name this packet and name the leased target."""
    if not GRANT.is_file():
        raise SystemExit("review-ready.json is absent; refusing")
    grant = read_json(GRANT)
    if grant.get("packet") != PACKET or grant.get("target") != str(TARGET):
        raise SystemExit("review-ready.json does not grant this packet and target; refusing")
    return sha(GRANT)


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


def frozen(variant, name):
    record = read_json(EVIDENCE / "build" / f"{variant}.json")["frozen"][name]
    return Path(record["path"]), record["sha256"]
