# Extended cluster grid observer

This adds only `cfg(test)` numeric observation requested by the 0066 cooked
cluster handoff. Production renderer, shaders, resource layouts and feature
admission are unchanged. Actual GPU execution of this extension is pending a
coordinated numerical GPU run; CPU compilation is not GPU or appearance acceptance.

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

## Coordinated GPU follow-up

After obtaining the exclusive GPU lease, run this exact ignored test in separate
baseline and cooked-cluster builds from the same source and toolchain:

```sh
./tools/cargo test --profile ci --locked -j2 -p incant_render --lib \
  lighting::gpu_tests::extended::extended_cluster_readback_cameras_viewports_and_retained_motion \
  -- --ignored --exact --nocapture
./tools/cargo test --profile ci --locked -j2 -p incant_render --features cooked-clusters --lib \
  lighting::gpu_tests::extended::extended_cluster_readback_cameras_viewports_and_retained_motion \
  -- --ignored --exact --nocapture
```

Retain both raw logs, test binary hashes, exact source/feature/toolchain identity,
adapter/OS identity and terminal exits. Compare all 84 named records exactly;
zero, missing, duplicate or additional records must fail the comparison. This
would establish numerical parity on the tested backend only, with no claim
about other GPUs, phone budgets or shader-hitch performance. Historical 0066
results remain separate and do not count as execution of this new test.

## Local verification

The original source001 fixture contained only lights and a camera. Independent
pre-execution review found that the retained renderer skips clustered compute
when there are no model draws. That fixture could observe an older grid rather
than test submitted light changes. No GPU execution had occurred. The original
CPU evidence is preserved in the [historical manifest](evidence/cluster-grid-readback-source001.json), and its source acceptance is superseded for this case.

Source002 includes the existing cooked triangle fixture, keeps its immutable
model bindings after source removal, asserts nonzero prepared primitive instances,
and moves only point/spot lights. A new CPU regression resolves this geometry
without creating a GPU device. Actual adapter information is printed by the
future GPU execution, not inferred from the machine name.

All three CPU cases pass in both default and cooked-cluster configurations;
four GPU tests remain explicitly ignored in each filtered run. Both complete
GPU test bodies compile. All-target Clippy passes in both configurations;
scoped rustfmt, conventions and diff checks also pass. These are seven successful
commands over 1,119 frozen engine/build files. Neither configuration has run the
new GPU test. Cooked-clusters also enables its existing cooked-output dependency.

The [current manifest](evidence/cluster-grid-readback.json) binds the source,
all terminal logs/receipts and two binaries copied outside the reusable build
target. A lossless archive retains the full freeze, repair and logs.
[Independent source review](evidence/cluster-grid-readback-002/source-review.json)
accepted the corrected fixture. Actual 84-record GPU parity remains pending.
