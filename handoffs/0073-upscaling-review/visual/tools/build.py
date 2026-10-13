#!/usr/bin/env python3
"""CPU-only child checkout, toolchain identity and the frozen review build for the
0073 supervised CPU build. Standard library only; Python 3.12+ (tarfile's data
filter).

Adapted from handoff 0068's committed, root-reviewed tools/build.py (read-only
reuse). It never runs a built executable. Every step calls the build grant and
lease gate first and runs one bounded process group under the packet flock. Any
failed build or source mismatch latches stop.json.

Source pinning comes only from root's build-ready.json: a committed source
(40-hex commit object, never a branch name) and root's source freeze by path and
SHA-256. The checkout is a `git archive` export of that commit (blob-verified,
so no working-tree or dirty file can enter). The freeze must name the same
commit if it names one, and its file list must cover every build input exactly:
the same set of paths under the plan's build-input roots, each with the same
SHA-256 (and byte size where listed); every other freeze entry must match too.

  python3 -I -B handoffs/0073-upscaling-review/visual/tools/build.py <step>

Steps:
  checkout   export and verify the granted commit, then require the granted
             freeze to match every build input exactly
  toolchain  rustc/cargo versions (no compilation) plus OS and hardware model
  review     copy the committed review helper into the checkout, build it with
             default features only (--no-run), refuse any cooked-* or
             native-metalfx feature, and freeze the executable chosen from
             Cargo's JSON messages; review.json is the build record the GPU
             grant binds by SHA-256
  lint       optional, CPU only, while the lease is held: clippy -D warnings on
             the review helper target
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

# Freeze file lists accepted from root's manifests ({path, sha256[, bytes]} rows).
FREEZE_LISTS = ("files", "sources", "unchanged_baselines", "baselines")
FREEZE_HEADS = ("head", "commit", "source_commit")


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


def freeze_entries(freeze):
    """Every {path, sha256[, bytes]} row in root's freeze; duplicate paths must agree."""
    rows = {}
    for key in FREEZE_LISTS:
        for row in freeze.get(key, []) if isinstance(freeze, dict) else []:
            path, digest = row.get("path"), row.get("sha256")
            if not isinstance(path, str) or not c.HEX64.fullmatch(str(digest)):
                c.refuse(f"freeze row without a path and SHA-256 in {key}")
            entry = {"sha256": digest, "bytes": row.get("bytes")}
            if rows.setdefault(path, entry) != entry:
                c.refuse("freeze lists one path with two identities: " + path)
    if not rows:
        c.refuse("the granted freeze lists no files")
    return rows


def under(path, roots):
    return any(path == root or path.startswith(root + "/") for root in roots)


def freeze_coverage(rows, entries, roots):
    """Exact build-input coverage: the freeze and the exported commit name the same
    paths under every build-input root, and every freeze row matches its file."""
    exported = {entry["path"] for entry in entries}
    inputs = {path for path in exported if under(path, roots)}
    frozen = {path for path in rows if under(path, roots)}
    mismatched = []
    for path, row in sorted(rows.items()):
        file = c.CHECKOUT / path
        if path not in exported or not file.is_file():
            continue
        if c.sha(file) != row["sha256"] or (row["bytes"] is not None
                                            and file.stat().st_size != row["bytes"]):
            mismatched.append(path)
    result = {"build_input_roots": list(roots), "build_inputs": len(inputs),
              "freeze_rows": len(rows), "freeze_build_input_rows": len(frozen),
              "inputs_missing_from_freeze": sorted(inputs - frozen)[:20],
              "freeze_rows_missing_from_commit": sorted(set(rows) - exported)[:20],
              "mismatched": mismatched[:20], "mismatched_count": len(mismatched)}
    result["exact"] = (not (inputs - frozen) and not (set(rows) - exported)
                       and not mismatched and bool(inputs))
    return result


