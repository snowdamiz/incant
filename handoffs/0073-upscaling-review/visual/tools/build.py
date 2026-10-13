#!/usr/bin/env python3
"""CPU-only child checkout, toolchain identity and the frozen review build for the
0073 upscaling visual stage. Standard library only; Python 3.12+ (tarfile's data
filter).

Adapted from handoff 0068's committed, root-reviewed tools/build.py (read-only
reuse). It never runs a built executable. Every step calls the grant/lease gate
first and runs one bounded process group under the packet flock. Any failed
build latches stop.json.

  python3 -I -B handoffs/0073-upscaling-review/visual/tools/build.py <step>

Steps:
  checkout   export the granted source commit with `git archive` into the
             evidence directory, verify every file against its git blob, require
             byte-identical build inputs to producer 0452c5f5 and the reviewed
             39-file freeze, and record identity
  toolchain  rustc/cargo versions (no compilation) plus OS and hardware model
  review     copy the committed review helper into the checkout, build it with
             default features only (--no-run), refuse any cooked-* feature, and
             freeze the executable chosen from Cargo's JSON messages
  lint       optional, CPU only, also allowed after the GPU latch while the lease
             is held: clippy -D warnings on the review helper target
"""

import fcntl
import json
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

PRODUCER = "0452c5f5afc11c99b7c90d111428cc23611c9590"
MANIFEST = "docs/spikes/evidence/temporal-upscaling.json"
MANIFEST_SHA256 = "2d2f4ffa9c6c43a95ab733f8fa3fa92ac4f5a1743f0759597e31534e70f531ca"
FREEZE_SHA256 = "b7ee5ca277f5ca3b1574dd6640638347ca068cb11ba207cb4c1642bcf9f67411"


def latch_stop(reason, record=None):
    if not c.STOP.exists():
        c.write_new_json(c.STOP, {"at": c.now(), "reason": reason, "record": record})


def run_group(label, argv, cwd, overrides, timeout, stdout_name, stderr_name):
    """One bounded process group under the packet flock; never leaves survivors.
    Interrupts stay pending until the attempt is recorded (common.blocked)."""
    c.LOGS.mkdir(parents=True, exist_ok=True)
    out_path, err_path = c.LOGS / stdout_name, c.LOGS / stderr_name
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        with c.blocked():
            started = c.now()
            with open(out_path, "x") as out, open(err_path, "x") as err:
                result = c.launch_recorded(argv, cwd, c.child_env(overrides), out, err, timeout)
            entry = {"label": label, "kind": "build", "argv": [str(a) for a in argv],
                     "cwd": str(cwd), "env": c.env_receipt(overrides), "started": started,
                     "ended": c.now(), **result, "timeout_s": timeout, "frames_debited": 0,
                     "stdout": str(out_path), "stdout_sha256": c.sha(out_path),
                     "stderr": str(err_path), "stderr_sha256": c.sha(err_path)}
            c.append_synced(c.POST_RELEASE_LEDGER if c.LATCH.exists() else c.LEDGER, entry)
        if result["interrupted_by"]:
            raise KeyboardInterrupt("interrupted by " + ", ".join(result["interrupted_by"]))
        if not result["group_empty_after"]:
            raise SystemExit("owned build process group remains after cleanup; see ledger")
    return entry


def build_overrides(label, target):
    return {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "CARGO_TARGET_DIR": str(target),
            "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)), "CARGO_TERM_COLOR": "never",
            "RUSTUP_AUTO_INSTALL": "0"}


# ------------------------------------------------------------------ checkout

def tree_entries(commit):
    raw = subprocess.run(["git", "ls-tree", "-r", "-z", "--full-tree", commit], cwd=c.ROOT,
                         capture_output=True, check=True).stdout
    entries = []
    for record in raw.split(b"\0"):
        if not record:
            continue
        meta, path = record.split(b"\t", 1)
        mode, kind, blob = meta.decode().split()
        if mode not in ("100644", "100755") or kind != "blob":
            c.refuse(f"unsupported tree entry {mode} {kind} {path!r}")
        entries.append({"path": path.decode(), "mode": mode, "blob": blob})
    return entries


def freeze_check():
    """The reviewed producer manifest, its 39-file freeze and every frozen file."""
    manifest_path = c.CHECKOUT / MANIFEST
    manifest = c.read_json(manifest_path)
    files = manifest["source_freeze"]["sources"] + manifest["source_freeze"]["unchanged_baselines"]
    rows = [{"path": f["path"], "reviewed_sha256": f["sha256"],
             "checkout_sha256": c.sha(c.CHECKOUT / f["path"])} for f in files]
    for row in rows:
        row["same"] = row["reviewed_sha256"] == row["checkout_sha256"]
    return {"manifest_sha256": c.sha(manifest_path),
            "manifest_is_reviewed": c.sha(manifest_path) == MANIFEST_SHA256,
            "freeze_sha256_is_reviewed": manifest.get("source_freeze_sha256") == FREEZE_SHA256,
            "files": rows, "count": len(rows), "all_same": all(r["same"] for r in rows)}


