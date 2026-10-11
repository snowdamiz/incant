# Native TypeScript audio controls

Native QuickJS behaviors can now queue `commands.setAudioPlaying(source, bool)`,
`commands.setAudioGain(source, db)` and `commands.setAudioBusGain(bus, db)`.
These update existing typed runtime audio declarations through the shared
`FrameCommands` transaction. The retained `SceneAudio` host observes the accepted
world after the native script succeeds. Scripts never receive mixer/device handles
or an authoring document, and no legacy gameplay path is involved.

## Supported behavior

`setAudioPlaying(false)` pauses an existing voice while retaining its cursor.
`true` starts an unstarted source or resumes a paused voice. This is desired
playing state, not a replay instruction: repeatedly setting true on an ended
voice does not restart it. False then true must reach separate mixer sync points
to create a new playback edge; both values in one transaction collapse to the
final desired state. A paused source that never started has no cursor/voice yet.
There is no stop, seek or restart operation in this slice.

Source and bus gains are finite values in [-80, 24] dB. Source gain preserves clip,
cursor, routing and every other declaration field. Bus gain preserves the startup
bus graph. The script adapter validates the original JavaScript number before
converting to f32, so an out-of-range number cannot become valid through rounding.
IDs remain canonical uppercase ULIDs; wrong arity, types and invalid values fail
explicitly. Gain calls require the corresponding existing source or bus.

The shared typed `StructuralCommand::ControlAudio` is also available to native
callers. Controls use each entity's final audio declaration, and controls for that
entity compose in submitted order. An ordinary native batch can install a source
and control it atomically. Missing/despawned entities, removed declarations, wrong
component roles, bad gains or an invalid final source/bus graph reject the entire
batch and retain its queue. No earlier control, numeric component command or
structural mutation is published on rejection.

Audio-only and component-only batches retain their narrow preflight paths; the
new helper copies only touched bounded audio declarations. It does not copy the
world or make a per-tick audio snapshot. General structural batches use the same
final-state validation before their existing publication boundary.

## Lifetime and transaction boundaries

The three script methods use the existing tick-local command capability and its
shared 65,536-command limit. Retained functions expire and never reactivate.
Argument errors caught by JavaScript still poison that transaction. Native final
validation, caught parsing errors, quota exhaustion and later script failure use
the existing numeric rollback and command discard path. No mixer command is sent
while guest code runs. The new parser participates in the cooked-script
compatibility digest; trusted cooks must match the engine identity.

The composed game session already synchronizes audio only after successful native
simulation/script acceptance. Rejected script transactions cannot publish their
audio controls. An accepted simulation step, asynchronous/device output and later
backend failures are not rewindable: the existing composed host fails terminally
and silences unpublished offline output. This change does not claim whole-device
or pre-simulation rollback, and it does not add another mixer sync path.

## Verification and limits

The [evidence record](evidence/native-script-audio-controls.json) pins source and
logs on top of the independently reviewed 2D query commit. All 254 scoped
runtime/script/game/audio tests pass, with zero ignored tests. Nine new tests
cover control ordering, native source installation/control, invalid final graphs,
wrong source/bus roles, numeric/structural rollback, f64 range boundaries, retained
capabilities and shared queue exhaustion.

Actual `OfflineGameSession` tests render static and streamed cooked clips through
Kira. Six ticks for each mode compare every PCM scalar with a separate retained
`Mixer` receiving the equivalent pause/resume and gain controls. The same voice
is kept across the reference pause/resume, so exact resumed samples check cursor
continuation. Additional tests check start/ended-voice edges and silence plus
terminal session behavior after script rejection. These are real offline mixer
results, not a mocked audio backend or a speaker/device playback claim.

All-target Clippy passes for runtime, script, game runtime, audio runtime and
headless. The headless build, compiler-disabled script check, strict SDK/template
TypeScript, generated SDK/bridge checks, formatting/conventions and normal-runtime
dependency guard pass. All 61 fresh schema/agent files remain byte-identical; this
slice adds typed native primitive methods without changing document schemas or
binary scene formats. Independent read-only production/test review by
`/root/runtime_review` found no actionable blocker.

This remains a bounded native QuickJS control surface. It does not add stop or
restart transport, clip loading, source creation/routing, bus topology edits,
audio-state queries, Wasm/static/browser parity, complete session saves, editor
controls or full Core Sample integration. Existing audio-used save refusal stays
intact. No GPU, rendered appearance, physical audio device, shipping/profile
performance, phone or phase gate is claimed.
