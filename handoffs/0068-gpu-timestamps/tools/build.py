#!/usr/bin/env python3
"""CPU-only child checkout, toolchain identity and frozen builds for handoff 0068.
Standard library only; needs Python 3.12+ for tarfile's data filter.

Adapted from handoff 0066's committed tools/build.py (read-only reuse). It never
runs a built executable. Every step calls the grant/lease gate first and runs
one bounded process group under the packet flock.

  python3 -I -B handoffs/0068-gpu-timestamps/tools/build.py <step>

Steps:
  checkout   export the granted source commit with `git archive` into the
             evidence directory, verify every file against its git blob, and
             record identity, the reviewed freeze comparison and ignore counts
  toolchain  rustc/cargo versions (no compilation) plus OS and hardware model
  tests      tests-render and tests-host from plan.json (--no-run); freeze the
             incant_render library, model_gpu, game and ui test binaries
  review     copy the committed review helper into the checkout, build it
             (--no-run) and freeze it; the incant_render library must be the
             same fresh artifact the tests build produced
  lint       optional, CPU only, also allowed after the GPU latch while the
             lease is held: clippy -D warnings on the review helper target
"""

import fcntl
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tarfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

IMPLEMENTATION = "99778642585a7b545a9448b9fca4a365ba622985"
IGNORE = re.compile(r"#\[ignore")


def run_group(label, argv, cwd, overrides, timeout, stdout_name, stderr_name):
    """One bounded process group under the packet flock; never leaves survivors."""
    c.LOGS.mkdir(parents=True, exist_ok=True)
    out_path, err_path = c.LOGS / stdout_name, c.LOGS / stderr_name
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        started = c.now()
        with open(out_path, "x") as out, open(err_path, "x") as err:
            child = subprocess.Popen(argv, cwd=cwd, env=c.child_env(overrides), stdout=out,
                                     stderr=err, start_new_session=True)
            timed_out = False
            try:
                code = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(child.pid, signal.SIGKILL)
                code = child.wait()
            finally:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        try:
            os.killpg(child.pid, 0)
            group_empty = False
        except ProcessLookupError:
            group_empty = True
        entry = {"label": label, "kind": "build", "argv": [str(a) for a in argv], "cwd": str(cwd),
                 "env": c.env_receipt(overrides), "pid": child.pid, "started": started,
                 "ended": c.now(), "exit": code, "timed_out": timed_out, "timeout_s": timeout,
                 "group_empty_after": group_empty, "frames_reserved": 0,
                 "submissions_reserved": 0, "stdout": str(out_path),
                 "stdout_sha256": c.sha(out_path), "stderr": str(err_path),
                 "stderr_sha256": c.sha(err_path)}
        with open(c.POST_RELEASE_LEDGER if c.LATCH.exists() else c.LEDGER, "a") as ledger:
            ledger.write(json.dumps(entry) + "\n")
    return entry


def build_overrides(label):
    return {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "CARGO_TARGET_DIR": str(c.TARGET),
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


def ignore_counts(root):
    """Source-level #[ignore attributes per suite, for explaining count changes."""
    groups = {"incant_render (lib + model_gpu)": root / "crates/incant_render",
              "incant_render_gpu_host": root / "crates/incant_render_gpu_host"}
    counts = {}
    for name, directory in groups.items():
        files = {}
        for path in sorted(directory.rglob("*.rs")):
            found = len(IGNORE.findall(path.read_text()))
            if found:
                files[str(path.relative_to(root))] = found
        counts[name] = {"total": sum(files.values()), "files": files}
    return counts


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
    freeze = c.read_json(c.FREEZE)
    freeze_check = [{"path": f["path"], "reviewed_sha256": f["sha256"],
                     "checkout_sha256": c.sha(c.CHECKOUT / f["path"]),
                     "same": c.sha(c.CHECKOUT / f["path"]) == f["sha256"]} for f in freeze["files"]]
    ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", IMPLEMENTATION, commit],
                              cwd=c.ROOT).returncode == 0
    changed = c.git("diff", "--stat", IMPLEMENTATION, commit, "--", "crates", "Cargo.toml",
                    "Cargo.lock", "third_party", "tools/cargo", "rust-toolchain.toml")
    c.write_new_json(c.BUILD / "source.json", {
        "grant_sha256": grant_sha, "tools": tools, "source_commit": commit, "tree": tree,
        "files": len(entries), "checkout_manifest_sha256": c.sha(c.BUILD / "checkout.json"),
        "reviewed_implementation": IMPLEMENTATION,
        "implementation_is_ancestor_or_same": ancestor,
        "build_input_diff_from_implementation": changed,
        "freeze_file_sha256": c.sha(c.FREEZE), "freeze_comparison": freeze_check,
        "freeze_all_same": all(f["same"] for f in freeze_check),
        "cargo_lock_sha256": c.sha(c.CHECKOUT / "Cargo.lock"),
        "rust_toolchain": (c.CHECKOUT / "rust-toolchain.toml").read_text(),
        "ignore_attributes": ignore_counts(c.CHECKOUT), "recorded": c.now()})
    print(json.dumps({"commit": commit, "files": len(entries), "freeze_all_same":
                      all(f["same"] for f in freeze_check), "ancestor": ancestor}, indent=1))


