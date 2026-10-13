"""Constants and guards for the filter execution namespace 0070-ssao-filter-001.

Standard library only. Reuses tools/common.py (the released build004 revision,
unchanged) by import and overrides only what binds this namespace: its
directory, helper crate, plan, frozen engine commit, grant formats, helper
digest and source-freeze scope. Nothing here builds, renders or touches a GPU.
"""

import hashlib
import json
import sys
from pathlib import Path

NAMESPACE_DIR = Path(__file__).resolve().parent
PACKET_DIR = NAMESPACE_DIR.parent
BASE_TOOLS = PACKET_DIR / "tools"
if str(BASE_TOOLS) not in sys.path:
    sys.path.insert(0, str(BASE_TOOLS))
import common as base  # noqa: E402  (tools/common.py, reused unchanged)
from common import *  # noqa: E402,F401,F403  (shared guards re-exported for runner.py's `c`)
from ledger import Refused  # noqa: E402

NAMESPACE = "0070-ssao-filter-001"
NAMESPACE_REL = f"{base.PACKET_REL}/ssao-filter-001"
REPAIR_REL = f"{base.PACKET_REL}/ssao-repair"
TOOLS = NAMESPACE_DIR
HELPER = NAMESPACE_DIR / "effects"
HELPER_PACKAGE = "incant_0070_ssao_filter_effects"
HELPER_LOCK_REL = f"{NAMESPACE_REL}/effects/Cargo.lock"
PLAN = NAMESPACE_DIR / "plan.json"
ENGINE_COMMIT = None  # build004's d2d2294e does not apply here; see engine_commit()
# Execution code: the reused orchestration, the analyzer and this namespace. None
# of it may be dirty at a freeze; only root-written packet files may be.
CODE_RELS = (base.TOOLS_REL, NAMESPACE_REL, REPAIR_REL)
# Generated, untracked helper locks that the freeze records by hash.
GENERATED = (HELPER_LOCK_REL, base.HELPER_LOCK_REL)
# The released build004 orchestration and analyzer files this namespace runs.
REUSED = ("runner.py", "common.py", "ledger.py", "lifecycle.py", "analyze.py", "ssao_reference.py")
ANALYZER = ("filter/analyze_filter.py", "filter/oracle.py", "filter/controls.py", "filter/cases.json",
            "quality_model.py", "quality_report.py", "sphere_rise_bound.py")


def load_plan():
    return json.loads(PLAN.read_text())


def engine_commit():
    """The frozen engine commit bound by the plan (None until root freezes it)."""
    commit = load_plan().get("engine_commit")
    if not isinstance(commit, str) or len(commit) != 40:
        raise Refused("the plan binds no frozen engine_commit; root freezes it with "
                      "`make_plan.py --engine-commit <40-hex>` after the source freeze")
    return commit


def tool_revision():
    """Exact bytes of the reused orchestration and analyzer files."""
    reused = {f"{base.TOOLS_REL}/{name}": base.sha256_file(BASE_TOOLS / name) for name in REUSED}
    analyzer = {f"{REPAIR_REL}/{name}": base.sha256_file(PACKET_DIR / "ssao-repair" / name) for name in ANALYZER}
    return {"reused_tools": reused, "analyzer": analyzer}


def source_freeze(granted_commit):
    """Exact source identity: HEAD is the granted commit, engine inputs equal the
    plan's frozen engine commit, and execution code is clean."""
    engine = engine_commit()
    head = base.git("rev-parse", "HEAD").strip()
    if head != granted_commit:
        raise Refused(f"HEAD {head} is not the granted commit {granted_commit}")
    tree = base.git("rev-parse", "HEAD^{tree}").strip()
    delta = base.git("diff", "--name-only", engine, head, "--", *base.ENGINE_PATHS).split()
    if delta:
        raise Refused(f"engine inputs differ from the frozen engine commit {engine}: {delta[:20]}")
    entries = base.parse_porcelain_z(base.git("status", "--porcelain=v1", "-z", "--untracked-files=all",
                                              "--ignored=no", binary=True))
    dirty, problems = [], []
    for xy, path, original in entries:
        packet_file = base._inside(path, base.PACKET_REL) and not any(base._inside(path, r) for r in CODE_RELS)
        if original is not None or not (packet_file or path in GENERATED):
            problems.append(f"{xy} {path}" + (f" <- {original}" if original else ""))
            continue
        actual = base.ROOT / path
        dirty.append({"status": xy, "path": path, "sha256": base.sha256_file(actual) if actual.is_file() else None})
    if problems:
        raise Refused(f"unreviewed working-tree changes: {problems[:20]}")
    return {"head": head, "tree": tree, "engine_commit": engine, "engine_paths_equal_frozen": True,
            "dirty_or_untracked": dirty}


