#!/usr/bin/env python3
"""CPU-only frozen builds for handoff 0067. Standard library only.

Adapted from handoff 0066's tools/build.py (read-only reuse). Runs ./tools/cargo
with --locked, --offline, --profile ci and -j2 in the leased target directory
under an explicit minimal environment, parses Cargo's JSON compiler-artifact
messages (target, kind, test flag, features, freshness), copies each selected
executable into the evidence directory and records SHA-256 values. It never
runs a built executable.

  python3 -I -B handoffs/0067-dynamic-resolution/tools/build.py <step>

Steps:
  freeze        source identities, host/toolchain facts, helper source digest
  tests         incant_render (lib + model_gpu) and incant_render_gpu_host (lib, game, ui) test
                binaries with the agreed default feature set (the reviewed CPU build's selection)
  helper-lock   seed tools/drs/Cargo.lock from the repository lock and let Cargo prune it offline
  verify-lock   check every helper lock entry against the repository lock
  helper        the packet test binary (drs_gpu) and appearance helper (drs_appearance)
  helper-rN     the same, rebuilt after a recorded test-code correction (bin/helper-rN)
"""

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

OWN = "incant_0067_drs"


def cargo(label, args, cwd=None):
    c.check_lease()
    c.check_source()
    env = c.build_env()
    argv = [str(c.ROOT / "tools/cargo"), *args, "--locked", "--offline", "--profile", "ci", "-j2",
            "--message-format=json-render-diagnostics"]
    logs = c.EVIDENCE / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    started = c.now()
    with open(logs / f"build-{label}.stdout.jsonl", "x") as out, open(logs / f"build-{label}.stderr.txt", "x") as err:
        result = subprocess.run(argv, cwd=cwd or c.ROOT, env=env, stdout=out, stderr=err)
    ended = c.now()
    artifacts = []
    for line in (logs / f"build-{label}.stdout.jsonl").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact":
            continue
        artifacts.append({"target": message["target"]["name"], "kind": message["target"]["kind"],
                          "test": message["profile"]["test"], "fresh": message["fresh"],
                          "executable": message.get("executable"), "features": message.get("features", []),
                          "package_id": message["package_id"], "opt_level": message["profile"]["opt_level"]})
    record = {"label": label, "argv": argv, "cwd": str(cwd or c.ROOT), "env": c.recorded_env(env),
              "started": started, "ended": ended, "exit": result.returncode,
              "stdout_sha256": c.sha(logs / f"build-{label}.stdout.jsonl"),
              "stderr_sha256": c.sha(logs / f"build-{label}.stderr.txt"), "artifacts": artifacts}
    if result.returncode != 0:
        c.write_new_json(c.EVIDENCE / "build" / f"{label}.failed.{c.now().replace(':', '')}.json", record)
        tail = (logs / f"build-{label}.stderr.txt").read_text()[-6000:]
        raise SystemExit(f"cargo {label} failed with {result.returncode}:\n{tail}")
    return record


def freeze_binaries(record, picks, group):
    """Copy selected executables to EVIDENCE/bin/<group>/<name> (read-only) and hash them."""
    destination = c.EVIDENCE / "bin" / group
    destination.mkdir(parents=True, exist_ok=True)
    frozen = {}
    for name, match in picks.items():
        found = [a for a in record["artifacts"] if a["executable"] and match(a)]
        if len(found) != 1:
            raise SystemExit(f"{group}/{name}: expected one artifact, found {len(found)}")
        source = Path(found[0]["executable"])
        target = destination / name
        if target.exists():
            raise SystemExit(f"{target} already exists; frozen binaries are never replaced")
        shutil.copyfile(source, target)
        os.chmod(target, 0o555)
        if c.sha(source) != c.sha(target):
            raise SystemExit(f"copy mismatch for {target}")
        frozen[name] = {"path": str(target), "sha256": c.sha(target), "bytes": target.stat().st_size,
                        "built_from": str(source), "fresh": found[0]["fresh"], "features": found[0]["features"],
                        "package_id": found[0]["package_id"]}
    return frozen


def unit_features(record):
    """{package id: sorted feature-set strings} for every non-test library unit in a build."""
    out = {}
    for a in record["artifacts"]:
        if a["test"] or not set(a["kind"]) & {"lib", "rlib", "proc-macro"}:
            continue
        out.setdefault(a["package_id"], set()).add(",".join(sorted(a["features"])))
    return {k: sorted(v) for k, v in sorted(out.items())}