# ------------------------------------------------------------------ toolchain

def toolchain():
    c.gate()
    c.tools_identity()
    c.verify_checkout()
    overrides = {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "RUSTUP_AUTO_INSTALL": "0",
                 "TMPDIR": str(c.fresh_dir(c.SCRATCH / "toolchain"))}
    record = {"recorded": c.now(), "env": c.env_receipt(overrides), "commands": {}}
    lock = open(c.LOCK, "a")
    fcntl.flock(lock, fcntl.LOCK_EX)
    for name, argv in [("cargo", [str(c.CHECKOUT / "tools/cargo"), "-V"]), ("rustc", ["rustc", "-vV"]),
                       ("sw_vers", ["sw_vers"]), ("uname", ["uname", "-srm"]),
                       ("hw_model", ["sysctl", "-n", "hw.model"]),
                       ("cpu", ["sysctl", "-n", "machdep.cpu.brand_string"])]:
        result = subprocess.run(argv, cwd=c.CHECKOUT, env=c.child_env(overrides), capture_output=True,
                                text=True, timeout=60)
        record["commands"][name] = {"argv": argv, "exit": result.returncode,
                                    "stdout": result.stdout, "stderr": result.stderr}
    lock.close()
    record["python"] = sys.version
    c.write_new_json(c.BUILD / "toolchain.json", record)
    print(json.dumps({k: v["stdout"].strip() for k, v in record["commands"].items()}, indent=1))


# ------------------------------------------------------------------ builds

def cargo(label, args):
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *args, *p["builds"]["common_flags"]]
    entry = run_group(label, argv, c.CHECKOUT, build_overrides(label), p["budgets"]["build_timeout_s"],
                      f"build-{label}.stdout.jsonl", f"build-{label}.stderr.txt")
    artifacts, scripts = [], []
    for line in Path(entry["stdout"]).read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "build-script-executed" and "/incant_render#" in message["package_id"]:
            scripts.append({"package_id": message["package_id"], "out_dir": message["out_dir"]})
        if message.get("reason") != "compiler-artifact":
            continue
        target = message["target"]
        if message.get("executable") or target["name"] == "incant_render":
            artifacts.append({"target": target["name"], "kind": target["kind"],
                              "test": message["profile"]["test"], "fresh": message["fresh"],
                              "executable": message.get("executable"),
                              "features": message.get("features", []),
                              "package_id": message["package_id"],
                              "filenames": {f: c.sha(f) for f in message.get("filenames", [])}})
    record = {**entry, "artifacts": artifacts, "incant_render_build_scripts": scripts}
    if entry["exit"] != 0 or entry["timed_out"]:
        c.write_new_json(c.BUILD / f"{label}.failed.json", record)
        raise SystemExit(f"cargo {label} failed ({entry['exit']}); see {entry['stderr']}")
    return record


