#!/usr/bin/env python3
"""CPU-only frozen builds for handoff 0066. Standard library only.

Adapted from handoff 0064's committed tools/build.py (read-only reuse). Runs
./tools/cargo with --locked, --offline, --profile ci and -j2 in the leased
target directory, parses Cargo's JSON compiler-artifact and build-script
messages, copies each selected executable into the ignored evidence directory
and records SHA-256 values. It never runs a built executable.

  python3 -I -B handoffs/0066-cooked-cluster-shaders/tools/build.py <step>

Steps:
  freeze              source identities: grant commit, manifest inputs, unchanged grid WGSL
  tools               incant_headless and the incant_cook cook_scene example (default features)
  tests-<variant>     incant_render (lib + model_gpu) and incant_render_gpu_host (game, ui) tests
  helper-lock         seed tools/review/Cargo.lock from the repository lock, let Cargo prune it offline
  verify-lock         check every helper lock entry against the repository lock
  review-<variant>    the packet review helper with that variant's features
  embed               byte-search frozen binaries for grid.wgsl and the actual ci cluster artifacts

Variants: baseline, candidate (see common.VARIANTS).
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

HELPER = c.TOOLS / "review"
GRID = "crates/incant_render/src/lighting/grid.wgsl"
CLUSTER_FILES = ("Metal-Compute.shader", "Vulkan-Compute.shader", "Dx12-Compute.shader")


def cargo(label, args):
    c.check_lease()
    c.check_source()
    argv = [str(c.ROOT / "tools/cargo"), *args, "--locked", "--offline", "--profile", "ci", "-j2",
            "--message-format=json-render-diagnostics"]
    logs = c.EVIDENCE / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    started = c.now()
    with open(logs / f"build-{label}.stdout.jsonl", "x") as out, open(logs / f"build-{label}.stderr.txt", "x") as err:
        result = subprocess.run(argv, cwd=c.ROOT, env=c.build_env(), stdout=out, stderr=err)
    ended = c.now()
    artifacts, scripts = [], []
    for line in (logs / f"build-{label}.stdout.jsonl").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") == "build-script-executed" and "/incant_render#" in message["package_id"]:
            scripts.append({"package_id": message["package_id"], "out_dir": message["out_dir"]})
        if message.get("reason") != "compiler-artifact":
            continue
        name = message["target"]["name"]
        if message.get("executable") or name == "incant_render":
            artifacts.append({"target": name, "kind": message["target"]["kind"],
                              "test": message["profile"]["test"], "fresh": message["fresh"],
                              "executable": message.get("executable"), "features": message.get("features", []),
                              "package_id": message["package_id"], "filenames": message.get("filenames", [])})
    record = {"label": label, "argv": argv, "cwd": str(c.ROOT), "env": {"CARGO_TARGET_DIR": str(c.TARGET)},
              "started": started, "ended": ended, "exit": result.returncode,
              "stdout_sha256": c.sha(logs / f"build-{label}.stdout.jsonl"),
              "stderr_sha256": c.sha(logs / f"build-{label}.stderr.txt"), "artifacts": artifacts,
              "incant_render_build_scripts": [{**s, "outputs": out_dir_files(s["out_dir"])} for s in scripts]}
    if result.returncode != 0:
        c.write_new_json(c.EVIDENCE / "build" / f"{label}.failed.json", record)
        raise SystemExit(f"cargo {label} failed with {result.returncode}; see {logs}/build-{label}.stderr.txt")
    return record


def out_dir_files(out_dir):
    """Every file incant_render's build script wrote (cooked shader families and generated manifests)."""
    root = Path(out_dir)
    return {str(p.relative_to(root)): {"sha256": c.sha(p), "bytes": p.stat().st_size}
            for p in sorted(root.rglob("*")) if p.is_file()} if root.is_dir() else {}


def freeze_binaries(record, picks, group):
    """Copy selected executables to EVIDENCE/bin/<group>/<name> and hash them."""
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
                        "built_from": str(source), "fresh": found[0]["fresh"], "features": found[0]["features"]}
    return frozen


def render_features(record):
    """Feature sets of every non-test incant_render library unit in this build. "default"
    only aliases physics-deterministic, so it is dropped before comparison."""
    return sorted({",".join(sorted(f for f in a["features"] if f != "default")) for a in record["artifacts"]
                   if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]})


