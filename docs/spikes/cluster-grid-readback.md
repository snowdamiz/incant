# Extended cluster grid observer

This adds only `cfg(test)` numeric observation requested by the 0066 cooked
cluster handoff. Production renderer, shaders, resource layouts and feature
admission are unchanged. The extension now passes on Apple M5 Pro Metal in both default and cooked-cluster
configurations: all 84 named raw count/mask digests match exactly.

The observer clones the already prepared grid from the lighting cache, queues a
counts/masks copy after the caller's submission, waits with bounded poll/map
timeouts, and unmaps after copying numeric words. It never calls the cache's
resource-creation helper. Retained frames therefore expose their actual submitted
buffers, without an accidental replacement with a fresh zero-filled grid.

The ignored test covers 75 combinations: 1, 31, 32, 33 and 4096 local lights;
320×192, 640×360, 960×540, 333×197 and an offset 640×360 viewport; preview,
authored perspective and authored orthographic cameras. Two directional lights
precede seeded point/spot lights. A guaranteed in-frustum point rejects false
all-zero observations. Viewport origin changes must preserve camera-relative
membership for otherwise identical dimensions.

Two retained sequences, perspective and orthographic, each submit an initial
frame, move the lights, move them entirely outside the view, then restore them
with a resized offset viewport. Same-size moves must retain the grid `Arc`;
resize must replace it. Actual membership must change after the first move,
clear entirely outside the view, and return after restore. A directional-only
control requires absent compute work and zero membership. In total, the test
prints 84 distinct `cluster-readback` records.

Every record validates count equals mask popcount and rejects bits beyond the
local-light count. Hashes cover explicit little-endian raw count/mask words.
Two CPU tests exercise valid mask boundaries and malformed count, tail-bit
and length refusal. A third resolves actual cooked model geometry and light
bindings after the temporary authoring sources have been dropped. No pixels, screenshots, image decoding or performance
measurements are involved.

## Numerical GPU verification

After 0067 released its GPU lease, the independently reviewed runner executed
one exact ignored test in each frozen binary, serially. Both exited successfully
and were reaped. Each emitted exactly 84 uniquely named records; the actual
adapter identities and all little-endian count/mask digests matched. The adapter
reported Apple M5 Pro, Metal, on macOS 27.0.1 arm64 (Darwin 27.0.0).

This covers the camera/light-count/viewport matrix, actual retained-frame
movement, offscreen clearing, resize and the directional-only control described
above. It proves numerical parity for these inputs on this recorded backend.
Other GPUs, reference-phone budgets, appearance and shader-hitch performance
remain outside this result. Historical 0066 appearance results remain separate.

The runner has bounded waits and kills/reaps its process groups in finalization.
CPU-only lifecycle controls separately exercised successful completion, timeout
and interruption; the interrupted exception was preserved while cleanup receipts
showed no surviving owned processes. The GPU lease is released.

## Local verification

The original source001 fixture contained only lights and a camera. Independent
pre-execution review found that the retained renderer skips clustered compute
when there are no model draws. That fixture could observe an older grid rather
than test submitted light changes. No GPU execution had occurred. The original
CPU evidence is preserved in the [historical manifest](evidence/cluster-grid-readback-source001.json), and its source acceptance is superseded for this case.

Source002 includes the existing cooked triangle fixture, keeps its immutable
model bindings after source removal, asserts nonzero prepared primitive instances,
and moves only point/spot lights. A new CPU regression resolves this geometry
without creating a GPU device. The GPU execution prints actual adapter information; it is not inferred from
the machine name.

All three CPU cases pass in both default and cooked-cluster configurations;
four GPU tests remain explicitly ignored in each filtered run. Both complete
GPU test bodies compile. All-target Clippy passes in both configurations;
scoped rustfmt, conventions and diff checks also pass. These are seven successful
commands over 1,119 frozen engine/build files. These CPU runs precede the separate, successful two-process GPU verification.
Cooked-clusters also enables its existing cooked-output dependency.

The [current manifest](evidence/cluster-grid-readback.json) binds the source,
all terminal logs/receipts and two binaries copied outside the reusable build
target. A lossless archive retains the full freeze, repair and logs.
[Independent source review](evidence/cluster-grid-readback-002/source-review.json)
accepted the corrected fixture. The GPU manifest retains both raw logs, the
runner, grant/release identities, 84-record comparison and lifecycle checks.
