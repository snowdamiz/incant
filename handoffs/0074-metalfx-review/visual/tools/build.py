#!/usr/bin/env python3
"""CPU-only child checkout, toolchain identity and frozen review build for
handoff 0074 stage B. Standard library only; Python 3.12+ for tarfile's data
filter.

Adapted from handoff 0072's committed visual/tools/build.py (visual tree
ac8a48de, read-only reuse). It never runs a built executable. Every step passes
the grant/stop/lease gate and runs one bounded process group under the packet
flock with deferred signals; a failed build latches stop.json. Root supervises
compilation: no step runs without root's explicit source grant.

  python3 -I -B handoffs/0074-metalfx-review/visual/tools/build.py <step>

Steps:
  checkout   export the granted commit with `git archive` into the evidence
             directory, verify every file and mode against git, require the
             granted tree, require root's committed build freeze ({commit,
             files}) to name that commit and cover the plan's build inputs
             exactly, and require the fixture's jitter copy to equal the source
  toolchain  rustc/cargo versions (no compilation) plus OS and hardware model
  review     copy the committed fixture into the checkout, build it (--no-run)
             with exactly the granted features and freeze it
  lint       optional, CPU only, also after the GPU latch while the lease is held;
             a failing lint keeps its receipt, latches stop.json and exits nonzero
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402


def run_group(label, argv, cwd, overrides, timeout, stdout_name, stderr_name):
    """One bounded build process group; never leaves survivors; a failure latches stop."""
    c.LOGS.mkdir(parents=True, exist_ok=True)
    out_path, err_path = c.LOGS / stdout_name, c.LOGS / stderr_name
    with c.packet_lock(), c.deferred_signals():
        started = c.now()
        with open(out_path, "x") as out, open(err_path, "x") as err:
            pid, code, timed_out, original, group_empty = c.supervise(
                argv, cwd, c.child_env(overrides), out, err, timeout)
        entry = {"label": label, "kind": "build", "argv": [str(a) for a in argv], "cwd": str(cwd),
                 "env": c.env_receipt(overrides), "pid": pid, "started": started, "ended": c.now(),
                 "exit": code, "timed_out": timed_out,
                 "wait_error": type(original[1]).__name__ if original else None,
                 "timeout_s": timeout, "group_empty_after": group_empty,
                 "stdout": str(out_path), "stdout_sha256": c.sha(out_path),
                 "stderr": str(err_path), "stderr_sha256": c.sha(err_path)}
        c.append_durable(c.POST_RELEASE_LEDGER if c.LATCH.exists() else c.LEDGER, entry)
        if code != 0 or timed_out or original or not group_empty:
            c.latch_stop(label, f"build exit {code}, timed_out {timed_out}, "
                                f"interrupt {entry['wait_error']}, group_empty {group_empty}")
        if original:
            raise original[1].with_traceback(original[2])
        if not group_empty:
            raise SystemExit("owned build process group remains after cleanup; see ledger")
    return entry


def build_overrides(label, data):
    return {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "CARGO_TARGET_DIR": str(c.target(data)),
            "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)), "CARGO_TERM_COLOR": "never",
            "RUSTUP_AUTO_INSTALL": "0"}


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


def freeze_rows(freeze, commit):
    """Root's committed build freeze {"commit": <granted commit>, "files": [{path,
    sha256, bytes}]} as {path: {sha256, bytes}}, or (None, reason). Paths are
    relative and unique; no other freeze shape is accepted."""
    if not isinstance(freeze, dict) or freeze.get("commit") != commit:
        return None, "the granted freeze does not name the granted commit"
    rows = {}
    for row in freeze.get("files") or []:
        path = row.get("path") if isinstance(row, dict) else None
        if (not isinstance(path, str) or Path(path).is_absolute() or ".." in Path(path).parts
                or re.fullmatch(r"[0-9a-f]{64}", str(row.get("sha256", ""))) is None
                or not (row.get("bytes") is None or isinstance(row.get("bytes"), int))
                or path in rows):
            return None, f"malformed or duplicate freeze row: {row!r}"[:300]
        rows[path] = {"sha256": row["sha256"], "bytes": row.get("bytes")}
    if not rows:
        return None, "the granted freeze lists no files"
    return rows, None


def under(path, roots):
    return any(path == root or path.startswith(root + "/") for root in roots)


def freeze_coverage(rows, entries, roots):
    """Exact build-input coverage, as in handoff 0073's build.py: the freeze and the
    exported commit name the same paths under every build-input root, every freeze
    row (a probe outside the roots included) is in the commit, and every row
    matches its exported file's SHA-256 and size."""
    exported = {entry["path"] for entry in entries}
    inputs = {path for path in exported if under(path, roots)}
    frozen = {path for path in rows if under(path, roots)}
    mismatched = sorted(
        path for path, row in rows.items()
        if path in exported and (c.CHECKOUT / path).is_file()
        and (c.sha(c.CHECKOUT / path) != row["sha256"]
             or (row["bytes"] is not None and (c.CHECKOUT / path).stat().st_size != row["bytes"])))
    result = {"build_input_roots": list(roots), "build_inputs": len(inputs),
              "freeze_rows": len(rows), "freeze_build_input_rows": len(frozen),
              "freeze_rows_outside_roots": sorted(set(rows) - frozen)[:20],
              "inputs_missing_from_freeze": sorted(inputs - frozen)[:20],
              "freeze_rows_missing_from_commit": sorted(set(rows) - exported)[:20],
              "mismatched": mismatched[:20], "mismatched_count": len(mismatched)}
    result["exact"] = bool(inputs) and inputs == frozen and set(rows) <= exported and not mismatched
    return result