def expected_features(variant):
    names = [f.split("/")[1] for f in c.VARIANTS[variant].split(",") if f]
    return ",".join(sorted(["physics-deterministic", *names]))


def check_features(variant, record):
    seen = render_features(record)
    if seen != [expected_features(variant)]:
        raise SystemExit(f"unexpected incant_render feature sets {seen}; expected {expected_features(variant)}")
    return seen


def shader_families(record):
    """Cooked shader outputs of the single incant_render build-script run used by this build."""
    runs = {json.dumps(s["outputs"], sort_keys=True): s for s in record["incant_render_build_scripts"]}
    if len(runs) != 1:
        raise SystemExit(f"expected one incant_render build-script output set, found {len(runs)}")
    return next(iter(runs.values()))


def tests(variant):
    record = cargo(f"tests-{variant}", ["test", "--no-run", "-p", "incant_render", "-p", "incant_render_gpu_host",
                                        "--features", c.VARIANTS[variant]])
    seen = check_features(variant, record)
    frozen = freeze_binaries(record, {
        "incant_render-lib-tests": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render",
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu",
        "gpu_host-game": lambda a: a["test"] and a["target"] == "game",
        "gpu_host-ui": lambda a: a["test"] and a["target"] == "ui",
    }, variant)
    c.write_new_json(c.EVIDENCE / "build" / f"tests-{variant}.json", {
        "variant": variant, "features": c.VARIANTS[variant], "incant_render_features_seen": seen,
        "head": c.git("rev-parse", "HEAD").strip(), "frozen": frozen, "shader_out_dir": shader_families(record),
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"variant": variant, "seen": seen, "frozen": {k: v["sha256"] for k, v in frozen.items()},
                      "out_dir_files": sorted(shader_families(record)["outputs"])}, indent=1))


def review(variant):
    features = c.VARIANTS[variant]
    for path in (HELPER / "Cargo.toml", HELPER / "Cargo.lock", HELPER / "src/main.rs"):
        if not path.is_file():
            raise SystemExit(f"missing helper source {path}")
    verify_lock(write=False)
    record = cargo(f"review-{variant}", ["build", "--manifest-path", str(HELPER / "Cargo.toml"),
                                         "--bin", "cluster_review", "--features", features])
    seen = check_features(variant, record)
    frozen = freeze_binaries(record, {"cluster_review": lambda a: a["target"] == "cluster_review"}, variant)
    c.write_new_json(c.EVIDENCE / "build" / f"review-{variant}.json", {
        "variant": variant, "features": features, "incant_render_features_seen": seen,
        "head": c.git("rev-parse", "HEAD").strip(), "frozen": frozen, "shader_out_dir": shader_families(record),
        "helper_sources": {str(p.relative_to(c.ROOT)): c.sha(p) for p in sorted(HELPER.rglob("*")) if p.is_file()},
        "builds": [record], "recorded": c.now()})
    print(json.dumps({"variant": variant, "seen": seen, "frozen": {k: v["sha256"] for k, v in frozen.items()}},
                     indent=1))


def tools():
    headless = cargo("incant-headless", ["build", "-p", "incant_headless", "--bin", "incant_headless"])
    cook = cargo("cook-scene", ["build", "-p", "incant_cook", "--example", "cook_scene"])
    frozen = freeze_binaries(headless, {"incant_headless": lambda a: a["target"] == "incant_headless"}, "tools")
    frozen.update(freeze_binaries(cook, {"cook_scene": lambda a: a["target"] == "cook_scene"}, "tools"))
    c.write_new_json(c.EVIDENCE / "build" / "tools.json", {
        "head": c.git("rev-parse", "HEAD").strip(), "frozen": frozen, "builds": [headless, cook],
        "recorded": c.now()})
    print(json.dumps({k: v["sha256"] for k, v in frozen.items()}, indent=1))


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
    lock = HELPER / "Cargo.lock"
    if lock.exists():
        raise SystemExit("helper Cargo.lock already exists; verify it instead of regenerating")
    shutil.copyfile(c.ROOT / "Cargo.lock", lock)
    argv = [str(c.ROOT / "tools/cargo"), "metadata", "--offline", "--format-version", "1",
            "--manifest-path", str(HELPER / "Cargo.toml")]
    result = subprocess.run(argv, cwd=c.ROOT, env=c.build_env(), capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"cargo metadata failed: {result.stderr[-4000:]}")
    verify_lock(how={"argv": argv, "exit": result.returncode, "stderr": result.stderr[-4000:]})


