#!/usr/bin/env python3
"""Complete command/exit ledger and release receipt for handoff 0063. CPU only.

Builds, engine calls and GPU children come from their own machine records
(build/*.json, fixtures/*/engine-calls.jsonl, gpu-ledger.jsonl). Every other
command run in this packet's session is listed explicitly below, in order,
with its observed exit status, including failed and refused attempts.

  python3 -I -B ledger.py <new-commands-json> <new-release-receipt-json>
"""

import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

T = "python3 -I -B handoffs/0063-cooked-ui-text-shaders/tools/"
E = str(c.EVIDENCE)
# (command, exit, note) in session order; builds, engine calls and GPU children are expanded from records.
SESSION = [
    (T + "lease.py acquire", 0, "target lease after Astra's review-ready.json named 0063"),
    (T + "build.py freeze", 0, "build/source-freeze.json"),
    (T + "build.py helper-lock", 1, "cargo metadata --offline exited 0 and wrote the pruned lock; my verifier then "
                                    "rejected it (it merged [[patch.unused]] tao into a package block and compared "
                                    "dependency references without Cargo's version-suffix elision)"),
    (T + "build.py verify-lock", 1, "same verifier defect; no record written"),
    (T + "build.py verify-lock", 0, "corrected verifier: 375 identical entries, 0 added edges"),
    ("background: build.py tools > logs/step-tools.txt", 1,
     "shell redirect failed before Python started (logs/ did not exist yet); no cargo ran"),
    ("background: build.py tools; build.py baseline; build.py candidate", 0, "see build records"),
    (T + "author.py build " + E + "/fixtures/r1", 0, "superseded: board W lacked exact in-scene controls"),
    (T + "embed.py " + E + "/analysis/embed.json", 0, "static shader-byte check of frozen binaries"),
    (T + "author.py build " + E + "/fixtures/r2", 0, "superseded after two baseline frames"),
    ("GPU children: 8 lists, 12 test groups, 2 r2 captures", 0, "see gpu-ledger.jsonl"),
    ("mv captures/baseline/{W-ortho,U-1x-unorm} captures-r2-superseded/baseline/", 0,
     "r2 frames kept and counted; MOVED.json"),
    (T + "author.py build " + E + "/fixtures/r3", 0, "yaw -60 deg; Arabic field long enough to overflow"),
    ("GPU children: 12 r3 captures", 0, "U-15x children exited 1 in both variants; see gpu-ledger.jsonl"),
    (T + "author.py add-case " + E + "/fixtures/r3 U-15x-name", 0, "supplementary request"),
    ("GPU children: 2 U-15x-name captures", 0, "see gpu-ledger.jsonl"),
    (T + "author.py source-free " + E + "/fixtures/r3 W", 0, "9 sources removed; caches unchanged"),
    (T + "author.py source-free " + E + "/fixtures/r3 U", 0, "9 sources removed; caches unchanged"),
    ("GPU children: 2 source-free candidate captures", 0, "see gpu-ledger.jsonl"),
    (T + "gpu.py release", 0, "gpu-released.json"),
    (T + "gpu.py test candidate lib after-release-check", 1, "expected refusal: latch present; no child or log"),
    (T + "gpu.py capture candidate W-ortho", 1, "refused by the source-free mode check before the latch check"),
    (T + "gpu.py capture candidate W-ortho --source-free", 1, "expected refusal: latch present; no child or log"),
    (T + "gpu.py list candidate ui", 1, "expected refusal: latch present; no child or log"),
    ("rustfmt --edition 2024 --check tools/review/src/main.rs", 1, "one line-break difference"),
    ("./tools/cargo clippy --manifest-path tools/review/Cargo.toml --locked --offline --profile ci -j2 "
     "--features <candidate> -- -D warnings", 0, "as-built source; logs/helper-clippy.txt"),
    ("cp tools/review/src/main.rs " + E + "/build/review-main.rs.as-built; rustfmt --edition 2024 main.rs; "
     "rustfmt --check", 0, "whitespace-only change; frozen binaries were built from the as-built bytes"),
    ("./tools/cargo clippy ... --features <baseline>; ... --features <candidate> -- -D warnings", 0,
     "formatted source; logs/helper-clippy-formatted.txt"),
    (T + "lease.py release", 1, "false positive: the process scan matched this shell's own command line"),
    (T + "lease.py release", 0, "target-lease.json released_at"),
    (T + "analyze.py " + E + " " + E + "/analysis/analysis-draft1.json", 0, "probes ignored clipping (draft)"),
    (T + "analyze.py " + E + " " + E + "/analysis/analysis-draft2.json", 0, "draft"),
    (T + "analyze.py " + E + " " + E + "/analysis/analysis.json", 0, "final"),
    (T + "selfcheck.py " + E + " " + E + "/analysis/selfcheck.json", 0, ""),
    (T + "figures.py " + E + " " + E + "/analysis/figures", 0, "first figure set; deleted and regenerated"),
    ("rm -r " + E + "/analysis/figures; " + T + "figures.py " + E + " " + E + "/analysis/figures", 0,
     "two figures re-laid out as columns to stay within 960x540"),
]


