#!/usr/bin/env python3
"""CPU-only mutation checks for the 0070 runner controls. Standard library only.

Each mutation re-introduces one defect from root review revision 2 inside the
end-to-end harness process only -- the committed helpers are never modified --
and runs the selftest control that guards it. Every control must then fail.
Scratch lives in `handoffs/0070-ambient-occlusion/.mutation-<pid>/` and is
removed. Requires committed helpers, like selftest.py.

  python3 -I -B mutations.py
"""

import json
import os
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import selftest as s  # noqa: E402

ANCHOR = "sys.exit(runner.run_command(argv, plan))"
MUTATIONS = {
    "separate packets do not share a GPU lock": (
        "runner.c.shared_gpu_lock_path = lambda: rig_root / 'own-packet.lock'\n",
        ("main", "other"), "e2e_two_packets"),
    "a failed child does not latch": (
        "runner.Ledger.failures = lambda self: {}\n",
        ("latch",), "e2e_latch"),
    "ownership by a second-resolution start time": (
        "runner.lifecycle.same_process = lambda cur, ret: bool(cur and ret) and cur['pid'] == ret['pid'] "
        "and abs(cur['start'][0] - ret['start'][0]) <= 1\n",
        ("foreign",), "e2e_recycled"),
    "survivors or a failed reconcile unblock": (
        "runner.Ledger.group_verified = staticmethod(lambda entry: entry['complete'] is not None "
        "or bool(entry['reconcile']))\n",
        ("survivor",), "e2e_survivors"),
}


def main():
    base = s.E2E
    if ANCHOR not in base:
        raise SystemExit("selftest harness changed; mutation anchor not found")
    report = {}
    try:
        for name, (patch, rig_names, control) in MUTATIONS.items():
            s.E2E = base.replace(ANCHOR, patch + ANCHOR)
            scratch = s.c.ROOT / s.c.PACKET_REL / f".mutation-{os.getpid()}"
            scratch.mkdir()
            rigs = {}
            try:
                for rig in rig_names:
                    rigs[rig] = s.Rig(scratch, rig)
                s.RESULTS.clear()
                getattr(s, control)(scratch, rigs)
                result = s.RESULTS[-1]
                report[name] = {"control": control, "caught": not result["ok"],
                                "failure": result.get("error", "")[:200]}
            finally:
                s.sweep(rigs)
                shutil.rmtree(scratch, ignore_errors=True)
    finally:
        s.E2E = base
    print(json.dumps(report, indent=1))
    return 0 if report and all(r["caught"] for r in report.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