def verify_lock(how=None, write=True):
    root_text, helper_text = (c.ROOT / "Cargo.lock").read_text(), (HELPER / "Cargo.lock").read_text()
    root, helper = lock_packages(root_text), lock_packages(helper_text)
    root_deps, helper_deps = resolve_refs(root), resolve_refs(helper)
    own = [key for key in helper if key[0] == "incant_0066_review"]
    foreign = [f"{k[0]} {k[1]}" for k in helper if k[0] != "incant_0066_review" and k not in root]
    checksum = [f"{k[0]} {k[1]}" for k in helper if k in root and helper[k][0] != root[k][0]]
    added = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in helper_deps[k] - root_deps[k])
             for k in helper if k in root and helper_deps[k] - root_deps[k]}
    dropped = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in root_deps[k] - helper_deps[k])
               for k in helper if k in root and root_deps[k] - helper_deps[k]}
    record = {"how": how, "repository_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
              "helper_lock_sha256": c.sha(HELPER / "Cargo.lock"), "repository_packages": len(root),
              "helper_packages": len(helper), "own_entries": [list(k) for k in own],
              "own_dependencies": sorted(f"{n} {v}" for n, v in helper_deps[own[0]]) if own else None,
              "not_in_repository_lock": foreign, "checksum_mismatches": checksum,
              "dependency_edges_added": added, "dependency_edges_dropped": dropped,
              "other_tables": re.findall(r"^\[\[(?!package\]\])[^\]]+\]\]\n(?:[^\n]+\n)*", helper_text, re.M),
              "recorded": c.now()}
    if len(own) != 1 or foreign or checksum or added:
        print(json.dumps(record, indent=1))
        raise SystemExit("helper lock is not a pruned copy of the repository lock")
    if write:
        c.write_new_json(c.EVIDENCE / "build" / f"helper-lock-{c.now().replace(':', '')}.json", record)
        print(json.dumps(record, indent=1))
    return record


# ------------------------------------------------------------------ identities

def freeze_sources():
    c.check_grant()
    manifest = c.read_json(c.MANIFEST)

    def blob(commit, path):
        shown = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=c.ROOT, capture_output=True)
        return c.sha_bytes(shown.stdout) if shown.returncode == 0 else None

    sources = [{"path": s["path"], "manifest": s["sha256"], "at_source_commit": blob(c.SOURCE_COMMIT, s["path"]),
                "worktree": c.sha(c.ROOT / s["path"]) if (c.ROOT / s["path"]).exists() else None}
               for s in manifest["sources"]]
    parent = f"{c.SOURCE_COMMIT}~1"
    review = manifest["source_review"]
    record = {
        "grant_sha256": c.sha(c.GRANT), "grant": c.read_json(c.GRANT), "source_check": c.check_source(),
        "head": c.git("rev-parse", "HEAD").strip(), "source_commit": c.SOURCE_COMMIT,
        "files_changed_since_source_commit": c.git("diff", "--name-only", c.SOURCE_COMMIT, "HEAD").split(),
        "status_porcelain_outside_packet": c.git("status", "--porcelain", "--", ".", f":(exclude)handoffs/{c.PACKET}"),
        "manifest_sources": sources,
        "manifest_matches_source_commit_and_worktree": all(
            s["manifest"] == s["at_source_commit"] == s["worktree"] for s in sources),
        "grid_wgsl": {"sha256": c.sha(c.ROOT / GRID), "at_parent_of_source_commit": blob(parent, GRID),
                      "diff_from_parent_empty": c.git("diff", "--stat", parent, "HEAD", "--", GRID) == ""},
        "manifest_sha256": c.sha(c.MANIFEST),
        "ci_artifacts_in_manifest": c.read_json(c.MANIFEST)["ci_validation"]["artifacts"],
        "spike_doc_sha256": c.sha(c.ROOT / "docs/spikes/cooked-cluster-shaders.md"),
        "source_review": {"path": review["path"], "manifest": review["sha256"],
                          "worktree": c.sha(c.ROOT / review["path"]),
                          "accepted": c.read_json(c.ROOT / review["path"]).get("accepted")},
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"),
        "cargo_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
        "cargo_version": subprocess.run([str(c.ROOT / "tools/cargo"), "--version"], cwd=c.ROOT, capture_output=True,
                                        text=True, env=c.build_env()).stdout.strip(),
        "rustc_version": subprocess.run([c.RUSTUP_BIN + "/rustc", "-vV"], cwd=c.ROOT, capture_output=True,
                                        text=True, env=c.build_env()).stdout.strip(),
        "recorded": c.now()}
    c.write_new_json(c.EVIDENCE / "build" / "source-freeze.json", record)
    print(json.dumps({k: v for k, v in record.items() if k not in ("manifest_sources", "grant")}, indent=1))
    if not (record["head"] == c.SOURCE_COMMIT and record["files_changed_since_source_commit"] == []
            and record["status_porcelain_outside_packet"] == ""
            and record["manifest_matches_source_commit_and_worktree"]
            and record["grid_wgsl"]["diff_from_parent_empty"]
            and record["grid_wgsl"]["sha256"] == record["grid_wgsl"]["at_parent_of_source_commit"]
            and record["source_review"]["manifest"] == record["source_review"]["worktree"]
            and record["source_review"]["accepted"] is True):
        raise SystemExit("source freeze check failed")


