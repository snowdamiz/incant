#!/usr/bin/env python3
"""0073 r4 compile fix: rename the WGSL-reserved helper `coherent` to
`motion_support_coherent` in the R4 resolve.wgsl and resolve_owner.wgsl copies
(definition and single call each), update only the Upscale and UpscaleOwner
combined trusted hashes in the R4 temporal/mod.rs copy, write corrected copies
and an exact patch against the R4 copies, and verify. Standard library and
read-only git only; run from the child worktree:

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_r4_compile_fix/compile_fix.py \\
        --base /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant \\
        --naga-keywords <cargo registry>/naga-29.0.4/src/keywords/wgsl.rs
"""
import argparse, difflib, hashlib, json, pathlib, re, shutil, subprocess, sys, tempfile
sys.dont_write_bytecode = True
BASE = "76be9608e99bf089fcb1e0908827330f70c28ac1"
HERE = pathlib.Path(__file__).resolve().parent
R4 = HERE.parent / "reconstruction_implementation_r4" / "src"
SHADERS = {"Upscale": "crates/incant_render/src/upscaling/resolve.wgsl",
           "UpscaleOwner": "crates/incant_render/src/upscaling/resolve_owner.wgsl"}
MOD = "crates/incant_shader_cook/src/temporal/mod.rs"
OLD, NEW = "coherent", "motion_support_coherent"
WORD = re.compile(r"\bcoherent\b")


