"""Shared constants and guards for the 0070 combined SSAO+bloom runner.

Standard library only (Python >= 3.11). Nothing here builds, renders or touches a
GPU. Guards refuse explicitly; callers record refusals in the ledger.
"""

import hashlib
import json
import os
import re
import subprocess
import tomllib
from pathlib import Path

from ledger import Refused

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parents[2]
PACKET = "0070-ambient-occlusion"
PACKET_REL = f"handoffs/{PACKET}"
TOOLS_REL = f"{PACKET_REL}/tools"
HELPER = TOOLS / "effects"
HELPER_LOCK_REL = f"{TOOLS_REL}/effects/Cargo.lock"
PLAN = TOOLS / "plan.json"
ENGINE_COMMIT = "b633f92d9d20cd8ed4c328b0b670ab7935797801"
# Build inputs that must be byte-identical to the reviewed engine union.
ENGINE_PATHS = ["crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo",
                "third_party", "tools/cargo"]
EVIDENCE_ROOT = Path("/Users/andreyyurlov/.codex/incant-evidence")
RUSTUP_BIN = "/opt/homebrew/opt/rustup/bin"
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
BACKENDS = ("Metal", "Vulkan", "Dx12")
# Real grants must name evidence outside the worktree. Only the CPU selftest
# harness process clears this, to keep its scratch evidence inside the worktree.
REQUIRE_EXTERNAL_EVIDENCE = True


def shared_gpu_lock_path():
    """One flock(2) file shared by every packet's native GPU runner. Read at
    call time: only the CPU selftest harness points EVIDENCE_ROOT at scratch."""
    return EVIDENCE_ROOT / "native-gpu.lock"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def load_plan():
    return json.loads(PLAN.read_text())


def git(*args, binary=False):
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True,
                            env={"PATH": SYSTEM_PATH, "LC_ALL": "C", "GIT_OPTIONAL_LOCKS": "0"})
    if result.returncode != 0:
        raise Refused(f"git {' '.join(args)} failed: {result.stderr.decode(errors='replace')[-300:]}")
    return result.stdout if binary else result.stdout.decode()


def parse_porcelain_z(data):
    """`git status --porcelain=v1 -z` entries as (XY, path, original-or-None).

    The raw bytes are split on NUL only. Nothing is stripped: the two status
    columns may start with a space (" M path"), and stripping the stream would
    shift the first entry's columns into its path.
    """
    fields = data.split(b"\0")
    if fields and fields[-1] == b"":
        fields.pop()
    entries, i = [], 0
    while i < len(fields):
        field = fields[i]
        if len(field) < 4 or field[2:3] != b" ":
            raise Refused(f"malformed porcelain entry {field[:80]!r}")
        xy, path = field[:2].decode(), field[3:].decode("utf-8", "surrogateescape")
        original = None
        if xy[0] in "RC":
            i += 1
            original = fields[i].decode("utf-8", "surrogateescape")
        entries.append((xy, path, original))
        i += 1
    return entries


def _inside(path, prefix):
    return path == prefix or path.startswith(prefix.rstrip("/") + "/")


def source_freeze(granted_commit):
    """Exact source identity, refusing anything outside the reviewed scope.

    Engine inputs must equal the reviewed union at ENGINE_COMMIT and be clean.
    Dirty or untracked files are allowed only inside the packet directory but
    outside tools/ (root-written brief/receipts) plus the build-generated helper
    Cargo.lock; their actual bytes are hashed into the freeze.
    """
    head = git("rev-parse", "HEAD").strip()
    if head != granted_commit:
        raise Refused(f"HEAD {head} is not the granted commit {granted_commit}")
    tree = git("rev-parse", "HEAD^{tree}").strip()
    engine_delta = git("diff", "--name-only", ENGINE_COMMIT, head, "--", *ENGINE_PATHS).split()
    if engine_delta:
        raise Refused(f"engine inputs differ from reviewed {ENGINE_COMMIT}: {engine_delta[:20]}")
    entries = parse_porcelain_z(git("status", "--porcelain=v1", "-z", "--untracked-files=all",
                                    "--ignored=no", binary=True))
    dirty, problems = [], []
    for xy, path, original in entries:
        allowed = (_inside(path, PACKET_REL) and not _inside(path, TOOLS_REL)) or path == HELPER_LOCK_REL
        if original is not None or not allowed:
            problems.append(f"{xy} {path}" + (f" <- {original}" if original else ""))
            continue
        actual = ROOT / path
        dirty.append({"status": xy, "path": path,
                      "sha256": sha256_file(actual) if actual.is_file() else None})
    if problems:
        raise Refused(f"unreviewed working-tree changes: {problems[:20]}")
    return {"head": head, "tree": tree, "engine_commit": ENGINE_COMMIT,
            "engine_paths_equal_reviewed": True, "dirty_or_untracked": dirty}


