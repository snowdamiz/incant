# Fixed temporal coverage

The opt-in fixed-resolution temporal resolve retains premultiplied coverage at
opaque depth silhouettes and opaque/MASK-to-clear boundaries. The existing
64-byte frame and four texture bindings are unchanged. Portable output upscaling
and MetalFX are separate implementations and are not modified here.

For each current pixel, a bounded 3×3 neighborhood supplies the range of expected
previous **view-space** depths. Clear is far-plane support. The range is expanded
by the existing absolute/relative tolerances. The closest valid expected-depth
motion is selected, with center priority on equal depth. This uses the existing
motion record, not a new current-depth input. Invalid opaque center motion still
rejects history, so a new owner cannot borrow a neighbor's motion history.

Positive bilinear history taps inside the viewport are admitted independently
against that range, then renormalized. Clear (alpha ≤ 0) is canonical zero RGBA, including malformed residual RGB.
Finite premultiplied RGBA, including
fractional alpha and zero-alpha clear, enters the same neighborhood clamp and
reactive blend. Pure clear neighborhoods clamp old coverage to zero. Background
more than one current pixel from a foreground edge has no foreground-depth
support. Invalid/outside history and absent support remain current-only.

Only the fixed-resolve trusted source hash changes in the shader cooker. The
prior single-depth program and a mutation restoring opaque-only history alpha
are explicitly refused. Backend layouts, bounds policy and six stage artifacts
retain the existing ABI. There is no runtime compiler or fallback addition.

## Numeric scope

A test-only f64 oracle exercises both projection mappings, an opaque depth edge,
an opaque-to-clear edge, alternating one-texel MASK occupancy, foreground motion,
invalid owner records, viewport limits and stable equal-depth selection. Over
16 frames the edge remains between both surfaces; with reaction disabled, its
step is bounded by `(1 - history_weight) * contrast`. This is an algebraic test
policy, not a new runtime default. Existing reactive reduction is unchanged and
can produce a larger step. Foreground absence at least two pixels behind the
moving edge restores the exact background. No image is generated or inspected.

The new ignored device test runs at most 150 tiny resolves, comparing every
output component with the oracle and retaining actual half-float history between
steps. It includes real nonuniform depth attachment writes and the actual
resolve/copy pipelines. These are controlled post-raster occupancy inputs; they
do not replace end-to-end model/MASK raster or rendered-quality review. This
producer **compiles but does not execute** that test, or any other GPU test.

Existing negative history fixtures that formerly used zero alpha now use
out-of-domain alpha 2: zero alpha is valid coverage under the new contract.
Projection, radiance reaction, positive-support renormalization, exclusion and
presentation registration regressions remain present.

## Verification and limits

Independent source/CPU review accepted source004 and closed the malformed-clear
finding; both review receipts are retained. Final parent packet review is pending.
The [evidence manifest](evidence/temporal-coverage.json) identifies each frozen
source generation and retained command. Final source004 repeats every applicable
CPU/check command after independent review found residual RGB at alpha zero. Default renderer CPU: 129 passed, 41
GPU cases ignored. Cooked-temporal/MetalFX CPU: 144 passed, 42 GPU cases ignored.
Cooker interface and source-identity tests: 3 passed. Strict all-target Clippy,
browser compilation, normal dependency guards, formatting and conventions pass.
Browser compilation retains five pre-existing timestamp dead-code warnings.
Final default and cooked test executables are compiled and hash-bound separately.

Initial test type inference, stale cooked-source hash, test-only Clippy and missing
root arguments to the dependency guard are retained as failed attempts. The
shader bytes are identical across source001–003; source002 adds the exact
cooker hash/tests, source003 only changes an ignored test comparison to `zip`. Source004 fixes the
reviewed malformed-clear invariant, updates its exact cooked hash and adds
current/history/neighbor CPU plus ignored-device controls. The historical finding,
source003 packet and both old binaries remain retained.

GPU numerical execution, Claude's fixed-mode appearance review and matched
shipping/device performance remain pending. This does not approve temporal
quality, portable upscaling, MetalFX, or any Phase 1 gate.
