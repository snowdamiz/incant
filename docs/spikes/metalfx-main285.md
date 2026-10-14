# Scoped Mac MetalFX path from main285

This default-off candidate starts at `1eca995cb36d9f9cada732dce7d52447569e63ac`.
It does not merge the private reconstruction branch. Bounded native correctness
controls passed on an Apple M5 Pro / Metal. A later real-material comparison
failed appearance review: thin lines disappear at 50 percent, silhouettes remain
stepped, and detail loses sharpness. This integration stays default-off and is
not an accepted rendering-quality mode. The built-in temporal upscaler remains
unfinished; this optional vendor path cannot satisfy that requirement.

The private producer `4bc810fe` established a raw-only command encoder, lifetime
leases and bounded targets. Its later capability correction `0c1864df` fixed a
concrete admission bug: Apple's output/input range `[1,3]` had been intersected
with input/output fractions, selecting the portable path at 75% and 50%. The
historical corrected CPU evidence `077f7c39` explicitly left corrected GPU execution
pending. Those commits and their rejected quality evidence remain private history.
This branch reuses only the reviewed low-level native/plan/lifetime contracts and
small coordinate/finite/depth boundary functions, with a new main-based graph.

`native-metalfx` is default-off on `incant_render` and `incant_render_gpu_host`.
Call `Renderer::enable_metalfx` before constructing a retained owner or host. The
retained `enable_metalfx` and `GpuRenderSettings::with_metalfx` take explicit
preference and depth support tolerances. The host wrapper also takes an explicit
initial output extent. `RetainedScene::prepare_metalfx_extent` and the matching
host method must be called before ordinary submission/resize, outside the
presentation loop. An unprepared extent has typed fallback/refusal; native
scaler construction never occurs inside retained submit. Selection is published only after the
single queue submission. Prefer mode reports the exact spatial fallback reason;
require mode refuses every unsupported case. The host wrapper preserves camera-cut
control, dynamic resolution within the prepared output extent, staged replacement
and screen-space UI.

The initial supported domain is retained opaque/MASK 3D, Metal, macOS 14.4 APIs,
queried device/format support, integral equal-aspect viewports and the actual
output/input scale range. World BLEND, sprites/text and active bloom select spatial
fallback or typed refusal. Screen UI is separate and remains supported. The
existing SSAO, shadow, light and opaque preparation order is retained. Cooked
shader configurations explicitly report unavailable; this candidate does not
silently introduce runtime WGSL into a cooked route. No existing default renderer
path or private reconstruction shader is imported.

The submitted batch is pre-wgpu, raw-only MetalFX, post-wgpu. It uses the renderer's
one queue and no foreign commit, wait or CPU texture copying. Pre initializes and
transitions every native target; post transitions the native output and applies
current-coverage normalization before the existing display transform. Raw guards
fence destruction; an Objective-C command-buffer association retains the scaler;
completion closures retain only safe wgpu texture owners. The native mutable owner
stays thread-bound. Failed/abandoned encoding dirties native history. Camera cut,
output/content resize, mode change, owner replacement or sequence gap resets it.
Automatic topology replacement inside host presentation is deliberately
unprepared (prefer falls back; require refuses). Explicit host texture/model
staging can prepare outside presentation, and validation rejects a staged
replacement superseded by an extent preparation.

Inputs are scene-linear premultiplied HDR, forward depth and current-to-previous
motion in internal pixels with both raster jitters removed. Explicit exposure and
pre-exposure are one. Scalers synchronously initialize only on explicit startup/extent
preparation; content-size changes reuse the fixed maximum textures/scaler. Engine
pack/normalize shaders compile only during explicit renderer startup. Apple's
private history/allocation costs are not measured. The retained engine targets
cost 33 bytes/output pixel plus 2048 bookkeeping per set, at most 4,194,304 pixels.
The per-owner 264 MiB + 4096-byte lifetime ceiling covers accepted, candidate and callback-retired
sets. It is separate from existing base-renderer allocations and is not a process
or device-memory budget.

Vendor alpha is not treated as coverage proof. Only admitted, fully covered,
depth-compatible current support uses vendor RGB. Mixed-depth/partial/clear
support uses the current reconstruction; thus silhouettes/thin geometry remain a
known quality limitation, not evidence of improved vendor reconstruction. Full
coverage canonicalizes alpha from contributing alphas independently of depth
compatibility. Final fractional alpha uses explicit integer binary16 RNE before
storage to preserve the zero-alpha/zero-RGB contract, following the independently
qualified boundary in private `a7d923f6`; no other experimental filter is imported.
The accepted eight-phase motion sequence is retained for this first contract
qualification. Apple's 32-sample recommendation at 2x is a later explicit quality
choice, not silently represented as implemented here.