def freeze(record, picks, group):
    destination = c.BIN / group
    destination.mkdir(parents=True, exist_ok=True)
    frozen = {}
    for name, match in picks.items():
        found = [a for a in record["artifacts"] if a["executable"] and match(a)]
        if len(found) != 1:
            raise SystemExit(f"{group}/{name}: expected one artifact, found {len(found)}")
        source = Path(found[0]["executable"])
        target = destination / name
        if target.exists():
            raise SystemExit(f"{target} exists; frozen binaries are never replaced")
        shutil.copyfile(source, target)
        os.chmod(target, 0o555)
        if c.sha(source) != c.sha(target):
            raise SystemExit(f"copy mismatch for {target}")
        frozen[name] = {"path": str(target), "sha256": c.sha(target), "bytes": target.stat().st_size,
                        "built_from": str(source), "fresh": found[0]["fresh"],
                        "features": found[0]["features"], "package_id": found[0]["package_id"]}
    return frozen


def library(record):
    """The non-test incant_render library artifact(s) of one build."""
    return [a for a in record["artifacts"]
            if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]]


def tests():
    c.gate()
    tools = c.tools_identity()
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is not None:
        c.refuse("the review helper is already in the checkout; regression binaries build without it")
    p = c.plan()
    render = cargo("tests-render", p["builds"]["tests-render"])
    host = cargo("tests-host", p["builds"]["tests-host"])
    frozen = freeze(render, {
        "incant_render-lib-tests": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render",
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu"}, "tests")
    frozen.update(freeze(host, {
        "gpu_host-game": lambda a: a["test"] and a["target"] == "game",
        "gpu_host-ui": lambda a: a["test"] and a["target"] == "ui"}, "tests"))
    c.write_new_json(c.BUILD / "tests.json", {
        "tools": tools, "source": source, "frozen": frozen,
        "incant_render_library": {"tests-render": library(render), "tests-host": library(host)},
        "builds": [render, host], "recorded": c.now()})
    print(json.dumps({k: v["sha256"] for k, v in frozen.items()}, indent=1))


def review():
    c.gate()
    tools = c.tools_identity()
    source = c.verify_checkout()
    tests_record = c.read_json(c.BUILD / "tests.json")
    destination = c.CHECKOUT / c.HELPER_DEST
    if destination.exists():
        c.refuse("review helper already copied; the copy is never replaced")
    shutil.copyfile(c.HELPER_SOURCE, destination)
    source = c.verify_checkout()
    record = cargo("review", c.plan()["builds"]["review"])
    frozen = freeze(record, {"gpu_timestamp_review": lambda a: a["test"] and a["target"] == "gpu_timestamp_review"},
                    "review")
    tested = tests_record["incant_render_library"]["tests-render"]
    linked = library(record)
    same = (len(tested) == 1 and len(linked) == 1 and linked[0]["fresh"]
            and linked[0]["filenames"] == tested[0]["filenames"])
    c.write_new_json(c.BUILD / "review.json", {
        "tools": tools, "source": source, "helper_sha256": c.sha(c.HELPER_SOURCE),
        "helper_copy": str(destination), "frozen": frozen,
        "library_same_as_tests_render": same, "incant_render_library": linked,
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"gpu_timestamp_review": frozen["gpu_timestamp_review"]["sha256"],
                      "library_same_as_tests_render": same}, indent=1))
    if not same:
        raise SystemExit("the review build did not reuse the tests-render incant_render library")


def lint():
    c.gate(after_gpu_release=True)
    tools = c.tools_identity()
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is None:
        c.refuse("build.py review first; lint checks the copied helper")
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *p["builds"]["lint"], *p["builds"]["common_flags"],
            "--", "-D", "warnings"]
    entry = run_group("lint", argv, c.CHECKOUT, build_overrides("lint"), p["budgets"]["build_timeout_s"],
                      "build-lint.stdout.jsonl", "build-lint.stderr.txt")
    c.write_new_json(c.BUILD / "lint.json", {"tools": tools, "source": source, **entry,
                                             "recorded": c.now()})
    print(json.dumps({k: entry[k] for k in ("label", "exit", "timed_out")}, indent=1))


if __name__ == "__main__":
    steps = {"checkout": checkout, "toolchain": toolchain, "tests": tests, "review": review,
             "lint": lint}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
