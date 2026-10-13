"""Shared constants and helpers for handoff 0063 tools. Standard library only.

Every tool runs as `python3 -I -B handoffs/0063-cooked-ui-text-shaders/tools/<tool>.py`
and inserts this directory into sys.path itself (-I does not).
"""

import datetime
import hashlib
import json
import os
from pathlib import Path

PACKET = "0063-cooked-ui-text-shaders"
TOOLS = Path(__file__).resolve().parent
PACKET_DIR = TOOLS.parent
ROOT = TOOLS.parents[2]
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0063")
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
GRANT = EVIDENCE / "review-ready.json"
LEASE = EVIDENCE / "target-lease.json"
LATCH = EVIDENCE / "gpu-released.json"
SOURCE_COMMIT = "bacdd6f29bee40a1a14e61b2a224344c26d14da5"
UI_COMMITS = ("f8b95de5161eed891fde419083d7c0dead34fdba", "972a36f1141c4bad338674ea480a1cdc699d9b8a")
BASE = "incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites"
VARIANTS = {
    "baseline": BASE,
    "candidate": BASE + ",incant_render/cooked-ui,incant_render/cooked-text",
    # Diagnosis only (brief: one added feature at a time if a comparison fails).
    "ui-only": BASE + ",incant_render/cooked-ui",
    "text-only": BASE + ",incant_render/cooked-text",
}
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
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write_new_json(path, value):
    """Create a new JSON file; never overwrite evidence."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "x", encoding="utf-8") as out:
        json.dump(value, out, indent=1, ensure_ascii=False)
        out.write("\n")


def grant():
    """Astra's grant must exist and name this packet; returns its SHA-256."""
    if not GRANT.is_file():
        raise SystemExit("review-ready.json is absent; refusing")
    if read_json(GRANT).get("packet") != PACKET:
        raise SystemExit("review-ready.json does not name this packet; refusing")
    return sha(GRANT)


def check_lease():
    """The exclusive target lease must be held and not released."""
    grant()
    if not LEASE.is_file():
        raise SystemExit("target lease missing; refusing")
    lease = read_json(LEASE)
    if lease.get("target_dir") != str(TARGET) or lease.get("released_at"):
        raise SystemExit("target lease released or foreign; refusing")
    return lease


def clean_env(extra=None):
    """Environment without INCANT_/RUST_/WGPU_/CARGO_/Metal overrides."""
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(("INCANT_", "RUST_", "WGPU_", "CARGO_", "MTL_", "METAL_", "RUSTFLAGS",
                                "RUSTDOCFLAGS"))
           and k not in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER")}
    if extra:
        env.update(extra)
    return env