def helper_digest():
    """SHA-256 over every helper byte (tools/, excluding the generated lock)."""
    files, digest = {}, hashlib.sha256()
    for path in sorted(p for p in TOOLS.rglob("*") if p.is_file()):
        rel = path.relative_to(ROOT).as_posix()
        parts = path.relative_to(TOOLS).parts
        if rel == HELPER_LOCK_REL or "__pycache__" in parts or parts[:2] == ("effects", "target"):
            continue
        files[rel] = sha256_file(path)
        digest.update(f"{rel} {files[rel]}\n".encode())
    return digest.hexdigest(), files


# ------------------------------------------------------------------ environment

ALWAYS_FORBIDDEN = re.compile(
    r"^(INCANT_|WGPU_|TRACY_|CARGO_FEATURE_|CARGO_BUILD_|CARGO_PROFILE_|CARGO_TARGET_(?!DIR$)|CARGO_ENCODED_|"
    r"RUSTFLAGS$|RUSTDOCFLAGS$|RUSTC$|RUSTC_WRAPPER$|RUSTC_WORKSPACE_WRAPPER$|RUSTUP_TOOLCHAIN$)")


def build_env(target_dir, tmpdir):
    """Explicit build environment: nothing is inherited except HOME, CARGO_HOME
    and RUSTUP_HOME, forwarded unchanged and only when already set."""
    env = {"PATH": f"{RUSTUP_BIN}:{SYSTEM_PATH}", "CARGO_TARGET_DIR": str(target_dir),
           "TMPDIR": str(tmpdir), "CARGO_TERM_COLOR": "never", "CARGO_INCREMENTAL": "0", "LC_ALL": "C"}
    for key in ("HOME", "CARGO_HOME", "RUSTUP_HOME"):
        if key in os.environ:
            env[key] = os.environ[key]
    if "HOME" not in env:
        raise Refused("HOME is required to locate the installed toolchain and is not set")
    return env


def run_env(tmpdir, backend):
    if backend not in BACKENDS:
        raise Refused(f"unknown expected backend {backend!r}")
    return {"PATH": SYSTEM_PATH, "TMPDIR": str(tmpdir), "RUST_BACKTRACE": "1", "LC_ALL": "C",
            "INCANT_EXPECT_GPU_BACKEND": backend}


def recorded_env(env):
    """Receipt form: key names, approved nonsecret values, forwarded keys unvalued."""
    forwarded = {"HOME", "CARGO_HOME", "RUSTUP_HOME"}
    for key in env:
        if key != "INCANT_EXPECT_GPU_BACKEND" and ALWAYS_FORBIDDEN.match(key):
            raise Refused(f"constructed environment contains forbidden key {key}")
    return {"keys": sorted(env),
            "values": {k: v for k, v in sorted(env.items()) if k not in forwarded},
            "forwarded_unchanged": sorted(k for k in env if k in forwarded)}


# ------------------------------------------------------------------ grants

def load_grant(path, kind, plan):
    path = Path(path)
    if not path.is_absolute() or not path.is_file():
        raise Refused(f"{kind} grant {path} is absent; preparation stops here (no polling)")
    raw = path.read_bytes()
    grant = json.loads(raw)
    evidence = Path(grant.get("evidence_dir", ""))
    expected = {"format": f"incant-0070-{kind}-grant-v1", "packet": PACKET, "kind": kind}
    problems = [f"{k}={grant.get(k)!r}" for k, v in expected.items() if grant.get(k) != v]
    if not evidence.is_absolute() or not evidence.is_dir():
        problems.append("evidence_dir must be an existing absolute directory")
    elif EVIDENCE_ROOT not in evidence.resolve().parents:
        problems.append(f"evidence_dir must be under {EVIDENCE_ROOT}")
    elif path.resolve().parent != evidence.resolve():
        problems.append("the grant file must live directly in its evidence_dir")
    if REQUIRE_EXTERNAL_EVIDENCE and (ROOT in evidence.resolve().parents or evidence.resolve() == ROOT):
        problems.append("evidence_dir must be outside the worktree")
    if grant.get("plan_sha256") != sha256_file(PLAN):
        problems.append("plan_sha256 does not match tools/plan.json")
    if grant.get("helper_digest") != helper_digest()[0]:
        problems.append("helper_digest does not match the helper bytes")
    if kind == "build":
        target = Path(grant.get("cargo_target_dir", ""))
        if not target.is_absolute():
            problems.append("cargo_target_dir must be absolute")
        if not isinstance(grant.get("max_jobs"), int) or not 1 <= grant["max_jobs"] <= plan["budgets"]["build_jobs_max"]:
            problems.append("max_jobs must be 1 or 2")
    if kind == "gpu" and grant.get("expected_backend") not in BACKENDS:
        problems.append("expected_backend must be Metal, Vulkan or Dx12")
    if problems:
        raise Refused(f"{kind} grant refused: {problems}")
    return grant, sha256_bytes(raw)


# ------------------------------------------------------------------ build inputs

