# Fixed-point UI reference integration on main154

The independent reference correction is integrated on actual main154 `edb1508789cf984746bc69a57079c8e1152d7b2c`.
Both source files and all 40 changed producer packet files are byte-identical to
Claude's accepted `de73103d`. The only renderer-module change is explanatory
commentary; shader and runtime behavior are unchanged. The original handoff
paths, Linux/Vulkan and Windows/DX12 failure logs, and parent instructions are
retained. Git's proposed directory relocation was corrected without byte changes.

All 1728 frozen inputs stayed unchanged. The current union passes
64 CPU tests across 4 groups with no failures; 82 GPU
cases remain intentionally ignored. All-target Clippy, wasm32 renderer compile,
dependency guard, formatting and conventions pass. [Integrated receipts](evidence/native-ui-unorm-main154.json)
retain seven commands, raw/compressed hashes and the full input inventory.

The model propagates bounded source, alpha and destination-product rounding for
8-bit UNORM blending. Exact opaque and untouched pixels and geometry/clip rules
remain checked; this is not a blanket tolerance increase. Claude's original
Metal and seven mutation controls remain scoped to their original build.

No new GPU, native or appearance validation occurred here. WARP arithmetic is
inferred from matching aggregate statistics, not captured pixels. Actual hosted
Linux/WARP success and all 23 release contexts remain required; checkpoint12
stays failed/frozen until a deliberately selected corrected successor runs.

Windows conformance separately reached its 40-minute job limit after successful
16m10s dual-host and 22m38s optimized MSVC controls. GitHub's retained annotation
explicitly identifies that limit; the cancellation is not a conformance assertion
failure. The same existing Windows job now has 90 minutes for all required shipping
stages. macOS/Linux retain 40 minutes, every existing check remains, and the job
still runs only in full checkpoint validation. Actionlint 1.7.12 (verified against
its published archive checksum) and six release-policy tests pass. This isolated
workflow edit followed the frozen renderer checks and changed no compiled inputs.