def arr(d):
    v = list(d)
    return "[\n                " + ", ".join(map(str, v[:16])) + ",\n                " + ", ".join(map(str, v[16:])) + ",\n            ]"


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--base", required=True); ap.add_argument("--naga-keywords")
    a = ap.parse_args()
    common = subprocess.run(["git", "-C", a.base, "show", f"{BASE}:crates/incant_render/src/upscaling/common.wgsl"],
                            check=True, capture_output=True).stdout.decode()
    files, rows, checks = {}, {}, {}
    mod_old = (R4 / MOD).read_text(); mod_new = mod_old
    for kind, rel in SHADERS.items():
        old = (R4 / rel).read_text()
        assert NEW not in old
        sites = [m.start() for m in WORD.finditer(old)]
        new = WORD.sub(NEW, old)
        lines = [l for l in old.splitlines() if WORD.search(l)]
        h_old = hashlib.sha256((common + "\n" + old).encode()).digest()
        h_new = hashlib.sha256((common + "\n" + new).encode()).digest()
        pin_old = f"            Self::{kind} => " + arr(h_old) + ","
        assert mod_new.count(pin_old) == 1, kind
        mod_new = mod_new.replace(pin_old, f"            Self::{kind} => " + arr(h_new) + ",")
        files[rel] = (old, new)
        checks[kind] = {
            "occurrences_renamed": len(sites),
            "renamed_lines": lines,
            "reverse_substitution_recovers_r4_bytes": new.replace(NEW, OLD) == old,
            "only_identifier_changed": [l for l in difflib.ndiff(old.splitlines(), new.splitlines()) if l[:1] in "+-"],
        }
        rows[kind] = {"r4_file_sha256": hashlib.sha256(old.encode()).hexdigest(),
                      "fixed_file_sha256": hashlib.sha256(new.encode()).hexdigest(),
                      "r4_combined_pin": h_old.hex(), "fixed_combined_pin": h_new.hex()}
    files[MOD] = (mod_old, mod_new)
    changed_mod = [l for l in difflib.ndiff(mod_old.splitlines(), mod_new.splitlines()) if l[:1] in "+-"]
    # Only the four byte lines of the two pins may differ.
    assert len(changed_mod) == 8 and all(re.fullmatch(r"[+-] {16}[\d, ]+,", l) for l in changed_mod), changed_mod
    for rel, (_, new) in files.items():
        out = HERE / "src" / rel; out.parent.mkdir(parents=True, exist_ok=True); out.write_text(new)
    patch = "".join(
        f"diff --git a/{rel} b/{rel}\n" + "".join(difflib.unified_diff(
            old.splitlines(True), new.splitlines(True), f"a/{rel}", f"b/{rel}", n=3))
        for rel, (old, new) in files.items())
    (HERE / "compile_fix.patch").write_text(patch)
    with tempfile.TemporaryDirectory(prefix="incant-0073-r4-compile-fix-", dir="/tmp") as tmp:
        for rel in files:
            dst = pathlib.Path(tmp) / rel; dst.parent.mkdir(parents=True, exist_ok=True); shutil.copy(R4 / rel, dst)
        chk = subprocess.run(["git", "apply", "--check", str(HERE / "compile_fix.patch")], cwd=tmp, capture_output=True, text=True)
        app = subprocess.run(["git", "apply", str(HERE / "compile_fix.patch")], cwd=tmp, capture_output=True, text=True)
        applied_equal = all((pathlib.Path(tmp) / rel).read_text() == new for rel, (_, new) in files.items())
    reserved = []
    if a.naga_keywords:
        words = set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', pathlib.Path(a.naga_keywords).read_text()))
        def declared(src):
            code = re.sub(r"//[^\n]*", "", src)
            decl = set(re.findall(r"\b(?:fn|let|var|const|struct)\s+([A-Za-z_]\w*)", code))
            return decl | set(re.findall(r"([A-Za-z_]\w*)\s*:", code))
        for rel, (old_src, _) in files.items():
            if rel.endswith(".wgsl"):
                # Negative control: the same scan on the R4 bytes must find exactly `coherent`.
                assert sorted(declared(old_src) & words) == [OLD], sorted(declared(old_src) & words)
        for rel, (_, new) in files.items():
            if rel.endswith(".wgsl"):
                code = re.sub(r"//[^\n]*", "", new)
                ident = set(re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", code))
                decl = set(re.findall(r"\b(?:fn|let|var|const|struct)\s+([A-Za-z_]\w*)", code))
                decl |= set(re.findall(r"([A-Za-z_]\w*)\s*:", code))
                hits = sorted((decl & words) - {"let", "var", "fn", "const", "struct", "return", "if", "else", "for", "continue", "break", "true", "false"})
                reserved.append({"file": rel, "declared_names_in_naga_wgsl_keyword_file": hits,
                                 "negative_control_r4_scan_finds_only": [OLD],
                                 "naga_keyword_count": len(words), "identifier_count": len(ident)})
    result = {
        "base_production": BASE, "source": "reconstruction_implementation_r4 copies at dc7701b2",
        "rename": {"from": OLD, "to": NEW, "reason": "naga: name `coherent` is a reserved keyword"},
        "programs": rows, "checks": checks,
        "mod_rs_changed_lines": changed_mod,
        "patch_sha256": hashlib.sha256(patch.encode()).hexdigest(),
        "patch_files": list(files),
        "git_apply_check_against_r4_copies": chk.returncode == 0, "git_apply_stderr": chk.stderr,
        "applied_bytes_equal_fixed_copies": app.returncode == 0 and applied_equal,
        "reserved_scan": reserved,
        "scope": "identifier rename and two trusted hashes only; no build, naga, cook, GPU or image",
    }
    ok = (all(c["occurrences_renamed"] == 2 and c["reverse_substitution_recovers_r4_bytes"] for c in checks.values())
          and result["git_apply_check_against_r4_copies"] and result["applied_bytes_equal_fixed_copies"]
          and all(not r["declared_names_in_naga_wgsl_keyword_file"] for r in reserved))
    result["all_checks_pass"] = ok
    (HERE / "result.json").write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    for k, v in rows.items():
        print(k, v["r4_combined_pin"][:16], "->", v["fixed_combined_pin"][:16])
    print("patch", result["patch_sha256"], "apply", result["git_apply_check_against_r4_copies"], "equal", result["applied_bytes_equal_fixed_copies"])
    print("reserved scan", [(r["file"].split("/")[-1], r["declared_names_in_naga_wgsl_keyword_file"]) for r in reserved])
    print("all_checks_pass", ok)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
