# Bounded native GPU timestamps

`Renderer::new_with_gpu_timing` opts into native **pass** timestamp queries.
Ordinary `new` and `headless` remain unchanged. Unsupported adapters and browser
requests fail with a typed error; CPU callback latency never substitutes for a
GPU query.

The graph records initialization, cluster assignment, shadows, models, sprites,
text and display when those groups actually encode work. Render and compute pass
descriptors write the endpoints. A multi-pass shadow group writes only its first
beginning and last ending; intermediate passes have no timestamp descriptor.
The overall interval starts at the first instrumented pass and ends at the last.
These labels do not include later screen-space UI, presentation, preparation,
uploads, or query resolve/readback. They are not a complete application frame
budget or a strict barrier guarantee. New graph passes must explicitly join the
instrumentation when composed; motion and post-processing producers are separate.

Three fixed slots each own fourteen query entries, a 112-byte resolve buffer and
a 112-byte readback buffer. A full pool skips measurement while rendering
continues. Pending and unread results retain their slots. `take_gpu_timing`
polls native callbacks without waiting and returns at most one result; sequence
numbers identify encoding order, which can differ from submission/completion.
There is no unbounded result history or blocking queue drain.

The submitted frame's completion callback creates and submits a separate tiny
resolve/copy command buffer. Its deferred map callback decodes the numeric payload,
drops the mapped view and unmaps before publishing. This adds an asynchronous
submission per measured frame; it does not wait for GPU completion in normal
rendering. Callback captures retain the exact device, queue, buffers and lease.
Dropping an unsubmitted frame releases its reservation. Failed mapping reports
once and permanently retires the slot because wgpu's public mapping bookkeeping
cannot safely be reset by an unconditional second unmap. Remaining slots continue.

Ticks are subtracted as integers before timestamp-period conversion, preserving
small deltas in large absolute counters. Reversed/wrapped counters are refused.
An entirely zero query payload now produces `GpuTimingError::Unwritten` instead
of a plausible zero-duration measurement. Equal nonzero endpoints remain valid
quantized zero intervals. Raw ticks and period accompany every accepted result.

## Metal correction and evidence

Claude 0068's original 90 GPU regressions and four timing-off/on appearance pairs
passed, but all eight instrumented results contained only zero counters. Those
results are preserved and **do not validate GPU measurement**. The original
encoder-write implementation is superseded.

A bounded numerical compute diagnostic compared encoder writes versus pass
begin/end descriptors, each resolved in the same buffer or after work completed.
All four cases verified their actual compute output. On the recorded Apple M5 Pro
Metal adapter, only pass descriptors with a later resolve returned nonzero ticks.
The diagnostic preserves that result without extrapolating to other drivers.

The corrected renderer passes 74 affected CPU tests with 91 GPU tests ignored,
all-target Clippy, browser compilation, formatting and diff checks. Browser
compilation has five dead-code warnings in the deliberately unavailable native
readback path; it is not a warning-free browser build. Independent source review
caught and corrected an empty timestamp descriptor on intermediate shadow
cascades; shared endpoint tests cover one, two, four and eight passes.

One actual numeric renderer test then submitted three 64×64 frames, including an
unmeasured frame under full-pool backpressure. The two instrumented frames returned
positive overall intervals of 304,666 and 406,209 ns, with nonzero initialization,
model and display samples. Out-of-order submission, unsubmitted abandonment,
asynchronous readback, absent text groups and slot reuse passed. These are
correctness observations, **not performance benchmarks**. All owned children were
reaped and the GPU/target reservation was released. No image was decoded by Astra.

[Retained evidence](evidence/gpu-timestamps-pass-resolve.json) binds the frozen
source, exact copied binaries, raw logs, historical failures, source-review
receipts, numeric results and process ledger. The original
[evidence](evidence/gpu-timestamps.json) remains historical. Revised appearance
parity, full GPU regressions and composition with current main remain pending.

The implementation uses pinned wgpu 29.0.4 pass timestamp descriptors,
`CommandEncoder::on_submitted_work_done`, `map_buffer_on_submit` and the queue's
timestamp period. No shader math or dependency resolution changed. This is not
the full profiler: memory/allocation tracking, complete-frame timing, network
capture from phones and matched shipping-profile reference-device measurements
remain open. CPU Tracy zones are a separate merged implementation.
