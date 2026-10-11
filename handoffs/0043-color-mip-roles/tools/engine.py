"""Reuse the audited0040 CLI harness with a new immutable binary/slot boundary.

All calls, including import/rpc, pass the guard immediately before Popen. Never
invoke the original0040 harness directly for this packet. Usage:
    engine = configure('candidate')  # or 'baseline'
    engine.init(...); engine.rpc(...); engine.screenshot(...)
The shared ledger/capture cap covers both frozen binaries. Calls are serial.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
ORIGINAL = ROOT / "handoffs/0040-static-fbx-import/tools"
ART = ROOT / "artifacts/0043-color-mip-roles"


class CaptureClosed(RuntimeError):
    pass


class Guard:
    def __init__(self, binary, grant, token):
        self.binary, self.grant, self.token = str(binary), Path(grant), token

    def __getattr__(self, name):
        return getattr(subprocess, name)

    def Popen(self, command, *args, **kwargs):
        if not isinstance(command, list) or command[0] != self.binary:
            raise CaptureClosed("only the selected frozen binary is allowed")
        try:
            current = self.grant.read_text().strip()
        except FileNotFoundError:
            raise CaptureClosed("0043 slot is closed; wait for Astra") from None
        if current != self.token:
            raise CaptureClosed("0043 slot does not match this frozen source and binary")
        return subprocess.Popen(command, *args, **kwargs)


def configure(which):
    if which not in ("baseline", "candidate"):
        raise ValueError("select baseline or candidate")
    # Separate module instances avoid accidentally switching an already-selected
    # harness when a control selects the other binary. No production code changes.
    sys.path.insert(0, str(ORIGINAL))
    spec = importlib.util.spec_from_file_location(f"color_roles_{which}", ORIGINAL / "engine.py")
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    metadata = ROOT / f"artifacts/tools/color-{which}.json"
    info = json.loads(metadata.read_text())
    harness.BINARY = ROOT / f"artifacts/tools/incant_headless-color-{which}"
    harness.BINARY_JSON = metadata
    harness.ART = ART
    harness.LEDGER = ART / "ledger.jsonl"
    harness.MAX_CAPTURES = 20
    harness.MAX_BYTES = 64 * 1024 * 1024
    harness.ART.mkdir(parents=True, exist_ok=True)
    token = info["source_commit"] + ":" + info["sha256"]
    harness.subprocess = Guard(harness.BINARY, ROOT / f"artifacts/tools/color-{which}-ready", token)
    harness.check_binary()
    return harness
