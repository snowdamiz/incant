# Bounded offline PCM streaming

The ordinary packaged player writes accepted PCM ticks to a same-directory
staging file and hashes the little-endian float bytes incrementally. It no longer
retains an entire run in a sample vector and a second byte vector. This removes
the former one-million-frame admission limit without growing capture memory
with duration. CLI and the existing mobile library entry share this path; no
input, package, checkpoint or report version changes.

The output working storage is `2 * ceil(sample_rate / tick_rate)` floats and an
8,192-byte encoding array. At 48 kHz / 60 Hz this is 14,592 bytes; at the admitted
maximum sample rate and minimum tick rate it is 1,544,192 bytes. These are derived
output-buffer bounds, not whole-engine heap, RSS, driver or device measurements.
The existing 3,600-tick, 8–192 kHz and 1–240 Hz scene limits remain. They imply at
most 691,200,000 stereo frames / 5,529,600,000 bytes on disk. Checked counters and
per-tick output size validation apply before each write. No tick performs a full
PCM projection or grows the output buffers.

Only completed output is synced and published using create-only atomic file
publication. Write, simulation, observation-admission, final-save preparation or
restart-construction failure drops staging. Existing output files are preserved.
An I/O error makes the encoder terminal. Abrupt process termination may leave
a hidden staging file; it does not publish a partial destination. This is not
universal power-loss recovery, and PCM plus a requested checkpoint remain two separate transactions:
a checkpoint publication failure may follow successful PCM publication.

Completed failed assertions retain their established diagnostic behavior. With
no requested save they publish diagnostic PCM and a failed report, then return
an error. With a requested save they publish the failed report and withhold both
PCM and save. Incomplete runs never publish a PCM prefix. Fractional resumed
clocks and whole-session restarts keep their existing accepted host/output
chronology; file frames are counted from actual accepted per-session samples.

Source review is accepted for source005, including the one-line Clippy iteration
correction. The player/window/mobile CPU suite has 69 passing tests, plus six
real checkpoint/restart tests; all-target Clippy, the runtime dependency guard,
formatting and whitespace checks pass. The static-module/render-input build
also passes with the retained trusted Wasm bundle. The original fixture compile error and
Clippy failure are retained separately from their corrected runs. Tests cover
actual native embedded movement beyond the old cap, a late guest failure after
one million staged frames, diagnostic output, no-clobber cleanup, partial writer
failure, exact byte/hash chunking and fractional checkpoint continuation.

The fresh ordinary Core package passes the complete 2,156-tick route at 48 kHz:
1,724,800 frames / 13,798,400 bytes, pickups at 127/508/1266/1842 and victory at
2066. It uses a private exact copy of Claude's f5 authored project and route
without art, controller or layout changes. The repeat report and PCM are exact;
the first 1,000 ticks match the prior buffered player in logs, observations and
PCM. The prior player refuses the same full duration. Completed failed assertions
retain exact diagnostic PCM, existing output survives no-clobber refusal, and a
missing output parent fails without publication or a staging leak. All eight
process controls pass after deleting the proof's authoring/build/debug maps.
The PCM contains finite, nonzero samples; this is a numerical audio check.

The first export succeeded, then its helper rejected the old-bound diagnostic
because it expected Display formatting instead of Rust's actual Debug output.
That red run is retained. The corrected exact diagnostic assertions passed
against the identical package in a fresh output directory; no recook or
production change occurred. The [evidence manifest](evidence/native-streaming-pcm.json)
binds seven production/test inputs and the original 1,770-row engine/tool freeze
(1,769 unchanged inputs plus the explicitly corrected proof helper),
compressed/raw logs, process commands, reports and all package file identities.
Final independent source/execution review is accepted; the [review receipt](evidence/native-streaming-pcm/reviews/native-streaming-pcm-final-20261012.json)
binds the preserved audited manifest. Publisher integration is pending.
Native-window, GPU, physical-device, performance and rendered appearance
evidence are outside this slice.
