# 0068 — native GPU timestamp execution and unchanged appearance

Use Claude Opus 5.5 through ACP with Max effort explicitly verified before every
new or resumed session. Read CLAUDE.md. Claude owns rendered-pixel review; Astra
owns source correctness. Do not substitute a model or effort setting.

This is preparation only. Do not build, execute GPU work or acquire the shared
target until root creates the readiness grant below. Packets 0066 and 0067 precede
this work. Do not poll for a grant or start another target to bypass the queue.

## Source and contract

Implementation `99778642585a7b545a9448b9fca4a365ba622985` is based on main186
`1c5658cf5d7eee8390b272394f38b8d9b49e1628`. Read
`docs/spikes/gpu-timestamps.md`, `docs/spikes/evidence/gpu-timestamps.json` and the
adjacent independent source-review receipt. The final source freeze is
`d41ba28b5cd1bf3007706d844904c4f846e0cdcc69134522c0c7061fc05a8f25`.
Seven focused and 70 affected CPU cases pass; 90 GPU cases were intentionally
ignored. All-target Clippy, browser compilation and dependency checks pass. No
actual GPU query result or appearance acceptance is claimed yet.

`Renderer::new_with_gpu_timing` explicitly requests native encoder timestamps.
Ordinary `Renderer::new` stays unchanged. There are three fixed query/readback
slots and seven optional scene-graph groups. A full pool skips instrumentation
without skipping rendering. Dropped unsubmitted command buffers release their
reservations. Successful callbacks unmap before publishing. Mapping failure
reports once and permanently retires that slot; no recovery allocation or unsafe
unmap. `take_gpu_timing` polls without waiting. Sequence numbers identify encoding
order, not submission/completion order.

These intervals exclude later UI, presentation, uploads and CPU work. Groups
must not be summed or assumed to establish strict hardware barriers or CPU/GPU
clock alignment. Zero durations can be legitimate quantization; reversed/wrapped
intervals are typed errors. This is correctness/appearance work, not a benchmark.

## Actual GPU verification

Freeze the exact source, test/adapter helper source, Cargo compiler artifacts,
feature sets, binaries, OS and adapter/backend before execution. Use locked
dependencies, ci profile and at most two compilation jobs. No gameplay, shader
math, filters, visual layout or public timing policy changes. If a missing native
observer or implementation defect is found, give Astra the exact reproducer and
wait for source review before compiling changes. Keep failures in the record.

Run the current ignored numeric test serially:

    ./tools/cargo test --locked --profile ci -j2 -p incant_render --lib \
      timing::gpu_tests::bounded_timestamp_queries_follow_real_submission_and_abandonment \
      -- --ignored --exact --nocapture --test-threads=1

It checks actual out-of-order submission, unsubmitted command-buffer abandonment,
full-pool backpressure, deferred query readback, raw values and slot reuse. Record
the actual query feature flags and period. Unsupported capability must remain an
explicit refusal, never substituted CPU timing or a claimed pass.

List and run the applicable existing native renderer and GPU-host regression
tests serially. The reviewed base has 90 ignored cases including the new numeric
test; explain any integrated-source count change and prevent running that test
twice by accident. Record all pass/fail/refusal outcomes and every child command.

For a few representative unchanged scenes, compare explicit timing disabled
against enabled using otherwise identical scene/format/viewport/feature inputs.
Include model/shadow/lighting, sprites/text, and nonzero viewport placement where
existing helpers support them. Observe complete unmasked frames and retain 3–6
matched pairs. Compare exact bytes where deterministic; any difference needs a
real diagnosis. Verify raw timestamp labels correspond to groups actually
encoded, absent groups remain absent, and rendering continues under full-pool
backpressure. Do not infer performance gains from the sample durations.

If callback-after-owner-drop coverage needs a new helper, request Astra review
of that small numeric helper first. Do not manufacture device loss or silently
relax timestamp validity checks. CPU failed-map controls are not a real device
mapping-failure test.

## Resource ownership and delivery

Only root may create
`/Users/andreyyurlov/.codex/incant-evidence/0068/review-ready.json`, naming the exact
source and confirming exclusive GPU and target release after 0067. Expected
target is `/Users/andreyyurlov/.codex/incant-core-presentation-target`; never use
root's `/Users/andreyyurlov/.codex/incant-gpu-timestamps-target` or another agent's
active target. Use a root-approved isolated child checkout. Evidence belongs in
`/Users/andreyyurlov/.codex/incant-evidence/0068`.

Run one bounded process group at a time: five minutes per existing GPU suite,
two minutes per added numeric/capture process. Beyond existing regression frames,
allow at most 48 new numeric lifecycle submissions and 24 total newly rendered
appearance frames, each at most 960×540. Every comparison/control/dry-run frame
counts toward that appearance total even when no PNG is retained. Existing
regression-suite frames are separately counted. Freeze case names and bounds first. Offscreen engine frames
only; no desktop/browser capture, native window, device audio, accounts or secrets.
Hash-check source/binary inputs for each child, terminate and reap timed-out
groups, then explicitly release GPU and target with no owned processes remaining.

For process receipts, record only explicit nonsecret overrides from a documented
allowlist (for example CARGO_TARGET_DIR and INCANT_NATIVE_PROJECT). Never serialize
the inherited environment, ACP/session transport values, credentials or account
metadata. Reuse the safe 0064/0066/0067 logger pattern; do not copy the original
0065 environment logger. No private receipts or secrets enter Git.

Return result.md, reproducible packet-only helpers, raw numeric receipts, source
and binary identities, and a small unedited screenshot selection. Keep visual and
numeric verdicts separate. Commit with Built-by: claude; no push, PR, CI or main
changes. This packet cannot approve reference-phone performance, other untested
GPU backends, the complete profiler or a phase gate.
