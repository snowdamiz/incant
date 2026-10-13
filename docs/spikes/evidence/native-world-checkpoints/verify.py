#!/usr/bin/env python3
"""Verify retained checkpoint evidence without compiling or executing binaries."""

import argparse
import base64
import gzip
import hashlib
import json
import re
from pathlib import Path, PurePosixPath


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--local-binaries", action="store_true")
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[3]
    manifest = json.loads((here / "manifest.json").read_bytes())
    archive = (here / manifest["archive"]["path"]).read_bytes()
    require(sha(archive) == manifest["archive"]["sha256"], "archive identity")
    rows = json.loads(gzip.decompress(archive))
    require(len(rows) == manifest["archive"]["records"], "record count")
    records = {}
    for row in rows:
        path = PurePosixPath(row["path"])
        require(not path.is_absolute() and ".." not in path.parts, "record path")
        require(row["path"] not in records, "duplicate record")
        data = base64.b64decode(row["data_base64"], validate=True)
        require(len(data) == row["bytes"] and sha(data) == row["sha256"], row["path"])
        records[row["path"]] = data

    freeze = records["freeze-004.json"]
    require(sha(freeze) == manifest["source_freeze_sha256"], "source freeze")
    frozen = {row["path"]: row["sha256"] for row in json.loads(freeze)["sources"]}
    for row in manifest["sources"]:
        path = row["path"]
        require(sha((root / path).read_bytes()) == row["sha256"] == frozen[path], path)
        require(sha(records["source-004/" + path]) == row["sha256"], "source copy " + path)

    for check in manifest["checks"]:
        name = check["name"]
        receipt_bytes = records[name + ".json"]
        require(sha(receipt_bytes) == check["receipt_sha256"], "receipt " + name)
        receipt = json.loads(receipt_bytes)
        log = records[name + ".log"]
        require(receipt["exit_code"] == check["exit_code"] == 0, "check " + name)
        require(sha(log) == receipt["log_sha256"], "log " + name)
        groups = re.findall(rb"test result: ok\. (\d+) passed;", log)
        require(sum(map(int, groups)) == check["passed"], "passed " + name)
        require(len(groups) == check["groups"], "groups " + name)

    # These failed producer runs are historical evidence, never a passing gate.
    for name in ["world-lib-001", "world-all-det-002"]:
        receipt = json.loads(records[name + ".json"])
        require(receipt["exit_code"] == 101, "historical failure " + name)
        require(sha(records[name + ".log"]) == receipt["log_sha256"], "historical log " + name)

    by_command = {}
    for row in manifest["binaries"]:
        by_command.setdefault(row["command"], []).append({k: v for k, v in row.items() if k != "command"})
        if args.local_binaries:
            require(sha(Path(row["copy"]).read_bytes()) == row["sha256"], "binary " + row["copy"])
    for name, binaries in by_command.items():
        require(binaries == json.loads(records["executables-" + name + ".json"]), "binary receipt " + name)
        executed = re.findall(r"Running .* \(([^)]+)\)", records[name + ".log"].decode())
        require(executed == [row["source"] for row in binaries], "executed binary list " + name)

    for review in manifest["reviews"]:
        require(sha(records[review["path"]]) == review["sha256"], "review " + review["path"])
    qualifier = "and local executable payloads" if args.local_binaries else "(executable payloads remain local)"
    print(f"Verified {len(rows)} records, {len(manifest['sources'])} source identities, "
          f"{len(manifest['checks'])} passing command receipts {qualifier}. "
          "No fresh test, GPU, device, or performance claim.")


if __name__ == "__main__":
    main()
