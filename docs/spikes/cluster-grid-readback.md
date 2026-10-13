# Extended cluster grid observer

This adds only `cfg(test)` numeric observation requested by the 0066 cooked
cluster handoff. Production renderer, shaders, resource layouts and feature
admission are unchanged. Actual GPU execution of this extension is pending a
coordinated Claude run; CPU compilation is not GPU or appearance acceptance.

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
The two CPU tests exercise valid mask boundaries and malformed count, tail-bit
and length refusal. No pixels, screenshots, image decoding or performance
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

The two CPU invariant cases pass in both default and `cooked-clusters` builds;
these are the same cases in two configurations, not four distinct tests.
Both complete ignored GPU test bodies compile. All-target Clippy passes with
warnings denied in both configurations. Scoped rustfmt, conventions and diff
checks pass. The source stayed byte-identical through all seven commands.

The [manifest](evidence/cluster-grid-readback.json) binds the three source inputs,
seven command receipts/logs and two compiled test binary identities. Lossless
logs and source copies are retained; large binaries remain outside the repository
in the private target and are identified by hash. Original evidence is under
`~/.codex/incant-evidence/cluster-grid-readback-001`. Independent [source review](evidence/cluster-grid-readback-source-review.json)
accepted the frozen source and CPU evidence. Actual GPU parity remains pending. No GPU command has been executed by this
implementation lane.