OVERRIDE_TABLES = ("build", "target", "env", "source", "patch", "profile", "unstable", "registries",
                   "registry", "net", "http", "alias", "host", "doc", "cargo-new", "term")
REVIEWED_REPO_CONFIG_KEYS = ["target.cfg(all(windows, target_env = \"msvc\")).rustdocflags",
                             "target.cfg(all(windows, target_env = \"msvc\")).rustflags"]


def _key_paths(table, prefix=""):
    out = []
    for key, value in table.items():
        name = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            out.extend(_key_paths(value, name))
        else:
            out.append(name)
    return out


def cargo_config_audit(env, plan, approved):
    """Every Cargo config file a build from ROOT would read, by hash and key
    names only (values are never recorded). Unreviewed files refuse."""
    candidates = []
    for directory in [ROOT, *ROOT.parents]:
        for name in ("config.toml", "config"):
            candidates.append(directory / ".cargo" / name)
    cargo_home = Path(env["CARGO_HOME"]) if "CARGO_HOME" in env else Path(env["HOME"]) / ".cargo"
    for name in ("config.toml", "config"):
        candidates.append(cargo_home / name)
    audit, problems = [], []
    for path in candidates:
        if not path.is_file():
            continue
        digest = sha256_file(path)
        try:
            keys = sorted(_key_paths(tomllib.loads(path.read_text())))
        except (tomllib.TOMLDecodeError, UnicodeDecodeError):
            keys = ["<unparseable>"]
        if path == ROOT / ".cargo" / "config.toml":
            ok = digest == plan["reviewed_repo_cargo_config_sha256"] and keys == REVIEWED_REPO_CONFIG_KEYS
        else:
            ok = approved.get(str(path)) == digest
        audit.append({"path": str(path), "sha256": digest, "keys": keys, "approved": ok})
        if not ok:
            problems.append(str(path))
    if problems:
        raise Refused(f"unapproved Cargo configuration (root must review by hash): {problems}")
    return audit


def _run_text(argv, env, cwd=ROOT):
    result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True)
    if result.returncode != 0:
        raise Refused(f"{argv[0]} failed: {result.stderr[-300:]}")
    return result.stdout.strip()


def compiler_inputs(env):
    """Hashes of the actual compiler binaries and libraries the build will use."""
    record = {"rustup_proxies": {}, "tools": {}, "sysroot_libraries": {}}
    for name in ("rustup", "cargo", "rustc"):
        proxy = Path(RUSTUP_BIN) / name
        if proxy.exists():
            real = proxy.resolve()
            record["rustup_proxies"][name] = {"path": str(proxy), "resolved": str(real),
                                              "sha256": sha256_file(real)}
    for name in ("rustc", "cargo", "clippy-driver", "cargo-clippy", "rustfmt"):
        path = Path(_run_text([f"{RUSTUP_BIN}/rustup", "which", name], env))
        record["tools"][name] = {"path": str(path), "sha256": sha256_file(path),
                                 "bytes": path.stat().st_size}
    sysroot = Path(_run_text([str(Path(record["tools"]["rustc"]["path"])), "--print", "sysroot"], env))
    host = re.search(r"^host: (\S+)$", _run_text([record["tools"]["rustc"]["path"], "-vV"], env), re.M).group(1)
    libraries = sorted((sysroot / "lib").glob("*.dylib")) + sorted((sysroot / "lib").glob("*.so"))
    std = sorted((sysroot / "lib" / "rustlib" / host / "lib").glob("*"))
    digest = hashlib.sha256()
    for path in libraries + std:
        if path.is_file():
            entry = sha256_file(path)
            record["sysroot_libraries"][str(path.relative_to(sysroot))] = entry
            digest.update(f"{path.relative_to(sysroot)} {entry}\n".encode())
    record["sysroot"] = str(sysroot)
    record["host"] = host
    record["sysroot_digest"] = digest.hexdigest()
    record["rustc_vV"] = _run_text([record["tools"]["rustc"]["path"], "-vV"], env)
    record["cargo_V"] = _run_text([f"{RUSTUP_BIN}/cargo", "-V"], env)
    # A rustup directory override outranks rust-toolchain.toml; refuse any other release.
    pinned = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    release = re.search(r"^release: (\S+)$", record["rustc_vV"], re.M)
    if release is None or release.group(1) != pinned:
        raise Refused(f"rustc is not the pinned toolchain {pinned}: {record['rustc_vV'][:200]!r}")
    record["pinned_channel"] = pinned
    return record


def host_facts():
    def run(*argv):
        return subprocess.run(list(argv), capture_output=True, text=True,
                              env={"PATH": SYSTEM_PATH, "LC_ALL": "C"}).stdout.strip()
    return {"sw_vers": run("sw_vers"), "uname_m": run("uname", "-m"),
            "hw_model": run("sysctl", "-n", "hw.model"),
            "cpu": run("sysctl", "-n", "machdep.cpu.brand_string")}