Primary contracts: [Apple's temporal integration explanation](https://developer.apple.com/videos/play/wwdc2022/10103/),
[Apple's temporal scaler API](https://developer.apple.com/documentation/metalfx/mtlfxtemporalscalerbase),
and pinned local wgpu29.0.4 `api/command_encoder.rs`/`wgpu-core/as_hal.rs`.
The latter requires ordinary and raw encoding on separate encoders and leaves
ending/submission ownership with wgpu. Device textures must remain tracked and
belong to the exact command-buffer device. These sources establish API contracts,
not current-candidate execution, performance or appearance.

Executed CPU qualification used pinned Rust 1.99 arm64, locked/offline CI profile,
Cargo/Rayon 2. Source003 passed native renderer/GPU-host library tests (174 + 4),
default library tests (173 + 4), cooked-native library compilation, native/default
all-target strict Clippy and cooked-native library strict Clippy. GPU tests remained
ignored in these suites; the exact immediate executables were copied before other
feature builds. Formatting and diff checks passed.
The ignored native regression declares eight submitted frames, seven mandatory
native selections (including 50%/75%) and one explicit fractional spatial fallback,
plus a no-submit output-format refusal, cut/reset and replacement checks. These five ignored controls were subsequently run once under a separate root
source/binary/protocol grant; selection counters do not establish visual quality.

Source001 CPU qualification passed native 174+4/default 173+4 library controls
and cooked-native compilation, then stopped at strict native Clippy because a
budget observer was used only by tests. Source002 marks that observer test-only
and separates vendor prewarming from ordinary submission. Historical 001
source/logs/binaries remain retained; no GPU was executed.

The boundary controls use the actual fixed WGSL/pipelines with uploaded half-float
current/vendor color and full-precision motion. Seventeen cases cover opaque,
partial, clear, silhouette, mixed-depth full coverage, fresh/invalid motion,
NaN/Inf, and static/moving jitter. Integer exponent bits classify nonfinite values
before arithmetic, rather than relying on floating comparisons under WGSL's
finite-math rules. One bounded submission/map retains raw planes and exact shader
hashes. These test boundary policy, not Apple's internal reconstruction quality.
A separate ignored native encoder control creates and abandons two extent owners
without submission; it checks native lifetime/teardown without claiming execution.


The reviewed source002 prewarm successor was preserved without a separate build.
Source003 adds only two ignored tests and their registration: prepared-target
retention across abandonment of a real native encode/post-completion command
buffer, and host stale-extent staging/publication with target acquisition forbidden.
The host control also proves invalid extent preparation preserves the prior stage;
fresh publication does not present a frame or advance motion history.

The separately granted GPU sequence completed all five children without retry:
17 exact boundary cases; two abandoned raw buffers; prepared-owner abandonment;
stale/fresh host publication; and eight retained scene submissions (seven actual
MetalFX selections and one declared spatial fallback). Actual supported output/input
range was `[1,3]`, so 50% and 75% select native on this device. The sequence used
nine explicit `Queue::submit` calls, one mapped buffer and ten raw native encodes,
three of which were abandoned. Vendor-internal initialization work is not counted.
All owned children returned through the retained process-group cleanup helper.
The 87,040-byte boundary readback, exact assembled shaders, adapter, logs and
original failed source001 attempt are retained. No screenshots or timing gate ran.

Portable receipt: [manifest](evidence/metalfx-main285/manifest.json) and
[verifier](evidence/metalfx-main285/verify.py). Replay with
`python3 docs/spikes/evidence/metalfx-main285/verify.py`; optional `--originals`
checks retained external records, `--binaries` checks immediate executable copies,
and `--git-source` checks the frozen compiler-input inventory against the immutable
source commit. Readback verification is a boundary-policy check; it cannot decide
appearance or replace actual material/motion image comparisons. The later material/motion comparison confirms the structural edge limitation;
quality repair, vendor memory/performance measurements and cooked shader support
remain open. The [material/motion capture](metalfx-image-motion.md) records the
appearance failure explicitly.

Final composition on main289 (`0885bab3`) preserves the production implementation
and adds only the real-material fixture, its test-only crate alias and negative
evidence. A fresh CPU join passes 176 renderer and four GPU-host library tests
(56 actual-GPU tests remain ignored), plus strict native-feature all-target
Clippy. Pinned Rust 1.99 arm64, locked/offline CI profile and jobs/Rayon 2 apply.
This join executes no GPU work and does not relabel the producer's separately
identified actual Metal results. [Raw commands, outputs and executable identities](evidence/metalfx-main289/manifest.json)
are replayable with `python3 docs/spikes/evidence/metalfx-main289/verify.py`.
Checkpoint CI now compiles this optional feature and runs its CPU contracts;
the feature stays off by default and no required check is removed.
