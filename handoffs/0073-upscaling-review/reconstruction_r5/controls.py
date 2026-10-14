#!/usr/bin/env python3
"""0073 R5 focused scalar/source controls. Standard library only; no renderer, Cargo,
GPU or image. Each rule is transcribed from the src/ mirrors (checked textually) and has a
mutation that restores the 825ff behaviour and must be detected.

    python3 -I -B handoffs/0073-upscaling-review/reconstruction_r5/controls.py --out .../controls.json
"""
import argparse, cmath, json, math, pathlib, sys
sys.dont_write_bytecode = True
H = pathlib.Path(__file__).resolve().parent
CHECKS = []
# Fixture REVIEW config (upscaling_owner_review.rs 650316a): h 0.9, threshold 4, strength 0.5.
H_W, THRESHOLD, STRENGTH = 0.9, 4.0, 0.5


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def cap(r):
    return H_W / (1 - H_W) * min(r * r, 4.0)


def reaction(history, fresh, lo, hi, old_rule=False):
    """R5: displacement the clamp applies; 825ff: distance to the single fresh estimate."""
    ref = fresh if old_rule else min(max(history, lo), hi)
    return min(max(abs(history - ref) / THRESHOLD, 0.0), 1.0)


def equilibrium(m, r, history, fresh, lo, hi, old_rule=False, frames=400):
    w = 0.0
    for _ in range(frames):
        k = (1 - STRENGTH * reaction(history, fresh, lo, hi, old_rule)) if w > 0 else 1.0
        w = min(min(w, cap(r)) * k + m, cap(r))
    return w


def residual(w, m=1.0, phases=8):
    """Amplitude of a phase-periodic (8-phase) error left by exponential accumulation with
    per-frame fraction a = m/(w+m): |a / (1 - (1-a) e^{i 2pi/8})|."""
    a = m / (w + m)
    return abs(a / (1 - (1 - a) * cmath.exp(2j * math.pi / phases)))


def slack(r, old=False):
    return 1 / 256 if old else r / 16


def coherent(deltas_px, r, old=False):
    return all(abs(d) < slack(r, old) for d in deltas_px)


