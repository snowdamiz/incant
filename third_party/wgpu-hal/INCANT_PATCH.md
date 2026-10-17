# Exact drawable presentation observer for wgpu-hal 29.0.4

Upstream crates.io archive SHA256:
`97ace1c17727311c22a46e4e3faf56ea6de81af99dcc839bdfb54857b94d448d`.
All 84 published files, MIT/Apache-2.0 licenses and original dependency
constraints are retained. `INCANT_UPSTREAM.json` records every upstream byte
hash; `INCANT_PATCH.diff` retains the original observer/creation delta, including
new files. Subsequent opt-in native cache/pacing successors are recorded in Git
history and their scoped documents; the historical patch is not regenerated as
an unrelated aggregate in each successor.
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

## Separate native creation observer (CPU-qualified foundation)

The default-off macOS `diagnostic-compilation` feature adds a device-owned
observer and hooks around the actual source/binary library and render/mesh/compute
pipeline creation calls. Cumulative counts do not decrement on destruction.
Backward clock comparisons retain explicit missing timestamps and cumulative
anomaly counts rather than silently fabricating valid zero-duration samples.
It changes no native options, cache/permutation policy, resource return values,
normal scheduling or default feature graph. See
`docs/spikes/native-compilation-observer.md` for exact scope and bounds.
The original source-only status is superseded by source
`dda197ae907a5d34e7c7a69a4041e1ec4f45aee2`: 19 exact collector controls and
actual HAL/host feature-on/off compilation plus strict lint passed. No native
creation API or GPU execution follows from that result. The later host integration is CPU-qualified below;
actual hardware compilation observations remain pending; existing presentation evidence
does not qualify compilation timing.


The following host-wiring successor uses edition-neutral spelling of a nested
observer-field access and panic imports so the exact collector tests can be
included by the Rust2024 player while the HAL remains Rust2021. Collector
semantics are unchanged; the preceding qualified source remains immutable.
Host wiring source `6fa2ba2540280d7bb088d4dd03ec2426779bd8b1` subsequently
passed142 observer-enabled and97 ordinary player tests,3 renderer binding controls,
and strict all-target on/off host lint. Its two failed attempts remain bound in
`docs/spikes/evidence/native-compilation-observer/host-cpu-qualification.json`.
Live native-creation evidence remains pending; no default feature was enabled.

## Fixed minimum-duration diagnostic (source only)

The default-off macOS `diagnostic-minimum-duration` feature admits one fixed
minimum previous display residence of 1/60 second on a fresh surface. First
configure seals the cold policy; acquisition copies it into that exact texture.
Recreation requires explicit selection on the new surface. Transaction
presentation and non-FIFO configuration refuse selected mode before acquisition.
The selected present path calls the command-buffer minimum-duration API before
the existing commit; the ordinary present path and observer callback ownership
remain in place. No timer, sleep, display link or simulation change is added.
See `docs/spikes/native-metal-minimum-duration.md`. This source addition alone
does not establish compilation, native execution, cadence or performance.
