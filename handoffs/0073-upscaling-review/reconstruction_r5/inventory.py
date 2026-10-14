#!/usr/bin/env python3
"""0073 R5: build repair.patch (baseline -> src mirrors), check it applies to the exact
825ff baseline copies in /tmp/incant-0073-r5-*, verify mirrors against the baseline
manifest and the candidate Git objects, and record the hash/ABI inventory. Read-only git.

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_r5/inventory.py \\
        --checkout /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant
"""
import argparse, difflib, hashlib, json, pathlib, re, shutil, subprocess, sys, tempfile
sys.dont_write_bytecode = True
H = pathlib.Path(__file__).resolve().parent
SOURCE = "825ff047284e84252d00f786a7b14cb6ecb44293"
sha = lambda b: hashlib.sha256(b).hexdigest()


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--checkout", required=True); a = ap.parse_args()
    manifest = {r["path"]: r["sha256"] for r in json.loads((H / "baseline.json").read_text())["files"]}
    src = sorted(str(p.relative_to(H / "src")) for p in (H / "src").rglob("*") if p.is_file())
    rows, patch = [], ""
    for rel in src:
        base = (H / "baseline" / rel).read_bytes()
        blob = subprocess.run(["git", "-C", a.checkout, "show", f"{SOURCE}:{rel}"], capture_output=True).stdout
        new = (H / "src" / rel).read_bytes()
        assert sha(base) == manifest[rel] == sha(blob), rel           # baseline copy is the exact 825ff object
        rows.append({"path": rel, "baseline_sha256": sha(base), "candidate_sha256": sha(new), "changed": base != new})
        if base != new:
            patch += f"diff --git a/{rel} b/{rel}\n" + "".join(difflib.unified_diff(
                base.decode().splitlines(True), new.decode().splitlines(True), f"a/{rel}", f"b/{rel}", n=3))
    (H / "repair.patch").write_text(patch)
    with tempfile.TemporaryDirectory(prefix="incant-0073-r5-", dir="/tmp") as tmp:
        for rel in src:
            d = pathlib.Path(tmp) / rel; d.parent.mkdir(parents=True, exist_ok=True); shutil.copy(H / "baseline" / rel, d)
        chk = subprocess.run(["git", "apply", "--check", str(H / "repair.patch")], cwd=tmp, capture_output=True, text=True)
        app = subprocess.run(["git", "apply", str(H / "repair.patch")], cwd=tmp, capture_output=True, text=True)
        equal = all((pathlib.Path(tmp) / r).read_bytes() == (H / "src" / r).read_bytes() for r in src)
    common = (H / "baseline/crates/incant_render/src/upscaling/common.wgsl").read_text()
    mod = (H / "src/crates/incant_shader_cook/src/temporal/mod.rs").read_text()
    pins = {}
    for kind, f in (("Upscale", "resolve.wgsl"), ("UpscaleOwner", "resolve_owner.wgsl")):
        m = re.search(rf"Self::{kind} => \[([\d,\s]+)\]", mod)
        pin = bytes(int(x) for x in m.group(1).replace("\n", " ").split(",") if x.strip()).hex()
        want = sha((common + "\n" + (H / "src/crates/incant_render/src/upscaling" / f).read_text()).encode())
        old = sha((common + "\n" + (H / "baseline/crates/incant_render/src/upscaling" / f).read_text()).encode())
        pins[kind] = {"baseline_pin": old, "candidate_pin": pin, "pin_matches_candidate_source": pin == want}
    out = {"source_commit": SOURCE, "production": "72b1ba8ec351e86d66a90c401e0a75a530633616", "files": rows,
           "patch_sha256": sha((H / "repair.patch").read_bytes()), "patch_changed_files": [r["path"] for r in rows if r["changed"]],
           "git_apply_check_against_825ff_copies": chk.returncode == 0, "applied_equals_mirrors": app.returncode == 0 and equal,
           "trusted_pins": pins,
           "abi": {"uniforms": "unchanged (Frame 96 B upscale, 112 B material, 160 B motion)", "bindings": "unchanged",
                   "targets_and_memory": "unchanged; no new texture, pass, pipeline or allocation",
                   "programs_changed": ["Upscale", "UpscaleOwner"], "programs_unchanged": ["Composite", "MetalFxPack", "MetalFxNormalize",
                   "Motion", "Depth", "MotionOwner", "Material", "Resolve", "Copy"],
                   "host_values": "lod bias floor -1.5 for internal material/MASK draws below 0.354 scale"},
           "scope": "source identity and patch applicability only; no build, cook, GPU or image"}
    (H / "inventory.json").write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    ok = out["git_apply_check_against_825ff_copies"] and out["applied_equals_mirrors"] and all(p["pin_matches_candidate_source"] for p in pins.values())
    print(out["patch_sha256"], out["patch_changed_files"], ok)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
