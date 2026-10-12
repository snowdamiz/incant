# 0059 — Actual Linux UI numerical mismatch

Claude Opus 5.5 over ACP, thinking explicitly Max, verified before prompting.
You own shaders, rendering references and rendered review. Read CLAUDE.md.
Repair the actual remaining checkpoint12 Linux UI failure without weakening
meaningful rendering coverage or expanding unrelated UI work.

## Authoritative failure

Frozen candidate10129769b82a5a6dd7b075f0d8f42bfcc33e9ae5 equals main150
58cca708545eb494868e18469fd4de98f47eb2da. The starting source here is main151;
the five source hashes recorded in evidence/checkpoint12-linux-failure.json must
still match before diagnosis. The root-motion/docs addition is unrelated.

Read the retained exact raw CI log (gzip) and compact receipt in this packet.
The actual adapter now reports llvmpipe LLVM20.1.2,256bits; Vulkan;
Mesa25.2.8-0ubuntu0.24.04.4. Do not infer adapter identity from installed packages.
The final UI GPU subset passes31 cases and fails
logical_layout_maps_to_physical_pixels_at_integer_and_fractional_scales:
scale1 Rgba8Unorm, outside245/excess2/widest2. sRGB scale1/2/1.5 checks
preceding that point pass; previous region, nested-clip, retained-buffer and
UI-over-world cases pass. Prior0057 interval correction is already in this base.
This is an observed failure, not a request to raise an arbitrary tolerance.

## Scope and acceptance

Inspect ui_draw_tests.rs, ui/ui.wgsl, ui/geometry.rs and their sampler/blending
contract. Explain the discrepancy from actual shader arithmetic/UNORM conversion,
filtering or rasterization rules, supported by retained diagnostic samples and
bounded numerical cases. Establish whether the implementation or the independent
reference is wrong, and fix that side. A reference adjustment must follow a
specified backend precision bound and still reject the known meaningful wrong
layout/clip/color/coverage variants; no golden snapshot substitution or broad
unexplained channel tolerance. Preserve the zero-coverage discard contract and
exact pixels where exact output is required. Avoid unrelated app restyling.

Run the focused UI GPU tests on the actual local adapter and relevant CPU cases,
then fmt/Clippy/wasm32 compile. Capture actual adapter identities and qualified
results. If a Linux software-Vulkan environment is already available locally,
use it to reproduce, but do not install a VM or assume a Mac run proves Linux.
Astra's integration owner will run the final real Linux checkpoint. Leave that
hosted confirmation explicitly pending unless you actually have its results.
Do not change CI requirements, trigger remote workflows, push or merge.

Use locked Rust1.99.0, ci profile, at most2 jobs, exclusive warm target
/Users/andreyyurlov/.codex/incant-native-target (previous0058 owner is released).
No other agent may use that target while this handoff is live. Retain source,
commands, full logs and exact before/after output hashes under this packet or
/Users/andreyyurlov/.codex/incant-evidence/0059. Never put irreplaceable evidence
only in temporary build directories. End/reap owned processes before returning.
No window/screen capture is needed to diagnose this numerical test. If a visual
regression requires native interaction, use only short bursts after20seconds of
real user idle, check foreground ownership, yield on input and restore focus;
window-only captures, fresh lease/expiry/release marker, no desktop screenshots.

## Deliverable

Commit the focused fix, regression/negative controls and evidence with
Built-by: claude. Preserve this brief. Write result.md with explicit model/effort,
root cause, exact source/log identities, actual adapter results, performance
impact, limitations and all owned process handles reaped. Do not claim a complete
Game UI/device/phase gate. Root will independently review and integrate.
