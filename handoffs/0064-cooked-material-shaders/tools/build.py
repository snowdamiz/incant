#!/usr/bin/env python3
"""CPU-only frozen builds for handoff 0064. Standard library only.

Runs ./tools/cargo with --locked, --offline, --profile ci and -j2 in the leased
target directory, parses Cargo's JSON artifact messages, copies each selected
executable into the ignored evidence directory and records SHA-256 values. It
never runs a built executable.

  python3 -I -B handoffs/0064-cooked-material-shaders/tools/build.py <step>

Steps:
  freeze              source identities: manifest inputs, unchanged WGSL, HEAD diff
  tools               incant_headless and the incant_cook cook_scene example (default features)
  tests-<variant>     incant_render (lib + model_gpu + examples) and incant_render_gpu_host tests;
                      also tests-default (repository default features; extra WGSL control)
  helper-lock         seed tools/review/Cargo.lock from the repository lock, let Cargo prune it offline
  verify-lock         check every helper lock entry against the repository lock
  review-<variant>    the packet review helper with that variant's features
  embed               byte-search frozen binaries for WGSL text and Astra's retained cooked MSL

Variants: baseline, candidate (see common.VARIANTS).
"""

import gzip
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
MANIFEST = c.ROOT / "docs/spikes/evidence/cooked-material-shaders.json"
WGSL = ["crates/incant_render/src/model_material.wgsl", "crates/incant_render/src/lighting/shade.wgsl",
        "crates/incant_render/src/shadows/shade.wgsl"]


def git(*args):
    return subprocess.run(["git", *args], cwd=c.ROOT, capture_output=True, text=True, check=True).stdout


