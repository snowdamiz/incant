#!/usr/bin/env python3
"""CPU-only frozen builds for handoff 0062. Standard library only.

Runs ./tools/cargo with --locked, --profile ci and -j2 in the leased target
directory, parses Cargo's JSON artifact messages, copies every produced
executable into the ignored evidence directory and records SHA-256 values.
It never runs a built executable.

  python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py <step>

Steps: baseline | candidate | cook | headless | freeze
"""

import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0062")
TARGET = Path("/Users/andreyyurlov/.codex/incant-core-presentation-target")
LEASE = EVIDENCE / "target-lease.json"
SOURCE_COMMIT = "23f92d716bde0afc137b160c5497a502848d63b8"
BASE = "incant_render/cooked-output,incant_render/cooked-geometry"
VARIANTS = {"baseline": BASE, "candidate": BASE + ",incant_render/cooked-sprites"}


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def check_lease():
    lease = json.loads(LEASE.read_text())
    if lease["target_dir"] != str(TARGET) or lease.get("released_at"):
        raise SystemExit("target lease missing or released; refusing to build")


def cargo(label, args):
    check_lease()
    env = {k: v for k, v in os.environ.items() if not k.startswith(("CARGO_", "RUSTFLAGS", "INCANT_"))}
    env["CARGO_TARGET_DIR"] = str(TARGET)
    env["PATH"] = "/opt/homebrew/opt/rustup/bin:" + env.get("PATH", "/usr/bin:/bin")
    argv = [str(ROOT / "tools/cargo"), *args, "--locked", "--profile", "ci", "-j2",
            "--message-format=json-render-diagnostics"]
    logs = EVIDENCE / "logs"
    logs.mkdir(exist_ok=True)
    started = now()
    with open(logs / f"build-{label}.stdout.jsonl", "x") as out, open(logs / f"build-{label}.stderr.txt", "x") as err:
        result = subprocess.run(argv, cwd=ROOT, env=env, stdout=out, stderr=err)
    ended = now()
    artifacts = []
    for line in (logs / f"build-{label}.stdout.jsonl").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message.get("executable"):
            artifacts.append({"target": message["target"]["name"], "kind": message["target"]["kind"],
                              "test": message["profile"]["test"], "fresh": message["fresh"],
                              "executable": message["executable"], "features": message.get("features", []),
                              "package_id": message["package_id"]})
    record = {"label": label, "argv": argv, "cwd": str(ROOT), "env": {"CARGO_TARGET_DIR": str(TARGET)},
              "started": started, "ended": ended, "exit": result.returncode,
              "stdout_sha256": sha(logs / f"build-{label}.stdout.jsonl"),
              "stderr_sha256": sha(logs / f"build-{label}.stderr.txt"), "artifacts": artifacts}
    if result.returncode != 0:
        (EVIDENCE / "build" / f"{label}.failed.json").write_text(json.dumps(record, indent=1) + "\n")
        raise SystemExit(f"cargo {label} failed with {result.returncode}; see {logs}/build-{label}.stderr.txt")
    return record


def freeze(record, picks, variant):
    """Copy selected executables to EVIDENCE/bin/<variant>/<name> and hash them."""
    destination = EVIDENCE / "bin" / variant
    destination.mkdir(parents=True, exist_ok=True)
    frozen = {}
    for name, match in picks.items():
        found = [a for a in record["artifacts"] if match(a)]
        if len(found) != 1:
            raise SystemExit(f"{variant}/{name}: expected one artifact, found {len(found)}")
        source = Path(found[0]["executable"])
        target = destination / name
        if target.exists():
            raise SystemExit(f"{target} already exists; frozen binaries are never replaced")
        shutil.copyfile(source, target)
        os.chmod(target, 0o555)
        if sha(source) != sha(target):
            raise SystemExit(f"copy mismatch for {target}")
        frozen[name] = {"path": str(target), "sha256": sha(target), "bytes": target.stat().st_size,
                        "built_from": str(source), "features": found[0]["features"], "fresh": found[0]["fresh"]}
    return frozen


