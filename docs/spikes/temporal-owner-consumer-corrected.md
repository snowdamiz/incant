# Corrected private owner-history consumer

The first bounded numeric GPU run remains failed: on Apple M5 Pro Metal,
case 3 read back about 0.405 where current reconstruction should have been near
zero. It stopped before completing the suite, and the cooked binary never ran.
The old result and frozen inputs are retained unchanged.

Claude Opus 5.5 with explicit Max effort reproduced two possible mechanisms on
CPU: a tiny positive reconstruction weight admitting a changed neighboring
owner, or a misread changed-owner table. The observed output alone does not
establish which mechanism the GPU followed. Independently, the old admission
rule allowed stale history at arbitrarily small positive support, and even a
large-support control demonstrated the same ownership problem.

Claude's reviewed correction permits a changed owner only when it is the anchor
whose motion supplies the reprojection. A changed secondary contributor rejects
the history tap. Coordinates, thresholds, overflow rules, resources and the
ordinary fallback shader remain unchanged. The cooker pins the corrected source.
The diagnosis report's `8.5 + 2^-20` is source position; the corresponding
reconstruction coordinate after subtracting one half is `8 + 2^-20`. Its scalar
cases 7 and 8 check invariance under this patch, not a full emulation of the
separate ordinary fallback shader.

The revised case 3 uses a history-disabled owner resolve in the same encoder.
Every color, depth and provenance byte must match this fresh reference. A
separate center bound permits at most `2^-20` color, and a distant zero-support
probe checks changed-table rejection. This supersedes the diagnosis's proposed
record assertion based only on nonzero color. The numeric test still has twelve
submissions and maps per binary; it now has twenty-seven render passes and a
six-plane, 24,576-byte packed readback. Cases 7 and 8 still compare exact bytes
with the original fallback shader.

A second ignored regression exercises the real retained renderer: rejected
frames, moving geometry, light and camera changes, resolution transitions,
explicit cuts, and disabling/re-enabling the owner attachment. It checks
accepted color/depth/record resources and motion metadata, with twelve renderer
submissions and twelve readback submissions. It does not establish visual quality.

The [CPU/compiler manifest](evidence/temporal-owner-consumer-corrected-cpu.json)
records all ten successful checks: 33 cooker tests, 152 default renderer tests,
167 cooked/native renderer tests, strict Clippy in both configurations, wasm,
dependency, formatting and convention checks. The 43/44 ignored GPU tests did
not run. All 2,737 source inputs remained fixed. Fifty-seven freshly rebuilt old
shader payloads are byte-identical; nine manifest producer stamps changed, and
67 old records were not rebuilt. The two new binaries are frozen separately.

Independent source reviews accepted the shader, reference fixture and retained
lifecycle test. The [fresh bounded Metal run](evidence/temporal-owner-consumer-corrected-gpu.json)
passed the numerical and retained tests in both frozen binaries on Apple M5 Pro.
All four children and their supervisor exited cleanly: 72 explicit submissions
and 48 maps. The corrected case 3 is byte-identical to its current-only reference;
its center is `4.172325e-7`, consistent with the tiny positive-weight CPU diagnosis.
The old binary was not rerun. All twelve printed numerical observations match
between WGSL and cooked Metal; retained pixel data are assertion-only, not an
external cross-backend image comparison.

Appearance acceptance remains pending. The older private
upscaling/MetalFX ancestry, appearance failures, current-main AO integration,
device measurements and performance gates remain open. This is not permission
to publish that entire private branch or a Phase 1 completion claim.