def helper_digest():
    """SHA-256 over every byte this namespace executes or binds: its own files
    (minus the generated lock, build outputs and caches), the reused tools/
    orchestration and the analyzer."""
    files = {}
    for path in sorted(p for p in NAMESPACE_DIR.rglob("*") if p.is_file()):
        rel = path.relative_to(base.ROOT).as_posix()
        parts = path.relative_to(NAMESPACE_DIR).parts
        if rel == HELPER_LOCK_REL or "__pycache__" in parts or parts[:2] == ("effects", "target") \
                or parts[0].startswith(".selftest-"):
            continue
        files[rel] = base.sha256_file(path)
    revision = tool_revision()
    files.update(revision["reused_tools"])
    files.update(revision["analyzer"])
    digest = hashlib.sha256()
    for rel in sorted(files):
        digest.update(f"{rel} {files[rel]}\n".encode())
    return digest.hexdigest(), dict(sorted(files.items()))


def load_grant(path, kind, plan):
    """A root-written grant for this namespace only (never a build004 grant)."""
    path = Path(path)
    if not path.is_absolute() or not path.is_file():
        raise Refused(f"{kind} grant {path} is absent; preparation stops here (no polling)")
    raw = path.read_bytes()
    grant = json.loads(raw)
    evidence = Path(grant.get("evidence_dir", ""))
    expected = {"format": f"incant-{NAMESPACE}-{kind}-grant-v1", "packet": base.PACKET,
                "namespace": NAMESPACE, "kind": kind}
    problems = [f"{k}={grant.get(k)!r}" for k, v in expected.items() if grant.get(k) != v]
    if not evidence.is_absolute() or not evidence.is_dir():
        problems.append("evidence_dir must be an existing absolute directory")
    elif base.EVIDENCE_ROOT not in evidence.resolve().parents:
        problems.append(f"evidence_dir must be under {base.EVIDENCE_ROOT}")
    elif evidence.resolve().name != NAMESPACE:
        problems.append(f"evidence_dir must be the {NAMESPACE} namespace directory")
    elif path.resolve().parent != evidence.resolve():
        problems.append("the grant file must live directly in its evidence_dir")
    if base.REQUIRE_EXTERNAL_EVIDENCE and (base.ROOT in evidence.resolve().parents or evidence.resolve() == base.ROOT):
        problems.append("evidence_dir must be outside the worktree")
    if grant.get("plan_sha256") != base.sha256_file(PLAN):
        problems.append("plan_sha256 does not match ssao-filter-001/plan.json")
    if grant.get("helper_digest") != helper_digest()[0]:
        problems.append("helper_digest does not match the namespace, reused tools and analyzer bytes")
    if grant.get("engine_commit") != plan.get("engine_commit"):
        problems.append("engine_commit does not match the plan's frozen engine commit")
    if kind == "build":
        if not Path(grant.get("cargo_target_dir", "")).is_absolute():
            problems.append("cargo_target_dir must be absolute")
        if not isinstance(grant.get("max_jobs"), int) or not 1 <= grant["max_jobs"] <= plan["budgets"]["build_jobs_max"]:
            problems.append("max_jobs must be 1 or 2")
    if kind == "gpu" and grant.get("expected_backend") not in base.BACKENDS:
        problems.append("expected_backend must be Metal, Vulkan or Dx12")
    if problems:
        raise Refused(f"{kind} grant refused: {problems}")
    return grant, base.sha256_bytes(raw)
