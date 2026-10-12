# Native animation crossfades

Phase 1 implementation evidence, not an animation or performance gate approval.

The native ECS playback component now owns a bounded two-input crossfade. The
destination is the existing clip/rest sample. A transition captures the outgoing
sample and its speed, playing and looping flags; committed fixed steps advance
both numeric clocks and the destination contribution. No model, pose, document,
JSON or command-bus data enters this per-step calculation.

`AnimationControl::Crossfade { target, duration }` selects a checked clip/rest
sample over a finite duration from zero through 60 seconds. Zero is immediate
selection. A nonzero request while a transition is active explicitly refuses the
entire structural batch: interruption of a blended pose needs a retained graph
pose, which this slice does not pretend to implement. Seek and Select cancel a
transition. The master playing flag is preserved when starting one.

Pausing freezes both clocks and the fade. Later speed and looping controls affect
the destination; outgoing settings remain captured. Starting while paused captures
a still outgoing pose, which remains still when the master resumes. This differs
from pausing an already-running crossfade, whose captured source resumes too.
Reaching the destination's endpoint early holds that endpoint while the fade
finishes. A rest destination remains live; a completed nonlooping clip stops once
the fade has finished. Fade duration advances independently of playback speed.
The accumulated floating-point error bound prevents integral-tick transitions
from lasting one extra frame; its tolerance stays below one nanosecond within the
supported 60-second/240-Hz limits.

`AnimationState.transition` exposes the captured clock, elapsed time and duration.
`blend_sample()` returns the outgoing sample plus the destination weight for a
canonical render consumer. This commit does not yet connect that observation to
render bindings, TypeScript controls or the Wasm SDK. Those are separate active
integration slices. It does not add authoring animation graphs, general blend
trees, retargeting, state machines, IK or root motion.

## Checkpoint compatibility

Only an active transition requires world-save version 5. Each animation row grows
from 36 to 72 bytes: its existing identity/sample/flags/speed prefix is unchanged;
the suffix stores outgoing clip/flags, a zero reserved byte, outgoing seconds and
speed, elapsed time and duration. An inactive suffix is entirely zero. Version 5
must contain at least one transition. When all transitions finish, serialization
returns to unchanged version 4 rows; scenes without animation still use version
3. Cooked scene encoding is unchanged.

Readers retain versions 1–4, validate exact membership, canonical IDs, flags,
finite ranges and the original cooked-scene binding before publication. Fresh
load and in-place restore preserve active and inactive rows exactly. Session
checkpoint admission accepts v5 alongside v3/v4, preserving script persistence,
tick and original program/scene binding checks. A save remains explicit cold work,
not a per-frame snapshot.

## Validation

The accompanying evidence receipt records frozen source hashes and actual local
test logs. Behavior tests cover outgoing loop/stop, reverse destination speed,
pause/resume, a paused start, rest and zero-duration clips, a stationary
destination, exact endpoints, high-rate long transitions, failed ticks, atomic
command refusal, mixed-row save replay, checksum-valid malformed saves and
ordinary fresh/in-place script-session checkpoint replay. Existing runtime and
scripting suites check older formats and the shared transaction paths.

These are local CPU tests. No rendered appearance, graphics/device benchmark,
IME, phone, frame-pacing or whole-phase completion is claimed.

## Combined actual-main122 integration

The original core, retained consumer and SDK commits are preserved in one current-
main union. Production sources merge without conflicts. The integrated checks
pass 367 ordinary CPU tests (71 GPU cases ignored) and 313 actual Wasmtime/compiled
wasm2c tests in 54 groups, with all-target Clippy, wasm32, generated contracts,
tooling, strict SDK types, formatting and dependency guards. These test scopes
overlap; their counts are not summed as distinct tests.

The current-lock public QuickJS export is rebuilt once with the reviewed probe.
After removing authoring/private build inputs and the proof-owned external debug
source map, seven successful player processes cover three identical 100-tick
replays and active/paused world-save-v5 continuations. Two failures withhold both
PCM and save output. All 62 clock/snapshot checks, final save bytes, accepted logs
and PCM continuations pass. [The integration receipt](evidence/native-crossfade-main122.json)
retains exact sources, independent reviews, logs and shipping-player identity.

The original proof's external debug-map limitation remains in its historical
receipt. The text export probe receives the same narrow map-removal correction,
with separately reviewed same-player replay evidence. No production behavior or
phase gate is inferred from those tooling corrections. Crossfade GPU appearance,
public Wasm project export, full static VM checkpoints, blend graphs/state machines,
IK/root motion and device/performance acceptance remain open.

The [final independent integration review](evidence/native-crossfade-main122-root-review.json)
accepted the frozen source and exact current-lock process receipts without findings.
