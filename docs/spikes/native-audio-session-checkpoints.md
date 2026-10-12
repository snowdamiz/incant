# Ordinary offline audio session checkpoints

This slice resumes the real static and streamed voices of the existing composed
CPU game. It preserves script persistent values/timers, native declarations,
optional hosted UI, and the audio owner's exact output clock at one accepted
boundary. It does not serialize a VM heap, a device callback, or raw physics.

## Retained state and ownership

`SceneAudio::checkpoint_for_world` requires the exact native world identity,
incarnation and committed tick. It compares accepted source/bus/listener caches
with native declarations before emitting bytes. Static and streamed voices retain
monotonic birth IDs, original mixer insertion order, current play state, fractional
source position and gain/pan/rate. Static voices also retain the unchanged Kira
resampler's four source-history frames and drain count. Pending replacements,
stopped retired sounds still occupying mixer capacity, and already removed
handle-only tombstones are distinct states. Nested bus topology and gains, spatial
source configuration, listener pose and master gain are preserved.

A missing current listener retains its actual historical mixer pose; removing an
AudioListener does not reset Kira's listener. Restoring an invented origin would
change a subsequent listener re-add interpolation. Desired source play state must
agree with playing/paused transport. A naturally completed one-shot may remain
stopped or have no voice while its authored playing flag remains true.

Initial tick-zero queues and master suspension are supported. The adapter saves
both independent pending pause/resume command bits, the settled master state,
and logical game suspension. Kira consumes pause then resume, so one logical
boolean cannot reconstruct its queues. A fresh paused master is constructed
paused before any processing; startup queues remain queued, while a previously
processed owner promotes reconstructed objects with a zero-frame backend call.
No sample is consumed to approximate a restore position.

## Checked construction

The session decoder first validates program/runtime identity, source origin,
native world and typed UI/audio sections. The sealed pending audio record binds
the actual restored world. It validates source/voice membership, birth order,
counts, transport kinds, numeric ranges and the exact rational clock
`floor(world_tick * output_rate / tick_rate)`. Actual packaged clip IDs,
fingerprints, content digests, rates and frame counts must match the complete
saved catalog. Static history is checked against the actual immutable PCM.

Only after these checks does the offline factory reconstruct its private audio
candidate. Host/UI preparation follows using the same restored world; guest
initialization is last. The pending owner checks that the reconstructed audio
checkpoint is byte-identical before starting the VM. Rejection leaves a separate
live session untouched. Candidate construction is offline and cannot emit to a
physical device. A trusted consumer's external side effects are not generally
reversible.

Static PCM is decoded once per content digest and shared by voices. The aggregate
static resident admission remains eight million stereo frames, including retired
voices; the save does not embed PCM. Catalog/source/retired/voice counts are each
bounded at 128, buses at 32. The audio section ceiling is
`176 + 128 * 92 + 128 * (25 + 8 + 240)` bytes. Every length/count/tag/finite-value
check occurs before publication; checksums detect corruption and compatibility
mismatch and are not authentication.

## Formats and compatibility

World save v8 retains audio declarations and sticky audio-used history. Its flags
include audio bit 1 and optionally the existing UI bit 0. Existing world versions
1–7 remain byte-for-byte selected for worlds without audio history. A world-only
save carries declarations, not an independent mixer; the session refuses to omit
the required audio owner.

Session v3 has a 136-byte header: existing magic/version/input policy/program
fingerprint/source origin through byte 80, six u32 little-endian lengths through
104, and SHA-256 through 136 covering the prefix plus body. Sections are world,
attachments, persistent values, timers, optional UI, and required audio. Existing
non-audio session v1/v2 framing and bytes remain unchanged. The audio section uses
`INCAUD01`, version 1, checked total length, checksum, binding/clocks/flags,
catalog, source ownership, retired order and transport records. The internal
144-byte static transport is `KIRAST01`, not a Rust object/arena dump.

Compatibility includes the actual trusted runtime/audio/assets source and pinned
Kira adapter. This conservative build identity does not attest every system
header or permit caller-supplied program identity. The Kira 0.12.5 vendor retains
the original static processing math; its optional adapter copies explicit,
checked settled state and adds a fresh-track paused constructor hook. It does not
expose arbitrary Kira graphs, effects, modulators, tweens, reverse playback or
partial loop-region serialization.

## Ordinary load and remaining boundaries

Physical input and UI router/native owner tokens restart neutral; saved committed
widget text/selection/logical focus and script opt-in state survive. Module
globals/closures remain ephemeral under the existing explicit script contract.
Active UI composition still refuses. Physics-used worlds, static/development Wasm
module checkpoints, device-driven audio restore, and native-window/GPU checkpoint
CLI routes remain unsupported. Audio effects and other capabilities absent from
the current native mixer are not silently reconstructed. General standalone
mixer mutations must reach an accepted processing boundary; this does not hide a
second simulation route.

