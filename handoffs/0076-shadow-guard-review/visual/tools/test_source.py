#!/usr/bin/env python3
"""CPU-only controls for build.py's root-granted, per-role source pinning (adapted
from 0072's test_source.py, 48a688c5, itself the accepted 0073 routine). A
throwaway git repository in a temporary directory (no global or system git
configuration, signing or hooks) holds a baseline commit and a candidate child
that changes one Rust file, so the real build.checkout(role) runs end to end for
both roles: build grant validation (each commit must be the plan's), lease gate,
commit-object check, freeze SHA-256 and head, `git archive` export with blob
verification and exact build-input coverage. Only the visual-tree identity and
the plan's two commits are test doubles. No Cargo, compiler or GPU process runs,
and nothing outside the temporary directory is touched.

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
CHANGED, CANDIDATE_TEXT = "crates/a/src/lib.rs", "pub fn a() -> u8 { 1 }\n"
GIT_ENV = {"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1",
           "GIT_AUTHOR_NAME": "synthetic", "GIT_AUTHOR_EMAIL": "synthetic@invalid",
           "GIT_COMMITTER_NAME": "synthetic", "GIT_COMMITTER_EMAIL": "synthetic@invalid"}
FILES = {"Cargo.toml": "[workspace]\n", "Cargo.lock": "# lock\n", "rust-toolchain.toml": "[toolchain]\n",
         ".cargo/config.toml": "# msvc only\n",
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
        cls.directory = tempfile.TemporaryDirectory(prefix="incant-0072-source-repo-")
        cls.repo = Path(cls.directory.name) / "repo"
        cls.repo.mkdir()
        git(cls.repo, "init", "-q")
        for path, text in FILES.items():
            target = cls.repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
        (cls.repo / "tools/cargo").chmod(0o755)
        git(cls.repo, "add", "-A")
        git(cls.repo, "commit", "-q", "-m", "synthetic baseline")
        cls.commits = {"baseline": git(cls.repo, "rev-parse", "HEAD")}
        (cls.repo / CHANGED).write_text(CANDIDATE_TEXT)
        git(cls.repo, "commit", "-q", "-am", "synthetic candidate")
        cls.commits["candidate"] = git(cls.repo, "rev-parse", "HEAD")

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="incant-0076-source-")
        root = Path(self.temp.name)
        e = root / "evidence"
        e.mkdir()
        self.paths = {"ROOT": self.repo, "EVIDENCE": e, "GRANT": e / "review-ready.json",
                      "BUILD_GRANT": e / "build-ready.json", "LEASE": e / "target-lease.json",
                      "LATCH": e / "gpu-released.json", "STOP": e / "stop.json",
                      "REFUSALS": e / "refusals.jsonl", "PACKET_LOCK": e / "packet.lock",
                      "CHECKOUT": e / "checkout", "BUILD": e / "build"}
        self.root = root
        self.stack = ExitStack()
        self.stack.enter_context(mock.patch.multiple(c, **self.paths))
        plan = {"sources": {role: {"commit": commit} for role, commit in self.commits.items()},
                "build_inputs": c.plan()["build_inputs"]}
        self.stack.enter_context(mock.patch.object(c, "plan", return_value=plan))
        self.stack.enter_context(mock.patch.object(
            c, "tools_identity", side_effect=lambda data: {"visual_tree": data["visual_tree"], "test_double": True}))
        self.stack.enter_context(redirect_stdout(io.StringIO()))

    def tearDown(self):
        self.stack.close()
        self.temp.cleanup()

    def rows(self, role):
        files = dict(FILES, **({CHANGED: CANDIDATE_TEXT} if role == "candidate" else {}))
        return [{"path": p, "sha256": sha(t.encode()), "bytes": len(t.encode())} for p, t in files.items()]

    def grant(self, freezes, commits=None, freeze_sha=None):
        """freezes: {role: freeze body}; every role absent from it is omitted from the grant."""
        sources = {}
        for role, freeze in freezes.items():
            path = self.root / f"freeze-{role}.json"
            path.write_text(json.dumps(freeze))
            sources[role] = {"source_commit": (commits or self.commits)[role],
                             "source_freeze": {"path": str(path),
                                               "sha256": (freeze_sha or {}).get(role) or sha(path.read_bytes())}}
        body = {"packet": c.PACKET, "stage": c.BUILD_STAGE, "holder": "synthetic-root", "sources": sources,
                "visual_tree": "a" * 40, "target": str(c.CODEX / "synthetic-0076-target")}
        self.paths["BUILD_GRANT"].write_text(json.dumps(body))
        self.paths["LEASE"].write_text(json.dumps({"grant_sha256": sha(self.paths["BUILD_GRANT"].read_bytes())}))

    def both(self):
        return {role: {"commit": self.commits[role], "files": self.rows(role)} for role in c.ROLES}

    def test_exact_committed_freezes_export_both_roles(self):
        self.grant(self.both())
        for role in c.ROLES:
            build.checkout(role)
        records = {role: c.read_json(c.build_dir(role) / "source.json") for role in c.ROLES}
        for role, record in records.items():
            self.assertTrue(record["coverage"]["exact"])
            self.assertEqual(record["coverage"]["build_inputs"], 8)  # .cargo is a build input
            self.assertEqual((record["role"], record["source_commit"]), (role, self.commits[role]))
            self.assertEqual(record["holder"], "synthetic-root")
        texts = [(c.checkout_dir(role) / CHANGED).read_text() for role in c.ROLES]
        self.assertEqual(texts, [FILES[CHANGED], CANDIDATE_TEXT])  # separate exports
        self.assertFalse(self.paths["STOP"].exists())
        RECORDS.append({"test": "exact_committed_freezes_export_both_roles",
                        "coverage": {role: r["coverage"] for role, r in records.items()}})

    def test_branch_name_is_not_a_source(self):
        self.grant(self.both(), commits=dict(self.commits, baseline="main"))
        with self.assertRaisesRegex(SystemExit, "40-hex"):
            build.checkout("baseline")
        self.assertFalse(self.paths["CHECKOUT"].exists())
        RECORDS.append({"test": "branch_name_refused", "refused_before_export": True})

    def test_commit_other_than_plan_refused(self):
        swapped = {"baseline": self.commits["candidate"], "candidate": self.commits["baseline"]}
        self.grant(self.both(), commits=swapped)
        with self.assertRaisesRegex(SystemExit, "exactly the plan's"):
            build.checkout("candidate")
        self.assertFalse(self.paths["CHECKOUT"].exists())
        RECORDS.append({"test": "commit_other_than_plan_refused", "refused_before_export": True})

    def test_missing_role_refused(self):
        self.grant({"baseline": self.both()["baseline"]})
        with self.assertRaisesRegex(SystemExit, "for baseline and candidate"):
            build.checkout("baseline")
        self.assertFalse(self.paths["CHECKOUT"].exists())
        RECORDS.append({"test": "missing_role_refused", "refused_before_export": True})

    def test_freeze_hash_must_match_grant(self):
        self.grant(self.both(), freeze_sha={"candidate": "0" * 64})
        with self.assertRaisesRegex(SystemExit, "SHA-256 differs"):
            build.checkout("candidate")
        self.assertFalse(self.paths["CHECKOUT"].exists())
        RECORDS.append({"test": "freeze_hash_mismatch_refused", "refused_before_export": True})

    def test_build_input_missing_from_freeze_stops(self):
        freezes = self.both()
        freezes["candidate"]["files"] = [r for r in freezes["candidate"]["files"] if r["path"] != CHANGED]
        self.grant(freezes)
        with self.assertRaisesRegex(SystemExit, "stop latched"):
            build.checkout("candidate")
        record = c.read_json(c.build_dir("candidate") / "source.json")
        self.assertFalse(record["coverage"]["exact"])
        self.assertEqual(record["coverage"]["inputs_missing_from_freeze"], [CHANGED])
        self.assertTrue(self.paths["STOP"].exists())
        RECORDS.append({"test": "build_input_missing_stops", "stop_latched": True})

    def test_baseline_freeze_cannot_pin_the_candidate(self):
        freezes = self.both()
        freezes["candidate"] = {"files": freezes["baseline"]["files"]}  # the candidate's own file differs
        self.grant(freezes)
        with self.assertRaisesRegex(SystemExit, "stop latched"):
            build.checkout("candidate")
        record = c.read_json(c.build_dir("candidate") / "source.json")
        self.assertEqual(record["coverage"]["mismatched"], [CHANGED])
        RECORDS.append({"test": "baseline_freeze_cannot_pin_the_candidate", "mismatched": [CHANGED]})


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and (len(args) != 2 or args[0] != "--receipt"):
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Source)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if args:
        with open(args[1], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                       "records": RECORDS}, file, indent=1)
            file.write("\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