def jsonl(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    builds = []
    for record in sorted((c.EVIDENCE / "build").glob("*.json")):
        value = c.read_json(record)
        for build in value.get("builds", []):
            builds.append({"argv": build["argv"], "exit": build["exit"], "started": build["started"],
                           "ended": build["ended"], "record": record.name})
    engine = {run: [{"tool": e["tool"], "argv": e["argv"], "exit": e["exit"], "started": e["started"],
                     "ended": e["ended"]} for e in jsonl(c.EVIDENCE / "fixtures" / run / "engine-calls.jsonl")]
              for run in ("r1", "r2", "r3")}
    gpu = [{k: e[k] for k in ("label", "kind", "variant", "pid", "started", "ended", "exit", "timed_out")}
           for e in jsonl(c.EVIDENCE / "gpu-ledger.jsonl")]
    ledger = {"packet": c.PACKET, "session_order": [{"order": i, "command": cmd, "exit": code, "note": note}
                                                    for i, (cmd, code, note) in enumerate(SESSION)],
              "cargo_builds": builds, "engine_calls": engine, "gpu_children": gpu,
              "totals": {"cargo_builds": len(builds), "cargo_failures": sum(b["exit"] != 0 for b in builds),
                         "engine_calls": {k: len(v) for k, v in engine.items()},
                         "engine_failures": sum(e["exit"] != 0 for v in engine.values() for e in v),
                         "gpu_children": len(gpu), "gpu_nonzero": [g["label"] for g in gpu if g["exit"] != 0],
                         "gpu_timeouts": sum(g["timed_out"] for g in gpu)}}
    c.write_new_json(sys.argv[1], ledger)
    listing = subprocess.run(["ps", "-axo", "pid=,command="], capture_output=True, text=True).stdout
    owned = [line.strip() for line in listing.splitlines()
             if str(c.EVIDENCE / "bin") in line or (str(c.TARGET) in line and "ledger.py" not in line)]
    lease, gpu_release = c.read_json(c.LEASE), c.read_json(c.LATCH)
    frames = sorted(c.EVIDENCE.glob("captures*/*/*/*.png"))
    receipt = {"packet": c.PACKET, "recorded": c.now(),
               "gpu": {"released_at": gpu_release["released_at"], "children": gpu_release["children"],
                       "frames_written": gpu_release["frames_written"], "ledger_sha256": gpu_release["ledger_sha256"],
                       "release_record_sha256": c.sha(c.LATCH), "first_start": gpu_release["first_start"],
                       "last_end": gpu_release["last_end"]},
               "target_lease": {"target_dir": lease["target_dir"], "acquired_at": lease["acquired_at"],
                                "released_at": lease["released_at"], "lease_sha256": c.sha(c.LEASE),
                                "left_as_warm_cache": True, "deleted": []},
               "engine_frames_on_disk": len(frames), "frame_limit": c.MAX_FRAMES,
               "processes_using_frozen_binaries_or_target_now": owned,
               "grant_sha256_now": c.sha(c.GRANT), "grant_unmodified": c.sha(c.GRANT) == lease["grant_sha256"],
               "note": "review-ready.json is Astra's and was never modified or deleted. No GPU child after "
                       f"{gpu_release['released_at']}; no cargo after {lease['released_at']}."}
    c.write_new_json(sys.argv[2], receipt)
    print(json.dumps(ledger["totals"], indent=1))
    print(json.dumps({k: receipt[k] for k in ("engine_frames_on_disk", "processes_using_frozen_binaries_or_target_now",
                                              "grant_unmodified")}))


if __name__ == "__main__":
    main()