def package(features, crate):
    """The single package id of a workspace crate in a unit_features map."""
    found = [k for k in features if re.search(rf"/crates/{crate}#", k)]
    if len(found) != 1:
        raise SystemExit(f"expected one {crate} package id, found {found}")
    return found[0]


def tests():
    record = cargo("tests", ["test", "--no-run", "-p", "incant_render", "-p", "incant_render_gpu_host"])
    frozen = freeze_binaries(record, {
        "lib": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render",
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu",
        "gpu_host-lib": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render_gpu_host",
        "gpu_host-game": lambda a: a["test"] and a["target"] == "game",
        "gpu_host-ui": lambda a: a["test"] and a["target"] == "ui",
    }, "tests")
    features = unit_features(record)
    c.write_new_json(c.EVIDENCE / "build" / "tests.json", {
        "head": c.git("rev-parse", "HEAD").strip(), "frozen": frozen, "unit_features": features,
        "build": record, "recorded": c.now()})
    print(json.dumps({"frozen": {k: v["sha256"] for k, v in frozen.items()},
                      "incant_render": features[package(features, "incant_render")],
                      "incant_render_gpu_host": features[package(features, "incant_render_gpu_host")]},
                     indent=1))


def helper(group="helper"):
    """`group` names the frozen binary directory and record (helper, helper-r2, ...)."""
    if (c.EVIDENCE / "build" / f"{group}.json").exists():
        raise SystemExit(f"{group} is already built; frozen helper builds are never replaced")
    for path in (c.HELPER / "Cargo.toml", c.HELPER / "Cargo.lock"):
        if not path.is_file():
            raise SystemExit(f"missing helper file {path}")
    verify_lock(write=False)
    digest, files = c.helper_digest()
    manifest = ["--manifest-path", str(c.HELPER / "Cargo.toml")]
    tests_record = cargo(f"{group}-tests", ["test", "--no-run", *manifest, "--test", "drs_gpu"])
    bin_record = cargo(f"{group}-bin", ["build", *manifest, "--bin", "drs_appearance"])
    frozen = freeze_binaries(tests_record, {"drs_gpu": lambda a: a["test"] and a["target"] == "drs_gpu"}, group)
    frozen.update(freeze_binaries(bin_record, {"drs_appearance": lambda a: a["target"] == "drs_appearance"},
                                  group))
    regression = c.read_json(c.EVIDENCE / "build" / "tests.json")["unit_features"]
    helper_units = unit_features(tests_record)
    for name, sets in unit_features(bin_record).items():
        helper_units[name] = sorted(set(helper_units.get(name, [])) | set(sets))
    shared = sorted(set(regression) & set(helper_units))
    differing = {n: {"regression": regression[n], "helper": helper_units[n]} for n in shared
                 if regression[n] != helper_units[n]}
    renderer_equal = all(regression[package(regression, n)] == helper_units.get(package(regression, n))
                         for n in ("incant_render", "incant_render_gpu_host"))
    if digest != c.helper_digest()[0]:
        raise SystemExit("helper sources changed during the build")
    c.write_new_json(c.EVIDENCE / "build" / f"{group}.json", {
        "head": c.git("rev-parse", "HEAD").strip(), "frozen": frozen, "helper_digest": digest,
        "helper_files": files, "unit_features": helper_units,
        "feature_comparison": {"shared_packages": len(shared), "differing": differing,
                               "renderer_and_host_identical": renderer_equal,
                               "only_in_helper": sorted(set(helper_units) - set(regression)),
                               "only_in_regression": sorted(set(regression) - set(helper_units))},
        "builds": [tests_record, bin_record], "recorded": c.now()})
    print(json.dumps({"frozen": {k: v["sha256"] for k, v in frozen.items()}, "helper_digest": digest,
                      "renderer_and_host_identical": renderer_equal, "differing": differing}, indent=1))
    if not renderer_equal:
        raise SystemExit("renderer or GPU host feature set differs from the regression build")


# ------------------------------------------------------------------ helper lock

