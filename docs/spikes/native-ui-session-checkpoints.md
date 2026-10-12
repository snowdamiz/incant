# Hosted native UI session checkpoints

This slice connects the bounded widget codec to the existing native session and
ordinary composed CPU player. A save captures committed native declarations,
script persistent values and timers, all text-input buffers and grapheme
selections, and logical widget focus at the same accepted tick. It does not save
an arbitrary JavaScript heap or silently restart an unopted behavior.

## Ownership and startup order

`incant_ui/native` adds an optional, authoring-free runtime dependency. A UI owner
created by `UiRuntime::for_world` retains the exact `WorldIdentity`, UI structural
generation and committed tick. `commit_for_world` and the corresponding router
commit advance that authority on every accepted sync, including unchanged paint.
Capturing another world's same stable IDs, a stale generation, or an older tick
refuses. Ordinary detached layout/widget users retain their existing API.

`PendingSessionCheckpoint` validates the trusted program/runtime identity, bound
native save and complete data-only widget section first. Its private typed
`PendingUiCheckpoint` validates membership, parents, kinds, text, selection and
section bounds without a renderer, filesystem or guest callback. The restored
world is the actual world later moved into the live session, not a clone.

The hosted factory calls `SceneHost::prepare_checkpoint`. The default refuses a
UI section before ordinary host preparation. `NativeRenderHost` restores a fresh
world-bound widget owner and neutral router, measures it with retained packaged
fonts, and creates matching paint and canonical draw input **before** calling the
consumer's `prepare`. Saved focus must be eligible in the new viewport. A changed
viewport can relayout; it cannot silently change saved text, selection or focus.
The pending session checks the restored payload and exact world authority before
initializing the VM, followed by audio setup. Candidate refusal does not mutate a
separate previously running game. Existing terminal tick failures remain terminal;
external effects from an arbitrary trusted consumer cannot be rolled back.

## Formats and compatibility

Native world save v7 includes committed `UiNode` declarations and sticky UI-used
history. Its flag is exactly 1; it uses the existing bounded scene and playback
sections, including animation graph state. Older saves cannot clear UI state in
an existing UI-used world. World saves alone remain distinct from hosted widget
state. The existing non-UI version selection and encodings v1–v6 are unchanged.

Session checkpoint v2 adds a fifth, explicitly bounded UI section. Its header is
132 bytes: the existing magic, version, ordinary-input policy, trusted program
fingerprint and source origin occupy bytes 0–80; five little-endian section lengths
occupy 80–100; SHA-256 occupies 100–132, covering bytes 0–100 and the body. Sections
are world, active attachments, persistent values, timers, and `INCUI001` widget
data. V1's 128-byte header, four sections and checksum framing remain unchanged
for non-UI sessions. V2 requires a UI-used v7 world, and v7 requires the UI section.
Hashes detect corruption and compatibility mismatch, not authentication.

The UI section is at most `80 + 48 * MAX_UI_NODES + MAX_SCENE_TEXT_BYTES` bytes;
the full session ceiling adds that amount to the existing `MAX_SAVE_BYTES + 2 MiB`.
Both outer and inner section/count/UTF-8/grapheme/numeric bounds are checked before
publication. No opaque unchecked public attachment or caller-selected bypass
boolean is introduced. The trusted build identity covers UI/text runtime source.

## Ordinary-load policy and remaining limits

Physical controls and the input clock start neutral. A fresh UI router, focus
owner and native context must be used; delayed old tokens cannot address it.
Logical focus and committed selection survive; pointer capture and presses are
discarded. Active preedit explicitly refuses until the application commits or
cancels it. Checkpoint creation never commits composition implicitly.

Plain `NativeSession::checkpoint_bytes`/load cannot omit mutable UI state: hosted
UI data requires `checkpoint_bytes_with_ui` and the pending typed-owner start
path. Existing `HostedGameSession`, `HostedOfflineGameSession` and CPU packaged
player APIs compose it without a new player mode. Backend settings may already
have caller-owned effects before the factory; an empty audio backend does not
mean an actual audio cursor was saved.

Physics-used, audio-used, static-module and development-module checkpoints remain
unsupported. GPU/window save/load CLI routes remain unsupported. OS IME state,
closures, provider/agent state and arbitrary external resources are not saved.
PCM output and a save file are not a multi-file atomic transaction. These changes
do not complete the Phase 1 full-session, device or Game UI gates.

## Validation

The affected CPU suite exercises native declaration/save round trips, stale
world/generation/tick authority, active preedit refusal, bounded malformed UI
sections, program mismatch, default-host refusal, consumer failure and live
candidate preservation. Actual retained font/paint tests preserve UTF-8 text,
reverse selection, logical focus, same-tick reparenting, persistent script values,
timers, and rational PCM positions. A fresh test process matches uninterrupted
final checkpoint bytes; that control compiles its fixed test behavior locally
and is not itself a shipping proof.

`tools/probes/native-ui-checkpoint-export.py` is the separate actual public
shipping proof. It authors a mixed mesh/world-text/UI project through public
commands, removes AudioSource through that same bus, verifies Undo/Redo and
journal reopen, and uses the ordinary exporter. It removes its authored inputs
and external debug source maps after the exporter removes its private build
storage. The copied CPU player then compares uninterrupted and resumed state,
checks fixed observation ticks and no-clobber output, and refuses malformed,
wrong-program/widget, over-limit, active-preedit and assertion-failed saves.

Final counts and immutable source/process identities are recorded in
[evidence/native-ui-session-checkpoints.json](evidence/native-ui-session-checkpoints.json).
The scoped independent source receipt is
[evidence/native-ui-session-checkpoints-review.json](evidence/native-ui-session-checkpoints-review.json).
The completed source and actual public proof review is
[evidence/native-ui-session-checkpoints-final-review.json](evidence/native-ui-session-checkpoints-final-review.json).
No GPU submission, native window, screenshot, appearance or performance gate is
claimed by these checks.