def checkout():
    data, grant_sha = c.build_gate()
    tools = c.tools_identity(data["visual_tree"])
    if sys.version_info < (3, 12):
        c.refuse("checkout needs Python 3.12+ for tarfile's data filter")
    commit = data["source_commit"]
    if c.CHECKOUT.exists() or (c.BUILD / "checkout.json").exists():
        c.refuse("child checkout already exists; it is never replaced")
    kind = subprocess.run(["git", "cat-file", "-t", commit], cwd=c.ROOT, capture_output=True,
                          text=True).stdout.strip()
    if kind != "commit":
        c.refuse(f"granted source {commit} is not a commit in the local object store")
    freeze_path = Path(data["source_freeze"]["path"])
    if not freeze_path.is_file() or c.sha(freeze_path) != data["source_freeze"]["sha256"]:
        c.refuse("the granted source freeze is absent or its SHA-256 differs from the grant")
    freeze = c.read_json(freeze_path)
    heads = {key: freeze[key] for key in FREEZE_HEADS if isinstance(freeze, dict) and key in freeze}
    if any(value != commit for value in heads.values()):
        c.refuse(f"the granted freeze names {heads}, not the granted commit {commit}")
    rows = freeze_entries(freeze)
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
    coverage = freeze_coverage(rows, entries, c.plan()["build_inputs"])
    c.write_new_json(c.BUILD / "source.json", {
        "build_grant_sha256": grant_sha, "tools": tools, "source_commit": commit, "tree": tree,
        "files": len(entries), "checkout_manifest_sha256": c.sha(c.BUILD / "checkout.json"),
        "source_freeze": {"path": str(freeze_path), "sha256": c.sha(freeze_path),
                          "names_commit": heads},
        "coverage": coverage, "cargo_lock_sha256": c.sha(c.CHECKOUT / "Cargo.lock"),
        "rust_toolchain": (c.CHECKOUT / "rust-toolchain.toml").read_text(), "recorded": c.now()})
    print(json.dumps({"commit": commit, "files": len(entries), "build_inputs": coverage["build_inputs"],
                      "freeze_rows": coverage["freeze_rows"], "exact": coverage["exact"]}, indent=1))
    if not coverage["exact"]:
        latch_stop("granted source freeze does not match every build input", coverage)
        raise SystemExit("the exported commit does not match the granted freeze exactly; stop latched")


# ------------------------------------------------------------------ toolchain

def toolchain():
    data, _ = c.build_gate()
    c.tools_identity(data["visual_tree"])
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
    data, grant_sha = c.build_gate()
    tools = c.tools_identity(data["visual_tree"])
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is not None or (c.BUILD / "review.json").exists():
        c.refuse("review helper already copied/built; the copy and binary are never replaced")
    destination = c.CHECKOUT / c.HELPER_DEST
    shutil.copyfile(c.HELPER_SOURCE, destination)
    source = c.verify_checkout()
    p = c.plan()
    record = cargo("review", p["builds"]["review"], Path(data["target"]))
    prefixes = tuple(p["builds"]["forbidden_feature_prefixes"])
    cooked = sorted({f for a in record["artifacts"] for f in a["features"] if f.startswith(prefixes)})
    found = [a for a in record["artifacts"]
             if a["executable"] and a["test"] and a["target"] == p["review"]["binary"]]
    library = [a for a in record["artifacts"]
               if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]]
    if cooked or len(found) != 1 or len(library) != 1:
        latch_stop("review build identity", {"cooked_features": cooked, "executables": len(found),
                                             "libraries": len(library)})
        raise SystemExit("review build: cooked or native-metalfx features, or not exactly one "
                         "helper executable and one incant_render library; stop latched")
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
        "build_grant_sha256": grant_sha, "source_commit": data["source_commit"],
        "source_freeze": data["source_freeze"],
        "source_record_sha256": c.sha(c.BUILD / "source.json"),
        "tools": tools, "source": source, "helper_sha256": c.sha(c.HELPER_SOURCE),
        "helper_copy": str(destination), "frozen": frozen,
        "incant_render_library": library[0], "cooked_features": cooked,
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"upscaling_review": frozen["sha256"], "features": frozen["features"],
                      "library_features": library[0]["features"],
                      "build_record_sha256": c.sha(c.BUILD / "review.json")}, indent=1))


def lint():
    data, _ = c.build_gate()
    tools = c.tools_identity(data["visual_tree"])
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
    if entry["exit"] != 0 or entry["timed_out"]:
        latch_stop("cargo lint failed", {"exit": entry["exit"], "stderr": entry["stderr"]})
        raise SystemExit(entry["exit"] or 1)


if __name__ == "__main__":
    c.install_interrupts()
    steps = {"checkout": checkout, "toolchain": toolchain, "review": review, "lint": lint}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
