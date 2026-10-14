#!/usr/bin/env python3
"""0073 r4: old/new composed-source sha256 of every cooked temporal program that
includes upscaling sources, and a check that the packet's temporal/mod.rs pins equal
the packet sources. The composition is the cooker's: common.wgsl + "\\n" + program.
Read-only git against exact 76be; run from the child worktree:

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_implementation_r4/inventory.py \\
        --base /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant \\
        --out handoffs/0073-upscaling-review/reconstruction_implementation_r4/inventory.json
"""
import argparse, hashlib, json, pathlib, re, subprocess, sys
sys.dont_write_bytecode = True
BASE = "76be9608e99bf089fcb1e0908827330f70c28ac1"
HERE = pathlib.Path(__file__).resolve().parent / "src"
PROGRAMS = {"Upscale": "upscaling/resolve.wgsl", "UpscaleOwner": "upscaling/resolve_owner.wgsl",
            "Composite": "upscaling/composite.wgsl", "MetalFxPack": "metalfx/prepare.wgsl",
            "MetalFxNormalize": "metalfx/composite.wgsl"}


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--base", required=True); ap.add_argument("--out")
    a = ap.parse_args()
    def old(rel):
        return subprocess.run(["git", "-C", a.base, "show", f"{BASE}:crates/incant_render/src/{rel}"],
                              check=True, capture_output=True).stdout.decode()
    def new(rel):
        p = HERE / "crates/incant_render/src" / rel
        return p.read_text() if p.exists() else old(rel)
    pins = {}
    mod = (HERE / "crates/incant_shader_cook/src/temporal/mod.rs").read_text()
    for name, body in re.findall(r"Self::(\w+) => \[([\d,\s]+)\]", mod):
        pins[name] = bytes(int(x) for x in body.replace("\n", " ").split(",") if x.strip()).hex()
    rows, ok = {}, True
    for name, rel in PROGRAMS.items():
        o = hashlib.sha256((old("upscaling/common.wgsl") + "\n" + old(rel)).encode()).hexdigest()
        n = hashlib.sha256((new("upscaling/common.wgsl") + "\n" + new(rel)).encode()).hexdigest()
        rows[name] = {"old": o, "new": n, "changed": o != n, "pin": pins.get(name), "pin_matches_new": pins.get(name) == n}
        ok = ok and pins.get(name) == n
    out = {"base": BASE, "composition": "common.wgsl + \\n + program (cooker and include_str)",
           "programs": rows, "all_pins_match_packet_sources": ok,
           "scope": "source identity only; no cook, naga, build or GPU"}
    if a.out:
        pathlib.Path(a.out).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    for k, v in rows.items():
        print(k, "changed" if v["changed"] else "unchanged", v["old"][:12], "->", v["new"][:12], "pin ok" if v["pin_matches_new"] else "PIN MISMATCH")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
