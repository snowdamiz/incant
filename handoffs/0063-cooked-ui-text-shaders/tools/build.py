#!/usr/bin/env python3
"""CPU-only frozen builds for handoff 0063. Standard library only.

Runs ./tools/cargo with --locked, --profile ci and -j2 in the leased target
directory, parses Cargo's JSON artifact messages, copies every produced
executable into the ignored evidence directory and records SHA-256 values.
It never runs a built executable.

  python3 -I -B handoffs/0063-cooked-ui-text-shaders/tools/build.py <step>

Steps:
  freeze        source identities (manifests, commits, unchanged WGSL/geometry/theme)
  helper-lock   derive tools/review/Cargo.lock from the repository lock; verify each entry
  baseline | candidate | ui-only | text-only
                incant_render lib + model_gpu tests, incant_render_gpu_host ui + game
                tests and the packet review helper, all with that variant's features
  tools         incant_headless (default features) and the incant_cook cook_scene example
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


def git(*args):
    return subprocess.run(["git", *args], cwd=c.ROOT, capture_output=True, text=True, check=True).stdout


def cargo(label, args, locked=True):
    c.check_lease()
    env = c.clean_env({"CARGO_TARGET_DIR": str(c.TARGET),
                       "PATH": "/opt/homebrew/opt/rustup/bin:" + os.environ.get("PATH", "/usr/bin:/bin")})
    argv = [str(c.ROOT / "tools/cargo"), *args, *(["--locked"] if locked else []), "--offline",
            "--profile", "ci", "-j2", "--message-format=json-render-diagnostics"]
    logs = c.EVIDENCE / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    started = c.now()
    with open(logs / f"build-{label}.stdout.jsonl", "x") as out, open(logs / f"build-{label}.stderr.txt", "x") as err:
        result = subprocess.run(argv, cwd=c.ROOT, env=env, stdout=out, stderr=err)
    ended = c.now()
    artifacts = []
    for line in (logs / f"build-{label}.stdout.jsonl").read_text().splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact":
            continue
        name = message["target"]["name"]
        if message.get("executable") or name in ("incant_render", "incant_render_gpu_host"):
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


def freeze(record, picks, variant):
    """Copy selected executables to EVIDENCE/bin/<variant>/<name> and hash them."""
    destination = c.EVIDENCE / "bin" / variant
    destination.mkdir(parents=True, exist_ok=True)
    frozen = {}
    for name, match in picks.items():
        found = [a for a in record["artifacts"] if a["executable"] and match(a)]
        if len(found) != 1:
            raise SystemExit(f"{variant}/{name}: expected one artifact, found {len(found)}")
        source = Path(found[0]["executable"])
        target = destination / name
        if target.exists():
            raise SystemExit(f"{target} already exists; frozen binaries are never replaced")
        shutil.copyfile(source, target)
        os.chmod(target, 0o555)
        if c.sha(source) != c.sha(target):
            raise SystemExit(f"copy mismatch for {target}")
        frozen[name] = {"path": str(target), "sha256": c.sha(target), "bytes": target.stat().st_size,
                        "built_from": str(source), "fresh": found[0]["fresh"]}
    return frozen


def render_features(record):
    """Features of every non-test incant_render library unit in this build ("default" only
    aliases physics-deterministic, so it is dropped)."""
    return sorted({",".join(sorted(f for f in a["features"] if f != "default")) for a in record["artifacts"]
                   if a["target"] == "incant_render" and a["kind"] == ["lib"] and not a["test"]})


def variant(name):
    features = c.VARIANTS[name]
    expected = ",".join(sorted(["physics-deterministic", *[f.split("/")[1] for f in features.split(",")]]))
    render = cargo(f"{name}-render-tests", ["test", "--no-run", "-p", "incant_render", "--features", features])
    host = cargo(f"{name}-gpu-host-tests", ["test", "--no-run", "-p", "incant_render_gpu_host",
                                            "--features", features])
    helper = cargo(f"{name}-review", ["build", "--manifest-path", str(HELPER / "Cargo.toml"),
                                      "--bin", "ui_text_review", "--features", features])
    seen = {label: render_features(record) for label, record in
            (("render", render), ("gpu_host", host), ("review", helper))}
    if any(units != [expected] for units in seen.values()):
        raise SystemExit(f"unexpected incant_render feature sets: {seen} (expected {expected})")
    frozen = freeze(render, {
        "incant_render-lib-tests": lambda a: a["test"] and a["kind"] == ["lib"] and a["target"] == "incant_render",
        "model_gpu": lambda a: a["test"] and a["target"] == "model_gpu",
    }, name)
    frozen.update(freeze(host, {
        "gpu_host-ui": lambda a: a["test"] and a["target"] == "ui",
        "gpu_host-game": lambda a: a["test"] and a["target"] == "game",
    }, name))
    frozen.update(freeze(helper, {"ui_text_review": lambda a: a["target"] == "ui_text_review"}, name))
    c.write_new_json(c.EVIDENCE / "build" / f"{name}.json", {
        "variant": name, "features": features, "incant_render_features_seen": seen,
        "head": git("rev-parse", "HEAD").strip(), "frozen": frozen,
        "helper_sources": {p.name: c.sha(p) for p in (HELPER / "Cargo.toml", HELPER / "Cargo.lock",
                                                      HELPER / "src/main.rs")},
        "builds": [render, host, helper], "recorded": c.now()})
    print(json.dumps({"variant": name, "frozen": {k: v["sha256"] for k, v in frozen.items()}, "seen": seen},
                     indent=1))


def tools():
    headless = cargo("incant-headless", ["build", "-p", "incant_headless", "--bin", "incant_headless"])
    cook = cargo("cook-scene", ["build", "-p", "incant_cook", "--example", "cook_scene"])
    frozen = freeze(headless, {"incant_headless": lambda a: a["target"] == "incant_headless"}, "tools")
    frozen.update(freeze(cook, {"cook_scene": lambda a: a["target"] == "cook_scene"}, "tools"))
    c.write_new_json(c.EVIDENCE / "build" / "tools.json", {
        "head": git("rev-parse", "HEAD").strip(), "frozen": frozen, "builds": [headless, cook],
        "recorded": c.now()})
    print(json.dumps({k: v["sha256"] for k, v in frozen.items()}, indent=1))


def lock_packages(text):
    """Parse Cargo.lock [[package]] blocks into {(name, version, source): (checksum, [dependency refs])}.
    Other tables such as [[patch.unused]] end a package block and are reported separately."""
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
    """Dependency references ("name", "name version" or "name version (source)") as (name, version)."""
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
    """Seed the helper lock with the repository lock, let Cargo prune it offline, then verify."""
    c.check_lease()
    lock = HELPER / "Cargo.lock"
    if lock.exists():
        raise SystemExit("helper Cargo.lock already exists; verify it instead of regenerating")
    shutil.copyfile(c.ROOT / "Cargo.lock", lock)
    env = c.clean_env({"CARGO_TARGET_DIR": str(c.TARGET),
                       "PATH": "/opt/homebrew/opt/rustup/bin:" + os.environ.get("PATH", "/usr/bin:/bin")})
    argv = [str(c.ROOT / "tools/cargo"), "metadata", "--offline", "--format-version", "1",
            "--manifest-path", str(HELPER / "Cargo.toml")]
    result = subprocess.run(argv, cwd=c.ROOT, env=env, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"cargo metadata failed: {result.stderr[-4000:]}")
    verify_lock({"argv": argv, "exit": result.returncode, "stderr": result.stderr[-4000:]})


def verify_lock(how=None):
    root_text, helper_text = (c.ROOT / "Cargo.lock").read_text(), (HELPER / "Cargo.lock").read_text()
    root, helper = lock_packages(root_text), lock_packages(helper_text)
    root_deps, helper_deps = resolve_refs(root), resolve_refs(helper)
    own = [key for key in helper if key[0] == "incant_0063_review"]
    # Identity: every other entry must exist in the repository lock with the same source and checksum.
    foreign = [f"{k[0]} {k[1]}" for k in helper if k[0] != "incant_0063_review" and k not in root]
    checksum = [f"{k[0]} {k[1]}" for k in helper if k in root and helper[k][0] != root[k][0]]
    # Dependency edges after normalising version-suffix elision; a pruned graph may only drop edges.
    added_edges = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in helper_deps[k] - root_deps[k])
                   for k in helper if k in root and helper_deps[k] - root_deps[k]}
    dropped_edges = {f"{k[0]} {k[1]}": sorted(f"{n} {v}" for n, v in root_deps[k] - helper_deps[k])
                     for k in helper if k in root and root_deps[k] - helper_deps[k]}
    record = {"how": how, "repository_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
              "helper_lock_sha256": c.sha(HELPER / "Cargo.lock"), "repository_packages": len(root),
              "helper_packages": len(helper), "own_entries": [list(k) for k in own],
              "own_dependencies": sorted(f"{n} {v}" for n, v in helper_deps[own[0]]) if own else None,
              "not_in_repository_lock": foreign, "checksum_mismatches": checksum,
              "dependency_edges_added": added_edges, "dependency_edges_dropped": dropped_edges,
              "other_tables": re.findall(r"^\[\[(?!package\]\])[^\]]+\]\]\n(?:[^\n]+\n)*", helper_text, re.M),
              "recorded": c.now()}
    print(json.dumps(record, indent=1))
    if len(own) != 1 or foreign or checksum or added_edges:
        raise SystemExit("helper lock is not a pruned copy of the repository lock")
    c.write_new_json(c.EVIDENCE / "build" / f"helper-lock-{c.now().replace(':', '')}.json", record)


def source_freeze():
    ui = c.read_json(c.ROOT / "docs/spikes/evidence/cooked-ui-shaders.json")
    text = c.read_json(c.ROOT / "docs/spikes/evidence/cooked-text-shaders.json")

    def blob(commit, path):
        shown = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=c.ROOT, capture_output=True)
        return c.sha_bytes(shown.stdout) if shown.returncode == 0 else None

    def entries(manifest, commit):
        return [{"path": s["path"], "manifest": s["sha256"], "at_commit": blob(commit, s["path"]),
                 "worktree": c.sha(c.ROOT / s["path"]) if (c.ROOT / s["path"]).exists() else None}
                for s in manifest["source"]["sources"]]

    ui_sources = entries(ui, c.UI_COMMITS[0])
    text_sources = entries(text, c.SOURCE_COMMIT)
    unchanged = ["crates/incant_render/src/ui/ui.wgsl", "crates/incant_render/src/text/text.wgsl",
                 "crates/incant_render/src/ui/geometry.rs", "crates/incant_render/src/ui/theme.rs",
                 "crates/incant_render/src/text/geometry.rs"]
    first_parent = f"{c.UI_COMMITS[0]}~1"
    changed = git("diff", "--name-only", c.SOURCE_COMMIT, "HEAD").split()
    record = {
        "head": git("rev-parse", "HEAD").strip(), "source_commit": c.SOURCE_COMMIT, "ui_commits": c.UI_COMMITS,
        "files_changed_since_source_commit": changed,
        "only_evidence_and_brief_changed": all(p.startswith(("docs/spikes/evidence/", f"handoffs/{c.PACKET}/"))
                                               for p in changed),
        "crates_diff_empty": git("diff", "--stat", c.SOURCE_COMMIT, "HEAD", "--", "crates", "Cargo.lock",
                                 "Cargo.toml", "tools/cargo", "rust-toolchain.toml", "third_party") == "",
        "status_porcelain_outside_packet": git("status", "--porcelain", "--", ".", f":(exclude)handoffs/{c.PACKET}"),
        "ui_manifest_sources": ui_sources,
        "ui_manifest_matches_ui_commit": all(s["manifest"] == s["at_commit"] for s in ui_sources),
        "ui_sources_changed_after_ui_commit": [s["path"] for s in ui_sources if s["worktree"] != s["manifest"]],
        "text_manifest_sources": text_sources,
        "text_manifest_matches_source_commit_and_worktree": all(
            s["manifest"] == s["at_commit"] == s["worktree"] for s in text_sources),
        "unchanged_since_before_ui_commit": {
            p: {"sha256": c.sha(c.ROOT / p),
                "diff_from_parent_of_ui_commit_empty": git("diff", "--stat", first_parent, "HEAD", "--", p) == ""}
            for p in unchanged},
        "pipeline_files_diff_from_parent_of_ui_commit": git("diff", "--stat", first_parent, c.SOURCE_COMMIT, "--",
                                                            "crates/incant_render/src/ui/mod.rs",
                                                            "crates/incant_render/src/text/drawing.rs",
                                                            "crates/incant_render/src/text/mod.rs"),
        "source_reviews": {name: {"path": m["source_review"]["path"], "manifest": m["source_review"]["sha256"],
                                  "worktree": c.sha(c.ROOT / m["source_review"]["path"])
                                  if (c.ROOT / m["source_review"]["path"]).exists() else None}
                           for name, m in (("ui", ui), ("text", text))},
        "manifests": {p: c.sha(c.ROOT / p) for p in ("docs/spikes/evidence/cooked-ui-shaders.json",
                                                    "docs/spikes/evidence/cooked-text-shaders.json",
                                                    "docs/spikes/cooked-ui-shaders.md",
                                                    "docs/spikes/cooked-text-shaders.md")},
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"),
        "cargo_lock_sha256": c.sha(c.ROOT / "Cargo.lock"),
        "cargo_version": subprocess.run([str(c.ROOT / "tools/cargo"), "--version"], cwd=c.ROOT,
                                        capture_output=True, text=True,
                                        env=c.clean_env({"PATH": "/opt/homebrew/opt/rustup/bin:/usr/bin:/bin"})
                                        ).stdout.strip(),
        "rustc_version": subprocess.run(["/opt/homebrew/opt/rustup/bin/rustc", "-vV"], cwd=c.ROOT,
                                        capture_output=True, text=True).stdout.strip(),
        "recorded": c.now()}
    c.write_new_json(c.EVIDENCE / "build" / "source-freeze.json", record)
    print(json.dumps({k: v for k, v in record.items() if not k.endswith("_sources")}, indent=1))


def main():
    steps = ("freeze", "helper-lock", "verify-lock", "tools", *c.VARIANTS)
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    step = sys.argv[1]
    if step == "freeze":
        source_freeze()
    elif step == "helper-lock":
        helper_lock()
    elif step == "verify-lock":
        verify_lock()
    elif step == "tools":
        tools()
    else:
        variant(step)


if __name__ == "__main__":
    main()