def embed():
    """Which cluster shader each frozen binary carries (byte search; no execution). Needles are
    the grid WGSL and the candidate build's actual ci-profile OUT_DIR artifacts, which must match
    the manifest's separately recorded ci hashes."""
    ci = {Path(a["path"]).name: a["sha256"] for a in c.read_json(c.MANIFEST)["ci_validation"]["artifacts"]}
    candidate = c.read_json(c.EVIDENCE / "build" / "tests-candidate.json")["shader_out_dir"]
    needles = {"grid.wgsl": (c.ROOT / GRID).read_bytes()}
    artifacts = {}
    for name in CLUSTER_FILES:
        data = (Path(candidate["out_dir"]) / "cluster-shaders" / name).read_bytes()
        artifacts[name] = {"bytes": len(data), "sha256": c.sha_bytes(data), "manifest_ci_sha256": ci.get(name),
                           "matches_manifest_ci": c.sha_bytes(data) == ci.get(name)}
        needles[f"cooked {name}"] = data
    out = {"candidate_out_dir": candidate["out_dir"], "cluster_artifacts": artifacts, "binaries": {},
           "out_dir_families": {}}
    for variant in c.VARIANTS:
        for step in ("tests", "review"):
            path = c.EVIDENCE / "build" / f"{step}-{variant}.json"
            if not path.exists():
                continue
            built = c.read_json(path)
            out["out_dir_families"][f"{step}-{variant}"] = {
                "out_dir": built["shader_out_dir"]["out_dir"],
                "families": sorted({k.split("/")[0] for k in built["shader_out_dir"]["outputs"]})}
            for name, entry in built["frozen"].items():
                data = Path(entry["path"]).read_bytes()
                if c.sha_bytes(data) != entry["sha256"]:
                    raise SystemExit(f"{entry['path']} changed since freezing")
                out["binaries"][f"{variant}/{name}"] = {
                    "sha256": entry["sha256"], "counts": {k: data.count(v) for k, v in needles.items()}}
    # Every non-cluster cooked family must be byte-identical between the variants' build-script outputs.
    base = c.read_json(c.EVIDENCE / "build" / "tests-baseline.json")["shader_out_dir"]["outputs"]
    cand = candidate["outputs"]
    out["non_cluster_outputs_identical"] = {
        k: base.get(k) == v for k, v in sorted(cand.items()) if not k.startswith("cluster-shaders/")}
    out["baseline_has_cluster_outputs"] = any(k.startswith("cluster-shaders/") for k in base)
    out["baseline_only_outputs"] = sorted(set(base) - set(cand))
    c.write_new_json(c.EVIDENCE / "build" / f"embed-{c.now().replace(':', '')}.json", out)
    print(json.dumps(out, indent=1))


def main():
    steps = {"freeze": freeze_sources, "tools": tools, "helper-lock": helper_lock, "verify-lock": verify_lock,
             "embed": embed}
    for variant in c.VARIANTS:
        steps[f"tests-{variant}"] = lambda v=variant: tests(v)
        steps[f"review-{variant}"] = lambda v=variant: review(v)
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()


if __name__ == "__main__":
    main()