def checkout():
    data, grant_sha = c.gate()
    tools = c.tools_identity()
    if sys.version_info < (3, 12):
        c.refuse("checkout needs Python 3.12+ for tarfile's data filter")
    commit = data["source_commit"]
    if c.CHECKOUT.exists() or (c.BUILD / "checkout.json").exists():
        c.refuse("child checkout already exists; it is never replaced")
    if subprocess.run(["git", "cat-file", "-e", commit + "^{commit}"], cwd=c.ROOT).returncode:
        c.refuse(f"granted source commit {commit} is not in the local object store")
    inputs = c.plan()["build_inputs"]
    same_inputs = subprocess.run(["git", "diff", "--quiet", PRODUCER, commit, "--", *inputs],
                                 cwd=c.ROOT).returncode == 0
    if not same_inputs:
        c.refuse(f"build inputs of {commit} differ from reviewed producer {PRODUCER}")
    tree = c.git("rev-parse", commit + "^{tree}").strip()
    entries = tree_entries(commit)
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        c.CHECKOUT.mkdir(parents=True)
        archive = subprocess.Popen(["git", "archive", "--format=tar", commit], cwd=c.ROOT,
                                   stdout=subprocess.PIPE)
        with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
            stream.extractall(c.CHECKOUT, filter="data")
        if archive.wait() != 0:
            c.refuse("git archive failed")
    mismatched = []
    for entry in entries:
        path = c.CHECKOUT / entry["path"]
        executable = bool(path.stat().st_mode & 0o100)
        if (path.is_symlink() or c.git_blob_sha1(path) != entry["blob"]
                or executable != (entry["mode"] == "100755")):
            mismatched.append(entry["path"])
    listed = {entry["path"] for entry in entries}
    extra = [str(Path(d, n).relative_to(c.CHECKOUT)) for d, _, names in os.walk(c.CHECKOUT)
             for n in names if str(Path(d, n).relative_to(c.CHECKOUT)) not in listed]
    if mismatched or extra:
        c.refuse(f"export differs from {commit}: {mismatched[:5]} extra {extra[:5]}")
    c.write_new_json(c.BUILD / "checkout.json", {"commit": commit, "tree": tree, "files": entries,
                                                 "method": "git archive --format=tar | tarfile data filter",
                                                 "recorded": c.now()})
    frozen = freeze_check()
    c.write_new_json(c.BUILD / "source.json", {
        "grant_sha256": grant_sha, "tools": tools, "source_commit": commit, "tree": tree,
        "files": len(entries), "checkout_manifest_sha256": c.sha(c.BUILD / "checkout.json"),
        "producer": PRODUCER, "build_inputs": inputs,
        "build_inputs_identical_to_producer": same_inputs, "freeze": frozen,
        "cargo_lock_sha256": c.sha(c.CHECKOUT / "Cargo.lock"),
        "rust_toolchain": (c.CHECKOUT / "rust-toolchain.toml").read_text(), "recorded": c.now()})
    print(json.dumps({"commit": commit, "files": len(entries), "freeze_files": frozen["count"],
                      "freeze_all_same": frozen["all_same"]}, indent=1))
    if not (frozen["manifest_is_reviewed"] and frozen["freeze_sha256_is_reviewed"]
            and frozen["count"] == 39 and frozen["all_same"]):
        latch_stop("reviewed producer freeze does not match the checkout", frozen)
        raise SystemExit("the checkout does not carry the reviewed 39-file freeze; stop latched")


# ------------------------------------------------------------------ toolchain

def toolchain():
    c.gate()
    c.tools_identity()
    c.verify_checkout()
    overrides = {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "RUSTUP_AUTO_INSTALL": "0",
                 "TMPDIR": str(c.fresh_dir(c.SCRATCH / "toolchain"))}
    record = {"recorded": c.now(), "env": c.env_receipt(overrides), "commands": {}}
    for name, argv in [("cargo", [str(c.CHECKOUT / "tools/cargo"), "-V"]),
                       ("rustc", ["rustc", "-vV"]), ("sw_vers", ["sw_vers"]),
                       ("uname", ["uname", "-srm"]), ("hw_model", ["sysctl", "-n", "hw.model"]),
                       ("cpu", ["sysctl", "-n", "machdep.cpu.brand_string"])]:
        entry = run_group("toolchain-" + name, argv, c.CHECKOUT, overrides, 60,
                          f"toolchain-{name}.stdout.txt", f"toolchain-{name}.stderr.txt")
        record["commands"][name] = {"argv": argv, "exit": entry["exit"],
                                    "stdout": Path(entry["stdout"]).read_text(),
                                    "stderr": Path(entry["stderr"]).read_text()}
    record["python"] = sys.version
    c.write_new_json(c.BUILD / "toolchain.json", record)
    print(json.dumps({k: v["stdout"].strip() for k, v in record["commands"].items()}, indent=1))