def cargo(label, args):
    c.check_lease()
    argv = [str(c.ROOT / "tools/cargo"), *args, "--locked", "--offline", "--profile", "ci", "-j2",
            "--message-format=json-render-diagnostics"]
    logs = c.EVIDENCE / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    started = c.now()
    with open(logs / f"build-{label}.stdout.jsonl", "x") as out, open(logs / f"build-{label}.stderr.txt", "x") as err:
        result = subprocess.run(argv, cwd=c.ROOT, env=c.build_env(), stdout=out, stderr=err)
    ended = c.now()
    artifacts = []
    for line in (logs / f"build-{label}.stdout.jsonl").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
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
              "stderr_sha256": c.sha(logs / f"build-{label}.stderr.txt"), "artifacts": artifacts}
    if result.returncode != 0:
        c.write_new_json(c.EVIDENCE / "build" / f"{label}.failed.json", record)
        raise SystemExit(f"cargo {label} failed with {result.returncode}; see {logs}/build-{label}.stderr.txt")
    return record


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
    only aliases physics-deterministic (absent when the helper disables default features),
    so it is dropped before comparison."""
    return sorted({",".join(sorted(f for f in a["features"] if f != "default")) for a in record["artifacts"]
                   if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]})


# Extra control: the repository's default features (every shader family on WGSL). Built and
# tested only (no review helper); it is not one of the brief's two compared variants.
TEST_VARIANTS = {**c.VARIANTS, "default": ""}


def expected_features(variant):
    names = [f.split("/")[1] for f in TEST_VARIANTS[variant].split(",") if f]
    return ",".join(sorted(["physics-deterministic", *names]))


def check_features(variant, record):
    seen = render_features(record)
    if seen != [expected_features(variant)]:
        raise SystemExit(f"unexpected incant_render feature sets {seen}; expected {expected_features(variant)}")
    return seen


def tests(variant):
    features = TEST_VARIANTS[variant]
    record = cargo(f"tests-{variant}", ["test", "--no-run", "-p", "incant_render", "-p", "incant_render_gpu_host",
                                        *(["--features", features] if features else [])])
    seen = check_features(variant, record)
    frozen = freeze_binaries(record, {
        "incant_render-lib-tests": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render",
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu",
        "gpu_host-game": lambda a: a["test"] and a["target"] == "game",
        "gpu_host-ui": lambda a: a["test"] and a["target"] == "ui",
        "native_frames": lambda a: a["target"] == "native_frames" and a["kind"] == ["example"],
    }, variant)
    c.write_new_json(c.EVIDENCE / "build" / f"tests-{variant}.json", {
        "variant": variant, "features": features, "incant_render_features_seen": seen,
        "head": git("rev-parse", "HEAD").strip(), "frozen": frozen, "builds": [record], "recorded": c.now()})
    print(json.dumps({"variant": variant, "seen": seen, "frozen": {k: v["sha256"] for k, v in frozen.items()}},
                     indent=1))


def review(variant):
    features = c.VARIANTS[variant]
    for path in (HELPER / "Cargo.toml", HELPER / "Cargo.lock", HELPER / "src/main.rs"):
        if not path.is_file():
            raise SystemExit(f"missing helper source {path}")
    verify_lock(write=False)
    record = cargo(f"review-{variant}", ["build", "--manifest-path", str(HELPER / "Cargo.toml"),
                                         "--bin", "material_review", "--features", features])
    seen = check_features(variant, record)
    frozen = freeze_binaries(record, {"material_review": lambda a: a["target"] == "material_review"}, variant)
    c.write_new_json(c.EVIDENCE / "build" / f"review-{variant}.json", {
        "variant": variant, "features": features, "incant_render_features_seen": seen,
        "head": git("rev-parse", "HEAD").strip(), "frozen": frozen,
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
        "head": git("rev-parse", "HEAD").strip(), "frozen": frozen, "builds": [headless, cook],
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
    own = [key for key in helper if key[0] == "incant_0064_review"]
    foreign = [f"{k[0]} {k[1]}" for k in helper if k[0] != "incant_0064_review" and k not in root]
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
    manifest = c.read_json(MANIFEST)

    def blob(commit, path):
        shown = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=c.ROOT, capture_output=True)
        return c.sha_bytes(shown.stdout) if shown.returncode == 0 else None

    sources = [{"path": s["path"], "manifest": s["sha256"], "at_source_commit": blob(c.SOURCE_COMMIT, s["path"]),
                "worktree": c.sha(c.ROOT / s["path"]) if (c.ROOT / s["path"]).exists() else None}
               for s in manifest["sources"]]
    parent = f"{c.SOURCE_COMMIT}~1"
    changed = git("diff", "--name-only", c.SOURCE_COMMIT, "HEAD").split()
    record = {
        "head": git("rev-parse", "HEAD").strip(), "source_commit": c.SOURCE_COMMIT,
        "files_changed_since_source_commit": changed,
        "only_brief_changed": changed == [f"handoffs/{c.PACKET}/brief.md"],
        "crates_diff_empty": git("diff", "--stat", c.SOURCE_COMMIT, "HEAD", "--", "crates", "Cargo.lock",
                                 "Cargo.toml", "tools/cargo", "rust-toolchain.toml", "third_party") == "",
        "status_porcelain_outside_packet": git("status", "--porcelain", "--", ".", f":(exclude)handoffs/{c.PACKET}"),
        "manifest_sources": sources,
        "manifest_matches_source_commit_and_worktree": all(
            s["manifest"] == s["at_source_commit"] == s["worktree"] for s in sources),
        "wgsl_unchanged_from_parent_of_source_commit": {
            p: {"sha256": c.sha(c.ROOT / p), "at_parent": blob(parent, p),
                "diff_empty": git("diff", "--stat", parent, "HEAD", "--", p) == ""} for p in WGSL},
        "manifest_sha256": c.sha(MANIFEST), "spike_doc_sha256": c.sha(c.ROOT / "docs/spikes/cooked-material-shaders.md"),
        "source_review": {"path": manifest["source_review"]["path"], "manifest": manifest["source_review"]["sha256"],
                          "worktree": c.sha(c.ROOT / manifest["source_review"]["path"])},
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"),
        "cargo_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
        "cargo_version": subprocess.run([str(c.ROOT / "tools/cargo"), "--version"], cwd=c.ROOT, capture_output=True,
                                        text=True, env=c.build_env()).stdout.strip(),
        "rustc_version": subprocess.run([c.RUSTUP_BIN + "/rustc", "-vV"], cwd=c.ROOT, capture_output=True,
                                        text=True, env=c.build_env()).stdout.strip(),
        "recorded": c.now()}
    c.write_new_json(c.EVIDENCE / "build" / "source-freeze.json", record)
    print(json.dumps({k: v for k, v in record.items() if k != "manifest_sources"}, indent=1))
    if not (record["only_brief_changed"] and record["crates_diff_empty"]
            and record["manifest_matches_source_commit_and_worktree"]
            and all(v["diff_empty"] for v in record["wgsl_unchanged_from_parent_of_source_commit"].values())):
        raise SystemExit("source freeze check failed")


def embed():
    """Which material shader each frozen binary carries (byte search; no execution)."""
    manifest = c.read_json(MANIFEST)
    retained = {}
    for artifact in manifest["artifacts"]:
        name = Path(artifact["path"]).name
        if name.endswith(".shader.gz") and not name.startswith(("36-", "37-")):
            data = gzip.decompress((c.ROOT / artifact["path"]).read_bytes())
            if c.sha_bytes(data) != artifact["sha256"]:
                raise SystemExit(f"retained artifact {name} does not match its manifest hash")
            retained[name] = data
    needles = {"model_material.wgsl": (c.ROOT / WGSL[0]).read_bytes(),
               "lighting/shade.wgsl": (c.ROOT / WGSL[1]).read_bytes(),
               "shadows/shade.wgsl": (c.ROOT / WGSL[2]).read_bytes(),
               **{f"cooked {k}": v for k, v in retained.items()}}
    out = {"retained_artifacts": {k: {"bytes": len(v), "sha256": c.sha_bytes(v)} for k, v in retained.items()},
           "binaries": {}}
    for variant in TEST_VARIANTS:
        for step in ("tests", "review"):
            path = c.EVIDENCE / "build" / f"{step}-{variant}.json"
            if not path.exists():
                continue
            for name, entry in c.read_json(path)["frozen"].items():
                data = Path(entry["path"]).read_bytes()
                if c.sha_bytes(data) != entry["sha256"]:
                    raise SystemExit(f"{entry['path']} changed since freezing")
                out["binaries"][f"{variant}/{name}"] = {
                    "sha256": entry["sha256"], "counts": {k: data.count(v) for k, v in needles.items()}}
    c.write_new_json(c.EVIDENCE / "build" / f"embed-{c.now().replace(':', '')}.json", out)
    print(json.dumps(out, indent=1))


def main():
    steps = {"freeze": freeze_sources, "tools": tools, "helper-lock": helper_lock, "verify-lock": verify_lock,
             "embed": embed}
    for variant in TEST_VARIANTS:
        steps[f"tests-{variant}"] = lambda v=variant: tests(v)
    for variant in c.VARIANTS:
        steps[f"review-{variant}"] = lambda v=variant: review(v)
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()


if __name__ == "__main__":
    main()
