# Exact drawable presentation observer for wgpu-hal 29.0.4

Upstream crates.io archive SHA256:
`97ace1c17727311c22a46e4e3faf56ea6de81af99dcc839bdfb54857b94d448d`.
All 84 published files, MIT/Apache-2.0 licenses and original dependency
constraints are retained. `INCANT_UPSTREAM.json` records every upstream byte
hash; `INCANT_PATCH.diff` is the complete current delta, including new files.
No upstream edition, platform dependencies or feature defaults change.

The default-off macOS `diagnostic-presentations` feature adds a surface binding,
an immutable acquisition ticket and a presented handler on that exact drawable.
The original acquire/present/commit/scheduling operations remain in place.
Callbacks retain only the collector and row index; no native object or host.

The standalone precursor's ten pure collector tests, actual host API compile,
strict host/HAL lint and feature-off check passed (see
`docs/spikes/native-presentation-observer.md`). The player integration extends
Context with a host attempt ID (still <=64 bytes/row) and adds a shutdown-only
pending scan. At the original integration milestone those extensions and the
player were not yet CPU-qualified; that historical status is superseded by
qualified source `8dbc74c4191f1e793e3e593ee68bdae28665e3ae` and the retained
CPU/build/partial-live packet in `docs/spikes/native-player-presentations.md`.
The pure control covers immutable attempt identity and pending delivery states.

No actual callback or timestamp claim follows from CPU qualification alone.
The separately retained live attempt delivered partial callbacks before an audio
failure; full replay/display coverage and performance acceptance remain open.
Per-present block allocation/native copying,
Arc and mutex operations have observation overhead. Missing callback, zero,
invalid, discard, abandoned and overflow states remain separate.

## Separate native creation observer (source-only milestone)

The default-off macOS `diagnostic-compilation` feature adds a device-owned
observer and hooks around the actual source/binary library and render/mesh/compute
pipeline creation calls. Cumulative counts do not decrement on destruction.
It changes no native options, cache/permutation policy, resource return values,
normal scheduling or default feature graph. See
`docs/spikes/native-compilation-observer.md` for exact scope and bounds.
This new seam is authored only: its CPU controls and actual hardware behavior
are not yet qualified. Existing presentation qualification does not cover it.
