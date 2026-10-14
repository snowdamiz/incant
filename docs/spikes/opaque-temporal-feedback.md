# Canonical opacity in temporal feedback

This isolated correction remains on the reconstruction experiment branch. It does
not approve the pending R5 visual work or complete an upscaling/phase gate.

Weighted floating-point division can round proved opaque alpha just below one.
RGBA16F stores the result as 0.99951171875; the next frame correctly rejects it as
fractional coverage, losing valid history. Actual stationary chart captures on
Apple M5 Pro/Metal reproduce this in both portable paths. The correction writes
exact alpha one only when the existing support/history rules prove full opacity.
It does not widen admission with an epsilon or make fractional coverage opaque.
The fixed-resolution temporal blend uses the same invariant. The three trusted
shader fingerprints are updated to the reviewed source; strict cooker checks stay.

The regression feeds real RGBA16F ping-pong buffers through eight jitter phases
and six lanes: ordinary, owner fallback and owner static, each opaque/fractional.
A supported 14-to-32 source/output scale is necessary: the initial power-of-two
16-to-32 fixture also passed old shaders and is retained as a failed discrimination.
At 14-to-32, old shaders fail immediately with alpha 0.9995117 at ordinary pixel
[6,6]; the candidate passes all eight submissions/maps and 48 resolves. Partial
coverage remains fractional and the existing provenance/mass rules are checked.

Local verification uses pinned Rust 1.99, locked/offline dependencies, test profile
and aarch64-apple-darwin. Default: 168 CPU tests and strict renderer Clippy pass.
Cooked temporal/native-MetalFX configuration: 183 CPU tests, strict renderer Clippy,
the actual Metal feedback regression and 14 existing temporal cases pass. Seven
shader-cooker temporal contract tests pass. The pre-fingerprint cooked refusal,
excessive-precision test literal lint, initial zero-test diagnostic selector and
one invalid supervisor Cargo subcommand are retained; corrected executions are
separate records, not overwritten successes.

Raw baseline and corrected diagnostics each retain 48 scene frames, 96 submissions,
48 maps and 288 raw planes. All sampled opaque chart pixels in the corrected last
jitter cycle remain exactly opaque. Source/negative-control review receipts and
all original logs are retained outside Git in
`~/.codex/incant-evidence/reconstruction-opacity-packet-001/` (908 records).
The complete `evidence.tar.gz` SHA-256 is
`fc5e692a37b60ccadaec551e53062949561a4b4ccc9a7a8082ad9feac4b1aa6b`.
Copied executable identities are in each `artifact.json`; binaries stay external.
These numerical observations do not establish appearance, frame rate, memory
budget, other-device behavior or a phase gate. Phase-dependent clamp erasure,
moving reconstruction and low-scale entry artifacts remain under investigation.