def write(name, value):
    path = EVIDENCE / "build" / f"{name}.json"
    path.parent.mkdir(exist_ok=True)
    with open(path, "x") as out:
        json.dump(value, out, indent=1)
        out.write("\n")
    print(json.dumps({k: v for k, v in value.items() if k != "builds"}, indent=1))


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout


def variant(name):
    features = VARIANTS[name]
    tests = cargo(f"{name}-tests", ["test", "--no-run", "-p", "incant_render", "--features", features])
    example = cargo(f"{name}-native-frames", ["build", "-p", "incant_render", "--features", features,
                                              "--example", "native_frames"])
    frozen = freeze(tests, {
        "incant_render-lib-tests": lambda a: a["test"] and a["kind"] == ["lib"],
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu",
    }, name)
    frozen.update(freeze(example, {"native_frames": lambda a: a["target"] == "native_frames"}, name))
    write(name, {"variant": name, "features": features, "head": git("rev-parse", "HEAD").strip(),
                 "frozen": frozen, "builds": [tests, example], "recorded": now()})


def tools(step):
    if step == "cook":
        record = cargo("cook-scene", ["build", "-p", "incant_cook", "--example", "cook_scene"])
        frozen = freeze(record, {"cook_scene": lambda a: a["target"] == "cook_scene"}, "tools")
    else:
        record = cargo("incant-headless", ["build", "-p", "incant_headless", "--bin", "incant_headless"])
        frozen = freeze(record, {"incant_headless": lambda a: a["target"] == "incant_headless"}, "tools")
    write(f"tools-{step}", {"step": step, "head": git("rev-parse", "HEAD").strip(), "frozen": frozen,
                            "builds": [record], "recorded": now()})


def source_freeze():
    manifest = json.loads((ROOT / "docs/spikes/evidence/cooked-sprite-shaders.json").read_text())
    sources = [{"path": s["path"], "manifest": s["sha256"], "worktree": sha(ROOT / s["path"]),
                "at_source_commit": hashlib.sha256(subprocess.run(
                    ["git", "show", f"{SOURCE_COMMIT}:{s['path']}"], cwd=ROOT, capture_output=True,
                    check=True).stdout).hexdigest()} for s in manifest["sources"]]
    wgsl = "crates/incant_render/src/sprites/sprite.wgsl"
    changed = git("diff", "--name-only", SOURCE_COMMIT, "HEAD").split()
    record = {"head": git("rev-parse", "HEAD").strip(), "source_commit": SOURCE_COMMIT,
              "files_changed_since_source_commit": changed,
              "only_evidence_and_brief_changed": all(p.startswith(("docs/spikes/evidence/", "handoffs/0062-"))
                                                    for p in changed),
              "crates_diff_empty": git("diff", "--stat", SOURCE_COMMIT, "HEAD", "--", "crates", "Cargo.lock",
                                       "Cargo.toml", "tools/cargo", "rust-toolchain.toml") == "",
              "status_porcelain": git("status", "--porcelain"),
              "sources": sources,
              "sources_match": all(s["manifest"] == s["worktree"] == s["at_source_commit"] for s in sources),
              "sprite_wgsl_sha256": sha(ROOT / wgsl),
              "sprite_wgsl_matches_manifest": sha(ROOT / wgsl) == manifest["unchanged_wgsl_sha256"],
              "sprite_wgsl_unchanged_from_parent": git("diff", "--stat", f"{SOURCE_COMMIT}~1", SOURCE_COMMIT,
                                                       "--", wgsl) == "",
              "cargo_lock_sha256": sha(ROOT / "Cargo.lock"),
              "cargo_version": subprocess.run([str(ROOT / "tools/cargo"), "--version"], cwd=ROOT,
                                              capture_output=True, text=True).stdout.strip(),
              "rustc_version": subprocess.run(
                  [subprocess.run(["rustup", "which", "rustc"], cwd=ROOT, capture_output=True, text=True,
                                  check=True).stdout.strip(), "-vV"],
                  cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip(),
              "recorded": now()}
    write("source-freeze", record)


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ("baseline", "candidate", "cook", "headless", "freeze"):
        raise SystemExit(__doc__)
    step = sys.argv[1]
    if step == "freeze":
        source_freeze()
    elif step in VARIANTS:
        variant(step)
    else:
        tools(step)


if __name__ == "__main__":
    main()
