"""Bind trusted export selection to the authored physics mode and compiled kernel.

The Rust document schema remains the final authority. This bounded preflight
chooses only fixed engine feature names; it never accepts project Cargo flags.
"""
import json
from native_package import require

MODES = ("deterministic", "fast")


def project_mode(payload):
    document = json.loads(payload)
    require(isinstance(document, dict) and isinstance(document.get("settings"), dict),
            "Missing project settings for physics selection")
    mode = document["settings"].get("physics_mode", "deterministic")
    require(isinstance(mode, str) and mode in MODES, "Unsupported project physics mode")
    return mode


def cargo_features(mode, capabilities):
    require(mode in MODES, "Unsupported project physics mode")
    # Eight-lane fast is a measured target optimization, not an authored mode.
    # No target is promoted to it without the plan's matched measurements.
    return list(capabilities) + (["physics-deterministic"] if mode == "deterministic" else [])


def verify_producer(receipt, mode):
    require(mode in MODES, "Unsupported project physics mode")
    require(receipt.get("physics_mode") == mode and receipt.get("physics_variant") == mode,
            "Project cook physics differs from the selected export kernel")
