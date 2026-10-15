"""Read-only replay of optional Vulkan cache CPU qualification; never runs native code."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for data in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(data)
    return h.hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key: " + key)
        result[key] = value
    return result


def parse(data):
    return json.loads(data, object_pairs_hook=unique)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--originals", action="store_true")
    parser.add_argument("--binaries", action="store_true")
    parser.add_argument("--git-source", action="store_true")
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[3]
    m = parse((here / "manifest.json").read_bytes())
    assert m["format"] == "incant-optional-pipeline-cache-cpu" and m["version"] == 1
    archive = (here / "records.json.gz").read_bytes()
    assert len(archive) <= 8 * 1024**2 and len(archive) == m["archive"]["bytes"]
    assert digest(archive) == m["archive"]["sha256"]
    with gzip.GzipFile(fileobj=io.BytesIO(archive)) as stream:
        raw = stream.read(16 * 1024**2 + 1)
    assert len(raw) <= 16 * 1024**2 and len(raw) == m["archive"]["inflated_bytes"]
    rows = parse(raw)
    assert len(rows) == m["archive"]["records"]
    records = {}
    for row in rows:
        path = PurePosixPath(row["path"])
        assert not path.is_absolute() and ".." not in path.parts and path.parts
        assert row["path"] not in records
        data = base64.b64decode(row["base64"], validate=True)
        assert len(data) <= 4 * 1024**2 and len(data) == row["bytes"]
        assert digest(data) == row["sha256"]
        records[row["path"]] = data
        if args.originals:
            assert file_digest(Path(m["original_root"]) / row["path"]) == row["sha256"]
    for path, expected in m["bindings"].items():
        assert digest(records[path]) == expected, path
    source_rows = 0
    for generation, commit in [("001", m["production_source"]), ("002", m["qualified_source"])]:
        freeze = parse(records[f"source-{generation}/freeze.json"])
        assert freeze["source"] == commit and freeze["base"] == m["base"]
        assert len({row["path"] for row in freeze["rows"]}) == len(freeze["rows"])
        for row in freeze["rows"]:
            original = f"source-{generation}/inputs/" + row["path"]
            retained = original if original in records else "source-002/inputs/" + row["path"]
            assert digest(records[retained]) == row["sha256"]
            assert len(records[retained]) == row["bytes"]
            if args.originals:
                assert file_digest(Path(m["original_root"]) / original) == row["sha256"]
            if args.git_source:
                data = subprocess.check_output(["git", "show", commit + ":" + row["path"]], cwd=root, timeout=15)
                assert digest(data) == row["sha256"], row["path"]
            source_rows += 1
    first = parse(records["source-001/freeze.json"])
    second = parse(records["source-002/freeze.json"])
    a = {r["path"]: r["sha256"] for r in first["rows"]}
    b = {r["path"]: r["sha256"] for r in second["rows"]}
    assert a.keys() == b.keys()
    assert [path for path in a if a[path] != b[path]] == ["crates/incant_render/tests/persistent_cache.rs"]
    plans = [parse(records[f"cpu-plan-{n:03}.json"]) for n in (1, 2, 3)]
    assert plans[0]["steps"] == plans[1]["steps"] == plans[2]["steps"]
    p = plans[2]
    assert p["source"] == m["qualified_source"] and p["jobs"] == p["rayon_threads"] == 2
    assert not p["gpu_or_window"] and p["stop_first_failure"]
    baseline = parse(records["target-baseline-003.json"])
    assert baseline["target"] == p["requested_target"]
    assert baseline["regular_tree_lstat_bytes"] == p["cumulative_target_initial_bytes"]
    grant = parse(records["root-cpu-grant-003.json"])
    assert grant["accepted_for_cpu"] and grant["source"] == p["source"]
    assert grant["plan_sha256"] == digest(records["cpu-plan-003.json"])
    assert grant["runner_sha256"] == digest(records["qualify-003.py"])
    result = parse(records["cpu-003/result.json"])
    assert result["outcome"] == "success" and result["error"] is None
    assert result["source"] == p["source"]
    assert result["plan_sha256"] == grant["plan_sha256"] and result["runner_sha256"] == grant["runner_sha256"]
    assert result["native_process_sha256"] == digest(records["native_process.py"])
    assert not any(result[k] for k in ["managed_call_active", "gpu_execution", "window_execution", "listener_execution", "native_player_execution"])
    assert result["elapsed_seconds"] <= p["overall_seconds"]
    assert result["minimum_sampled_free_bytes"] >= p["minimum_free_bytes"]
    assert result["target_initial_bytes"] == p["cumulative_target_initial_bytes"]
    assert result["target_last_sampled_bytes"] - result["target_initial_bytes"] <= p["maximum_target_growth_bytes"]
    commands = result["commands"]
    assert len(commands) == len(p["steps"]) + 1 and commands[0]["name"] == "rust-version"
    copies = []
    for command in commands:
        assert command["exit_code"] == 0 and command["managed_returned_after_cleanup"] and "error" not in command
        for stream in ("stdout", "stderr"):
            data = records[f"cpu-003/{command['name']}.{stream}"]
            assert digest(data) == command[stream + "_sha256"] and len(data) <= p["maximum_stream_bytes"]
        copies.extend(command.get("artifacts", []))
    for step, command in zip(p["steps"], commands[1:]):
        assert command["name"] == step["name"] and command["argv"] == step["argv"]
        if step.get("tests"):
            log = records[f"cpu-003/{step['name']}.stdout"].decode()
            summaries = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", log)
            assert summaries and all(s[0] == "ok" and s[2] == "0" for s in summaries)
            assert sum(int(s[1]) for s in summaries) == step["expected_test_count"]
        if step.get("copy_artifacts"):
            assert "--no-run" in step["argv"] and command["artifacts"]
    assert [(r["path"], r["sha256"]) for r in copies] == [(r["path"], r["sha256"]) for r in m["copied_binaries"]]
    if args.binaries:
        for row in m["copied_binaries"]:
            path = Path(row["path"])
            assert path.stat().st_size == row["bytes"] and file_digest(path) == row["sha256"]
    print(json.dumps({"records": len(records), "source_rows": source_rows, "executed_commands": len(commands),
        "cpu_tests_on": 19, "cpu_tests_off": 3, "external_binaries_rehashed": len(copies) if args.binaries else 0,
        "git_source_verified": args.git_source, "vulkan_execution": False, "window_or_capture": False}))


if __name__ == "__main__":
    main()
