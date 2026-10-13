#!/usr/bin/env python3
"""CPU-only controls for build.py's root-granted source pinning. Standard library only.

A throwaway git repository in a temporary directory (git run with no global or
system configuration, signing or hooks) stands in for the repository, so the
real build.checkout() runs end to end: build grant validation, lease gate,
commit-object check, freeze SHA-256 and head, `git archive` export with blob
verification, and exact build-input coverage. Only the visual-tree identity is a
test double. No Cargo, compiler or GPU process runs, and nothing outside the
temporary directory is touched.

  python3 -I -B test_source.py [--receipt NEW_PATH]
"""

import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import ExitStack, redirect_stdout
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402
import common as c  # noqa: E402

RECORDS = []
GIT_ENV = {"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
           "GIT_AUTHOR_NAME": "synthetic", "GIT_AUTHOR_EMAIL": "synthetic@invalid",
           "GIT_COMMITTER_NAME": "synthetic", "GIT_COMMITTER_EMAIL": "synthetic@invalid"}
FILES = {"Cargo.toml": "[workspace]\n", "Cargo.lock": "# lock\n", "rust-toolchain.toml": "[toolchain]\n",
         "tools/cargo": "#!/bin/sh\nexec cargo \"$@\"\n", "tools/probe.py": "print(1)\n",
         "crates/a/src/lib.rs": "pub fn a() {}\n", "crates/a/Cargo.toml": "[package]\n",
         "third_party/x/LICENSE": "MIT\n", "docs/note.md": "reviewed doc\n"}


