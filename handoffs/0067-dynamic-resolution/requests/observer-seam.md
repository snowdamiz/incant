# 0067 request to Astra: test-only delivery observer for the composed host

Status: request only. 0067 changed no engine source and needs no production
change. Everything below was verified through public contracts; this request
covers what those contracts cannot express exactly.

## What public contracts could not show exactly

1. **The delivered value.** `GpuRenderConsumer` exposes `resolution_frame()` and
   `discarded_resolution_samples()`. The `(ResolutionFrame, elapsed_ms)` pair that
   `accept` takes from the completion slot is not observable. 0067 bounded each
   delivered value from outside with a witness: a second
   `Queue::on_submitted_work_done` callback that the test provider registers in
   `RenderFrame::present`, just before the consumer registers its own. Each
   delivered value then lies in `[lo, hi]`:
   - `lo` = witness dispatch − the provider's `acquire`;
   - `hi` = the next logged event − the tick call.

   Interval widths in the paced run were 0.14–0.27 ms against band edges of 15.0
   and 18.33 ms, so classification was unambiguous. Exact values would remove
   the dependence on wgpu's same-batch callback ordering.
2. **The reason for a discard.** In the composed route, the reason for a
   discard (foreign owner, stale epoch, duplicate, previous scale, pixel
   camera) collapses to `record_frame_time(..) -> Ok(false)` and a counter.
   0067 built each lifecycle sequence so that every discard has exactly one
   possible cause.
3. **Controller history.** The overload, recovery and cooldown counters are
   private. 0067 replays them from receipts with an independent policy model,
   in Rust in-test and in Python offline.
4. **Native ticket interleaving.** The interleaving in which an owner takes a
   completed result and reserves a new slot before the completed ticket's
   destructor runs needs cross-thread scheduling between `publish` and `Drop`.
   It cannot be forced natively without a hook. The CPU regression
   `completed_ticket_destruction_cannot_cancel_a_new_callback` covers it. 0067
   only shows that native deliveries continued: no reservation was lost across
   the runs.

## Precise seam (cfg(test) in `incant_render_gpu_host`, constant storage)

- Add a `#[cfg(test)] last: Option<(u64 /*sequence*/, u8 /*scale16*/, f64 /*ms*/, bool /*accepted*/)>`
  field to `resolution::Completion`. Set it in `GpuRenderConsumer::accept`, at the
  point where `completion.take()` yields a sample and `record_frame_time` returns.
- Add a `#[cfg(test)] pub(crate) fn last_resolution_delivery(&self)` accessor on
  `GpuRenderConsumer` that returns it.
- Optionally, add a `#[cfg(test)]` typed discard reason in
  `ResolutionOwner::observe` (incant_render), returned by a test-only sibling of
  `record_frame_time`.

`cfg(test)` is visible only to the crate's own unit tests. These checks therefore
need the 0067 composed-route cases moved into the crate: either
`incant_render_gpu_host/src` unit tests, or `tests/` behind a non-default
`test-observers` feature. Packet sources:
`handoffs/0067-dynamic-resolution/tools/drs/{src,tests}`. The fixture rebuilds
handoff 0056's review composition from the packaged test fonts plus generated
cubes. No unbounded trace, wait or timestamp query is needed.

## Observation for the controller owner (not a defect claim)

In the unpaced natural-cadence run, seven back-to-back offscreen ticks
(about 0.9 ms each, 6.3 ms total) finished before frame 1's work was observed
complete; the first frame of a process needed 6.6–6.9 ms in the paced run. The
host's non-blocking `check_device` polls therefore dispatched no completion,
and the controller received no samples: the scale correctly stayed at its
initial value. Feedback depends on host pacing and on polls after GPU
completion, which matches the documented "callback-dispatch delay". A host that
renders faster than the GPU completes will first see a sample several frames
late, carrying queue backlog. No real-phone or windowed-vsync measurement was
made.
