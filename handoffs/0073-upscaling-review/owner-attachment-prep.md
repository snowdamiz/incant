# 0073 owner attachment — preparation only

Preserve the existing 0073 design branch, ACP session, reports, and previous grants.
This additive packet is not a GPU grant. Root currently owns scheduling; the 0072
warmup must finish and its actual release be verified before a fresh authority.
Resume Claude Opus 5.5 through ACP only after explicitly verifying Max as well as
the model. No build, render, window, screenshot, image inspection or GPU probe is
authorized by this preparation packet.

The exact producer is `613ee59aec1f2bce080d4a1e335dc45f732b993f` in the private
native-agent-play checkout. Read `docs/spikes/temporal-owner-attachment.md`, its
separate verification note, `crates/incant_render/src/motion_owner_device_tests.rs`,
and the adjacent `owner-attachment-plan.json`. The plan pins the final source,
independent/root reviews, executable, command, environment and expected outcome.
Use immutable copies; do not merge, edit or rebuild renderer source for this run.

Prepare a small supervised runner by reusing the existing reviewed lease/process
helpers where applicable, with a fresh evidence directory and fresh root authority.
Do not reuse any prior consumed/released grant. Root must review the final runner
hash before granting GPU execution. The runner must reject changed source/executable
identities, missing executable bit, altered exact filter, changed budget, malformed
authority and absent predecessor release. Its CPU checks may enumerate tests and
exercise process cleanup with ordinary small processes; they must not execute the
ignored test. No extra dependency installation or Cargo target access is needed.

The proposed GPU command runs **one** exact test from the frozen cooked-temporal,
native-metalfx binary. The fixture does not enable MetalFX or timing. Its six on/off
comparisons are **12 actual renders plus 12 batched readback queue submissions =
24 explicit submissions**, not six frames. Setup uses queue writes; there are no
additional explicit fixture submissions. Maximum output is 32×32; source sizes
are 32 and 16. No image files or pixel/appearance judgment belongs in this run.
The other compiled default-WGSL binary is deliberately outside this budget. Do
not run it too, retry, extend cases, or add readbacks without a new reviewed grant.

Before launching, atomically acquire the global GPU lease and packet lock, verify
the previous holder's release and empty owned process groups, and irreversibly
predebit the full attempt. Launch in a new process group with the plan's minimal
environment, regular retained stdout/stderr files, and a 90-second timeout. Never
override HOME or inherit unrelated INCANT/WGPU/RUST/Cargo flags. On every outcome,
including BaseException, terminate any surviving owned group, wait/reap, preserve
the original exception and output, then record truthful release and PID/PGID state.
Fail closed if cleanup cannot establish no survivors. No untracked background work.

Accept only exit zero, exactly one passed test/zero failed/zero ignored, required
Metal backend with actual adapter identity, and this exact final marker:

```text
MOTION_OWNER renders=12 readback_submissions=12 max_extent=32x32 source_removed=true no_images=true
```

The adapter/start marker alone is insufficient: an unsupported early return has
no final marker and must be reported as failure/refusal, not success. The actual
assertions check all owner texels against analytic coverage, first/unchanged/
deformed transforms and skin palettes, MASK and BLEND exclusions, packing changes,
failed-attempt rollback, exact allocation reuse, and byte-identical motion/depth/
color on/off. Record failure verbatim; do not widen geometry or numeric tolerances.

Return a committed preparation result and exact helper hashes with Built-by:
claude, still no GPU execution. After root separately grants execution, retain raw
command/output/OS/adapter/PID and release receipts. This test would establish only
numeric owner attachment behavior. History-consumer correctness, native owner
packing, temporal appearance, physical GPU memory and performance remain open.