def checkout():
    data, grant_sha = c.gate()
    tools = c.tools_identity(data)
    if sys.version_info < (3, 12):
        c.refuse("checkout needs Python 3.12+ for tarfile's data filter")
    commit = data["source_commit"]
    if c.CHECKOUT.exists() or (c.BUILD / "checkout.json").exists():
        c.refuse("child checkout already exists; it is never replaced")
    if subprocess.run(["git", "cat-file", "-e", commit + "^{commit}"], cwd=c.ROOT).returncode:
        c.refuse(f"granted source commit {commit} is not in the local object store")
    tree = c.git("rev-parse", commit + "^{tree}").strip()
    if tree != data["source_tree"]:
        c.refuse(f"commit {commit} has tree {tree}, not the granted {data['source_tree']}")
    freeze_path = Path(data["source_freeze"]["path"])
    if not freeze_path.is_file() or c.sha(freeze_path) != data["source_freeze"]["sha256"]:
        c.refuse("root's source freeze is absent or differs from its granted SHA-256")
    try:
        rows, problem = freeze_rows(c.read_json(freeze_path), commit)
    except ValueError as error:
        rows, problem = None, f"root's source freeze is not JSON: {error}"
    if problem:
        c.refuse(problem)
    shown = subprocess.run(["git", "show", f"{commit}:{c.JITTER_SOURCE}"], cwd=c.ROOT,
                           capture_output=True, text=True)
    jitter = c.samples(shown.stdout) if shown.returncode == 0 else None
    if jitter is None or jitter != c.samples(c.HELPER_SOURCE.read_text()):
        c.refuse("the fixture's SAMPLES copy differs from the granted source's jitter table")
    entries = tree_entries(commit)
    with c.packet_lock():
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
    c.write_new_json(c.BUILD / "checkout.json", {
        "commit": commit, "tree": tree, "files": entries,
        "method": "git archive --format=tar | tarfile data filter", "recorded": c.now()})
    coverage = freeze_coverage(rows, entries, c.plan()["build_inputs"])
    c.write_new_json(c.BUILD / "source.json", {
        "grant_sha256": grant_sha, "tools": tools, "source_commit": commit, "tree": tree,
        "files": len(entries), "checkout_manifest_sha256": c.sha(c.BUILD / "checkout.json"),
        "source_freeze": data["source_freeze"], "coverage": coverage,
        "features": c.features(data), "jitter_table": jitter,
        "cargo_lock_sha256": c.sha(c.CHECKOUT / "Cargo.lock"),
        "rust_toolchain": (c.CHECKOUT / "rust-toolchain.toml").read_text(), "recorded": c.now()})
    print(json.dumps({"commit": commit, "files": len(entries), "build_inputs": coverage["build_inputs"],
                      "freeze_rows": coverage["freeze_rows"], "exact": coverage["exact"]}))
    if not coverage["exact"]:
        c.latch_stop("checkout", "root's freeze does not cover the build inputs exactly")
        raise SystemExit("the exported commit does not match the granted freeze exactly; stop latched")


