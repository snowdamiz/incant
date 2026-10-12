# Native Rust/Wasm audio declarations and transport

This private Phase 1 slice appends `incant_audio_v1` operations 59–61 to the shared
registry. Existing operations 0–58, their records and input-v1 bytes are unchanged.
The native audio ancestor is 75503a64; immutable HUD ancestors 622c3a7b/1c513bf9
supply operations 56–58 and routed UI admission. This slice changes no scene,
world, session or audio checkpoint format.

`Tick::audio` copies the desired committed declarations at callback entry. It
reports stable scene/entity/clip/bus identities, exact u64 world ticks and
transport epochs, bounded bus parents, gains, source options, spatial positions
and listener rotations. It never reports an actual mixer cursor, backend voice
state or native handle. A completed one-shot may still have `playing = true`.
Retained copied bytes stay unchanged by commands queued later in the callback.
The frame is at most 32,240 bytes for 32 buses, 128 sources and one listener
(161 distinct owners; a listener may share a source owner). Optional IDs have
explicit presence bits, so the all-zero stable identity remains representable.
Rows are sorted and unique; inactive fields, roles, flags, finite/range checks
and bounded acyclic bus associations are canonical and checked before exposure.

`Tick::audio_commands` accepts an ordered batch of SetPlaying, Stop, Restart,
Seek, SourceGain and BusGain. Stop/restart/seek advance the exact checked u64
transport epoch; pause/resume does not. Source and bus gain requests validate
the original f64 range before conversion to the native f32 declaration. Seek
admits finite [0,3600] seconds; the audio host separately checks the captured
clip's actual duration when it synchronizes. A duration failure is terminal
audio-host refusal after native command acceptance, not a rollback claim.

Each batch has a 16-byte header and at most 1024 fixed 32-byte commands, a maximum
of 32,784 bytes. The complete canonical batch and remaining shared structural
command budget are checked before any row is staged. These commands share the
existing 1024-command, 65,536-call and 128-MiB-transfer callback quotas. The host
validates the complete declared output range and transfer budget before copying
any result. Authorized audio failures poison the transaction even when caught;
SDK scratch/argument errors cancel the callback too. Stale epochs reject before
poisoning a later callback. Existing exact world identity, callback lifetime,
final-owner validation and whole numeric/structural/log publication remain in
force. No audio backend is called by the Wasm import itself.

The production source and public probe are independently reviewed. Ten focused
parser/native boundary tests and the complete actual Wasmtime/wasm2c suite pass:
483 tests in 87 groups, no failures or ignored cases, eight-package all-target
Clippy and three fresh development processes. The real Rust guest matches all
186 declaration scalars and native committed state on each of 96 ticks. Actual
48-kHz stereo PCM matches the separate typed native mixer exactly for static
22,050-Hz and streamed 32,000-Hz clips, spatial movement, source/bus gains,
pauses, stop/restart and seek. Development module reload after tick 48 preserves
native transport without relying on guest globals.

Twenty-nine raw modes per host and ten Rust failure variants cover copied-buffer
bounds, canonical requests, original-f64 refusal, shared quota exhaustion,
stale epochs and caught errors with complete numeric/structural/log rollback.
The positive raw controls preserve epochs above 2^53, including 1024 ordered
restarts. The [durable host packet](evidence/native-wasm-audio.json) preserves
earlier test-only failures and qualifies the removed prepared inputs separately
from the 405 retained compiled artifacts and 17 guest sources. Final independent
execution review and the ordinary public source-free package proof are pending.

The public authoring/development preflight also passes: real WAV and Rust module
imports, atomic scene selection, Undo/Redo and journal reopen, then deletion of
the private guest/WAV sources and guest build. The captured-only Wasmtime game
matches all 17,856 declaration scalars, 248 bounded vector checks and exact native
PCM over 96 ticks. Six specific refusal diagnostics produce no public output.
The separate native oracle and headless runner build, oracle Clippy, SDK wasm32,
formatting and diff checks pass; [supplemental receipts](evidence/native-wasm-audio-build.json)
bind those inputs. The fresh ordinary shipping export is the remaining route.

Arbitrary guest checkpoints remain explicitly unsupported. This is local CPU
work; it provides no rendered approval, physical audio-device or hosted-phone
performance evidence, full SDK completion or Phase 1 gate approval.