def lock_packages(text):
    """Cargo.lock [[package]] blocks as {(name, version, source): (checksum, [dependency refs])}."""
    packages = {}
    for block in re.split(r"\n(?=\[\[)", text):
        if not block.startswith("[[package]]"):
            continue
        fields = dict(re.findall(r'^(name|version|source|checksum) = "([^"]*)"', block, re.M))
        deps = re.search(r"^dependencies = \[\n(.*?)^\]", block, re.M | re.S)
        refs = re.findall(r'^ "([^"]*)",$', deps.group(1), re.M) if deps else []
        packages[(fields["name"], fields["version"], fields.get("source"))] = (fields.get("checksum"), refs)
    return packages


def resolve_refs(packages):
    """Dependency references ("name", "name version", "name version (source)") as (name, version)."""
    by_name = {}
    for name, version, _ in packages:
        by_name.setdefault(name, []).append(version)
    resolved = {}
    for key, (_, refs) in packages.items():
        out = set()
        for ref in refs:
            parts = ref.split(" ")
            out.add((parts[0], parts[1] if len(parts) > 1 else by_name[parts[0]][0]))
        resolved[key] = out
    return resolved


def helper_lock():
    c.check_lease()
    lock = c.HELPER / "Cargo.lock"
    if lock.exists():
        raise SystemExit("helper Cargo.lock already exists; verify it instead of regenerating")
    shutil.copyfile(c.ROOT / "Cargo.lock", lock)
    argv = [str(c.ROOT / "tools/cargo"), "metadata", "--offline", "--format-version", "1",
            "--manifest-path", str(c.HELPER / "Cargo.toml")]
    result = subprocess.run(argv, cwd=c.ROOT, env=c.build_env(), capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"cargo metadata failed: {result.stderr[-4000:]}")
    verify_lock(how={"argv": argv, "exit": result.returncode, "stderr": result.stderr[-4000:]})


def verify_lock(how=None, write=True):
    root_text, helper_text = (c.ROOT / "Cargo.lock").read_text(), (c.HELPER / "Cargo.lock").read_text()
    root, helper_lock_packages = lock_packages(root_text), lock_packages(helper_text)
    root_deps, helper_deps = resolve_refs(root), resolve_refs(helper_lock_packages)
    own = [key for key in helper_lock_packages if key[0] == OWN]
    foreign = [f"{k[0]} {k[1]}" for k in helper_lock_packages if k[0] != OWN and k not in root]
    checksum = [f"{k[0]} {k[1]}" for k in helper_lock_packages
                if k in root and helper_lock_packages[k][0] != root[k][0]]
    added = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in helper_deps[k] - root_deps[k])
             for k in helper_lock_packages if k in root and helper_deps[k] - root_deps[k]}
    dropped = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in root_deps[k] - helper_deps[k])
               for k in helper_lock_packages if k in root and root_deps[k] - helper_deps[k]}
    record = {"how": how, "repository_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
              "helper_lock_sha256": c.sha(c.HELPER / "Cargo.lock"), "repository_packages": len(root),
              "helper_packages": len(helper_lock_packages), "own_entries": [list(k) for k in own],
              "own_dependencies": sorted(f"{n} {v}" for n, v in helper_deps[own[0]]) if own else None,
              "not_in_repository_lock": foreign, "checksum_mismatches": checksum,
              "dependency_edges_added": added, "dependency_edges_dropped": dropped,
              "recorded": c.now()}
    if len(own) != 1 or foreign or checksum or added:
        print(json.dumps(record, indent=1))
        raise SystemExit("helper lock is not a pruned copy of the repository lock")
    if write:
        c.write_new_json(c.EVIDENCE / "build" / f"helper-lock-{c.now().replace(':', '')}.json", record)
        print(json.dumps({k: v for k, v in record.items() if k != "dependency_edges_dropped"}, indent=1))
    return record


# ------------------------------------------------------------------ identities

def host_facts():
    def run(*argv):
        result = subprocess.run(list(argv), capture_output=True, text=True, env=c.minimal_env())
        return result.stdout.strip()
    return {"sw_vers": run("sw_vers"), "uname_m": run("uname", "-m"), "hw_model": run("sysctl", "-n", "hw.model"),
            "cpu": run("sysctl", "-n", "machdep.cpu.brand_string"),
            "memsize_bytes": run("sysctl", "-n", "hw.memsize")}