def lod_bias(n16, old=False, floor=-1.5):
    b = math.log2(n16 / 16)
    return min(b, 0.0) if old else min(max(b, floor), 0.0)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out"); a = ap.parse_args()
    up = (H / "src/crates/incant_render/src/upscaling/resolve.wgsl").read_text()
    own = (H / "src/crates/incant_render/src/upscaling/resolve_owner.wgsl").read_text()
    ref = (H / "src/crates/incant_render/src/upscaling/evidence_reference.rs").read_text()
    host = (H / "src/crates/incant_render/src/upscaling/owner.rs").read_text()
    mot = (H / "src/crates/incant_render/src/motion/retained.rs").read_text()
    jit = (H / "src/crates/incant_render/src/temporal/jitter.rs").read_text()
    # Source bindings: the transcribed rules are the ones in the mirrors.
    for name, s in (("UP", up), ("OWN", own)):
        check(f"{name}: reaction from clamp displacement, clamped history blended",
              "let clamped=clamp(history,minimum,maximum);\n    let change=abs(history.rgb-clamped.rgb);" in s
              and "    history=clamped;\n" in s and "abs(history.rgb-fresh.rgb)" not in s)
        check(f"{name}: coherence slack is one jitter-lattice step r/16 output px",
              "fn motion_slack()->f32 {return frame.output.z/frame.source.z/16.0;}" in s
              and "vec2f(motion_slack())" in s and "MOTION_SLACK" not in s)
    check("OWN static proof unchanged (stationary OWN quality path keeps its 1/256 anchor test, no clamp)",
          "all(abs(old-p)<vec2f(1.0/256.0))" in own and "let w=min(s.x,evidence_cap());" in own)
    check("cfg(test) evidence reference mirrors both shader changes",
          ".zip(&clamped)" in ref and "output[2] / f64::from(self.frame.source[2]) / 16." in ref and "1. / 256." in ref)
    check("material and MASK use the same floored bias", "pub(crate) const MIN_LOD_BIAS: f32 = -1.5;" in jit
          and "b.clamp(crate::temporal::jitter::MIN_LOD_BIAS, 0.)" in host and "b.clamp(crate::temporal::jitter::MIN_LOD_BIAS, 0.)" in mot)
    # 1. Reaction: high-contrast texel whose history lies inside the current support range.
    r, m = 2.0, 1.0
    w_new = equilibrium(m, r, history=0.5, fresh=1.0, lo=0.0, hi=1.0)
    w_old = equilibrium(m, r, history=0.5, fresh=1.0, lo=0.0, hi=1.0, old_rule=True)
    check("stripes/checker texel (history inside range): R5 mass reaches the cap 36", abs(w_new - 36.0) < 1e-9, w_new)
    check("mutation: 825ff rule at |H - fresh| = 0.5 holds mass at 16, not 36 (detected)", abs(w_old - 16.0) < 1e-6, w_old)
    w_full = equilibrium(m, r, history=0.0, fresh=1.0, lo=0.0, hi=1.0, old_rule=True)
    check("825ff rule at full contrast |H - fresh| = 1 holds mass at 8 (one jitter cycle)", abs(w_full - 8.0) < 1e-6, w_full)
    check("8-phase aliasing residual amplitude: R5 0.036 vs 825ff 0.079 (contrast 0.5) and 0.152 (contrast 1)",
          residual(w_new) < 0.04 and residual(w_old) > 2 * residual(w_new) and residual(w_full) > 4 * residual(w_new),
          [round(residual(w_new), 4), round(residual(w_old), 4), round(residual(w_full), 4)])
    lo, hi = 0.0, 0.2
    rx = reaction(1.0, 0.1, lo, hi)
    check("true change (history 1.0 outside current range [0, 0.2]) still reacts: reaction 0.2, clamp to 0.2",
          abs(rx - 0.2) < 1e-12 and min(max(1.0, lo), hi) == 0.2, rx)
    check("reaction 0 cannot keep out-of-range stale colour: the clamp bounds it every frame",
          min(max(5.0, 0.0), 0.3) == 0.3)
    # 2. Coherence slack.
    dolly = [0.0, 0.02, 0.04, 0.03, 0.01, 0.0, 0.02, 0.04, 0.03]       # output px, perspective spread
    check("perspective dolly spread 0.04 px at r=2 is admitted (slack 0.125 px)", coherent(dolly, 2.0))
    check("mutation: 1/256 px refuses that same motion (detected)", not coherent(dolly, 2.0, old=True))
    check("independent coplanar mover (0.1 UV x 1920 = 192 px) still refused", not coherent([0.0] * 8 + [0.1 * 1920], 2.0))
    check("boundary: a difference equal to the slack is refused (>=)", not coherent([0.125], 2.0) and coherent([0.1249], 2.0))
    check("slack scales with the ratio: 1/16 px at r=1, 1/4 px at r=4", slack(1.0) == 0.0625 and slack(4.0) == 0.25)
    # 3. LOD floor.
    check("bias -1.5 at 4/16 (was -2) and 5/16 (was -1.678)", lod_bias(4) == -1.5 and lod_bias(5) == -1.5
          and lod_bias(4, old=True) == -2.0 and abs(lod_bias(5, old=True) + 1.678) < 1e-3)
    check("bias unchanged at 6/16..16/16 (8/16 stays -1, 16/16 stays 0)",
          all(lod_bias(n) == lod_bias(n, old=True) for n in range(6, 17)) and lod_bias(8) == -1.0 and lod_bias(16) == 0.0)
    check("floor derivation: -log2(sqrt(8 samples per texel)) = -1.5", abs(-math.log2(math.sqrt(8)) + 1.5) < 1e-12)
    out = {"scope": "scalar transcriptions of the R5 src mirrors; not a render, compile or appearance claim",
           "config": {"history_weight": H_W, "reactive_threshold": THRESHOLD, "reactive_strength": STRENGTH},
           "checks": CHECKS, "all_checks_pass": all(c["ok"] for c in CHECKS)}
    if a.out:
        pathlib.Path(a.out).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    failed = [c for c in CHECKS if not c["ok"]]
    print(f"checks={len(CHECKS)} failed={len(failed)}")
    for c in failed:
        print("FAILED", c)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