def toolchain():
    data, _ = c.gate()
    c.tools_identity(data)
    c.verify_checkout()
    overrides = {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "RUSTUP_AUTO_INSTALL": "0",
                 "TMPDIR": str(c.fresh_dir(c.SCRATCH / "toolchain"))}
    record = {"recorded": c.now(), "env": c.env_receipt(overrides), "commands": {},
              "python": sys.version}
    with c.packet_lock():
        for name, argv in [("cargo", [str(c.CHECKOUT / "tools/cargo"), "-V"]),
                           ("rustc", ["rustc", "-vV"]), ("sw_vers", ["sw_vers"]),
                           ("uname", ["uname", "-srm"]), ("hw_model", ["sysctl", "-n", "hw.model"]),
                           ("cpu", ["sysctl", "-n", "machdep.cpu.brand_string"])]:
            result = subprocess.run(argv, cwd=c.CHECKOUT, env=c.child_env(overrides),
                                    capture_output=True, text=True, timeout=60)
            record["commands"][name] = {"argv": argv, "exit": result.returncode,
                                        "stdout": result.stdout, "stderr": result.stderr}
    c.write_new_json(c.BUILD / "toolchain.json", record)
    print(json.dumps({k: v["stdout"].strip() for k, v in record["commands"].items()}, indent=1))


def feature_args(data):
    """Exactly the granted feature set; nothing defaults to cooked or uncooked."""
    return ["--features", ",".join(c.features(data))]


def cargo(label, args, data):
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *args, *feature_args(data),
            *p["builds"]["common_flags"]]
    entry = run_group(label, argv, c.CHECKOUT, build_overrides(label, data),
                      p["budgets"]["build_timeout_s"], f"build-{label}.stdout.jsonl",
                      f"build-{label}.stderr.txt")
    artifacts = []
    for line in Path(entry["stdout"]).read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
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
    record = {**entry, "artifacts": artifacts}
    if entry["exit"] != 0 or entry["timed_out"]:
        c.write_new_json(c.BUILD / f"{label}.failed.json", record)
        raise SystemExit(f"cargo {label} failed ({entry['exit']}); see {entry['stderr']}")
    return record


def freeze(record, picks, group):
    """Copy each uniquely identified artifact (by Cargo's JSON, never by file
    name) into the evidence bin directory, read-only and executable, by hash."""
    destination = c.BIN / group
    destination.mkdir(parents=True, exist_ok=True)
    frozen = {}
    for name, match in picks.items():
        found = [a for a in record["artifacts"] if a["executable"] and match(a)]
        if len(found) != 1:
            c.latch_stop(f"freeze-{name}", f"expected one artifact, found {len(found)}")
            raise SystemExit(f"{group}/{name}: expected one artifact, found {len(found)}")
        source = Path(found[0]["executable"])
        target = destination / name
        if target.exists():
            c.refuse(f"{target} exists; frozen binaries are never replaced")
        shutil.copyfile(source, target)
        os.chmod(target, 0o555)
        c.require_executable(target)
        built = found[0]["filenames"].get(str(source))  # hashed right after the build
        if c.sha(source) != c.sha(target) or (built is not None and built != c.sha(target)):
            c.latch_stop(f"freeze-{name}", "copy hash mismatch")
            raise SystemExit(f"copy mismatch for {target}")
        frozen[name] = {"path": str(target), "sha256": c.sha(target), "bytes": target.stat().st_size,
                        "mode": oct(target.stat().st_mode & 0o777), "built_from": str(source),
                        "fresh": found[0]["fresh"], "features": found[0]["features"],
                        "package_id": found[0]["package_id"]}
    return frozen


