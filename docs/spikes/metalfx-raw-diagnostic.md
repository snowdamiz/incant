# MetalFX raw vendor/guard diagnostic

This source-only successor of image001 observes the unchanged default-off native
route. The prior 15-image appearance result remains **FAIL**. It neither repairs
silhouettes nor changes a quality threshold, background domain, shader kernel,
reactivity, mip selection, jitter sequence, exposure or motion convention.
No new native build or GPU execution has run at this source freeze.

`INCANT_METALFX_RAW_DIAGNOSTIC=1` opts the existing ignored material image fixture
into four raw observations: MFX50/MFX75 at frames 16 and 24. Absence preserves its
original capture route; any other value refuses. All production shader sources
and the 96-frame scene/input chronology remain byte-identical. The fixture still
has 15 display captures and 111 explicit submits, 15 maps, 64 native encodes and
239 explicit command buffers. Four isolated R8Uint mask passes are included in
four existing copy encoders; they have no timestamp queries and never feed the
normalized output, native history or subsequent frames.

Test-only format-specific COPY_SRC usage and a private observation accessor are
compiled only under test + native-metalfx + macOS. Observation must additionally
be enabled on each fixture owner: unrelated native tests do not retain snapshots.
A snapshot holds safe wgpu handles and typed Arc owners for native and motion
resources, including the native allocation lease. It becomes visible only through
the existing accepted submission path. These are retained resources, not immutable
copies of their contents: the fixture reads immediately, before its next submit.
No raw pointer, alternate queue, public capability or engine-agent action is added.

Each selected map contains exactly 12 planes: existing RGBA8 display; source HDR
RGBA16, source motion RGBA32 and depth32; packed color RGBA16, depthR32, motionRG16,
reactiveR8; vendor and normalized RGBA16; exposureR16; and maskR8Uint. Original
Frame96 and motion Frame144 bytes and actual scaler getter values are retained.
Native getters describe configuration after encode, not GPU completion. Copies
use the same wgpu queue, wait for the specific submitted copy command, and require
a successful full-buffer map callback before reading. The mask binds the exact
Frame96 and source/current/native/depth views; each bit is independently visible:
coverage, positive depth, compatible depth, finite native output, nonnegative
native RGB, and selected native RGB. Bits 6 and 7 must remain zero.

Pinned wgpu-core 29.0.4 refuses partial depth/stencil texture copies. Therefore the
source depth subresource is copied in full and cropped after mapping. This makes
the aligned 50% map 18,386,176 bytes and the 75% map 25,229,056 bytes. The latter
exceeds the original proposal's 24 MiB by 63,232 bytes; the reviewed successor
uses a **25 MiB** map cap. Row strides and offsets are 256-byte aligned. Saved raw
planes still contain only active source pixels. Added raw payload is 73,159,208
bytes including four exposure texels; existing display raw adds 31,104,000 bytes.
The unchanged aggregate output cap is **128 MiB**, including PNGs, source fixture,
uniforms, shader and report. Copy and evidence lengths are checked explicitly.

The independent external `analyze.py` uses f64 bilinear/source-depth predicates,
checks all file identities before reading, and records predicate disagreements
instead of adjusting tolerances for float32 boundaries. Selected native output
can be checked against the captured half-float vendor RGB exactly; fallback
bilinear half differences are reported as observations. It checks normalized
finite/nonnegative/zero-alpha containment. It does not interpret scalar agreement
as appearance success. Fresh generated fixture ULIDs differ across runs; prior
image bindings compare semantic roles, transforms, payload fingerprints and
settings after removing those ID fields, explicitly without a cross-run entity
identity claim. Full/mfx arms within one run still use exact shared input records.

The review packet is external at
`~/.codex/incant-evidence/metalfx-raw-diagnostic-001/`. Its frozen CPU proposal is
one serial native-feature image unit test build (three CPU tests plus one ignored
GPU test), then strict all-target Clippy, Rust 1.99 arm64/CI/locked/offline/jobs2,
Rayon2 and a 6 GiB free floor. No shader or capture execution is authorized by
this document. The eventual GPU proposal remains one 120-second child within a
150-second stage, no retries, using an immediately copied/listed executable and
an explicit root lease. Prior image001 compilation failure, corrected source,
raw images and negative review are preserved in their original evidence packet.
