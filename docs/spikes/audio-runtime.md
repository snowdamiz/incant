# Cooked audio, Kira mixing and script-driven export

Date: 2026-10-09. This is a Phase 1 increment, not completion of audio or its
device/performance gates. Computer use and local captures remain paused. No
speaker output, account or credentials were used.

## Import and retained assets

The shared importer accepts project-local WAV and OGG Vorbis using Symphonia
0.6.0 (locked 0.6.1 components). Mono/stereo at 8–192 kHz cooks to stereo
little-endian f32 PCM; mono duplicates both channels. Source grants, stable IDs,
provenance, watch/reimport and Undo use the existing command bus. Texture settings
on audio fail before publication. Cooked playback requires no original source.
Other codecs, containers and multichannel layouts fail explicitly.

Manifests record the decoder-version/source fingerprint, source dimensions,
frame count, full content hash and hashes for 8,192-frame chunks. Loading checks
all content with bounded reads, finite samples and exact byte length. Later chunk
reads recheck their hash. Retained file handles survive atomic reimport; in-place
corruption reports an error. Temporary files publish PCM before the manifest.
Rejected commands may leave unreferenced cook-cache files, not authored assets.

Limits: source files use the existing 128 MiB grant; decoded PCM is at most
256 MiB per clip; manifests are at most 512 KiB, decoder packets 65,536 frames,
and cooks one million packets. AssetStore counts audio manifests in its resident
budget; PCM stays file-backed. Mixer buffers, temporary decoder allocations, OS
file cache and externally retained versions are outside that published-set cap.

## Mixer and gameplay semantics

`incant_audio` uses Kira 0.12.5 for mixing, nested buses and spatial tracks. Limits
are 32 buses, 128 sources and one listener. Bus graphs reject duplicate/missing IDs
and cycles. Static PCM is shared by content hash with an aggregate 8,388,608-frame
(64 MiB) cache limit, plus bounded transient loading allocations. Use `streaming`
for music beyond the static budget.

Native streaming uses Kira's decoder worker with independent seekable cursors.
Offline streaming retains two chunks (128 KiB PCM) per voice plus bounded read/
conversion scratch. Kira's public four-point Hermite interpolation runs
synchronously, avoiding gaps when export outruns a worker. Device callbacks never
select this synchronous path. Both use Kira routing and spatialization. Initial
resampler transients can differ between static and streaming modes; bit-identical
output between modes is not promised. A bounded error queue preserves failures
when stopped voices are removed.

Typed AudioSource, AudioBus and AudioListener schemas validate audio asset/bus
references, ranges, spatial transforms and a unique listener. Generated TypeScript
exports these types. Scripts use ordinary `api.command({op:'set_component', ...})`,
with the same atomic validation as editor/agent commands.

`SceneAudio` projects committed state onto the mixer. Spatial positions include
ancestors; listener rotation composes ancestor quaternions independently of scale.
Kira's attenuation is linear in decibels, from full volume at `min_distance` to
silence at `max_distance`; its ear directions are angled 22.5 degrees from the
head's lateral axis. Gain/pan/rate edits keep the cursor. `playing=false` pauses;
true resumes. An ended one-shot stays ended until a false-to-true edge. Changes to
clip, start position, routing, stream/loop or spatial settings restart that source.
Removing it stops it. Bus gain changes live; bus creation/removal/reparenting
requires restarting playback and otherwise fails explicitly. Runtime sync errors
are terminal, so partially applied audio ticks cannot continue silently.

## Headless host

```
incant_headless import game.incant.json sounds/music.ogg
incant_headless play game.incant.json --ticks 60 --compiled-script behavior.js --audio-output mix.wav
```

`--audio-output` exports without a GPU or device, defaulting to stereo float WAV
at 48 kHz; `--audio-rate` accepts 8–192 kHz. Reports include frame count, peak,
samples at full scale and PCM SHA-256. Kira clamps its final mixed output to
[-1,1]; the WAV preserves that output. The full-scale count is not a pre-clamp peak
meter. A 60-tick/60 Hz run writes exactly 48,000 frames. Cumulative integer division
prevents rounding drift for non-divisible sample/tick rates.

