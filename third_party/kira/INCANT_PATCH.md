# Incant offline checkpoint extension to Kira 0.12.5

Published crate archive SHA-256: `aae7115ef1161313b8f71f49487702a662ab333bd2a47bf69af410b69112a519`.
MIT OR Apache-2.0 licenses and original sources are retained. Cargo registry
markers are omitted. The optional `offline-checkpoint` feature adds a bounded
static-sound continuation adapter around the existing StaticSound. Its sample
processing math remains unchanged.

The adapter copies checked explicit fields: fixed gain/pan/rate, settled playback
state/fade, integer/fractional transport, whole-clip loop, four resampler history
frames and drain count. Restored samples must match supplied immutable PCM.
Unsupported reverse/partial-region/fade/modulator/delayed-start state refuses.
Offline wrappers publish state after processing and preserve whether the original
sound remains in its track or was removed. Pending controls refuse until accepted;
no raw Rust layouts, arenas, callback pointers or executable state are decoded.

A feature-gated `TrackBuilder::offline_initial_paused` hook initializes a fresh
track with the same settled paused playback-manager state before processing.
The ordinary constructor remains playing. This lets the composed offline owner
restore master suspension without advancing child sounds or spending a sample.
The existing processor and independent pause/resume command order are unchanged.

Five added adapter tests plus all 94 pre-existing library tests pass. The Incant
mixer/session controls and actual public packaged-player PCM continuation are
recorded in `docs/spikes/native-audio-session-checkpoints.md` and its evidence.
The exact nine-file upstream patch and both source identities are retained there.
Standalone strict Clippy encounters two unchanged upstream derivable_impls;
with that single named lint excluded it passes. Device compilation has an
unchanged upstream custom_device dead-code warning. Neither upstream style issue
is rewritten in this patch.

This is ordinary offline continuation, not a physical device snapshot, arbitrary
Kira graph/effect serializer, VM heap save or raw physics snapshot. Device/module/
physics and other unimplemented full-session boundaries remain explicit refusals.