def freeze():
    grant_sha = c.check_grant()
    source = c.check_source()
    manifest = c.read_json(c.MANIFEST)

    def blob(commit, path):
        shown = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=c.ROOT, capture_output=True,
                               env=c.minimal_env())
        return c.sha_bytes(shown.stdout) if shown.returncode == 0 else None

    sources = [{"path": s["path"], "manifest": s["sha256"], "at_source_commit": blob(c.SOURCE_COMMIT, s["path"]),
                "worktree": c.sha(c.ROOT / s["path"])} for s in manifest["sources"]]
    review = manifest["source_review"]
    env = c.build_env()
    digest, files = c.helper_digest()
    record = {
        "grant_sha256": grant_sha, "grant": c.read_json(c.GRANT), "source_check": source,
        "head": c.git("rev-parse", "HEAD").strip(), "source_commit": c.SOURCE_COMMIT,
        "implementation_commit": c.IMPLEMENTATION_COMMIT, "base_commit": c.BASE_COMMIT,
        "implementation_files": c.git("diff", "--name-only", c.BASE_COMMIT, c.IMPLEMENTATION_COMMIT).split(),
        "files_after_implementation": c.git("diff", "--name-only", c.IMPLEMENTATION_COMMIT, c.SOURCE_COMMIT).split(),
        "wgsl_changed_since_base": c.git("diff", "--name-only", c.BASE_COMMIT, c.SOURCE_COMMIT, "--",
                                         "*.wgsl").split(),
        "cargo_lock_changed_since_base": c.git("diff", "--name-only", c.BASE_COMMIT, c.SOURCE_COMMIT, "--",
                                               "Cargo.lock").split(),
        "status_porcelain_outside_packet": c.git("status", "--porcelain", "--untracked-files=all", "--", ".",
                                                 f":(exclude)handoffs/{c.PACKET}"),
        "manifest_sha256": c.sha(c.MANIFEST), "manifest_sources": sources,
        "manifest_matches_source_commit_and_worktree": all(
            s["manifest"] == s["at_source_commit"] == s["worktree"] for s in sources),
        "source_review": {"path": review["path"], "manifest": review["sha256"],
                          "worktree": c.sha(c.ROOT / review["path"]),
                          "accepted": c.read_json(c.ROOT / review["path"]).get("accepted")},
        "spike_doc_sha256": c.sha(c.ROOT / "docs/spikes/dynamic-resolution.md"),
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"),
        "brief_committed_sha256": blob("HEAD", f"handoffs/{c.PACKET}/brief.md"),
        "cargo_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
        "cargo_version": subprocess.run([str(c.ROOT / "tools/cargo"), "--version"], cwd=c.ROOT,
                                        capture_output=True, text=True, env=env).stdout.strip(),
        "rustc_version": subprocess.run([c.RUSTUP_BIN + "/rustc", "-vV"], cwd=c.ROOT, capture_output=True,
                                        text=True, env=env).stdout.strip(),
        "host": host_facts(), "helper_digest": digest, "helper_files": files,
        "cases_sha256": c.sha(c.CASES), "plan_sha256": c.sha(c.PLAN), "recorded": c.now()}
    c.write_new_json(c.FREEZE, record)
    print(json.dumps({k: v for k, v in record.items() if k not in ("manifest_sources", "grant", "helper_files")},
                     indent=1))
    if not (record["head"] == c.SOURCE_COMMIT and record["status_porcelain_outside_packet"] == ""
            and record["manifest_matches_source_commit_and_worktree"]
            and record["wgsl_changed_since_base"] == [] and record["cargo_lock_changed_since_base"] == []
            and record["files_after_implementation"] == [f"handoffs/{c.PACKET}/brief.md"]
            and record["source_review"]["manifest"] == record["source_review"]["worktree"]
            and record["source_review"]["accepted"] is True
            and record["brief_sha256"] == record["brief_committed_sha256"]):
        raise SystemExit("source freeze check failed")


def main():
    steps = {"freeze": freeze, "tests": tests, "helper-lock": helper_lock, "verify-lock": verify_lock,
             "helper": helper, **{f"helper-r{n}": (lambda n=n: helper(f"helper-r{n}")) for n in range(2, 10)}}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()


if __name__ == "__main__":
    main()
