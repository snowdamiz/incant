# Motion-owner attachment — accepted CPU and offline scope

Producer `613ee59aec1f2bce080d4a1e335dc45f732b993f` preserves exact reviewed
source003. The original implementation note retains its source-freeze wording;
this separate closure records subsequent review without rewriting that evidence.

Root independently accepted the final source/CPU/offline packet after replaying
its assertion audit. The retained [receipt](evidence/temporal-owner-attachment-final-cpu-root.json)
has SHA-256 `d232a2241667c7258a4211489b851b2b7bdb7f72bf72b245d613fa678d23d3e2`.
It binds [manifest](evidence/temporal-owner-attachment.json)
`eff501183d1226cba85a192b8b5db30e9f8b78eec9a630c5f791501071c523fc`,
213 original/lossless records, 127 payloads, 30 frozen inputs, and two executable
copies. Source reviews and original failed attempts remain in the archive.

Nine final commands pass: renderer CPU tests, strict Clippy for both default and
cooked/native features, browser compilation, dependency graph and repository
checks. Default has 143 passed/41 ignored; cooked/native has 158 passed/42 ignored.
The suites overlap. Eleven cooker tests and nine offline compiler/validator
commands also pass; twelve original Motion/Depth backend payloads remain exact.
Metal compilation and SPIR-V validation are not GPU execution; DXIL execution is
not established. All owned build processes are terminal and reaped.

The separately prepared [0073 plan](../../handoffs/0073-upscaling-review/owner-attachment-plan.json)
proposes one exact cooked/native test invocation: twelve renders and twelve
batched readbacks, 24 explicit queue submissions, at most 32×32, no images. The
completion marker is required so an unsupported early return cannot count as
acceptance. CPU-only enumeration confirms the exact test in both copies; only the
cooked/native copy is proposed for this first bounded run. Root GPU authority and
Claude execution remain pending. No new build or target ownership is needed.

This supplies source-owner transport, not history consumption or a visual-quality
fix. The original logical admission ceiling remains unchanged; queued driver
allocations are not a claimed physical bound. Appearance, performance, phones,
fixed-temporal/MetalFX owner combinations and later history consumers stay open.
