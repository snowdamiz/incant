# PCM streaming integration on main180

Accepted producer `49e18d9a` is composed with actual main180 on frozen source
`f47170c2`. All five Rust implementation/test files, the proof helper and Cargo.lock
match the producer byte-for-byte. The player manifest preserves the already-merged
physics defaults, forwarding and defaults-off dependency edges; the only new lock
edge is the already-pinned sha2 dependency.

All eleven integration commands pass: 69 player/mobile/window CPU cases and six
checkpoint/restart cases, all-target Clippy, static-module and mobile compilation,
normal dependency and physics forwarding guards, formatting and conventions.
The static check reuses a byte-verified trusted bulk fixture for compilation only.

A fresh ordinary shipping export runs eight actual process controls after its
private authoring, build storage and newly generated source maps are removed.
The full Core route completes 2,156 ticks and 1,724,800 stereo frames. Repeat
reports and PCM are exact; the first 1,000 ticks match the preserved buffered
player. Both old-bound and I/O refusals publish no partial output, existing output
survives no-clobber refusal, and completed failed observations retain diagnostic
PCM. All samples are finite and output is nonzero. The live package remains
byte-identical through these checks.

The [manifest](evidence/native-pcm-main180.json) retains 1,946 frozen inputs,
source comparisons, exact commands/streams, package/report identities and the
assertion-only collector. Independent current-main final review is pending.
The earlier producer failure/corrected helper evidence remains separate.

This is ordinary offline CPU validation, with no GPU, native-window interaction,
phone performance or rendered appearance acceptance. PCM and checkpoint
publication remain separate transactions; physics saves remain unsupported.
