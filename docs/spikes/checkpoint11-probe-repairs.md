# Checkpoint 11 probe contract repairs

Checkpoint 11 froze main #141 (`89c6180b`) at `82aa2bc3`. All eight workflows
finished: 20 of 23 required contexts succeeded, with Windows desktop, Linux
desktop and macOS checks failing. Release was not promoted. Full failed job
logs, the exact context snapshot and source identities are retained in
[evidence](evidence/checkpoint11-probe-repairs.json).

The Windows model-only render probe received the original eight geometry
counters plus four zero text counters. Its exact dictionary omitted those
fields. The corrected dictionary requires all twelve exact values; changed,
missing and extra fields still reject. PNG validation, cache-only source removal
and changed reimport controls are unchanged. The old predicate fails the actual
hosted response; focused and full Python checks pass (234 cases, four skipped).

The macOS four-player proof reached the manual-input refusal after completing
prior successful work, then compared the error against a diagnostic removed by
the optional UI argument parser. The helper now expects that parser's specific
spatial or plain argument error. It still requires failure, empty stdout and no
PCM. Six actual CLI cases exercise `--input` and an unknown option in plain,
CPU and GPU argument modes with all scene/device inputs missing. All refuse at
the argument boundary; no GPU initialization occurs. The retained copied player
and byte-identical parser inputs are bound in the evidence. The std-only Rust
integration test was compiled directly against that copied player, avoiding
writes to the active integration target; normal Cargo test discovery retains it.

Both corrections have independent source review. No production renderer, shader
or player parser behavior changes. The initial local formatting command lacked
`cargo-fmt` on PATH; the corrected pinned-toolchain invocation passed unchanged
source. The Linux UI readback differences remain an independent Claude review,
and no numeric threshold changes here. Actual Windows and complete four-player
shipping reruns remain for a deliberately frozen repaired checkpoint.
