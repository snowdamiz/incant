#!/usr/bin/env python3
"""CPU-only child checkouts, toolchain identity and frozen builds for handoff
0076's shadow receiver guard comparison. Standard library only; Python 3.12+
(tarfile's data filter). Root supervises these steps in the foreground; never a
background build.

Adapted narrowly from 0072's build.py (48a688c5, with root's cd4f4cd7 fix) and
its accepted 0073 source-pin routine, now per role: `baseline` and `candidate`
each get their own `git archive` export, build record and frozen binary, built
with identical arguments, one after the other in the one granted Cargo target.
It never runs a built executable. Every step passes the build grant/stop/lease
gate and runs one bounded process group under the packet flock with deferred
signals; any failed build or source mismatch latches stop.json. No library test
binary is built: this slice reruns no numeric control.

Source pinning comes only from root's build-ready.json: per role a 40-hex commit
object (never a branch name, and equal to the plan's) and root's source freeze
by path and SHA-256. The checkout is a `git archive` export of that commit (blob-
and mode-verified). The freeze must name the same commit if it names one, and
its rows must cover every build input exactly: the same paths under the plan's
build-input roots, each with the same SHA-256 and byte size; every other freeze
row present in the commit must match too.

  python3 -I -B handoffs/0076-shadow-guard-review/visual/tools/build.py <step> <role>

Steps (role is baseline or candidate):
  checkout   export and verify the role's granted commit against its freeze
  toolchain  rustc/cargo versions in the role's checkout (no compilation) plus OS
             and hardware model
  review     copy the committed fixture into the role's checkout, build it
             (--no-run), freeze it; build/<role>/review.json is the record the
             GPU grant binds
  lint       CPU only, while the lease is held: clippy -D warnings on the role's
             review fixture target
"""

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