# ------------------------------------------------------------------ build

def cargo(label, args, target):
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *args, *p["builds"]["common_flags"]]
    entry = run_group(label, argv, c.CHECKOUT, build_overrides(label, target),
                      p["budgets"]["build_timeout_s"], f"build-{label}.stdout.jsonl",
                      f"build-{label}.stderr.txt")
    artifacts = []
    for line in Path(entry["stdout"]).read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact":
            continue
        artifact_target = message["target"]
        if message.get("executable") or artifact_target["name"] == "incant_render":
            artifacts.append({"target": artifact_target["name"], "kind": artifact_target["kind"],
                              "test": message["profile"]["test"], "fresh": message["fresh"],
                              "executable": message.get("executable"),
                              "features": message.get("features", []),
                              "package_id": message["package_id"],
                              "filenames": {f: c.sha(f) for f in message.get("filenames", [])}})
    record = {**entry, "artifacts": artifacts}
    if entry["exit"] != 0 or entry["timed_out"]:
        c.write_new_json(c.BUILD / f"{label}.failed.json", record)
        latch_stop(f"cargo {label} failed", {"exit": entry["exit"], "stderr": entry["stderr"]})
        raise SystemExit(f"cargo {label} failed ({entry['exit']}); see {entry['stderr']}; stop latched")
    return record


def review():
    data, _ = c.gate()
    tools = c.tools_identity()
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is not None or (c.BUILD / "review.json").exists():
        c.refuse("review helper already copied/built; the copy and binary are never replaced")
    destination = c.CHECKOUT / c.HELPER_DEST
    shutil.copyfile(c.HELPER_SOURCE, destination)
    source = c.verify_checkout()
    p = c.plan()
    record = cargo("review", p["builds"]["review"], Path(data["target"]))
    prefix = p["builds"]["forbidden_feature_prefix"]
    cooked = sorted({f for a in record["artifacts"] for f in a["features"] if f.startswith(prefix)})
    found = [a for a in record["artifacts"]
             if a["executable"] and a["test"] and a["target"] == p["review"]["binary"]]
    library = [a for a in record["artifacts"]
               if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]]
    if cooked or len(found) != 1 or len(library) != 1:
        latch_stop("review build identity", {"cooked_features": cooked, "executables": len(found),
                                             "libraries": len(library)})
        raise SystemExit("review build: cooked features, or not exactly one helper executable "
                         "and one incant_render library; stop latched")
    c.BIN.mkdir(parents=True, exist_ok=True)
    frozen_path = c.BIN / p["review"]["binary"]
    if frozen_path.exists():
        raise SystemExit(f"{frozen_path} exists; frozen binaries are never replaced")
    built = Path(found[0]["executable"])
    shutil.copyfile(built, frozen_path)
    os.chmod(frozen_path, 0o555)
    if c.sha(built) != c.sha(frozen_path):
        raise SystemExit("frozen copy differs from the built executable")
    frozen = {"path": str(frozen_path), "sha256": c.sha(frozen_path),
              "bytes": frozen_path.stat().st_size, "mode": oct(frozen_path.stat().st_mode & 0o777),
              "built_from": str(built), "fresh": found[0]["fresh"],
              "features": found[0]["features"], "package_id": found[0]["package_id"]}
    c.write_new_json(c.BUILD / "review.json", {
        "tools": tools, "source": source, "helper_sha256": c.sha(c.HELPER_SOURCE),
        "helper_copy": str(destination), "frozen": frozen,
        "incant_render_library": library[0], "cooked_features": cooked,
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"upscaling_review": frozen["sha256"], "features": frozen["features"],
                      "library_features": library[0]["features"]}, indent=1))


def lint():
    data, _ = c.gate(after_gpu_release=True)
    tools = c.tools_identity()
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is None:
        c.refuse("build.py review first; lint checks the copied helper")
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *p["builds"]["lint"], *p["builds"]["common_flags"],
            "--", "-D", "warnings"]
    entry = run_group("lint", argv, c.CHECKOUT, build_overrides("lint", Path(data["target"])),
                      p["budgets"]["build_timeout_s"], "build-lint.stdout.jsonl",
                      "build-lint.stderr.txt")
    c.write_new_json(c.BUILD / "lint.json", {"tools": tools, "source": source, **entry,
                                             "recorded": c.now()})
    print(json.dumps({k: entry[k] for k in ("label", "exit", "timed_out")}, indent=1))


if __name__ == "__main__":
    c.install_interrupts()
    steps = {"checkout": checkout, "toolchain": toolchain, "review": review, "lint": lint}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
