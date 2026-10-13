# Bounded native GPU timestamps

Phase 1 profiler groundwork. `Renderer::new_with_gpu_timing` explicitly enables
native encoder timestamp queries; ordinary `new` and `headless` behavior remains
unchanged. An unsupported adapter or browser request fails with a typed error.
No clock estimate or CPU callback duration is substituted for a GPU query.

The existing scene graph records its overall interval and separate initialization,
cluster assignment, shadow, model, sprite, text and display intervals. Absent
groups have no sample. These are instrumented command-encoder groups, not a claim
that every group contains exactly one hardware render pass. The measurements
exclude later screen-space UI, presentation, CPU preparation, queue uploads and
query resolve/readback. Instrumentation itself can change scheduling and cost.

Three fixed slots each own one query set, one 128-byte resolve buffer and one
128-byte CPU readback buffer. A full pool skips instrumentation while rendering
continues. Pending and unread results keep their slots; callers consume at most
one result per `take_gpu_timing` call. That method polls native callbacks without
waiting. Sequence numbers identify encoding order, which need not equal submission
or completion order. `gpu_timing_stats` reports pending, ready, skipped and
abandoned and retired measurements. There is no unbounded history, filesystem I/O, network
transport or blocking queue drain.

Deferred mapping belongs to the returned command buffer. Dropping it before
submission drops its lease and returns the slot. After submission, a callback
decodes only the small numeric timestamp payload, drops the mapped view and
unmaps the buffer before publishing the result. The lease is disarmed before
publication so its destructor cannot retire a newly reused slot. GPU resources
remain owned by wgpu while commands are pending. A failed mapping becomes a typed
readback error exactly once and permanently retires that slot. wgpu's public
mapping bookkeeping can remain set after a failed map, while the backend is
already unmapped; neither reuse nor an unconditional second unmap is safe.
Other slots keep working, and retiring all three simply skips future samples.
Device loss follows the existing renderer failure path.

The queue's timestamp period converts ticks to nanoseconds. Integer subtraction
precedes floating conversion so small intervals survive large absolute counters.
Reversed or wrapped counters are discarded, not converted into plausible positive
durations. Zero can be legitimate timestamp quantization. Raw ticks and period are
retained alongside each duration. These clocks do not establish CPU/GPU clock
alignment or add up to a frame-time budget: wgpu explicitly permits backend and
driver command reordering around encoder timestamps.

The implementation uses the repository's pinned wgpu 29.0.4
[encoder query API](https://docs.rs/wgpu/29.0.4/wgpu/struct.CommandEncoder.html#method.write_timestamp),
[deferred mapping](https://docs.rs/wgpu/29.0.4/wgpu/struct.CommandEncoder.html#method.map_buffer_on_submit)
and [timestamp period](https://docs.rs/wgpu/29.0.4/wgpu/struct.Queue.html#method.get_timestamp_period).
No dependency resolution changes or shader changes are needed.

Seven focused CPU tests pass and cover capability refusal, period and
counter handling, capacity/backpressure, abandoned reservations, cross-thread
completion, failed-map retirement, labeled query decoding and warmed
allocation-free bookkeeping. Independent review found and corrected the failed-map
reuse case before GPU execution. The
first allocation assertion exposed the platform mutex's lazy thread parking
initialization; the corrected test explicitly warms it before measuring steady
state. This says nothing about wgpu's callback allocations or full-frame allocation
behavior. One compiled, ignored GPU test exercises actual out-of-order submissions,
unsubmitted command-buffer drops, asynchronous query readback and slot reuse;
it has not yet run. The complete affected CPU run passes 70 cases with 90 GPU
cases intentionally ignored. All-target Clippy, the browser build, runtime
dependency boundary, formatting and generated conventions pass. Independent
source review accepts the corrected implementation. Frozen source copies, raw
logs, the two prior check attempts and the review receipt are retained in the
[evidence manifest](evidence/gpu-timestamps.json). Actual query execution and
appearance parity remain queued for Claude's exclusive GPU lane.

This is not the complete profiler: Tracy CPU integration, memory accounting,
full-frame UI timing, browser pass timestamps, on-device capture and matched
shipping-profile reference-phone performance evidence remain separate work.