def run_group(label, argv, cwd, overrides, timeout, stdout_name, stderr_name, holder):
    """One bounded build process group; never leaves survivors; a failure latches stop."""
    c.LOGS.mkdir(parents=True, exist_ok=True)
    out_path, err_path = c.LOGS / stdout_name, c.LOGS / stderr_name
    with c.packet_lock(), c.deferred_signals():
        started = c.now()
        with open(out_path, "x") as out, open(err_path, "x") as err:
            pid, code, timed_out, original, group_empty = c.supervise(
                argv, cwd, c.child_env(overrides), out, err, timeout)
        entry = {"label": label, "kind": "build", "holder": holder,
                 "argv": [str(a) for a in argv], "cwd": str(cwd),
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


def freeze_coverage(rows, entries, roots, checkout_root):
    """Exact build-input coverage: the freeze and the exported commit name the same
    paths under every build-input root, and every freeze row matches its file."""
    exported = {entry["path"] for entry in entries}
    inputs = {path for path in exported if under(path, roots)}
    frozen = {path for path in rows if under(path, roots)}
    mismatched = []
    for path, row in sorted(rows.items()):
        file = checkout_root / path
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


def checkout(role):
    data, grant_sha = c.build_gate()
    tools = c.tools_identity(data)
    if sys.version_info < (3, 12):
        c.refuse("checkout needs Python 3.12+ for tarfile's data filter")
    source = data["sources"][role]
    commit = source["source_commit"]
    root, build = c.checkout_dir(role), c.build_dir(role)
    if root.exists() or (build / "checkout.json").exists():
        c.refuse(f"the {role} checkout already exists; it is never replaced")
    kind = subprocess.run(["git", "cat-file", "-t", commit], cwd=c.ROOT, capture_output=True,
                          text=True).stdout.strip()
    if kind != "commit":
        c.refuse(f"granted source {commit} is not a commit in the local object store")
    freeze_path = Path(source["source_freeze"]["path"])
    if not freeze_path.is_file() or c.sha(freeze_path) != source["source_freeze"]["sha256"]:
        c.refuse(f"the granted {role} source freeze is absent or its SHA-256 differs from the grant")
    freeze = c.read_json(freeze_path)
    heads = {key: freeze[key] for key in FREEZE_HEADS if isinstance(freeze, dict) and key in freeze}
    if any(value != commit for value in heads.values()):
        c.refuse(f"the granted freeze names {heads}, not the granted commit {commit}")
    rows = freeze_entries(freeze)
    tree = c.git("rev-parse", commit + "^{tree}").strip()
    entries = tree_entries(commit)
    with c.packet_lock():
        root.mkdir(parents=True)
        archive = subprocess.Popen(["git", "archive", "--format=tar", commit], cwd=c.ROOT,
                                   stdout=subprocess.PIPE)
        with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
            stream.extractall(root, filter="data")
        if archive.wait() != 0:
            c.refuse("git archive failed")
    mismatched = []
    for entry in entries:
        path = root / entry["path"]
        executable = bool(path.stat().st_mode & 0o100)
        if (path.is_symlink() or c.git_blob_sha1(path) != entry["blob"]
                or executable != (entry["mode"] == "100755")):
            mismatched.append(entry["path"])
    listed = {entry["path"] for entry in entries}
    extra = [str(Path(d, n).relative_to(root)) for d, _, names in os.walk(root)
             for n in names if str(Path(d, n).relative_to(root)) not in listed]
    if mismatched or extra:
        c.refuse(f"export differs from {commit}: {mismatched[:5]} extra {extra[:5]}")
    c.write_new_json(build / "checkout.json", {
        "role": role, "commit": commit, "tree": tree, "files": entries,
        "method": "git archive --format=tar | tarfile data filter", "recorded": c.now()})
    coverage = freeze_coverage(rows, entries, c.plan()["build_inputs"], root)
    c.write_new_json(build / "source.json", {
        "role": role, "build_grant_sha256": grant_sha, "holder": data["holder"], "tools": tools,
        "source_commit": commit, "tree": tree, "files": len(entries),
        "checkout_manifest_sha256": c.sha(build / "checkout.json"),
        "source_freeze": {"path": str(freeze_path), "sha256": c.sha(freeze_path),
                          "names_commit": heads},
        "coverage": coverage, "cargo_lock_sha256": c.sha(root / "Cargo.lock"),
        "rust_toolchain": (root / "rust-toolchain.toml").read_text(), "recorded": c.now()})
    print(json.dumps({"role": role, "commit": commit, "files": len(entries),
                      "build_inputs": coverage["build_inputs"], "freeze_rows": coverage["freeze_rows"],
                      "exact": coverage["exact"]}, indent=1))
    if not coverage["exact"]:
        c.latch_stop(f"checkout-{role}", "granted source freeze does not match every build input")
        raise SystemExit(f"the exported {role} commit does not match its freeze exactly; stop latched")


# ------------------------------------------------------------------ toolchain

def toolchain(role):
    data, _ = c.build_gate()
    c.tools_identity(data)
    c.verify_checkout(role)
    root, record_path = c.checkout_dir(role), c.build_dir(role) / "toolchain.json"
    if record_path.exists():
        c.refuse(f"the {role} toolchain record exists; it is never replaced")
    overrides = {"PATH": c.RUSTUP_BIN + ":" + c.SYSTEM_PATH, "RUSTUP_AUTO_INSTALL": "0",
                 "TMPDIR": str(c.fresh_dir(c.SCRATCH / f"toolchain-{role}"))}
    record = {"role": role, "recorded": c.now(), "holder": data["holder"],
              "env": c.env_receipt(overrides), "commands": {}, "python": sys.version}
    with c.packet_lock():
        for name, argv in [("cargo", [str(root / "tools/cargo"), "-V"]),
                           ("rustc", ["rustc", "-vV"]), ("sw_vers", ["sw_vers"]),
                           ("uname", ["uname", "-srm"]), ("hw_model", ["sysctl", "-n", "hw.model"]),
                           ("cpu", ["sysctl", "-n", "machdep.cpu.brand_string"])]:
            result = subprocess.run(argv, cwd=root, env=c.child_env(overrides),
                                    capture_output=True, text=True, timeout=60)
            record["commands"][name] = {"argv": argv, "exit": result.returncode,
                                        "stdout": result.stdout, "stderr": result.stderr}
    c.write_new_json(record_path, record)
    print(json.dumps({k: v["stdout"].strip() for k, v in record["commands"].items()}, indent=1))


# ------------------------------------------------------------------ builds

def cargo(label, args, data, role):
    p = c.plan()
    root = c.checkout_dir(role)
    argv = [str(root / "tools/cargo"), *args, *p["builds"]["common_flags"]]
    entry = run_group(label, argv, root, build_overrides(label, data),
                      p["budgets"]["build_timeout_s"], f"build-{label}.stdout.jsonl",
                      f"build-{label}.stderr.txt", data["holder"])
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
        c.write_new_json(c.build_dir(role) / f"{label}.failed.json", record)
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


def review(role):
    data, _ = c.build_gate()
    tools = c.tools_identity(data)
    c.verify_checkout(role)
    build = c.build_dir(role)
    if (build / "review.json").exists():
        c.refuse(f"the {role} review binary is already frozen; the build record is never replaced")
    destination = c.checkout_dir(role) / c.HELPER_DEST
    if destination.exists():
        c.refuse(f"the review fixture is already copied into the {role} checkout; never replaced")
    shutil.copyfile(c.HELPER_SOURCE, destination)
    source = c.verify_checkout(role)
    record = cargo(f"{role}-review", c.plan()["builds"]["review"], data, role)
    frozen = freeze(record, {"temporal_visual_review": lambda a: a["test"] and a["target"] == "temporal_visual_review"},
                    role)
    c.write_new_json(build / "review.json", {
        "role": role, "holder": data["holder"], "build_grant_sha256": c.sha(c.BUILD_GRANT),
        "source_commit": data["sources"][role]["source_commit"], "tools": tools, "source": source,
        "source_record_sha256": c.sha(build / "source.json"),
        "helper_sha256": c.sha(c.HELPER_SOURCE), "helper_copy": str(destination),
        "frozen": frozen, "incant_render_library": library(record), "builds": [record],
        "recorded": c.now()})
    print(json.dumps({"role": role, "temporal_visual_review": frozen["temporal_visual_review"]["sha256"],
                      "features": frozen["temporal_visual_review"]["features"]}, indent=1))


def lint(role):
    data, _ = c.build_gate()
    tools = c.tools_identity(data)
    source = c.verify_checkout(role)
    if source["helper_copy_sha256"] is None:
        c.refuse(f"build.py review {role} first; lint checks the copied fixture")
    if (c.build_dir(role) / "lint.json").exists():
        c.refuse(f"the {role} lint record exists; it is never replaced")
    p = c.plan()
    root = c.checkout_dir(role)
    argv = [str(root / "tools/cargo"), *p["builds"]["lint"], *p["builds"]["common_flags"],
            "--", "-D", "warnings"]
    entry = run_group(f"{role}-lint", argv, root, build_overrides(f"{role}-lint", data),
                      p["budgets"]["build_timeout_s"], f"build-{role}-lint.stdout.jsonl",
                      f"build-{role}-lint.stderr.txt", data["holder"])
    c.write_new_json(c.build_dir(role) / "lint.json", {"role": role, "tools": tools, "source": source,
                                                       **entry, "recorded": c.now()})
    print(json.dumps({k: entry[k] for k in ("label", "exit", "timed_out")}, indent=1))
    if entry["exit"] != 0 or entry["timed_out"]:
        raise SystemExit(entry["exit"] if entry["exit"] else 1)


if __name__ == "__main__":
    steps = {"checkout": checkout, "toolchain": toolchain, "review": review, "lint": lint}
    if len(sys.argv) != 3 or sys.argv[1] not in steps or sys.argv[2] not in c.ROLES:
        raise SystemExit(__doc__)
    steps[sys.argv[1]](sys.argv[2])