PCM publication and save publication are not a multi-file transaction. These
changes do not complete the full Phase 1 save, device, performance or Game UI gate.

## Verification

Actual Kira sound controls, retained SceneAudio controls and composed VM tests
compare output samples and final checkpoint bytes for static/streamed mixed
rates, fractional cursors, looping, pause/resume, gain/bus changes, queued
replacement/retirement and future voice insertion. Tick-zero and suspended tests
exercise both queued and processed master state. Malformed/catalog/clock/program
and owner mismatches reject before live candidate publication.

`tools/probes/native-audio-checkpoint-export.py` authors through public commands,
checks Undo/Redo/journal reopen, and invokes the ordinary public native exporter.
The fresh CPU packaged player runs after authored inputs, build storage and
external debug source maps are removed. It compares uninterrupted 40 ticks with
13 saved plus 27 restored ticks, including real PCM concatenation, script/timer
state, retained poses and UI state at a different viewport. It also checks
checksummed wrong audio catalogs/clocks, wrong program/widget, corrupt and
oversized input, failed assertions, active preedit and no-clobber publication.
The final affected suite passed 410 tests across 48 result groups with no failures
or ignored tests; scoped all-target Clippy passed. All 99 Kira library tests
passed. Minimal audio, compiler-disabled script/static-module, no-UI composed,
wasm32 audio and device compile checks passed, along with the authoring-free
normal dependency graph. The vendor Clippy exception is limited to two unchanged
upstream `derivable_impls` sites; the device build also retains its unchanged
upstream `custom_device` warning.

The 9,202,464-byte fresh shipping player passed four positive runs. The final
baseline/restored/repeated saves share SHA-256
`ac194193c67ef51eab79d3be7f9220e2b0bf57a70b9c09a0c66a44c479feb04b`.
Its 256,000-byte baseline PCM equals 83,200 split bytes plus 172,800 resumed
bytes. Source clip rates remain 22,050 and 32,000 Hz; output is 48,000 Hz.
Separate composed tests cover 44,100 Hz output at 144 ticks/second and exact
fractional output phase.

The original fresh-run helper checked only nonzero negative exits and is retained
as preliminary evidence. A corrected helper reran all four successes and nine
refusals against that same immutable package, matching the precise diagnostic
and exit code, retaining argv/cwd/environment keys, and checking all 16 actual
player/data files before and after. It does not override HOME. The original
exporter's pre-existing isolated startup environment is recorded separately; it
was not changed by this slice. A separate author-only run verifies the corrected
command/RPC receipt capture; it is not claimed as the original package's journal.

See [evidence/native-audio-session-checkpoints.json](evidence/native-audio-session-checkpoints.json)
for exact sources, logs, commands, preliminary and strict process receipts, and
all remaining limits. No GPU, window, screenshot or audible
physical device is used.

The independent final source/process audit is retained in
[evidence/native-audio-session-checkpoints-final-review.json](evidence/native-audio-session-checkpoints-final-review.json).

## Integration with main #145

The accepted audio sources compose with current public Wasm physics, managed IME
and numeric animation additions. The Cargo merge preserves both the existing Tao
vendor and this pinned Kira adapter. All 160 other accepted source/build inputs
match exactly; only those two Cargo files differ from the original accepted
packet. After merging actual main #145, all 1672 frozen inputs remain identical
to the tested tree; subsequent differences are documentation and status only.

The integrated suite passes 422 affected CPU cases in 50 groups and all 99 Kira
cases, with no failures or ignored tests. Scoped all-target Clippy, static-module
UI/audio and compiler-disabled checks, device compilation, wasm32, normal runtime
dependency checks and formatting/conventions pass. The original named upstream
`clippy::derivable_impls` allowance remains limited to the Kira check.

The fresh ordinary public exporter builds a shipping player with SHA-256
`8650d5fc031c8810520660f68b12894feb05c1942c768a543d49d45c6c22165b`. Four source/build/map-deleted
processes preserve widget/script/timer state, real static/streamed audio and exact
final checkpoint bytes. Uninterrupted PCM equals the concatenated pre-save and
resumed PCM, and both resumed processes emit identical PCM. Nine controls match
precise expected refusals; no-clobber preserves the existing save. PCM can already
be published before that save refusal, so this is not a multi-file transaction.
Sixteen retained runtime package files remain unchanged across the controls.

[Current integration evidence](evidence/native-audio-main145.json) binds all
literal commands, source inventories, compressed logs, actual package identities,
save/PCM hashes and process receipts. The actual package remains under
`~/.codex/incant-integration-evidence/audio-main145-001/public-export`.
This is CPU ordinary-load evidence; the unsupported routes and gates above remain
unchanged.

The [independent current-main review](evidence/native-audio-main145/root-final-review.json)
reverified every frozen input, retained log and actual package/output identity,
including exact saves, PCM continuation and refusal diagnostics.