Audio sync follows each committed script tick and fills that tick's interval.
Zero ticks produce an empty valid WAV. Export has a 256 MiB PCM limit, staged files
and no-clobber publication. Existing outputs, log/save conflicts and reserved
frame/report names are rejected. Script/audio failures do not publish the staged
WAV or game save. Completed runs with failed assertions retain diagnostic audio
while withholding the save. Files are individually atomic, not a cross-file
filesystem transaction.

Logical saves retain declarations, not playheads, resampler history or pending
sounds. Export after `--load-save` restarts playing sources at their component
`start_seconds`. Exact audio continuity across saves is not claimed.

## Verification and open work

Twenty new tests cover real Vorbis signal error, chunk boundaries, source-free
loading, corruption/truncation, watch/reimport/Undo, atomic graph validation,
bus gain, pause/resume/stop, resource rejection, worker errors, deterministic
batch-independent offline loops/rates, inherited positions/listener rotations,
terminal errors, CLI script control, float-WAV reimport, output conflicts and
failure publication. The full workspace passed 214 tests before the final
voice-capacity case; the audio/headless suites were rerun after that case and the
report naming correction. The distinct passing Rust total is 215. Workspace
Clippy, 315 UI tests/build, five tool tests, generated contracts, strict TypeScript
and the native release package pass.

`tools/probes/game-audio.py` imports WAV and real Vorbis, creates buses/sources over
durable RPC, compiles strict TypeScript, deletes both sources and exports two
one-second runs. Their PCM hashes match exactly. Left RMS is 0.2441395, pause RMS
zero, resume RMS 0.2441395. A -6 dB music bus change measures ratio 0.5011254
(expected 0.5011872; different windows of the lossy Vorbis signal account for the
small difference). Authored JSON/journal hashes stay unchanged. Existing WAVs
cannot be overwritten.

macOS CPAL, iOS simulator device-backend and WASM library compilation pass. No
device was opened, no iOS app launched and no local pixels captured. Hosted source
and Windows/Linux jobs run the public probe and device-backend compilation;
platform CI checks Android/iOS device and WASM compilation. Hosted results for
`246a380` passed all twelve checks, including Windows retained-reader repair.
The integration with merged timers passes 224 Rust tests, workspace Clippy,
315 UI tests/build, five tool tests, generated contracts, strict TypeScript and
the native release package. The public audio probe also drives pause/resume/gain
from timer callbacks: PCM matches update-driven control exactly. A save at tick
25 retains the paused source and restores callbacks at ticks 31 and 41; the
resumed source stays silent until its deadline and the final logical state
matches uninterrupted play. Public timer/save/input probes pass. Updated hosted
checks and merge remain pending. Compilation is not playback evidence.

Open: native editor controls/playback wiring, browser resources/streaming and
autoplay, physical device output/latency, interruptions/device switches, mobile
audio sessions, cursor saves, dynamic bus topology, DSP/effects, performance
workloads and the Core Sample. Browser streaming fails explicitly. Offline tests
do not satisfy these gates.

Machine-readable evidence: [audio-runtime-2026-10-09.json](evidence/audio-runtime-2026-10-09.json).

### Windows cache repair correction

The first Windows desktop run at `0f7390f` failed the corrupt-cache repair test:
`tempfile::persist` could not replace PCM while an existing reader held it open.
The cooker now uses the standard library's atomic rename, which includes the
Windows POSIX replacement fallback absent from tempfile's MoveFileExW path (see
[Rust rename documentation](https://doc.rust-lang.org/std/fs/fn.rename.html)).
The temporary-file guard still cleans up failed publication; no unlink or
in-place rewrite is used. The regression also imports identical PCM under a
different source fingerprint while both readers remain alive. Repaired readers
stay valid, and the retained corrupt reader continues rejecting its old bytes.
The three local asset-audio tests and asset Clippy pass. Hosted Windows run
38021329759 (job 114122758280) passed the corrected asset tests, public audio
probe, native build and GPU checks at `246a380`.
