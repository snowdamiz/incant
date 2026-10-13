#!/usr/bin/env python3
"""Read-only receipt for root's granted StageA run (handoff 0073). Stdlib only.

  python3 -I -B handoffs/0073-upscaling-review/tools/numeric_receipt.py

It never launches a process or touches the GPU, and it writes nothing under the
evidence directory. It re-verifies the release/ledger/debit hash chain, checks
that every ledger PID and process group is gone, parses the adapter line and all
28 scalar rows the numeric case printed, and compares each row with
expectations derived here from the test's fixture semantics (not copied from its
assertion table). Raw outputs are retained losslessly as deterministic gzip
named by their raw SHA-256 under evidence/numeric/, with a manifest receipt.
"""

import gzip
import hashlib
import json
import os
import re
from pathlib import Path

PACKET_DIR = Path(__file__).resolve().parents[1]
EVIDENCE = Path.home() / ".codex/incant-evidence/0073-upscaling-numeric"
OUT = PACKET_DIR / "evidence" / "numeric"
TESTS = [
    "upscaling::system::device_tests::actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay",
    "models::retained::upscaling_tests::upscaling_color_motion_geometry_and_dynamic_source_extent_commit_together",
]
RETAINED = ["review-ready.json", "attempt-started.json", "attempt-debits.jsonl", "gpu-ledger.jsonl",
            "gpu-results.json", "gpu-released.json", "summary.json", "runner-console.stdout",
            "runner-console.stderr"] + [t + suffix for t in TESTS for suffix in ("-gpu.stdout", "-gpu.stderr")]
ROW = re.compile(r"^upscaling mode(\d+) attachment(\d): \[([^\]]*)\]$", re.M)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def group_alive(pid):
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False


def expected(mode):
    """Fixture semantics at output pixel (3,3): output rect [1,1,6,6] over a 4x4
    source rect [0.5,0.5,3,3] samples four mid-gray current texels; history weight
    0.5, prior history 0.25. Rejection keeps current 0.5; acceptance mixes 0.375."""
    accepted, current = [0.375, 0.375, 0.375, 1.0], [0.5, 0.5, 0.5, 1.0]
    history = {
        0: accepted,                 # compatible perspective history
        1: accepted,                 # orthographic linear depth 50.05 matches
        2: current,                  # prior depth 10 incompatible
        3: current,                  # motion leaves the output viewport
        4: [0.0, 0.0, 0.0, 0.0],     # alpha-zero radiance residual is dropped
        5: [0.2, 0.1, 0.0, 0.5],     # partial coverage stays current-only
        6: current,                  # full reaction removes history weight
        7: accepted,                 # overlay never enters history
        8: [0.0, 0.5, 0.5, 1.0],     # negative current channel cleaned, current-only
        9: [0.0, 0.0, 0.0, 0.0],     # NaN current is cleaned to zero
        10: current,                 # negative history cannot authorize reuse
        11: current,                 # NaN motion
        12: current,                 # no previous frame
        13: [2.0, 0.5, 100.0, 1.0],  # bounded HDR survives unchanged
    }[mode]
    over = [0.1, 0.05, 0.0, 0.5] if mode == 7 else [0.0] * 4
    return history, [o + h * (1 - over[3]) for o, h in zip(over, history)]


def main():
    raw = {name: (EVIDENCE / name).read_bytes() for name in RETAINED}
    released = json.loads(raw["gpu-released.json"])
    ledger = [json.loads(line) for line in raw["gpu-ledger.jsonl"].decode().splitlines()]
    results = json.loads(raw["gpu-results.json"])
    grant = json.loads(raw["review-ready.json"])
    chain = {
        "release_ledger_sha256_matches": released["ledger_sha256"] == sha(raw["gpu-ledger.jsonl"]),
        "release_debits_sha256_matches": released["debits_sha256"] == sha(raw["attempt-debits.jsonl"]),
        "release_lists_no_live_processes": released["owned_processes"] == [] and released["ledger_pids_alive"] == [],
        "grant_runner_sha256_matches_started": json.loads(raw["attempt-started.json"])["runner_sha256"] == grant["runner_sha256"],
        "children_two_in_test_order": [row["name"] for row in ledger] == TESTS and released["children"] == 2,
        "ledger_equals_results": ledger == results,
        "every_child_exit_zero_no_timeout": all(r["exit"] == 0 and not r["timeout"] and r["wait_error"] is None for r in ledger),
        "binary_sha256_is_granted_copy": all(r["binary_sha256"] == grant["executable_copy"]["sha256"] for r in ledger),
        "stdout_stderr_hashes_match": all(
            sha(raw[r["name"] + "-gpu.stdout"]) == r["stdout"]["sha256"]
            and sha(raw[r["name"] + "-gpu.stderr"]) == r["stderr"]["sha256"] for r in ledger),
        "ledger_pids_and_groups_gone_now": not any(alive(r["pid"]) or group_alive(r["pid"]) for r in ledger),
        "granted_copy_sha256_unchanged": sha((EVIDENCE / "incant_render-upscale-tests").read_bytes()) == grant["executable_copy"]["sha256"],
    }
    first = raw[TESTS[0] + "-gpu.stdout"].decode()
    adapter = [line.split("upscaling adapter: ", 1)[1] for line in first.splitlines()
               if "upscaling adapter: " in line]
    rows = []
    for mode, attachment, values in ROW.findall(first):
        actual = [float(v) for v in values.split(",")]
        want = expected(int(mode))[int(attachment)]
        rows.append({"mode": int(mode), "attachment": int(attachment), "actual": actual, "expected": want,
                     "max_abs_error": max(abs(a - b) for a, b in zip(actual, want))})
    numeric = {
        "adapter_lines": adapter,
        "rows": rows,
        "row_count": len(rows),
        "all_modes_both_attachments": sorted((r["mode"], r["attachment"]) for r in rows)
        == [(m, a) for m in range(14) for a in range(2)],
        "worst_abs_error": max(r["max_abs_error"] for r in rows) if rows else None,
        "within_test_tolerance_0_001": all(r["max_abs_error"] < 0.001 for r in rows),
        "libtest_lines": {t: [line for line in raw[t + "-gpu.stdout"].decode().splitlines()
                              if line.startswith("test result:")] for t in TESTS},
    }
    OUT.mkdir(parents=True, exist_ok=True)
    artifacts = []
    for name in RETAINED:
        data = raw[name]
        packed = gzip.compress(data, 9, mtime=0)
        target = OUT / (sha(data) + ".gz")
        if target.exists() and target.read_bytes() != packed:
            raise SystemExit(f"{target} exists with different bytes")
        target.write_bytes(packed)
        artifacts.append({"original": str(EVIDENCE / name), "raw_sha256": sha(data), "raw_bytes": len(data),
                          "retained": str(target.relative_to(PACKET_DIR.parents[1])),
                          "retained_sha256": sha(packed)})
    receipt = {"evidence_dir": str(EVIDENCE), "chain": chain, "chain_all_pass": all(chain.values()),
               "numeric": numeric, "artifacts": artifacts,
               "limits": ["Metal on the recorded adapter only; not a benchmark",
                          "scalar readback rows only; no image or appearance evidence",
                          "the retained case prints no rows; its pass is its own assertions"]}
    (OUT / "receipt.json").write_text(json.dumps(receipt, indent=1) + "\n")
    print(json.dumps({"chain_all_pass": receipt["chain_all_pass"], "rows": len(rows),
                      "worst_abs_error": numeric["worst_abs_error"], "adapter": adapter}, indent=1))


if __name__ == "__main__":
    main()