def git(repo, *args):
    env = {**os.environ, **GIT_ENV}
    return subprocess.run(["git", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", *args],
                          cwd=repo, env=env, capture_output=True, text=True, check=True).stdout.strip()


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Source(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix="incant-0073-source-repo-")
        cls.repo = Path(cls.directory.name) / "repo"
        cls.repo.mkdir()
        git(cls.repo, "init", "-q")
        for path, text in FILES.items():
            target = cls.repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
        (cls.repo / "tools/cargo").chmod(0o755)
        git(cls.repo, "add", "-A")
        git(cls.repo, "commit", "-q", "-m", "synthetic source")
        cls.commit = git(cls.repo, "rev-parse", "HEAD")
        cls.tree = git(cls.repo, "rev-parse", "HEAD^{tree}")

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="incant-0073-source-")
        root = Path(self.temp.name)
        e = root / "evidence"
        e.mkdir()
        self.paths = {"ROOT": self.repo, "EVIDENCE": e, "GRANT": e / "review-ready.json",
                      "BUILD_GRANT": e / "build-ready.json", "LEASE": e / "target-lease.json",
                      "LATCH": e / "gpu-released.json", "STOP": e / "stop.json",
                      "REFUSALS": e / "gpu-refusals.jsonl", "LOCK": e / "gpu.lock",
                      "CHECKOUT": e / "checkout", "BUILD": e / "build"}
        self.freeze_path = root / "freeze.json"
        self.stack = ExitStack()
        self.stack.enter_context(mock.patch.multiple(c, **self.paths))
        self.stack.enter_context(mock.patch.object(
            c, "tools_identity", side_effect=lambda tree: {"visual_tree": tree, "test_double": True}))
        self.stack.enter_context(redirect_stdout(io.StringIO()))

    def tearDown(self):
        self.stack.close()
        self.temp.cleanup()

    def rows(self):
        return [{"path": p, "sha256": sha(t.encode()), "bytes": len(t.encode())} for p, t in FILES.items()]

    def grant(self, freeze, commit=None, freeze_sha=None):
        self.freeze_path.write_text(json.dumps(freeze))
        body = {"packet": c.PACKET, "stage": c.BUILD_STAGE, "source_commit": commit or self.commit,
                "source_freeze": {"path": str(self.freeze_path),
                                  "sha256": freeze_sha or sha(self.freeze_path.read_bytes())},
                "visual_tree": "a" * 40, "target": str(c.CODEX / "synthetic-0073-target")}
        self.paths["BUILD_GRANT"].write_text(json.dumps(body))
        self.paths["LEASE"].write_text(json.dumps(
            {"grant_sha256": sha(self.paths["BUILD_GRANT"].read_bytes())}))

    def refused(self, test, pattern):
        with self.assertRaisesRegex(SystemExit, pattern):
            build.checkout()
        self.assertFalse(self.paths["CHECKOUT"].exists())
        RECORDS.append({"test": test, "refused_before_export": True, "matched": pattern})

    def stopped(self, test, field):
        with self.assertRaisesRegex(SystemExit, "stop latched"):
            build.checkout()
        record = c.read_json(self.paths["BUILD"] / "source.json")
        self.assertFalse(record["coverage"]["exact"])
        self.assertTrue(record["coverage"][field])
        self.assertTrue(self.paths["STOP"].exists())
        RECORDS.append({"test": test, "stop_latched": True, field: record["coverage"][field]})

    def test_exact_committed_freeze_exports(self):
        self.grant({"head": self.commit, "files": self.rows()})
        build.checkout()
        record = c.read_json(self.paths["BUILD"] / "source.json")
        self.assertTrue(record["coverage"]["exact"])
        self.assertEqual(record["coverage"]["build_inputs"], 7)
        self.assertFalse(self.paths["STOP"].exists())
        self.assertTrue(os.access(self.paths["CHECKOUT"] / "tools/cargo", os.X_OK))
        RECORDS.append({"test": "exact_committed_freeze_exports", "coverage": record["coverage"],
                        "tree": record["tree"] == self.tree})

    def test_branch_name_is_not_a_source(self):
        self.grant({"files": self.rows()}, commit="main")
        self.refused("branch_name_refused", "40-hex source commit")

    def test_non_commit_object_is_not_a_source(self):
        self.grant({"files": self.rows()}, commit=self.tree)
        self.refused("tree_object_refused", "is not a commit")

    def test_freeze_hash_must_match_grant(self):
        self.grant({"files": self.rows()}, freeze_sha="0" * 64)
        self.refused("freeze_hash_mismatch_refused", "SHA-256 differs from the grant")

    def test_freeze_head_must_be_granted_commit(self):
        self.grant({"head": "1" * 40, "files": self.rows()})
        self.refused("freeze_head_mismatch_refused", "not the granted commit")

    def test_dirty_file_in_freeze_stops(self):
        rows = self.rows()
        rows[5]["sha256"] = sha(b"working-tree edit\n")  # crates/a/src/lib.rs
        self.grant({"head": self.commit, "files": rows})
        self.stopped("dirty_file_stops", "mismatched")

    def test_uncommitted_freeze_row_stops(self):
        self.grant({"head": self.commit, "files": [*self.rows(), {"path": "docs/untracked.md",
                                                                 "sha256": sha(b"x"), "bytes": 1}]})
        self.stopped("uncommitted_freeze_row_stops", "freeze_rows_missing_from_commit")

    def test_build_input_missing_from_freeze_stops(self):
        self.grant({"head": self.commit, "files": [r for r in self.rows()
                                                   if r["path"] != "third_party/x/LICENSE"]})
        self.stopped("build_input_missing_stops", "inputs_missing_from_freeze")

    def test_size_mismatch_stops(self):
        rows = self.rows()
        rows[0]["bytes"] += 1
        self.grant({"head": self.commit, "files": rows})
        self.stopped("size_mismatch_stops", "mismatched")

    def test_conflicting_duplicate_rows_refuse(self):
        rows = self.rows()
        self.grant({"head": self.commit, "files": rows,
                    "sources": [{**rows[0], "sha256": sha(b"other")}]})
        self.refused("conflicting_duplicate_refused", "two identities")


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Source)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if len(sys.argv) == 3 and sys.argv[1] == "--receipt":
        with open(sys.argv[2], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                       "failures": len(result.failures), "errors": len(result.errors),
                       "python": sys.version.split()[0], "records": RECORDS}, file, indent=2)
            file.write("\n")
    elif len(sys.argv) != 1:
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    raise SystemExit(0 if result.wasSuccessful() else 1)