def library(record):
    """The non-test incant_render library artifact(s) of one build."""
    return [a for a in record["artifacts"]
            if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]]


def features_admitted(granted, built):
    """The library carries every granted feature, and an uncooked grant carries
    no cooked feature (motion admission would refuse a partial cooked build)."""
    cooked = any(f.startswith("cooked-") for f in built)
    return set(granted) <= set(built) and (cooked == ("cooked-temporal" in granted))


def review():
    data, _ = c.gate()
    tools = c.tools_identity(data)
    c.verify_checkout()
    source_record = c.BUILD / "source.json"
    if not source_record.exists() or not c.read_json(source_record).get("coverage", {}).get("exact"):
        c.refuse("build.py checkout with exact freeze coverage first")
    destination = c.CHECKOUT / c.HELPER_DEST
    if destination.exists():
        c.refuse("review fixture already copied; the copy is never replaced")
    shutil.copyfile(c.HELPER_SOURCE, destination)
    source = c.verify_checkout()
    record = cargo("review", c.plan()["builds"]["review"], data)
    frozen = freeze(record, {"metalfx_visual_review": lambda a: a["test"] and a["target"] == "metalfx_visual_review"},
                    "review")
    linked = library(record)
    admitted = len(linked) == 1 and features_admitted(c.features(data), linked[0]["features"])
    c.write_new_json(c.BUILD / "review.json", {
        "tools": tools, "source": source, "helper_sha256": c.sha(c.HELPER_SOURCE),
        "helper_copy": str(destination), "frozen": frozen, "granted_features": c.features(data),
        "library_features_admitted": admitted, "incant_render_library": linked,
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"metalfx_visual_review": frozen["metalfx_visual_review"]["sha256"],
                      "library_features_admitted": admitted}, indent=1))
    if not admitted:
        c.latch_stop("review", "the review build's incant_render features differ from the grant")
        raise SystemExit("the review build's incant_render features differ from the grant")


def lint():
    data, _ = c.gate(allow_stopped=True, after_gpu_release=True)
    tools = c.tools_identity(data)
    source = c.verify_checkout()
    if source["helper_copy_sha256"] is None:
        c.refuse("build.py review first; lint checks the copied fixture")
    p = c.plan()
    argv = [str(c.CHECKOUT / "tools/cargo"), *p["builds"]["lint"], *feature_args(data),
            *p["builds"]["common_flags"], "--", "-D", "warnings"]
    entry = run_group("lint", argv, c.CHECKOUT, build_overrides("lint", data),
                      p["budgets"]["build_timeout_s"], "build-lint.stdout.jsonl", "build-lint.stderr.txt")
    c.write_new_json(c.BUILD / "lint.json", {"tools": tools, "source": source, **entry,
                                             "recorded": c.now()})
    print(json.dumps({k: entry[k] for k in ("label", "exit", "timed_out")}, indent=1))
    if entry["exit"] != 0 or entry["timed_out"] or entry["wait_error"] or not entry["group_empty_after"]:
        # run_group already latched stop.json; the receipt above is retained either way.
        c.latch_stop("lint", f"lint exit {entry['exit']}, timed_out {entry['timed_out']}")
        raise SystemExit(f"lint failed (exit {entry['exit']}); receipt {c.BUILD / 'lint.json'}, "
                         "stop latched")


if __name__ == "__main__":
    steps = {"checkout": checkout, "toolchain": toolchain, "review": review, "lint": lint}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
