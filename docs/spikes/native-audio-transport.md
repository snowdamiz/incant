# Native audio stop, restart and seek

TypeScript behaviors can queue `commands.stopAudio(source)`,
`commands.restartAudio(source)` and `commands.seekAudio(source, seconds)` through
the existing typed native command transaction. These complement the existing
pause/resume and gain controls; scripts never receive mixer or device handles.

## Playback behavior

Stop sets the desired position to zero and pauses the source. Restart sets it to
zero and plays it, including an already completed one-shot. Seek accepts a finite
position from zero through 3,600 seconds and preserves the desired playing state.
The retained audio host checks the position against the actual clip duration.
A paused seek stays silent until playback resumes.

Each explicit transport request advances a checked `u64` epoch, even when its
position and playing flag equal the previous values. The retained mixer consumes
that epoch so repeated restarts do not disappear as identical declarations.
`setAudioPlaying` keeps its previous pause/resume meaning: repeated `true` does
not replay a completed voice. Controls for an entity compose in submitted order;
a batch publishes its final declaration at the existing mixer synchronization
boundary, rather than producing intermediate device output for every command.

Wrong source roles, missing entities, invalid arguments and epoch overflow reject
the native batch atomically. The script methods use the existing tick-local
capability, shared command quota, sticky parsing failure and numeric rollback.
Retained functions expire. A later backend error remains terminal; this does not
claim rollback of an accepted simulation step or already emitted device samples.

## Scene and checkpoint compatibility

The authored schema is unchanged and newly cooked declarations start at epoch
zero. Scene v19 is selected only when at least one source has a nonzero transport
epoch. Older scenes decode to epoch zero, and unchanged declarations retain the
existing scene encoding. The epoch is encoded as an integer without conversion
through JavaScript's number representation. Checked scene admission budgets the
additional eight bytes per source.

Audio checkpoint v2 similarly adds the consumed epoch only when needed; existing
zero-epoch checkpoints retain v1 bytes. Restore checks the consumed epoch against
the restored native world before constructing a new owner. A save cannot silently
bind a mixer that has not consumed the world's latest transport request. The
outer session format is unchanged. Static resampler history, streamed position,
voice retirement order, bus state and fractional output clocks remain covered by
the existing checked checkpoint representation.

## Verification

The focused real-mixer tests pass for static and streamed clips, including
completed one-shot restart, stop, paused seek, resumed playback, transport binding
and exact PCM/checkpoint continuation in a fresh owner. The six-package CPU suite
passes 530 tests in 75 result groups with no failures or ignored tests; its
all-target Clippy check passes. Focused runs overlap that suite and are not added
to its count. Independent source review accepted the production changes and the
two final test corrections. Additional checks pass for the GPU/static-Wasm
adapters, compiler-disabled script runtime, wasm32 audio runtime, strict SDK
types, generated contracts and the normal-runtime dependency graph.
[Retained evidence](evidence/native-audio-transport.json) records these source
identities, exact commands and original failures.

The first new checkpoint fixture failed because it attempted capture before the
mixer consumed its queued change. The fixture now renders at the accepted
boundary; the original failure remains in the evidence. No production behavior
was changed to bypass the boundary check.

The opt-in `--transport` variant of the ordinary audio checkpoint export proof
adds stop/paused-seek/restart, repeated one-shots and a checksummed wrong-epoch
refusal. The fresh ordinary shipping export passes in four separate processes:
40 uninterrupted ticks, a 13-tick checkpoint, and two fresh 27-tick continuations.
Their final save bytes match exactly, as does the concatenated PCM. Paused ticks
are silent; resumed/restarted clips emit samples. Ten targeted refusals preserve
the documented checkpoint and output boundaries. Authoring inputs, project cache,
private build storage and source maps are absent before the copied player runs.
The independent final execution audit accepts the retained package and results. This record does not claim
physical audio playback, browser/Wasm SDK parity, reference-phone performance,
Core Sample music integration or a completed phase gate.
