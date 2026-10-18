"""Replay owned numerical buffers only; never execute archived source/binaries."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import tempfile

import dependency_data


def verify():
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / "manifest.json").read_text())
    data = (here / "records.json.gz").read_bytes()
    assert len(data) == manifest["archive"]["bytes"] < 1024 * 1024
    assert hashlib.sha256(data).hexdigest() == manifest["archive"]["sha256"]
    for name, digest in manifest["verifiers"].items():
        assert hashlib.sha256((here / name).read_bytes()).hexdigest() == digest
    with gzip.open(here / "records.json.gz", "rb") as f:
        raw = f.read(12 * 1024 * 1024 + 1)
    assert len(raw) == manifest["archive"]["uncompressed_bytes"] <= 12 * 1024 * 1024
    rows = json.loads(raw)
    assert [{k: v for k, v in row.items() if k != "base64"} for row in rows] == manifest["records"]
    assert len({row["path"] for row in rows}) == len(rows)
    with tempfile.TemporaryDirectory(prefix="incant-samplex-data-") as temporary:
        root = Path(temporary)
        for row in rows:
            path = Path(row["path"])
            assert not path.is_absolute() and ".." not in path.parts
            value = base64.b64decode(row["base64"], validate=True)
            assert len(value) == row["bytes"] and hashlib.sha256(value).hexdigest() == row["sha256"]
            p = root / path
            p.parent.mkdir(parents=True, exist_ok=True)
            with p.open("xb") as f:
                f.write(value)
        result = dependency_data.validate(root / "capture", root / "old", root / "prior")
        recorded = json.loads((root / "receipts/verified-owner-capture.json").read_text())
        assert {k: v for k, v in result.items() if k != "records"} == {k: v for k, v in recorded.items() if k != "records"}
        for row in recorded["records"]:
            path = "capture/" + row["path"].split("/capture/", 1)[1]
            value = (root / path).read_bytes()
            assert len(value) == row["bytes"] and hashlib.sha256(value).hexdigest() == row["sha256"]
        plan = json.loads((root / "receipts/plan.json").read_text())
        close = json.loads((root / "receipts/closeout.json").read_text())
        assert plan["head"] == close["source"] == manifest["source"]
        assert plan["artifact"] == close["artifact"] == manifest["artifact"]
        pids = set()
        for name in ["visibility", "data"]:
            r = json.loads((root / f"receipts/{name}-result.json").read_text())
            t = json.loads((root / f"receipts/{name}-terminal.json").read_text())
            w = json.loads((root / f"receipts/{name}-wrapper.json").read_text())
            assert r["exit"] == t["actual_tool_terminal_exit"] == 0 and r["error"] is None
            assert r["stage_completed_successfully"] is True
            assert r["source"] == manifest["source"] and r["artifact"] == manifest["artifact"]
            assert t["result_sha256"] == hashlib.sha256((root / f"receipts/{name}-result.json").read_bytes()).hexdigest()
            assert r["log_sha256"] == hashlib.sha256((root / f"receipts/{name}.log").read_bytes()).hexdigest()
            owned = {w["pid"], *r["sampled_owned_pids"], *(x["leader_pid"] for x in r["owned_cleanup_records"])}
            assert owned and all(type(pid) is int and pid > 0 for pid in owned)
            assert {x["pid"] for x in t["observed_absence"]} == owned
            assert all(x["pid_absent"] is True and x["process_group_absent"] is True for x in t["observed_absence"])
            pids |= owned
        assert {x["pid"] for x in close["observed_absence"]} == pids
        assert all(x["pid_absent"] is True and x["process_group_absent"] is True for x in close["observed_absence"])
        assert close["capture_and_data_complete"] is True and close["visibility_accepted"] is False
        return {"records": len(rows), "arms": result["arms"], "differences": result["field_differences_from_baseline"], "owned_identities": len(pids), "native_rerun": False}


if __name__ == "__main__":
    print(json.dumps(verify(), indent=2))
